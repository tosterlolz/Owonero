package main

import (
	"encoding/json"
	"fmt"
	"os"
	"time"
)

// Wallet przechowuje tylko adres (możesz dodać klucz prywatny jeśli chcesz podpisywać transakcje)
type Wallet struct {
	Address string `json:"address"`
	// PrivKey string `json:"privkey,omitempty"` // opcjonalnie
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
	w := Wallet{Address: address}
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
