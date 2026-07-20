# Hypixel Bazaar TUI

<p align="center">
  <img width="256" height="256" alt="app icon" src="https://github.com/user-attachments/assets/0da9be5f-2f7a-4ca2-9c26-658f06566c71" />
</p>

<p align="center">
  <a href="https://crates.io/crates/hypixel-bazaar-tui"><img alt="crates.io" src="https://img.shields.io/crates/v/hypixel-bazaar-tui.svg" /></a>
  <a href="https://crates.io/crates/hypixel-bazaar-tui"><img alt="downloads" src="https://img.shields.io/crates/d/hypixel-bazaar-tui.svg" /></a>
  <a href="./LICENSE"><img alt="license" src="https://img.shields.io/crates/l/hypixel-bazaar-tui.svg" /></a>
</p>

A terminal UI for browsing the Hypixel Skyblock Bazaar. Search products, check buy/sell prices and spreads, and look at price history charts, all from the terminal.

## Screenshots

<img width="997" height="737" alt="Screenshot 2026-07-20 at 12 37 41 PM" src="https://github.com/user-attachments/assets/ff3e0d1f-d419-4c63-8b13-0113d93b8e22" />


<img width="997" height="737" alt="Screenshot 2026-07-20 at 12 38 15 PM" src="https://github.com/user-attachments/assets/f69c9a98-db8c-4b17-b3d5-0037fb340e8c" />


## Install

```bash
cargo install hypixel-bazaar-tui
```

Or build from source:

```bash
git clone https://github.com/Feromond/hypixel-bazaar-tui.git
cd hypixel-bazaar-tui
cargo run --release
```

## Usage

Start typing to search for a product, then `Enter` to open it.

**Search view**

| Key | Action |
| --- | --- |
| `Up` / `Down` | Move selection |
| `Ctrl+Up` / `Ctrl+Down` | Jump to top / bottom |
| `PageUp` / `PageDown` | Jump 20 rows |
| `Ctrl+S` | Toggle sort (relevance / flip profit) |
| `Enter` | Open product |
| `Esc` | Clear search, or quit if empty |

**Product view**

| Key | Action |
| --- | --- |
| `p` | Toggle chart % / absolute mode |
| `m` | Toggle SMA overlay |
| `g` | Toggle midline |
| `r` | Refresh |
| `Esc` / `b` | Back to search |

`Ctrl+C` quits from anywhere.

## Building on Windows

The build script bundles `icons/hypixel-bazaar-tui.ico` into the binary. Swap that file out if you want a different icon, then `cargo build --release`.

## License

MIT
