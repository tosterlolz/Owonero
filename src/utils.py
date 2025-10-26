"""
Owonero - A lightweight blockchain cryptocurrency
Utility functions and constants
"""

import os
import sys
import platform
import json
import hashlib
import secrets
from typing import Dict, List, Any, Optional
import time
import urllib.request

# ANSI Color constants
RESET = "\033[0m"
RED = "\033[31m"
GREEN = "\033[32m"
YELLOW = "\033[33m"
BLUE = "\033[34m"
MAGENTA = "\033[35m"
CYAN = "\033[36m"
WHITE = "\033[37m"
BOLD = "\033[1m"
UNDERLINE = "\033[4m"
REVERSE = "\033[7m"
DIM = "\033[2m"
ITALIC = "\033[3m"
STRIKETHROUGH = "\033[9m"

# Application constants
VERSION = "0.3.6"
BLOCKCHAIN_FILE = "blockchain.json"
WALLET_FILE = "wallet.json"

def print_colored(message: str, color: str = RESET, bold: bool = False) -> None:
    """Print a colored message to stdout"""
    prefix = BOLD if bold else ""
    print(f"{prefix}{color}{message}{RESET}")

def print_error(message: str) -> None:
    """Print an error message in red"""
    print_colored(message, RED, True)

def print_success(message: str) -> None:
    """Print a success message in green"""
    print_colored(message, GREEN)

def print_warning(message: str) -> None:
    """Print a warning message in yellow"""
    print_colored(message, YELLOW)

def print_info(message: str) -> None:
    """Print an info message in cyan"""
    print_colored(message, CYAN)

def get_platform_info() -> tuple[str, str]:
    """Get the current platform and architecture"""
    system = platform.system().lower()
    machine = platform.machine().lower()

    # Normalize architecture names
    if machine in ['x86_64', 'amd64']:
        arch = 'amd64'
    elif machine in ['i386', 'i686', 'x86']:
        arch = '386'
    elif machine == 'aarch64':
        arch = 'arm64'
    else:
        arch = machine

    return system, arch

def check_for_updates() -> None:
    """Check for updates from GitHub releases"""
    try:
        api_url = "https://api.github.com/repos/tosterlolz/Owonero/releases/latest"
        with urllib.request.urlopen(api_url, timeout=10) as response:
            if response.status != 200:
                print_warning(f"Update check failed: HTTP {response.status}")
                return

            data = json.loads(response.read().decode())
            latest_version = data.get('tag_name', '').lstrip('v')

            if latest_version == VERSION:
                print_success(f"You are running the latest version ({VERSION})")
                return

            if is_version_newer(latest_version, VERSION):
                print_warning(f"New version available: {latest_version} (current: {VERSION})")
                print_info("Run 'git pull' to update to the latest version")
            else:
                print_success(f"You are running the latest version ({VERSION})")

    except Exception as e:
        print_warning(f"Failed to check for updates: {e}")

def is_version_newer(latest: str, current: str) -> bool:
    """Compare version strings (simple semantic versioning)"""
    try:
        latest_parts = [int(x) for x in latest.split('.')]
        current_parts = [int(x) for x in current.split('.')]

        for i in range(max(len(latest_parts), len(current_parts))):
            latest_num = latest_parts[i] if i < len(latest_parts) else 0
            current_num = current_parts[i] if i < len(current_parts) else 0

            if latest_num > current_num:
                return True
            elif latest_num < current_num:
                return False

        return False
    except:
        return False

def load_json_file(filepath: str) -> Optional[dict]:
    """Load and parse a JSON file"""
    try:
        if not os.path.exists(filepath):
            return None
        with open(filepath, 'r', encoding='utf-8') as f:
            return json.load(f)
    except Exception as e:
        print_error(f"Failed to load {filepath}: {e}")
        return None

def save_json_file(filepath: str, data: Any) -> bool:
    """Save data to a JSON file"""
    try:
        with open(filepath, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
        return True
    except Exception as e:
        print_error(f"Failed to save {filepath}: {e}")
        return False

def ensure_directory(path: str) -> None:
    """Ensure a directory exists"""
    os.makedirs(path, exist_ok=True)

def get_file_size_mb(filepath: str) -> float:
    """Get file size in MB"""
    try:
        return os.path.getsize(filepath) / (1024 * 1024)
    except:
        return 0.0

def format_timestamp(ts: Optional[str] = None) -> str:
    """Format timestamp for RFC3339"""
    if ts is None:
        return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    return ts