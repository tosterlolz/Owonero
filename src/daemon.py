"""
Owonero - Network Daemon
Async peer-to-peer networking and block synchronization
"""

import asyncio
import json
import logging
from typing import List, Dict, Set, Optional, Tuple
import random

from utils import print_error, print_success, print_info, print_warning
from blockchain import Blockchain, Block, Transaction, mine_block
from wallet import Wallet, get_wallet_info

# Set up logging for asyncio
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class PeerManager:
    """Manages peer connections and addresses"""

    def __init__(self):
        self.peers: Set[str] = set()
        self.lock = asyncio.Lock()

    async def add_peer(self, address: str) -> bool:
        """Add a peer address"""
        async with self.lock:
            if address and address not in self.peers:
                self.peers.add(address)
                return True
        return False

    async def remove_peer(self, address: str) -> bool:
        """Remove a peer address"""
        async with self.lock:
            if address in self.peers:
                self.peers.remove(address)
                return True
        return False

    async def get_peers(self) -> List[str]:
        """Get list of all peers"""
        async with self.lock:
            return list(self.peers)

    async def get_random_peer(self) -> Optional[str]:
        """Get a random peer address"""
        peers = await self.get_peers()
        if not peers:
            return None
        return random.choice(peers)


class AsyncDaemon:
    """Async network daemon for peer-to-peer communication"""

    def __init__(self, port: int, blockchain: Blockchain, peer_manager: PeerManager):
        self.port = port
        self.blockchain = blockchain
        self.peer_manager = peer_manager
        self.server = None
        self.running = False
        self.tasks: List[asyncio.Task] = []

    async def start(self) -> bool:
        """Start the async daemon"""
        try:
            self.server = await asyncio.start_server(
                self._handle_connection,
                '0.0.0.0',
                self.port
            )

            self.running = True

            # Start background sync task
            sync_task = asyncio.create_task(self._periodic_sync())
            self.tasks.append(sync_task)

            addr = self.server.sockets[0].getsockname()
            print_success(f"Async daemon started on {addr}")
            return True

        except Exception as e:
            print_error(f"Failed to start async daemon: {e}")
            return False

    async def stop(self) -> None:
        """Stop the async daemon"""
        self.running = False

        # Cancel all tasks
        for task in self.tasks:
            task.cancel()

        # Wait for tasks to complete
        if self.tasks:
            await asyncio.gather(*self.tasks, return_exceptions=True)

        # Close server
        if self.server:
            self.server.close()
            await self.server.wait_closed()

        print_info("Async daemon stopped")

    async def _handle_connection(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        """Handle an async client connection"""
        addr = writer.get_extra_info('peername')
        print_info(f"New async connection from {addr[0]}:{addr[1]}")

        try:
            buffer = ""
            while self.running:
                try:
                    # Read data with timeout
                    data = await asyncio.wait_for(reader.read(4096), timeout=30.0)
                    if not data:
                        break

                    buffer += data.decode('utf-8', errors='ignore')

                    # Process complete lines
                    while '\n' in buffer:
                        line, buffer = buffer.split('\n', 1)
                        line = line.strip()

                        if line:
                            # Special handling for submitblock
                            if line.lower() == "submitblock":
                                writer.write(b"Send block JSON on next line\n")
                                await writer.drain()
                                # Wait for block JSON
                                block_json_line = None
                                # If buffer already has a line, use it
                                if '\n' in buffer:
                                    block_json_line, buffer = buffer.split('\n', 1)
                                    block_json_line = block_json_line.strip()
                                else:
                                    block_json_line = (await reader.readline()).decode().strip()
                                try:
                                    block_data = json.loads(block_json_line)
                                    block = Block.from_dict(block_data)
                                    difficulty = self.blockchain.get_dynamic_difficulty()
                                    if self.blockchain.add_block(block, difficulty):
                                        writer.write(b"ok\n")
                                        await writer.drain()
                                    else:
                                        writer.write(b"error: block validation failed\n")
                                        await writer.drain()
                                except Exception as e:
                                    writer.write(f"error: invalid block json {e}\n".encode())
                                    await writer.drain()
                                continue

                            response = await self._process_command(line)
                            writer.write(response.encode())
                            await writer.drain()

                except asyncio.TimeoutError:
                    continue
                except Exception as e:
                    print_error(f"Receive error from {addr}: {e}")
                    break

        except Exception as e:
            print_error(f"Connection handler error from {addr}: {e}")
        finally:
            writer.close()
            await writer.wait_closed()

    async def _process_command(self, command: str) -> str:
        """Process a command from a client"""
        try:
            parts = command.split()
            if not parts:
                return "error: empty command\n"

            cmd = parts[0].lower()

            if cmd == "mineractive":
                print_info("Miner active")
                return "ok\n"

            elif cmd == "getchain":
                chain_data = [block.to_dict() for block in self.blockchain.chain]
                response = json.dumps(chain_data) + "\n"
                return response

            elif cmd == "getlatestblock":
                # Return only the latest block as JSON
                if len(self.blockchain.chain) == 0:
                    return "error: chain empty\n"
                latest_block = self.blockchain.chain[-1]
                response = json.dumps(latest_block.to_dict()) + "\n"
                return response

            elif cmd == "getheight":
                height = str(self.blockchain.get_height()) + "\n"
                return height

            elif cmd == "submitblock":
                return "Send block JSON on next line\n"

            elif cmd == "sendtx":
                return "Send transaction JSON on next line\n"

            elif cmd == "getblocks":
                if len(parts) != 3:
                    return "error: usage: getblocks START END\n"

                try:
                    start = int(parts[1])
                    end = int(parts[2])
                    blocks = self.blockchain.get_blocks_range(start, end)
                    if blocks:
                        block_data = [block.to_dict() for block in blocks]
                        response = json.dumps(block_data) + "\n"
                        return response
                    else:
                        return "error: invalid range\n"
                except ValueError:
                    return "error: invalid block indices\n"

            elif cmd == "addpeer":
                if len(parts) < 2:
                    return "error: usage: addpeer ADDRESS\n"

                peer_addr = parts[1]
                if await self.peer_manager.add_peer(peer_addr):
                    return "ok\n"
                else:
                    return "error: peer already exists\n"

            elif cmd == "removepeer":
                if len(parts) < 2:
                    return "error: usage: removepeer ADDRESS\n"

                peer_addr = parts[1]
                if await self.peer_manager.remove_peer(peer_addr):
                    return "ok\n"
                else:
                    return "error: peer not found\n"

            elif cmd == "getpeers":
                peers = await self.peer_manager.get_peers()
                response = json.dumps(peers) + "\n"
                return response

            elif cmd == "getwallet":
                if len(parts) < 2:
                    return "error: usage: getwallet ADDRESS\n"

                wallet_addr = parts[1]
                wallet_info = get_wallet_info(wallet_addr, self.blockchain)
                if wallet_info:
                    response = json.dumps(wallet_info) + "\n"
                    return response
                else:
                    return "error: wallet not found\n"

            elif cmd == "sync":
                asyncio.create_task(self._sync_with_peers())
                return "sync initiated\n"

            else:
                return f"error: unknown command '{cmd}'\n"

        except Exception as e:
            print_error(f"Command processing error: {e}")
            return "error: internal server error\n"

    async def _periodic_sync(self) -> None:
        """Periodic blockchain sync with peers"""
        while self.running:
            try:
                await asyncio.sleep(300)  # Sync every 5 minutes
                await self._sync_with_peers()
            except asyncio.CancelledError:
                break
            except Exception as e:
                print_error(f"Periodic sync error: {e}")

    async def _sync_with_peers(self) -> None:
        """Sync blockchain with peers"""
        print_info("Starting async blockchain sync...")

        peers = await self.peer_manager.get_peers()
        if not peers:
            print_warning("No peers available for sync")
            return

        for peer_addr in peers:
            try:
                print_info(f"Syncing with peer: {peer_addr}")
                if await self._sync_with_peer(peer_addr):
                    print_success(f"Successfully synced with {peer_addr}")
                    break
            except Exception as e:
                print_error(f"Failed to sync with {peer_addr}: {e}")

    async def _sync_with_peer(self, peer_addr: str) -> bool:
        """Sync with a specific peer"""
        try:
            host, port_str = peer_addr.rsplit(':', 1)
            port = int(port_str)

            reader, writer = await asyncio.open_connection(host, port)

            try:
                # Get peer height
                writer.write(b"getheight\n")
                await writer.drain()

                height_data = await reader.readline()
                peer_height = int(height_data.decode().strip())

                local_height = self.blockchain.get_height()

                if peer_height <= local_height:
                    writer.close()
                    await writer.wait_closed()
                    return True  # Already up to date

                print_info(f"Peer height: {peer_height}, local height: {local_height}")

                # Request missing blocks
                writer.write(f"getblocks {local_height + 1} {peer_height}\n".encode())
                await writer.drain()

                blocks_data = await reader.read(65536)
                blocks_json = blocks_data.decode()

                try:
                    block_data = json.loads(blocks_json)
                    for block_dict in block_data:
                        block = Block.from_dict(block_dict)
                        difficulty = self.blockchain.get_dynamic_difficulty()
                        if self.blockchain.add_block(block, difficulty):
                            print_success(f"Synced block {block.index}")
                        else:
                            print_error(f"Failed to validate block {block.index}")
                            return False

                    # Save blockchain
                    from utils import BLOCKCHAIN_FILE
                    self.blockchain.save_to_file(BLOCKCHAIN_FILE)

                except json.JSONDecodeError:
                    print_error("Invalid block data received")
                    return False

            finally:
                writer.close()
                await writer.wait_closed()

            return True

        except Exception as e:
            print_error(f"Async sync error: {e}")
            return False


async def run_async_daemon(port: int, blockchain: Blockchain, peer_manager: PeerManager) -> None:
    """Run the async network daemon"""
    daemon = AsyncDaemon(port, blockchain, peer_manager)

    if not await daemon.start():
        return

    try:
        # Keep daemon running
        while daemon.running:
            await asyncio.sleep(1.0)
    except KeyboardInterrupt:
        print_info("Shutting down async daemon...")
    finally:
        await daemon.stop()


async def connect_to_peer_async(peer_addr: str, command: str) -> Optional[str]:
    """Connect to a peer and send a command asynchronously"""
    try:
        host, port_str = peer_addr.rsplit(':', 1)
        port = int(port_str)

        reader, writer = await asyncio.open_connection(host, port)

        try:
            writer.write(f"{command}\n".encode())
            await writer.drain()

            response_data = await reader.read(65536)
            response = response_data.decode().strip()
            print_info(f"[DEBUG] connect_to_peer_async raw response from {peer_addr}: {repr(response)}")
            return response

        finally:
            writer.close()
            await writer.wait_closed()

    except Exception as e:
        print_error(f"Failed to connect to peer {peer_addr}: {e}")
        return None