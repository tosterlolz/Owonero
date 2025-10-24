package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"log"
	"net"
	"strings"
	"sync/atomic"
	"time"
)

// startMining kopie bloki i wysyła je do node
// blocksToMine == 0 -> mine forever
func startMining(walletPath, nodeAddr string, initialDifficulty, blocksToMine, threads int) error {
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

	// shared state
	var minedCount int64
	var attempts int64
	blockCh := make(chan Block, threads*2)
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

	// stats printer: show H/s (hash attempts per second) and SOL/s (accepted blocks/sec)
	go func() {
		ticker := time.NewTicker(1 * time.Second)
		defer ticker.Stop()
		prevMined := int64(0)
		for {
			select {
			case <-done:
				return
			case <-ticker.C:
				h := atomic.SwapInt64(&attempts, 0)
				mined := atomic.LoadInt64(&minedCount)
				sols := mined - prevMined
				prevMined = mined
				// human-friendly formatting
				hfmt := fmt.Sprintf("%d", h)
				if h >= 1000 {
					hfmt = fmt.Sprintf("%.2fk", float64(h)/1000.0)
				}
				fmt.Printf("H/s: %s    SOL/s: %d\n", hfmt, sols)
			}
		}
	}()

	// worker goroutines
	for i := 0; i < threads; i++ {
		go func(id int) {
			for {
				if blocksToMine > 0 && atomic.LoadInt64(&minedCount) >= int64(blocksToMine) {
					return
				}

				// Always use latest chain head for mining
				prev := atomicHeadBlock.Load().(Block)

				coinbase := Transaction{From: "coinbase", To: w.Address, Amount: 1}
				newBlock := mineBlock(prev, []Transaction{coinbase}, initialDifficulty, &attempts)

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

				// try send, but don't block forever
				select {
				case blockCh <- newBlock:
				default:
					time.Sleep(100 * time.Millisecond)
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
