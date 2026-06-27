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
use std::sync::atomic::{AtomicI32, AtomicU8, Ordering};
#[cfg(target_os = "linux")]
use std::process::Command;

// side encoding: 0=Right,1=Left,2=Top,3=Bottom (Kali is on which side of laptop)
// When Kali is to the Right → the return edge is the LEFT edge of Kali (x ≤ 0)
// When Kali is to the Left  → return edge = RIGHT edge (x ≥ screen_w)
// When Kali is to the Top   → return edge = BOTTOM edge (y ≥ screen_h)
// When Kali is to the Bottom→ return edge = TOP edge (y ≤ 0)

#[cfg(target_os = "linux")]
fn xdotool_name_to_evdev(name: &str) -> Option<Key> {
    match name {
        "a" => Some(Key::KEY_A), "b" => Some(Key::KEY_B), "c" => Some(Key::KEY_C),
        "d" => Some(Key::KEY_D), "e" => Some(Key::KEY_E), "f" => Some(Key::KEY_F),
        "g" => Some(Key::KEY_G), "h" => Some(Key::KEY_H), "i" => Some(Key::KEY_I),
        "j" => Some(Key::KEY_J), "k" => Some(Key::KEY_K), "l" => Some(Key::KEY_L),
        "m" => Some(Key::KEY_M), "n" => Some(Key::KEY_N), "o" => Some(Key::KEY_O),
        "p" => Some(Key::KEY_P), "q" => Some(Key::KEY_Q), "r" => Some(Key::KEY_R),
        "s" => Some(Key::KEY_S), "t" => Some(Key::KEY_T), "u" => Some(Key::KEY_U),
        "v" => Some(Key::KEY_V), "w" => Some(Key::KEY_W), "x" => Some(Key::KEY_X),
        "y" => Some(Key::KEY_Y), "z" => Some(Key::KEY_Z),
        "0" => Some(Key::KEY_0), "1" => Some(Key::KEY_1), "2" => Some(Key::KEY_2),
        "3" => Some(Key::KEY_3), "4" => Some(Key::KEY_4), "5" => Some(Key::KEY_5),
        "6" => Some(Key::KEY_6), "7" => Some(Key::KEY_7), "8" => Some(Key::KEY_8),
        "9" => Some(Key::KEY_9),
        "Return"    => Some(Key::KEY_ENTER),
        "BackSpace" => Some(Key::KEY_BACKSPACE),
        "Tab"       => Some(Key::KEY_TAB),
        "space"     => Some(Key::KEY_SPACE),
        "Delete"    => Some(Key::KEY_DELETE),
        "Insert"    => Some(Key::KEY_INSERT),
        "Home"      => Some(Key::KEY_HOME),
        "End"       => Some(Key::KEY_END),
        "Prior"     => Some(Key::KEY_PAGEUP),
        "Next"      => Some(Key::KEY_PAGEDOWN),
        "Up"        => Some(Key::KEY_UP),
        "Down"      => Some(Key::KEY_DOWN),
        "Left"      => Some(Key::KEY_LEFT),
        "Right"     => Some(Key::KEY_RIGHT),
        "F1"  => Some(Key::KEY_F1),  "F2"  => Some(Key::KEY_F2),
        "F3"  => Some(Key::KEY_F3),  "F4"  => Some(Key::KEY_F4),
        "F5"  => Some(Key::KEY_F5),  "F6"  => Some(Key::KEY_F6),
        "F7"  => Some(Key::KEY_F7),  "F8"  => Some(Key::KEY_F8),
        "F9"  => Some(Key::KEY_F9),  "F10" => Some(Key::KEY_F10),
        "F11" => Some(Key::KEY_F11), "F12" => Some(Key::KEY_F12),
        "shift"     => Some(Key::KEY_LEFTSHIFT),
        "ctrl"      => Some(Key::KEY_LEFTCTRL),
        "alt"       => Some(Key::KEY_LEFTALT),
        "super"     => Some(Key::KEY_LEFTMETA),
        "Caps_Lock" => Some(Key::KEY_CAPSLOCK),
        "Escape"    => Some(Key::KEY_ESC),
        "comma"     => Some(Key::KEY_COMMA),
        "period"    => Some(Key::KEY_DOT),
        "semicolon" => Some(Key::KEY_SEMICOLON),
        "apostrophe"=> Some(Key::KEY_APOSTROPHE),
        "bracketleft"  => Some(Key::KEY_LEFTBRACE),
        "bracketright" => Some(Key::KEY_RIGHTBRACE),
        "backslash" => Some(Key::KEY_BACKSLASH),
        "slash"     => Some(Key::KEY_SLASH),
        "minus"     => Some(Key::KEY_MINUS),
        "equal"     => Some(Key::KEY_EQUAL),
        "grave"     => Some(Key::KEY_GRAVE),
        _ => None,
    }
}

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
                .name("glide-kvm Virtual Mouse")
                .with_keys(&keys)?
                .with_relative_axes(&rel_axes)?
                .build()
        })();
        if dev.is_err() { info!("Virtual mouse: /dev/uinput unavailable, falling back to xdotool"); }
        Mutex::new(dev.ok())
    };

    static ref VIRTUAL_KEYBOARD: Mutex<Option<evdev::uinput::VirtualDevice>> = {
        let dev = (|| -> std::io::Result<evdev::uinput::VirtualDevice> {
            let mut keys = AttributeSet::new();
            for k in [Key::KEY_A,Key::KEY_B,Key::KEY_C,Key::KEY_D,Key::KEY_E,Key::KEY_F,
                      Key::KEY_G,Key::KEY_H,Key::KEY_I,Key::KEY_J,Key::KEY_K,Key::KEY_L,
                      Key::KEY_M,Key::KEY_N,Key::KEY_O,Key::KEY_P,Key::KEY_Q,Key::KEY_R,
                      Key::KEY_S,Key::KEY_T,Key::KEY_U,Key::KEY_V,Key::KEY_W,Key::KEY_X,
                      Key::KEY_Y,Key::KEY_Z,Key::KEY_0,Key::KEY_1,Key::KEY_2,Key::KEY_3,
                      Key::KEY_4,Key::KEY_5,Key::KEY_6,Key::KEY_7,Key::KEY_8,Key::KEY_9,
                      Key::KEY_ENTER,Key::KEY_BACKSPACE,Key::KEY_TAB,Key::KEY_SPACE,
                      Key::KEY_ESC,Key::KEY_DELETE,Key::KEY_INSERT,Key::KEY_HOME,Key::KEY_END,
                      Key::KEY_PAGEUP,Key::KEY_PAGEDOWN,Key::KEY_UP,Key::KEY_DOWN,
                      Key::KEY_LEFT,Key::KEY_RIGHT,Key::KEY_F1,Key::KEY_F2,Key::KEY_F3,
                      Key::KEY_F4,Key::KEY_F5,Key::KEY_F6,Key::KEY_F7,Key::KEY_F8,
                      Key::KEY_F9,Key::KEY_F10,Key::KEY_F11,Key::KEY_F12,
                      Key::KEY_LEFTSHIFT,Key::KEY_RIGHTSHIFT,Key::KEY_LEFTCTRL,
                      Key::KEY_RIGHTCTRL,Key::KEY_LEFTALT,Key::KEY_RIGHTALT,
                      Key::KEY_LEFTMETA,Key::KEY_RIGHTMETA,Key::KEY_CAPSLOCK,
                      Key::KEY_COMMA,Key::KEY_DOT,Key::KEY_SEMICOLON,Key::KEY_APOSTROPHE,
                      Key::KEY_LEFTBRACE,Key::KEY_RIGHTBRACE,Key::KEY_BACKSLASH,
                      Key::KEY_SLASH,Key::KEY_MINUS,Key::KEY_EQUAL,Key::KEY_GRAVE] {
                keys.insert(k);
            }
            VirtualDeviceBuilder::new()?
                .name("glide-kvm Virtual Keyboard")
                .with_keys(&keys)?
                .build()
        })();
        if dev.is_err() { info!("Virtual keyboard: /dev/uinput unavailable, falling back to xdotool"); }
        Mutex::new(dev.ok())
    };

    // Kali cursor position (tracked by accumulating incoming deltas)
    static ref CURSOR_X: AtomicI32 = AtomicI32::new(960);
    static ref CURSOR_Y: AtomicI32 = AtomicI32::new(540);

    // Which side of the laptop Kali lives on (0=Right,1=Left,2=Top,3=Bottom)
    static ref KALI_SIDE: AtomicU8 = AtomicU8::new(0);

    // Last client address — used to send ReturnToHost back
    static ref LAST_CLIENT: Mutex<Option<SocketAddr>> = Mutex::new(None);
}

