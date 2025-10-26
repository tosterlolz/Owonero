package main

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/x509"
	"encoding/json"
	"encoding/pem"
	"fmt"
	"os"
	"time"
)

// Wallet stores address, public key, and private key (ECDSA)
type Wallet struct {
	Address string `json:"address"`
	PubKey  string `json:"pubkey"`
	PrivKey string `json:"privkey"`
}

// loadOrCreateWallet loads wallet from file or creates a new one if not found
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
	// Generate unique address
	address := fmt.Sprintf("OWO%016x", time.Now().UnixNano())

	// Generate ECDSA keys
	priv, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return Wallet{}, fmt.Errorf("failed to generate private key: %v", err)
	}
	privBytes, err := x509.MarshalECPrivateKey(priv)
	if err != nil {
		return Wallet{}, fmt.Errorf("failed to serialize private key: %v", err)
	}
	privPem := pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: privBytes})

	pubBytes, err := x509.MarshalPKIXPublicKey(&priv.PublicKey)
	if err != nil {
		return Wallet{}, fmt.Errorf("failed to serialize public key: %v", err)
	}
	pubPem := pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: pubBytes})

	w := Wallet{
		Address: address,
		PubKey:  string(pubPem),
		PrivKey: string(privPem),
	}
	data, _ := json.MarshalIndent(w, "", "  ")
	if err := os.WriteFile(path, data, 0600); err != nil {
		return Wallet{}, err
	}
	return w, nil
}

// getBalance calculates wallet balance by scanning the blockchain
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

// CreateSignedTransaction creates and signs a transaction from this wallet
func (w *Wallet) CreateSignedTransaction(to string, amount int) (*Transaction, error) {
	tx := &Transaction{
		From:   w.Address,
		To:     to,
		Amount: amount,
	}
	err := SignTransaction(tx, w.PrivKey)
	if err != nil {
		return nil, err
	}
	return tx, nil
}
