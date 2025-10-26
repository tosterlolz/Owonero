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

// discoverPeers connects to a node and gets its peer list
func discoverPeers(nodeAddr string) ([]string, error) {
	conn, err := net.Dial("tcp", nodeAddr)
	if err != nil {
		return nil, fmt.Errorf("cannot connect to node for peer discovery: %v", err)
	}
	defer conn.Close()

	reader := bufio.NewReader(conn)
	if line, err := reader.ReadString('\n'); err == nil {
		_ = line // ignore greeting
	}

	// Get peers
	fmt.Fprintf(conn, "getpeers\n")
	var peers []string
	if err := json.NewDecoder(reader).Decode(&peers); err != nil {
		return nil, fmt.Errorf("cannot read peers: %v", err)
	}

	return peers, nil
}

// startMining kopie bloki i wysyła je do node
// blocksToMine == 0 -> mine forever
func startMining(walletPath, nodeAddr string, blocksToMine, threads int, pool bool) error {
	w, err := loadOrCreateWallet(walletPath)
	if err != nil {
		return err
	}

	fmt.Printf("Mining for wallet %s to node %s\n", w.Address, nodeAddr)

	// połącz z node
	conn, err := net.Dial("tcp", nodeAddr)
	if err != nil {
		return fmt.Errorf("cannot connect to node: %v", err)
	}
	defer conn.Close()

	// consume possible greeting line from node (e.g. "owonero-daemon ...")
	reader := bufio.NewReader(conn)
	if line, err := reader.ReadString('\n'); err == nil {
		_ = line // ignore greeting
	}

	// pobierz ostatni blok node
	fmt.Fprintf(conn, "getchain\n")
	var chain Blockchain
	if err := json.NewDecoder(reader).Decode(&chain); err != nil {
		return fmt.Errorf("cannot read chain from node: %v", err)
	}
	var lastBlock = chain.Chain[len(chain.Chain)-1]

	// Now that we have the connection, tell the node about discovered peers
	if peers, err := discoverPeers(nodeAddr); err == nil {
		fmt.Printf("Discovered %d peers from node, sharing with node\n", len(peers))
		for _, peer := range peers {
			if peer != "" && peer != nodeAddr {
				fmt.Fprintf(conn, "addpeer\n%s\n", peer)
				resp, _ := reader.ReadString('\n')
				_ = resp // ignore response
			}
		}
	}

	// shared state
	var minedCount int64
	var attempts int64
	blockCh := make(chan Block, threads*2)
	shareCh := make(chan struct {
		Wallet   string
		Nonce    int
		Attempts int64
		Block    Block
	}, threads*2)
	errCh := make(chan error, 1)
	done := make(chan struct{})
	var atomicHeadHash atomic.Value
	atomicHeadHash.Store(lastBlock.Hash)
	var atomicHeadBlock atomic.Value
	atomicHeadBlock.Store(lastBlock)

	// submitter: single goroutine that sends blocks to node and updates lastBlock
	go func() {
		for {
			select {
			case b := <-blockCh:
				// Only submit if block is on top of current head
				headHash := atomicHeadHash.Load().(string)
				if b.PrevHash != headHash {
					// stale block, skip submission
					continue
				}
				fmt.Fprintf(conn, "submitblock\n")
				blockJSON, _ := json.Marshal(b)
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
					fmt.Printf("\033[32mBlock accepted! Index=%d Hash=%s\033[0m\n", b.Index, b.Hash)
					atomicHeadHash.Store(b.Hash)
					atomicHeadBlock.Store(b)
				} else {
					if strings.HasPrefix(resp, "error: block invalid") {
						fmt.Fprintf(conn, "getchain\n")
						var ch Blockchain
						if err := json.NewDecoder(reader).Decode(&ch); err != nil {
							select {
							case errCh <- fmt.Errorf("cannot refresh chain after rejection: %v", err):
							default:
							}
							close(done)
							return
						}
						if len(ch.Chain) > 0 {
							atomicHeadHash.Store(ch.Chain[len(ch.Chain)-1].Hash)
							atomicHeadBlock.Store(ch.Chain[len(ch.Chain)-1])
						}
						time.Sleep(200 * time.Millisecond)
						continue
					}
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

	// share submitter
	go func() {
		for {
			select {
			case s := <-shareCh:
				fmt.Fprintf(conn, "submitshare\n")
				shareJSON, _ := json.Marshal(s)
				fmt.Fprintf(conn, "%s\n", shareJSON)
				resp, _ := reader.ReadString('\n')
				_ = resp
			case <-done:
				return
			}
		}
	}()

	// stats printer: show H/s (hash attempts per second), SOL/s (accepted blocks/sec), and average hashrate
	go func() {
		ticker := time.NewTicker(1 * time.Second)
		defer ticker.Stop()
		prevMined := int64(0)
		// Store attempts for minute, hour, day
		var attemptsHistory []int64
		var attemptsMinute, attemptsHour, attemptsDay int64
		for {
			select {
			case <-done:
				return
			case <-ticker.C:
				h := atomic.SwapInt64(&attempts, 0)
				mined := atomic.LoadInt64(&minedCount)
				sols := mined - prevMined
				prevMined = mined
				// Track attempts for averages
				attemptsHistory = append(attemptsHistory, h)
				if len(attemptsHistory) > 86400 {
					attemptsHistory = attemptsHistory[1:]
				}
				// Calculate averages
				attemptsMinute = 0
				attemptsHour = 0
				attemptsDay = 0
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
				// human-friendly formatting
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

	// Use all available CPU cores if threads==0 or threads<1
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
				prev := atomicHeadBlock.Load().(Block)

				coinbase := Transaction{From: "coinbase", To: w.Address, Amount: 1}
				dynDiff := chain.GetDynamicDifficulty()
				if pool {
					dynDiff -= 2
					if dynDiff < 1 {
						dynDiff = 1
					}
				}
				newBlock := mineBlock(prev, []Transaction{coinbase}, dynDiff, &attempts)

				// return if signalled to stop
				select {
				case <-done:
					return
				default:
				}

				// If chain head changed while mining, skip this stale result
				headHash := atomicHeadHash.Load().(string)
				if newBlock.PrevHash != headHash {
					continue
				}

				if pool {
					select {
					case shareCh <- struct {
						Wallet   string
						Nonce    int
						Attempts int64
						Block    Block
					}{w.Address, newBlock.Nonce, atomic.LoadInt64(&attempts), newBlock}:
					default:
					}
				} else {
					// try send, but don't block forever
					select {
					case blockCh <- newBlock:
					default:
						time.Sleep(100 * time.Millisecond)
					}
				}
			}
		}(i)
	}

	// wait for completion or error
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

	// infinite mode: block until an error is reported
	return <-errCh
}