/// Get Kali screen dimensions via xdotool getdisplaygeometry
#[cfg(target_os = "linux")]
fn get_screen_size() -> (i32, i32) {
    let xauth = std::env::var("XAUTHORITY").unwrap_or_default();
    if let Ok(out) = Command::new("xdotool")
        .env("DISPLAY", ":0")
        .env("XAUTHORITY", &xauth)
        .arg("getdisplaygeometry")
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        if parts.len() == 2 {
            let w = parts[0].parse().unwrap_or(1920);
            let h = parts[1].parse().unwrap_or(1080);
            info!("Kali screen size detected: {}x{}", w, h);
            return (w, h);
        }
    }
    info!("Could not detect screen size, defaulting to 1920x1080");
    (1920, 1080)
}

pub struct NetworkEngine {
    socket: UdpSocket,
}

impl NetworkEngine {
    pub async fn bind(addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        info!("Glide-KVM UDP Engine bound on {}", addr);

        // Detect Kali screen size at startup
        #[cfg(target_os = "linux")]
        {
            let (w, h) = get_screen_size();
            CURSOR_X.store(w / 2, Ordering::SeqCst);
            CURSOR_Y.store(h / 2, Ordering::SeqCst);
        }

        Ok(Self { socket })
    }

    #[allow(dead_code)]
    pub async fn send_event(&self, event: &InputEvent, target: SocketAddr) -> Result<()> {
        let bytes = bincode::serialize(event)?;
        self.socket.send_to(&bytes, target).await?;
        Ok(())
    }

