#!/usr/bin/env python3
"""
Owonero - A lightweight blockchain cryptocurrency
Main entry point with command-line interface
"""

import argparse
import sys
import os
import asyncio

# Add src directory to path for imports
sys.path.insert(0, os.path.dirname(__file__))

from utils import (
    print_error, print_success, print_info, print_warning,
    RED, GREEN, YELLOW, BLUE, CYAN, MAGENTA, RESET, BOLD,
    VERSION, check_for_updates, BLOCKCHAIN_FILE
)
from blockchain import Blockchain
from wallet import Wallet, load_or_create_wallet, get_balance
from daemon import PeerManager, run_async_daemon, connect_to_peer_async
from miner import start_async_mining
from wallet_tui import wallet_tui_main
from web_stats import start_web_stats_server


def print_ascii_logo():
    """Print the ASCII logo with gradient effect"""
    logo = f"""
⠀⠀⠀⠀⡰⠁⠀⠀⢀⢔⣔⣤⠐⠒⠒⠒⠒⠠⠄⢀⠀⠐⢀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⡐⢀⣾⣷⠪⠑⠛⠛⠛⠂⠠⠶⢶⣿⣦⡀⠀⠈⢐⢠⣑⠤⣀⠀⠀⠀
⠀⢀⡜⠀⢸⠟⢁⠔⠁⠀⠀⠀⠀⠀⠀⠀⠉⠻⢷⠀⠀⠀⡦⢹⣷⣄⠀⢀⣀⡀
⠀⠸⠀⠠⠂⡰⠁⡜⠀⠀⠀⠀⠀⠀⠀⡀⠀⠀⠈⠇⠀⠀⢡⠙⢿⣿⣾⣿⣿⠃
⠀⠀⠠⠁⠰⠁⢠⢀⠀⠀⡄⠀⠀⠀⠀⠁⠀⠀⠀⠀⠀⠀⠀⢢⠀⢉⡻⣿⣇⠀
⠀⠠⠁⠀⡇⠀⡀⣼⠀⢰⡇⠀⠀⠀⠀⢸⠀⠀⠀⠀⠀⠀⠀⢸⣧⡈⡘⣷⠟⠀    _____          ________
⠀⠀⠀⠈⠀⠀⣧⢹⣀⡮⡇⠀⠀⠀⢸⢸⡄⠀⠀⠀⠀⠀⠀⢸⠈⠈⠲⠇⠀⠀  / __ \\ \\        / /  ____|
⠀⢰⠀⢸⢰⢰⠘⠀⢶⠀⢷⡄⠈⠁⡚⡾⢧⢠⡀⢠⠀⠀⠀⢸⡀⠀⠀⠰⠀  | |  | \\ \\  /\\  / /| |__
⣧⠈⡄⠈⣿⡜⢱⣶⣦⠀⠀⢠⠆⠀⣁⣀⠘⢸⠀⢸⠀⡄⠀⠀⡆⠀⠠⡀⠃  | |  | |\\ \\/  \\/ / |  __|
⢻⣷⡡⢣⣿⠃⠘⠿⠏⠀⠀⠀⠂⠀⣿⣿⣿⡇⠀⡀⣰⡗⠄⡀⠰⠀⠀⠀⠀  | |__| | \\  /\\  /  | |____
⠀⠙⢿⣜⢻⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠋⢁⢡⠀⡷⣿⠁⠈⠋⠢⢇⠀⡀⠀   \\_____/   \\/  \\/  |______|
⠀⠀⠈⢻⠀⡆⠀⠀⠀⠀⠀⠀⠀⠀⠐⠆⡘⡇⠀⣼⣿⡇⢀⠀⠀⠀⢱⠁⠀ 							   V.{VERSION}
⠐⢦⣀⠸⡀⢸⣦⣄⡀⠒⠄⠀⠀⠀⢀⣀⣴⠀⣸⣿⣿⠁⣼⢦⠀⠀⠘⠀		
⠀⠀⢎⠳⣇⠀⢿⣿⣿⣶⣤⡶⣾⠿⠋⣁⡆⡰⢿⣿⣿⡜⢣⠀⢆⡄⠇⠀
⠀⠀⠈⡄⠈⢦⡘⡇⠟⢿⠙⡿⢀⠐⠁⢰⡜⠀⠀⠙⢿⡇⠀⡆⠈⡟⠀⠀      
"""

    # Simple color gradient simulation
    lines = logo.strip().split('\n')
    colors = [RED, YELLOW, GREEN, CYAN, BLUE, MAGENTA]

    for i, line in enumerate(lines):
        color = colors[i % len(colors)] if line.strip() else RESET
        print(color + line + RESET)


