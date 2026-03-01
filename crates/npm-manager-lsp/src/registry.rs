use std::sync::Arc;
use std::time::{Duration, Instant};

use regex::Regex;
use tokio::sync::Semaphore;

use crate::state::ServerState;
use crate::types::{CachedVersionInfo, DependencyEntry, NpmRegistryResponse, VersionStatus};

const NPM_REGISTRY_URL: &str = "https://registry.npmjs.org";
const MAX_CONCURRENT_FETCHES: usize = 10;

static PRERELEASE_PATTERN: &str =
    r"(?i)(?:alpha|beta|rc|dev|post|preview|snapshot|canary|insider|insiders|internal|development)";

/// Fetch version info from npm registry for all dependencies, updating their status in place.
pub async fn fetch_all(state: &ServerState, dependencies: &mut [DependencyEntry]) {
    let config = state.config();
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_FETCHES));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let mut handles = Vec::new();

    for i in 0..dependencies.len() {
        let name = dependencies[i].name.clone();
        let cache_ttl = Duration::from_secs(config.cache_ttl_seconds);

        // Check cache first
        if let Some(cached) = state.registry_cache.get(&name) {
            if cached.fetched_at.elapsed() < cache_ttl {
                dependencies[i].status =
                    compute_status(&dependencies[i].clean_version, &cached, config.stable_only);
                continue;
            }
        }

        let sem = semaphore.clone();
        let client = client.clone();
        let stable_only = config.stable_only;
        let clean_version = dependencies[i].clean_version.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let result = fetch_package(&client, &name).await;
            (i, name, clean_version, result, stable_only)
        });
        handles.push(handle);
    }

    for handle in handles {
        if let Ok((idx, name, clean_version, result, stable_only)) = handle.await {
            match result {
                Ok(cached) => {
                    dependencies[idx].status =
                        compute_status(&clean_version, &cached, stable_only);
                    state.registry_cache.insert(name, cached);
                }
                Err(FetchError::NotFound) => {
                    dependencies[idx].status = VersionStatus::NotFound;
                }
                Err(FetchError::Network(e)) => {
                    dependencies[idx].status = VersionStatus::Error(e);
                }
            }
        }
    }
}

fn compute_status(
    clean_version: &str,
    cached: &CachedVersionInfo,
    stable_only: bool,
) -> VersionStatus {
    let latest = if stable_only {
        filter_stable_latest(&cached.versions).unwrap_or(&cached.latest_version)
    } else {
        &cached.latest_version
    };

    let version_exists = cached.versions.iter().any(|v| v == clean_version);

    if !version_exists {
        VersionStatus::Invalid {
            latest: latest.clone(),
        }
    } else if clean_version == latest {
        VersionStatus::UpToDate
    } else {
        VersionStatus::Outdated {
            latest: latest.clone(),
        }
    }
}

fn filter_stable_latest(versions: &[String]) -> Option<&String> {
    let re = Regex::new(PRERELEASE_PATTERN).ok()?;
    versions.iter().rev().find(|v| !re.is_match(v))
}

enum FetchError {
    NotFound,
    Network(String),
}

async fn fetch_package(
    client: &reqwest::Client,
    name: &str,
) -> std::result::Result<CachedVersionInfo, FetchError> {
    let url = format!("{NPM_REGISTRY_URL}/{name}");

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| FetchError::Network(e.to_string()))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(FetchError::NotFound);
    }

    if !response.status().is_success() {
        return Err(FetchError::Network(format!(
            "HTTP {}",
            response.status()
        )));
    }

    let data: NpmRegistryResponse = response
        .json()
        .await
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let versions: Vec<String> = data.versions.keys().cloned().collect();
    let latest_version = data
        .dist_tags
        .get("latest")
        .cloned()
        .or_else(|| versions.last().cloned())
        .unwrap_or_default();

    Ok(CachedVersionInfo {
        latest_version,
        versions,
        fetched_at: Instant::now(),
    })
}
