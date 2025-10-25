package main

import (
	"encoding/json"
	"fmt"
	"net/http"
)

func startWebStatsServer(bc *Blockchain, port int) {
	http.HandleFunc("/stats", func(w http.ResponseWriter, r *http.Request) {
		stats := map[string]interface{}{
			"chain_height":       len(bc.Chain) - 1,
			"latest_block_hash":  bc.Chain[len(bc.Chain)-1].Hash,
			"total_transactions": totalTransactions(bc),
			"active_miners":      getActiveMiners(),
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(stats)
	})

	fmt.Printf("Web stats server listening on :%d\n", port)
	http.ListenAndServe(fmt.Sprintf(":%d", port), nil)

}

func getActiveMiners() int {
	return len(miners)
}

func totalTransactions(bc *Blockchain) int {
	total := 0
	for _, blk := range bc.Chain {
		total += len(blk.Transactions)
	}
	return total
}
