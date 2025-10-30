# Owonero — TODO

## High-priority tasks

- Finish replacing remaining hardcoded `"blockchain.json"` occurrences with `config::get_blockchain_path()` (ensures all components use the same config-dir chain file).
- Verify end-to-end coinbase persistence:
  - Start the daemon and miner that include diagnostic logs.
  - Capture miner: `[miner] submitting block JSON: {...}`
  - Capture daemon: `[daemon] received submitblock JSON: {...}`
  - If miner JSON contains coinbase but daemon JSON does not, investigate the receive/parse/validation path.
- If daemon rejects coinbase TXs due to missing signature/pub_key, either mark coinbase as exempt from signature validation or construct a minimal coinbase `pub_key` value when mining.