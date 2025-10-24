package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"sync/atomic"
	"time"
)

// Transaction reprezentuje prostą transakcję
type Transaction struct {
	From   string `json:"from"`
	To     string `json:"to"`
	Amount int    `json:"amount"`
}

// Block struktura bloku
type Block struct {
	Index        int           `json:"index"`
	Timestamp    string        `json:"timestamp"`
	Transactions []Transaction `json:"transactions"`
	PrevHash     string        `json:"prev_hash"`
	Hash         string        `json:"hash"`
	Nonce        int           `json:"nonce"`
}

// Blockchain - łańcuch bloków
type Blockchain struct {
	Chain []Block `json:"chain"`
}

// calculateHash liczy SHA256 bloku (zawiera transakcje)
func calculateHash(b Block) string {
	txBytes, _ := json.Marshal(b.Transactions)
	record := fmt.Sprintf("%d%s%s%s%d", b.Index, b.Timestamp, string(txBytes), b.PrevHash, b.Nonce)
	sum := sha256.Sum256([]byte(record))
	return hex.EncodeToString(sum[:])
}

// createGenesisBlock - genesis
func createGenesisBlock() Block {
	g := Block{
		Index:        0,
		Timestamp:    time.Now().UTC().Format(time.RFC3339),
		Transactions: []Transaction{{From: "genesis", To: "network", Amount: 0}},
		PrevHash:     "",
		Nonce:        0,
	}
	g.Hash = calculateHash(g)
	return g
}

// mineBlock - proof-of-work: hash musi zaczynać się od difficulty zer
// If attemptsPtr != nil, the function will atomically increment *attemptsPtr for each hash attempt.
func mineBlock(prev Block, txs []Transaction, difficulty int, attemptsPtr *int64) Block {
	targetPrefix := ""
	for i := 0; i < difficulty; i++ {
		targetPrefix += "0"
	}

	var b Block
	nonce := 0
	for {
		b = Block{
			Index:        prev.Index + 1,
			Timestamp:    time.Now().UTC().Format(time.RFC3339),
			Transactions: txs,
			PrevHash:     prev.Hash,
			Nonce:        nonce,
		}
		h := calculateHash(b)
		if attemptsPtr != nil {
			atomic.AddInt64(attemptsPtr, 1)
		}
		if len(h) >= difficulty && h[:difficulty] == targetPrefix {
			b.Hash = h
			break
		}
		nonce++
	}
	return b
}

// validateBlock - sprawdza poprawność: prevHash, hash, index, PoW
func (bc *Blockchain) validateBlock(b Block, difficulty int) bool {
	last := bc.Chain[len(bc.Chain)-1]
	if b.PrevHash != last.Hash {
		return false
	}
	if calculateHash(b) != b.Hash {
		return false
	}
	if b.Index != last.Index+1 {
		return false
	}
	// check PoW: hash must start with difficulty zeros
	targetPrefix := ""
	for i := 0; i < difficulty; i++ {
		targetPrefix += "0"
	}
	if len(b.Hash) < difficulty || b.Hash[:difficulty] != targetPrefix {
		return false
	}
	return true
}

// AddBlock - dodaje blok jeżeli walidacja przejdzie
func (bc *Blockchain) AddBlock(b Block, difficulty int) bool {
	if bc.validateBlock(b, difficulty) {
		bc.Chain = append(bc.Chain, b)
		return true
	}
	return false
}

// SaveToFile - zapisuje blockchain do pliku JSON
func (bc *Blockchain) SaveToFile(path string) error {
	data, err := json.MarshalIndent(bc, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, data, 0644)
}

// LoadFromFile - ładuje blockchain z pliku JSON; jeśli brak pliku tworzy genesis
func (bc *Blockchain) LoadFromFile(path string) error {
	if _, err := os.Stat(path); os.IsNotExist(err) {
		// nowy blockchain z genesis
		bc.Chain = []Block{createGenesisBlock()}
		return bc.SaveToFile(path)
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	var tmp Blockchain
	if err := json.Unmarshal(data, &tmp); err != nil {
		return err
	}
	// dodatkowa kontrola: jeśli pusty -> genesis
	if len(tmp.Chain) == 0 {
		tmp.Chain = []Block{createGenesisBlock()}
	}
	bc.Chain = tmp.Chain
	return nil
}
