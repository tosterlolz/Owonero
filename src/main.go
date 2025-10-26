package main

import (
	"archive/zip"
	"bufio"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"time"

	"github.com/iskaa02/qalam/gradient"
)

const blockchainFile = "blockchain.json"
const ver = "0.3.5"

type GitHubRelease struct {
	TagName string        `json:"tag_name"`
	Assets  []GitHubAsset `json:"assets"`
}

type GitHubAsset struct {
	Name               string `json:"name"`
	BrowserDownloadURL string `json:"browser_download_url"`
}

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

// Removed static daemonDifficulty
var miners []string

func checkForUpdates() {
	// Create HTTP client with timeout
	client := &http.Client{
		Timeout: 10 * time.Second,
	}

	resp, err := client.Get("https://api.github.com/repos/tosterlolz/Owonero/releases/latest")
	if err != nil {
		fmt.Printf("\033[33mFailed to check for updates: %v\033[0m\n", err)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		fmt.Printf("\033[33mUpdate check failed: HTTP %d\033[0m\n", resp.StatusCode)
		return
	}

	var release GitHubRelease
	if err := json.NewDecoder(resp.Body).Decode(&release); err != nil {
		fmt.Printf("\033[33mFailed to parse update info: %v\033[0m\n", err)
		return
	}

	latestVer := strings.TrimPrefix(release.TagName, "v")
	if latestVer == ver {
		fmt.Printf("\033[32mYou are running the latest version (%s)\033[0m\n", ver)
		return
	}

	// Check if latest version is actually newer
	if isVersionNewer(latestVer, ver) {
		fmt.Printf("\033[33mNew version available: %s (current: %s)\033[0m\n", latestVer, ver)
		fmt.Printf("\033[36mDownloading update...\033[0m\n")
		downloadAndInstallUpdate(client, release)
	} else {
		fmt.Printf("\033[32mYou are running the latest version (%s)\033[0m\n", ver)
	}
}

func isVersionNewer(latest, current string) bool {
	// Simple version comparison (assumes semantic versioning)
	latestParts := strings.Split(latest, ".")
	currentParts := strings.Split(current, ".")

	for i := 0; i < len(latestParts) && i < len(currentParts); i++ {
		latestNum, err1 := strconv.Atoi(latestParts[i])
		currentNum, err2 := strconv.Atoi(currentParts[i])
		if err1 != nil || err2 != nil {
			return false
		}
		if latestNum > currentNum {
			return true
		}
		if latestNum < currentNum {
			return false
		}
	}
	return len(latestParts) > len(currentParts)
}

func downloadAndInstallUpdate(client *http.Client, release GitHubRelease) {
	// Determine asset name
	osName := runtime.GOOS
	arch := runtime.GOARCH
	var assetName string
	if osName == "windows" {
		assetName = fmt.Sprintf("owonero-%s-%s.zip", osName, arch)
	} else {
		assetName = fmt.Sprintf("owonero-%s-%s.zip", osName, arch)
	}

	var downloadURL string
	for _, asset := range release.Assets {
		if asset.Name == assetName {
			downloadURL = asset.BrowserDownloadURL
			break
		}
	}

	if downloadURL == "" {
		fmt.Printf("\033[31mNo suitable update found for %s/%s\033[0m\n", osName, arch)
		return
	}

	// Download the update
	resp, err := client.Get(downloadURL)
	if err != nil {
		fmt.Printf("\033[31mFailed to download update: %v\033[0m\n", err)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		fmt.Printf("\033[31mDownload failed: HTTP %d\033[0m\n", resp.StatusCode)
		return
	}

	// Get current executable path
	execPath, err := os.Executable()
	if err != nil {
		fmt.Printf("\033[31mFailed to get executable path: %v\033[0m\n", err)
		return
	}

	// Create backup
	backupPath := execPath + ".backup"
	if err := os.Rename(execPath, backupPath); err != nil {
		fmt.Printf("\033[31mFailed to create backup: %v\033[0m\n", err)
		return
	}

	// Download to temp zip file first
	tempZipPath := execPath + ".tmp.zip"
	out, err := os.Create(tempZipPath)
	if err != nil {
		fmt.Printf("\033[31mFailed to create temp zip file: %v\033[0m\n", err)
		os.Rename(backupPath, execPath) // restore
		return
	}
	defer out.Close()

	if _, err := io.Copy(out, resp.Body); err != nil {
		fmt.Printf("\033[31mFailed to write update zip: %v\033[0m\n", err)
		os.Remove(tempZipPath)
		os.Rename(backupPath, execPath) // restore
		return
	}
	out.Close()

	// Extract the zip file
	fmt.Printf("\033[36mExtracting update...\033[0m\n")
	if err := extractZip(tempZipPath, filepath.Dir(execPath)); err != nil {
		fmt.Printf("\033[31mFailed to extract update: %v\033[0m\n", err)
		os.Remove(tempZipPath)
		os.Rename(backupPath, execPath) // restore
		return
	}

	// Clean up zip file
	os.Remove(tempZipPath)

	// Make executable on Unix
	if osName != "windows" {
		if err := os.Chmod(execPath, 0755); err != nil {
			fmt.Printf("\033[31mFailed to make executable: %v\033[0m\n", err)
			os.Rename(backupPath, execPath) // restore
			return
		}
	}

	// Clean up backup
	os.Remove(backupPath)

	fmt.Printf("\033[32mUpdate installed successfully! Please restart the application.\033[0m\n")
	os.Exit(0)
}

