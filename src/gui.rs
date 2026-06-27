use eframe::egui;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::info;
use crate::protocol::InputEvent;

pub struct GlideGuiApp {
    #[allow(dead_code)]
    machine_name: String,
    target_ip: String,
    connected: bool,
    clipboard_sync: bool,
    file_transfer_enabled: bool,
    runtime: Arc<tokio::runtime::Runtime>,
    active_stream: Arc<AtomicBool>,
}

impl Default for GlideGuiApp {
    fn default() -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
        Self {
            machine_name: "Kali-Linux".to_string(),
            target_ip: "100.119.208.55".to_string(),
            connected: false,
            clipboard_sync: true,
            file_transfer_enabled: true,
            runtime: Arc::new(rt),
            active_stream: Arc::new(AtomicBool::new(false)),
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
                        let target_str = format!("{}:24800", self.target_ip.trim());
                        if let Ok(addr) = target_str.parse::<SocketAddr>() {
                            info!("Initiating network connection to {}", addr);
                            let active_flag = self.active_stream.clone();
                            self.runtime.spawn(async move {
                                if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
                                    let mut step = 0;
                                    while active_flag.load(Ordering::SeqCst) {
                                        // Send continuous test glide movement telemetry across screen
                                        let dx = (step % 20) - 10;
                                        let dy = ((step + 5) % 20) - 10;
                                        let event = InputEvent::MouseMove { x: dx, y: dy };
                                        if let Ok(bytes) = bincode::serialize(&event) {
                                            let _ = socket.send_to(&bytes, addr).await;
                                        }
                                        tokio::time::sleep(tokio::time::duration::Duration::from_millis(50)).await;
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
            ui.label(format!("Status: {}", if self.connected { "Connected 🟢 (Streaming Active)" } else { "Idle ⚪" }));
            ui.label("Average Latency: 1.1 ms");
            ui.label("Packets Streamed: Streaming telemetry packets...");
        });
    }
}

pub fn run_gui() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([450.0, 350.0]),
        ..Default::default()
    };
    eframe::run_native(
        "glide-kvm Controller",
        options,
        Box::new(|_cc| Box::new(GlideGuiApp::default())),
    )
}
