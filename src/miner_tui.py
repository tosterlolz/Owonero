"""
Owonero - Miner Terminal User Interface (xmrig-style)
Live mining stats and controls
"""

import os
import sys
import time
import asyncio
from typing import Optional

from utils import print_info, print_success, print_error, YELLOW, GREEN, CYAN, RESET, BOLD
from miner import AsyncMiner

async def miner_tui(wallet_address: str, node_address: str, threads: int = 1, blocks: int = 0):
    miner = AsyncMiner(wallet_address, node_address, threads)
    stats = {'blocks': 0, 'attempts': 0, 'hashrate': 0.0}
    running = True

    async def stats_updater():
        last_attempts = 0
        last_time = time.time()
        while running and miner.running:
            await asyncio.sleep(1)
            elapsed = time.time() - last_time
            if elapsed > 0:
                stats['hashrate'] = (miner.total_attempts - last_attempts) / elapsed
            stats['blocks'] = miner.blocks_found
            stats['attempts'] = miner.total_attempts
            last_attempts = miner.total_attempts
            last_time = time.time()

    async def tui_loop():
        print(BOLD + CYAN + "Owonero Miner TUI" + RESET)
        print(YELLOW + "Press Q to quit." + RESET)
        while running and miner.running:
            print(f"{CYAN}Blocks found:{RESET} {GREEN}{stats['blocks']}{RESET} | "
                  f"{CYAN}Attempts:{RESET} {YELLOW}{stats['attempts']}{RESET} | "
                  f"{CYAN}Hashrate:{RESET} {GREEN}{stats['hashrate']:.1f} H/s{RESET}", end='\r')
            await asyncio.sleep(0.5)
        print()
        print_success(f"Mining stopped. Found {stats['blocks']} blocks, {stats['attempts']} attempts.")

    async def input_loop():
        nonlocal running
        while running and miner.running:
            try:
                if sys.platform == 'win32':
                    import msvcrt
                    if msvcrt.kbhit():
                        ch = msvcrt.getch().decode().lower()
                        if ch == 'q':
                            break
                else:
                    import select
                    if select.select([sys.stdin], [], [], 0.1)[0]:
                        ch = sys.stdin.read(1).lower()
                        if ch == 'q':
                            break
                await asyncio.sleep(0.1)
            except Exception:
                break
        running = False
        await miner.stop_mining()

    await miner.start_mining(blocks)
    await asyncio.gather(stats_updater(), tui_loop(), input_loop())

# Entry point for main.py integration
async def run_miner_tui(wallet_or_address: str, node_address: str, threads: int = 1, blocks: int = 0):
    from wallet import validate_address, load_or_create_wallet
    wallet_address = wallet_or_address
    if not validate_address(wallet_or_address):
        wallet = load_or_create_wallet(wallet_or_address)
        wallet_address = wallet.address
    await miner_tui(wallet_address, node_address, threads, blocks)
