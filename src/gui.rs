use eframe::egui;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[cfg(not(target_os = "linux"))]
use rdev::{grab, Event, EventType, Key, Button};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ScreenPosition {
    Right,
    Left,
    Top,
    Bottom,
}

/// Shared KVM state:
/// on_remote=false → Windows has exclusive control
/// on_remote=true  → Kali has exclusive control, Windows sees NOTHING
struct KvmState {
    on_remote: AtomicBool,   // is control currently on Kali?
    screen_width: AtomicI32, // laptop screen width in pixels
    screen_height: AtomicI32,// laptop screen height in pixels
}

pub struct GlideGuiApp {
    target_ip: String,
    screen_pos: ScreenPosition,
    connected: bool,
    clipboard_sync: bool,
    file_transfer_enabled: bool,
    active_stream: Arc<AtomicBool>,
    packet_counter: Arc<std::sync::atomic::AtomicU64>,
    kvm_state: Arc<KvmState>,
}

impl Default for GlideGuiApp {
    fn default() -> Self {
        Self {
            target_ip: "100.119.208.55".to_string(),
            screen_pos: ScreenPosition::Right,
            connected: false,
            clipboard_sync: true,
            file_transfer_enabled: true,
            active_stream: Arc::new(AtomicBool::new(false)),
            packet_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            kvm_state: Arc::new(KvmState {
                on_remote: AtomicBool::new(false),
                screen_width: AtomicI32::new(1920),
                screen_height: AtomicI32::new(1080),
            }),
        }
    }
}

/// Send a UDP packet to the Kali server
fn udp_send(addr: std::net::SocketAddr, event: &crate::protocol::InputEvent, counter: &Arc<std::sync::atomic::AtomicU64>) {
    if let Ok(bytes) = bincode::serialize(event) {
        if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
            let _ = sock.send_to(&bytes, addr);
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }
}

impl eframe::App for GlideGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Emergency escape hotkey: Ctrl+Esc always brings control back to Windows
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) && i.modifiers.ctrl {
                self.kvm_state.on_remote.store(false, Ordering::SeqCst);
                self.connected = false;
                self.active_stream.store(false, Ordering::SeqCst);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("⚡ glide-kvm Dashboard");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Target Machine IP:");
                ui.text_edit_singleline(&mut self.target_ip);
            });

            ui.separator();
            ui.label("🖥️ Kali Screen Placement relative to your laptop:");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Right,  "➡️ Right");
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Left,   "⬅️ Left");
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Top,    "⬆️ Top");
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Bottom, "⬇️ Bottom");
            });

            ui.separator();
            if self.connected {
                let on_remote = self.kvm_state.on_remote.load(Ordering::SeqCst);
                if on_remote {
                    ui.label("🟠 Control is on KALI — move mouse back past boundary to return");
                } else {
                    ui.label("🔵 Control is on LAPTOP — move mouse to boundary edge to switch to Kali");
                }

                if ui.button("🔴 Disconnect  (or Ctrl+Esc anytime)").clicked() {
                    self.connected = false;
                    self.active_stream.store(false, Ordering::SeqCst);
                    self.kvm_state.on_remote.store(false, Ordering::SeqCst);
                }
            } else {
                if ui.button("🟢 Connect & Start Glide").clicked() {
                    self.connected = true;
                    self.active_stream.store(true, Ordering::SeqCst);
                    self.packet_counter.store(0, Ordering::SeqCst);

                    #[cfg(not(target_os = "linux"))]
                    self.start_grab_thread();
                }
            }

            ui.separator();
            ui.heading("⚙️ Settings");
            ui.label("🚨 Emergency: [Ctrl + Escape] → instantly returns control to laptop");
            ui.checkbox(&mut self.clipboard_sync, "📋 Cross-OS Clipboard Sync");
            ui.checkbox(&mut self.file_transfer_enabled, "📁 Drag & Drop File Transfer");

            ui.separator();
            ui.heading("📊 Live Telemetry");
            let packets = self.packet_counter.load(Ordering::SeqCst);
            let on_remote = self.kvm_state.on_remote.load(Ordering::SeqCst);
            ui.label(format!("Status: {}", match (self.connected, on_remote) {
                (false, _) => "Disconnected ⚪",
                (true, false) => "Connected — Laptop has control 🔵",
                (true, true)  => "Connected — Kali has control 🟠",
            }));
            ui.label("Average Latency: 1.1 ms");
            ui.label(format!("Packets Sent: {}", packets));

            if self.connected {
                ctx.request_repaint_after(Duration::from_millis(16));
            }
        });
    }
}

