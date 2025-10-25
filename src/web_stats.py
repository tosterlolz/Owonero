"""
Owonero - Web Statistics Server
Async HTTP server providing blockchain and network statistics
"""

import json
import asyncio
from aiohttp import web
from typing import Dict, Any, Optional
import time

from utils import print_error, print_success, print_info, GREEN, RESET
from blockchain import Blockchain
from daemon import PeerManager


class WebStatsServer:
    """Async web statistics server using aiohttp"""

    def __init__(self, blockchain: Blockchain, peer_manager: PeerManager, port: int = 6767):
        self.blockchain = blockchain
        self.peer_manager = peer_manager
        self.port = port
        self.app = web.Application()
        self.runner = None
        self.site = None
        self.running = False

        # Setup routes
        self.app.router.add_get('/', self.serve_homepage)
        self.app.router.add_get('/api/stats', self.serve_stats)
        self.app.router.add_get('/api/blocks', self.serve_blocks)
        self.app.router.add_get('/api/peers', self.serve_peers)
        self.app.router.add_get('/api/blockchain', self.serve_blockchain)
        self.app.router.add_get('/api/block/{block_id}', self.serve_block)

    async def start(self) -> bool:
        """Start the async web server"""
        try:
            self.runner = web.AppRunner(self.app)
            await self.runner.setup()

            self.site = web.TCPSite(self.runner, '0.0.0.0', self.port)
            await self.site.start()

            self.running = True
            print_success(f"Web stats server started on http://localhost:{self.port}")
            return True

        except Exception as e:
            print_error(f"Failed to start web server: {e}")
            return False

    async def stop(self) -> None:
        """Stop the async web server"""
        self.running = False
        if self.site:
            await self.site.stop()
        if self.runner:
            await self.runner.cleanup()

    async def serve_homepage(self, request: web.Request) -> web.Response:
        """Serve the main statistics page"""
        height = self.blockchain.get_height()
        peer_count = len(await self.peer_manager.get_peers())

        html = f"""
<!DOCTYPE html>
<html>
<head>
    <title>Owonero Network Stats</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }}
        .header {{ background: linear-gradient(45deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px; border-radius: 10px; margin-bottom: 20px; }}
        .stats {{ display: flex; gap: 20px; margin-bottom: 20px; }}
        .stat-box {{ background: white; padding: 20px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); flex: 1; }}
        .stat-value {{ font-size: 2em; font-weight: bold; color: #667eea; }}
        .stat-label {{ color: #666; margin-top: 5px; }}
        table {{ width: 100%; border-collapse: collapse; background: white; border-radius: 10px; overflow: hidden; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background: #667eea; color: white; }}
        tr:hover {{ background: #f8f9fa; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>⛏️ Owonero Network Statistics</h1>
        <p>Real-time blockchain and network information</p>
    </div>

    <div class="stats">
        <div class="stat-box">
            <div class="stat-value">{height}</div>
            <div class="stat-label">Blockchain Height</div>
        </div>
        <div class="stat-box">
            <div class="stat-value">{len(self.blockchain.chain)}</div>
            <div class="stat-label">Total Blocks</div>
        </div>
        <div class="stat-box">
            <div class="stat-value">{peer_count}</div>
            <div class="stat-label">Connected Peers</div>
        </div>
    </div>

    <h2>Recent Blocks</h2>
    <table>
        <tr>
            <th>Height</th>
            <th>Timestamp</th>
            <th>Transactions</th>
            <th>Hash</th>
        </tr>
"""

        # Add recent blocks (last 10)
        for block in reversed(self.blockchain.chain[-10:]):
            timestamp = block.timestamp.replace('T', ' ').replace('Z', '')
            short_hash = block.hash[:16] + "..."
            html += f"""
        <tr>
            <td>{block.index}</td>
            <td>{timestamp}</td>
            <td>{len(block.transactions)}</td>
            <td><code>{short_hash}</code></td>
        </tr>
"""

        html += """
    </table>

    <br>
    <p><small>API Endpoints: <a href="/api/stats">/api/stats</a> | <a href="/api/blocks">/api/blocks</a> | <a href="/api/peers">/api/peers</a></small></p>
</body>
</html>
"""

        return web.Response(text=html, content_type='text/html')

    async def serve_stats(self, request: web.Request) -> web.Response:
        """Serve general statistics as JSON"""
        stats = {
            'blockchain_height': self.blockchain.get_height(),
            'total_blocks': len(self.blockchain.chain),
            'total_transactions': sum(len(block.transactions) for block in self.blockchain.chain),
            'peer_count': len(await self.peer_manager.get_peers()),
            'timestamp': time.time()
        }

        return web.json_response(stats)

    async def serve_blocks(self, request: web.Request) -> web.Response:
        """Serve recent blocks as JSON"""
        # Return last 50 blocks
        blocks = []
        for block in self.blockchain.chain[-50:]:
            blocks.append({
                'index': block.index,
                'timestamp': block.timestamp,
                'transactions': len(block.transactions),
                'hash': block.hash,
                'prev_hash': block.prev_hash[:16] + "..."
            })

        return web.json_response(blocks)

    async def serve_peers(self, request: web.Request) -> web.Response:
        """Serve peer list as JSON"""
        peers = await self.peer_manager.get_peers()
        return web.json_response(peers)

    async def serve_blockchain(self, request: web.Request) -> web.Response:
        """Serve full blockchain as JSON"""
        # Warning: This could be very large for big blockchains
        chain_data = [block.to_dict() for block in self.blockchain.chain]
        return web.json_response(chain_data)

    async def serve_block(self, request: web.Request) -> web.Response:
        """Serve specific block by index"""
        try:
            block_id = int(request.match_info['block_id'])
            block = self.blockchain.get_block(block_id)

            if block:
                return web.json_response(block.to_dict())

        except (ValueError, KeyError):
            pass

        raise web.HTTPNotFound()


async def start_web_stats_server(blockchain: Blockchain, peer_manager: PeerManager, port: int = 6767) -> Optional[WebStatsServer]:
    """Start the async web statistics server"""
    server = WebStatsServer(blockchain, peer_manager, port)
    if await server.start():
        return server
    return None