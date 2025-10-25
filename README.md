# Owonero

![banner](./assets/owe.png)

A lightweight, proof-of-work blockchain cryptocurrency written in Python with async networking. Features automatic updates, peer-to-peer networking, and efficient mining with dynamic difficulty adjustment.

## ✨ Features

- **Async Networking**: High-performance asynchronous I/O for peer-to-peer communication
- **Proof-of-Work Mining**: Memory-hard mining algorithm with dynamic difficulty
- **Peer-to-Peer Networking**: Decentralized network with automatic peer discovery
- **Automatic Updates**: Self-updating from GitHub releases
- **Wallet System**: Secure address generation and transaction management
- **Incremental Sync**: Efficient blockchain synchronization with chunked downloads
- **Cross-Platform**: Windows, Linux, and macOS support
- **TCP Protocol**: Simple, reliable network communication

## 📋 Minimum Requirements

### System Requirements
- **OS**: Windows 10+, Linux (Ubuntu 18.04+, CentOS 7+), macOS 10.15+
- **CPU**: 32-bit processor with at least 2 cores
- **RAM**: 512 MB minimum, 1 GB recommended
- **Storage**: 100 MB free space for blockchain and binaries
- **Network**: Stable internet connection for peer-to-peer communication

### Software Requirements
- **Python**: Version 3.11 or later
- **pip**: Python package installer
- **Git**: For cloning the repository
- **GitHub CLI** (optional): For automated releases

### Network Requirements
- **Open Ports**: TCP port 6969 (configurable)
- **Firewall**: Allow outbound connections to peers
- **Internet Access**: Required for updates and peer discovery

## 🚀 Quick Start

#### Run
```bash
# Clone repository
git clone https://github.com/tosterlolz/Owonero.git
cd Owonero

# Install Python dependencies
pip install -r requirements.txt

# Run the application
python src/main.py
```

#### Update Existing Installation
```bash
# Navigate to your Owonero directory
cd Owonero

# Pull latest changes
git pull origin python-experimental

# Update Python dependencies (if requirements.txt changed)
pip install -r requirements.txt --upgrade

# Restart your application
python src/main.py
```

### 2. Start Your First Node

```bash
# Start daemon (network node)
python src/main.py -d -p 6969

# Or connect to existing network
python src/main.py -d -n existing-node.com:6969 -p 6969
```

### 3. Start Mining

```bash
# Mine with 4 threads
python src/main.py -m -n localhost:6969 -t 4

# Mine to remote node
python src/main.py -m -n node.example.com:6969 -t 8
```

### 4. Check Your Wallet

```bash
# View balance and address
python src/main.py
```

## 📖 Usage Guide

### Command Line Options

#### Daemon Mode
```bash
python src/main.py -d [options]
```
- `-d`: Run as network daemon
- `-p PORT`: Listening port (default: 6969)
- `-n HOST:PORT`: Connect to existing node
- `-peers "ADDR1,ADDR2"`: Initial peer addresses
- `--no-init`: Skip local blockchain, sync from peers
- `--no-update`: Skip automatic update check

#### Mining Mode
```bash
python src/main.py -m [options]
```
- `-m`: Start mining
- `-n HOST:PORT`: Node to submit blocks to
- `-w FILE`: Wallet file (default: wallet.json)
- `-t THREADS`: Number of mining threads (default: 1)
- `-b BLOCKS`: Blocks to mine (0 = unlimited)

#### Wallet Mode
```bash
python src/main.py [options]
```
- `-w FILE`: Custom wallet file
- `-tui`: Launch terminal user interface

### Network Protocol

Owonero uses a simple TCP-based protocol. Connect using telnet or netcat:

```bash
# Connect to daemon
telnet localhost 6969

# Get blockchain height
getheight
42

# Get peer list
getpeers
["192.168.1.100:6969", "node.example.com:6969"]

# Add new peer
addpeer
192.168.1.101:6969
ok
```

#### Available Commands

| Command | Description | Response |
|---------|-------------|----------|
| `getchain` | Full blockchain (JSON) | Blockchain JSON |
| `getheight` | Current block height | Integer |
| `getblocks START END` | Block range | Blocks JSON array |
| `submitblock` | Submit mined block | JSON payload required |
| `sendtx` | Submit transaction | JSON payload required |
| `getpeers` | Known peers list | JSON array |
| `addpeer` | Add peer | Address on next line |
| `removepeer` | Remove peer | Address on next line |
| `getwallet` | Wallet information | Address on next line |
| `sync` | Force sync | Initiates sync |
| `mineractive` | Report active miner | Miner address |

