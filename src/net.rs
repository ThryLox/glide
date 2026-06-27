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

    pub async fn send_event(&self, event: &InputEvent, target: SocketAddr) -> Result<()> {
        let bytes = bincode::serialize(event)?;
        self.socket.send_to(&bytes, target).await?;
        Ok(())
    }

    pub async fn run_server_loop(self) -> Result<()> {
        let mut buf = [0u8; 1024];
        info!("Server listening for active input events...");
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

    fn simulate_os_input(event: &InputEvent) {
        #[cfg(target_os = "linux")]
        {
            match event {
                InputEvent::MouseMove { x, y } => {
                    // Simulate mouse movement on Linux screen using xdotool
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", ":0")
                        .args(["mousemove_relative", "--", &x.to_string(), &y.to_string()])
                        .spawn();
                }
                InputEvent::MouseButton { button, pressed } => {
                    let action = if *pressed { "mousedown" } else { "mouseup" };
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", ":0")
                        .args([action, &button.to_string()])
                        .spawn();
                }
                InputEvent::KeyPress { key_code, pressed } => {
                    let action = if *pressed { "keydown" } else { "keyup" };
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", ":0")
                        .args([action, &key_code.to_string()])
                        .spawn();
                }
                _ => {}
            }
        }
    }
}
