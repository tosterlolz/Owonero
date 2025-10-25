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

// Wallet przechowuje adres, klucz publiczny i prywatny (ECDSA)
type Wallet struct {
	Address string `json:"address"`
	PubKey  string `json:"pubkey"`
	PrivKey string `json:"privkey"`
}

// loadOrCreateWallet - jeśli nie ma pliku tworzy nowy adres OWO...
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
	// generuj adres OWO + hex timestamp dla unikalności
	address := fmt.Sprintf("OWO%016x", time.Now().UnixNano())

	// generuj klucze ECDSA
	priv, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return Wallet{}, fmt.Errorf("nie można wygenerować klucza: %v", err)
	}
	privBytes, err := x509.MarshalECPrivateKey(priv)
	if err != nil {
		return Wallet{}, fmt.Errorf("nie można zserializować klucza prywatnego: %v", err)
	}
	privPem := pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: privBytes})

	pubBytes, err := x509.MarshalPKIXPublicKey(&priv.PublicKey)
	if err != nil {
		return Wallet{}, fmt.Errorf("nie można zserializować klucza publicznego: %v", err)
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

// getBalance - liczy saldo portfela skanując blockchain
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
