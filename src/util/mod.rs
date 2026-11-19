pub mod http;

pub(crate) fn sanitize_filename(value: &str, default_name: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            other => other,
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        default_name.to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn env_bool(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}
