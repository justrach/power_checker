# Power Monitor

A powerful open-source tool that provides real-time insights into your device's energy consumption and carbon footprint, linked to your location.

## Features

- 🔌 **Real-time Power Monitoring**: Track your Mac's power consumption in real-time
- 🌍 **Carbon Footprint Tracking**: See the environmental impact of your device usage
- 📈 **Historical Data**: View power usage trends with interactive charts
- 🔄 **Auto-refresh**: Data updates every 5 seconds
- 💻 **macOS Support**: Built specifically for macOS systems

## Prerequisites

- macOS 13.x or later
- [Rust](https://www.rust-lang.org/tools/install)
- [Bun](https://bun.sh)

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/power-monitor.git
   cd power-monitor
   ```

2. Install dependencies:
   ```bash
   bun install
   ```

3. Run the development server:
   ```bash
   bun run tauri dev
   ```

## Building

To create a production build:

```bash
bun run tauri build
```

The built application will be available in the `src-tauri/target/release` directory.

## Technical Details

- Frontend: React + TypeScript + Vite
- Backend: Rust + Tauri
- UI Framework: Mantine
- Charts: Chart.js with react-chartjs-2

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see LICENSE file for details
