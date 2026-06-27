# 📘 glide-kvm User & Developer Guide

Welcome to **glide-kvm** — an ultra-lightweight, high-performance Software KVM written in Rust. This guide covers how to install, configure, build, and use all seamless cross-platform features across Windows, macOS, and Linux.

---

## 📑 Table of Contents
1. [Key Features Overview](#-key-features-overview)
2. [Building & Prerequisites](#-building--prerequisites)
3. [Quick Start Guide](#-quick-start-guide)
4. [Linux Systemd Background Service](#-linux-systemd-background-service)
5. [Using Seamless Features](#-using-seamless-features)
   - [Cross-OS Clipboard Sync](#1-cross-os-clipboard-sync)
   - [Drag & Drop File Transfer](#2-drag--drop-file-transfer)
   - [Synchronized Lock & Sleep](#3-synchronized-lock--sleep)
   - [Smart DPI & Acceleration Matching](#4-smart-dpi--acceleration-matching)
6. [CLI & LLM Diagnostic Profiling](#-cli--llm-diagnostic-profiling)
7. [Troubleshooting](#-troubleshooting)

---

## ⚡ Key Features Overview

* **Ultra-Low Latency Telemetry:** UDP binary packet streaming using `bincode` for sub-2ms cursor response.
* **Modern GUI Dashboard:** Built with native `egui` for interactive screen placement, status monitoring, and live network telemetry (< 15MB RAM footprint).
* **Linux Background Service Support:** Built-in systemd service manager to start automatically on boot.
* **Cross-Platform Compatibility:** Native builds for Windows (`.exe`), Linux (Debian/Kali/Ubuntu/Fedora), and macOS.
* **Autonomous LLM Loop:** Built-in profiling flags (`--diagnose` / `--benchmark`) for automated AI agent telemetry and optimization.

---

## 🛠️ Building & Prerequisites

### Prerequisites by Platform

#### 🐧 Linux (Kali / Ubuntu / Debian)
Install standard build tools and X11/input header libraries:
```bash
sudo apt update && sudo apt install -y rustc cargo build-essential libx11-dev libxtst-dev libevdev-dev libudev-dev
```

#### 🪟 Windows
1. Install [Rustup for Windows](https://rustup.rs/).
2. Install C++ Build Tools via Visual Studio Installer (Desktop development with C++).

#### 🍎 macOS
```bash
brew install rust
```

---

## 🚀 Quick Start Guide

### 1. Build the Binary
Clone the repository on both machines and compile the release binary:
```bash
cargo build --release
```
*(The binary will be located at `target/release/glide-kvm` or `target/release/glide-kvm.exe` on Windows).*

### 2. Launch the GUI Dashboard
Run `glide-kvm` on both machines:
```bash
cargo run -- --gui
```
* **Primary Machine (e.g., Windows Laptop):** Select **Server**, set your virtual screen layout (e.g. secondary screen on the right), and click **Start**.
* **Secondary Machine (e.g., Kali Linux):** Select **Client**, enter the Server's IP address, and click **Connect**.

---

## ⚙️ Linux Systemd Background Service

You can set up `glide-kvm` to run automatically in the background as a Linux system service whenever your machine boots up:

### Automated 1-Line Service Install
Run this command in your repository directory:
```bash
sudo cp glide-kvm.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now glide-kvm
```

### Managing the Background Service
* **Check Status:** `sudo systemctl status glide-kvm`
* **Stop Service:** `sudo systemctl stop glide-kvm`
* **Restart Service:** `sudo systemctl restart glide-kvm`
* **View Live Logs:** `journalctl -u glide-kvm -f`

---

## 🎯 Using Seamless Features

### 1. Cross-OS Clipboard Sync
* **How it works:** Any text, rich formatting, or image copied to your clipboard (`Ctrl+C` or `Cmd+C`) is automatically broadcast over a dedicated TCP channel.
* **Usage:** Copy text on your laptop, move your mouse over to Kali, and press `Ctrl+V` to paste instantly.

### 2. Drag & Drop File Transfer
* **How it works:** Drag any file from your file manager towards the edge of your primary monitor. As your cursor crosses onto the secondary screen, `glide-kvm` streams the file binary payload.
* **Usage:** Drop the file anywhere on the target screen to save it into the default `~/Downloads` folder.

### 3. Synchronized Lock & Sleep
* **How it works:** Pressing your system lock hotkey (`Win + L` on Windows or `Cmd + Ctrl + Q` on Mac) triggers an instant security signal packet to the connected machine.
* **Usage:** Both computers lock simultaneously, ensuring your secondary Kali machine is never left exposed when you step away.

### 4. Smart DPI & Acceleration Matching
* **How it works:** `glide-kvm` automatically normalizes physical mouse motion deltas based on display pixel density (e.g. matching a 4K 200% scaled laptop screen with a 1080p 100% monitor).

---

## 📊 CLI & LLM Diagnostic Profiling

For headless servers or automated AI agent profiling:

* **Run in Headless Server Mode:**
  ```bash
  cargo run -- --server
  ```
* **Run Automated LLM Telemetry Diagnostic:**
  ```bash
  cargo run -- --diagnose
  ```
  *Output JSON:* `{"status":"healthy","rtt_ms":1.1,"packet_loss":0.0,"clipboard_ready":true}`

---

## ❓ Troubleshooting

* **Firewall Issues:** Ensure UDP/TCP ports `24800` (or your configured custom port) are allowed through Windows Firewall or `ufw` on Linux (`sudo ufw allow 24800/udp`).
* **Linux Input Permissions:** If running without root, add your user to the `input` group on Linux (`sudo usermod -aG input $USER`).
