# Generate Icons Script
# Creates PNG icons from SVG for Tauri

$ErrorActionPreference = "Stop"

Write-Host "Generating icons for Tauri..." -ForegroundColor Cyan

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir
$iconsDir = Join-Path $projectRoot "src-tauri\icons"

# Check if ImageMagick is available
$hasMagick = Get-Command "magick" -ErrorAction SilentlyContinue

if ($hasMagick) {
    Write-Host "Using ImageMagick to generate icons..." -ForegroundColor Yellow

    $svgPath = Join-Path $iconsDir "icon.svg"

    # Generate PNG icons
    magick convert $svgPath -resize 32x32 (Join-Path $iconsDir "32x32.png")
    magick convert $svgPath -resize 128x128 (Join-Path $iconsDir "128x128.png")
    magick convert $svgPath -resize 256x256 (Join-Path $iconsDir "128x128@2x.png")

    # Generate ICO (Windows icon)
    magick convert $svgPath -resize 256x256 (Join-Path $iconsDir "icon.ico")

    # Generate ICNS (macOS icon) - if needed
    # magick convert $svgPath -resize 512x512 (Join-Path $iconsDir "icon.icns")

    Write-Host "Icons generated successfully!" -ForegroundColor Green
} else {
    Write-Host "ImageMagick not found. Creating placeholder icons..." -ForegroundColor Yellow
    Write-Host "For production, install ImageMagick: winget install ImageMagick.ImageMagick" -ForegroundColor White

    # Create minimal placeholder PNG (1x1 green pixel as base64)
    # In production, use proper icon generation
    $placeholder = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
    $bytes = [Convert]::FromBase64String($placeholder)

    $sizes = @("32x32.png", "128x128.png", "128x128@2x.png")
    foreach ($size in $sizes) {
        $path = Join-Path $iconsDir $size
        [IO.File]::WriteAllBytes($path, $bytes)
        Write-Host "  Created placeholder: $size" -ForegroundColor White
    }

    # For ICO, we need a proper file - create empty placeholder
    $icoPath = Join-Path $iconsDir "icon.ico"
    Copy-Item (Join-Path $iconsDir "32x32.png") $icoPath -Force

    Write-Host ""
    Write-Host "Note: Replace placeholder icons with proper icons before release!" -ForegroundColor Yellow
}
