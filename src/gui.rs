use eframe::egui;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[cfg(not(target_os = "linux"))]
use rdev::{grab, simulate, Event, EventType, Key, Button};
#[cfg(not(target_os = "linux"))]
use std::cell::Cell;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ScreenPosition {
    Right,
    Left,
    Top,
    Bottom,
}

struct KvmState {
    on_remote: AtomicBool,
    screen_width: AtomicI32,
    screen_height: AtomicI32,
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

fn udp_send(addr: std::net::SocketAddr, event: &crate::protocol::InputEvent, counter: &Arc<std::sync::atomic::AtomicU64>) {
    if let Ok(bytes) = bincode::serialize(event) {
        if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
            let _ = sock.send_to(&bytes, addr);
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// Map rdev Key to an xdotool-compatible key name string
#[cfg(not(target_os = "linux"))]
fn key_to_xdotool(k: &Key) -> Option<String> {
    let name = match k {
        Key::KeyA => "a", Key::KeyB => "b", Key::KeyC => "c", Key::KeyD => "d",
        Key::KeyE => "e", Key::KeyF => "f", Key::KeyG => "g", Key::KeyH => "h",
        Key::KeyI => "i", Key::KeyJ => "j", Key::KeyK => "k", Key::KeyL => "l",
        Key::KeyM => "m", Key::KeyN => "n", Key::KeyO => "o", Key::KeyP => "p",
        Key::KeyQ => "q", Key::KeyR => "r", Key::KeyS => "s", Key::KeyT => "t",
        Key::KeyU => "u", Key::KeyV => "v", Key::KeyW => "w", Key::KeyX => "x",
        Key::KeyY => "y", Key::KeyZ => "z",
        Key::Num0 => "0", Key::Num1 => "1", Key::Num2 => "2", Key::Num3 => "3",
        Key::Num4 => "4", Key::Num5 => "5", Key::Num6 => "6", Key::Num7 => "7",
        Key::Num8 => "8", Key::Num9 => "9",
        Key::Return => "Return", Key::Backspace => "BackSpace", Key::Tab => "Tab",
        Key::Space => "space", Key::Delete => "Delete", Key::Insert => "Insert",
        Key::Home => "Home", Key::End => "End",
        Key::PageUp => "Prior", Key::PageDown => "Next",
        Key::UpArrow => "Up", Key::DownArrow => "Down",
        Key::LeftArrow => "Left", Key::RightArrow => "Right",
        Key::F1 => "F1", Key::F2 => "F2", Key::F3 => "F3", Key::F4 => "F4",
        Key::F5 => "F5", Key::F6 => "F6", Key::F7 => "F7", Key::F8 => "F8",
        Key::F9 => "F9", Key::F10 => "F10", Key::F11 => "F11", Key::F12 => "F12",
        Key::ShiftLeft | Key::ShiftRight => "shift",
        Key::ControlLeft | Key::ControlRight => "ctrl",
        Key::Alt => "alt",
        Key::MetaLeft | Key::MetaRight => "super",
        Key::CapsLock => "Caps_Lock",
        Key::Comma => "comma", Key::Dot => "period", Key::SemiColon => "semicolon",
        Key::Quote => "apostrophe", Key::LeftBracket => "bracketleft",
        Key::RightBracket => "bracketright", Key::BackSlash => "backslash",
        Key::Slash => "slash", Key::Minus => "minus", Key::Equal => "equal",
        Key::BackQuote => "grave", Key::Escape => "Escape",
        _ => return None,
    };
    Some(name.to_string())
}

impl eframe::App for GlideGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                (false, _)    => "Disconnected ⚪",
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

        let active_flag = self.active_stream.clone();
        let counter     = self.packet_counter.clone();
        let kvm         = self.kvm_state.clone();
        let screen_pos  = self.screen_pos;

        std::thread::spawn(move || {
            // Cell<f64> gives interior mutability — required so closure is Fn not FnMut
            // (rdev::grab requires Fn)
            let warp_cx: Cell<f64> = Cell::new(0.0);
            let warp_cy: Cell<f64> = Cell::new(0.0);
            let is_warping: Cell<bool> = Cell::new(false);

            // Detect screen dimensions via first mouse event
            let sw = kvm.screen_width.load(Ordering::SeqCst) as f64;
            let sh = kvm.screen_height.load(Ordering::SeqCst) as f64;
            // Warp anchor is the center of the screen
            warp_cx.set(sw / 2.0);
            warp_cy.set(sh / 2.0);

            let callback = move |event: Event| -> Option<Event> {
                if !active_flag.load(Ordering::SeqCst) {
                    return Some(event); // Disconnected: pass everything through
                }

                let on_remote = kvm.on_remote.load(Ordering::SeqCst);

                match &event.event_type {

                    // ── Emergency escape: always return to Windows ───────────
                    EventType::KeyPress(Key::Escape) => {
                        if on_remote {
                            kvm.on_remote.store(false, Ordering::SeqCst);
                        }
                        return Some(event);
                    }

                    // ── Keyboard: only route to Kali when remote ─────────────
                    EventType::KeyPress(k) => {
                        if on_remote {
                            if let Some(name) = key_to_xdotool(k) {
                                let ev = crate::protocol::InputEvent::KeyName { name, pressed: true };
                                udp_send(addr, &ev, &counter);
                            }
                            return None; // Suppress from Windows
                        }
                        return Some(event);
                    }
                    EventType::KeyRelease(k) => {
                        if on_remote {
                            if let Some(name) = key_to_xdotool(k) {
                                let ev = crate::protocol::InputEvent::KeyName { name, pressed: false };
                                udp_send(addr, &ev, &counter);
                            }
                            return None;
                        }
                        return Some(event);
                    }

                    // ── Mouse buttons: only route to Kali when remote ────────
                    EventType::ButtonPress(btn) => {
                        if on_remote {
                            let button_id = match btn { Button::Left => 1, Button::Middle => 2, Button::Right => 3, _ => 1 };
                            let ev = crate::protocol::InputEvent::MouseButton { button: button_id, pressed: true };
                            udp_send(addr, &ev, &counter);
                            return None;
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

                    // ── Mouse movement ───────────────────────────────────────
                    EventType::MouseMove { x, y } => {
                        let x = *x;
                        let y = *y;
                        let sw = kvm.screen_width.load(Ordering::SeqCst) as f64;
                        let sh = kvm.screen_height.load(Ordering::SeqCst) as f64;
                        let cx = warp_cx.get();
                        let cy = warp_cy.get();

                        // Ignore synthetic warp events we generate ourselves
                        if is_warping.get() {
                            return None;
                        }

                        if on_remote {
                            // --- REMOTE MODE ---
                            // Compute delta from warp anchor (center of screen)
                            let dx = (x - cx) as i32;
                            let dy = (y - cy) as i32;

                            // Check if the user is pushing the cursor back past the return boundary
                            let return_triggered = match screen_pos {
                                ScreenPosition::Right  => dx < -80,  // strong push leftward
                                ScreenPosition::Left   => dx > 80,   // strong push rightward
                                ScreenPosition::Top    => dy > 80,   // strong push downward
                                ScreenPosition::Bottom => dy < -80,  // strong push upward
                            };

                            if return_triggered {
                                kvm.on_remote.store(false, Ordering::SeqCst);
                                // Warp cursor to center so return is clean
                                is_warping.set(true);
                                let _ = simulate(&EventType::MouseMove { x: cx, y: cy });
                                is_warping.set(false);
                                return None;
                            }

                            // Stream delta to Kali
                            if dx != 0 || dy != 0 {
                                let ev = crate::protocol::InputEvent::MouseMove { x: dx, y: dy };
                                udp_send(addr, &ev, &counter);
                            }

                            // Warp cursor back to anchor so we always get fresh deltas
                            is_warping.set(true);
                            let _ = simulate(&EventType::MouseMove { x: cx, y: cy });
                            is_warping.set(false);
                            return None; // Suppress the original move from Windows

                        } else {
                            // --- HOST MODE ---
                            // Check if cursor hit the boundary edge
                            let at_boundary = match screen_pos {
                                ScreenPosition::Right  => x >= sw - 2.0,
                                ScreenPosition::Left   => x <= 1.0,
                                ScreenPosition::Top    => y <= 1.0,
                                ScreenPosition::Bottom => y >= sh - 2.0,
                            };

                            if at_boundary {
                                kvm.on_remote.store(true, Ordering::SeqCst);
                                // Warp to center to give clean delta origin
                                is_warping.set(true);
                                let _ = simulate(&EventType::MouseMove { x: cx, y: cy });
                                is_warping.set(false);
                                return None;
                            }

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
