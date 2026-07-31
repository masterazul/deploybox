use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

pub fn host_label(head: &str) -> Option<String> {
    for line in head.split("\r\n") {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("host:") {
            let host = rest.trim();
            let host = host.split(':').next().unwrap_or(host);
            let label = host.split('.').next().unwrap_or(host);
            if !label.is_empty() {
                return Some(label.to_string());
            }
        }
    }
    None
}

pub fn content_length(head: &str) -> Option<usize> {
    for line in head.split("\r\n") {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

pub fn rewrite_head(head: &str) -> String {
    let mut kept: Vec<&str> = head
        .split("\r\n")
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            !line.is_empty()
                && !lower.starts_with("connection:")
                && !lower.starts_with("proxy-connection:")
        })
        .collect();
    kept.push("Connection: close");
    let mut out = kept.join("\r\n");
    out.push_str("\r\n\r\n");
    out
}

pub fn serve(port: u16, routes: HashMap<String, u16>) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!(
        "proxy on http://127.0.0.1:{port} routing {} preview(s) by <name>.localhost",
        routes.len()
    );
    for stream in listener.incoming().flatten() {
        let routes = routes.clone();
        std::thread::spawn(move || {
            let _ = handle(stream, &routes);
        });
    }
    Ok(())
}

fn handle(mut client: TcpStream, routes: &HashMap<String, u16>) -> std::io::Result<()> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut split = None;
    while split.is_none() {
        let n = client.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        split = find_header_end(&buf);
        if buf.len() > 64 * 1024 {
            break;
        }
    }

    let Some(end) = split else {
        return Ok(());
    };
    let head = String::from_utf8_lossy(&buf[..end]).to_string();
    let body = &buf[end + 4..];

    let target = host_label(&head).and_then(|label| routes.get(&label).copied());
    let Some(upstream_port) = target else {
        let _ = client.write_all(
            b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 14\r\nConnection: close\r\n\r\nno such preview",
        );
        return Ok(());
    };

    let mut upstream = TcpStream::connect(("127.0.0.1", upstream_port))?;
    upstream.write_all(rewrite_head(&head).as_bytes())?;
    upstream.write_all(body)?;

    if let Some(total) = content_length(&head) {
        let mut remaining = total.saturating_sub(body.len());
        let mut chunk = [0u8; 4096];
        while remaining > 0 {
            let n = client.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            let take = n.min(remaining);
            upstream.write_all(&chunk[..take])?;
            remaining -= take;
        }
    }

    std::io::copy(&mut upstream, &mut client)?;
    Ok(())
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}
