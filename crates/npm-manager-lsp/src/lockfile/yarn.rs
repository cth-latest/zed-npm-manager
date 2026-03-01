use std::collections::HashMap;

use regex::Regex;

/// Parse yarn.lock and extract installed versions.
/// Handles both yarn v1 (custom format) and yarn v2+ (also custom but similar).
pub fn parse(content: &str) -> Option<HashMap<String, String>> {
    let mut versions = HashMap::new();
    let mut current_packages: Vec<String> = Vec::new();

    // Match header lines like: "express@^4.18.0":  or  express@^4.18.0, express@^4.17.0:
    let header_re = Regex::new(r#"^["']?([^,\n]+?)["']?(?:,\s*["']?[^,\n]+?["']?)*:\s*$"#).ok()?;
    // Match version line like:   version "4.18.2"  or   version: "4.18.2"
    let version_re = Regex::new(r#"^\s+version:?\s+"([^"]+)""#).ok()?;
    // Extract package name from spec like "express@^4.18.0"
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
