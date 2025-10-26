package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"strings"
)

func wallet_main(nodeAddr string) {
	walletPath := "wallet.json"
	w, err := loadOrCreateWallet(walletPath)
	if err != nil {
		fmt.Println("Error loading wallet:", err)
		return
	}

	reader := bufio.NewReader(os.Stdin)
	for {
		fmt.Println("\n==== Owonero TUI Wallet ====")
		fmt.Println("1. Show address")
		fmt.Println("2. Show public key")
		fmt.Println("3. Show balance")
		fmt.Println("4. Send transaction")
		fmt.Println("5. Exit")
		fmt.Print("Select option: ")
		input, _ := reader.ReadString('\n')
		input = strings.TrimSpace(input)
		switch input {
		case "1":
			fmt.Println("Address:", w.Address)
		case "2":
			fmt.Println("Public Key:\n", w.PubKey)
		case "3":
			// Query balance using TCP getwallet
			conn, err := net.Dial("tcp", nodeAddr)
			if err != nil {
				fmt.Println("Error connecting to node:", err)
				break
			}
			defer conn.Close()
			fmt.Fprintln(conn, "getwallet")
			fmt.Fprintln(conn, w.Address)
			respReader := bufio.NewReader(conn)
			// Skip node greeting line
			_, _ = respReader.ReadString('\n')
			resp, err := respReader.ReadString('\n')
			if err != nil {
				fmt.Println("Error reading response:", err)
				break
			}
			resp = strings.TrimSpace(resp)
			if strings.HasPrefix(resp, "error:") {
				fmt.Println(resp)
			} else if strings.HasPrefix(resp, "{") {
				var walletInfo struct {
					Address string `json:"address"`
					Balance int    `json:"balance"`
				}
				if err := json.Unmarshal([]byte(resp), &walletInfo); err != nil {
					fmt.Println("Error parsing wallet info:", err)
					fmt.Println("Raw response:", resp)
				} else {
					fmt.Printf("Address: %s\nBalance: %d\n", walletInfo.Address, walletInfo.Balance)
				}
			} else {
				fmt.Println("Unexpected response:", resp)
			}
		case "4":
			fmt.Print("Recipient address: ")
			recipient, _ := reader.ReadString('\n')
			recipient = strings.TrimSpace(recipient)
			fmt.Print("Amount: ")
			var amount int
			fmt.Scanf("%d\n", &amount)
			// Set From to public key PEM for node signature verification
			tx := &Transaction{
				From:   w.PubKey,
				To:     recipient,
				Amount: amount,
			}
			err := SignTransaction(tx, w.PrivKey)
			if err != nil {
				fmt.Println("Error creating transaction:", err)
				continue
			}
			fmt.Println("Signed transaction:")
			fmt.Printf("%+v\n", tx)
			// Submit transaction to node
			conn, err := net.Dial("tcp", nodeAddr)
			if err != nil {
				fmt.Println("Error connecting to node:", err)
				continue
			}
			defer conn.Close()
			fmt.Fprintln(conn, "sendtx")
			txJson, _ := json.Marshal(tx)
			fmt.Fprintln(conn, string(txJson))
			respReader := bufio.NewReader(conn)
			// Skip node greeting
			_, _ = respReader.ReadString('\n')
			resp, err := respReader.ReadString('\n')
			if err != nil {
				fmt.Println("Error reading response:", err)
				continue
			}
			resp = strings.TrimSpace(resp)
			fmt.Println("Node response:", resp)
		case "5":
			fmt.Println("Exiting wallet.")
			return
		default:
			fmt.Println("Invalid option.")
		}
	}
}