async def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(
        description='Owonero - A lightweight blockchain cryptocurrency',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  main.py -d -p 6969                         # Start daemon on port 6969
  main.py -m -n localhost:6969 -t 4          # Mine with 4 threads to default wallet
  main.py -m -w OWO123... -n localhost:6969  # Mine to specific address
  main.py -tui                               # Launch wallet TUI
  main.py                                    # Show wallet balance
        """
    )

    # Core options
    parser.add_argument('-d', '--daemon', action='store_true',
                       help='Run as network daemon')
    parser.add_argument('-tui', action='store_true',
                       help='Run wallet in Terminal User Interface mode')
    parser.add_argument('-p', '--port', type=int, default=6969,
                       help='Daemon listening port (default: 6969)')
    parser.add_argument('-web', type=int, default=0,
                       help='Web stats server port (0 = disabled, default: 0)')

    # Wallet options
    parser.add_argument('-w', '--wallet', default='wallet.json',
                       help='Wallet file path (default: wallet.json) OR wallet address to mine to')

    # Mining options
    parser.add_argument('-m', '--mine', action='store_true',
                       help='Start mining blocks')
    parser.add_argument('-b', '--blocks', type=int, default=0,
                       help='Number of blocks to mine (0 = unlimited)')
    parser.add_argument('-t', '--threads', type=int, default=1,
                       help='Number of mining threads (default: 1)')

    # Network options
    parser.add_argument('-n', '--node', default='localhost:6969',
                       help='Node address to connect to (default: localhost:6969)')
    parser.add_argument('--peers', default='',
                       help='Comma-separated list of initial peer addresses')

    # Other options
    parser.add_argument('--no-update', action='store_true',
                       help='Skip automatic update check on startup')
    parser.add_argument('--no-init', action='store_true',
                       help="Don't initialize blockchain.json, rely on syncing")

    args = parser.parse_args()

    # Print logo
    print_ascii_logo()

    # Check for updates (unless disabled)
    if not args.no_update:
        check_for_updates()
    else:
        print_warning("Update check skipped (--no-update flag used)")

    # Handle TUI mode
    if args.tui:
        wallet_tui_main(args.node)
        return

    # Load or initialize blockchain
    blockchain = Blockchain()
    if not args.no_init:
        print_info("Loading blockchain...")
        if not blockchain.load_from_file(BLOCKCHAIN_FILE):
            print_error("Failed to initialize blockchain")
            sys.exit(1)
        print_success(f"Blockchain loaded (height: {blockchain.get_height()})")
    else:
        print_warning("Skipping blockchain initialization (--no-init flag used)")

    # Handle daemon mode
    if args.daemon:
        peer_manager = PeerManager()

        # Add initial peers
        if args.peers:
            peer_list = [peer.strip() for peer in args.peers.split(',')]
            for peer in peer_list:
                if peer:
                    await peer_manager.add_peer(peer)

        # Add node address as peer if specified
        if args.node != 'localhost:6969':
            print_info(f"Adding peer from -n flag: {args.node}")
            await peer_manager.add_peer(args.node)

        print_success(f"Daemon starting with {len(await peer_manager.get_peers())} peers")

        # Start web stats server if requested
        web_server = None
        if args.web > 0:
            web_server = await start_web_stats_server(blockchain, peer_manager, args.web)
            if web_server:
                print_success(f"Web stats server enabled on :{args.web}")
            else:
                print_error("Failed to start web stats server")

        try:
            await run_async_daemon(args.port, blockchain, peer_manager)
        finally:
            if web_server:
                await web_server.stop()

        return

    # Handle mining mode
    if args.mine:
        print_info("Starting mining...")
        if not await start_async_mining(args.wallet, args.node, args.blocks, args.threads):
            print_error("Mining failed")
            sys.exit(1)
        return

    # Default: Show wallet information
    wallet = load_or_create_wallet(args.wallet)
    balance = get_balance(wallet.address, blockchain)

    print()
    print(f"{YELLOW}Wallet:{RESET} {GREEN}{wallet.address}{RESET}")
    print(f"{YELLOW}Balance:{RESET} {GREEN}{balance}{RESET} OWO")
    print(f"{YELLOW}Chain height:{RESET} {CYAN}{blockchain.get_height()}{RESET}")

    # Show network info if connected
    if args.node != 'localhost:6969':
        try:
            response = await connect_to_peer_async(args.node, "getheight")
            if response:
                network_height = int(response)
                print(f"{YELLOW}Network height:{RESET} {CYAN}{network_height}{RESET}")
                if network_height > blockchain.get_height():
                    print_warning("Local blockchain may be behind network")
        except:
            print_warning("Could not connect to network")


if __name__ == '__main__':
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n" + YELLOW + "Interrupted by user" + RESET)
        sys.exit(1)
    except Exception as e:
        print_error(f"Fatal error: {e}")
        sys.exit(1)
