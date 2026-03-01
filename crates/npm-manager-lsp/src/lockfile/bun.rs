use std::collections::HashMap;

/// Parse bun.lock (Bun v2 text-based JSON format) and extract installed versions.
/// The format has a "packages" object with entries like:
///   "name": ["name@version", ...]
pub fn parse(content: &str) -> Option<HashMap<String, String>> {
    // bun.lock is a JSON-like format (JSONC with trailing commas allowed)
    // Try parsing as JSON first; if that fails, try stripping comments/trailing commas
    let json: serde_json::Value = serde_json::from_str(content)
        .or_else(|_| {
            // Strip trailing commas before } and ]
            let cleaned = content
                .lines()
                .map(|line| {
                    // Remove single-line comments
                    if let Some(idx) = line.find("//") {
                        &line[..idx]
                    } else {
                        line
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            serde_json::from_str(&cleaned)
        })
        .ok()?;

    let obj = json.as_object()?;
    let packages = obj.get("packages")?.as_object()?;

    let mut versions = HashMap::new();

    for (name, value) in packages {
        // Each entry is an array where first element is "name@version"
        if let Some(arr) = value.as_array() {
            if let Some(first) = arr.first().and_then(|v| v.as_str()) {
                // Parse "name@version" - handle scoped packages
                if let Some(ver) = extract_version_from_spec(first) {
                    versions.insert(name.clone(), ver);
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

fn extract_version_from_spec(spec: &str) -> Option<String> {
    // Handle "@scope/name@version" and "name@version"
    let at_pos = if spec.starts_with('@') {
        spec[1..].find('@').map(|p| p + 1)
    } else {
        spec.rfind('@')
    };

    at_pos.map(|pos| spec[pos + 1..].to_string())
}
