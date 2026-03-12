use std::collections::HashMap;

use regex::Regex;

pub fn parse(content: &str) -> Option<HashMap<String, String>> {
    let mut versions = HashMap::new();
    let mut current_packages: Vec<String> = Vec::new();

    let header_re = Regex::new(r#"^["']?([^,\n]+?)["']?(?:,\s*["']?[^,\n]+?["']?)*:\s*$"#).ok()?;
    let version_re = Regex::new(r#"^\s+version:?\s+"([^"]+)""#).ok()?;
    let name_re = Regex::new(r#"^(@?[^@\s]+)@"#).ok()?;

    for line in content.lines() {
        if let Some(caps) = header_re.captures(line) {
            current_packages.clear();
            let spec = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Some(name_caps) = name_re.captures(spec) {
                let name = name_caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if !name.is_empty() {
                    current_packages.push(name.to_string());
                }
            }
        } else if let Some(caps) = version_re.captures(line) {
            let ver = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            for pkg in &current_packages {
                versions.entry(pkg.clone()).or_insert_with(|| ver.to_string());
            }
            current_packages.clear();
        }
    }

    if versions.is_empty() {
        None
    } else {
        Some(versions)
    }
}
