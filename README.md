<p align="center">
  <img src="assets/icon.png" width="128" height="128" alt="Felex Logo">
</p>

<h1 align="center">Felex</h1>

<p align="center">
  <b>Professional Feed Ration Optimizer for Livestock</b><br>
  <i>Профессиональный оптимизатор рационов кормления</i>
</p>

<p align="center">
  <a href="https://github.com/danilkotelnikov/Felex/releases/latest"><img src="https://img.shields.io/github/v/release/danilkotelnikov/Felex?style=for-the-badge&color=2ea44f&label=Download" alt="Download"></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?style=for-the-badge&logo=windows" alt="Windows">
  <img src="https://img.shields.io/badge/license-MIT-orange?style=for-the-badge" alt="License">
  <img src="https://img.shields.io/github/downloads/danilkotelnikov/Felex/total?style=for-the-badge&color=blue&label=Downloads" alt="Downloads">
</p>

<p align="center">
  <a href="#-quick-install">Install</a> &bull;
  <a href="#-features">Features</a> &bull;
  <a href="#-screenshots">Screenshots</a> &bull;
  <a href="#-ai-assistant">AI Assistant</a> &bull;
  <a href="#-faq">FAQ</a>
</p>

---

## What is Felex?

**Felex** is a free desktop application that helps farmers, nutritionists, and researchers **calculate, optimize, and balance feed rations** for cattle (dairy & beef), swine, and poultry.

It combines **linear programming optimization** with an **AI-powered assistant** — all running locally on your computer, no internet required.

> **Felex** — бесплатное настольное приложение для расчёта, оптимизации и балансирования рационов кормления КРС (молочного и мясного), свиней и птицы. Сочетает линейное программирование с ИИ-ассистентом — всё работает локально, без интернета.

---

## Quick Install

### Option 1: Installer (Recommended)

