use std::net::SocketAddr;
use tokio::net::UdpSocket;
use anyhow::Result;
use tracing::{info, error};
use crate::protocol::InputEvent;

#[cfg(target_os = "linux")]
use evdev::uinput::VirtualDeviceBuilder;
#[cfg(target_os = "linux")]
use evdev::{AttributeSet, RelativeAxisType, EventType, InputEvent as EvdevEvent, Key};
#[cfg(target_os = "linux")]
use std::sync::Mutex;
#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
lazy_static::lazy_static! {
    static ref VIRTUAL_MOUSE: Mutex<Option<evdev::uinput::VirtualDevice>> = {
        let dev = (|| -> std::io::Result<evdev::uinput::VirtualDevice> {
            let mut keys = AttributeSet::new();
            keys.insert(Key::BTN_LEFT);
            keys.insert(Key::BTN_RIGHT);
            keys.insert(Key::BTN_MIDDLE);

            let mut rel_axes = AttributeSet::new();
            rel_axes.insert(RelativeAxisType::REL_X);
            rel_axes.insert(RelativeAxisType::REL_Y);
            rel_axes.insert(RelativeAxisType::REL_WHEEL);

            VirtualDeviceBuilder::new()?
                .name("glide-kvm Virtual Input")
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
                    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", &display)
                        .env("XAUTHORITY", "/run/user/1000/.mutter-Xwaylandauth.4QFTR3")
                        .args(["mousemove_relative", "--", &x.to_string(), &y.to_string()])
                        .spawn();
                }
                InputEvent::MouseButton { button, pressed } => {
                    if let Ok(mut guard) = VIRTUAL_MOUSE.lock() {
                        if let Some(ref mut dev) = *guard {
                            let key = match button {
                                1 => Key::BTN_LEFT,
                                2 => Key::BTN_MIDDLE,
                                3 => Key::BTN_RIGHT,
                                _ => Key::BTN_LEFT,
                            };
                            let val = if *pressed { 1 } else { 0 };
                            let ev = EvdevEvent::new(EventType::KEY, key.0, val);
                            let _ = dev.emit(&[ev]);
                            return;
                        }
                    }
                    let action = if *pressed { "mousedown" } else { "mouseup" };
                    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", &display)
                        .env("XAUTHORITY", "/run/user/1000/.mutter-Xwaylandauth.4QFTR3")
                        .args([action, &button.to_string()])
                        .spawn();
                }
                InputEvent::Scroll { delta_y, .. } => {
                    if let Ok(mut guard) = VIRTUAL_MOUSE.lock() {
                        if let Some(ref mut dev) = *guard {
                            let ev = EvdevEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL.0, *delta_y as i32);
                            let _ = dev.emit(&[ev]);
                            return;
                        }
                    }
                    let btn = if *delta_y > 0.0 { "4" } else { "5" };
                    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", &display)
                        .env("XAUTHORITY", "/run/user/1000/.mutter-Xwaylandauth.4QFTR3")
                        .args(["click", btn])
                        .spawn();
                }
                InputEvent::KeyPress { key_code, pressed } => {
                    let action = if *pressed { "keydown" } else { "keyup" };
                    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
                    let _ = Command::new("xdotool")
                        .env("DISPLAY", &display)
                        .env("XAUTHORITY", "/run/user/1000/.mutter-Xwaylandauth.4QFTR3")
                        .args([action, &key_code.to_string()])
                        .spawn();
                }
                _ => {}
            }
        }
    }
}
