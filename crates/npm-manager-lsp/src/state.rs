use dashmap::DashMap;
use std::sync::RwLock;
use tower_lsp::lsp_types::Url;

use crate::types::{CachedVersionInfo, Config, DocumentState};

pub struct ServerState {
    pub documents: DashMap<Url, DocumentState>,
    pub registry_cache: DashMap<String, CachedVersionInfo>,
    pub config: RwLock<Config>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
            registry_cache: DashMap::new(),
            config: RwLock::new(Config::default()),
        }
    }

    pub fn config(&self) -> Config {
        self.config.read().unwrap().clone()
    }
}
