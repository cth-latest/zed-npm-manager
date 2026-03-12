use std::collections::HashMap;

pub fn parse(content: &str) -> Option<HashMap<String, String>> {
    let yaml: serde_yml::Value = serde_yml::from_str(content).ok()?;
    let root = yaml.as_mapping()?;

    let mut versions = HashMap::new();

    if let Some(importers) = root
        .get(&serde_yml::Value::String("importers".into()))
        .and_then(|v| v.as_mapping())
    {
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

    if let Some(packages) = root
        .get(&serde_yml::Value::String("packages".into()))
        .and_then(|v| v.as_mapping())
    {
        for (key_val, _) in packages {
            if let Some(key) = key_val.as_str() {
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

fn extract_version_from_pnpm(s: &str) -> String {
    s.split('(').next().unwrap_or(s).trim().to_string()
}

fn parse_pnpm_package_key(key: &str) -> Option<(String, String)> {
    let key = key.strip_prefix('/').unwrap_or(key);

    let at_pos = if key.starts_with('@') {
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