func extractZip(zipPath, destDir string) error {
	r, err := zip.OpenReader(zipPath)
	if err != nil {
		return err
	}
	defer r.Close()

	for _, f := range r.File {
		fpath := filepath.Join(destDir, f.Name)
		if !strings.HasPrefix(fpath, filepath.Clean(destDir)+string(os.PathSeparator)) {
			return fmt.Errorf("illegal file path: %s", fpath)
		}

		if f.FileInfo().IsDir() {
			os.MkdirAll(fpath, os.ModePerm)
			continue
		}

		if err = os.MkdirAll(filepath.Dir(fpath), os.ModePerm); err != nil {
			return err
		}

		outFile, err := os.OpenFile(fpath, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, f.Mode())
		if err != nil {
			return err
		}

		rc, err := f.Open()
		if err != nil {
			outFile.Close()
			return err
		}

		_, err = io.Copy(outFile, rc)
		outFile.Close()
		rc.Close()

		if err != nil {
			return err
		}
	}
	return nil
}

func handleConn(conn net.Conn, bc *Blockchain, pm *PeerManager, shares map[string]int64) {
	defer conn.Close()
	fmt.Fprintf(conn, "owonero-daemon height=%d\n", len(bc.Chain)-1)
	scanner := bufio.NewScanner(conn)
	for scanner.Scan() {
		line := scanner.Text()
		switch line {
		case "mineractive":
			miners = append(miners, conn.RemoteAddr().String())
			fmt.Fprintln(conn, "ok")
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
			dynDiff := bc.GetDynamicDifficulty()
			if bc.AddBlock(blk, dynDiff) {
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
		case "getblocks":
			if !scanner.Scan() {
				fmt.Fprintln(conn, "error: expected start and end block indices on next line")
				continue
			}
			blockRange := strings.TrimSpace(scanner.Text())
			parts := strings.Split(blockRange, " ")
			if len(parts) != 2 {
				fmt.Fprintln(conn, "error: expected 'start end' format")
				continue
			}
			start, err1 := strconv.Atoi(parts[0])
			end, err2 := strconv.Atoi(parts[1])
			if err1 != nil || err2 != nil {
				fmt.Fprintln(conn, "error: invalid block range")
				continue
			}
			if start < 0 || end >= len(bc.Chain) || start > end {
				fmt.Fprintln(conn, "error: invalid block range")
				continue
			}
			// Send blocks in range
			blocks := bc.Chain[start : end+1]
			bs, _ := json.Marshal(blocks)
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
		case "removepeer":
			if !scanner.Scan() {
				fmt.Fprintln(conn, "error: expected peer address on next line")
				continue
			}
			peerAddr := strings.TrimSpace(scanner.Text())
			if peerAddr != "" {
				pm.RemovePeer(peerAddr)
				fmt.Fprintln(conn, "ok")
			} else {
				fmt.Fprintln(conn, "error: empty peer address")
			}
		case "getwallet":
			if !scanner.Scan() {
				fmt.Fprintln(conn, "error: expected wallet address on next line")
				continue
			}
			walletAddr := strings.TrimSpace(scanner.Text())
			if walletAddr != "" {
				// Get wallet information
				walletInfo := getWalletInfo(walletAddr, bc)
				if walletInfo != nil {
					bs, _ := json.Marshal(walletInfo)
					fmt.Fprintln(conn, string(bs))
				} else {
					fmt.Fprintln(conn, "error: wallet not found")
				}
			} else {
				fmt.Fprintln(conn, "error: empty wallet address")
			}
		case "sync":
			syncWithPeers(pm, bc)
			fmt.Fprintln(conn, "sync initiated")
		case "submitshare":
			if !scanner.Scan() {
				fmt.Fprintln(conn, "error: expected share json on next line")
				continue
			}
			var share struct {
				Wallet   string `json:"wallet"`
				Nonce    int    `json:"nonce"`
				Attempts int64  `json:"attempts"`
				Block    Block  `json:"block"`
			}
			if err := json.Unmarshal([]byte(scanner.Text()), &share); err != nil {
				fmt.Fprintln(conn, "error: cannot parse share json:", err)
				continue
			}
			// verify share: check if hash meets share diff
			calculatedHash := calculateHash(share.Block)
			dynDiff := bc.GetDynamicDifficulty()
			shareDiff := dynDiff - 2
			if shareDiff < 1 {
				shareDiff = 1
			}
			if strings.HasPrefix(calculatedHash, strings.Repeat("0", shareDiff)) {
				// valid share, record
				shares[share.Wallet] += share.Attempts
				fmt.Printf("Accepted share from %s: %d attempts (total shares: %d)\n", share.Wallet, share.Attempts, shares[share.Wallet])
				fmt.Fprintln(conn, "ok")
			} else {
				fmt.Fprintln(conn, "error: invalid share")
			}
		default:
			fmt.Fprintln(conn, "unknown command. supported: getchain, getheight, submitblock, sendtx, getpeers, addpeer, sync")
		}
	}
}

type Config struct {
	NodeAddress     string   `json:"node_address"`
	DaemonPort      int      `json:"daemon_port"`
	WebPort         int      `json:"web_port"`
	WalletPath      string   `json:"wallet_path"`
	MiningThreads   int      `json:"mining_threads"`
	Peers           []string `json:"peers"`
	AutoUpdate      bool     `json:"auto_update"`
	SyncOnStartup   bool     `json:"sync_on_startup"`
	TargetBlockTime int      `json:"target_block_time"`
}

func main() {
	// Print ASCII logo with gradient
	var bc Blockchain
	g, err := gradient.NewGradient("magenta", "pink")
	if err != nil {
		log.Fatalf("Failed to create gradient: %v", err)
	}
	g.Print(fmt.Sprintf(asciiLogo, ver))

	// Load config
	var config Config
	if data, err := os.ReadFile("config.json"); err == nil {
		if err := json.Unmarshal(data, &config); err != nil {
			fmt.Printf("Warning: failed to parse config.json: %v\n", err)
			config = Config{
				NodeAddress:     "localhost:6969",
				DaemonPort:      6969,
				WebPort:         6767,
				WalletPath:      "wallet.json",
				MiningThreads:   1,
				Peers:           []string{},
				AutoUpdate:      true,
				SyncOnStartup:   true,
				TargetBlockTime: 30,
			}
		}
	} else {
		config = Config{
			NodeAddress:     "localhost:6969",
			DaemonPort:      6969,
			WebPort:         6767,
			WalletPath:      "wallet.json",
			MiningThreads:   1,
			Peers:           []string{},
			AutoUpdate:      true,
			SyncOnStartup:   true,
			TargetBlockTime: 30,
		}
	}

	// Parse flags early to check for no-update
	noUpdate := flag.Bool("no-update", !config.AutoUpdate, "skip automatic update check on startup")
	daemon := flag.Bool("d", false, "run as daemon")
	tui := flag.Bool("tui", false, "run wallet in TUI mode")
	port := flag.Int("p", config.DaemonPort, "daemon port")
	webPort := flag.Int("web", config.WebPort, "web stats server port")
	walletPath := flag.String("w", config.WalletPath, "wallet file path")
	mine := flag.Bool("m", false, "mine blocks (uses -w wallet file)")
	blocks := flag.Int("b", 0, "how many blocks to mine when mining (0 = mine forever)")
	pool := flag.Bool("pool", false, "enable pool mining mode")
	// Removed static mining difficulty flag

	var nodeAddr string
	flag.StringVar(&nodeAddr, "n", config.NodeAddress, "node address host:port")
	flag.StringVar(&nodeAddr, "node", config.NodeAddress, "node address host:port")
	var threads int
	flag.IntVar(&threads, "t", config.MiningThreads, "number of mining threads")
	flag.IntVar(&threads, "threads", config.MiningThreads, "number of mining threads")
	var peersStr string
	flag.StringVar(&peersStr, "peers", strings.Join(config.Peers, ","), "comma-separated list of peer addresses (host:port)")
	noInit := flag.Bool("no-init", false, "don't initialize blockchain.json, rely on syncing")

	flag.Parse()

	// Check for updates (unless disabled)
	if !*noUpdate {
		checkForUpdates()
	} else {
		fmt.Printf("\033[33mUpdate check skipped (--no-update flag used)\033[0m\n")
	}
	if *tui {
		wallet_main(nodeAddr)
		return
	}
	for _, a := range flag.Args() {
		if strings.Contains(a, ":") && !strings.HasPrefix(a, "OWO") {
			nodeAddr = a
			break
		}
	}

	if !*noInit {
		if err := bc.LoadFromFile(blockchainFile); err != nil {
			log.Fatalf("Cannot init blockchain: %v", err)
		}
		bc.TargetBlockTime = config.TargetBlockTime
		_ = bc.SaveToFile(blockchainFile)
	} else {
		fmt.Println("Skipping blockchain initialization (--no-init flag used)")
	}

	// Sync with specified node if not default
	if nodeAddr != "localhost:6969" {
		pm := &PeerManager{}
		pm.AddPeer(nodeAddr)
		fmt.Println("Syncing blockchain with specified node...")
		syncWithPeers(pm, &bc)
		_ = bc.SaveToFile(blockchainFile)
	}

	if *daemon {
		// Removed static daemonDifficulty assignment
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
		fmt.Printf("\033[32mDaemon starting with %d peers\033[0m\n", len(pm.GetPeers()))
		go startWebStatsServer(&bc, *webPort)
		runDaemon(*port, &bc, pm, *pool)
		return
	}

	if *mine {
		if err := startMining(*walletPath, nodeAddr, *blocks, threads, *pool); err != nil {
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
