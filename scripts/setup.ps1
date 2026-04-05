# Felex Setup Script
# This script checks and installs all required dependencies for Felex

$ErrorActionPreference = "Stop"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Felex - Setup Script" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Check if running as administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "Warning: Not running as administrator. Some installations may require elevation." -ForegroundColor Yellow
}

# Function to check if a command exists
function Test-Command {
    param($Command)
    $null = Get-Command $Command -ErrorAction SilentlyContinue
    return $?
}

# Function to check version
function Get-VersionNumber {
    param($VersionString)
    if ($VersionString -match '(\d+)\.(\d+)') {
        return [int]$Matches[1] * 100 + [int]$Matches[2]
    }
    return 0
}

Write-Host "Checking dependencies..." -ForegroundColor Yellow
Write-Host ""

# Check Node.js
Write-Host "[1/6] Checking Node.js..." -ForegroundColor White
if (Test-Command "node") {
    $nodeVersion = node --version
    Write-Host "  Found: Node.js $nodeVersion" -ForegroundColor Green

    $versionNum = Get-VersionNumber $nodeVersion
    if ($versionNum -lt 1800) {
        Write-Host "  Warning: Node.js 18+ recommended" -ForegroundColor Yellow
    }
} else {
    Write-Host "  Node.js not found. Installing via winget..." -ForegroundColor Yellow
    if (Test-Command "winget") {
        winget install OpenJS.NodeJS.LTS -e --accept-source-agreements --accept-package-agreements
    } else {
        Write-Host "  Please install Node.js manually from: https://nodejs.org/" -ForegroundColor Red
        Write-Host "  Then run this script again." -ForegroundColor Red
        exit 1
    }
}

# Check npm
Write-Host "[2/6] Checking npm..." -ForegroundColor White
if (Test-Command "npm") {
    $npmVersion = npm --version
    Write-Host "  Found: npm $npmVersion" -ForegroundColor Green
} else {
    Write-Host "  npm not found. It should be installed with Node.js." -ForegroundColor Red
    exit 1
}

# Check Rust
Write-Host "[3/6] Checking Rust..." -ForegroundColor White
if (Test-Command "rustc") {
    $rustVersion = rustc --version
    Write-Host "  Found: $rustVersion" -ForegroundColor Green
} else {
    Write-Host "  Rust not found. Installing via rustup..." -ForegroundColor Yellow

    # Download and run rustup
    $rustupUrl = "https://win.rustup.rs/x86_64"
    $rustupPath = "$env:TEMP\rustup-init.exe"

    Write-Host "  Downloading rustup..." -ForegroundColor White
    Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupPath

    Write-Host "  Running rustup installer..." -ForegroundColor White
    Start-Process -FilePath $rustupPath -ArgumentList "-y" -Wait

    # Refresh PATH
    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "User")

    if (Test-Command "rustc") {
        Write-Host "  Rust installed successfully!" -ForegroundColor Green
    } else {
        Write-Host "  Please restart your terminal and run this script again." -ForegroundColor Yellow
        exit 0
    }
}

# Check Cargo
Write-Host "[4/6] Checking Cargo..." -ForegroundColor White
if (Test-Command "cargo") {
    $cargoVersion = cargo --version
    Write-Host "  Found: $cargoVersion" -ForegroundColor Green
} else {
    Write-Host "  Cargo not found. Please reinstall Rust." -ForegroundColor Red
    exit 1
}

# Check Tauri CLI
Write-Host "[5/6] Checking Tauri CLI..." -ForegroundColor White
$tauriVersion = npm list -g @tauri-apps/cli 2>$null | Select-String "@tauri-apps/cli"
if ($tauriVersion) {
    Write-Host "  Found: Tauri CLI installed" -ForegroundColor Green
} else {
    Write-Host "  Installing Tauri CLI..." -ForegroundColor Yellow
    npm install -g @tauri-apps/cli
}

# Check for Visual Studio Build Tools
Write-Host "[6/6] Checking Visual Studio Build Tools..." -ForegroundColor White
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vsWhere) {
    $vsPath = & $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($vsPath) {
        Write-Host "  Found: Visual Studio Build Tools" -ForegroundColor Green
    } else {
        Write-Host "  Visual Studio Build Tools not found." -ForegroundColor Yellow
        Write-Host "  Please install Visual Studio Build Tools with C++ workload." -ForegroundColor Yellow
        Write-Host "  Download: https://visualstudio.microsoft.com/visual-cpp-build-tools/" -ForegroundColor White
    }
} else {
    Write-Host "  Visual Studio not found. Required for Rust compilation." -ForegroundColor Yellow
    Write-Host "  Download: https://visualstudio.microsoft.com/visual-cpp-build-tools/" -ForegroundColor White
}

Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Installing Project Dependencies" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Get script directory and project root
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

Write-Host "Project root: $projectRoot" -ForegroundColor White
Write-Host ""

# Install npm dependencies
Write-Host "Installing npm dependencies..." -ForegroundColor Yellow
Push-Location $projectRoot
npm install
Pop-Location

Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "  Setup Complete!" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor White
Write-Host "  1. Run 'npm run dev' to start development server" -ForegroundColor White
Write-Host "  2. Run 'npm run build' to build the application" -ForegroundColor White
Write-Host "  3. Run 'npm run tauri build' to create installer" -ForegroundColor White
Write-Host ""
Write-Host "For AI agent features, install Ollama:" -ForegroundColor White
Write-Host "  https://ollama.ai/download" -ForegroundColor Cyan
Write-Host "  Then run: ollama pull qwen3.5:4b" -ForegroundColor White
Write-Host "  Or for better quality: ollama pull qwen3.5:9b" -ForegroundColor White
Write-Host ""
