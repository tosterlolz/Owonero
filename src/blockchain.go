package main

import (
	"crypto/ecdsa"
	"crypto/rand"
	"crypto/sha256"
	"crypto/x509"
	"encoding/hex"
	"encoding/json"
	"encoding/pem"
	"fmt"
	"math/big"
	"os"
	"sync/atomic"
	"time"
)

// Transaction reprezentuje prostą transakcję
type Transaction struct {
	From      string `json:"from"`
	To        string `json:"to"`
	Amount    int    `json:"amount"`
	Signature string `json:"signature"`
}

// ...istniejące typy Block, Blockchain...

// SignTransaction - podpisuje transakcję kluczem prywatnym (ECDSA)
func SignTransaction(tx *Transaction, privPem string) error {
	privBlock, _ := pem.Decode([]byte(privPem))
	if privBlock == nil {
		return fmt.Errorf("nie można zdekodować klucza prywatnego")
	}
	priv, err := x509.ParseECPrivateKey(privBlock.Bytes)
	if err != nil {
		return fmt.Errorf("nie można sparsować klucza prywatnego: %v", err)
	}
	// Hashujemy dane transakcji
	msg := fmt.Sprintf("%s|%s|%d", tx.From, tx.To, tx.Amount)
	hash := sha256.Sum256([]byte(msg))
	r, s, err := ecdsa.Sign(rand.Reader, priv, hash[:])
	if err != nil {
		return fmt.Errorf("błąd podpisywania: %v", err)
	}
	sigBytes, _ := json.Marshal(struct {
		R string `json:"r"`
		S string `json:"s"`
	}{R: r.Text(16), S: s.Text(16)})
	tx.Signature = hex.EncodeToString(sigBytes)
	return nil
}

// VerifyTransactionSignature - weryfikuje podpis transakcji
func VerifyTransactionSignature(tx *Transaction, pubPem string) bool {
	pubBlock, _ := pem.Decode([]byte(pubPem))
	if pubBlock == nil {
		return false
	}
	pubAny, err := x509.ParsePKIXPublicKey(pubBlock.Bytes)
	if err != nil {
		return false
	}
	pub, ok := pubAny.(*ecdsa.PublicKey)
	if !ok {
		return false
	}
	msg := fmt.Sprintf("%s|%s|%d", tx.From, tx.To, tx.Amount)
	hash := sha256.Sum256([]byte(msg))
	// Dekoduj podpis
	sigBytes, err := hex.DecodeString(tx.Signature)
	if err != nil {
		return false
	}
	var sig struct {
		R string `json:"r"`
		S string `json:"s"`
	}
	if err := json.Unmarshal(sigBytes, &sig); err != nil {
		return false
	}
	r := new(big.Int)
	s := new(big.Int)
	r.SetString(sig.R, 16)
	s.SetString(sig.S, 16)
	return ecdsa.Verify(pub, hash[:], r, s)
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
		Timestamp:    "2025-10-11T00:00:00Z", // Fixed timestamp for all nodes
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
func (bc *Blockchain) validateBlock(b Block, difficulty int, skipPow bool) bool {
	if len(bc.Chain) == 0 {
		// Genesis block validation
		if b.Index != 0 {
			fmt.Printf("Genesis block validation failed: Index must be 0, got %d\n", b.Index)
			return false
		}
		if b.PrevHash != "" {
			fmt.Printf("Genesis block validation failed: PrevHash must be empty, got %s\n", b.PrevHash)
			return false
		}
		if calculateHash(b) != b.Hash {
			fmt.Printf("Genesis block validation failed: Hash mismatch (calculated %s, stored %s)\n", calculateHash(b), b.Hash)
			return false
		}
		return true
	}

	last := bc.Chain[len(bc.Chain)-1]
	if b.PrevHash != last.Hash {
		fmt.Printf("Block %d validation failed: PrevHash mismatch (expected %s, got %s)\n", b.Index, last.Hash, b.PrevHash)
		return false
	}
	if calculateHash(b) != b.Hash {
		fmt.Printf("Block %d validation failed: Hash mismatch (calculated %s, stored %s)\n", b.Index, calculateHash(b), b.Hash)
		return false
	}
	if b.Index != last.Index+1 {
		fmt.Printf("Block %d validation failed: Index mismatch (expected %d, got %d)\n", b.Index, last.Index+1, b.Index)
		return false
	}
	if !skipPow {
		// check PoW: hash must start with difficulty zeros
		targetPrefix := ""
		for i := 0; i < difficulty; i++ {
			targetPrefix += "0"
		}
		if len(b.Hash) < difficulty || b.Hash[:difficulty] != targetPrefix {
			return false
		}
	}
	return true
}

// AddBlock - dodaje blok jeżeli walidacja przejdzie
func (bc *Blockchain) AddBlock(b Block, difficulty int) bool {
	return bc.AddBlockSkipPow(b, difficulty, false)
}

// AddBlockSkipPow - dodaje blok z opcjonalnym pominięciem sprawdzania PoW
func (bc *Blockchain) AddBlockSkipPow(b Block, difficulty int, skipPow bool) bool {
	if bc.validateBlock(b, difficulty, skipPow) {
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
