//! Observability hook string validation (M8 narrow v0).
//!
//! Policy: hook labels are single-line ASCII identifiers extended with `_`, `-`, `.`.
//! No escape sequences beyond a literal backslash-double-quote in source (`\"` → `"`).

/// Maximum byte length for tap/record/profile/replay hook strings.
pub const MAX_OBS_HOOK_LABEL_BYTES: usize = 128;

/// Validate a decoded observability hook label (tap/record/profile/replay args).
/// Also used defense-in-depth for unsafe/extern names on boundary surfaces (host/foreign lowering prep).
pub fn validate_obs_hook_label(label: &str) -> Result<(), &'static str> {
    if label.is_empty() {
        return Err("observability hook label must not be empty");
    }
    if label.len() > MAX_OBS_HOOK_LABEL_BYTES {
        return Err("observability hook label exceeds maximum length");
    }
    if label
        .chars()
        .any(|c| c.is_control() || c == '\n' || c == '\r')
    {
        return Err("observability hook label must not contain control characters");
    }
    if !label
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err("observability hook label contains invalid character");
    }
    Ok(())
}

/// Decode a lexer `StringLit` token body (quotes stripped). Rejects unsupported escapes.
pub fn decode_obs_hook_string_lit(raw: &str) -> Result<String, &'static str> {
    let inner = raw.trim_matches('"');
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some(_) => {
                    return Err("observability hook string supports only \\\" escape sequences");
                }
                None => return Err("unterminated escape in observability hook string"),
            }
        } else {
            out.push(c);
        }
    }
    validate_obs_hook_label(&out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_multiline_hook_label() {
        assert!(validate_obs_hook_label("a\ninclude!").is_err());
    }

    #[test]
    fn rejects_control_chars() {
        assert!(validate_obs_hook_label("a\x00b").is_err());
    }

    #[test]
    fn accepts_safe_charset() {
        assert!(validate_obs_hook_label("health_probe.v1").is_ok());
    }

    #[test]
    fn decode_rejects_include_escape() {
        assert!(decode_obs_hook_string_lit("\"a\\nb\"").is_err());
    }

    #[test]
    fn validate_boundary_names_as_defense() {
        // prep for unsafe/extern body/name sanitization (hook label policy reused)
        assert!(validate_obs_hook_label("HostCopy").is_ok());
        assert!(validate_obs_hook_label("host_helper").is_ok());
    }
}
