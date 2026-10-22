//! Favourite instruments, stored in ~/.opendembow_favorites (one name per line).

use std::{collections::HashSet, path::PathBuf};

fn path() -> Option<PathBuf> {
    let h = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(h).join(".opendembow_favorites"))
}

pub fn load() -> HashSet<String> {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}

pub fn save(f: &HashSet<String>) {
    if let Some(p) = path() {
        let mut v: Vec<&str> = f.iter().map(|s| s.as_str()).collect();
        v.sort();
        let _ = std::fs::write(p, v.join("\n"));
    }
}