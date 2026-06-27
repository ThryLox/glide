# ⚡ glide-kvm

**glide-kvm** is an ultra-lightweight, high-performance Software KVM (Keyboard & Mouse sharing) written in Rust. It enables seamless cursor glide, cross-platform clipboard synchronization, file drag-and-drop, and input sharing between machines across different operating systems (e.g., Windows, macOS, Kali Linux) with near-zero network and system resource overhead.

---

## 🎨 Features
- **Ultra-Low Latency Input:** UDP binary event streaming using `serde` & `bincode` for sub-2ms cursor response.
- **📋 Cross-Platform Clipboard Sync:** Instant real-time sharing of text, rich formatting, and images across OS clipboards.
- **📁 File Drag & Drop Transfer:** Drag files off the edge of one screen and drop them directly onto your secondary OS desktop.
- **🔒 Synchronized Screen Locking:** Locking your primary computer automatically locks the connected secondary machine.
- **🎯 Smart DPI Matching:** Smooth, uniform cursor movement across displays with different resolutions and scaling factors.
- **Modern GUI Dashboard:** Built with `egui` for visual screen placement and live network telemetry.
- **LLM Diagnostic Loop:** Integrated benchmarking and automated profiling endpoints for autonomous AI agent analysis.

---

## 🚀 Getting Started

### Prerequisites
- Rust & Cargo (v1.75+)
- Linux dependencies: `libx11-dev`, `libxtst-dev`, `libevdev-dev`, `libudev-dev`

### Building from Source
```bash
cargo build --release
```

### Running
```bash
cargo run -- --gui
```
