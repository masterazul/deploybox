use std::net::TcpListener;

pub fn slug(input: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "app".to_string()
    } else {
        trimmed
    }
}

pub fn free_port(start: u16, end: u16, taken: &[u16]) -> Option<u16> {
    (start..=end)
        .find(|port| !taken.contains(port) && TcpListener::bind(("127.0.0.1", *port)).is_ok())
}
