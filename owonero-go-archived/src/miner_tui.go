package main

import (
	"fmt"
	"os"
	"os/signal"
	"syscall"
	"time"
)

// RunMinerUI renders a simple xmrig-like console UI using ANSI escapes.
// It listens on stats channel for MinerStats and updates once per second.
func RunMinerUI(stats <-chan MinerStats, done <-chan struct{}) {
	// terminal control sequences
	clear := "\x1b[2J"
	home := "\x1b[H"
	hideCursor := "\x1b[?25l"
	showCursor := "\x1b[?25h"

	// handle CTRL-C to restore cursor
	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigs
		fmt.Print(showCursor)
		os.Exit(0)
	}()

	last := MinerStats{}
	ticker := time.NewTicker(1 * time.Second)
	defer ticker.Stop()

	fmt.Print(hideCursor)
	defer fmt.Print(showCursor)

	for {
		select {
		case <-done:
			// exit UI
			fmt.Print(showCursor)
			return
		case s := <-stats:
			last = s
		case <-ticker.C:
			// redraw
			fmt.Print(clear)
			fmt.Print(home)
			fmt.Printf("OWONERO Miner UI\n")
			fmt.Printf("----------------------------\n")
			fmt.Printf("Threads         : %d\n", last.Threads)
			// format H/s
			hfmt := fmt.Sprintf("%d H/s", last.TotalHps)
			if last.TotalHps >= 1000 {
				hfmt = fmt.Sprintf("%.2f kH/s", float64(last.TotalHps)/1000.0)
			}
			fmt.Printf("Total H/s       : %s\n", hfmt)
			fmt.Printf("SOL/s (blocks)  : %d\n", last.Sols)
			fmt.Printf("Blocks mined    : %d\n", last.Mined)
			fmt.Printf("Attempts/s      : %d\n", last.Attempts)
			fmt.Printf("Avg/min         : %.1f H/s\n", last.AvgMin)
			fmt.Printf("Avg/hr          : %.1f H/s\n", last.AvgHour)
			fmt.Printf("Avg/day         : %.1f H/s\n", last.AvgDay)
			fmt.Printf("----------------------------\n")
			fmt.Printf("Press Ctrl-C to quit miner UI (miner continues in background).\n")
		}
	}
}
