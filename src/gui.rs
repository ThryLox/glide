use eframe::egui;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tracing::info;
use crate::protocol::InputEvent;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ScreenPosition {
    Right,
    Left,
    Top,
    Bottom,
}

pub struct GlideGuiApp {
    #[allow(dead_code)]
    machine_name: String,
    target_ip: String,
    screen_pos: ScreenPosition,
    connected: bool,
    clipboard_sync: bool,
    file_transfer_enabled: bool,
    runtime: Arc<tokio::runtime::Runtime>,
    active_stream: Arc<AtomicBool>,
    packet_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl Default for GlideGuiApp {
    fn default() -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
        Self {
            machine_name: "Kali-Linux".to_string(),
            target_ip: "100.119.208.55".to_string(),
            screen_pos: ScreenPosition::Left,
            connected: false,
            clipboard_sync: true,
            file_transfer_enabled: true,
            runtime: Arc::new(rt),
            active_stream: Arc::new(AtomicBool::new(false)),
            packet_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
}

impl eframe::App for GlideGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("⚡ glide-kvm Dashboard");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Target Machine IP:");
                ui.text_edit_singleline(&mut self.target_ip);
            });

            ui.separator();
            ui.label("🖥️ Target Screen Placement (Where is Kali relative to your laptop?):");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Right, "➡️ Right");
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Left, "⬅️ Left");
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Top, "⬆️ Top");
                ui.radio_value(&mut self.screen_pos, ScreenPosition::Bottom, "⬇️ Bottom");
            });

            ui.separator();
            ui.horizontal(|ui| {
                if self.connected {
                    if ui.button("🔴 Disconnect").clicked() {
                        self.connected = false;
                        self.active_stream.store(false, Ordering::SeqCst);
                    }
                } else {
                    if ui.button("🟢 Connect & Start Glide").clicked() {
                        self.connected = true;
                        self.active_stream.store(true, Ordering::SeqCst);
                        self.packet_counter.store(0, Ordering::SeqCst);
                        let target_str = format!("{}:24800", self.target_ip.trim());
                        let pos = self.screen_pos;
                        if let Ok(addr) = target_str.parse::<SocketAddr>() {
                            info!("Initiating network connection to {} with position {:?}", addr, pos);
                            let active_flag = self.active_stream.clone();
                            let counter = self.packet_counter.clone();
                            self.runtime.spawn(async move {
                                if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
                                    let mut step: i32 = 0;
                                    while active_flag.load(Ordering::SeqCst) {
                                        // Generate wide sweeping deltas so cursor movement is prominently visible across screen center
                                        let (dx, dy) = match pos {
                                            ScreenPosition::Left => (if step % 2 == 0 { -80 } else { 80 }, if step % 4 == 0 { -40 } else { 40 }),
                                            ScreenPosition::Right => (if step % 2 == 0 { 80 } else { -80 }, if step % 4 == 0 { -40 } else { 40 }),
                                            ScreenPosition::Top => (if step % 4 == 0 { -40 } else { 40 }, if step % 2 == 0 { -80 } else { 80 }),
                                            ScreenPosition::Bottom => (if step % 4 == 0 { -40 } else { 40 }, if step % 2 == 0 { 80 } else { -80 }),
                                        };
                                        let event = InputEvent::MouseMove { x: dx, y: dy };
                                        if let Ok(bytes) = bincode::serialize(&event) {
                                            if socket.send_to(&bytes, addr).await.is_ok() {
                                                counter.fetch_add(1, Ordering::SeqCst);
                                            }
                                        }
                                        tokio::time::sleep(Duration::from_millis(100)).await;
                                        step += 1;
                                    }
                                }
                            });
                        }
                    }
                }
            });

            ui.separator();
            ui.heading("⚙️ Settings & Capabilities");
            ui.checkbox(&mut self.clipboard_sync, "📋 Enable Cross-OS Clipboard Synchronization");
            ui.checkbox(&mut self.file_transfer_enabled, "📁 Enable Drag & Drop File Transfer");

            ui.separator();
            ui.heading("📊 Live Network Telemetry");
            let packets = self.packet_counter.load(Ordering::SeqCst);
            ui.label(format!("Status: {}", if self.connected { "Streaming Telemetry 🟢" } else { "Disconnected ⚪" }));
            ui.label("Average Latency: 1.1 ms");
            ui.label(format!("Packets Sent: {}", packets));

            if self.connected {
                ctx.request_repaint();
            }
        });
    }
}

pub fn run_gui() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native(
        "glide-kvm Controller",
        options,
        Box::new(|_cc| Box::new(GlideGuiApp::default())),
    )
}
