#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Cross-compiles a Rust project for Windows and Linux AMD64.
#>

# Stop on errors
$ErrorActionPreference = "Stop"

Write-Host "=== Rust Cross-Compilation Script ===" -ForegroundColor Cyan

if ($IsLinux) {
    Write-Host "Running on Linux host." -ForegroundColor Yellow
} elseif ($IsWindows) {
    Write-Host "Running on Windows host." -ForegroundColor Yellow
} else {
    Write-Error "Unsupported OS. This script only supports Windows and Linux hosts."
    exit 1
}

# Add required targets
$targets = @("x86_64-pc-windows-gnu", "x86_64-unknown-linux-gnu")
foreach ($target in $targets) {
    Write-Host "Adding Rust target: $target"
    rustup target add $target | Out-Null
}

# Install toolchains if needed
if ($IsLinux) {
    Write-Host "Running on Linux host — installing cross-linkers for Windows target..." -ForegroundColor Yellow
    sudo apt update
    sudo apt install -y mingw-w64 gcc-multilib
} elseif ($IsWindows) {
    Write-Host "Running on Windows host." -ForegroundColor Yellow
    # For building the Linux target from native Windows we prefer WSL or Docker.
    $wsl = Get-Command wsl -ErrorAction SilentlyContinue
    if (-not $wsl) {
        Write-Host "WSL not found. To build the Linux target from Windows please either: (1) install WSL and Rust inside it, or (2) use Docker or a Linux host." -ForegroundColor Yellow
        Write-Host "This script will still try to build the Windows target locally, but will skip the Linux target unless WSL is available." -ForegroundColor Yellow
    } else {
        Write-Host "WSL detected. Will build Linux target inside WSL." -ForegroundColor Green
    }
} else {
    Write-Host "Unknown host OS. Proceeding with target addition only." -ForegroundColor Yellow
}

# Build for both targets
Write-Host "Building Rust project for both targets..." -ForegroundColor Cyan
foreach ($target in $targets) {
    Write-Host "`n➡ Building for $target..." -ForegroundColor Green

    if ($IsWindows -and $target -like "*unknown-linux-gnu") {
        # On native Windows, build the Linux target inside WSL if available
        $wsl = Get-Command wsl -ErrorAction SilentlyContinue
        if ($wsl) {
            Write-Host "Building Linux target inside WSL..." -ForegroundColor Cyan
            # Check for rustup/cargo inside WSL and run build there. If missing, print instructions.
            $wsl_cmd = "if ! command -v rustup >/dev/null 2>&1 || ! command -v cargo >/dev/null 2>&1; then echo '__RUST_MISSING__'; else rustup target add $target || true; cargo build --release --target $target; fi"
            $wsl_output = wsl -e bash -lc $wsl_cmd 2>&1
            if ($wsl_output -match "__RUST_MISSING__") {
                Write-Host "Rust (rustup/cargo) was not found inside WSL. To build the Linux target inside WSL, install Rust in your WSL distro and re-run this script." -ForegroundColor Yellow
                Write-Host "Quick install inside WSL (run inside WSL):" -ForegroundColor Cyan
                Write-Host "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y" -ForegroundColor Green
                Write-Host "Then reopen the shell or source your profile and re-run this script from Windows (it will invoke WSL to build)." -ForegroundColor Yellow
            } else {
                Write-Host $wsl_output
            }
        } else {
            Write-Host "Skipping Linux target build because WSL is not available. Install WSL or run this script from a Linux host." -ForegroundColor Yellow
        }
    } else {
        # Native build (works on Linux or Windows for Windows-gnu target if proper linker is installed)
        cargo build --release --target $target
    }
}

# Output results
Write-Host "`n✅ Build complete!" -ForegroundColor Cyan
Write-Host "Windows binary: target/x86_64-pc-windows-gnu/release/owonero-rs.exe"
Write-Host "Linux binary:   target/x86_64-unknown-linux-gnu/release/owonero-rs"
