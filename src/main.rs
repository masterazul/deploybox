use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use deploybox::state::{self, Preview};
use deploybox::{docker, proxy, util};

const USAGE: &str = "deploybox - one-command preview deployments on docker

usage:
  deploybox up <path> [--name <n>] [--container-port <p>] [--image <ref>] [--port <p>]
  deploybox ls
  deploybox down <name>
  deploybox proxy [--port <p>]

up      build the Dockerfile at <path> (or run --image) and expose it on a free port
ls      list previews and their container status
down    stop and remove a preview
proxy   reverse-proxy every preview under http://<name>.localhost:<port>
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("up") => up(&args[1..]),
        Some("ls") => ls(),
        Some("down") => down(&args[1..]),
        Some("proxy") => run_proxy(&args[1..]),
        Some("version") | Some("-V") => {
            println!("deploybox {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("help") | Some("-h") | Some("--help") => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            print!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn first_positional(args: &[String]) -> Option<&String> {
    let value_flags = ["--name", "--container-port", "--image", "--port"];
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if value_flags.contains(&arg.as_str()) {
            i += 2;
        } else if arg.starts_with("--") {
            i += 1;
        } else {
            return Some(arg);
        }
    }
    None
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn up(args: &[String]) -> ExitCode {
    let path = first_positional(args);
    let image_flag = flag(args, "--image");
    if path.is_none() && image_flag.is_none() {
        return fail("up needs a <path> or --image <ref>");
    }

    let default_name = image_flag
        .map(|i| i.split([':', '/']).next_back().unwrap_or(i))
        .or_else(|| path.and_then(|p| Path::new(p).file_name().and_then(|s| s.to_str())))
        .unwrap_or("app");
    let name = util::slug(flag(args, "--name").unwrap_or(default_name));
    let container_port: u16 = flag(args, "--container-port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(80);

    let image = match image_flag {
        Some(reference) => reference.to_string(),
        None => {
            let tag = format!("deploybox/{name}:latest");
            println!("building {} -> {tag}", path.unwrap());
            if let Err(e) = docker::build(path.unwrap(), &tag) {
                return fail(&format!("build failed: {e}"));
            }
            tag
        }
    };

    let host_port = match flag(args, "--port").and_then(|p| p.parse().ok()) {
        Some(p) => p,
        None => match util::free_port(8100, 8999) {
            Some(p) => p,
            None => return fail("no free port in 8100-8999"),
        },
    };

    let container = format!("deploybox-{name}");
    let _ = docker::remove(&container);
    let id = match docker::run(&image, &container, host_port, container_port) {
        Ok(id) => id,
        Err(e) => return fail(&format!("run failed: {e}")),
    };

    let path_state = state::default_path();
    let mut previews = state::load(&path_state);
    state::upsert(
        &mut previews,
        Preview {
            name: name.clone(),
            container,
            image,
            port: host_port,
            container_port,
            created_at: now(),
        },
    );
    if let Err(e) = state::save(&path_state, &previews) {
        eprintln!("warning: could not persist state: {e}");
    }

    println!("preview '{name}' is up ({})", &id[..id.len().min(12)]);
    println!("  direct: http://127.0.0.1:{host_port}");
    println!("  proxied: http://{name}.localhost:8080  (run `deploybox proxy`)");
    ExitCode::SUCCESS
}

fn ls() -> ExitCode {
    let previews = state::load(&state::default_path());
    let status: HashMap<String, String> = match docker::ps_raw() {
        Ok(raw) => docker::parse_ps(&raw)
            .into_iter()
            .map(|(name, st, _ports)| (name, st))
            .collect(),
        Err(_) => HashMap::new(),
    };
    if previews.is_empty() {
        println!("no previews");
        return ExitCode::SUCCESS;
    }
    println!("{:<18} {:<8} {:<28} status", "name", "port", "image");
    for p in &previews {
        let st = status
            .get(&p.container)
            .cloned()
            .unwrap_or_else(|| "gone".into());
        println!("{:<18} {:<8} {:<28} {}", p.name, p.port, p.image, st);
    }
    ExitCode::SUCCESS
}

fn down(args: &[String]) -> ExitCode {
    let Some(name) = first_positional(args) else {
        return fail("down needs a <name>");
    };
    let path = state::default_path();
    let mut previews = state::load(&path);
    let Some(preview) = previews.iter().find(|p| &p.name == name).cloned() else {
        return fail(&format!("no preview named '{name}'"));
    };
    if let Err(e) = docker::remove(&preview.container) {
        eprintln!("warning: docker rm: {e}");
    }
    previews.retain(|p| &p.name != name);
    if let Err(e) = state::save(&path, &previews) {
        return fail(&e);
    }
    println!("removed preview '{name}'");
    ExitCode::SUCCESS
}

fn run_proxy(args: &[String]) -> ExitCode {
    let port: u16 = flag(args, "--port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let previews = state::load(&state::default_path());
    if previews.is_empty() {
        return fail("no previews to route");
    }
    let routes: HashMap<String, u16> = previews.into_iter().map(|p| (p.name, p.port)).collect();
    match proxy::serve(port, routes) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(&format!("proxy: {e}")),
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::first_positional;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn first_positional_skips_flag_values() {
        assert_eq!(
            first_positional(&args(&["--name", "web", "./app"])).unwrap(),
            "./app"
        );
        assert_eq!(
            first_positional(&args(&["./app", "--name", "web"])).unwrap(),
            "./app"
        );
        assert_eq!(first_positional(&args(&["--image", "traefik/whoami"])), None);
        assert!(first_positional(&args(&[])).is_none());
    }
}
