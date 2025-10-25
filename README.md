# Owonero

A simple proof-of-work blockchain cryptocurrency written in Go.

## Features

- Proof-of-work mining with adjustable difficulty
- Peer-to-peer networking with automatic blockchain syncing
- Wallet system with address generation
- TCP-based communication protocol
- JSON-based blockchain storage

## Building

```bash
# Clone the repository
git clone https://github.com/tosterlolz/Owonero.git
cd Owonero/main

# Build the binary
./build.ps1 
```

## Quick Start

### 1. Start a Daemon (Network Node)

```bash
# Connect to existing network
./owonero -d -n owonero.yabai.buzz:6969 -p 6969

# Or start your own network
./owonero -d -p 6969
```

### 2. Start Mining

```bash
# Mine to local daemon
./owonero -m -n localhost:6969 -t 4

# Mine to remote daemon
./owonero -m -n owonero.yabai.buzz:6969 -t 4
```

### 3. Check Wallet Balance

```bash
# View wallet info and blockchain status
./owonero
```

## Usage


### Daemon TCP Commands

| Command         | Description                                 |
|-----------------|---------------------------------------------|
| `getchain`      | Get the full blockchain (JSON)              |
| `getheight`     | Get current blockchain height (int)         |
| `submitblock`   | Submit a mined block (JSON payload)         |
| `sendtx`        | Submit a signed transaction (JSON payload)  |
| `getpeers`      | Get list of known peers (JSON array)        |
| `addpeer`       | Add a new peer (address on next line)       |
| `removepeer`    | Remove a peer (address on next line)        |
| `getwallet`     | Get wallet info (address on next line)      |
| `mineractive`   | Report active miner (address on next line)  |
| `sync`          | Force blockchain sync with peers            |

**Example:**

```
getchain
{ ... blockchain JSON ... }

sendtx
{ ... transaction JSON ... }

addpeer
192.168.1.101:6969
ok

mineractive
OWO1234567890ABCDEF
ok
```

### Command Line Flags

#### Daemon Mode
- `-d`: Run as daemon
- `-p PORT`: Listening port (default: 34567)
- `-diff NUM`: Mining difficulty (default: 3)
- `-peers "ADDR1,ADDR2"`: Initial peer list
- `--no-init`: Skip blockchain initialization (rely on syncing)

#### Mining Mode
- `-m`: Start mining
- `-n HOST:PORT`: Node address to connect to
- `-w FILE`: Wallet file path (default: wallet.json)
- `-b NUM`: Number of blocks to mine (0 = infinite)
- `-t NUM`: Number of mining threads (default: 1)

#### General
- `-h`: Show help

## Network Protocol

Owonero uses a simple TCP-based protocol. All commands are text-based with optional JSON payloads.

### Example Session

```bash
# Connect to daemon
telnet localhost 6969

# Get blockchain height
getheight
98

# Get peer list
getpeers
["192.168.1.100:6969", "node.example.com:6969"]

# Add a peer
addpeer
192.168.1.101:6969
ok
```

## Mining

Mining uses proof-of-work with configurable difficulty. The hash must start with N zeros where N is the difficulty level.

### Mining Rewards

- 1 OWO per mined block
- Rewards are automatically credited to your wallet

### Performance

- Use multiple threads with `-t` flag
- Higher difficulty = more computation required
- Monitor hashrate in mining output

## Wallet

Wallets are stored as JSON files containing your address.

### Creating a Wallet

```bash
# First run will create wallet.json
./owonero
```

### Using Custom Wallet

```bash
./owonero -w mywallet.json
```

## Blockchain Syncing

Daemons automatically sync with peers every 30 seconds. You can also manually trigger sync:

```bash
echo "sync" | nc localhost 6969
```

### Peer Discovery

- Connect to known peers with `-peers` flag
- Peers exchange peer lists automatically
- Miners discover peers from their connected daemon

## Development

### Project Structure

```
src/
├── main.go      # Entry point and CLI
├── daemon.go    # Network daemon
├── miner.go     # Mining logic
├── wallet.go    # Wallet management
└── blockchain.go # Blockchain core
```

### Adding New Features

1. Protocol commands in `handleConn()` in main.go
2. CLI flags in `main()` function
3. Core logic in appropriate module

## Troubleshooting

### Cannot Connect to Node
- Ensure daemon is running
- Check firewall settings
- Verify address and port

### Mining Not Working
- Ensure connected to a running daemon
- Check wallet file exists
- Verify network connectivity

### Sync Issues
- Use `--no-init` for clean sync
- Check peer connectivity
- Review daemon logs

## License

This project is open source. See LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## Disclaimer

This is a educational project. Not intended for production use or financial transactions.</content>