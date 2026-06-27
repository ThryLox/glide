use arboard::Clipboard;
use tracing::{info, error};

#[allow(dead_code)]
pub struct ClipboardManager {
    clipboard: Option<Clipboard>,
}

#[allow(dead_code)]
impl ClipboardManager {
    pub fn new() -> Self {
        match Clipboard::new() {
            Ok(cb) => Self { clipboard: Some(cb) },
            Err(e) => {
                error!("Failed to initialize OS clipboard: {}", e);
                Self { clipboard: None }
            }
        }
    }

    pub fn get_text(&mut self) -> Option<String> {
        if let Some(ref mut cb) = self.clipboard {
            cb.get_text().ok()
        } else {
            None
        }
    }

    pub fn set_text(&mut self, text: &str) -> bool {
        if let Some(ref mut cb) = self.clipboard {
            if let Err(e) = cb.set_text(text.to_string()) {
                error!("Failed to set clipboard text: {}", e);
                false
            } else {
                info!("Clipboard text updated successfully");
                true
            }
        } else {
            false
        }
    }
}
