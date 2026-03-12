mod bun;
mod npm;
mod pnpm;
mod yarn;

use std::collections::HashMap;
use std::path::Path;

pub fn resolve_installed_versions(dir: &Path) -> HashMap<String, String> {
    let mut current = Some(dir);

    while let Some(d) = current {
        let lock_path = d.join("package-lock.json");
        if lock_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&lock_path) {
                if let Some(versions) = npm::parse(&content) {
                    return versions;
                }
            }
        }

        let lock_path = d.join("pnpm-lock.yaml");
        if lock_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&lock_path) {
                if let Some(versions) = pnpm::parse(&content) {
                    return versions;
                }
            }
        }

        let lock_path = d.join("yarn.lock");
        if lock_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&lock_path) {
                if let Some(versions) = yarn::parse(&content) {
                    return versions;
                }
            }
        }

        let lock_path = d.join("bun.lock");
        if lock_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&lock_path) {
                if let Some(versions) = bun::parse(&content) {
                    return versions;
                }
            }
        }

        current = d.parent();
    }

    HashMap::new()
}
