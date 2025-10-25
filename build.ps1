# Build Owonero for Windows and Linux
# This script compiles the Go project for multiple platforms

param(
    [switch]$Release,
    [switch]$Help,
    [string]$Version
)

# Show help if requested
if ($Help) {
    Write-Host "Owonero Build Script" -ForegroundColor Cyan
    Write-Host "Usage: .\build.ps1 [-Release] [-Version <version>] [-Help]" -ForegroundColor White
    Write-Host ""
    Write-Host "Parameters:" -ForegroundColor Yellow
    Write-Host "  -Release    Create GitHub release with zips" -ForegroundColor White
    Write-Host "  -Version    Override version (default: read from main.go)" -ForegroundColor White
    Write-Host "  -Help       Show this help message" -ForegroundColor White
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor Yellow
    Write-Host "  .\build.ps1                          # Build binaries only" -ForegroundColor White
    Write-Host "  .\build.ps1 -Release                 # Build and create release" -ForegroundColor White
    Write-Host "  .\build.ps1 -Release -Version 1.2.3  # Build with custom version" -ForegroundColor White
    exit 0
}

# Stop on any error
$ErrorActionPreference = 'Stop'

# Ensure bin directory exists
$binDir = Join-Path $PSScriptRoot 'bin'
if (-not (Test-Path $binDir)) {
    New-Item -ItemType Directory -Path $binDir | Out-Null
}

# Get version from source code or parameter
if (-not $Version) {
    $version = Select-String -Path 'src\main.go' -Pattern 'const ver = "([^"]+)"' | ForEach-Object { $_.Matches.Groups[1].Value }
    if (-not $version) {
        Write-Error "Could not find version in main.go. Use -Version parameter to specify manually."
        exit 1
    }
} else {
    $version = $Version
    Write-Host "Using specified version: $version" -ForegroundColor Yellow
}

Write-Host "Building Owonero version $version" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan

# Kill any running instances
Write-Host "Stopping any running instances..." -ForegroundColor Yellow
taskkill.exe /F /IM owonero-amd64.exe > $null 2>&1
taskkill.exe /F /IM owonero > $null 2>&1

# Define build targets
$targets = @(
    @{ OS = 'windows'; Arch = 'amd64'; BinaryName = 'owonero-amd64.exe' },
    @{ OS = 'windows'; Arch = '386'; BinaryName = 'owonero-x86.exe' },
    @{ OS = 'linux'; Arch = 'amd64'; BinaryName = 'owonero-amd64' },
    @{ OS = 'linux'; Arch = '386'; BinaryName = 'owonero-x86' },
    @{ OS = 'linux'; Arch = 'arm64'; BinaryName = 'owonero-arm64' }
)

$buildResults = @()

foreach ($target in $targets) {
    Write-Host "Building for $($target.OS) ($($target.Arch))..." -ForegroundColor Green

    $env:GOOS = $target.OS
    $env:GOARCH = $target.Arch
    $binaryPath = Join-Path $binDir $target.BinaryName

    try {
        go build -ldflags "-X main.version=$version" -o $binaryPath ./src
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✓ Successfully built $($target.BinaryName)" -ForegroundColor Green
            $buildResults += @{ Target = $target; Success = $true; Path = $binaryPath }
        } else {
            Write-Host "✗ Failed to build for $($target.OS)/$($target.Arch)" -ForegroundColor Red
            $buildResults += @{ Target = $target; Success = $false; Path = $null }
        }
    } catch {
        Write-Host "✗ Error building for $($target.OS)/$($target.Arch): $($_.Exception.Message)" -ForegroundColor Red
        $buildResults += @{ Target = $target; Success = $false; Path = $null }
    }
}

# Reset environment variables
$env:GOOS = $null
$env:GOARCH = $null

# Show build summary
$successfulBuilds = $buildResults | Where-Object { $_.Success }
$failedBuilds = $buildResults | Where-Object { -not $_.Success }

Write-Host ""
Write-Host "Build Summary:" -ForegroundColor Cyan
Write-Host "==============" -ForegroundColor Cyan
Write-Host "Successful builds: $($successfulBuilds.Count)" -ForegroundColor Green
Write-Host "Failed builds: $($failedBuilds.Count)" -ForegroundColor Red

if ($failedBuilds.Count -gt 0) {
    Write-Host ""
    Write-Host "Failed targets:" -ForegroundColor Red
    foreach ($failed in $failedBuilds) {
        Write-Host "  - $($failed.Target.OS)/$($failed.Target.Arch)" -ForegroundColor Red
    }
}

if ($successfulBuilds.Count -eq 0) {
    Write-Error "No builds succeeded. Exiting."
    exit 1
}

if ($Release) {
    Write-Host ""
    Write-Host "Creating GitHub release..." -ForegroundColor Yellow

    # Check if gh CLI is available
    try {
        $null = gh --version
    } catch {
        Write-Error "GitHub CLI (gh) is not installed or not in PATH. Please install it from https://cli.github.com/"
        exit 1
    }

    # Create zip archives for successful builds
    $zipFiles = @()
    foreach ($build in $successfulBuilds) {
        $zipName = "owonero-$($build.Target.OS)-$($build.Target.Arch).zip"
        $zipPath = Join-Path $binDir $zipName

        Write-Host "Creating $zipName..." -ForegroundColor Yellow
        try {
            Compress-Archive -Path $build.Path -DestinationPath $zipPath -Force
            $zipFiles += $zipPath
            Write-Host "✓ Created $zipName" -ForegroundColor Green
        } catch {
            Write-Host "✗ Failed to create $zipName" -ForegroundColor Red
        }
    }

    if ($zipFiles.Count -eq 0) {
        Write-Error "No zip files created. Cannot create release."
        exit 1
    }

    # Create GitHub release
    $tagName = "v$version"
    $releaseName = "Owonero $version"
    $releaseNotes = @"
Automated release of Owonero $version

Built on $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')

Build targets:
$(foreach ($build in $successfulBuilds) { "  - $($build.Target.OS)/$($build.Target.Arch)`n" })
"@

    # Check if release already exists
    $existingRelease = gh release view $tagName 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Release $tagName already exists. Updating..." -ForegroundColor Yellow
        gh release delete $tagName -y
    }

    # Create new release
    gh release create $tagName --title $releaseName --notes $releaseNotes

    # Upload assets
    foreach ($zipFile in $zipFiles) {
        $fileName = Split-Path $zipFile -Leaf
        Write-Host "Uploading $fileName..." -ForegroundColor Yellow
        gh release upload $tagName $zipFile --clobber
        Write-Host "✓ Uploaded $fileName" -ForegroundColor Green
    }

    Write-Host ""
    Write-Host "Release $tagName created and assets uploaded successfully!" -ForegroundColor Green
    Write-Host "View release at: https://github.com/tosterlolz/Owonero/releases/tag/$tagName" -ForegroundColor Cyan
} else {
    Write-Host ""
    Write-Host "Skipping release creation. Use -Release flag to create GitHub release." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Build completed!" -ForegroundColor Cyan
