use std::net::SocketAddr;
use tokio::net::UdpSocket;
use anyhow::Result;
use tracing::{info, error};
use crate::protocol::InputEvent;

#[cfg(target_os = "linux")]
use evdev::uinput::VirtualDeviceBuilder;
#[cfg(target_os = "linux")]
use evdev::{AttributeSet, RelativeAxisType, EventType, InputEvent as EvdevEvent};
#[cfg(target_os = "linux")]
use std::sync::Mutex;
#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
lazy_static::lazy_static! {
    static ref VIRTUAL_MOUSE: Mutex<Option<evdev::uinput::VirtualDevice>> = {
        let dev = (|| -> std::io::Result<evdev::uinput::VirtualDevice> {
            let mut keys = AttributeSet::new();
            keys.insert(evdev::Key::BTN_LEFT);
            keys.insert(evdev::Key::BTN_RIGHT);
            keys.insert(evdev::Key::BTN_MIDDLE);

            let mut rel_axes = AttributeSet::new();
            rel_axes.insert(RelativeAxisType::REL_X);
            rel_axes.insert(RelativeAxisType::REL_Y);

            VirtualDeviceBuilder::new()?
                .name("glide-kvm Virtual Mouse")
                .with_keys(&keys)?
                .with_relative_axes(&rel_axes)?
                .build()
        })();

        if dev.is_err() {
            info!("Kernel /dev/uinput virtual device creation skipped or permission denied. Operating in Xwayland/Mutter fallback mode.");
        }
        
        Mutex::new(dev.ok())
    };
}

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
                    if let Ok(mut guard) = VIRTUAL_MOUSE.lock() {
                        if let Some(ref mut dev) = *guard {
                            let ev_x = EvdevEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, *x);
                            let ev_y = EvdevEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, *y);
                            let _ = dev.emit(&[ev_x, ev_y]);
                            return;
                        }
                    }
                    
                    // Fallback to xdotool with Mutter Wayland auth
                    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", &display)
                        .env("XAUTHORITY", "/run/user/1000/.mutter-Xwaylandauth.4QFTR3")
                        .args(["mousemove_relative", "--", &x.to_string(), &y.to_string()])
                        .spawn();
                }
                _ => {}
            }
        }
    }
}
