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

        while self.running:
            try:
                print_info(f"[DEBUG] Mining worker {task_id} loop start. Fetching latest block from node...")
                # Fetch latest block from node
                response = await connect_to_peer_async(self.node_address, "getlatestblock")
                print_info(f"[DEBUG] Task {task_id}: Raw node response: {repr(response)}")
                if not response:
                    print_error(f"Task {task_id}: Failed to fetch latest block from node.")
                    await asyncio.sleep(5)
                    continue

                # Parse latest block from node response
                lines = response.strip().split('\n')
                if len(lines) >= 2:
                    json_data = lines[1]
                else:
                    json_data = response.strip()
                print_info(f"[DEBUG] Task {task_id}: JSON data for latest block: {json_data}")
                try:
                    block_data = json.loads(json_data)
                    prev_block = Block.from_dict(block_data)
                except Exception as e:
                    print_error(f"Task {task_id}: Error parsing latest block: {e}")
                    await asyncio.sleep(2)
                    continue

                coinbase_tx = Transaction(
                    from_addr="coinbase",
                    to_addr=self.wallet_address,
                    amount=50
                )
                transactions = [coinbase_tx]
                # Use difficulty from previous block
                blockchain = Blockchain()
                blockchain.chain = [prev_block]
                difficulty = blockchain.get_dynamic_difficulty()
                print_info(f"[DEBUG] Difficulty for mining: {difficulty}")
                print_info(f"Task {task_id}: Mining block {prev_block.index + 1} (difficulty: {difficulty})")

                print_info(f"[DEBUG] Starting mine_block for block {prev_block.index + 1}")
                start_time = time.time()
                block, block_attempts = mine_block(prev_block, transactions, difficulty)
                end_time = time.time()
                print_info(f"[DEBUG] mine_block finished. Attempts: {block_attempts}, Time: {end_time - start_time:.2f}s")

                attempts += block_attempts

                print_info(f"[DEBUG] Submitting block {block.index} to node...")
                submit_ok = await self._submit_block(block)
                print_info(f"[DEBUG] Block submission result: {submit_ok}")
                if submit_ok:
                    blocks_found += 1
                    hashrate = block_attempts / (end_time - start_time)
                    print_success(f"Task {task_id}: Block {block.index} found! Hashrate: {hashrate:.1f} H/s")
                    if max_blocks > 0 and blocks_found >= max_blocks:
                        print_info(f"[DEBUG] Task {task_id} reached block limit {max_blocks}")
                        break
                else:
                    print_warning(f"Task {task_id}: Block {block.index} rejected")

                await asyncio.sleep(0.1)
            except Exception as e:
                print_error(f"[DEBUG] Async mining task {task_id} error: {e}")
                await asyncio.sleep(2)
        print_info(f"[DEBUG] Mining worker {task_id} exiting. Total attempts: {attempts}, Blocks found: {blocks_found}")
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
        """Sync only the latest block from node asynchronously"""
        try:
            response = await connect_to_peer_async(self.node_address, "getlatestblock")
            if not response:
                return False

            lines = response.strip().split('\n')
            if len(lines) >= 2:
                json_data = lines[1]
            else:
                json_data = response.strip()

            block_data = json.loads(json_data)
            blockchain.chain = [Block.from_dict(block_data)]
            return True

        except Exception as e:
            print_error(f"Failed to sync latest block: {e}")
            return False

    async def _submit_block(self, block: Block) -> bool:
        """Submit mined block to node asynchronously (two-step protocol)"""
        import asyncio
        import socket
        try:
            host, port_str = self.node_address.rsplit(':', 1)
            port = int(port_str)
            reader, writer = await asyncio.open_connection(host, port)

            # Step 1: Send submitblock command
            writer.write(b"submitblock\n")
            await writer.drain()

            # Step 2: Wait for daemon prompt
            prompt = await reader.readline()
            prompt_str = prompt.decode().strip().lower()
            if not prompt_str.startswith("send block json"):
                print_error(f"Daemon did not prompt for block JSON: {prompt_str}")
                writer.close()
                await writer.wait_closed()
                return False

            # Step 3: Send block JSON
            block_json = json.dumps(block.to_dict()) + "\n"
            writer.write(block_json.encode())
            await writer.drain()

            # Step 4: Read response
            response = await reader.readline()
            response_str = response.decode().strip().lower()

            writer.close()
            await writer.wait_closed()

            return response_str == "ok"
        except Exception as e:
            print_error(f"Failed to submit block: {e}")
            return False


async def start_async_mining(wallet_or_address: str, node_address: str, blocks: int, threads: int) -> bool:
    """Start async mining with the given parameters"""
    try:
        # Determine if wallet_or_address is a file path or address
        from wallet import validate_address, load_or_create_wallet

        wallet_address = None
        wallet = None
        if validate_address(wallet_or_address):
            wallet_address = wallet_or_address
            print_info(f"Using wallet address: {wallet_address}")
        else:
            wallet = load_or_create_wallet(wallet_or_address)
            if not wallet or not wallet.address:
                print_error(f"Failed to load wallet from {wallet_or_address}")
                return False
            wallet_address = wallet.address
            print_info(f"Using wallet from file: {wallet_or_address}")

        # Test node connection before starting miner
        from daemon import connect_to_peer_async
        test_response = await connect_to_peer_async(node_address, "getchain")
        if not test_response:
            print_error(f"Failed to connect to node at {node_address}. Is the daemon running?")
            return False

        # Create async miner
        miner = AsyncMiner(wallet_address, node_address, threads)

        # Setup signal handlers for graceful shutdown
        def signal_handler():
            print_info("Received shutdown signal...")
            asyncio.create_task(miner.stop_mining())

        loop = asyncio.get_running_loop()
        for sig in (signal.SIGINT, signal.SIGTERM):
            try:
                loop.add_signal_handler(sig, signal_handler)
            except NotImplementedError:
                pass  # Not supported on Windows for SIGTERM

        # Start mining
        started = await miner.start_mining(blocks)
        if not started:
            print_error("Failed to start async mining (already running or internal error)")
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
        import traceback
        print_error(f"Async mining error: {e}")
        traceback.print_exc()
        return False


async def mine_forever_async(wallet_or_address: str, node_address: str, threads: int) -> bool:
    """Mine indefinitely using async"""
    return await start_async_mining(wallet_or_address, node_address, 0, threads)


async def mine_blocks_async(wallet_or_address: str, node_address: str, block_count: int, threads: int) -> bool:
    """Mine a specific number of blocks using async"""
    return await start_async_mining(wallet_or_address, node_address, block_count, threads)