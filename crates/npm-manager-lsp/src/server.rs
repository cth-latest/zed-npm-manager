use std::sync::Arc;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::parser;
use crate::registry;
use crate::lockfile;
use crate::state::ServerState;
use crate::types::{Config, FileType, VersionStatus};

pub struct Backend {
    pub client: Client,
    pub state: Arc<ServerState>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(ServerState::new()),
        }
    }

    fn file_type_for_uri(&self, uri: &Url) -> Option<FileType> {
        let path = uri.path();
        if path.ends_with("/package.json") || path == "package.json" {
            Some(FileType::PackageJson)
        } else if path.ends_with("/pnpm-workspace.yaml")
            || path.ends_with("/pnpm-workspace.yml")
            || path == "pnpm-workspace.yaml"
            || path == "pnpm-workspace.yml"
        {
            Some(FileType::PnpmWorkspace)
        } else {
            None
        }
    }

    async fn process_document(&self, uri: Url, text: String) {
        let file_type = match self.file_type_for_uri(&uri) {
            Some(ft) => ft,
            None => return,
        };

        let mut dependencies = match file_type {
            FileType::PackageJson => parser::package_json::parse(&text),
            FileType::PnpmWorkspace => parser::pnpm_workspace::parse(&text),
        };

        if dependencies.is_empty() {
            self.state.documents.remove(&uri);
            self.client
                .publish_diagnostics(uri, vec![], None)
                .await;
            return;
        }

        // Resolve installed versions from lock files
        let config = self.state.config();
        if config.show_installed_version {
            if let Ok(path) = uri.to_file_path() {
                if let Some(dir) = path.parent() {
                    let installed = lockfile::resolve_installed_versions(dir);
                    for dep in &mut dependencies {
                        if let Some(ver) = installed.get(&dep.name) {
                            dep.installed_version = Some(ver.clone());
                        }
                    }
                }
            }
        }

        // Fetch registry info for all dependencies
        registry::fetch_all(&self.state, &mut dependencies).await;

        // Store document state
        let doc_state = crate::types::DocumentState {
            content: text,
            dependencies: dependencies.clone(),
            file_type,
        };
        self.state.documents.insert(uri.clone(), doc_state);

        // Publish diagnostics
        let diagnostics = build_diagnostics(&dependencies);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Parse initialization options for config
        if let Some(opts) = params.initialization_options {
            if let Ok(config) = serde_json::from_value::<Config>(opts) {
                *self.state.config.write().unwrap() = config;
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                inlay_hint_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("npm-manager-lsp initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.process_document(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.process_document(uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.state.documents.remove(&uri);
        self.client
            .publish_diagnostics(uri, vec![], None)
            .await;
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = &params.text_document.uri;
        let doc = match self.state.documents.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let config = self.state.config();
        let hints: Vec<InlayHint> = doc
            .dependencies
            .iter()
            .filter(|dep| {
                let line = dep.line;
                line >= params.range.start.line && line <= params.range.end.line
            })
            .map(|dep| {
                let label = format_inlay_label(dep, config.show_installed_version);
                InlayHint {
                    position: Position::new(dep.line, dep.col_end),
                    label: InlayHintLabel::String(label),
                    kind: None,
                    text_edits: None,
                    tooltip: None,
                    padding_left: Some(true),
                    padding_right: None,
                    data: None,
                }
            })
            .collect();

        Ok(Some(hints))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let doc = match self.state.documents.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let dep = doc.dependencies.iter().find(|d| {
            d.line == pos.line && pos.character >= d.col_start && pos.character <= d.col_end
        });

        let dep = match dep {
            Some(d) => d,
            None => return Ok(None),
        };

        let mut lines = vec![format!("**{}**", dep.name)];
        lines.push(format!("- Specified: `{}`", dep.raw_version));

        match &dep.status {
            VersionStatus::UpToDate => lines.push("- Status: Up to date".to_string()),
            VersionStatus::Outdated { latest } => {
                lines.push(format!("- Latest: `{latest}`"));
                lines.push("- Status: Update available".to_string());
            }
            VersionStatus::Invalid { latest } => {
                lines.push(format!("- Latest: `{latest}`"));
                lines.push("- Status: Version not found in registry".to_string());
            }
            VersionStatus::NotFound => {
                lines.push("- Status: Package not found".to_string());
            }
            VersionStatus::Error(e) => {
                lines.push(format!("- Status: Error — {e}"));
            }
            VersionStatus::Loading => {
                lines.push("- Status: Loading...".to_string());
            }
        }

        if let Some(installed) = &dep.installed_version {
            lines.push(format!("- Installed: `{installed}`"));
        }

        lines.push(format!("- Section: `{}`", dep.dep_type));
        lines.push(format!(
            "- [npmjs.com](https://www.npmjs.com/package/{})",
            dep.name
        ));

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: lines.join("\n"),
            }),
            range: Some(Range::new(
                Position::new(dep.line, dep.col_start),
                Position::new(dep.line, dep.col_end),
            )),
        }))
    }
}

fn format_inlay_label(dep: &crate::types::DependencyEntry, show_installed: bool) -> String {
    let status = match &dep.status {
        VersionStatus::Loading => "...".to_string(),
        VersionStatus::UpToDate => "\u{2713}".to_string(), // ✓
        VersionStatus::Outdated { latest } => format!("\u{2191} {latest}"), // ↑
        VersionStatus::Invalid { latest } => format!("\u{26a0} {latest}"), // ⚠
        VersionStatus::NotFound => "\u{2717}".to_string(),  // ✗
        VersionStatus::Error(_) => "\u{2717}".to_string(),  // ✗
    };

    if show_installed {
        if let Some(installed) = &dep.installed_version {
            if dep.clean_version != *installed {
                return format!("{status}  (installed: {installed})");
            }
        }
    }

    status
}

fn build_diagnostics(dependencies: &[crate::types::DependencyEntry]) -> Vec<Diagnostic> {
    dependencies
        .iter()
        .filter_map(|dep| {
            let (message, severity) = match &dep.status {
                VersionStatus::Outdated { latest } => (
                    format!("{}: {} -> {latest}", dep.name, dep.clean_version),
                    DiagnosticSeverity::INFORMATION,
                ),
                VersionStatus::Invalid { latest } => (
                    format!(
                        "{}: version {} not found (latest: {latest})",
                        dep.name, dep.clean_version
                    ),
                    DiagnosticSeverity::WARNING,
                ),
                VersionStatus::NotFound => (
                    format!("{}: package not found in npm registry", dep.name),
                    DiagnosticSeverity::ERROR,
                ),
                VersionStatus::Error(e) => (
                    format!("{}: {e}", dep.name),
                    DiagnosticSeverity::WARNING,
                ),
                _ => return None,
            };

            Some(Diagnostic {
                range: Range::new(
                    Position::new(dep.line, dep.col_start),
                    Position::new(dep.line, dep.col_end),
                ),
                severity: Some(severity),
                source: Some("npm-manager".to_string()),
                message,
                ..Default::default()
            })
        })
        .collect()
}
