# Felex Build Installer Script
# Creates MSI and NSIS installers for Windows

$ErrorActionPreference = "Stop"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Felex - Build Installer" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Get script directory and project root
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

Push-Location $projectRoot

try {
    # Check dependencies
    Write-Host "Checking build dependencies..." -ForegroundColor Yellow

    if (-not (Get-Command "node" -ErrorAction SilentlyContinue)) {
        throw "Node.js not found. Run setup.ps1 first."
    }

    if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
        throw "Rust/Cargo not found. Run setup.ps1 first."
    }

    # Add Windows SDK to PATH for RC.EXE (resource compiler)
    $windowsSdkPath = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
    if (Test-Path $windowsSdkPath) {
        $latestSdk = Get-ChildItem $windowsSdkPath -Directory | Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } | Sort-Object Name -Descending | Select-Object -First 1
        if ($latestSdk) {
            $rcPath = Join-Path $latestSdk.FullName "x64"
            if (Test-Path $rcPath) {
                $env:PATH = "$rcPath;$env:PATH"
                Write-Host "Added Windows SDK to PATH: $rcPath" -ForegroundColor Green
            }
        }
    }

    # Install dependencies if needed
    if (-not (Test-Path "node_modules")) {
        Write-Host "Installing npm dependencies..." -ForegroundColor Yellow
        npm install
    }

    # Build frontend
    Write-Host ""
    Write-Host "Building frontend..." -ForegroundColor Yellow
    npm run build

    if (-not $?) {
        throw "Frontend build failed"
    }

    # Build Tauri application
    Write-Host ""
    Write-Host "Building Tauri application..." -ForegroundColor Yellow
    Write-Host "This may take several minutes on first build..." -ForegroundColor White

    npm run tauri build

    if (-not $?) {
        throw "Tauri build failed"
    }

    # Find and display output
    Write-Host ""
    Write-Host "============================================" -ForegroundColor Green
    Write-Host "  Build Complete!" -ForegroundColor Green
    Write-Host "============================================" -ForegroundColor Green
    Write-Host ""

    # Bundle path - using custom target dir to avoid OneDrive locking
    $bundlePath = "C:\FelexBuild\tauri-target\release\bundle"

    # Also copy to project dist folder for easy access
    $distPath = Join-Path $projectRoot "dist"
    if (-not (Test-Path $distPath)) {
        New-Item -ItemType Directory -Path $distPath | Out-Null
    }

    if (Test-Path "$bundlePath\msi") {
        Write-Host "MSI Installers:" -ForegroundColor White
        Get-ChildItem "$bundlePath\msi\*.msi" | ForEach-Object {
            Write-Host "  $($_.FullName)" -ForegroundColor Cyan
            Copy-Item $_.FullName -Destination $distPath -Force
        }
    }

    if (Test-Path "$bundlePath\nsis") {
        Write-Host ""
        Write-Host "NSIS Installer:" -ForegroundColor White
        Get-ChildItem "$bundlePath\nsis\*.exe" | ForEach-Object {
            Write-Host "  $($_.FullName)" -ForegroundColor Cyan
            Copy-Item $_.FullName -Destination $distPath -Force
        }
    }

    Write-Host ""
    Write-Host "Installers copied to:" -ForegroundColor Green
    Write-Host "  $distPath" -ForegroundColor Cyan
    Write-Host ""

} catch {
    Write-Host ""
    Write-Host "Build failed: $_" -ForegroundColor Red
    exit 1
} finally {
    Pop-Location
}
