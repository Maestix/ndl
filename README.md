# NDL - Nhentai Downloader

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange)](https://www.rust-lang.org/) [![Tokio](https://img.shields.io/badge/Runtime-Tokio-teal)](https://tokio.rs) [![Tokio](https://img.shields.io/badge/Http-reqwest-yellow)](https://tokio.rs)
A minimalist Nhentai downloader written in Rust! 🦀

## Features

- ⚡ **Blazingly fast** 🚀downloads powered by Rust's async/await and Tokio's multi-threaded runtime with non-blocking requests
- 📚 Download entire manga galleries with just a link
- 🌐 **Multi-platform support** (Windows, Linux, macOS)
- 👌 **Zero configuration** - just run and download
- 🔒 No registration, no API keys, no nonsense
- 🚀 Automatic concurrent downloads

## Installation

### From Source
```bash
git clone https://github.com/Maestix/ndl
cd ndl
cargo build --release
# Binary will be at target/release/ndl
```
### Usage
```bash
./ndl <link to nhentai manga>
```