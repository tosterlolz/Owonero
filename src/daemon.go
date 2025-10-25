package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"strconv"
	"strings"
	"sync"
	"time"
)

// Peer represents a network peer
type Peer struct {
	Address string `json:"address"`
}

type WalletInfo struct {
	Address       string `json:"address"`
	TotalReceived int64  `json:"total_received"`
	TotalSent     int64  `json:"total_sent"`
	Balance       int64  `json:"balance"`
}

// PeerManager manages the list of known peers
type PeerManager struct {
	peers []Peer
	mutex sync.RWMutex
}

// AddPeer adds a new peer to the list if not already present
func (pm *PeerManager) AddPeer(address string) {
	pm.mutex.Lock()
	defer pm.mutex.Unlock()
	for _, p := range pm.peers {
		if p.Address == address {
			return
		}
	}
	pm.peers = append(pm.peers, Peer{Address: address})
}

// GetPeers returns a copy of the current peer list
func (pm *PeerManager) GetPeers() []Peer {
	pm.mutex.RLock()
	defer pm.mutex.RUnlock()
	peers := make([]Peer, len(pm.peers))
	copy(peers, pm.peers)
	return peers
}

// RemovePeer removes a peer from the list
func (pm *PeerManager) RemovePeer(address string) {
	pm.mutex.Lock()
	defer pm.mutex.Unlock()
	for i, p := range pm.peers {
		if p.Address == address {
			pm.peers = append(pm.peers[:i], pm.peers[i+1:]...)
			break
		}
	}
}

func getWalletInfo(address string, bc *Blockchain) *WalletInfo {
	var totalReceived, totalSent int64
	for _, block := range bc.Chain {
		for _, tx := range block.Transactions {
			if tx.To == address {
				totalReceived += int64(tx.Amount)
			}
			if tx.From == address {
				totalSent += int64(tx.Amount)
			}
		}
	}
	return &WalletInfo{
		Address:       address,
		TotalReceived: totalReceived,
		TotalSent:     totalSent,
		Balance:       totalReceived - totalSent,
	}
}

// syncWithPeer attempts to sync blockchain with a specific peer
func syncWithPeer(peerAddr string, bc *Blockchain, pm *PeerManager) error {
	fmt.Printf("%sAttempting to sync with peer %s%s\n", Cyan, peerAddr, Reset)
	conn, err := net.Dial("tcp", peerAddr)
	if err != nil {
		return fmt.Errorf("cannot connect to peer %s: %v", peerAddr, err)
	}
	defer conn.Close()

	reader := bufio.NewReader(conn)
	// Read and ignore greeting line
	if greeting, err := reader.ReadString('\n'); err == nil {
		fmt.Printf("%sConnected to peer %s: %s%s", Green, peerAddr, strings.TrimSpace(greeting), Reset)
	}

	// Get peer's chain height first
	fmt.Fprintf(conn, "getheight\n")
	heightLine, err := reader.ReadString('\n')
	if err != nil {
		return fmt.Errorf("cannot read peer height response: %v", err)
	}
	heightLine = strings.TrimSpace(heightLine)
	peerHeight, err := strconv.Atoi(heightLine)
	if err != nil {
		return fmt.Errorf("cannot parse peer height '%s': %v", heightLine, err)
	}
	fmt.Printf("%sPeer %s height: %d, local height: %d%s\n", Yellow, peerAddr, peerHeight, len(bc.Chain)-1, Reset)

	localHeight := len(bc.Chain) - 1
	if peerHeight <= localHeight && localHeight >= 0 {
		fmt.Printf("%sPeer %s is not ahead, skipping sync%s\n", Yellow, peerAddr, Reset)
		return nil // peer is not ahead
	}

	// Determine which blocks to sync
	startBlock := localHeight + 1
	if localHeight < 0 {
		startBlock = 0 // sync from genesis if local chain is empty
	}

	blocksToSync := peerHeight - localHeight
	if localHeight < 0 {
		blocksToSync = peerHeight + 1 // include genesis
	}
	fmt.Printf("%sSyncing %d blocks from peer %s (starting from block %d)%s\n", Cyan, blocksToSync, peerAddr, startBlock, Reset)

	// Sync blocks in chunks to avoid overwhelming the connection
	const chunkSize = 100
	totalSynced := 0

	for chunkStart := startBlock; chunkStart <= peerHeight; chunkStart += chunkSize {
		chunkEnd := chunkStart + chunkSize - 1
		if chunkEnd > peerHeight {
			chunkEnd = peerHeight
		}

		fmt.Printf("%sRequesting blocks %d to %d from peer %s%s\n", Cyan, chunkStart, chunkEnd, peerAddr, Reset)

		// Request block range from peer
		fmt.Fprintf(conn, "getblocks\n")
		fmt.Fprintf(conn, "%d %d\n", chunkStart, chunkEnd)

		var blocks []Block
		if err := json.NewDecoder(reader).Decode(&blocks); err != nil {
			return fmt.Errorf("cannot read blocks from peer: %v", err)
		}

		// Validate and add received blocks
		for _, block := range blocks {
			// For genesis block when local chain is empty, accept without validation
			if len(bc.Chain) == 0 && block.Index == 0 {
				bc.Chain = append(bc.Chain, block)
				fmt.Printf("%sAccepted genesis block from peer %s%s\n", Green, peerAddr, Reset)
				totalSynced++
				continue
			}

			targetBlockTime := 30 // seconds per block, tune as needed
			dynDiff := bc.GetDynamicDifficulty(targetBlockTime)
			if bc.AddBlockSkipPow(block, dynDiff, true) { // skip PoW validation during sync
				fmt.Printf("%sSynced block %d from peer %s%s\n", Green, block.Index, peerAddr, Reset)
				totalSynced++
			} else {
				fmt.Printf("%sBlock %d validation failed%s\n", Red, block.Index, Reset)
				return fmt.Errorf("failed to validate block %d from peer %s", block.Index, peerAddr)
			}
		}
	}

	// Get peer's peer list and add them to our list
	fmt.Fprintf(conn, "getpeers\n")
	var peerPeers []string
	if err := json.NewDecoder(reader).Decode(&peerPeers); err != nil {
		fmt.Printf("%sWarning: could not get peer list from %s: %v%s\n", Yellow, peerAddr, err, Reset)
	} else {
		for _, pp := range peerPeers {
			if pp != "" && pp != peerAddr { // don't add self
				pm.AddPeer(pp)
				fmt.Printf("%sAdded peer %s from peer %s%s\n", Green, pp, peerAddr, Reset)
			}
		}
	}

	// Save updated blockchain
	if err := bc.SaveToFile(blockchainFile); err != nil {
		return fmt.Errorf("failed to save synced blockchain: %v", err)
	}

	fmt.Printf("%sSuccessfully synced %d blocks from peer %s%s\n", Green, totalSynced, peerAddr, Reset)
	return nil
}

