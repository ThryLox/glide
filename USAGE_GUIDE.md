# 📘 glide-kvm User & Developer Guide

Welcome to the comprehensive user guide and technical manual for **glide-kvm**. This document covers setup, configuration, network configuration, troubleshooting, and architecture details.

---

## 📖 Table of Contents
1. [System Requirements](#1-system-requirements)
2. [Network Configuration & Tailscale](#2-network-configuration--tailscale)
3. [Installation & Build Instructions](#3-installation--build-instructions)
4. [Using the GUI Controller](#4-using-the-gui-controller)
5. [Configuring Linux Systemd Background Service](#5-configuring-linux-systemd-background-service)
6. [Troubleshooting & FAQs](#6-troubleshooting--faqs)

---

## 1. System Requirements

### Supported Operating Systems
- **Windows:** Windows 10 / 11 (64-bit)
- **Linux:** Kali Linux, Ubuntu, Debian, Fedora, Arch Linux (Wayland & X11 supported)
- **macOS:** macOS Catalina (10.15) or newer

### Permissions
- **Linux:** Write access to `/dev/uinput` (User added to `input` group: `sudo usermod -aG input $USER`)
- **Windows:** Standard user execution (Accessibility permissions automatically requested by `rdev` when required)

---

## 2. Network Configuration & Tailscale

**glide-kvm** uses UDP sockets over ports **24800** (Input Stream) and **24801** (ReturnSignal Stream).

### Connecting over Tailscale VPN Mesh (Recommended)
Using Tailscale provides secure, end-to-end encrypted mesh connectivity between your devices across cellular networks, Wi-Fi, or distinct physical locations.

1. Install Tailscale on both machines (`https://tailscale.com`).
2. Run `tailscale status` on your target server machine to retrieve its Tailscale IP (e.g., `100.119.208.55`).
3. Enter this Tailscale IP into the **Target Machine IP** box inside the `glide-kvm` dashboard on your primary laptop.

---

## 3. Installation & Build Instructions

### 🐧 Installing Dependencies on Linux
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libevdev-dev autoconf automake libtool libx11-dev libxtst-dev libxdo-dev xdotool
```

### 🔨 Compiling Release Executables
```bash
git clone https://github.com/ThryLox/glide.git
cd glide
cargo build --release
```
The compiled release binary will be situated at `target/release/glide-kvm` (or `glide-kvm.exe` on Windows).

---

## 4. Using the GUI Controller

1. Launch `cargo run --release` on your primary laptop.
2. Enter your secondary machine's IP address (e.g., `100.119.208.55`).
3. Select your **Target Screen Placement** radio option (`Right`, `Left`, `Top`, `Bottom`) to reflect where your physical secondary monitor sits relative to your laptop.
4. Verify that **Your laptop screen resolution** matches your primary display (e.g., `W: 1920 H: 1080`).
5. Click **🟢 Connect & Start Glide**.

---

## 5. Configuring Linux Systemd Background Service

To run `glide-kvm` persistently in headless background mode on your secondary Linux computer:

### Automated Installation
```bash
cargo run --release -- --install-service
```
This automatically generates `~/.config/systemd/user/glide-kvm.service` and enables the unit.

### Managing the Service
- Check status: `systemctl --user status glide-kvm`
- View real-time packet logs: `journalctl --user -u glide-kvm -f`
- Restart background service: `systemctl --user restart glide-kvm`

---

## 6. Troubleshooting & FAQs

### ❓ Question: The mouse transfers over, but the cursor is stuck or moves sluggishly?
**Answer:** Ensure the screen resolution inputs (`W:` and `H:`) in the GUI dashboard match your physical laptop screen dimensions. This ensures mouse warp anchors align cleanly.

### ❓ Question: My keyboard isn't typing inside remote terminal windows?
**Answer:** Verify that Kali's background daemon has write access to `/dev/uinput`. Run `sudo usermod -aG input $USER` and restart Kali's background service (`systemctl --user restart glide-kvm`).

### ❓ Question: Control feels stuck on the remote computer?
**Answer:** Press **`[Ctrl + Escape]`** anytime on your host keyboard. This emergency panic shortcut immediately breaks all remote input grabs and restores 100% peripheral control to your laptop.
