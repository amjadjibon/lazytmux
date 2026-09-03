/// Maximum length of a tmux session or window name, in characters.
pub const MAX_NAME_CHARS: usize = 64;

/// Sanitizes user input for tmux session and window names.
/// - Strips control characters, terminal escape sequences, and null bytes
/// - Strips leading hyphens (which tmux CLI could parse as options)
/// - Replaces colon ':' and period '.' (used internally by tmux as target separators) with '_'
/// - Trims leading/trailing whitespace
/// - Caps length at `MAX_NAME_CHARS` characters to prevent buffer overflow/DoS
pub fn sanitize_tmux_name(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if c == ':' || c == '.' { '_' } else { c })
        .collect();

    // Strip leading dashes and whitespace until neither remains. Trimming and
    // dash-stripping must alternate: "- -flag" would otherwise leave "-flag".
    let mut trimmed = cleaned.trim();
    loop {
        let stripped = trimmed.strip_prefix('-').unwrap_or(trimmed).trim();
        if stripped == trimmed {
            break;
        }
        trimmed = stripped;
    }

    // Cap length by characters, not bytes: slicing on a byte index would panic
    // in the middle of a multi-byte character.
    trimmed.chars().take(MAX_NAME_CHARS).collect()
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
    fn test_sanitize_never_returns_leading_dash() {
        // Dashes separated by whitespace must not survive the strip loop.
        assert_eq!(sanitize_tmux_name("- -kill-session"), "kill-session");
        assert_eq!(sanitize_tmux_name("-\t-s"), "s");
        assert_eq!(sanitize_tmux_name("  -  -  -x  "), "x");
        assert_eq!(sanitize_tmux_name("- - -"), "");
        for input in ["- -x", "-\t-\t-y", " - - z "] {
            assert!(!sanitize_tmux_name(input).starts_with('-'));
        }
    }

    #[test]
    fn test_sanitize_length_cap() {
        let long_name = "a".repeat(100);
        let sanitized = sanitize_tmux_name(&long_name);
        assert_eq!(sanitized.chars().count(), MAX_NAME_CHARS);
    }

    #[test]
    fn test_sanitize_multibyte_name_does_not_panic() {
        // A byte-index cap would split '日' (3 bytes) mid-character and panic.
        let sanitized = sanitize_tmux_name(&"日".repeat(100));
        assert_eq!(sanitized.chars().count(), MAX_NAME_CHARS);

        for filler in ["日", "🚀", "é", "한"] {
            let out = sanitize_tmux_name(&filler.repeat(80));
            assert_eq!(out.chars().count(), MAX_NAME_CHARS);
        }
    }
}
