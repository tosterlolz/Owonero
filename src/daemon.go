package main

import (
	"fmt"
	"log"
	"net"
	"strconv"
)

func runDaemon(port int, bc *Blockchain) {
	ln, err := net.Listen("tcp", ":"+strconv.Itoa(port))
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}
	defer ln.Close()
	fmt.Printf("Daemon listening on :%d  (height=%d)\n", port, len(bc.Chain)-1)

	for {
		conn, err := ln.Accept()
		if err != nil {
			log.Println("Accept error:", err)
			continue
		}
		go handleConn(conn, bc) // wywołanie goroutine, funkcja używana
	}
}
