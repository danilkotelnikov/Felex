const sharp = require('sharp');
const pngToIco = require('png-to-ico').default;
const fs = require('fs');
const path = require('path');

const iconsDir = path.join(__dirname, '..', 'src-tauri', 'icons');

// Create a simple icon with "F" letter and green gradient background
async function createIcon(size) {
  // Create SVG with the Felex "F" logo
  const svg = `
    <svg width="${size}" height="${size}" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style="stop-color:#22c55e;stop-opacity:1" />
          <stop offset="100%" style="stop-color:#16a34a;stop-opacity:1" />
        </linearGradient>
      </defs>
      <rect width="${size}" height="${size}" rx="${size * 0.15}" fill="url(#grad)"/>
      <text x="50%" y="55%" dominant-baseline="middle" text-anchor="middle"
            font-family="Arial, sans-serif" font-weight="bold" font-size="${size * 0.65}"
            fill="white">F</text>
    </svg>
  `;

  return sharp(Buffer.from(svg)).png().toBuffer();
}

async function main() {
  console.log('Generating Felex icons...');

  // Ensure icons directory exists
  if (!fs.existsSync(iconsDir)) {
    fs.mkdirSync(iconsDir, { recursive: true });
  }

  // Generate PNG icons at different sizes
  const sizes = [
    { name: '32x32.png', size: 32 },
    { name: '128x128.png', size: 128 },
    { name: '128x128@2x.png', size: 256 },
  ];

  for (const { name, size } of sizes) {
    const buffer = await createIcon(size);
    const filePath = path.join(iconsDir, name);
    fs.writeFileSync(filePath, buffer);
    console.log(`Created ${name} (${size}x${size})`);
  }

  // Generate icon.icns placeholder for macOS (just copy 256x256)
  const icnsBuffer = await createIcon(256);
  fs.writeFileSync(path.join(iconsDir, 'icon.icns'), icnsBuffer);
  console.log('Created icon.icns (256x256 placeholder)');

  // Generate ICO file for Windows - use 256x256 PNG file path
  const png256Path = path.join(iconsDir, '128x128@2x.png');

  try {
    const icoBuffer = await pngToIco(png256Path);
    fs.writeFileSync(path.join(iconsDir, 'icon.ico'), icoBuffer);
    console.log('Created icon.ico from 256x256 PNG');
  } catch (err) {
    console.error('ICO generation error:', err.message);
    // Create minimal ICO manually
    const pngBuffer = fs.readFileSync(png256Path);
    fs.writeFileSync(path.join(iconsDir, 'icon.ico'), pngBuffer);
    console.log('Created icon.ico (fallback)');
  }

  console.log('\nAll icons generated successfully!');
}

main().catch(console.error);
