package main

import (
	"bufio"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"net"
	"strings"

	"github.com/iskaa02/qalam/gradient"
)

const blockchainFile = "blockchain.json"
const ver = "0.1.3"

const asciiLogo = `
⠀⠀⠀⠀⡰⠁⠀⠀⢀⢔⣔⣤⠐⠒⠒⠒⠒⠠⠄⢀⠀⠐⢀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⡐⢀⣾⣷⠪⠑⠛⠛⠛⠂⠠⠶⢶⣿⣦⡀⠀⠈⢐⢠⣑⠤⣀⠀⠀⠀
⠀⢀⡜⠀⢸⠟⢁⠔⠁⠀⠀⠀⠀⠀⠀⠀⠉⠻⢷⠀⠀⠀⡦⢹⣷⣄⠀⢀⣀⡀
⠀⠸⠀⠠⠂⡰⠁⡜⠀⠀⠀⠀⠀⠀⠀⡀⠀⠀⠈⠇⠀⠀⢡⠙⢿⣿⣾⣿⣿⠃
⠀⠀⠠⠁⠰⠁⢠⢀⠀⠀⡄⠀⠀⠀⠀⠁⠀⠀⠀⠀⠀⠀⠀⢢⠀⢉⡻⣿⣇⠀
⠀⠠⠁⠀⡇⠀⡀⣼⠀⢰⡇⠀⠀⠀⠀⢸⠀⠀⠀⠀⠀⠀⠀⢸⣧⡈⡘⣷⠟⠀     ______          ________ 
⠀⠀⠀⠈⠀⠀⣧⢹⣀⡮⡇⠀⠀⠀⢸⢸⡄⠀⠀⠀⠀⠀⠀⢸⠈⠈⠲⠇⠀⠀    / __ \ \        / /  ____|
⠀⢰⠀⢸⢰⢰⠘⠀⢶⠀⢷⡄⠈⠁⡚⡾⢧⢠⡀⢠⠀⠀⠀⢸⡀⠀⠀⠰⠀   | |  | \ \  /\  / /| |__
⣧⠈⡄⠈⣿⡜⢱⣶⣦⠀⠀⢠⠆⠀⣁⣀⠘⢸⠀⢸⠀⡄⠀⠀⡆⠀⠠⡀⠃  | |  | |\ \/  \/ / |  __| 
⢻⣷⡡⢣⣿⠃⠘⠿⠏⠀⠀⠀⠂⠀⣿⣿⣿⡇⠀⡀⣰⡗⠄⡀⠰⠀⠀⠀⠀  | |__| | \  /\  /  | |____
⠀⠙⢿⣜⢻⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠋⢁⢡⠀⡷⣿⠁⠈⠋⠢⢇⠀⡀⠀   \_____/   \/  \/   |______|
⠀⠀⠈⢻⠀⡆⠀⠀⠀⠀⠀⠀⠀⠀⠐⠆⡘⡇⠀⣼⣿⡇⢀⠀⠀⠀⢱⠁⠀ 							   V.%s
⠐⢦⣀⠸⡀⢸⣦⣄⡀⠒⠄⠀⠀⠀⢀⣀⣴⠀⣸⣿⣿⠁⣼⢦⠀⠀⠘⠀		
⠀⠀⢎⠳⣇⠀⢿⣿⣿⣶⣤⡶⣾⠿⠋⣁⡆⡰⢿⣿⣿⡜⢣⠀⢆⡄⠇⠀
⠀⠀⠈⡄⠈⢦⡘⡇⠟⢿⠙⡿⢀⠐⠁⢰⡜⠀⠀⠙⢿⡇⠀⡆⠈⡟⠀⠀      
`

var daemonDifficulty int

