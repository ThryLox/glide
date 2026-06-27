# ⚡ glide-kvm

<p align="center">
  <b>Ultra-Low Latency Cross-Platform Software KVM written in Rust</b><br>
  <i>Seamless mouse, keyboard, clipboard, and file sharing across Windows, Linux, and macOS.</i>
</p>

---

## 🎨 Overview

**glide-kvm** is a next-generation, high-performance Software KVM (Keyboard, Video & Mouse / Peripheral sharing engine) engineered from the ground up in Rust. It transforms multiple physical machines running different operating systems (such as Windows laptops, Kali Linux workstations, and macOS devices) into a unified, fluid multi-monitor desktop environment.

Built with atomic memory primitives, kernel-level `/dev/uinput` virtual devices, and custom binary UDP network serialization, **glide-kvm** delivers real-time input passthrough with sub-millisecond response latency over local networks and VPN meshes like Tailscale.

---

## ✨ Features

- **🚀 Ultra-Low Latency Network Engine:** Custom UDP binary serialization using `serde` and `bincode` over persistent sockets for sub-millisecond input passthrough.
- **🖥️ Seamless Bidirectional Edge Gliding:** Move your physical mouse off the boundary edge of your laptop screen to glide directly into your secondary desktop, and return naturally when hitting the reverse screen boundary.
- **⌨️ Unified Hardware Peripheral Context:** Combines virtual mouse and keyboard handles into a single `/dev/uinput` peripheral device so Linux desktop compositors (GNOME Wayland, KDE) pair active keyboard focus with hover coordinates automatically.
- **🛡️ Exclusive OS Input Interception (`rdev::grab`):** When operating on a remote desktop, host inputs are completely captured and suppressed at the OS hardware level—preventing accidental dual-screen clicking or window focus leaks.
- **🚨 Emergency Panic & KVM Hotkeys:**
  - `[Scroll Lock]`: Toggle input focus between host and remote instantly.
  - `[Ctrl + Alt + G]`: Force direct switch to remote workstation.
  - `[Ctrl + Escape]`: Emergency panic switch to break all remote locks and return 100% control to host laptop.
- **📋 Cross-OS Clipboard Synchronization:** Instant real-time text, rich formatting, and clipboard buffer syncing powered by `arboard`.
- **📁 File Drag & Drop Transfer:** Stream chunks of documents directly off the screen boundary for native file reception across operating systems.
- **🖥️ Native `egui` Dashboard:** Modern dark-mode GUI controller with visual screen orientation radio selection (`Right`, `Left`, `Top`, `Bottom`), customizable screen resolution scaling, and atomic real-time network packet telemetry counters.
- **⚙️ Background Daemon Mode (`systemctl --user`):** Automated systemd service generator (`--install-service`) for persistent, unattended boot-time server execution on headless or desktop daemons.

---

## 🏗️ Architecture

```
+------------------------------------+          UDP Network Stream          +------------------------------------+
|         Client (Host Laptop)       |  ==================================> |       Server (Workstation)         |
|  - System-wide rdev OS Grab Hook   |      Port 24800 (Input Events)       |  - Tokio Async UDP Listener        |
|  - Atomic State Engine (Host/Rem)  |                                      |  - Kernel /dev/uinput Driver       |
|  - Win32/macOS Hardware Intercept  |  <================================== |  - Wayland / X11 Fallback Handler  |
+------------------------------------+      Port 24801 (ReturnToHost)       +------------------------------------+
```

---

## 🚀 Quick Start

### Prerequisites & Dependencies

#### 🐧 Linux (Ubuntu / Kali / Debian / Fedora / Arch)
Before compiling or running on Linux, ensure build toolchains and X11 development headers are installed:
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libevdev-dev autoconf automake libtool libx11-dev libxtst-dev libxdo-dev xdotool
```

#### 🪟 Windows
Ensure you have the latest Rust toolchain installed:
```powershell
winget install Rustlang.Rustup
```

---

### Installation & Execution

#### 1. Clone the Repository
```bash
git clone https://github.com/ThryLox/glide.git
cd glide
```

#### 2. Run the Desktop GUI Controller (Default Mode)
Simply launch `cargo run` without flags to open the native `egui` control dashboard:
```bash
cargo run --release
```

#### 3. Run in Background Server Mode
On your target machine (e.g., secondary Linux workstation), start the background listener:
```bash
cargo run --release -- --server
```

#### 4. Install Automated Background Daemon (Linux Service)
To configure `glide-kvm` to automatically start in the background whenever your workstation boots up:
```bash
cargo run --release -- --install-service
```

---

## 🎮 KVM Switch Hotkeys & Controls

| Hotkey | Target Action | Description |
|---|---|---|
| **`[Scroll Lock]`** | Toggle Focus | Switches mouse & keyboard control back and forth between Host and Remote. |
| **`[Ctrl + Alt + G]`** | Switch to Remote | Instantly directs peripheral input focus to your secondary workstation. |
| **`[Ctrl + Escape]`** | **Emergency Reset** | Instantly breaks remote input grab and restores 100% control to host laptop. |

---

## 📄 License
Distributed under the MIT License. See `LICENSE` for more details.
