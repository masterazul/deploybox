use std::process::Command;

pub const LABEL: &str = "deploybox=1";

pub fn cli(args: &[&str]) -> Result<String, String> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .map_err(|e| format!("could not run docker: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn build(context: &str, tag: &str) -> Result<(), String> {
    cli(&["build", "-t", tag, context]).map(|_| ())
}

pub fn run(image: &str, name: &str, host_port: u16, container_port: u16) -> Result<String, String> {
    let publish = format!("127.0.0.1:{host_port}:{container_port}");
    cli(&[
        "run", "-d", "--label", LABEL, "--name", name, "-p", &publish, image,
    ])
}

pub fn remove(reference: &str) -> Result<(), String> {
    cli(&["rm", "-f", reference]).map(|_| ())
}

pub fn ps_raw() -> Result<String, String> {
    cli(&[
        "ps",
        "-a",
        "--filter",
        "label=deploybox=1",
        "--format",
        "{{.Names}}|{{.Status}}|{{.Ports}}",
    ])
}

pub fn parse_ps(raw: &str) -> Vec<(String, String, String)> {
    raw.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '|');
            let name = parts.next()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let status = parts.next().unwrap_or("").trim().to_string();
            let ports = parts.next().unwrap_or("").trim().to_string();
            Some((name, status, ports))
        })
        .collect()
}
