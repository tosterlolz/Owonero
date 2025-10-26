package main

import (
	"crypto/ecdsa"
	crand "crypto/rand"
	"crypto/sha256"
	"crypto/sha3"
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

// Dynamic difficulty adjustment
func (bc *Blockchain) GetDynamicDifficulty(targetBlockTime int) int {
	minDifficulty := 1
	maxDifficulty := 7 // Lower max difficulty for easier mining
	window := 10       // Number of blocks to average
	if len(bc.Chain) <= window {
		return minDifficulty
	}
	latest := bc.Chain[len(bc.Chain)-1]
	prev := bc.Chain[len(bc.Chain)-window]
	tLatest, _ := time.Parse(time.RFC3339, latest.Timestamp)
	tPrev, _ := time.Parse(time.RFC3339, prev.Timestamp)
	avgBlockTime := int(tLatest.Sub(tPrev).Seconds()) / window
	diff := bc.Chain[len(bc.Chain)-1].Index // Use last block's difficulty if stored
	if avgBlockTime < targetBlockTime {
		diff++
	} else if avgBlockTime > targetBlockTime {
		diff--
	}
	if diff < minDifficulty {
		diff = minDifficulty
	}
	if diff > maxDifficulty {
		diff = maxDifficulty
	}
	return diff
}

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
	r, s, err := ecdsa.Sign(crand.Reader, priv, hash[:])
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

// BlockForHash - struct for calculating hash without the hash field
type BlockForHash struct {
	Index        int           `json:"index"`
	Timestamp    string        `json:"timestamp"`
	Transactions []Transaction `json:"transactions"`
	PrevHash     string        `json:"prev_hash"`
	Nonce        int           `json:"nonce"`
}

// calculateHash liczy SHA256 bloku (zawiera transakcje)
func calculateHash(b Block) string {
	// Use rx/owo PoW logic for hash calculation
	blockForHash := BlockForHash{
		Index:        b.Index,
		Timestamp:    b.Timestamp,
		Transactions: b.Transactions,
		PrevHash:     b.PrevHash,
		Nonce:        b.Nonce,
	}
	blockBytes, _ := json.Marshal(blockForHash)
	memSize := 1024 * 1024 // 1MB buffer, must match mineBlock
	mem := make([]byte, memSize)
	// Deterministic memory buffer: seed with block index and prev hash
	seed := sha256.Sum256([]byte(fmt.Sprintf("%d%s", b.Index, b.PrevHash)))
	for i := 0; i < memSize; i++ {
		mem[i] = seed[i%len(seed)]
	}
	acc := uint64(b.Nonce)
	for i := 0; i < 8; i++ {
		idx := (b.Nonce*31 + i*7919) % memSize
		acc ^= uint64(mem[idx]) << (i * 8)
	}
	puzzle := (b.Nonce ^ len(blockBytes)) + int(acc&0xFFFF)
	hashInput := append(blockBytes, mem[(b.Nonce*13)%memSize])
	hashInput = append(hashInput, byte(puzzle&0xFF))
	for i := 0; i < 8; i++ {
		hashInput = append(hashInput, byte((acc>>(i*8))&0xFF))
	}
	h := sha3.Sum256(hashInput)
	return hex.EncodeToString(h[:])
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

// mineBlock - optimized rx/owo PoW: combines SHA3, random memory, and math puzzle
func mineBlock(prev Block, txs []Transaction, difficulty int, attemptsPtr *int64) Block {
	targetPrefix := ""
	for i := 0; i < difficulty; i++ {
		targetPrefix += "0"
	}

	var b Block
	nonce := 0
	memSize := 1024 * 1024 // 1MB buffer for GPU mining
	mem := make([]byte, memSize)

	// Pre-calculate block data outside the loop
	b.Index = prev.Index + 1
	b.Timestamp = time.Now().UTC().Format(time.RFC3339)
	b.Transactions = txs
	b.PrevHash = prev.Hash

	// Seed memory buffer once per block (optimized seeding)
	seed := sha256.Sum256([]byte(fmt.Sprintf("%d%s", b.Index, b.PrevHash)))
	// Fill memory in chunks for better performance
	for i := 0; i < memSize; i += 32 {
		end := i + 32
		if end > memSize {
			end = memSize
		}
		copy(mem[i:end], seed[:end-i])
	}

	// Pre-marshal block data (without nonce)
	blockForHash := BlockForHash{
		Index:        b.Index,
		Timestamp:    b.Timestamp,
		Transactions: b.Transactions,
		PrevHash:     b.PrevHash,
		// Nonce will be set per attempt
	}
	blockBytesBase, _ := json.Marshal(blockForHash)

	// Pre-allocate hash input buffer to avoid repeated allocations
	maxInputSize := len(blockBytesBase) + 1 + 8 + 1 // blockBytes + mem byte + puzzle byte + acc bytes
	hashInput := make([]byte, 0, maxInputSize)

	for {
		b.Nonce = nonce

		// Update blockForHash with current nonce
		blockForHash.Nonce = nonce
		blockBytes, _ := json.Marshal(blockForHash)

		// rx/owo: optimized memory access pattern
		acc := uint64(nonce)
		// Pre-compute base index to reduce calculations
		baseIdx := nonce * 31 % memSize
		step := 7919 % memSize

		for i := 0; i < 8; i++ {
			idx := (baseIdx + i*step) % memSize
			acc ^= uint64(mem[idx]) << (i * 8)
		}
		puzzle := (nonce ^ len(blockBytes)) + int(acc&0xFFFF)

		// Build hash input efficiently
		hashInput = hashInput[:0] // reset length, keep capacity
		hashInput = append(hashInput, blockBytes...)
		hashInput = append(hashInput, mem[(nonce*13)%memSize])
		hashInput = append(hashInput, byte(puzzle&0xFF))
		// Add acc as 8 bytes (more efficient than byte-by-byte)
		hashInput = append(hashInput, byte(acc), byte(acc>>8), byte(acc>>16), byte(acc>>24),
			byte(acc>>32), byte(acc>>40), byte(acc>>48), byte(acc>>56))

		h := sha3.Sum256(hashInput)

		if attemptsPtr != nil {
			atomic.AddInt64(attemptsPtr, 1)
		}

		// Check if hash meets difficulty (check raw bytes for better performance)
		valid := true
		if difficulty > 0 {
			// Check bytes directly (each byte represents 2 hex chars)
			for i := 0; i < (difficulty+1)/2 && i < 32; i++ {
				b := h[i]
				if difficulty > i*2 && b>>4 != 0 { // Check high nibble
					valid = false
					break
				}
				if difficulty > i*2+1 && (b&0x0F) != 0 { // Check low nibble
					valid = false
					break
				}
			}
		}
		if valid {
			b.Hash = hex.EncodeToString(h[:])
			break
		}

		nonce++
	}
	return b
}

// validateBlock - sprawdza poprawność: prevHash, hash, index, PoW (rx/owo)
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
		// check PoW: hash must start with difficulty zeros (optimized check)
		if difficulty > 0 && len(b.Hash) >= difficulty {
			hashBytes := []byte(b.Hash)
			validPow := true
			for i := 0; i < (difficulty+1)/2 && i < len(hashBytes)/2; i++ {
				if difficulty > i*2 && hashBytes[i*2] != '0' {
					validPow = false
					break
				}
				if difficulty > i*2+1 && hashBytes[i*2+1] != '0' {
					validPow = false
					break
				}
			}
			if !validPow {
				return false
			}
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
	// Recalculate hashes to fix any inconsistencies from old hash calculation
	for i := range tmp.Chain {
		tmp.Chain[i].Hash = calculateHash(tmp.Chain[i])
	}
	bc.Chain = tmp.Chain
	return nil
}
