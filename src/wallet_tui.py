"""
Owonero - Terminal User Interface
Interactive wallet management interface
"""

import os
import sys
import time
import asyncio
from typing import Optional, Tuple, Callable

from utils import print_error, print_success, print_info, print_warning, RED, GREEN, YELLOW, BLUE, CYAN, RESET, BOLD
from wallet import Wallet, load_or_create_wallet, get_balance, send_transaction, validate_address
from blockchain import Blockchain
from daemon import connect_to_peer_async


def clear_screen() -> None:
    """Clear the terminal screen"""
    os.system('cls' if os.name == 'nt' else 'clear')


def print_header(title: str) -> None:
    """Print a formatted header"""
    width = 60
    print(BOLD + BLUE + "=" * width + RESET)
    print(BOLD + BLUE + f"{' ' * ((width - len(title)) // 2)}{title}" + RESET)
    print(BOLD + BLUE + "=" * width + RESET)
    print()


def print_menu(options: list) -> None:
    """Print a menu with numbered options"""
    for i, option in enumerate(options, 1):
        print(f"{CYAN}{i}.{RESET} {option}")
    print()


def get_user_input(prompt: str, validator: Optional[Callable[[str], bool]] = None) -> str:
    """Get user input with optional validation"""
    while True:
        try:
            value = input(f"{YELLOW}{prompt}{RESET}").strip()
            if validator and not validator(value):
                print_error("Invalid input. Please try again.")
                continue
            return value
        except KeyboardInterrupt:
            print("\n" + YELLOW + "Use 'q' to quit" + RESET)
            continue
        except EOFError:
            return "q"


def show_wallet_info(wallet: Wallet, blockchain: Blockchain) -> None:
    """Display wallet information"""
    clear_screen()
    print_header("Wallet Information")

    balance = get_balance(wallet.address, blockchain)

    print(f"{BOLD}Address:{RESET} {GREEN}{wallet.address}{RESET}")
    print(f"{BOLD}Balance:{RESET} {GREEN}{balance}{RESET} OWO")
    print(f"{BOLD}Blockchain Height:{RESET} {CYAN}{blockchain.get_height()}{RESET}")
    print()

    # Show recent transactions
    print(BOLD + "Recent Transactions:" + RESET)
    transactions = []

    for block in blockchain.chain[-10:]:  # Last 10 blocks
        for tx in block.transactions:
            if tx.from_addr == wallet.address or tx.to_addr == wallet.address:
                tx_type = "SENT" if tx.from_addr == wallet.address else "RECEIVED"
                color = RED if tx_type == "SENT" else GREEN
                transactions.append({
                    'block': block.index,
                    'type': tx_type,
                    'amount': tx.amount,
                    'address': tx.to_addr if tx.from_addr == wallet.address else tx.from_addr,
                    'color': color
                })

    if transactions:
        print(f"{'Block':<6} {'Type':<8} {'Amount':<8} {'Address':<42}")
        print("-" * 70)
        for tx in transactions[-10:]:  # Show last 10 transactions
            print(f"{tx['block']:<6} {BOLD}{tx['color']}{tx['type']:<8}{RESET} {tx['amount']:<8} {tx['address']:<42}")
    else:
        print("No transactions found")

    print()
    input(YELLOW + "Press Enter to continue..." + RESET)


