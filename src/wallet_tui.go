package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"strconv"
	"strings"
)

func clearScreen() {
	fmt.Print("\033[2J\033[1;1H") // Clear screen and move cursor to top
}

func printHeader() {
	fmt.Printf("%s%s", Cyan, Bold)
	fmt.Println("╔══════════════════════════════════════════════╗")
	fmt.Println("║              Owonero TUI Wallet              ║")
	fmt.Println("╚══════════════════════════════════════════════╝")
	fmt.Printf("%s", Reset)
}

func printMenu() {
	fmt.Printf("\n%s%sAvailable Options:%s\n", Yellow, Bold, Reset)
	fmt.Printf("  %s1.%s Show wallet address\n", Green, Reset)
	fmt.Printf("  %s2.%s Show public key\n", Green, Reset)
	fmt.Printf("  %s3.%s Check balance\n", Green, Reset)
	fmt.Printf("  %s4.%s Send transaction\n", Green, Reset)
	fmt.Printf("  %s5.%s Transaction history\n", Green, Reset)
	fmt.Printf("  %s6.%s Clear screen\n", Green, Reset)
	fmt.Printf("  %s7.%s Exit wallet\n", Green, Reset)
	fmt.Printf("\n%sChoose an option (1-7): %s", Bold, Reset)
}

func getBalanceFromNode(nodeAddr, address string) (int, error) {
	conn, err := net.Dial("tcp", nodeAddr)
	if err != nil {
		return 0, fmt.Errorf("failed to connect to node: %v", err)
	}
	defer conn.Close()

	fmt.Fprintln(conn, "getwallet")
	fmt.Fprintln(conn, address)

	respReader := bufio.NewReader(conn)
	// Skip node greeting line
	_, _ = respReader.ReadString('\n')

	resp, err := respReader.ReadString('\n')
	if err != nil {
		return 0, fmt.Errorf("failed to read response: %v", err)
	}

	resp = strings.TrimSpace(resp)
	if strings.HasPrefix(resp, "error:") {
		return 0, fmt.Errorf("node error: %s", resp[6:])
	}

	if !strings.HasPrefix(resp, "{") {
		return 0, fmt.Errorf("unexpected response format: %s", resp)
	}

	var walletInfo struct {
		Address string `json:"address"`
		Balance int    `json:"balance"`
	}

	if err := json.Unmarshal([]byte(resp), &walletInfo); err != nil {
		return 0, fmt.Errorf("failed to parse wallet info: %v", err)
	}

	return walletInfo.Balance, nil
}

func sendTransactionToNode(nodeAddr string, tx *Transaction) error {
	conn, err := net.Dial("tcp", nodeAddr)
	if err != nil {
		return fmt.Errorf("failed to connect to node: %v", err)
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
		return fmt.Errorf("failed to read response: %v", err)
	}

	resp = strings.TrimSpace(resp)
	if strings.HasPrefix(resp, "error:") {
		return fmt.Errorf("transaction failed: %s", resp[6:])
	}

	return nil
}

