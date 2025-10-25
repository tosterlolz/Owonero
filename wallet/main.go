package main

import (
	"bufio"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/sha256"
	"crypto/x509"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"strings"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
)

// Wallet stores address, public and private key (ECDSA)
type Wallet struct {
	Address string `json:"address"`
	PubKey  string `json:"pubkey"`
	PrivKey string `json:"privkey"`
}

// Transaction represents a simple transaction
type Transaction struct {
	From      string `json:"from"`
	To        string `json:"to"`
	Amount    int    `json:"amount"`
	Signature string `json:"signature"`
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
	var w Wallet
	if _, err := os.Stat(path); err == nil {
		data, err := os.ReadFile(path)
		if err != nil {
			return Wallet{}, err
		}
		if err := json.Unmarshal(data, &w); err != nil {
			return Wallet{}, err
		}
	}

	// If wallet is missing address, generate one
	if w.Address == "" {
		w.Address = fmt.Sprintf("OWO%016x", time.Now().UnixNano())
	}

	// If wallet is missing keys, generate new ECDSA key pair
	if w.PubKey == "" || w.PrivKey == "" {
		priv, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
		if err != nil {
			return Wallet{}, fmt.Errorf("failed to generate keys: %v", err)
		}
		privBytes, err := x509.MarshalECPrivateKey(priv)
		if err != nil {
			return Wallet{}, fmt.Errorf("failed to marshal private key: %v", err)
		}
		pubBytes, err := x509.MarshalPKIXPublicKey(&priv.PublicKey)
		if err != nil {
			return Wallet{}, fmt.Errorf("failed to marshal public key: %v", err)
		}
		w.PrivKey = base64.StdEncoding.EncodeToString(privBytes)
		w.PubKey = base64.StdEncoding.EncodeToString(pubBytes)
	}

	// Save wallet to file
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

// SignTransaction - podpisuje transakcję kluczem prywatnym (ECDSA)

func SignTransaction(tx *Transaction, privPem string) error {
	privBytes, err := base64.StdEncoding.DecodeString(privPem)
	if err != nil {
		return fmt.Errorf("cannot decode private key base64: %v", err)
	}
	priv, err := x509.ParseECPrivateKey(privBytes)
	if err != nil {
		return fmt.Errorf("cannot parse private key: %v", err)
	}
	// Hashujemy dane transakcji
	msg := fmt.Sprintf("%s|%s|%d", tx.From, tx.To, tx.Amount)
	hash := sha256.Sum256([]byte(msg))
	r, s, err := ecdsa.Sign(rand.Reader, priv, hash[:])
	if err != nil {
		return fmt.Errorf("sign error: %v", err)
	}
	sigBytes, _ := json.Marshal(struct {
		R string `json:"r"`
		S string `json:"s"`
	}{R: r.Text(16), S: s.Text(16)})
	tx.Signature = hex.EncodeToString(sigBytes)
	return nil
}

// syncBlockchainFromNode - sync blockchain from daemon node
func syncBlockchainFromNode(nodeAddr string) (*Blockchain, error) {
	conn, err := net.Dial("tcp", nodeAddr)
	if err != nil {
		return nil, fmt.Errorf("cannot connect to node: %v", err)
	}
	defer conn.Close()

	fmt.Fprintf(conn, "getchain\n")

	// Use bufio.Reader to read lines until we find JSON
	reader := bufio.NewReader(conn)
	var jsonLine string
	for {
		line, err := reader.ReadString('\n')
		if err != nil {
			return nil, fmt.Errorf("cannot read blockchain: %v", err)
		}
		// Find first line that looks like JSON
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "{") || strings.HasPrefix(trimmed, "[") {
			jsonLine = trimmed
			break
		}
	}

	var bc Blockchain
	if err := json.Unmarshal([]byte(jsonLine), &bc); err != nil {
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
	addressBtn := widget.NewButton("Address: "+wallet.Address, func() {
		w.Clipboard().SetContent(wallet.Address)
		dialog.ShowInformation("Copied", "Wallet address copied to clipboard!", w)
	})
	balanceLabel := widget.NewLabel("Balance: Loading...")
	nodeEntry := widget.NewEntry()
	nodeEntry.SetText(nodeAddr)
	nodeEntry.SetPlaceHolder("Node address (host:port)")

	recipientEntry := widget.NewEntry()
	recipientEntry.SetPlaceHolder("Recipient address (OWO... or PEM)")
	amountEntry := widget.NewEntry()
	amountEntry.SetPlaceHolder("Amount")

	sendBtn := widget.NewButton("Send OWO", func() {
		recipient := recipientEntry.Text
		amountStr := amountEntry.Text
		node := nodeEntry.Text
		if recipient == "" || amountStr == "" {
			dialog.ShowError(fmt.Errorf("recipient and amount required"), w)
			return
		}
		var amount int
		_, err := fmt.Sscanf(amountStr, "%d", &amount)
		if err != nil || amount <= 0 {
			dialog.ShowError(fmt.Errorf("invalid amount"), w)
			return
		}

		// Prepare transaction
		tx := Transaction{
			From:   wallet.PubKey, // wysyłamy PEM klucza publicznego
			To:     recipient,
			Amount: amount,
		}
		// Sign transaction
		if err := SignTransaction(&tx, wallet.PrivKey); err != nil {
			dialog.ShowError(fmt.Errorf("sign error: %v", err), w)
			return
		}

		// Send transaction to node
		go func() {
			conn, err := net.Dial("tcp", node)
			if err != nil {
				dialog.ShowError(fmt.Errorf("cannot connect to node: %v", err), w)
				return
			}
			defer conn.Close()
			fmt.Fprintf(conn, "sendtx\n")
			txJson, _ := json.Marshal(tx)
			fmt.Fprintf(conn, "%s\n", txJson)
			resp, _ := bufio.NewReader(conn).ReadString('\n')
			if strings.HasPrefix(resp, "ok") {
				dialog.ShowInformation("Transaction sent", "Transaction sent successfully!", w)
			} else {
				dialog.ShowError(fmt.Errorf("node error: %s", resp), w)
			}
		}()
	})

	refreshBtn := widget.NewButton("Refresh Balance", func() {
		balanceLabel.SetText("Balance: Loading...")

		bc, err := syncBlockchainFromNode(nodeEntry.Text)
		if err != nil {
			dialog.ShowError(fmt.Errorf("failed to sync: %v", err), w)
			balanceLabel.SetText("Balance: Error")
			return
		}

		balance := getBalance(wallet, bc)
		balanceLabel.SetText(fmt.Sprintf("Balance: %d OWO", balance))
	})

	// Layout
	content := container.NewVBox(
		widget.NewLabel("Owonero Wallet"),
		addressBtn,
		balanceLabel,
		widget.NewLabel("Node:"),
		nodeEntry,
		widget.NewLabel("Send OWO:"),
		recipientEntry,
		amountEntry,
		sendBtn,
		refreshBtn,
	)

	w.SetContent(content)
	w.Resize(fyne.NewSize(400, 400))

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
