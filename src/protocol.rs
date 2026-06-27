use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    MouseMove { x: i32, y: i32 },
    MouseMoveAbsolute { x: i32, y: i32 },
    MouseButton { button: u8, pressed: bool },
    KeyPress { key_code: u32, pressed: bool },
    KeyName { name: String, pressed: bool },  // xdotool-compatible key name
    Scroll { delta_x: f32, delta_y: f32 },
    LockState { locked: bool },
    SetLayout { side: u8 },   // Sent by Windows client on connect: 0=Right,1=Left,2=Top,3=Bottom
    ReturnToHost,             // Sent by Kali → Windows when cursor hits the return edge
}


#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessage {
    Handshake { machine_name: String, screen_width: u32, screen_height: u32 },
    ClipboardText(String),
    ClipboardImage { width: usize, height: usize, bytes: Vec<u8> },
    FileStart { file_name: String, total_bytes: u64 },
    FileChunk { chunk_index: u64, data: Vec<u8> },
    FileEnd,
}