func wallet_main(nodeAddr string) {
	walletPath := "wallet.json"
	w, err := loadOrCreateWallet(walletPath)
	if err != nil {
		fmt.Printf("%s%sError loading wallet: %v%s\n", Red, Bold, err, Reset)
		return
	}

	reader := bufio.NewReader(os.Stdin)

	for {
		clearScreen()
		printHeader()

		// Show wallet address prominently
		fmt.Printf("\n%s%sYour Wallet Address:%s\n", Yellow, Bold, Reset)
		fmt.Printf("%s%s%s\n", Green, w.Address, Reset)

		printMenu()

		input, _ := reader.ReadString('\n')
		input = strings.TrimSpace(input)

		switch input {
		case "1":
			fmt.Printf("\n%s%sWallet Address:%s\n", Blue, Bold, Reset)
			fmt.Printf("%s%s%s\n", Green, w.Address, Reset)
			fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
			reader.ReadString('\n')

		case "2":
			fmt.Printf("\n%s%sPublic Key:%s\n", Blue, Bold, Reset)
			fmt.Printf("%s%s%s\n", Green, w.PubKey, Reset)
			fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
			reader.ReadString('\n')

		case "3":
			fmt.Printf("\n%s%sChecking Balance...%s\n", Blue, Bold, Reset)

			balance, err := getBalanceFromNode(nodeAddr, w.Address)
			if err != nil {
				fmt.Printf("%s%sError: %v%s\n", Red, Bold, err, Reset)
			} else {
				fmt.Printf("%s%sBalance: %d OWON%s\n", Green, Bold, balance, Reset)
			}

			fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
			reader.ReadString('\n')

		case "4":
			fmt.Printf("\n%s%sSend Transaction%s\n", Blue, Bold, Reset)

			// Get recipient address
			fmt.Printf("%sRecipient address: %s", Yellow, Reset)
			recipient, _ := reader.ReadString('\n')
			recipient = strings.TrimSpace(recipient)

			if recipient == "" {
				fmt.Printf("%s%sError: Recipient address cannot be empty%s\n", Red, Bold, Reset)
				fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
				reader.ReadString('\n')
				continue
			}

			// Get amount
			fmt.Printf("%sAmount to send: %s", Yellow, Reset)
			amountStr, _ := reader.ReadString('\n')
			amountStr = strings.TrimSpace(amountStr)

			amount, err := strconv.Atoi(amountStr)
			if err != nil || amount <= 0 {
				fmt.Printf("%s%sError: Invalid amount. Must be a positive number.%s\n", Red, Bold, Reset)
				fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
				reader.ReadString('\n')
				continue
			}

			// Check balance before proceeding
			balance, err := getBalanceFromNode(nodeAddr, w.Address)
			if err != nil {
				fmt.Printf("%s%sError checking balance: %v%s\n", Red, Bold, err, Reset)
				fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
				reader.ReadString('\n')
				continue
			}

			if balance < amount {
				fmt.Printf("%s%sError: Insufficient balance. You have %d OWON, trying to send %d OWON.%s\n",
					Red, Bold, balance, amount, Reset)
				fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
				reader.ReadString('\n')
				continue
			}

			// Confirmation
			fmt.Printf("\n%s%sTransaction Details:%s\n", Cyan, Bold, Reset)
			fmt.Printf("  From: %s\n", w.Address)
			fmt.Printf("  To: %s\n", recipient)
			fmt.Printf("  Amount: %d OWON\n", amount)
			fmt.Printf("  Fee: 0 OWON\n")
			fmt.Printf("\n%sConfirm transaction? (y/N): %s", Yellow, Reset)

			confirm, _ := reader.ReadString('\n')
			confirm = strings.TrimSpace(strings.ToLower(confirm))

			if confirm != "y" && confirm != "yes" {
				fmt.Printf("%s%sTransaction cancelled.%s\n", Yellow, Bold, Reset)
				fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
				reader.ReadString('\n')
				continue
			}

			// Create and sign transaction
			tx := &Transaction{
				From:   w.PubKey,
				To:     recipient,
				Amount: amount,
			}

			err = SignTransaction(tx, w.PrivKey)
			if err != nil {
				fmt.Printf("%s%sError signing transaction: %v%s\n", Red, Bold, err, Reset)
				fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
				reader.ReadString('\n')
				continue
			}

			// Send transaction
			fmt.Printf("\n%s%sSending transaction...%s\n", Blue, Bold, Reset)
			err = sendTransactionToNode(nodeAddr, tx)
			if err != nil {
				fmt.Printf("%s%sError: %v%s\n", Red, Bold, err, Reset)
			} else {
				fmt.Printf("%s%sTransaction sent successfully!%s\n", Green, Bold, Reset)
			}

			fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
			reader.ReadString('\n')

		case "5":
			fmt.Printf("\n%s%sTransaction History%s\n", Blue, Bold, Reset)
			fmt.Printf("%s%sFeature coming soon!%s\n", Yellow, Bold, Reset)
			fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
			reader.ReadString('\n')

		case "6":
			// Clear screen is handled at the beginning of the loop
			continue

		case "7":
			fmt.Printf("\n%s%sThank you for using Owonero TUI Wallet!%s\n", Green, Bold, Reset)
			return

		default:
			fmt.Printf("\n%s%sInvalid option. Please choose 1-7.%s\n", Red, Bold, Reset)
			fmt.Printf("\n%sPress Enter to continue...%s", Yellow, Reset)
			reader.ReadString('\n')
		}
	}
}
