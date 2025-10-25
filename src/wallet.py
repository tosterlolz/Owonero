"""
Owonero - Wallet management
Key generation, transaction signing, and balance management
"""

import os
import json
import secrets
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass, asdict
import base64

try:
    import ecdsa
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric import rsa
    from cryptography.hazmat.backends import default_backend
except ImportError:
    print("Required packages not installed. Run: pip install -r requirements.txt")
    ecdsa = None

from utils import print_error, print_success, print_info, save_json_file, load_json_file
from blockchain import Blockchain, Transaction, sign_transaction, verify_transaction_signature


@dataclass
class Wallet:
    """Represents a cryptocurrency wallet"""
    address: str
    private_key_pem: str
    public_key_pem: str

    def to_dict(self) -> dict:
        return {
            'address': self.address,
            'privkey': self.private_key_pem,
            'pubkey': self.public_key_pem
        }

    @classmethod
    def from_dict(cls, data: dict) -> 'Wallet':
        # Handle both Go format (privkey/pubkey) and Python format (private_key_pem/public_key_pem)
        private_key_pem = data.get('private_key_pem') or data.get('privkey')
        public_key_pem = data.get('public_key_pem') or data.get('pubkey')
        
        if not private_key_pem or not public_key_pem:
            raise ValueError("Wallet data missing private or public key")
            
        return cls(
            address=data['address'],
            private_key_pem=private_key_pem,
            public_key_pem=public_key_pem
        )


def generate_wallet() -> Wallet:
    """Generate a new wallet with ECDSA keypair"""
    if ecdsa is None:
        raise ImportError("ecdsa package required for wallet generation")

    assert ecdsa is not None  # Type guard for static analysis

    # Generate ECDSA keypair
    private_key = ecdsa.SigningKey.generate(curve=ecdsa.SECP256k1)
    public_key = private_key.verifying_key

    # Convert to PEM format
    private_key_pem = private_key.to_pem().decode()  # type: ignore
    public_key_pem = public_key.to_pem().decode()  # type: ignore

    # Create address from public key hash
    import hashlib
    pub_key_bytes = public_key.to_string("compressed")  # type: ignore
    address = "OWO" + hashlib.sha256(pub_key_bytes).hexdigest()[:38].upper()

    return Wallet(address, private_key_pem, public_key_pem)


def load_or_create_wallet(wallet_path: str) -> Wallet:
    """Load wallet from file or create a new one"""
    data = load_json_file(wallet_path)
    if data:
        try:
            return Wallet.from_dict(data)
        except Exception as e:
            print_error(f"Failed to load wallet: {e}")

    print_info("Creating new wallet...")
    wallet = generate_wallet()

    if save_json_file(wallet_path, wallet.to_dict()):
        print_success(f"Wallet saved to {wallet_path}")
    else:
        print_error("Failed to save wallet")

    return wallet


def get_balance(wallet_address: str, blockchain: Blockchain) -> int:
    """Calculate wallet balance from blockchain"""
    balance = 0

    for block in blockchain.chain:
        for tx in block.transactions:
            if tx.to_addr == wallet_address:
                balance += tx.amount
            if tx.from_addr == wallet_address:
                balance -= tx.amount

    return balance


def create_transaction(wallet: Wallet, to_address: str, amount: int) -> Optional[Transaction]:
    """Create and sign a transaction"""
    if amount <= 0:
        print_error("Amount must be positive")
        return None

    tx = Transaction(wallet.address, to_address, amount)

    if not sign_transaction(tx, wallet.private_key_pem):
        print_error("Failed to sign transaction")
        return None

    return tx


def validate_transaction(tx: Transaction, blockchain: Blockchain) -> bool:
    """Validate a transaction"""
    # Check signature
    if not verify_transaction_signature(tx, tx.from_addr):
        print_error("Invalid transaction signature")
        return False

    # Check balance
    balance = get_balance(tx.from_addr, blockchain)
    if balance < tx.amount:
        print_error(f"Insufficient balance: {balance} < {tx.amount}")
        return False

    # Check amount is positive
    if tx.amount <= 0:
        print_error("Transaction amount must be positive")
        return False

    return True


def get_wallet_info(wallet_address: str, blockchain: Blockchain) -> Optional[Dict[str, Any]]:
    """Get detailed wallet information"""
    balance = get_balance(wallet_address, blockchain)

    # Get transaction history
    transactions = []
    for block in blockchain.chain:
        for tx in block.transactions:
            if tx.from_addr == wallet_address or tx.to_addr == wallet_address:
                transactions.append({
                    'block_index': block.index,
                    'timestamp': block.timestamp,
                    'from': tx.from_addr,
                    'to': tx.to_addr,
                    'amount': tx.amount,
                    'type': 'sent' if tx.from_addr == wallet_address else 'received'
                })

    return {
        'address': wallet_address,
        'balance': balance,
        'transaction_count': len(transactions),
        'transactions': transactions[-20:]  # Last 20 transactions
    }


def send_transaction(wallet: Wallet, to_address: str, amount: int, blockchain: Blockchain) -> bool:
    """Send a transaction by adding it to the latest block"""
    tx = create_transaction(wallet, to_address, amount)
    if not tx:
        return False

    if not validate_transaction(tx, blockchain):
        return False

    # Add to latest block (simplified - in real implementation would go to mempool)
    if len(blockchain.chain) == 0:
        print_error("Blockchain empty")
        return False

    last_block = blockchain.chain[-1]
    last_block.transactions.append(tx)

    # Save blockchain
    from utils import BLOCKCHAIN_FILE
    if blockchain.save_to_file(BLOCKCHAIN_FILE):
        print_success(f"Transaction sent: {amount} to {to_address}")
        return True
    else:
        print_error("Failed to save transaction")
        return False


def import_wallet_from_private_key(private_key_pem: str) -> Optional[Wallet]:
    """Import wallet from private key PEM"""
    try:
        if ecdsa is None:
            raise ImportError("ecdsa package required")

        assert ecdsa is not None  # Type guard for static analysis

        private_key = ecdsa.SigningKey.from_pem(private_key_pem.encode())
        public_key = private_key.verifying_key

        public_key_pem = public_key.to_pem().decode()  # type: ignore

        # Create address
        import hashlib
        pub_key_bytes = public_key.to_string("compressed")  # type: ignore
        address = "OWO" + hashlib.sha256(pub_key_bytes).hexdigest()[:38].upper()

        return Wallet(address, private_key_pem, public_key_pem)
    except Exception as e:
        print_error(f"Failed to import wallet: {e}")
        return None


def export_wallet_private_key(wallet: Wallet) -> str:
    """Export wallet private key"""
    return wallet.private_key_pem


def validate_address(address: str) -> bool:
    """Validate wallet address format"""
    if not address.startswith("OWO"):
        return False
    if len(address) != 41:  # OWO + 38 hex chars
        return False
    try:
        int(address[3:], 16)
        return True
    except ValueError:
        return False