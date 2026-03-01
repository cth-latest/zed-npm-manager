use regex::Regex;

use crate::types::{clean_version, DependencyEntry, DependencyType, VersionStatus};

/// Sections in package.json that contain dependencies as { "name": "version" }
const DEP_SECTIONS: &[(&str, DependencyType)] = &[
    ("dependencies", DependencyType::Dependencies),
    ("devDependencies", DependencyType::DevDependencies),
    ("peerDependencies", DependencyType::PeerDependencies),
    ("optionalDependencies", DependencyType::OptionalDependencies),
    ("bundledDependencies", DependencyType::BundledDependencies),
];

pub fn parse(text: &str) -> Vec<DependencyEntry> {
    let json: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let obj = match json.as_object() {
        Some(o) => o,
        None => return vec![],
    };

    let mut all_deps: Vec<(String, String, DependencyType)> = Vec::new();

    // Standard dependency sections
    for (section, dep_type) in DEP_SECTIONS {
        if let Some(deps) = obj.get(*section).and_then(|v| v.as_object()) {
            for (name, version) in deps {
                if let Some(v) = version.as_str() {
                    all_deps.push((name.clone(), v.to_string(), dep_type.clone()));
                }
            }
        }
    }

    // Bun catalogs: root-level "catalog" object
    if let Some(catalog) = obj.get("catalog").and_then(|v| v.as_object()) {
        for (name, version) in catalog {
            if let Some(v) = version.as_str() {
                all_deps.push((name.clone(), v.to_string(), DependencyType::Catalog));
            }
        }
    }

    // Bun catalogs: root-level "catalogs" object with named sub-catalogs
    if let Some(catalogs) = obj.get("catalogs").and_then(|v| v.as_object()) {
        for (catalog_name, catalog_obj) in catalogs {
            if let Some(deps) = catalog_obj.as_object() {
                for (name, version) in deps {
                    if let Some(v) = version.as_str() {
                        all_deps.push((
                            name.clone(),
                            v.to_string(),
                            DependencyType::NamedCatalog(catalog_name.clone()),
                        ));
                    }
                }
            }
        }
    }

    // Find line positions for each dependency via regex
    let mut entries = Vec::with_capacity(all_deps.len());
    for (name, raw_version, dep_type) in all_deps {
        let cleaned = clean_version(&raw_version).to_string();
        if cleaned.is_empty() {
            continue;
        }

        if let Some((line, col_start, col_end)) = find_dep_position(text, &name, &raw_version) {
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

/// Find the line and column span of a dependency entry like `"name": "^1.2.3"` in the text.
fn find_dep_position(text: &str, name: &str, version: &str) -> Option<(u32, u32, u32)> {
    let pattern = format!(
        r#""{}"\s*:\s*"{}""#,
        regex::escape(name),
        regex::escape(version)
    );
    let re = match Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return None,
    };

    let m = re.find(text)?;
    let start_byte = m.start();

    // Convert byte offset to line/col
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
