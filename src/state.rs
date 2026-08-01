use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Preview {
    pub name: String,
    pub container: String,
    pub image: String,
    pub port: u16,
    pub container_port: u16,
    pub created_at: u64,
}

fn home() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn default_path() -> PathBuf {
    match std::env::var("DEPLOYBOX_HOME") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => home().join(".deploybox"),
    }
    .join("state.json")
}

pub fn load(path: &Path) -> Result<Vec<Preview>, String> {
    match std::fs::read(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
    }
}

pub fn save(path: &Path, previews: &[Preview]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(previews).map_err(|e| e.to_string())?;
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

pub fn upsert(previews: &mut Vec<Preview>, preview: Preview) {
    previews.retain(|p| p.name != preview.name);
    previews.push(preview);
}
