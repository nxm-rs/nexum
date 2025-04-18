//! Display utilities for the Keycard CLI

use colored::Colorize;
use std::fmt::Display;

/// A formatted section title
pub struct SectionTitle(pub &'static str);

impl Display for SectionTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\n{}", self.0.bold().underline())
    }
}

/// Format a section header
pub fn section_title(title: &'static str) -> SectionTitle {
    SectionTitle(title)
}

/// Format a success message
pub fn success(message: &str) -> String {
    format!("✅ {}", message.green().bold())
}

/// Format a warning message
pub fn warning(message: &str) -> String {
    format!("⚠️  {}", message.yellow().bold())
}

/// Format an info message
pub fn info(message: &str) -> String {
    format!("ℹ️  {}", message.blue())
}

/// Format a key-value section for important outputs
pub fn key_value_box(title: &str, items: Vec<(&str, String)>) -> String {
    let mut result = format!("{}", title.bold().underline());

    for (key, value) in items {
        result.push_str(&format!("\n  {}: {}", key.bold(), value));
    }

    result
}

/// Format a sensitive data warning
pub fn sensitive_data_warning() -> String {
    warning("SAVE THIS INFORMATION SECURELY - IT WON'T BE SHOWN AGAIN!")
}
