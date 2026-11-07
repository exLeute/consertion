//! System clipboard with fallbacks: OSC 52 over SSH, and an in-app buffer if nothing else works.

use std::io::Write;

pub struct Clipboard {
    // Kept alive for the whole run: on X11/Wayland the owning process serves the contents.
    sys: Option<arboard::Clipboard>,
    local: String,
}

impl Clipboard {
    pub fn new() -> Self {
        Self { sys: arboard::Clipboard::new().ok(), local: String::new() }
    }

    pub fn set(&mut self, text: &str) {
        self.local = text.to_string();
        let ok = self.sys.as_mut().is_some_and(|c| c.set_text(text).is_ok());
        if !ok || std::env::var_os("SSH_TTY").is_some() {
            // OSC 52 lets the *local* terminal set its clipboard, even through SSH.
            let b64 = crate::catalog::text::base64_encode(text.as_bytes());
            let _ = write!(std::io::stdout(), "\x1b]52;c;{b64}\x07");
            let _ = std::io::stdout().flush();
        }
    }

    pub fn get(&mut self) -> String {
        self.sys.as_mut().and_then(|c| c.get_text().ok()).unwrap_or_else(|| self.local.clone())
    }
}