## 🛠️ Building from Source

### Prerequisites
- Python 3.11+
- pip (Python package installer)
- Git
- PowerShell (Windows) or Bash (Linux/macOS)

### Development Setup

```bash
# Clone with submodules
git clone --recursive https://github.com/tosterlolz/Owonero.git
cd Owonero

# Install dependencies
pip install -r requirements.txt

# Run the application
python src/main.py
```

## 🔄 Continuous Integration

Owonero uses GitHub Actions for automated building and releasing. The CI/CD pipeline:

- **Triggers**: On push to `master`/`main` branch and pull requests
- **Platforms**: Windows (AMD64, i386), Linux (AMD64, i386, ARM64)
- **Artifacts**: Binaries uploaded as workflow artifacts
- **Releases**: Automatic GitHub releases with zipped binaries

### Workflow Files
- `.github/workflows/build.yml` - Main build workflow

### Manual Release
You can also trigger builds manually:
1. Go to **Actions** tab in GitHub
2. Select **Build Owonero** workflow
3. Click **Run workflow**
4. Optionally specify a custom version

### Build Status
[![Build Status](https://github.com/tosterlolz/Owonero/actions/workflows/build.yml/badge.svg)](https://github.com/tosterlolz/Owonero/actions/workflows/build.yml)

## 📊 Monitoring

### Web Stats Interface
When running a daemon, access web stats at `http://localhost:6767`

### Mining Performance
- Monitor hashrate in mining output
- Adjust thread count with `-t` flag
- Higher difficulty requires more computational power

### Network Status
```bash
# Check daemon status
echo "getheight" | nc localhost 6969

# View connected peers
echo "getpeers" | nc localhost 6969
```

## 🔒 Security

- **Private Keys**: Never share your wallet files
- **Network**: Use firewall to restrict access to daemon port
- **Updates**: Automatic updates download from official GitHub releases
- **Mining**: Secure proof-of-work prevents double-spending

## 🐛 Troubleshooting

### Common Issues

#### "Cannot connect to node"
- Verify daemon is running: `netstat -an | grep 6969`
- Check firewall settings
- Ensure correct host:port format

#### "Mining not working"
- Confirm connection to daemon
- Check wallet file exists: `ls wallet.json`
- Verify sufficient system resources

#### "Sync fails"
- Use `--no-init` flag for clean sync
- Check network connectivity
- Verify peer addresses are reachable

#### "Update fails"
- Check internet connection
- Verify GitHub API access
- Use `--no-update` to skip updates

### Debug Mode
```bash
# Enable debug logging
OWONERO_LOG_LEVEL=debug python src/main.py -d

# Run with Python debugger
python -m pdb src/main.py -d
```

### Getting Help
- Check daemon logs for error messages
- Verify system meets minimum requirements
- Test network connectivity with ping/telnet

## 🤝 Contributing

We welcome contributions! Please follow these steps:

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature-name`
3. **Make** your changes with tests
4. **Test** thoroughly: `python -m pytest` (if tests are added)
5. **Submit** a pull request

### Development Guidelines
- Follow Python coding standards (PEP 8)
- Add tests for new features
- Update documentation
- Use meaningful commit messages

### Project Structure
```
src/
├── main.py           # CLI and main entry point
├── daemon.py         # Async network daemon and peer management
├── miner.py          # Async mining logic and task management
├── wallet.py         # Wallet creation and management
├── wallet_tui.py     # Terminal user interface
├── blockchain.py     # Core blockchain logic
├── web_stats.py      # Async web statistics interface
└── utils.py          # Utility functions

requirements.txt      # Python dependencies
build.ps1            # Cross-platform build script
README.md           # This file
LICENSE             # MIT License
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⚠️ Disclaimer

**Educational Purpose Only**

This software is for educational and experimental purposes. It is not intended for production use or real financial transactions. The developers are not responsible for any financial losses or security issues arising from its use.

### Known Limitations
- Not audited for security vulnerabilities
- No formal economic analysis
- Experimental consensus mechanism
- Limited scalability testing

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/tosterlolz/Owonero/issues)
- **Discussions**: [GitHub Discussions](https://github.com/tosterlolz/Owonero/discussions)
- **Documentation**: This README and inline code comments

---

**Happy mining! ⛏️**</content>