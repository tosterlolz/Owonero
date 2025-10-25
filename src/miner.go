package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net"
	"runtime"
	"strings"
	"sync/atomic"
	"time"
)

// discoverPeers connects to a node and retrieves its peer list
func discoverPeers(nodeAddr string) ([]string, error) {
	conn, err := net.Dial("tcp", nodeAddr)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to node for peer discovery: %v", err)
	}
	defer conn.Close()

	reader := bufio.NewReader(conn)
	// Ignore greeting
	reader.ReadString('\n')

	// Request peer list
	fmt.Fprintf(conn, "getpeers\n")
	var peers []string
	if err := json.NewDecoder(reader).Decode(&peers); err != nil {
		return nil, fmt.Errorf("failed to decode peers: %v", err)
	}
	return peers, nil
}

// startMining mines blocks and submits them to the node
// blocksToMine == 0 means mine forever
func startMining(walletPath, nodeAddr string, initialDifficulty, blocksToMine, threads int) error {
	wallet, err := loadOrCreateWallet(walletPath)
	if err != nil {
		return err
	}

	fmt.Printf("Mining for wallet %s to node %s\n", wallet.Address, nodeAddr)

	// Connect to node
	conn, err := net.Dial("tcp", nodeAddr)
	if err != nil {
		return fmt.Errorf("failed to connect to node: %v", err)
	}
	defer conn.Close()

	reader := bufio.NewReader(conn)
	// Ignore greeting
	reader.ReadString('\n')

	// Report as active miner
	fmt.Fprintf(conn, "mineractive\n%s\n", wallet.Address)
	resp, _ := reader.ReadString('\n')
	_ = resp // ignore response

	// Get latest block from node
	fmt.Fprintf(conn, "getchain\n")
	// Skip node greeting line before reading JSON
	_, _ = reader.ReadString('\n')
	var chain Blockchain
	if err := json.NewDecoder(reader).Decode(&chain); err != nil {
		return fmt.Errorf("failed to read chain from node: %v", err)
	}
	lastBlock := chain.Chain[len(chain.Chain)-1]

	// Share discovered peers with node
	if peers, err := discoverPeers(nodeAddr); err == nil {
		fmt.Printf("Discovered %d peers from node, sharing with node\n", len(peers))
		for _, peer := range peers {
			if peer != "" && peer != nodeAddr {
				fmt.Fprintf(conn, "addpeer\n%s\n", peer)
				reader.ReadString('\n') // Ignore response
			}
		}
	}

	// Shared state
	var minedCount int64
	var attempts int64
	blockCh := make(chan Block, threads*2)
	errCh := make(chan error, 1)
	done := make(chan struct{})
	var atomicHeadHash atomic.Value
	atomicHeadHash.Store(lastBlock.Hash)
	var atomicHeadBlock atomic.Value
	atomicHeadBlock.Store(lastBlock)

	// Goroutine: submits blocks to node and updates lastBlock
	go func() {
		for {
			select {
			case block := <-blockCh:
				// Only submit if block is on top of current head
				headHash := atomicHeadHash.Load().(string)
				if block.PrevHash != headHash {
					// Stale block, skip submission
					continue
				}
				fmt.Fprintf(conn, "submitblock\n")
				blockJSON, _ := json.Marshal(block)
				fmt.Fprintf(conn, "%s\n", blockJSON)

				resp, rerr := reader.ReadString('\n')
				if rerr != nil {
					select {
					case errCh <- fmt.Errorf("read response error: %v", rerr):
					default:
					}
					close(done)
					return
				}
				resp = strings.TrimSpace(resp)
				if resp == "ok" {
					fmt.Printf("\033[32mBlock accepted! Index=%d Hash=%s\033[0m\n", block.Index, block.Hash)
					atomicHeadHash.Store(block.Hash)
					atomicHeadBlock.Store(block)
				} else if strings.HasPrefix(resp, "error: block invalid") {
					fmt.Fprintf(conn, "getchain\n")
					var refreshedChain Blockchain
					if err := json.NewDecoder(reader).Decode(&refreshedChain); err != nil {
						select {
						case errCh <- fmt.Errorf("failed to refresh chain after rejection: %v", err):
						default:
						}
						close(done)
						return
					}
					if len(refreshedChain.Chain) > 0 {
						atomicHeadHash.Store(refreshedChain.Chain[len(refreshedChain.Chain)-1].Hash)
						atomicHeadBlock.Store(refreshedChain.Chain[len(refreshedChain.Chain)-1])
					}
					time.Sleep(200 * time.Millisecond)
					continue
				} else {
					select {
					case errCh <- fmt.Errorf("node rejected block: %s", resp):
					default:
					}
					close(done)
					return
				}
				atomic.AddInt64(&minedCount, 1)
				if blocksToMine > 0 && atomic.LoadInt64(&minedCount) >= int64(blocksToMine) {
					close(done)
					return
				}
			case <-done:
				return
			}
		}
	}()

	// Goroutine: prints mining statistics (H/s, SOL/s, averages)
	go func() {
		ticker := time.NewTicker(1 * time.Second)
		defer ticker.Stop()
		prevMined := int64(0)
		var attemptsHistory []int64
		for {
			select {
			case <-done:
				return
			case <-ticker.C:
				h := atomic.SwapInt64(&attempts, 0)
				mined := atomic.LoadInt64(&minedCount)
				sols := mined - prevMined
				prevMined = mined
				attemptsHistory = append(attemptsHistory, h)
				if len(attemptsHistory) > 86400 {
					attemptsHistory = attemptsHistory[1:]
				}
				// Calculate averages
				var attemptsMinute, attemptsHour, attemptsDay int64
				for i := 0; i < len(attemptsHistory); i++ {
					if i >= len(attemptsHistory)-60 {
						attemptsMinute += attemptsHistory[i]
					}
					if i >= len(attemptsHistory)-3600 {
						attemptsHour += attemptsHistory[i]
					}
					attemptsDay += attemptsHistory[i]
				}
				avgMin := float64(attemptsMinute) / 60.0
				avgHour := float64(attemptsHour) / 3600.0
				avgDay := float64(attemptsDay) / 86400.0
				// Format output
				hfmt := fmt.Sprintf("%d", h)
				if h >= 1000 {
					hfmt = fmt.Sprintf("%.2fk", float64(h)/1000.0)
				}
				minFmt := fmt.Sprintf("%.0f", avgMin)
				hourFmt := fmt.Sprintf("%.0f", avgHour)
				dayFmt := fmt.Sprintf("%.0f", avgDay)
				if avgMin >= 1000 {
					minFmt = fmt.Sprintf("%.2fk", avgMin/1000.0)
				}
				if avgHour >= 1000 {
					hourFmt = fmt.Sprintf("%.2fk", avgHour/1000.0)
				}
				if avgDay >= 1000 {
					dayFmt = fmt.Sprintf("%.2fk", avgDay/1000.0)
				}
				fmt.Printf("H/s: %s    SOL/s: %d    Avg/min: %s    Avg/hr: %s    Avg/day: %s\n", hfmt, sols, minFmt, hourFmt, dayFmt)
			}
		}
	}()

	// Use all available CPU cores if threads < 1
	numThreads := threads
	if numThreads < 1 {
		numThreads = runtime.NumCPU()
	}
	for i := 0; i < numThreads; i++ {
		go func(id int) {
			for {
				if blocksToMine > 0 && atomic.LoadInt64(&minedCount) >= int64(blocksToMine) {
					return
				}
				// Always use latest chain head for mining
				prevBlock := atomicHeadBlock.Load().(Block)
				coinbase := Transaction{From: "coinbase", To: wallet.Address, Amount: 1}
				newBlock := mineBlock(prevBlock, []Transaction{coinbase}, initialDifficulty, &attempts)
				// Stop if signalled
				select {
				case <-done:
					return
				default:
				}
				// If chain head changed while mining, skip stale result
				headHash := atomicHeadHash.Load().(string)
				if newBlock.PrevHash != headHash {
					continue
				}
				// Try to send, but don't block forever
				select {
				case blockCh <- newBlock:
				default:
					time.Sleep(100 * time.Millisecond)
				}
			}
		}(i)
	}

	// Wait for completion or error
	if blocksToMine > 0 {
		for {
			if atomic.LoadInt64(&minedCount) >= int64(blocksToMine) {
				break
			}
			select {
			case e := <-errCh:
				return e
			default:
				time.Sleep(200 * time.Millisecond)
			}
		}
		return nil
	}
	// Infinite mode: block until an error is reported
	return <-errCh
}