1. Go to **[Releases](https://github.com/danilkotelnikov/Felex/releases/latest)**
2. Download **`Felex_1.0.0_x64-setup.exe`**
3. Run the installer — follow the wizard (2 clicks)
4. Launch **Felex** from the Start Menu or Desktop

> No admin rights required. Installs to your user profile.

### Option 2: MSI Installer

Download **`Felex_1.0.0_x64_en-US.msi`** (English) or **`Felex_1.0.0_x64_ru-RU.msi`** (Russian) from [Releases](https://github.com/danilkotelnikov/Felex/releases/latest).

### Option 3: Portable (No install)

1. Download **`Felex_1.0.0_x64_portable.zip`** from [Releases](https://github.com/danilkotelnikov/Felex/releases/latest)
2. Extract anywhere
3. Run **`Felex.exe`**

### System Requirements

| | Minimum | Recommended |
|---|---|---|
| **OS** | Windows 10 (64-bit) | Windows 11 |
| **RAM** | 4 GB | 8–16 GB (for AI features) |
| **Disk** | 50 MB | 500 MB (with AI model) |
| **CPU** | Any x64 | Intel i5 / AMD Ryzen 5+ |

---

## Features

### Core Calculation Engine

- **35+ nutrients tracked** — energy, protein, amino acids, fiber, minerals, vitamins
- **3 optimization modes** — minimize cost, balance nutrients, or fine-tune with fixed feeds
- **Real-time validation** — warnings for Ca:P imbalance, energy deficit, selenium toxicity, low NDF, and more
- **Economic analysis** — cost per day/month/year, cost per kg of milk/meat, cost breakdown by feed category

### Animals Supported

| Species | Production Types |
|---|---|
| **Dairy Cattle** | Dry period, Fresh cow, Early/Mid/Late lactation (20–35+ kg/day) |
| **Beef Cattle** | Growing (300–500+ kg), Finishing |
| **Swine** | Starter, Grower, Finisher, Gestating sows, Lactating sows |
| **Poultry** | Broiler (starter/grower/finisher), Layer (pre-lay, peak, late) |

### Feed Library

- **1000+ feeds** pre-loaded with complete nutrient profiles
- Russian and international feed databases
- Auto-import from gov.cap.ru government database
- Create custom feeds with your own lab data
- Price tracking with historical trends

### Smart Features

- **Drag-and-drop** feed ordering
- **Lock feeds** to keep specific amounts during optimization
- **Breed-specific adjustments** — norms adapt to Holstein, Simmental, Jersey, etc.
- **Export reports** — PDF, Excel, CSV
- **Dark & light themes**
- **Russian & English interface**
- **Workspace system** — organize rations into projects and folders

---

## AI Assistant

Felex includes a built-in **AI nutritionist** powered by local language models (no cloud, no data leaves your PC).

### What it can do:
- Suggest feeds to balance a deficient ration
- Explain nutrient interactions and requirements
- Answer feeding questions for specific animal types
- Search the feed library by nutrient profile

### Setup (optional — Felex works fully without AI):

1. Install [Ollama](https://ollama.ai/download) (free, 1-minute install)
2. Pull the model:
   ```
   ollama pull qwen3.5:4b
   ```
   *For better quality (needs 16 GB RAM):*
   ```
   ollama pull qwen3.5:9b
   ```
3. Start Felex — the AI connects automatically

> The AI assistant is **completely optional**. All calculation, optimization, and analysis features work without it.

---

## How It Works

```
┌──────────────┐     ┌───────────────────┐     ┌──────────────┐
│  You enter   │────▶│   Felex Engine     │────▶│  Optimized   │
│  feeds +     │     │  Linear Programming│     │  ration with │
│  animal data │     │  (Simplex Method)  │     │  min cost    │
└──────────────┘     └───────────────────┘     └──────────────┘
                              │
                     ┌────────▼────────┐
                     │  Validates vs   │
                     │  feeding norms  │
                     │  (NRC + Russian)│
                     └─────────────────┘
```

**Optimization** finds the cheapest feed combination that meets all nutrient requirements for your specific animal group. It solves in under 100 ms — even for complex rations with 30+ feeds.

---

## Quick Start Guide

### 1. Create a Ration
- Click **File → New Ration** or the **+** button
- Select the animal group (e.g., *Dairy Cattle — 30 kg milk/day*)
- Set animal count for your herd

### 2. Add Feeds
- Browse the **Feed Library** panel on the right
- Click **+** to add a feed, or drag it into the ration table
- Adjust amounts (kg/day per head)

### 3. Check Nutrients
- Switch to the **Nutrients** tab
- Green = OK, Yellow = borderline, Red = critical
- The status bar shows overall compliance

### 4. Optimize
- Click the **Optimize** button
- Choose mode: *Minimize Cost* or *Balance Nutrients*
- Review the result — Felex adjusts feed amounts to meet all norms at lowest cost

### 5. Export
- **File → Export** or use the **Report** tab
- Choose PDF, Excel, or CSV
- Share with your team or print for the barn

---

## Feeding Standards

Felex includes norms from authoritative sources:

- **Калашников А.П. и др.** — Нормы и рационы кормления с.-х. животных (2003)
- **NRC Dairy Cattle** — Nutrient Requirements of Dairy Cattle, 7th ed. (2001)
- **NRC Swine** — Nutrient Requirements of Swine, 11th ed. (2012)
- **NRC Poultry** — Nutrient Requirements of Poultry, 9th ed. (1994)

All norms are adjustable — override any value for your specific conditions.

---

## FAQ

**Q: Is Felex really free?**
Yes, completely free and open-source (MIT license). No subscriptions, no ads, no data collection.

**Q: Does it need internet?**
No. Felex works 100% offline. Internet is only needed for optional price updates and AI model download.

**Q: Can I add my own feeds?**
Yes. Click "Create Feed" and enter your lab analysis data. You can also import feeds from CSV/Excel.

**Q: How accurate is the optimizer?**
Felex uses the same mathematical method (linear programming / simplex) as commercial software like WinFeed and BESTMIX. Results are scientifically rigorous.

**Q: Is my data safe?**
Everything stays on your computer. No cloud, no telemetry, no accounts. Your rations are saved as local files.

**Q: What about Mac/Linux?**
Currently Windows only. Cross-platform support is planned for future releases.

---

## Tech Stack

Built with modern, high-performance technologies:

| Layer | Technology |
|---|---|
| Core Engine | **Rust** — memory-safe, C-level performance |
| Optimizer | **MinLP** — pure Rust linear programming solver |
| Database | **SQLite** — embedded, zero-config |
| Frontend | **React 18 + TypeScript + Tailwind CSS** |
| Desktop Shell | **Tauri 2.0** — 10x smaller than Electron |
| AI Backend | **Ollama** — local LLM inference |

---

## License

MIT License — free for personal, educational, and commercial use.

---

<p align="center">
  <b>Made for farmers, nutritionists, and researchers</b><br>
  <i>Создано для фермеров, зоотехников и исследователей</i>
</p>

<p align="center">
  <a href="https://github.com/danilkotelnikov/Felex/releases/latest">
    <img src="https://img.shields.io/badge/Download%20Felex-v1.0.0-2ea44f?style=for-the-badge&logo=windows" alt="Download">
  </a>
</p>
