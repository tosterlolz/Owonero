# Owonero

![banner](./assets/owe.png)

A lightweight, proof-of-work blockchain cryptocurrency written in Go. Features automatic updates, peer-to-peer networking, and efficient mining with dynamic difficulty adjustment.

## ✨ Features

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
- **Go**: Version 1.19 or later (for building from source)
- **Git**: For cloning the repository
- **GitHub CLI** (optional): For automated releases

### Network Requirements
- **Open Ports**: TCP port 6969 (configurable)
- **Firewall**: Allow outbound connections to peers
- **Internet Access**: Required for updates and peer discovery

## 🚀 Quick Start

### 1. Download and Install

#### Option A: Download Pre-built Binary
```bash
# Download latest release from GitHub
curl -L https://github.com/tosterlolz/Owonero/releases/latest/download/owonero-linux-amd64.zip -o owonero.zip
unzip owonero.zip
chmod +x owonero
```

#### Option B: Build from Source
```bash
# Clone repository
git clone https://github.com/tosterlolz/Owonero.git
cd Owonero

# Build for your platform
./build.ps1                    # Windows PowerShell
# OR
go build -o owonero ./src
```

### 2. Start Your First Node

```bash
# Start daemon (network node)
./owonero -d -p 6969

# Or connect to existing network
./owonero -d -n owonero.yabai.buzz:6969 -p 6969
```

### 3. Start Mining

```bash
# Mine with 4 threads
./owonero -m -n localhost:6969 -t 4 # SOLO

# Mine to remote node
./owonero -m -n owonero.yabai.buzz:6969 -t 8 # SOLO
./owonero -m --pool -n owonero.yabai.buzz:6969 -t 8 # POOL
```

### 4. Check Your Wallet

```bash
# View balance and address
./owonero
```

## 📖 Usage Guide

### Command Line Options

#### Daemon Mode
```bash
./owonero -d [options]
```
- `-d`: Run as network daemon
- `-p PORT`: Listening port (default: 6969)
- `-n HOST:PORT`: Connect to existing node
- `-peers "ADDR1,ADDR2"`: Initial peer addresses
- `--no-init`: Skip local blockchain, sync from peers
- `--no-update`: Skip automatic update check

#### Mining Mode
```bash
./owonero -m [options]
```
- `-m`: Start mining
- `-n HOST:PORT`: Node to submit blocks to
- `-w FILE`: Wallet file (default: wallet.json)
- `-t THREADS`: Number of mining threads (default: 1)
- `-b BLOCKS`: Blocks to mine (0 = unlimited)

#### Wallet Mode
```bash
./owonero [options]
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
- Go 1.19+
- Git
- PowerShell (Windows) or Bash (Linux/macOS)

### Build Commands

```bash
# Windows (PowerShell)
./build.ps1                    # Build for current platform
./build.ps1 -Help             # Show help

# Linux/macOS
go build -o owonero ./src     # Single platform build

# Cross-platform build
GOOS=linux GOARCH=amd64 go build -o owonero-linux ./src
GOOS=windows GOARCH=amd64 go build -o owonero-windows.exe ./src
```

### Development Setup

```bash
# Clone with submodules
git clone --recursive https://github.com/tosterlolz/Owonero.git
cd Owonero

# Install dependencies
go mod download

# Run tests
go test ./...

# Build with debug info
go build -tags debug -o owonero-debug ./src
```

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
OWONERO_LOG_LEVEL=debug ./owonero -d

# Build with debug symbols
go build -tags debug -o owonero-debug ./src
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
4. **Test** thoroughly: `go test ./...`
5. **Submit** a pull request

### Development Guidelines
- Follow Go coding standards
- Add tests for new features
- Update documentation
- Use meaningful commit messages

### Project Structure
```
src/
├── main.go           # CLI and main entry point
├── daemon.go         # Network daemon and peer management
├── miner.go          # Mining logic and thread management
├── wallet.go         # Wallet creation and management
├── wallet_tui.go     # Terminal user interface
├── blockchain.go     # Core blockchain logic
├── web_stats.go      # Web statistics interface
└── go.mod           # Go module dependencies

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