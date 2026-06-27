# ⚡ glide-kvm

**glide-kvm** is an ultra-lightweight, high-performance Software KVM (Keyboard & Mouse sharing) written in Rust. It enables seamless cursor glide and input sharing between machines across different operating systems (e.g., Windows, macOS, Kali Linux) with near-zero network and system resource overhead.

---

## 🎨 Features
- **Ultra-Low Latency:** UDP binary event streaming using `serde` & `bincode`.
- **Modern GUI Dashboard:** Built with `egui` for visual screen placement and live telemetry.
- **Cross-Platform Input Drivers:** Native OS hooks and input injection (`uinput`/`evdev` on Linux).
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
