use deploybox::docker::parse_ps;
use deploybox::proxy::{content_length, host_label, rewrite_head};
use deploybox::util::slug;

#[test]
fn slugify_names() {
    assert_eq!(slug("My Cool App!"), "my-cool-app");
    assert_eq!(slug("api_v2"), "api-v2");
    assert_eq!(slug("   ---"), "app");
}

#[test]
fn host_label_from_headers() {
    let head = "GET / HTTP/1.1\r\nHost: blog.localhost:8080\r\nAccept: */*";
    assert_eq!(host_label(head).as_deref(), Some("blog"));
    assert_eq!(
        host_label("GET / HTTP/1.1\r\nHost: api.localhost").as_deref(),
        Some("api")
    );
    assert_eq!(host_label("GET / HTTP/1.1\r\nUser-Agent: x"), None);
}

#[test]
fn rewrite_head_forces_close() {
    let head = "GET / HTTP/1.1\r\nHost: a.localhost\r\nConnection: keep-alive";
    let out = rewrite_head(head);
    let lower = out.to_ascii_lowercase();
    assert!(lower.contains("connection: close"));
    assert!(!lower.contains("keep-alive"));
    assert!(out.ends_with("\r\n\r\n"));
}

#[test]
fn reads_content_length() {
    let head = "POST /x HTTP/1.1\r\nHost: a.localhost\r\nContent-Length: 42\r\nAccept: */*";
    assert_eq!(content_length(head), Some(42));
    assert_eq!(content_length("GET / HTTP/1.1\r\nHost: a.localhost"), None);
    assert_eq!(content_length("POST / HTTP/1.1\r\nContent-Length: nope"), None);
}

#[test]
fn parse_docker_ps() {
    let raw = "deploybox-api|Up 3 minutes|127.0.0.1:8100->80/tcp\n\ndeploybox-web|Exited (0)|";
    let rows = parse_ps(raw);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "deploybox-api");
    assert_eq!(rows[1].1, "Exited (0)");
}