func handleConn(conn net.Conn, bc *Blockchain, pm *PeerManager) {
	defer conn.Close()
	fmt.Fprintf(conn, "owonero-daemon height=%d\n", len(bc.Chain)-1)
	scanner := bufio.NewScanner(conn)
	for scanner.Scan() {
		line := scanner.Text()
		switch line {
		case "getchain":
			bs, _ := json.Marshal(bc)
			fmt.Fprintln(conn, string(bs))
		case "getheight":
			fmt.Fprintln(conn, len(bc.Chain)-1)
		case "submitblock":
			if !scanner.Scan() {
				fmt.Fprintln(conn, "error: expected block json on next line")
				continue
			}
			var blk Block
			if err := json.Unmarshal([]byte(scanner.Text()), &blk); err != nil {
				fmt.Fprintln(conn, "error: cannot parse block json:", err)
				continue
			}
			if bc.AddBlock(blk, daemonDifficulty) {
				_ = bc.SaveToFile(blockchainFile)
				fmt.Fprintln(conn, "ok")
			} else {
				fmt.Fprintln(conn, "error: block invalid")
			}
		case "sendtx":
			if !scanner.Scan() {
				fmt.Fprintln(conn, "error: expected transaction json on next line")
				continue
			}
			var tx Transaction
			if err := json.Unmarshal([]byte(scanner.Text()), &tx); err != nil {
				fmt.Fprintln(conn, "error: cannot parse transaction json:", err)
				continue
			}
			// Weryfikacja podpisu
			if !VerifyTransactionSignature(&tx, tx.From) { // zakładamy, że pole From to PEM klucza publicznego
				fmt.Fprintln(conn, "error: invalid transaction signature")
				continue
			}
			// Dodaj do mempoola lub bezpośrednio do bloku (tu uproszczone: do ostatniego bloku)
			if len(bc.Chain) == 0 {
				fmt.Fprintln(conn, "error: blockchain empty")
				continue
			}
			last := &bc.Chain[len(bc.Chain)-1]
			last.Transactions = append(last.Transactions, tx)
			_ = bc.SaveToFile(blockchainFile)
			fmt.Fprintln(conn, "ok")
		case "getpeers":
			peers := pm.GetPeers()
			peerAddrs := make([]string, len(peers))
			for i, p := range peers {
				peerAddrs[i] = p.Address
			}
			bs, _ := json.Marshal(peerAddrs)
			fmt.Fprintln(conn, string(bs))
		case "addpeer":
			if !scanner.Scan() {
				fmt.Fprintln(conn, "error: expected peer address on next line")
				continue
			}
			peerAddr := strings.TrimSpace(scanner.Text())
			if peerAddr != "" {
				pm.AddPeer(peerAddr)
				fmt.Fprintln(conn, "ok")
			} else {
				fmt.Fprintln(conn, "error: empty peer address")
			}
		case "sync":
			syncWithPeers(pm, bc, daemonDifficulty)
			fmt.Fprintln(conn, "sync initiated")
		default:
			fmt.Fprintln(conn, "unknown command. supported: getchain, getheight, submitblock, sendtx, getpeers, addpeer, sync")
		}
	}
}

func main() {
	// Print ASCII logo with gradient
	g, err := gradient.NewGradient("magenta", "pink")
	if err != nil {
		log.Fatalf("Failed to create gradient: %v", err)
	}
	g.Print(fmt.Sprintf(asciiLogo, ver))

	daemon := flag.Bool("d", false, "run as daemon")
	port := flag.Int("p", 6969, "daemon port")
	walletPath := flag.String("w", "wallet.json", "wallet file path")
	mine := flag.Bool("m", false, "mine blocks (uses -w wallet file)")
	blocks := flag.Int("b", 0, "how many blocks to mine when mining (0 = mine forever)")
	diff := flag.Int("diff", 3, "mining difficulty (leading zeros)")

	var nodeAddr string
	flag.StringVar(&nodeAddr, "n", "localhost:6969", "node address host:port")
	flag.StringVar(&nodeAddr, "node", "localhost:6969", "node address host:port")
	var threads int
	flag.IntVar(&threads, "t", 1, "number of mining threads")
	flag.IntVar(&threads, "threads", 1, "number of mining threads")
	var peersStr string
	flag.StringVar(&peersStr, "peers", "", "comma-separated list of peer addresses (host:port)")
	noInit := flag.Bool("no-init", false, "don't initialize blockchain.json, rely on syncing")

	flag.Parse()

	for _, a := range flag.Args() {
		if strings.Contains(a, ":") && !strings.HasPrefix(a, "OWO") {
			nodeAddr = a
			break
		}
	}

	var bc Blockchain
	if !*noInit {
		if err := bc.LoadFromFile(blockchainFile); err != nil {
			log.Fatalf("Cannot init blockchain: %v", err)
		}
		_ = bc.SaveToFile(blockchainFile)
	} else {
		fmt.Println("Skipping blockchain initialization (--no-init flag used)")
	}

	if *daemon {
		daemonDifficulty = *diff
		pm := &PeerManager{}
		// Add initial peers from command line
		if peersStr != "" {
			peerList := strings.Split(peersStr, ",")
			for _, peer := range peerList {
				peer = strings.TrimSpace(peer)
				if peer != "" {
					pm.AddPeer(peer)
				}
			}
		}
		// Also add the node address as a peer if specified
		if nodeAddr != "localhost:6969" { // don't add default
			fmt.Printf("Adding peer from -n flag: %s\n", nodeAddr)
			pm.AddPeer(nodeAddr)
		}
		fmt.Printf("Daemon starting with %d peers\n", len(pm.GetPeers()))
		runDaemon(*port, &bc, pm, *diff)
		return
	}

	if *mine {
		if err := startMining(*walletPath, nodeAddr, *diff, *blocks, threads); err != nil {
			log.Fatalf("Mining failed: %v", err)
		}
		return
	}

	w, err := loadOrCreateWallet(*walletPath)
	if err != nil {
		log.Fatalf("Wallet error: %v", err)
	}
	if err := bc.LoadFromFile(blockchainFile); err != nil {
		log.Fatalf("Blockchain load error: %v", err)
	}
	fmt.Printf("\033[33mWallet:\033[0m \033[32m%s\033[0m\n", w.Address)
	fmt.Printf("\033[33mBalance:\033[0m \033[32m%d\033[0m\n", getBalance(w, &bc))
	fmt.Printf("\033[33mChain height:\033[0m \033[35m%d\033[0m\n", len(bc.Chain)-1)
}
