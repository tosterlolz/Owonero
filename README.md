# Owonero

![banner](./assets/owe.png)

Current release: v0.4.3 (dev)

A lightweight, proof-of-work blockchain cryptocurrency written in Rust. Features automatic updates, peer-to-peer networking, and efficient mining with the RX/OWO memory-hard algorithm.

IMPORTANT: Windows users — we publish release binaries on GitHub releases. If you are on Windows we recommend downloading the pre-built release for convenience and correctness.

## ✨ Features

- **RX/OWO Mining Algorithm**: Advanced memory-hard proof-of-work based on RandomX principles, ASIC-resistant with 2MB scratchpad
- **Peer-to-Peer Networking**: Decentralized network with automatic peer discovery
- **Automatic Updates**: Self-updating from GitHub releases
- **Wallet System**: Secure ECDSA address generation and transaction management
- **Professional TUI**: Ratatui-based terminal interface for mining with real-time statistics
- **Incremental Sync**: Efficient blockchain synchronization with chunked downloads
- **Cross-Platform**: Windows, Linux, and macOS support
- **Async Architecture**: Tokio-based concurrent processing for optimal performance

## 📋 Minimum Requirements

### System Requirements
- **OS**: Windows 10+, Linux (Ubuntu 18.04+, CentOS 7+), macOS 10.15+
- **CPU**: 32-bit processor with at least 2 cores
- **RAM**: 512 MB minimum, 1 GB recommended
- **Storage**: 100 MB free space for blockchain and binaries
- **Network**: Stable internet connection for peer-to-peer communication

### Software Requirements
- **Rust**: Version 1.70 or later (for building from source)
- **Cargo**: Rust package manager
- **Git**: For cloning the repository
- **GitHub CLI** (optional): For automated releases

### Network Requirements
- **Open Ports**: TCP port 6969 (configurable)
- **Firewall**: Allow outbound connections to peers
- **Internet Access**: Required for updates and peer discovery

## 🚀 Quick Start

### 1. Download and Install

#### Option A: Download Pre-built Binary from [releases](https://github.com/tosterlolz/Owonero/releases)

#### Option B: Build from Source
```bash
# Clone repository
git clone https://github.com/tosterlolz/Owonero.git
cd Owonero

# Build for your platform
sudo make install               # Release build
```

### 2. Start Your First Node

```bash
# Start daemon (network node)
owonero -d

# Or connect to existing network
owonero  -d -n owonero.yabai.buzz:6969
```

## 🔨 RX/OWO Mining Algorithm

Owonero uses the **RX/OWO** algorithm, a custom memory-hard proof-of-work system inspired by RandomX. This algorithm is designed to be ASIC-resistant while remaining efficient on general-purpose CPUs.

### Algorithm Features

- **2MB Scratchpad**: Large memory requirement prevents ASIC optimization
- **Complex Memory Access**: Pseudo-random memory reads/writes with multiple cache levels
- **ASIC-Resistant Operations**: Bit rotations, multiplications, and non-linear arithmetic
- **Dynamic Entropy**: Block data influences memory access patterns
- **2048 Iterations**: Multiple computational rounds per hash (current default in code)

### Mining Performance

The algorithm scales well with:
- **CPU Cores**: Linear scaling with thread count
- **Memory Bandwidth**: Higher memory speed improves performance
- **Cache Size**: Larger CPU caches provide better performance

### Hardware Recommendations

- **CPU**: Modern multi-core processor (4+ cores recommended)
- **RAM**: 4GB+ system memory (mining uses ~2MB per thread)
- **Cache**: CPUs with large L3 cache perform better

### Install completions
```bash
owonero --install-completions <shell>
# for bash:
owonero --install-completions bash
```

### Mining Commands

```bash
# Solo mining with 8 threads
owonero --mine --node localhost:6969 --threads 8

# Pool mining (if supported)
owonero  --mine --pool --node pool.example.com:6969 --threads 8

# Mining with TUI interface
owonero  --mine --miner-ui --node localhost:6969 --threads 4
```

### 4. Check Your Wallet

```bash
# View balance and address
owonero 
```

## 📖 Usage Guide

### Command Line Options

#### Daemon Mode
```bash
owonero  -d [options]
```
- `-d`: Run as network daemon
- `-p PORT`: Listening port (default: 6969)
- `-n HOST:PORT`: Connect to existing node
- `-peers "ADDR1,ADDR2"`: Initial peer addresses
- `--no-init`: Skip local blockchain, sync from peers
- `--no-update`: Skip automatic update check

#### Mining Mode
```bash
owonero  -m [options]
```
- `-m`: Start mining
- `-n HOST:PORT`: Node to submit blocks to
- `-w FILE`: Wallet file (default: wallet.json)
- `-t THREADS`: Number of mining threads (default: 1)
- `-b BLOCKS`: Blocks to mine (0 = unlimited)

#### Wallet Mode
```bash
owonero  [options]
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
- Rust 1.70+
- Cargo package manager
- Git
- PowerShell (Windows) or Bash (Linux/macOS)

### Build Commands

```bash
# Release build (optimized)
cargo build --release

# Debug build
cargo build

# Windows PowerShell script
./build.ps1

# Cross-platform builds
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-pc-windows-gnu
cargo build --release --target x86_64-apple-darwin
```

### Development Setup

```bash
# Clone repository
git clone https://github.com/tosterlolz/Owonero.git
cd owonero-rs

# Install dependencies
cargo fetch

# Run tests
cargo test

# Build with debug symbols
cargo build

# Run with debug output
RUST_LOG=debug cargo run -- --help
```

### Dependencies

- **tokio**: Async runtime
- **serde**: Serialization
- **sha3**: Cryptographic hashing
- **ring**: Cryptographic operations
- **ratatui/crossterm**: Terminal UI
- **clap**: Command line parsing
- **anyhow**: Error handling

## 📊 Monitoring

### Web Stats Interface
When running a daemon, access web stats at `http://localhost:6767/`

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

## � Documentation

More detailed developer and build documentation is available in the `docs/` directory:

- `docs/BUILD.md` — step-by-step build instructions for Linux, WSL, macOS, and notes for Windows users.
- `docs/TROUBLESHOOTING.md` — common build/runtime issues and fixes (OpenSSL, WSL, hashrate stalls).

If you'd like, we can add a Dockerfile and CI workflow to produce reproducible release artifacts.

## �🔒 Security

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
OWONERO_LOG_LEVEL=debug owonero  -d

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
owonero-rs/
├── src/
│   ├── main.rs           # CLI entry point and command routing
│   ├── blockchain.rs     # RX/OWO algorithm and blockchain logic
│   ├── miner.rs          # Async mining with thread management
│   ├── miner_ui.rs       # Ratatui-based terminal interface
│   ├── wallet.rs         # ECDSA wallet management
│   ├── daemon.rs         # Async TCP server and peer management
│   ├── config.rs         # JSON configuration management
│   └── update.rs         # GitHub release checking
├── Cargo.toml           # Rust dependencies and metadata
├── build.ps1            # Cross-platform build script
├── README.md           # This documentation
└── LICENSE             # MIT License
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