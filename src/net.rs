use std::net::SocketAddr;
use tokio::net::UdpSocket;
use anyhow::Result;
use tracing::{info, error};
use crate::protocol::InputEvent;

#[cfg(target_os = "linux")]
use std::process::Command;

pub struct NetworkEngine {
    socket: UdpSocket,
}

impl NetworkEngine {
    pub async fn bind(addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        info!("Glide-KVM UDP Engine bound on {}", addr);
        Ok(Self { socket })
    }

    #[allow(dead_code)]
    pub async fn send_event(&self, event: &InputEvent, target: SocketAddr) -> Result<()> {
        let bytes = bincode::serialize(event)?;
        self.socket.send_to(&bytes, target).await?;
        Ok(())
    }

    pub async fn run_server_loop(self) -> Result<()> {
        let mut buf = [0u8; 1024];
        info!("Server listening for active input events...");
        
        // Grant local X11 display permissions & wake monitor on Linux server start
        #[cfg(target_os = "linux")]
        Self::wake_display();

        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    if let Ok(event) = bincode::deserialize::<InputEvent>(&buf[..len]) {
                        info!("Received input event from {}: {:?}", addr, event);
                        Self::simulate_os_input(&event);
                    }
                }
                Err(e) => {
                    error!("UDP socket error: {}", e);
                }
            }
        }
    }

    fn wake_display() {
        #[cfg(target_os = "linux")]
        {
            let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
            let xauth = std::env::var("XAUTHORITY").unwrap_or_else(|_| "/home/thrylox/.Xauthority".to_string());
            
            // Allow local X11 connections
            let _ = Command::new("xhost")
                .env("DISPLAY", &display)
                .env("XAUTHORITY", &xauth)
                .args(["+local:"])
                .spawn();

            // Force wake up display hardware and reset screensaver
            let _ = Command::new("xset")
                .env("DISPLAY", &display)
                .env("XAUTHORITY", &xauth)
                .args(["dpms", "force", "on"])
                .spawn();

            let _ = Command::new("xset")
                .env("DISPLAY", &display)
                .env("XAUTHORITY", &xauth)
                .args(["s", "reset"])
                .spawn();

            let _ = Command::new("xdotool")
                .env("DISPLAY", &display)
                .env("XAUTHORITY", &xauth)
                .args(["key", "Shift_L"])
                .spawn();
        }
    }

    fn simulate_os_input(event: &InputEvent) {
        #[cfg(target_os = "linux")]
        {
            // Wake display when receiving motion
            Self::wake_display();

            let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
            let xauth = std::env::var("XAUTHORITY").unwrap_or_else(|_| {
                let default_path = "/home/thrylox/.Xauthority";
                if std::path::Path::new(default_path).exists() {
                    default_path.to_string()
                } else {
                    "/run/user/1000/gdm/Xauthority".to_string()
                }
            });

            match event {
                InputEvent::MouseMove { x, y } => {
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", &display)
                        .env("XAUTHORITY", &xauth)
                        .args(["mousemove_relative", "--", &x.to_string(), &y.to_string()])
                        .spawn();
                }
                InputEvent::MouseButton { button, pressed } => {
                    let action = if *pressed { "mousedown" } else { "mouseup" };
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", &display)
                        .env("XAUTHORITY", &xauth)
                        .args([action, &button.to_string()])
                        .spawn();
                }
                InputEvent::KeyPress { key_code, pressed } => {
                    let action = if *pressed { "keydown" } else { "keyup" };
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", &display)
                        .env("XAUTHORITY", &xauth)
                        .args([action, &key_code.to_string()])
                        .spawn();
                }
                _ => {}
            }
        }
    }
}
