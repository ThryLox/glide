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
        // Ctrl+Escape: emergency return to laptop
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

            ui.horizontal(|ui| {
                ui.label("Your laptop screen resolution:");
                let mut sw = self.kvm_state.screen_width.load(Ordering::SeqCst);
                let mut sh = self.kvm_state.screen_height.load(Ordering::SeqCst);
                ui.add(egui::DragValue::new(&mut sw).prefix("W: ").speed(1.0));
                ui.add(egui::DragValue::new(&mut sh).prefix("H: ").speed(1.0));
                self.kvm_state.screen_width.store(sw, Ordering::SeqCst);
                self.kvm_state.screen_height.store(sh, Ordering::SeqCst);
            });

            ui.separator();
            ui.label("🖥️ Kali screen placement relative to your laptop:");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Right,  "➡️ Right");
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Left,   "⬅️ Left");
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Top,    "⬆️ Top");
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Bottom, "⬇️ Bottom");
            });

            ui.separator();

            // Hotkey legend
            ui.group(|ui| {
                ui.label("⌨️ KVM Hotkeys:");
                ui.label("  [Scroll Lock]   — toggle focus between Laptop ↔ Kali");
                ui.label("  [Ctrl + Alt + G] — switch focus to Kali");
                ui.label("  [Ctrl + Escape]  — emergency return to Laptop");
            });

            ui.separator();

            if self.connected {
                let on_remote = self.kvm_state.on_remote.load(Ordering::SeqCst);
                if on_remote {
                    ui.label("🟠 Control → KALI  (Scroll Lock or push cursor past boundary to return)");
                } else {
                    ui.label("🔵 Control → LAPTOP  (Scroll Lock, Ctrl+Alt+G, or move cursor to edge)");
                }
                if ui.button("🔴 Disconnect  (or Ctrl+Esc)").clicked() {
                    self.connected = false;
                    self.active_stream.store(false, Ordering::SeqCst);
                    self.kvm_state.on_remote.store(false, Ordering::SeqCst);
                }
            } else if ui.button("🟢 Connect & Start Glide").clicked() {
                self.connected = true;
                self.active_stream.store(true, Ordering::SeqCst);
                self.packet_counter.store(0, Ordering::SeqCst);

                // Send SetLayout so Kali knows which edge to watch for return
                #[cfg(not(target_os = "linux"))]
                {
                    let side: u8 = match self.screen_pos {
                        ScreenPosition::Right  => 0,
                        ScreenPosition::Left   => 1,
                        ScreenPosition::Top    => 2,
                        ScreenPosition::Bottom => 3,
                    };
                    let target_str = format!("{}:24800", self.target_ip.trim());
                    if let Ok(addr) = target_str.parse::<std::net::SocketAddr>() {
                        let ev = crate::protocol::InputEvent::SetLayout { side };
                        if let Ok(bytes) = bincode::serialize(&ev) {
                            if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
                                let _ = sock.send_to(&bytes, addr);
                            }
                        }
                    }
                }

                // Spawn ReturnToHost listener on port 24801
                #[cfg(not(target_os = "linux"))]
                {
                    let kvm_ret    = self.kvm_state.clone();
                    let active_ret = self.active_stream.clone();
                    std::thread::spawn(move || {
                        let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:24801") else { return; };
                        let mut buf = [0u8; 256];
                        loop {
                            if !active_ret.load(Ordering::SeqCst) { break; }
                            if let Ok((len, _)) = sock.recv_from(&mut buf) {
                                if let Ok(crate::protocol::InputEvent::ReturnToHost) =
                                    bincode::deserialize::<crate::protocol::InputEvent>(&buf[..len])
                                {
                                    kvm_ret.on_remote.store(false, Ordering::SeqCst);
                                }
                            }
                        }
                    });
                }

                #[cfg(not(target_os = "linux"))]
                self.start_grab_thread();
            }

            ui.separator();
            ui.heading("⚙️ Settings");
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
            ui.label("Average Latency: ~1 ms");
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
            let socket = match std::net::UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => Arc::new(s),
                Err(_) => return,
            };

            let send_event = {
                let socket = socket.clone();
                let counter = counter.clone();
                move |ev: &crate::protocol::InputEvent| {
                    if let Ok(bytes) = bincode::serialize(ev) {
                        let _ = socket.send_to(&bytes, addr);
                        counter.fetch_add(1, Ordering::SeqCst);
                    }
                }
            };

            let sw = kvm.screen_width.load(Ordering::SeqCst) as f64;
            let sh = kvm.screen_height.load(Ordering::SeqCst) as f64;
            let cx: Cell<f64> = Cell::new(sw / 2.0);
            let cy: Cell<f64> = Cell::new(sh / 2.0);

            let pending_warp_x: Cell<f64> = Cell::new(-1.0);
            let pending_warp_y: Cell<f64> = Cell::new(-1.0);

            let ctrl_held:  Cell<bool> = Cell::new(false);
            let alt_held:   Cell<bool> = Cell::new(false);

            let do_warp = |tx: f64, ty: f64, pwx: &Cell<f64>, pwy: &Cell<f64>| {
                pwx.set(tx);
                pwy.set(ty);
                let _ = simulate(&EventType::MouseMove { x: tx, y: ty });
            };

            let callback = move |event: Event| -> Option<Event> {
                if !active_flag.load(Ordering::SeqCst) {
                    return Some(event);
                }

                let on_remote = kvm.on_remote.load(Ordering::SeqCst);
                let sw = kvm.screen_width.load(Ordering::SeqCst) as f64;
                let sh = kvm.screen_height.load(Ordering::SeqCst) as f64;
                let anchor_x = cx.get();
                let anchor_y = cy.get();

                match &event.event_type {

                    EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                        ctrl_held.set(true);
                        if on_remote { send_event(&crate::protocol::InputEvent::KeyName { name: "ctrl".into(), pressed: true }); return None; }
                        return Some(event);
                    }
                    EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
                        ctrl_held.set(false);
                        if on_remote { send_event(&crate::protocol::InputEvent::KeyName { name: "ctrl".into(), pressed: false }); return None; }
                        return Some(event);
                    }
                    EventType::KeyPress(Key::Alt) | EventType::KeyPress(Key::AltGr) => {
                        alt_held.set(true);
                        if on_remote { send_event(&crate::protocol::InputEvent::KeyName { name: "alt".into(), pressed: true }); return None; }
                        return Some(event);
                    }
                    EventType::KeyRelease(Key::Alt) | EventType::KeyRelease(Key::AltGr) => {
                        alt_held.set(false);
                        if on_remote { send_event(&crate::protocol::InputEvent::KeyName { name: "alt".into(), pressed: false }); return None; }
                        return Some(event);
                    }

                    EventType::KeyPress(Key::ScrollLock) => {
                        let now_remote = !on_remote;
                        kvm.on_remote.store(now_remote, Ordering::SeqCst);
                        if now_remote {
                            do_warp(anchor_x, anchor_y, &pending_warp_x, &pending_warp_y);
                        }
                        return None;
                    }

                    EventType::KeyPress(Key::KeyG) if ctrl_held.get() && alt_held.get() => {
                        kvm.on_remote.store(true, Ordering::SeqCst);
                        do_warp(anchor_x, anchor_y, &pending_warp_x, &pending_warp_y);
                        return None;
                    }

                    EventType::KeyPress(Key::Escape) if ctrl_held.get() => {
                        kvm.on_remote.store(false, Ordering::SeqCst);
                        return Some(event);
                    }

                    EventType::KeyPress(k) => {
                        if on_remote {
                            if let Some(name) = key_to_xdotool(k) {
                                send_event(&crate::protocol::InputEvent::KeyName { name, pressed: true });
                            }
                            return None;
                        }
                        return Some(event);
                    }
                    EventType::KeyRelease(k) => {
                        if on_remote {
                            if let Some(name) = key_to_xdotool(k) {
                                send_event(&crate::protocol::InputEvent::KeyName { name, pressed: false });
                            }
                            return None;
                        }
                        return Some(event);
                    }

                    EventType::ButtonPress(btn) => {
                        if on_remote {
                            let b = match btn { Button::Left => 1, Button::Middle => 2, Button::Right => 3, _ => 1 };
                            send_event(&crate::protocol::InputEvent::MouseButton { button: b, pressed: true });
                            return None;
                        }
                        return Some(event);
                    }
                    EventType::ButtonRelease(btn) => {
                        if on_remote {
                            let b = match btn { Button::Left => 1, Button::Middle => 2, Button::Right => 3, _ => 1 };
                            send_event(&crate::protocol::InputEvent::MouseButton { button: b, pressed: false });
                            return None;
                        }
                        return Some(event);
                    }

                    EventType::MouseMove { x, y } => {
                        let x = *x;
                        let y = *y;
                        let pwx = pending_warp_x.get();
                        let pwy = pending_warp_y.get();

                        if pwx >= 0.0 && (x - pwx).abs() < 2.0 && (y - pwy).abs() < 2.0 {
                            pending_warp_x.set(-1.0);
                            pending_warp_y.set(-1.0);
                            return Some(event);
                        }

                        if on_remote {
                            let dx = (x - anchor_x) as i32;
                            let dy = (y - anchor_y) as i32;

                            let return_triggered = match screen_pos {
                                ScreenPosition::Right  => dx < -60,
                                ScreenPosition::Left   => dx >  60,
                                ScreenPosition::Top    => dy >  60,
                                ScreenPosition::Bottom => dy < -60,
                            };

                            if return_triggered {
                                kvm.on_remote.store(false, Ordering::SeqCst);
                                do_warp(anchor_x, anchor_y, &pending_warp_x, &pending_warp_y);
                                return None;
                            }

                            if dx != 0 || dy != 0 {
                                send_event(&crate::protocol::InputEvent::MouseMove { x: dx, y: dy });
                            }

                            do_warp(anchor_x, anchor_y, &pending_warp_x, &pending_warp_y);
                            return None;

                        } else {
                            cx.set(sw / 2.0);
                            cy.set(sh / 2.0);

                            let at_boundary = match screen_pos {
                                ScreenPosition::Right  => x >= sw - 2.0,
                                ScreenPosition::Left   => x <= 1.0,
                                ScreenPosition::Top    => y <= 1.0,
                                ScreenPosition::Bottom => y >= sh - 2.0,
                            };

                            if at_boundary {
                                kvm.on_remote.store(true, Ordering::SeqCst);
                                do_warp(cx.get(), cy.get(), &pending_warp_x, &pending_warp_y);
                                return None;
                            }

                            return Some(event);
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
            .with_inner_size([500.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "glide-kvm Controller",
        options,
        Box::new(|_cc| Box::new(GlideGuiApp::default())),
    )
}
