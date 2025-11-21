# Hypixel Bazaar TUI

A high-performance Terminal User Interface (TUI) for browsing the Hypixel Skyblock Bazaar, written in Rust.

This application allows you to monitor Bazaar prices, view product details, track price history with charts, and analyze buy/sell orders directly from your terminal.

## Features

- **Real-time Data**: Fetches live data from the Hypixel API.
- **Search & Filtering**: Quickly find products by name.
- **Detailed Analytics**:
  - View Buy/Sell prices and Spread.
  - Analyze market volume and moving averages.
  - Top Buy/Sell orders list.
- **Interactive Charts**: Visual price history with Simple Moving Averages (SMA).
- **Keyboard Navigation**: Efficient controls for power users.

## Screenshots

_(Screenshots coming soon)_

## Installation

### From Source

Ensure you have [Rust and Cargo installed](https://rustup.rs/).

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/hypixel-bazaar-tui.git
   cd hypixel-bazaar-tui
   ```

2. Build and run:
   ```bash
   cargo run --release
   ```

## Usage

- **Search**: Type to filter products.
- **Navigation**: Use `Up`/`Down` arrows to select a product.
- **Details**: Press `Enter` to view detailed stats and charts for the selected product.
- **Back**: Press `Esc` to go back or quit.
- **Sort**: `Ctrl+S` to toggle sorting modes (if implemented).

## Tech Stack

- **Rust**: Core language.
- **Ratatui**: TUI library for rendering.
- **Tokio**: Asynchronous runtime.
- **Reqwest**: HTTP client for API requests.

## Building for Windows

This project includes a build script to bundle an icon for Windows releases.

1. Place your `.ico` file in the `icons/` folder (e.g., `icons/hypixel-bazaar-tui.ico`).
2. The `build.rs` script is configured to use `icons/hypixel-bazaar-tui.ico`.
3. Run `cargo build --release`.