// syncWithPeers attempts to sync blockchain with all known peers
func syncWithPeers(pm *PeerManager, bc *Blockchain) {
	peers := pm.GetPeers()
	fmt.Printf("%ssyncWithPeers called with %d peers%s\n", Cyan, len(peers), Reset)
	if len(peers) == 0 {
		return
	}

	fmt.Printf("%sAttempting to sync with %d peers...%s\n", Cyan, len(peers), Reset)
	synced := false

	for _, peer := range peers {
		if err := syncWithPeer(peer.Address, bc, pm); err != nil {
			fmt.Printf("%sSync with peer %s failed: %v%s\n", Red, peer.Address, err, Reset)
		} else {
			synced = true
		}
	}

	if synced {
		fmt.Printf("%sBlockchain sync complete. New height: %d%s\n", Green, len(bc.Chain)-1, Reset)
	}
}

func runDaemon(port int, bc *Blockchain, pm *PeerManager) {
	ln, err := net.Listen("tcp", ":"+strconv.Itoa(port))
	if err != nil {
		fmt.Printf("%s%sFailed to listen: %v%s\n", Red, Bold, err, Reset)
		os.Exit(1)
	}
	defer ln.Close()
	fmt.Printf("%sDaemon listening on :%d%s  %s(height=%d)%s\n", Green, port, Reset, Yellow, len(bc.Chain)-1, Reset)

	// Initial sync with peers if any are configured
	if len(pm.GetPeers()) > 0 {
		fmt.Printf("%sPerforming initial sync with configured peers...%s\n", Cyan, Reset)
		targetBlockTime := 30                        // seconds per block, tune as needed
		_ = bc.GetDynamicDifficulty(targetBlockTime) // can be used for mining, not for sync here
		syncWithPeers(pm, bc)
	}

	// Start periodic syncing with peers
	go func() {
		ticker := time.NewTicker(30 * time.Second) // sync every 30 seconds
		defer ticker.Stop()
		for range ticker.C {
			targetBlockTime := 30 // seconds per block, tune as needed
			_ = bc.GetDynamicDifficulty(targetBlockTime)
			syncWithPeers(pm, bc)
		}
	}()

	for {
		conn, err := ln.Accept()
		if err != nil {
			fmt.Printf("%sAccept error: %v%s\n", Red, err, Reset)
			continue
		}
		go handleConn(conn, bc, pm) // wywołanie goroutine, funkcja używana
	}
}
