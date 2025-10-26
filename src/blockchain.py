"""
Owonero - Blockchain implementation
Core blockchain data structures and proof-of-work mining
"""

import hashlib
import json
import os
import time
import secrets
from typing import List, Dict, Any, Optional, Tuple, Callable
from dataclasses import dataclass, asdict
import struct

from utils import print_error, print_success, print_info, print_warning, save_json_file, load_json_file, format_timestamp


@dataclass
class Transaction:
    """Represents a cryptocurrency transaction"""
    from_addr: str
    to_addr: str
    amount: int
    signature: str = ""

    def to_dict(self) -> dict:
        return {
            'from': self.from_addr,
            'to': self.to_addr,
            'amount': self.amount,
            'signature': self.signature
        }

    @classmethod
    def from_dict(cls, data: dict) -> 'Transaction':
        # Handle both "from" (Go) and "from_addr" (Python) field names
        from_addr = data.get('from_addr') or data.get('from')
        return cls(
            from_addr=str(from_addr),
            to_addr=str(data['to_addr']) if 'to_addr' in data else str(data.get('to', '')),
            amount=int(data['amount']),
            signature=str(data.get('signature', ''))
        )


@dataclass
class Block:
    """Represents a blockchain block"""
    index: int
    timestamp: str
    transactions: List[Transaction]
    prev_hash: str
    hash: str = ""
    nonce: int = 0

    def to_dict(self) -> dict:
        return {
            'index': self.index,
            'timestamp': self.timestamp,
            'transactions': [tx.to_dict() for tx in self.transactions],
            'prev_hash': self.prev_hash,
            'hash': self.hash,
            'nonce': self.nonce
        }

    @classmethod
    def from_dict(cls, data: dict) -> 'Block':
        transactions = [Transaction.from_dict(tx) for tx in data.get('transactions', [])]
        return cls(
            index=int(data['index']),
            timestamp=data['timestamp'],
            transactions=transactions,
            prev_hash=data['prev_hash'],
            hash=data.get('hash', ''),
            nonce=int(data.get('nonce', 0))
        )


