# Build Owonero for Windows and Linux
# This script compiles the Go project for multiple platforms

taskkill.exe /F /IM owonero.exe > $null 2>&1

# Stop on any error
$ErrorActionPreference = 'Stop'

# Ensure bin directory exists
$binDir = Join-Path $PSScriptRoot 'bin'
if (-not (Test-Path $binDir)) {
    New-Item -ItemType Directory -Path $binDir | Out-Null
}

# Build for Windows (amd64)
Write-Host 'Building for Windows (amd64)...' -ForegroundColor Green
$env:GOOS = 'windows'
$env:GOARCH = 'amd64'
go build -o (Join-Path $binDir 'owonero.exe') ./src
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed for Windows"
    exit 1
}

# Build for Linux (amd64)
Write-Host 'Building for Linux (amd64)...' -ForegroundColor Green
$env:GOOS = 'linux'
$env:GOARCH = 'amd64'
go build -o (Join-Path $binDir 'owonero') ./src
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed for Linux"
    exit 1
}

Write-Host 'All builds completed successfully!' -ForegroundColor Cyan
