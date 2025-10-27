# Building Owonero from Source

This document covers step-by-step build instructions for Linux, WSL, macOS, and notes for Windows users.

## Recommended: Windows users

- We publish signed release binaries on GitHub Releases. If you are on Windows and don't need to modify the code, download the latest release instead of building from source.

## Linux (native)

Prerequisites (Debian/Ubuntu):

```bash
sudo apt update
sudo apt install -y build-essential curl git pkg-config libssl-dev ca-certificates
```

Install Rust (if not installed):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
rustup update
```

Build:

```bash
git clone https://github.com/tosterlolz/Owonero.git
cd Owonero
cargo build --release
# Linux release binary: target/release/owonero-rs
```

Cross-target build (from Linux host to linux target is identical). To build for other targets use `--target` and install the target with `rustup target add`.

## WSL (Windows Subsystem for Linux)

If you prefer building the Linux binary from Windows, using WSL is the easiest path.

1. Install WSL (Windows 10/11): https://learn.microsoft.com/windows/wsl/install
2. Open your WSL distro (Ubuntu recommended) and follow the Linux instructions above.

Note: If you invoke the repository's `build.ps1` from PowerShell it will run the Linux build inside WSL automatically (when WSL is available). Make sure `rustup` and `cargo` are installed inside the WSL distro and that `pkg-config` and `libssl-dev` are installed (see Linux section above).

## macOS

Prerequisites:

```bash
# Install Xcode command line tools
xcode-select --install
# Install Homebrew (if you don't have it): https://brew.sh/
brew install pkg-config openssl
```

Then install Rust and build:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
cargo build --release
# macOS binary: target/release/owonero-rs
```

## Common build failure: `openssl-sys` cannot find OpenSSL

If you see an error from `openssl-sys` complaining it can't find OpenSSL or `pkg-config`:

- On Debian/Ubuntu (WSL/LINUX): `sudo apt install pkg-config libssl-dev`
- On Fedora: `sudo dnf install pkgconf-pkg-config openssl-devel`
- On macOS with Homebrew: `brew install openssl pkg-config` and set `PKG_CONFIG_PATH` if needed.

You can also point the build to a custom OpenSSL installation:

```bash
export OPENSSL_DIR=/path/to/openssl
export PKG_CONFIG_PATH=$OPENSSL_DIR/lib/pkgconfig
cargo build --release --target x86_64-unknown-linux-gnu
```

## Cross-compilation notes

- Cross-compiling Windows and macOS binaries from Linux/Windows requires additional toolchains and linkers (mingw-w64, osxcross, etc.). For most users building on the target platform or using WSL is simpler.
- Our `build.ps1` script tries to build the Linux target inside WSL when run from PowerShell. If you see a message about missing `rustup`/`cargo` inside WSL, install Rust inside the WSL distro and re-run the script.

## Release binaries

Windows users: prefer the pre-built release assets on GitHub Releases. They are easiest and avoid native dependency headaches.

---
| Tip: If you want automated reproducible builds we can add a Dockerfile or GitHub Actions workflow to produce release artifacts. Let us know if you'd like that added.
