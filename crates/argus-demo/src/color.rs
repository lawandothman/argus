//! Minimal truecolor ANSI styling.
//!
//! The session terminal reports `truecolor`, so 24-bit foreground colors render
//! directly. Bold and dim reset with `22m` (normal intensity) rather than `0m`,
//! so they compose with surrounding color without clearing it.

/// Wrap `text` in a 24-bit foreground color.
pub fn fg(text: &str, (r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[39m")
}

/// Render `text` in bold.
pub fn bold(text: &str) -> String {
    format!("\x1b[1m{text}\x1b[22m")
}

/// Render `text` dimmed.
pub fn dim(text: &str) -> String {
    format!("\x1b[2m{text}\x1b[22m")
}
