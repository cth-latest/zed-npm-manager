use std::collections::HashMap;

pub fn parse(content: &str) -> Option<HashMap<String, String>> {
    let json: serde_json::Value = serde_json::from_str(content).ok()?;
    let obj = json.as_object()?;

    let mut versions = HashMap::new();

    if let Some(packages) = obj.get("packages").and_then(|v| v.as_object()) {
        for (key, value) in packages {
            if let Some(name) = key.strip_prefix("node_modules/") {
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