class Blockchain:
    """Main blockchain class managing the chain of blocks"""

    def __init__(self):
        self.chain: List[Block] = []

    def get_dynamic_difficulty(self, target_block_time: int = 30) -> int:
        """Calculate dynamic mining difficulty based on recent block times"""
        min_difficulty = 1
        max_difficulty = 3  # Lowered for easier mining
        window = 10

        if len(self.chain) <= window:
            print_info(f"Difficulty: {min_difficulty} (not enough blocks for adjustment)")
            return min_difficulty

        latest = self.chain[-1]
        prev = self.chain[-window]

        try:
            t_latest = time.mktime(time.strptime(latest.timestamp, "%Y-%m-%dT%H:%M:%SZ"))
            t_prev = time.mktime(time.strptime(prev.timestamp, "%Y-%m-%dT%H:%M:%SZ"))
            avg_block_time = int((t_latest - t_prev) / window)
            print_info(f"Average block time: {avg_block_time}s, Target: {target_block_time}s")
        except Exception as e:
            print_warning(f"Difficulty fallback: {min_difficulty} due to error: {e}")
            return min_difficulty

        # Start from a sensible baseline difficulty (not block index).
        # Using the block index as difficulty made difficulty grow unbounded
        # and made mining practically impossible. Start from min_difficulty
        # and adjust by one step based on average block time.
        current_diff = min_difficulty

        if avg_block_time < target_block_time:
            current_diff += 1
        elif avg_block_time > target_block_time:
            current_diff = max(min_difficulty, current_diff - 1)

        final_diff = max(min_difficulty, min(max_difficulty, current_diff))
        print_info(f"Dynamic difficulty set to: {final_diff}")
        return final_diff

    def validate_block(self, block: Block, difficulty: int, skip_pow: bool = False) -> bool:
        """Validate a block's integrity"""
        if len(self.chain) == 0:
            # Genesis block validation
            if block.index != 0:
                print_error(f"Genesis block validation failed: Index must be 0, got {block.index}")
                return False
            if block.prev_hash != "":
                print_error(f"Genesis block validation failed: PrevHash must be empty, got {block.prev_hash}")
                return False
            if calculate_hash(block) != block.hash:
                print_error(f"Genesis block validation failed: Hash mismatch")
                return False
            return True

        last = self.chain[-1]

        if block.prev_hash != last.hash:
            print_error(f"Block {block.index} validation failed: PrevHash mismatch")
            return False

        if calculate_hash(block) != block.hash:
            print_error(f"Block {block.index} validation failed: Hash mismatch")
            return False

        if block.index != last.index + 1:
            print_error(f"Block {block.index} validation failed: Index mismatch")
            return False

        if not skip_pow and not self._validate_pow(block.hash, difficulty):
            print_error(f"Block {block.index} validation failed: Invalid proof-of-work")
            return False

        return True

    def _validate_pow(self, block_hash: str, difficulty: int) -> bool:
        """Validate proof-of-work for a block hash"""
        if difficulty <= 0 or len(block_hash) < difficulty:
            return True

        try:
            hash_bytes = bytes.fromhex(block_hash)
        except:
            return False

        # Check if hash starts with required number of zeros
        for i in range((difficulty + 1) // 2):
            if difficulty > i * 2 and hash_bytes[i] != 0:
                return False
            if difficulty > i * 2 + 1 and (hash_bytes[i] & 0x0F) != 0:
                return False
        return True

    def add_block(self, block: Block, difficulty: int) -> bool:
        """Add a block to the chain if validation passes"""
        return self.add_block_skip_pow(block, difficulty, False)

    def add_block_skip_pow(self, block: Block, difficulty: int, skip_pow: bool) -> bool:
        """Add a block with optional PoW validation"""
        if self.validate_block(block, difficulty, skip_pow):
            self.chain.append(block)
            return True
        return False

    def save_to_file(self, path: str) -> bool:
        """Save blockchain to JSON file"""
        data = [block.to_dict() for block in self.chain]
        return save_json_file(path, data)

    def load_from_file(self, path: str) -> bool:
        """Load blockchain from JSON file"""
        data = load_json_file(path)
        print_info(f"[DEBUG] load_from_file: loading {path}, data is None: {data is None}")
        if data is None:
            # Create genesis block if file doesn't exist
            self.chain = [create_genesis_block()]
            print_info(f"[DEBUG] load_from_file: created genesis block, chain length: {len(self.chain)}")
            return self.save_to_file(path)

        try:
            # Handle both Go format {"chain": [...]} and Python format [...]
            if isinstance(data, dict) and "chain" in data:
                block_data_list = data["chain"]
            elif isinstance(data, list):
                block_data_list = data
            else:
                raise ValueError("Invalid blockchain data format")

            print_info(f"[DEBUG] load_from_file: block_data_list length: {len(block_data_list)}")
            self.chain = []
            for i, block_data in enumerate(block_data_list):
                try:
                    block = Block.from_dict(block_data)
                    self.chain.append(block)
                except Exception as e:
                    print_error(f"Failed to load block {i}: {e}")
                    raise

            # Additional validation: ensure we have at least genesis
            if len(self.chain) == 0:
                print_warning("[DEBUG] load_from_file: chain is empty after loading, creating genesis block")
                self.chain = [create_genesis_block()]

            # Recalculate hashes to fix any inconsistencies
            for block in self.chain:
                block.hash = calculate_hash(block)

            return True
        except Exception as e:
            print_error(f"Failed to load blockchain: {e}")
            import traceback
            traceback.print_exc()
            # If loading fails, create genesis block
            print_warning("[DEBUG] load_from_file: exception during load, creating genesis block")
            self.chain = [create_genesis_block()]
            return self.save_to_file(path)

    def get_height(self) -> int:
        """Get current blockchain height"""
        return len(self.chain) - 1

    def get_block(self, index: int) -> Optional[Block]:
        """Get block by index"""
        if 0 <= index < len(self.chain):
            return self.chain[index]
        return None

    def get_blocks_range(self, start: int, end: int) -> List[Block]:
        """Get blocks in range [start, end]"""
        if start < 0 or end >= len(self.chain) or start > end:
            return []
        return self.chain[start:end+1]


def calculate_hash(block: Block, mem: Optional[bytearray] = None) -> str:
    """Calculate SHA3-256 hash of a block using rx/owo algorithm.

    If a precomputed `mem` buffer is provided it will be used instead of
    rebuilding the 2MB memory buffer. This is a significant optimization
    when hashing the same block multiple times with different nonces.
    """

    # Create block data for hashing (exclude the hash field itself)
    block_for_hash = {
        'index': block.index,
        'timestamp': block.timestamp,
        'transactions': [tx.to_dict() for tx in block.transactions],
        'prev_hash': block.prev_hash,
        'nonce': block.nonce
    }

    block_bytes = json.dumps(block_for_hash, sort_keys=True, separators=(',', ':')).encode()

    # rx/owo memory-hard algorithm
    mem_size = 2 * 1024 * 1024  # 2MB

    # Use provided mem if available to avoid recomputing the 2MB buffer per-nonce
    if mem is None:
        mem = bytearray(mem_size)
        # Deterministic memory seeding
        seed = hashlib.sha256(f"{block.index}{block.prev_hash}".encode()).digest()
        for i in range(0, mem_size, 32):
            end = min(i + 32, mem_size)
            mem[i:end] = seed[:end-i]

    # CPU and memory intensive calculations
    acc = block.nonce
    base_idx = block.nonce * 31 % mem_size
    step = 7919 % mem_size

    for i in range(12):
        idx = (base_idx + i * step) % mem_size
        acc ^= mem[idx] << ((i % 4) * 8)
        if i % 3 == 0:
            acc = (acc << 7) ^ (acc >> 11) ^ acc

    puzzle = (block.nonce ^ len(block_bytes)) + (acc & 0xFFFF)

    # Build final hash input
    hash_input = block_bytes
    hash_input += mem[(block.nonce * 13) % mem_size].to_bytes(1, 'big')
    hash_input += puzzle.to_bytes(2, 'big')

    # Add acc as 8 bytes
    hash_input += acc.to_bytes(8, 'big')

    # Additional CPU work
    for j in range(4):
        acc = (acc << 5) ^ (acc >> 3) ^ len(hash_input)

    hash_input += (acc & 0xFF).to_bytes(1, 'big')
    hash_input += ((acc >> 8) & 0xFF).to_bytes(1, 'big')

    # Final SHA3-256 hash
    return hashlib.sha3_256(hash_input).hexdigest()


def create_genesis_block() -> Block:
    """Create the genesis block"""
    genesis = Block(
        index=0,
        timestamp="2025-10-11T00:00:00Z",  # Fixed timestamp
        transactions=[Transaction("genesis", "network", 0)],
        prev_hash="",
        nonce=0
    )
    genesis.hash = calculate_hash(genesis)
    return genesis


def mine_block(prev_block: Block, transactions: List[Transaction], difficulty: int,
               progress_callback: Optional[Callable[[int], None]] = None,
               report_every: int = 1000) -> Tuple[Block, int]:
    """
    Mine a new block using rx/owo proof-of-work
    Returns (block, attempts)

    Optional progress_callback(delta_attempts) will be called periodically
    every `report_every` attempts so callers (like the miner) can update
    shared statistics in real-time. This fixes hashrate reporting when no
    blocks are found for a long time.
    """
    target_prefix = "0" * difficulty

    block = Block(
        index=prev_block.index + 1,
        timestamp=format_timestamp(),
        transactions=transactions,
        prev_hash=prev_block.hash,
        nonce=0
    )

    # Pre-calculate memory buffer once and reuse it for each nonce
    mem_size = 2 * 1024 * 1024
    mem = bytearray(mem_size)

    seed = hashlib.sha256(f"{block.index}{block.prev_hash}".encode()).digest()
    for i in range(0, mem_size, 32):
        end = min(i + 32, mem_size)
        mem[i:end] = seed[:end-i]

    attempts = 0
    nonce = 0

    while True:
        block.nonce = nonce

        # Pass the precomputed mem buffer to avoid recomputing it per-nonce
        block.hash = calculate_hash(block, mem=mem)
        attempts += 1

        # Periodically report progress to caller
        if progress_callback is not None and report_every > 0 and attempts % report_every == 0:
            try:
                progress_callback(report_every)
            except Exception:
                # Don't let progress reporting break mining
                pass

        if block.hash.startswith(target_prefix):
            # Report any remaining attempts that haven't been reported yet
            if progress_callback is not None and report_every > 0 and attempts % report_every != 0:
                try:
                    progress_callback(attempts % report_every)
                except Exception:
                    pass
            return block, attempts

        nonce += 1


def sign_transaction(tx: Transaction, private_key_pem: str) -> bool:
    """Sign a transaction with ECDSA private key"""
    try:
        # For now, implement a simple signature scheme
        # In production, this would use proper ECDSA
        import ecdsa
        import base64

        # Parse PEM private key
        priv_key = ecdsa.SigningKey.from_pem(private_key_pem)

        # Create message to sign
        message = f"{tx.from_addr}|{tx.to_addr}|{tx.amount}"
        signature = priv_key.sign(message.encode())

        tx.signature = base64.b64encode(signature).decode()
        return True
    except Exception as e:
        print_error(f"Failed to sign transaction: {e}")
        return False


def verify_transaction_signature(tx: Transaction, public_key_pem: str) -> bool:
    """Verify transaction signature"""
    try:
        import ecdsa
        import base64

        # Parse PEM public key
        pub_key = ecdsa.VerifyingKey.from_pem(public_key_pem)

        # Recreate message
        message = f"{tx.from_addr}|{tx.to_addr}|{tx.amount}"

        # Decode signature
        signature = base64.b64decode(tx.signature)

        return pub_key.verify(signature, message.encode())
    except:
        return False