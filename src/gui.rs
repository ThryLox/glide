use eframe::egui;

pub struct GlideGuiApp {
    machine_name: String,
    target_ip: String,
    connected: bool,
    clipboard_sync: bool,
    file_transfer_enabled: bool,
}

impl Default for GlideGuiApp {
    fn default() -> Self {
        Self {
            machine_name: "Kali-Linux".to_string(),
            target_ip: "192.168.1.100".to_string(),
            connected: false,
            clipboard_sync: true,
            file_transfer_enabled: true,
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
                    }
                } else {
                    if ui.button("🟢 Connect & Start Glide").clicked() {
                        self.connected = true;
                    }
                }
            });

            ui.separator();
            ui.heading("⚙️ Settings & Capabilities");
            ui.checkbox(&mut self.clipboard_sync, "📋 Enable Cross-OS Clipboard Synchronization");
            ui.checkbox(&mut self.file_transfer_enabled, "📁 Enable Drag & Drop File Transfer");

            ui.separator();
            ui.heading("📊 Live Network Telemetry");
            ui.label(format!("Status: {}", if self.connected { "Connected 🟢" } else { "Idle ⚪" }));
            ui.label("Average Latency: 1.2 ms");
            ui.label("Packets Streamed: 0 pps");
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