    pub async fn run_server_loop(self) -> Result<()> {
        let mut buf = [0u8; 4096];
        info!("Server listening for active input events...");

        #[cfg(target_os = "linux")]
        let (screen_w, screen_h) = get_screen_size();
        #[cfg(not(target_os = "linux"))]
        let (screen_w, screen_h) = (1920i32, 1080i32);

        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    // Store the client address for sending ReturnToHost back
                    #[cfg(target_os = "linux")]
                    {
                        if let Ok(mut guard) = LAST_CLIENT.lock() {
                            *guard = Some(addr);
                        }
                    }

                    if let Ok(event) = bincode::deserialize::<InputEvent>(&buf[..len]) {
                        info!("Received input event from {}: {:?}", addr, event);

                        // Handle SetLayout before dispatching to OS input
                        #[cfg(target_os = "linux")]
                        if let InputEvent::SetLayout { side } = &event {
                            KALI_SIDE.store(*side, Ordering::SeqCst);
                            info!("Layout set: Kali is on side {}", side);
                            continue;
                        }

                        // Check if cursor has hit the return edge after this move
                        #[cfg(target_os = "linux")]
                        if let InputEvent::MouseMove { x, y } = &event {
                            let cx = CURSOR_X.fetch_add(*x, Ordering::SeqCst) + x;
                            let cy = CURSOR_Y.fetch_add(*y, Ordering::SeqCst) + y;

                            // Clamp to screen bounds
                            let cx = cx.max(0).min(screen_w);
                            let cy = cy.max(0).min(screen_h);
                            CURSOR_X.store(cx, Ordering::SeqCst);
                            CURSOR_Y.store(cy, Ordering::SeqCst);

                            let side = KALI_SIDE.load(Ordering::SeqCst);
                            let at_return_edge = match side {
                                0 => cx <= 2,              // Kali is RIGHT of laptop → return edge is LEFT of Kali
                                1 => cx >= screen_w - 2,   // Kali is LEFT  → return edge is RIGHT
                                2 => cy >= screen_h - 2,   // Kali is TOP   → return edge is BOTTOM
                                3 => cy <= 2,              // Kali is BOTTOM→ return edge is TOP
                                _ => false,
                            };

                            if at_return_edge {
                                info!("Cursor hit return edge at ({}, {}) — sending ReturnToHost", cx, cy);
                                // Send ReturnToHost back to Windows client
                                if let Ok(guard) = LAST_CLIENT.lock() {
                                    if let Some(client_addr) = *guard {
                                        // Send to client on port 24801 (return channel)
                                        let mut return_addr = client_addr;
                                        return_addr.set_port(24801);
                                        if let Ok(bytes) = bincode::serialize(&InputEvent::ReturnToHost) {
                                            let _ = std::net::UdpSocket::bind("0.0.0.0:0")
                                                .map(|s| s.send_to(&bytes, return_addr));
                                        }
                                    }
                                }
                                // Reset cursor to center after returning
                                CURSOR_X.store(screen_w / 2, Ordering::SeqCst);
                                CURSOR_Y.store(screen_h / 2, Ordering::SeqCst);
                            }
                        }

                        Self::simulate_os_input(&event);
                    }
                }
                Err(e) => { error!("UDP socket error: {}", e); }
            }
        }
    }

    fn xdotool(args: &[&str]) {
        #[cfg(target_os = "linux")]
        {
            let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
            let xauth   = std::env::var("XAUTHORITY").unwrap_or_default();
            let _ = Command::new("xdotool")
                .env("DISPLAY", &display)
                .env("XAUTHORITY", &xauth)
                .args(args)
                .spawn();
        }
    }

    fn simulate_os_input(event: &InputEvent) {
        #[cfg(target_os = "linux")]
        match event {
            InputEvent::MouseMove { x, y } => {
                if let Ok(mut g) = VIRTUAL_MOUSE.lock() {
                    if let Some(ref mut dev) = *g {
                        let ex = EvdevEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, *x);
                        let ey = EvdevEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, *y);
                        let _ = dev.emit(&[ex, ey]);
                        return;
                    }
                }
                Self::xdotool(&["mousemove_relative", "--", &x.to_string(), &y.to_string()]);
            }
            InputEvent::MouseMoveAbsolute { x, y } => {
                Self::xdotool(&["mousemove", &x.to_string(), &y.to_string()]);
            }
            InputEvent::MouseButton { button, pressed } => {
                if let Ok(mut g) = VIRTUAL_MOUSE.lock() {
                    if let Some(ref mut dev) = *g {
                        let k = match button { 1 => Key::BTN_LEFT, 2 => Key::BTN_MIDDLE, _ => Key::BTN_RIGHT };
                        let ev = EvdevEvent::new(EventType::KEY, k.0, if *pressed { 1 } else { 0 });
                        let _ = dev.emit(&[ev]);
                        return;
                    }
                }
                Self::xdotool(&[if *pressed { "mousedown" } else { "mouseup" }, &button.to_string()]);
            }
            InputEvent::Scroll { delta_y, .. } => {
                if let Ok(mut g) = VIRTUAL_MOUSE.lock() {
                    if let Some(ref mut dev) = *g {
                        let ev = EvdevEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL.0, *delta_y as i32);
                        let _ = dev.emit(&[ev]);
                        return;
                    }
                }
                Self::xdotool(&["click", if *delta_y > 0.0 { "4" } else { "5" }]);
            }
            InputEvent::KeyName { name, pressed } => {
                if let Some(evdev_key) = xdotool_name_to_evdev(name) {
                    if let Ok(mut g) = VIRTUAL_KEYBOARD.lock() {
                        if let Some(ref mut dev) = *g {
                            let val = if *pressed { 1 } else { 0 };
                            let ev   = EvdevEvent::new(EventType::KEY, evdev_key.0, val);
                            let sync = EvdevEvent::new(EventType::SYNCHRONIZATION, 0, 0);
                            let _ = dev.emit(&[ev, sync]);
                            return;
                        }
                    }
                }
                Self::xdotool(&[if *pressed { "keydown" } else { "keyup" }, name.as_str()]);
            }
            InputEvent::KeyPress { key_code, pressed } => {
                Self::xdotool(&[if *pressed { "keydown" } else { "keyup" }, &key_code.to_string()]);
            }
            _ => {}
        }
    }
}
