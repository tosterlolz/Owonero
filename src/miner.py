"""
Owonero - Mining functionality
Async proof-of-work mining with concurrent task support
"""

import asyncio
import time
import signal
import sys
import json
from typing import List, Optional, Callable, Tuple
import os

from utils import print_error, print_success, print_info, print_warning, BLOCKCHAIN_FILE
from blockchain import Blockchain, Block, Transaction, mine_block
from daemon import connect_to_peer_async


class AsyncMiner:
    """Async proof-of-work miner using asyncio tasks"""

    def __init__(self, wallet_address: str, node_address: str, threads: int = 1):
        self.wallet_address = wallet_address
        self.node_address = node_address
        self.threads = threads
        self.running = False
        self.tasks: List[asyncio.Task] = []
        self.total_attempts = 0
        self.blocks_found = 0
        self.lock = asyncio.Lock()

    async def start_mining(self, blocks: int = 0) -> bool:
        """Start async mining process"""
        if self.running:
            print_warning("Mining already running")
            return False

        self.running = True
        self.tasks = []

        print_info(f"Starting async mining with {self.threads} concurrent tasks...")
        print_info(f"Wallet: {self.wallet_address}")
        print_info(f"Node: {self.node_address}")

        # Start mining tasks
        for i in range(self.threads):
            task = asyncio.create_task(self._mining_worker(i, blocks))
            self.tasks.append(task)

        # Start stats task
        stats_task = asyncio.create_task(self._stats_worker())
        self.tasks.append(stats_task)

        return True

    async def stop_mining(self) -> None:
        """Stop async mining process"""
        if not self.running:
            return

        print_info("Stopping async mining...")
        self.running = False

        # Cancel all tasks
        for task in self.tasks:
            task.cancel()

        # Wait for tasks to complete
        try:
            await asyncio.gather(*self.tasks, return_exceptions=True)
        except asyncio.CancelledError:
            pass

        print_success(f"Async mining stopped. Found {self.blocks_found} blocks, {self.total_attempts} total attempts")

    async def _mining_worker(self, task_id: int, max_blocks: int) -> None:
        """Async mining worker task"""
        blocks_found = 0
        attempts = 0

        try:
            while self.running:
                # Get current blockchain state from node
                blockchain = Blockchain()
                if not await self._sync_blockchain(blockchain):
                    await asyncio.sleep(5)  # Wait before retry
                    continue

                if len(blockchain.chain) == 0:
                    await asyncio.sleep(1)
                    continue

                # Create coinbase transaction
                coinbase_tx = Transaction(
                    from_addr="coinbase",
                    to_addr=self.wallet_address,
                    amount=50  # Block reward
                )

                # Get pending transactions (simplified - just coinbase for now)
                transactions = [coinbase_tx]

                # Get current difficulty
                difficulty = blockchain.get_dynamic_difficulty()

                # Mine block
                prev_block = blockchain.chain[-1]
                print_info(f"Task {task_id}: Mining block {prev_block.index + 1} (difficulty: {difficulty})")

                start_time = time.time()
                block, block_attempts = mine_block(prev_block, transactions, difficulty)
                end_time = time.time()

                attempts += block_attempts

                # Submit block to node
                if await self._submit_block(block):
                    blocks_found += 1
                    hashrate = block_attempts / (end_time - start_time)
                    print_success(f"Task {task_id}: Block {block.index} found! Hashrate: {hashrate:.1f} H/s")

                    # Check if we've reached the block limit
                    if max_blocks > 0 and blocks_found >= max_blocks:
                        break
                else:
                    print_warning(f"Task {task_id}: Block {block.index} rejected")

                # Small delay to prevent overwhelming the node
                await asyncio.sleep(0.1)

        except asyncio.CancelledError:
            pass
        except Exception as e:
            print_error(f"Async mining task {task_id} error: {e}")
        finally:
            async with self.lock:
                self.total_attempts += attempts
                self.blocks_found += blocks_found

    async def _stats_worker(self) -> None:
        """Async statistics reporting task"""
        start_time = time.time()

        try:
            while self.running:
                await asyncio.sleep(60)  # Report every minute

                elapsed = time.time() - start_time
                if elapsed > 0:
                    hashrate = self.total_attempts / elapsed
                    print_info(f"Async mining stats - Blocks: {self.blocks_found}, Attempts: {self.total_attempts}, Hashrate: {hashrate:.1f} H/s")
        except asyncio.CancelledError:
            pass

    async def _sync_blockchain(self, blockchain: Blockchain) -> bool:
        """Sync blockchain from node asynchronously"""
        try:
            response = await connect_to_peer_async(self.node_address, "getchain")
            if not response:
                return False

            # Handle multi-line response from Go daemon (status line + JSON)
            lines = response.strip().split('\n')
            if len(lines) >= 2:
                # Skip the status line and parse the JSON from the second line
                json_data = lines[1]
            else:
                # Fallback for single-line JSON response
                json_data = response.strip()

            chain_data = json.loads(json_data)
            blockchain.chain = [Block.from_dict(block_data) for block_data in chain_data]
            return True

        except Exception as e:
            print_error(f"Failed to sync blockchain: {e}")
            return False

    async def _submit_block(self, block: Block) -> bool:
        """Submit mined block to node asynchronously"""
        try:
            block_json = json.dumps(block.to_dict())
            response = await connect_to_peer_async(self.node_address, f"submitblock\n{block_json}")

            if not response:
                return False

            # Handle multi-line response - check the last line for success
            lines = response.strip().split('\n')
            last_line = lines[-1].lower() if lines else ""

            return last_line == "ok"

        except Exception as e:
            print_error(f"Failed to submit block: {e}")
            return False


async def start_async_mining(wallet_or_address: str, node_address: str, blocks: int, threads: int) -> bool:
    """Start async mining with the given parameters"""
    try:
        # Determine if wallet_or_address is a file path or address
        from wallet import validate_address, load_or_create_wallet
        
        if validate_address(wallet_or_address):
            # It's an address, use it directly
            wallet_address = wallet_or_address
            print_info(f"Using wallet address: {wallet_address}")
        else:
            # It's a file path, load the wallet
            wallet = load_or_create_wallet(wallet_or_address)
            wallet_address = wallet.address
            print_info(f"Using wallet from file: {wallet_or_address}")

        # Create async miner
        miner = AsyncMiner(wallet_address, node_address, threads)

        # Setup signal handlers for graceful shutdown
        def signal_handler():
            print_info("Received shutdown signal...")
            asyncio.create_task(miner.stop_mining())

        # Handle signals in asyncio way
        loop = asyncio.get_running_loop()
        for sig in (signal.SIGINT, signal.SIGTERM):
            loop.add_signal_handler(sig, signal_handler)

        # Start mining
        if not await miner.start_mining(blocks):
            print_error("Failed to start async mining")
            return False

        # Keep main task alive
        try:
            while miner.running:
                await asyncio.sleep(1.0)
        except asyncio.CancelledError:
            pass
        finally:
            await miner.stop_mining()

        return True

    except Exception as e:
        print_error(f"Async mining error: {e}")
        return False


async def mine_forever_async(wallet_or_address: str, node_address: str, threads: int) -> bool:
    """Mine indefinitely using async"""
    return await start_async_mining(wallet_or_address, node_address, 0, threads)


async def mine_blocks_async(wallet_or_address: str, node_address: str, block_count: int, threads: int) -> bool:
    """Mine a specific number of blocks using async"""
    return await start_async_mining(wallet_or_address, node_address, block_count, threads)