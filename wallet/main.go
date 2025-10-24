package main

import (
	"encoding/json"
	"fmt"
	"net"
	"os"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
)

// Wallet stores only address (you can add private key if you want to sign transactions)
type Wallet struct {
	Address string `json:"address"`
	// PrivKey string `json:"privkey,omitempty"` // optional
}

// Transaction represents a simple transaction
type Transaction struct {
	From   string `json:"from"`
	To     string `json:"to"`
	Amount int    `json:"amount"`
}

// Block structure
type Block struct {
	Index        int           `json:"index"`
	Timestamp    string        `json:"timestamp"`
	Transactions []Transaction `json:"transactions"`
	PrevHash     string        `json:"prev_hash"`
	Hash         string        `json:"hash"`
	Nonce        int           `json:"nonce"`
}

// Blockchain - chain of blocks
type Blockchain struct {
	Chain []Block `json:"chain"`
}

// loadOrCreateWallet - if no file exists creates new OWO address...
func loadOrCreateWallet(path string) (Wallet, error) {
	if _, err := os.Stat(path); err == nil {
		data, err := os.ReadFile(path)
		if err != nil {
			return Wallet{}, err
		}
		var w Wallet
		if err := json.Unmarshal(data, &w); err != nil {
			return Wallet{}, err
		}
		return w, nil
	}
	// generate OWO + hex timestamp for uniqueness
	address := fmt.Sprintf("OWO%016x", time.Now().UnixNano())
	w := Wallet{Address: address}
	data, _ := json.MarshalIndent(w, "", "  ")
	if err := os.WriteFile(path, data, 0600); err != nil {
		return Wallet{}, err
	}
	return w, nil
}

// getBalance - counts wallet balance by scanning blockchain
func getBalance(w Wallet, bc *Blockchain) int {
	balance := 0
	for _, blk := range bc.Chain {
		for _, tx := range blk.Transactions {
			if tx.To == w.Address {
				balance += tx.Amount
			}
			if tx.From == w.Address {
				balance -= tx.Amount
			}
		}
	}
	return balance
}

// syncBlockchainFromNode - sync blockchain from daemon node
func syncBlockchainFromNode(nodeAddr string) (*Blockchain, error) {
	conn, err := net.Dial("tcp", nodeAddr)
	if err != nil {
		return nil, fmt.Errorf("cannot connect to node: %v", err)
	}
	defer conn.Close()

	fmt.Fprintf(conn, "getchain\n")
	var response string
	fmt.Fscanf(conn, "%s", &response) // skip the height line

	// scanner := fmt.Sprintf("%s", conn)

	// Simple implementation - in real app you'd use bufio.Scanner
	data := make([]byte, 1024*1024) // 1MB buffer
	n, err := conn.Read(data)
	if err != nil {
		return nil, fmt.Errorf("cannot read blockchain: %v", err)
	}

	var bc Blockchain
	if err := json.Unmarshal(data[:n], &bc); err != nil {
		return nil, fmt.Errorf("cannot parse blockchain: %v", err)
	}

	return &bc, nil
}

func main() {
	a := app.New()
	w := a.NewWindow("Owonero Wallet")

	walletPath := "wallet.json"
	nodeAddr := "owonero.yabai.buzz:6969"

	// Load or create wallet
	wallet, err := loadOrCreateWallet(walletPath)
	if err != nil {
		dialog.ShowError(err, w)
		return
	}

	// UI Elements
	addressLabel := widget.NewLabel("Address: " + wallet.Address)
	balanceLabel := widget.NewLabel("Balance: Loading...")
	nodeEntry := widget.NewEntry()
	nodeEntry.SetText(nodeAddr)
	nodeEntry.SetPlaceHolder("Node address (host:port)")

	refreshBtn := widget.NewButton("Refresh Balance", func() {
		balanceLabel.SetText("Balance: Loading...")

		bc, err := syncBlockchainFromNode(nodeEntry.Text)
		if err != nil {
			dialog.ShowError(fmt.Errorf("Failed to sync: %v", err), w)
			balanceLabel.SetText("Balance: Error")
			return
		}

		balance := getBalance(wallet, bc)
		balanceLabel.SetText(fmt.Sprintf("Balance: %d OWO", balance))
	})

	// Layout
	content := container.NewVBox(
		widget.NewLabel("Owonero Wallet"),
		addressLabel,
		balanceLabel,
		widget.NewLabel("Node:"),
		nodeEntry,
		refreshBtn,
	)

	w.SetContent(content)
	w.Resize(fyne.NewSize(400, 300))

	// Initial balance load
	go func() {
		bc, err := syncBlockchainFromNode(nodeAddr)
		if err != nil {
			balanceLabel.SetText("Balance: Cannot connect to node")
			return
		}
		balance := getBalance(wallet, bc)
		balanceLabel.SetText(fmt.Sprintf("Balance: %d OWO", balance))
	}()

	w.ShowAndRun()
}
