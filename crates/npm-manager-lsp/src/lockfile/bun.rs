use std::collections::HashMap;

pub fn parse(content: &str) -> Option<HashMap<String, String>> {
    let json: serde_json::Value = serde_json::from_str(content)
        .or_else(|_| {
            let cleaned = content
                .lines()
                .map(|line| {
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
        if let Some(arr) = value.as_array() {
            if let Some(first) = arr.first().and_then(|v| v.as_str()) {
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
    let at_pos = if spec.starts_with('@') {
        spec[1..].find('@').map(|p| p + 1)
    } else {
        spec.rfind('@')
    };

    at_pos.map(|pos| spec[pos + 1..].to_string())
}
