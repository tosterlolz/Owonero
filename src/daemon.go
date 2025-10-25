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
func syncWithPeer(peerAddr string, bc *Blockchain, pm *PeerManager, difficulty int) error {
	fmt.Printf("Attempting to sync with peer %s\n", peerAddr)
	conn, err := net.Dial("tcp", peerAddr)
	if err != nil {
		return fmt.Errorf("cannot connect to peer %s: %v", peerAddr, err)
	}
	defer conn.Close()

	reader := bufio.NewReader(conn)
	// Read and ignore greeting line
	if greeting, err := reader.ReadString('\n'); err == nil {
		fmt.Printf("Connected to peer %s: %s", peerAddr, strings.TrimSpace(greeting))
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
	fmt.Printf("Peer %s height: %d, local height: %d\n", peerAddr, peerHeight, len(bc.Chain)-1)

	localHeight := len(bc.Chain) - 1
	if peerHeight <= localHeight && localHeight >= 0 {
		fmt.Printf("Peer %s is not ahead, skipping sync\n", peerAddr)
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
	fmt.Printf("Syncing %d blocks from peer %s (starting from block %d)\n", blocksToSync, peerAddr, startBlock)

	for i := startBlock; i <= peerHeight; i++ {
		if i >= len(peerChain.Chain) {
			break
		}
		block := peerChain.Chain[i]

		// For genesis block when local chain is empty, accept without validation
		if len(bc.Chain) == 0 && block.Index == 0 {
			bc.Chain = append(bc.Chain, block)
			fmt.Printf("Accepted genesis block from peer %s\n", peerAddr)
			continue
		}

		if bc.AddBlockSkipPow(block, difficulty, true) { // skip PoW validation during sync
			fmt.Printf("Synced block %d from peer %s\n", i, peerAddr)
		} else {
			fmt.Printf("Block %d validation failed\n", i)
			return fmt.Errorf("failed to validate block %d from peer %s", i, peerAddr)
		}
	}

	// Get peer's peer list and add them to our list
	fmt.Fprintf(conn, "getpeers\n")
	var peerPeers []string
	if err := json.NewDecoder(reader).Decode(&peerPeers); err != nil {
		fmt.Printf("Warning: could not get peer list from %s: %v\n", peerAddr, err)
	} else {
		for _, pp := range peerPeers {
			if pp != "" && pp != peerAddr { // don't add self
				pm.AddPeer(pp)
				fmt.Printf("Added peer %s from peer %s\n", pp, peerAddr)
			}
		}
	}

	// Save updated blockchain
	if err := bc.SaveToFile(blockchainFile); err != nil {
		return fmt.Errorf("failed to save synced blockchain: %v", err)
	}

	fmt.Printf("Successfully synced %d blocks from peer %s\n", blocksToSync, peerAddr)
	return nil
}

// syncWithPeers attempts to sync blockchain with all known peers
func syncWithPeers(pm *PeerManager, bc *Blockchain, difficulty int) {
	peers := pm.GetPeers()
	fmt.Printf("syncWithPeers called with %d peers\n", len(peers))
	if len(peers) == 0 {
		return
	}

	fmt.Printf("Attempting to sync with %d peers...\n", len(peers))
	synced := false

	for _, peer := range peers {
		if err := syncWithPeer(peer.Address, bc, pm, difficulty); err != nil {
			fmt.Printf("Sync with peer %s failed: %v\n", peer.Address, err)
		} else {
			synced = true
		}
	}

	if synced {
		fmt.Printf("Blockchain sync complete. New height: %d\n", len(bc.Chain)-1)
	}
}

func runDaemon(port int, bc *Blockchain, pm *PeerManager, difficulty int) {
	ln, err := net.Listen("tcp", ":"+strconv.Itoa(port))
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}
	defer ln.Close()
	fmt.Printf("Daemon listening on :%d  (height=%d)\n", port, len(bc.Chain)-1)

	// Initial sync with peers if any are configured
	if len(pm.GetPeers()) > 0 {
		fmt.Println("Performing initial sync with configured peers...")
		syncWithPeers(pm, bc, difficulty)
	}

	// Start periodic syncing with peers
	go func() {
		ticker := time.NewTicker(30 * time.Second) // sync every 30 seconds
		defer ticker.Stop()
		for range ticker.C {
			syncWithPeers(pm, bc, difficulty)
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