def send_funds(wallet: Wallet, blockchain: Blockchain, node_address: str) -> None:
    """Send funds to another address"""
    clear_screen()
    print_header("Send Funds")

    balance = get_balance(wallet.address, blockchain)
    print(f"{BOLD}Your Balance:{RESET} {GREEN}{balance}{RESET} OWO")
    print()

    if balance <= 0:
        print_error("Insufficient balance to send funds")
        input(YELLOW + "Press Enter to continue..." + RESET)
        return

    # Get recipient address
    def validate_addr(addr: str) -> bool:
        return validate_address(addr) and addr != wallet.address

    recipient = get_user_input("Recipient address (OWO...): ", validate_addr)
    if recipient.lower() == 'q':
        return

    # Get amount
    def validate_amount(amt: str) -> bool:
        try:
            amount = int(amt)
            return 0 < amount <= balance
        except ValueError:
            return False

    amount_str = get_user_input(f"Amount to send (max: {balance}): ", validate_amount)
    if amount_str.lower() == 'q':
        return

    amount = int(amount_str)

    # Confirm transaction
    print()
    print_warning("Transaction Details:")
    print(f"  From: {wallet.address}")
    print(f"  To: {recipient}")
    print(f"  Amount: {amount} OWO")
    print()

    confirm = get_user_input("Confirm transaction? (y/N): ").lower()
    if confirm not in ['y', 'yes']:
        print_info("Transaction cancelled")
        time.sleep(1)
        return

    # Send transaction
    print_info("Sending transaction...")
    if send_transaction(wallet, recipient, amount, blockchain):
        print_success("Transaction sent successfully!")
    else:
        print_error("Transaction failed")

    input(YELLOW + "Press Enter to continue..." + RESET)


def show_network_status(node_address: str) -> None:
    """Show network status"""
    clear_screen()
    print_header("Network Status")

    print(f"{BOLD}Connected Node:{RESET} {CYAN}{node_address}{RESET}")
    print()

    try:
        # Get height
        response = asyncio.run(connect_to_peer_async(node_address, "getheight"))
        if response:
            height = int(response)
            print(f"{BOLD}Network Height:{RESET} {GREEN}{height}{RESET}")
        else:
            print_error("Failed to get network height")

        # Get peers
        response = asyncio.run(connect_to_peer_async(node_address, "getpeers"))
        if response:
            import json
            peers = json.loads(response)
            print(f"{BOLD}Connected Peers:{RESET} {GREEN}{len(peers)}{RESET}")
            if peers:
                print("\nPeers:")
                for peer in peers[:10]:  # Show first 10 peers
                    print(f"  {CYAN}{peer}{RESET}")
                if len(peers) > 10:
                    print(f"  ... and {len(peers) - 10} more")
        else:
            print_error("Failed to get peer list")

    except Exception as e:
        print_error(f"Network error: {e}")

    print()
    input(YELLOW + "Press Enter to continue..." + RESET)


def wallet_tui_main(node_address: str) -> None:
    """Main TUI function"""
    # Load wallet
    wallet_path = os.getenv('WALLET_PATH', 'wallet.json')
    wallet = load_or_create_wallet(wallet_path)

    # Load blockchain
    blockchain = Blockchain()
    blockchain_file = os.getenv('BLOCKCHAIN_FILE', 'blockchain.json')
    if not blockchain.load_from_file(blockchain_file):
        print_error("Failed to load blockchain")
        return

    while True:
        clear_screen()
        print_header("Owonero Wallet")

        balance = get_balance(wallet.address, blockchain)
        print(f"{BOLD}Address:{RESET} {GREEN}{wallet.address}{RESET}")
        print(f"{BOLD}Balance:{RESET} {GREEN}{balance}{RESET} OWO")
        print()

        menu_options = [
            "View Wallet Details",
            "Send Funds",
            "Network Status",
            "Refresh Blockchain",
            "Quit"
        ]

        print_menu(menu_options)

        choice = get_user_input("Select option (1-5): ")

        if choice == '1':
            show_wallet_info(wallet, blockchain)
        elif choice == '2':
            send_funds(wallet, blockchain, node_address)
        elif choice == '3':
            show_network_status(node_address)
        elif choice == '4':
            print_info("Refreshing blockchain...")
            if blockchain.load_from_file(blockchain_file):
                print_success("Blockchain refreshed")
            else:
                print_error("Failed to refresh blockchain")
            time.sleep(2)
        elif choice.lower() in ['5', 'q', 'quit']:
            print_info("Goodbye!")
            break
        else:
            print_error("Invalid option")
            time.sleep(1)


if __name__ == "__main__":
    if len(sys.argv) > 1:
        node_addr = sys.argv[1]
    else:
        node_addr = "localhost:6969"

    try:
        wallet_tui_main(node_addr)
    except KeyboardInterrupt:
        print("\n" + YELLOW + "Exiting..." + RESET)
    except Exception as e:
        print_error(f"TUI error: {e}")