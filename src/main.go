package main

import (
	"archive/zip"
	"bufio"
	"encoding/json"
	"flag"
	"fmt"
	"io"
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
const ver = "0.3.6"

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
		fmt.Printf("%sFailed to check for updates: %v%s\n", Yellow, err, Reset)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		fmt.Printf("%sUpdate check failed: HTTP %d%s\n", Yellow, resp.StatusCode, Reset)
		return
	}

	var release GitHubRelease
	if err := json.NewDecoder(resp.Body).Decode(&release); err != nil {
		fmt.Printf("%sFailed to parse update info: %v%s\n", Yellow, err, Reset)
		return
	}

	latestVer := strings.TrimPrefix(release.TagName, "v")
	if latestVer == ver {
		fmt.Printf("%sYou are running the latest version (%s)%s\n", Green, ver, Reset)
		return
	}

	// Check if latest version is actually newer
	if isVersionNewer(latestVer, ver) {
		fmt.Printf("%sNew version available: %s (current: %s)%s\n", Yellow, latestVer, ver, Reset)
		fmt.Printf("%sDownloading update...%s\n", Cyan, Reset)
		downloadAndInstallUpdate(client, release)
	} else {
		fmt.Printf("%sYou are running the latest version (%s)%s\n", Green, ver, Reset)
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
		fmt.Printf("%sNo suitable update found for %s/%s%s\n", Red, osName, arch, Reset)
		return
	}

	// Download the update
	resp, err := client.Get(downloadURL)
	if err != nil {
		fmt.Printf("%sFailed to download update: %v%s\n", Red, err, Reset)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		fmt.Printf("%sDownload failed: HTTP %d%s\n", Red, resp.StatusCode, Reset)
		return
	}

	// Get current executable path
	execPath, err := os.Executable()
	if err != nil {
		fmt.Printf("%sFailed to get executable path: %v%s\n", Red, err, Reset)
		return
	}

	// Create backup
	backupPath := execPath + ".backup"
	if err := os.Rename(execPath, backupPath); err != nil {
		fmt.Printf("%sFailed to create backup: %v%s\n", Red, err, Reset)
		return
	}

	// Download to temp zip file first
	tempZipPath := execPath + ".tmp.zip"
	out, err := os.Create(tempZipPath)
	if err != nil {
		fmt.Printf("%sFailed to create temp zip file: %v%s\n", Red, err, Reset)
		os.Rename(backupPath, execPath) // restore
		return
	}
	defer out.Close()

	if _, err := io.Copy(out, resp.Body); err != nil {
		fmt.Printf("%sFailed to write update zip: %v%s\n", Red, err, Reset)
		os.Remove(tempZipPath)
		os.Rename(backupPath, execPath) // restore
		return
	}
	out.Close()

	// Extract the zip file
	fmt.Printf("%sExtracting update...%s\n", Cyan, Reset)
	if err := extractZip(tempZipPath, filepath.Dir(execPath)); err != nil {
		fmt.Printf("%sFailed to extract update: %v%s\n", Red, err, Reset)
		os.Remove(tempZipPath)
		os.Rename(backupPath, execPath) // restore
		return
	}

	// Clean up zip file
	os.Remove(tempZipPath)

	// Make executable on Unix
	if osName != "windows" {
		if err := os.Chmod(execPath, 0755); err != nil {
			fmt.Printf("%sFailed to make executable: %v%s\n", Red, err, Reset)
			os.Rename(backupPath, execPath) // restore
			return
		}
	}

	// Clean up backup
	os.Remove(backupPath)

	fmt.Printf("%sUpdate installed successfully! Please restart the application.%s\n", Green, Reset)
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

func handleConn(conn net.Conn, bc *Blockchain, pm *PeerManager) {
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
			targetBlockTime := 30 // seconds per block, tune as needed
			dynDiff := bc.GetDynamicDifficulty(targetBlockTime)
			if bc.AddBlock(blk, dynDiff) {
				_ = bc.SaveToFile(blockchainFile)
				// Log who mined the block
				if len(blk.Transactions) > 0 && blk.Transactions[0].From == "coinbase" {
					fmt.Printf("%sBlock %d mined by %s%s\n", Green, blk.Index, blk.Transactions[0].To, Reset)
				}
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
		default:
			fmt.Fprintln(conn, "unknown command. supported: getchain, getheight, submitblock, sendtx, getpeers, addpeer, sync")
		}
	}
}

func main() {
	// Print ASCII logo with gradient
	var bc Blockchain
	g, err := gradient.NewGradient("magenta", "pink")
	if err != nil {
		fmt.Printf("%s%sFailed to create gradient: %v%s\n", Red, Bold, err, Reset)
		os.Exit(1)
	}
	g.Print(fmt.Sprintf(asciiLogo, ver))

	// Parse flags early to check for no-update
	noUpdate := flag.Bool("no-update", false, "skip automatic update check on startup")
	daemon := flag.Bool("d", false, "run as daemon")
	tui := flag.Bool("tui", false, "run wallet in TUI mode")
	port := flag.Int("p", 6969, "daemon port")
	// Default web stats server disabled (0). Use --web <port> to enable (e.g. --web 6767).
	webPort := flag.Int("web", 0, "web stats server port (0 = disabled). Example: --web 6767")
	walletPath := flag.String("w", "wallet.json", "wallet file path")
	mine := flag.Bool("m", false, "mine blocks (uses -w wallet file)")
	blocks := flag.Int("b", 0, "how many blocks to mine when mining (0 = mine forever)")
	// Removed static mining difficulty flag

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

	// Check for updates (unless disabled)
	if !*noUpdate {
		checkForUpdates()
	} else {
		fmt.Printf("%sUpdate check skipped (--no-update flag used)%s\n", Yellow, Reset)
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
			fmt.Printf("%s%sCannot init blockchain: %v%s\n", Red, Bold, err, Reset)
			os.Exit(1)
		}
		_ = bc.SaveToFile(blockchainFile)
	} else {
		fmt.Println("Skipping blockchain initialization (--no-init flag used)")
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
		fmt.Printf("%sDaemon starting with %d peers%s\n", Green, len(pm.GetPeers()), Reset)
		if *webPort > 0 {
			go startWebStatsServer(&bc, *webPort)
			fmt.Printf("%sWeb stats server enabled on :%d%s\n", Green, *webPort, Reset)
		} else {
			fmt.Printf("%sWeb stats server disabled (use --web <port> to enable)%s\n", Yellow, Reset)
		}
		runDaemon(*port, &bc, pm)
		return
	}

	if *mine {
		if err := startMining(*walletPath, nodeAddr, *blocks, threads); err != nil {
			fmt.Printf("%s%sMining failed: %v%s\n", Red, Bold, err, Reset)
			os.Exit(1)
		}
		return
	}

	w, err := loadOrCreateWallet(*walletPath)
	if err != nil {
		fmt.Printf("%s%sWallet error: %v%s\n", Red, Bold, err, Reset)
		os.Exit(1)
	}
	if err := bc.LoadFromFile(blockchainFile); err != nil {
		fmt.Printf("%s%sBlockchain load error: %v%s\n", Red, Bold, err, Reset)
		os.Exit(1)
	}
	fmt.Printf("%sWallet:%s %s%s%s\n", Yellow, Reset, Green, w.Address, Reset)
	fmt.Printf("%sBalance:%s %s%d%s\n", Yellow, Reset, Green, getBalance(w, &bc), Reset)
	fmt.Printf("%sChain height:%s %s%d%s\n", Yellow, Reset, Magenta, len(bc.Chain)-1, Reset)
}
