use std::collections::HashMap;

/// Parse pnpm-lock.yaml and extract installed versions.
/// Handles both pnpm v6+ (packages with /name@version keys) and v9+ (snapshots/packages).
pub fn parse(content: &str) -> Option<HashMap<String, String>> {
    let yaml: serde_yml::Value = serde_yml::from_str(content).ok()?;
    let root = yaml.as_mapping()?;

    let mut versions = HashMap::new();

    // Try importers first (workspace root dependencies)
    if let Some(importers) = root
        .get(&serde_yml::Value::String("importers".into()))
        .and_then(|v| v.as_mapping())
    {
        // Look at the root importer "."
        if let Some(root_importer) = importers
            .get(&serde_yml::Value::String(".".into()))
            .and_then(|v| v.as_mapping())
        {
            for section in ["dependencies", "devDependencies", "optionalDependencies"] {
                if let Some(deps) = root_importer
                    .get(&serde_yml::Value::String(section.into()))
                    .and_then(|v| v.as_mapping())
                {
                    for (name_val, info) in deps {
                        let name = name_val.as_str()?;
                        // pnpm v9: { version: "1.2.3", ... } or just "1.2.3"
                        let ver = if let Some(map) = info.as_mapping() {
                            map.get(&serde_yml::Value::String("version".into()))
                                .and_then(|v| v.as_str())
                                .map(extract_version_from_pnpm)
                        } else {
                            info.as_str().map(extract_version_from_pnpm)
                        };
                        if let Some(v) = ver {
                            versions.insert(name.to_string(), v);
                        }
                    }
                }
            }
        }
    }

    // Also try packages key for additional version info
    if let Some(packages) = root
        .get(&serde_yml::Value::String("packages".into()))
        .and_then(|v| v.as_mapping())
    {
        for (key_val, _) in packages {
            if let Some(key) = key_val.as_str() {
                // Format: "/name@version" or "name@version"
                if let Some((name, ver)) = parse_pnpm_package_key(key) {
                    versions.entry(name).or_insert(ver);
                }
            }
        }
    }

    if versions.is_empty() {
        None
    } else {
        Some(versions)
    }
}

/// Extract clean version from pnpm version strings like "1.2.3(peer@4.0.0)"
fn extract_version_from_pnpm(s: &str) -> String {
    // Strip anything after ( which is peer dependency info
    s.split('(').next().unwrap_or(s).trim().to_string()
}

/// Parse pnpm package key like "/express@4.18.2" or "@scope/pkg@1.0.0"
fn parse_pnpm_package_key(key: &str) -> Option<(String, String)> {
    let key = key.strip_prefix('/').unwrap_or(key);

    // Find the last @ that's not at position 0 (to handle @scope/pkg)
    let at_pos = if key.starts_with('@') {
        // Scoped package: find @ after the first /
        key[1..].find('@').map(|p| p + 1)
    } else {
        key.rfind('@')
    };

    let at_pos = at_pos?;
    let name = &key[..at_pos];
    let version = extract_version_from_pnpm(&key[at_pos + 1..]);

    if name.is_empty() || version.is_empty() {
        return None;
    }

    Some((name.to_string(), version))
}
