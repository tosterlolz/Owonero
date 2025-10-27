# Troubleshooting

This page collects common issues when building and running Owonero and how to fix them.

## 1) WSL: `rustup` or `cargo` not found

Symptom: `build.ps1` reports "rustup: command not found" when attempting to build inside WSL.

Fix: Install Rust inside your WSL distro:

```bash
# inside WSL
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
rustup update
```

Then re-run `.uild.ps1` from PowerShell or build inside WSL manually.

## 2) `openssl-sys` cannot find OpenSSL

Symptom: Cargo build fails with `Could not find directory of OpenSSL installation` and references `openssl-sys`.

Fix (Debian/Ubuntu / WSL):

```bash
sudo apt update
sudo apt install -y pkg-config libssl-dev
```

Fix (Fedora/CentOS):

```bash
sudo dnf install -y pkgconf-pkg-config openssl-devel
```

Fix (macOS):

```bash
brew install openssl pkg-config
# If pkg-config can't find openssl, set:
export PKG_CONFIG_PATH="$(brew --prefix openssl)/lib/pkgconfig"
```

If OpenSSL is installed in a custom location, set the `OPENSSL_DIR` and `PKG_CONFIG_PATH` environment variables to point to the correct directories before running `cargo build`.

## 3) Missing `pkg-config`

Symptom: Cargo build fails saying `pkg-config` is not found.

Fix (Debian/Ubuntu): `sudo apt install -y pkg-config`

## 4) Permission denied when overwriting binary on Windows

Symptom: `cargo build` fails to overwrite an existing exe because it's running or locked.

Fix: Close any running instances of `owonero.exe` (Task Manager) or reboot. Alternatively, remove the binary manually after stopping the process.

## 5) Low hashrate / mining stalls

Symptom: Initial high hashrate for a few seconds then drop to low sustained H/s.

Possible causes & mitigation:
- Workers are blocking on submission channels; the code includes forwarder threads and std channels to avoid this. Ensure you're running the latest code that sends to std channels and uses forwarders.
- Open file handles or antivirus on Windows interfering. Exclude the project directory from antivirus scans.
- CPU frequency scaling / thermal throttling. Monitor `htop`/Task Manager and ensure CPU governor is performance.
- If you used `OWONERO_MINING_ITERATIONS`, try tuning it down to verify CPU-bound behavior.

## 6) Further help

If you still have trouble, gather build logs and the output from running `.uild.ps1` and open an issue on GitHub with the logs attached: https://github.com/tosterlolz/Owonero/issues
