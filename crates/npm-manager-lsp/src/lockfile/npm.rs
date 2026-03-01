use std::collections::HashMap;

/// Parse package-lock.json and extract installed versions.
/// Supports lockfileVersion 2/3 (packages key) and v1 (dependencies key).
pub fn parse(content: &str) -> Option<HashMap<String, String>> {
    let json: serde_json::Value = serde_json::from_str(content).ok()?;
    let obj = json.as_object()?;

    let mut versions = HashMap::new();

    // lockfileVersion 2/3: packages with "node_modules/name" keys
    if let Some(packages) = obj.get("packages").and_then(|v| v.as_object()) {
        for (key, value) in packages {
            // Keys are like "node_modules/express" or "node_modules/@scope/pkg"
            if let Some(name) = key.strip_prefix("node_modules/") {
                // Skip nested node_modules (transitive deps)
                if name.contains("node_modules/") {
                    continue;
                }
                if let Some(ver) = value.get("version").and_then(|v| v.as_str()) {
                    versions.insert(name.to_string(), ver.to_string());
                }
            }
        }
        if !versions.is_empty() {
            return Some(versions);
        }
    }

    // lockfileVersion 1: flat dependencies object
    if let Some(deps) = obj.get("dependencies").and_then(|v| v.as_object()) {
        for (name, value) in deps {
            if let Some(ver) = value.get("version").and_then(|v| v.as_str()) {
                versions.insert(name.clone(), ver.to_string());
            }
        }
        if !versions.is_empty() {
            return Some(versions);
        }
    }

    None
}
