package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"log"
	"net"
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
	fmt.Printf("\033[36mAttempting to sync with peer %s\033[0m\n", peerAddr)
	conn, err := net.Dial("tcp", peerAddr)
	if err != nil {
		return fmt.Errorf("cannot connect to peer %s: %v", peerAddr, err)
	}
	defer conn.Close()

	reader := bufio.NewReader(conn)
	// Read and ignore greeting line
	if greeting, err := reader.ReadString('\n'); err == nil {
		fmt.Printf("\033[32mConnected to peer %s: %s\033[0m", peerAddr, strings.TrimSpace(greeting))
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
	fmt.Printf("\033[33mPeer %s height: %d, local height: %d\033[0m\n", peerAddr, peerHeight, len(bc.Chain)-1)

	localHeight := len(bc.Chain) - 1
	if peerHeight <= localHeight && localHeight >= 0 {
		fmt.Printf("\033[33mPeer %s is not ahead, skipping sync\033[0m\n", peerAddr)
		return nil // peer is not ahead
	}

	// Get full chain from peer
	fmt.Fprintf(conn, "getchain\n")
	var peerChain Blockchain
	if err := json.NewDecoder(reader).Decode(&peerChain); err != nil {
		return fmt.Errorf("cannot read peer chain: %v", err)
	}

	// Determine which blocks to sync
	startBlock := localHeight + 1
	if localHeight < 0 {
		startBlock = 0 // sync from genesis if local chain is empty
	}

	// Validate and add missing blocks
	blocksToSync := peerHeight - localHeight
	if localHeight < 0 {
		blocksToSync = peerHeight + 1 // include genesis
	}
	fmt.Printf("\033[36mSyncing %d blocks from peer %s (starting from block %d)\033[0m\n", blocksToSync, peerAddr, startBlock)

	for i := startBlock; i <= peerHeight; i++ {
		if i >= len(peerChain.Chain) {
			break
		}
		block := peerChain.Chain[i]

		// For genesis block when local chain is empty, accept without validation
		if len(bc.Chain) == 0 && block.Index == 0 {
			bc.Chain = append(bc.Chain, block)
			fmt.Printf("\033[32mAccepted genesis block from peer %s\033[0m\n", peerAddr)
			continue
		}

		targetBlockTime := 30 // seconds per block, tune as needed
		dynDiff := bc.GetDynamicDifficulty(targetBlockTime)
		if bc.AddBlockSkipPow(block, dynDiff, true) { // skip PoW validation during sync
			fmt.Printf("\033[32mSynced block %d from peer %s\033[0m\n", i, peerAddr)
		} else {
			fmt.Printf("\033[31mBlock %d validation failed\033[0m\n", i)
			return fmt.Errorf("failed to validate block %d from peer %s", i, peerAddr)
		}
	}

	// Get peer's peer list and add them to our list
	fmt.Fprintf(conn, "getpeers\n")
	var peerPeers []string
	if err := json.NewDecoder(reader).Decode(&peerPeers); err != nil {
		fmt.Printf("\033[33mWarning: could not get peer list from %s: %v\033[0m\n", peerAddr, err)
	} else {
		for _, pp := range peerPeers {
			if pp != "" && pp != peerAddr { // don't add self
				pm.AddPeer(pp)
				fmt.Printf("\033[32mAdded peer %s from peer %s\033[0m\n", pp, peerAddr)
			}
		}
	}

	// Save updated blockchain
	if err := bc.SaveToFile(blockchainFile); err != nil {
		return fmt.Errorf("failed to save synced blockchain: %v", err)
	}

	fmt.Printf("\033[32mSuccessfully synced %d blocks from peer %s\033[0m\n", blocksToSync, peerAddr)
	return nil
}

// syncWithPeers attempts to sync blockchain with all known peers
func syncWithPeers(pm *PeerManager, bc *Blockchain) {
	peers := pm.GetPeers()
	fmt.Printf("\033[36msyncWithPeers called with %d peers\033[0m\n", len(peers))
	if len(peers) == 0 {
		return
	}

	fmt.Printf("\033[36mAttempting to sync with %d peers...\033[0m\n", len(peers))
	synced := false

	for _, peer := range peers {
		if err := syncWithPeer(peer.Address, bc, pm); err != nil {
			fmt.Printf("\033[31mSync with peer %s failed: %v\033[0m\n", peer.Address, err)
		} else {
			synced = true
		}
	}

	if synced {
		fmt.Printf("\033[32mBlockchain sync complete. New height: %d\033[0m\n", len(bc.Chain)-1)
	}
}

func runDaemon(port int, bc *Blockchain, pm *PeerManager) {
	ln, err := net.Listen("tcp", ":"+strconv.Itoa(port))
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}
	defer ln.Close()
	fmt.Printf("\033[32mDaemon listening on :%d\033[0m  \033[33m(height=%d)\033[0m\n", port, len(bc.Chain)-1)

	// Initial sync with peers if any are configured
	if len(pm.GetPeers()) > 0 {
		fmt.Println("\033[36mPerforming initial sync with configured peers...\033[0m")
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
			log.Println("Accept error:", err)
			continue
		}
		go handleConn(conn, bc, pm) // wywołanie goroutine, funkcja używana
	}
}
