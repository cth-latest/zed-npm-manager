use serde::Deserialize;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    PackageJson,
    PnpmWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    Dependencies,
    DevDependencies,
    PeerDependencies,
    OptionalDependencies,
    BundledDependencies,
    Catalog,
    NamedCatalog(String),
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dependencies => write!(f, "dependencies"),
            Self::DevDependencies => write!(f, "devDependencies"),
            Self::PeerDependencies => write!(f, "peerDependencies"),
            Self::OptionalDependencies => write!(f, "optionalDependencies"),
            Self::BundledDependencies => write!(f, "bundledDependencies"),
            Self::Catalog => write!(f, "catalog"),
            Self::NamedCatalog(name) => write!(f, "catalogs.{name}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DependencyEntry {
    pub name: String,
    pub raw_version: String,
    pub clean_version: String,
    pub line: u32,
    pub col_start: u32,
    pub col_end: u32,
    pub dep_type: DependencyType,
    pub status: VersionStatus,
    pub installed_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VersionStatus {
    Loading,
    UpToDate,
    Outdated { latest: String },
    Invalid { latest: String },
    NotFound,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct CachedVersionInfo {
    pub latest_version: String,
    pub versions: Vec<String>,
    pub fetched_at: Instant,
}

#[derive(Debug, Clone)]
pub struct DocumentState {
    pub content: String,
    pub dependencies: Vec<DependencyEntry>,
    pub file_type: FileType,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub stable_only: bool,
    #[serde(default = "default_true")]
    pub show_installed_version: bool,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,
}

fn default_true() -> bool {
    true
}

fn default_cache_ttl() -> u64 {
    300
}

impl Default for Config {
    fn default() -> Self {
        Self {
            stable_only: false,
            show_installed_version: true,
            cache_ttl_seconds: 300,
        }
    }
}

/// npm registry response (abbreviated)
#[derive(Debug, Deserialize)]
pub struct NpmRegistryResponse {
    #[serde(default)]
    pub versions: HashMap<String, serde_json::Value>,
    #[serde(default, rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
}

/// Clean version specifier prefixes from a version string
pub fn clean_version(version: &str) -> &str {
    let v = version.trim();
    // Strip leading >=, <=, >, <, =, ^, ~
    let v = v.strip_prefix(">=").unwrap_or(v);
    let v = v.strip_prefix("<=").unwrap_or(v);
    let v = v.strip_prefix('>').unwrap_or(v);
    let v = v.strip_prefix('<').unwrap_or(v);
    let v = v.strip_prefix('=').unwrap_or(v);
    let v = v.strip_prefix('^').unwrap_or(v);
    let v = v.strip_prefix('~').unwrap_or(v);
    v.trim()
}
