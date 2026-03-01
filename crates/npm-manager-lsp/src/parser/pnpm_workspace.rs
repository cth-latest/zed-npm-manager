use regex::Regex;

use crate::types::{clean_version, DependencyEntry, DependencyType, VersionStatus};

pub fn parse(text: &str) -> Vec<DependencyEntry> {
    let yaml: serde_yml::Value = match serde_yml::from_str(text) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let mapping = match yaml.as_mapping() {
        Some(m) => m,
        None => return vec![],
    };

    let mut all_deps: Vec<(String, String, DependencyType)> = Vec::new();

    // Single catalog: { catalog: { pkg: "^1.0.0" } }
    if let Some(catalog) = mapping.get(&serde_yml::Value::String("catalog".into())) {
        if let Some(map) = catalog.as_mapping() {
            for (k, v) in map {
                if let (Some(name), Some(version)) = (k.as_str(), v.as_str()) {
                    all_deps.push((
                        name.to_string(),
                        version.to_string(),
                        DependencyType::Catalog,
                    ));
                }
            }
        }
    }

    // Named catalogs: { catalogs: { default: { pkg: "^1.0.0" } } }
    if let Some(catalogs) = mapping.get(&serde_yml::Value::String("catalogs".into())) {
        if let Some(catalogs_map) = catalogs.as_mapping() {
            for (catalog_name_val, catalog_obj) in catalogs_map {
                let catalog_name = match catalog_name_val.as_str() {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                if let Some(deps_map) = catalog_obj.as_mapping() {
                    for (k, v) in deps_map {
                        if let (Some(name), Some(version)) = (k.as_str(), v.as_str()) {
                            all_deps.push((
                                name.to_string(),
                                version.to_string(),
                                DependencyType::NamedCatalog(catalog_name.clone()),
                            ));
                        }
                    }
                }
            }
        }
    }

    let mut entries = Vec::with_capacity(all_deps.len());
    for (name, raw_version, dep_type) in all_deps {
        let cleaned = clean_version(&raw_version).to_string();
        if cleaned.is_empty() {
            continue;
        }

        if let Some((line, col_start, col_end)) = find_yaml_dep_position(text, &name, &raw_version)
        {
            entries.push(DependencyEntry {
                name,
                raw_version,
                clean_version: cleaned,
                line,
                col_start,
                col_end,
                dep_type,
                status: VersionStatus::Loading,
                installed_version: None,
            });
        }
    }

    entries
}

/// Find position of a YAML dependency line like `  package-name: ^1.2.3`
fn find_yaml_dep_position(text: &str, name: &str, version: &str) -> Option<(u32, u32, u32)> {
    // YAML keys may be quoted or unquoted
    let pattern = format!(
        r#"(?m)^(\s*)['"]?{}['"]?\s*:\s*['"]?{}['"]?\s*$"#,
        regex::escape(name),
        regex::escape(version)
    );
    let re = match Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return None,
    };

    let m = re.find(text)?;
    let start_byte = m.start();

    let mut line = 0u32;
    let mut line_start_byte = 0usize;

    for (i, ch) in text[..start_byte].char_indices() {
        if ch == '\n' {
            line += 1;
            line_start_byte = i + 1;
        }
    }
    let col_start = (start_byte - line_start_byte) as u32;
    let end_col = col_start + m.len() as u32;

    Some((line, col_start, end_col))
}
