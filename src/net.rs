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

// Mapping from xdotool key name string → evdev Key enum
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
    // Virtual mouse: relative axes + mouse buttons
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
        if dev.is_err() {
            info!("Virtual mouse: /dev/uinput not available, falling back to xdotool");
        }
        Mutex::new(dev.ok())
    };

    // Virtual keyboard: all standard keys
    static ref VIRTUAL_KEYBOARD: Mutex<Option<evdev::uinput::VirtualDevice>> = {
        let dev = (|| -> std::io::Result<evdev::uinput::VirtualDevice> {
            let mut keys = AttributeSet::new();
            // Letters
            for k in [Key::KEY_A,Key::KEY_B,Key::KEY_C,Key::KEY_D,Key::KEY_E,
                      Key::KEY_F,Key::KEY_G,Key::KEY_H,Key::KEY_I,Key::KEY_J,
                      Key::KEY_K,Key::KEY_L,Key::KEY_M,Key::KEY_N,Key::KEY_O,
                      Key::KEY_P,Key::KEY_Q,Key::KEY_R,Key::KEY_S,Key::KEY_T,
                      Key::KEY_U,Key::KEY_V,Key::KEY_W,Key::KEY_X,Key::KEY_Y,Key::KEY_Z] {
                keys.insert(k);
            }
            // Digits
            for k in [Key::KEY_0,Key::KEY_1,Key::KEY_2,Key::KEY_3,Key::KEY_4,
                      Key::KEY_5,Key::KEY_6,Key::KEY_7,Key::KEY_8,Key::KEY_9] {
                keys.insert(k);
            }
            // Special keys
            for k in [Key::KEY_ENTER,Key::KEY_BACKSPACE,Key::KEY_TAB,Key::KEY_SPACE,
                      Key::KEY_ESC,Key::KEY_DELETE,Key::KEY_INSERT,
                      Key::KEY_HOME,Key::KEY_END,Key::KEY_PAGEUP,Key::KEY_PAGEDOWN,
                      Key::KEY_UP,Key::KEY_DOWN,Key::KEY_LEFT,Key::KEY_RIGHT,
                      Key::KEY_F1,Key::KEY_F2,Key::KEY_F3,Key::KEY_F4,
                      Key::KEY_F5,Key::KEY_F6,Key::KEY_F7,Key::KEY_F8,
                      Key::KEY_F9,Key::KEY_F10,Key::KEY_F11,Key::KEY_F12,
                      Key::KEY_LEFTSHIFT,Key::KEY_RIGHTSHIFT,
                      Key::KEY_LEFTCTRL,Key::KEY_RIGHTCTRL,
                      Key::KEY_LEFTALT,Key::KEY_RIGHTALT,
                      Key::KEY_LEFTMETA,Key::KEY_RIGHTMETA,
                      Key::KEY_CAPSLOCK,
                      Key::KEY_COMMA,Key::KEY_DOT,Key::KEY_SEMICOLON,
                      Key::KEY_APOSTROPHE,Key::KEY_LEFTBRACE,Key::KEY_RIGHTBRACE,
                      Key::KEY_BACKSLASH,Key::KEY_SLASH,Key::KEY_MINUS,
                      Key::KEY_EQUAL,Key::KEY_GRAVE] {
                keys.insert(k);
            }
            VirtualDeviceBuilder::new()?
                .name("glide-kvm Virtual Keyboard")
                .with_keys(&keys)?
                .build()
        })();
        if dev.is_err() {
            info!("Virtual keyboard: /dev/uinput not available, falling back to xdotool");
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
        let mut buf = [0u8; 4096];
        info!("Server listening for active input events...");
        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    if let Ok(event) = bincode::deserialize::<InputEvent>(&buf[..len]) {
                        info!("Received input event from {}: {:?}", addr, event);
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
            let display  = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
            let xauth    = std::env::var("XAUTHORITY")
                .unwrap_or_else(|_| "/run/user/1000/.mutter-Xwaylandauth.4QFTR3".to_string());
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
                let action = if *pressed { "mousedown" } else { "mouseup" };
                Self::xdotool(&[action, &button.to_string()]);
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
            // KeyName: try uinput keyboard first, fall back to xdotool
            InputEvent::KeyName { name, pressed } => {
                if let Some(evdev_key) = xdotool_name_to_evdev(name) {
                    if let Ok(mut g) = VIRTUAL_KEYBOARD.lock() {
                        if let Some(ref mut dev) = *g {
                            let val = if *pressed { 1 } else { 0 };
                            let ev = EvdevEvent::new(EventType::KEY, evdev_key.0, val);
                            // Emit key event + mandatory sync event
                            let sync = EvdevEvent::new(EventType::SYNCHRONIZATION, 0, 0);
                            let _ = dev.emit(&[ev, sync]);
                            return;
                        }
                    }
                }
                // xdotool fallback
                let action = if *pressed { "keydown" } else { "keyup" };
                Self::xdotool(&[action, name.as_str()]);
            }
            InputEvent::KeyPress { key_code, pressed } => {
                let action = if *pressed { "keydown" } else { "keyup" };
                Self::xdotool(&[action, &key_code.to_string()]);
            }
            _ => {}
        }
    }
}
