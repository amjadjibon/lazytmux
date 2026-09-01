/// Sanitizes user input for tmux session and window names.
/// - Strips control characters, terminal escape sequences, and null bytes
/// - Strips leading hyphens (which tmux CLI could parse as options)
/// - Replaces colon ':' and period '.' (used internally by tmux as target separators) with '_'
/// - Trims leading/trailing whitespace
/// - Caps length at 64 characters to prevent buffer overflow/DoS
pub fn sanitize_tmux_name(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if c == ':' || c == '.' { '_' } else { c })
        .collect();

    // Trim whitespace
    let mut trimmed = cleaned.trim();

    // Strip leading dashes to prevent option injection
    while trimmed.starts_with('-') {
        trimmed = &trimmed[1..];
    }
    trimmed = trimmed.trim();

    // Cap length to 64 chars
    if trimmed.len() > 64 {
        trimmed = &trimmed[..64];
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_normal_name() {
        assert_eq!(sanitize_tmux_name("my-project"), "my-project");
        assert_eq!(sanitize_tmux_name(" dev_server "), "dev_server");
    }

    #[test]
    fn test_sanitize_dangerous_characters() {
        // Strip control characters & newlines
        assert_eq!(sanitize_tmux_name("dev\n\r\0server"), "devserver");
        // Strip terminal escape sequence
        assert_eq!(sanitize_tmux_name("\x1b[31mred\x1b[0m"), "[31mred[0m");
        // Replace colon and period
        assert_eq!(sanitize_tmux_name("session:1.0"), "session_1_0");
        // Strip leading hyphen
        assert_eq!(sanitize_tmux_name("-flag"), "flag");
        assert_eq!(sanitize_tmux_name("---safe"), "safe");
    }

    #[test]
    fn test_sanitize_length_cap() {
        let long_name = "a".repeat(100);
        let sanitized = sanitize_tmux_name(&long_name);
        assert_eq!(sanitized.len(), 64);
    }
}