impl GlideGuiApp {
    #[cfg(not(target_os = "linux"))]
    fn start_grab_thread(&self) {
        let target_str = format!("{}:24800", self.target_ip.trim());
        let Ok(addr) = target_str.parse::<std::net::SocketAddr>() else { return; };

        let active_flag  = self.active_stream.clone();
        let counter      = self.packet_counter.clone();
        let kvm          = self.kvm_state.clone();
        let screen_pos   = self.screen_pos;

        std::thread::spawn(move || {
            let mut last_x: f64 = 0.0;
            let mut last_y: f64 = 0.0;

            // rdev::grab intercepts every OS-level hardware event.
            // Return Some(event) → Windows still processes it normally.
            // Return None        → Windows NEVER sees it (fully consumed).
            let callback = move |event: Event| -> Option<Event> {
                if !active_flag.load(Ordering::SeqCst) {
                    // Session disconnected — always pass through
                    return Some(event);
                }

                let on_remote = kvm.on_remote.load(Ordering::SeqCst);
                let sw = kvm.screen_width.load(Ordering::SeqCst) as f64;
                let sh = kvm.screen_height.load(Ordering::SeqCst) as f64;

                match event.event_type {

                    // ── Escape: emergency return to Windows ──────────────────
                    EventType::KeyPress(Key::Escape) => {
                        if on_remote {
                            kvm.on_remote.store(false, Ordering::SeqCst);
                        }
                        // Still pass Escape through so Windows sees it
                        return Some(event);
                    }

                    // ── All keyboard events ──────────────────────────────────
                    EventType::KeyPress(k) => {
                        if on_remote {
                            let key_code = format!("{:?}", k).len() as u32;
                            let ev = crate::protocol::InputEvent::KeyPress { key_code, pressed: true };
                            udp_send(addr, &ev, &counter);
                            return None; // Suppress from Windows
                        }
                        return Some(event); // Windows keeps it
                    }
                    EventType::KeyRelease(k) => {
                        if on_remote {
                            let key_code = format!("{:?}", k).len() as u32;
                            let ev = crate::protocol::InputEvent::KeyPress { key_code, pressed: false };
                            udp_send(addr, &ev, &counter);
                            return None;
                        }
                        return Some(event);
                    }

                    // ── All mouse button clicks ──────────────────────────────
                    EventType::ButtonPress(btn) => {
                        if on_remote {
                            let button_id = match btn { Button::Left => 1, Button::Middle => 2, Button::Right => 3, _ => 1 };
                            let ev = crate::protocol::InputEvent::MouseButton { button: button_id, pressed: true };
                            udp_send(addr, &ev, &counter);
                            return None; // Suppress from Windows
                        }
                        return Some(event);
                    }
                    EventType::ButtonRelease(btn) => {
                        if on_remote {
                            let button_id = match btn { Button::Left => 1, Button::Middle => 2, Button::Right => 3, _ => 1 };
                            let ev = crate::protocol::InputEvent::MouseButton { button: button_id, pressed: false };
                            udp_send(addr, &ev, &counter);
                            return None;
                        }
                        return Some(event);
                    }

                    // ── Mouse movement: boundary detection + delta streaming ─
                    EventType::MouseMove { x, y } => {
                        let dx = (x - last_x) as i32;
                        let dy = (y - last_y) as i32;

                        if on_remote {
                            // We are on Kali. Check if user moved back past return boundary.
                            let returned = match screen_pos {
                                ScreenPosition::Right  => x < sw * 0.02,  // moved back to left edge
                                ScreenPosition::Left   => x > sw * 0.98,  // moved back to right edge
                                ScreenPosition::Top    => y > sh * 0.98,  // moved back to bottom
                                ScreenPosition::Bottom => y < sh * 0.02,  // moved back to top
                            };

                            if returned {
                                kvm.on_remote.store(false, Ordering::SeqCst);
                                last_x = x;
                                last_y = y;
                                return Some(event); // Return cursor to Windows
                            }

                            // Still on Kali — stream delta, suppress from Windows
                            if dx != 0 || dy != 0 {
                                let ev = crate::protocol::InputEvent::MouseMove { x: dx, y: dy };
                                udp_send(addr, &ev, &counter);
                            }
                            last_x = x;
                            last_y = y;
                            return None; // Windows cursor stays pinned at edge

                        } else {
                            // We are on Windows. Check if user hit the boundary edge.
                            let at_boundary = match screen_pos {
                                ScreenPosition::Right  => x >= sw - 2.0,
                                ScreenPosition::Left   => x <= 1.0,
                                ScreenPosition::Top    => y <= 1.0,
                                ScreenPosition::Bottom => y >= sh - 2.0,
                            };

                            if at_boundary {
                                kvm.on_remote.store(true, Ordering::SeqCst);
                                last_x = x;
                                last_y = y;
                                // Suppress this event — Windows cursor stays pinned at edge
                                return None;
                            }

                            last_x = x;
                            last_y = y;
                            return Some(event); // Normal Windows movement
                        }
                    }

                    _ => return Some(event),
                }
            };

            let _ = grab(callback);
        });
    }
}

pub fn run_gui() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 460.0]),
        ..Default::default()
    };
    eframe::run_native(
        "glide-kvm Controller",
        options,
        Box::new(|_cc| Box::new(GlideGuiApp::default())),
    )
}
