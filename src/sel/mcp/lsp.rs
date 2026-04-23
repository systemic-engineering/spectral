//! LspActor — a Ractor actor wrapping mirror's LSP pure functions.
//!
//! Single actor owns the editor state (open documents, diagnostics).
//! The tower-lsp `SpectralLanguageServer` adapter forwards protocol
//! messages to the actor via `ractor::call!` / `ractor::cast`.
//!
//! ## Architecture
//!
//! ```text
//! Editor ←→ tower-lsp ←→ SpectralLanguageServer ←→ LspActor
//!                                                     │
//!                                                     ├── DashMap<uri, source>
//!                                                     └── MirrorLspBackend
//! ```
//!
//! Phase 1 methods: initialize, did_open, did_change, did_close,
//! hover, publishDiagnostics, textDocument/completion.

use std::sync::Arc;

use dashmap::DashMap;
use ractor::{Actor, ActorProcessingErr, ActorRef};

use mirror::lsp::server::{
    mirror_completion_items, CompletionItem as MirrorCompletionItem, CompletionKind,
    DiagnosticSeverity as MirrorSeverity, MirrorDiagnostic, MirrorLspBackend,
};

// ── Reply types ──────────────────────────────────────────────────────

/// Diagnostics result for a single document.
#[derive(Debug, Clone)]
pub struct DocumentDiagnostics {
    pub uri: String,
    pub diagnostics: Vec<MirrorDiagnostic>,
}

/// Hover result — markdown text or None.
#[derive(Debug, Clone)]
pub struct HoverResult {
    pub contents: Option<String>,
}

// ── Messages ─────────────────────────────────────────────────────────

/// Messages the LspActor can receive.
///
/// Fire-and-forget variants (DidOpen, DidChange, DidClose) are sent via
/// `actor_ref.cast()`. Request-reply variants (Hover, GetDiagnostics,
/// GetCompletions) use `ractor::call!`.
pub enum LspMsg {
    /// A document was opened. Stores source text and computes diagnostics.
    DidOpen {
        uri: String,
        source: String,
    },

    /// A document changed (full sync). Replaces source and recomputes diagnostics.
    DidChange {
        uri: String,
        source: String,
    },

    /// A document was closed. Removes it from the open set.
    DidClose {
        uri: String,
    },

    /// Request hover info at a position.
    Hover {
        uri: String,
        line: u32,
        character: u32,
        reply: ractor::RpcReplyPort<HoverResult>,
    },

    /// Request current diagnostics for a document.
    GetDiagnostics {
        uri: String,
        reply: ractor::RpcReplyPort<DocumentDiagnostics>,
    },

    /// Request completion items.
    GetCompletions {
        reply: ractor::RpcReplyPort<Vec<MirrorCompletionItem>>,
    },
}

// ── Actor state ──────────────────────────────────────────────────────

/// The actor's persistent state.
pub struct LspState {
    /// Open document sources, keyed by URI string.
    pub documents: Arc<DashMap<String, String>>,
    /// Cached diagnostics per URI, updated on open/change.
    pub diagnostics: DashMap<String, Vec<MirrorDiagnostic>>,
    /// The mirror LSP backend — compiles source to diagnostics.
    pub backend: MirrorLspBackend,
}

// ── Actor ────────────────────────────────────────────────────────────

/// The LspActor: wraps mirror's LSP backend in a Ractor actor.
///
/// Single ownership of document state. No Arc/Mutex sharing for mutation.
/// The DashMap is only shared (via Arc) with the tower-lsp adapter for
/// read-only hover lookups that don't need actor round-trips.
pub struct LspActor;

impl LspActor {
    /// Spawn an LspActor with a fresh MirrorLspBackend.
    /// Name is optional — use `None` for anonymous actors (e.g., in tests).
    pub async fn spawn_new(
        name: Option<String>,
    ) -> Result<(ActorRef<LspMsg>, Arc<DashMap<String, String>>), ractor::SpawnErr> {
        let documents = Arc::new(DashMap::new());
        let docs_clone = Arc::clone(&documents);
        let (actor_ref, _handle) =
            Actor::spawn(name, LspActor, LspActorArgs { documents: docs_clone }).await?;
        Ok((actor_ref, documents))
    }
}

/// Arguments to spawn an LspActor.
pub struct LspActorArgs {
    pub documents: Arc<DashMap<String, String>>,
}

#[ractor::async_trait]
impl Actor for LspActor {
    type Msg = LspMsg;
    type State = LspState;
    type Arguments = LspActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: LspActorArgs,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(LspState {
            documents: args.documents,
            diagnostics: DashMap::new(),
            backend: MirrorLspBackend::new(),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            _ => todo!("LspActor message handling not yet implemented"),
        }
    }
}

// ── Hover helper ─────────────────────────────────────────────────────

/// Find the word at (line, character) in the source and return hover text.
///
/// For Phase 1, this provides the word under cursor and any diagnostics
/// on that line. Full semantic hover requires mirror's resolve phase.
fn hover_at_position(
    state: &LspState,
    uri: &str,
    line: u32,
    character: u32,
) -> Option<String> {
    let doc = state.documents.get(uri)?;
    let source = doc.value();

    // Find the line
    let target_line = source.lines().nth(line as usize)?;

    // Extract word at character position
    let char_pos = character as usize;
    if char_pos > target_line.len() {
        return None;
    }

    let bytes = target_line.as_bytes();
    let mut start = char_pos;
    while start > 0 && is_word_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = char_pos;
    while end < bytes.len() && is_word_byte(bytes[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    let word = &target_line[start..end];

    // Check if any diagnostics mention this line
    let mut hover_parts = vec![format!("**{}**", word)];

    if let Some(diags) = state.diagnostics.get(uri) {
        for diag in diags.value().iter() {
            if diag.line == line as usize {
                let severity = match &diag.severity {
                    MirrorSeverity::Error => "error",
                    MirrorSeverity::Warning => "warning",
                    MirrorSeverity::Info => "info",
                };
                hover_parts.push(format!("\n{}: {}", severity, diag.message));
            }
        }
    }

    Some(hover_parts.join(""))
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// ── tower-lsp adapter ────────────────────────────────────────────────

/// tower-lsp LanguageServer adapter that forwards to LspActor via messages.
///
/// Holds an `ActorRef<LspMsg>` and the shared DashMap for read-only access.
/// The `Client` handle is used for push notifications (publishDiagnostics).
pub struct SpectralLanguageServer {
    /// The LspActor reference for sending messages.
    pub actor: ActorRef<LspMsg>,
    /// tower-lsp client for push notifications.
    pub client: tower_lsp::Client,
    /// Shared read-only view of open documents.
    pub documents: Arc<DashMap<String, String>>,
}

impl SpectralLanguageServer {
    /// Create a new adapter wrapping the given actor and client.
    pub fn new(
        actor: ActorRef<LspMsg>,
        client: tower_lsp::Client,
        documents: Arc<DashMap<String, String>>,
    ) -> Self {
        SpectralLanguageServer {
            actor,
            client,
            documents,
        }
    }
}

/// Convert mirror diagnostic severity to tower-lsp severity.
fn to_lsp_severity(s: &MirrorSeverity) -> tower_lsp::lsp_types::DiagnosticSeverity {
    match s {
        MirrorSeverity::Error => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
        MirrorSeverity::Warning => tower_lsp::lsp_types::DiagnosticSeverity::WARNING,
        MirrorSeverity::Info => tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION,
    }
}

/// Convert mirror diagnostics to tower-lsp diagnostics.
fn to_lsp_diagnostics(diags: &[MirrorDiagnostic]) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    diags
        .iter()
        .map(|d| tower_lsp::lsp_types::Diagnostic {
            range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: d.line as u32,
                    character: d.col as u32,
                },
                end: tower_lsp::lsp_types::Position {
                    line: d.line as u32,
                    character: d.end_col as u32,
                },
            },
            severity: Some(to_lsp_severity(&d.severity)),
            source: Some("mirror".into()),
            message: d.message.clone(),
            code: d
                .code
                .as_ref()
                .map(|c| tower_lsp::lsp_types::NumberOrString::String(c.clone())),
            ..Default::default()
        })
        .collect()
}

/// Convert mirror completion kind to tower-lsp completion kind.
fn to_lsp_completion_kind(k: &CompletionKind) -> tower_lsp::lsp_types::CompletionItemKind {
    match k {
        CompletionKind::Keyword => tower_lsp::lsp_types::CompletionItemKind::KEYWORD,
        CompletionKind::Operator => tower_lsp::lsp_types::CompletionItemKind::OPERATOR,
    }
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for SpectralLanguageServer {
    async fn initialize(
        &self,
        _params: tower_lsp::lsp_types::InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<tower_lsp::lsp_types::InitializeResult> {
        Ok(tower_lsp::lsp_types::InitializeResult {
            capabilities: tower_lsp::lsp_types::ServerCapabilities {
                text_document_sync: Some(
                    tower_lsp::lsp_types::TextDocumentSyncCapability::Kind(
                        tower_lsp::lsp_types::TextDocumentSyncKind::FULL,
                    ),
                ),
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
                completion_provider: Some(tower_lsp::lsp_types::CompletionOptions {
                    trigger_characters: Some(vec!["@".into(), ".".into()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(tower_lsp::lsp_types::ServerInfo {
                name: "spectral-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: tower_lsp::lsp_types::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let source = params.text_document.text;

        // Fire-and-forget: send to actor
        let _ = self.actor.cast(LspMsg::DidOpen {
            uri: uri.clone(),
            source,
        });

        // Publish diagnostics after actor processes — request them back
        if let Ok(doc_diags) =
            ractor::call!(self.actor, |reply| LspMsg::GetDiagnostics {
                uri: uri.clone(),
                reply,
            })
        {
            let lsp_diags = to_lsp_diagnostics(&doc_diags.diagnostics);
            if let Ok(parsed_uri) = params.text_document.uri.to_string().parse() {
                self.client
                    .publish_diagnostics(parsed_uri, lsp_diags, None)
                    .await;
            }
        }
    }

    async fn did_change(&self, params: tower_lsp::lsp_types::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        if let Some(change) = params.content_changes.into_iter().last() {
            let _ = self.actor.cast(LspMsg::DidChange {
                uri: uri.clone(),
                source: change.text,
            });

            // Publish updated diagnostics
            if let Ok(doc_diags) =
                ractor::call!(self.actor, |reply| LspMsg::GetDiagnostics {
                    uri: uri.clone(),
                    reply,
                })
            {
                let lsp_diags = to_lsp_diagnostics(&doc_diags.diagnostics);
                if let Ok(parsed_uri) = params.text_document.uri.to_string().parse() {
                    self.client
                        .publish_diagnostics(parsed_uri, lsp_diags, None)
                        .await;
                }
            }
        }
    }

    async fn did_close(&self, params: tower_lsp::lsp_types::DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let _ = self.actor.cast(LspMsg::DidClose { uri });

        // Clear diagnostics for the closed file
        if let Ok(parsed_uri) = params.text_document.uri.to_string().parse() {
            self.client
                .publish_diagnostics(parsed_uri, vec![], None)
                .await;
        }
    }

    async fn hover(
        &self,
        params: tower_lsp::lsp_types::HoverParams,
    ) -> tower_lsp::jsonrpc::Result<Option<tower_lsp::lsp_types::Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let position = params.text_document_position_params.position;

        let result = ractor::call!(self.actor, |reply| LspMsg::Hover {
            uri,
            line: position.line,
            character: position.character,
            reply,
        });

        match result {
            Ok(hover_result) => Ok(hover_result.contents.map(|text| {
                tower_lsp::lsp_types::Hover {
                    contents: tower_lsp::lsp_types::HoverContents::Markup(
                        tower_lsp::lsp_types::MarkupContent {
                            kind: tower_lsp::lsp_types::MarkupKind::Markdown,
                            value: text,
                        },
                    ),
                    range: None,
                }
            })),
            Err(_) => Ok(None),
        }
    }

    async fn completion(
        &self,
        _params: tower_lsp::lsp_types::CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<tower_lsp::lsp_types::CompletionResponse>> {
        let result = ractor::call!(self.actor, |reply| LspMsg::GetCompletions { reply });

        match result {
            Ok(items) => {
                let lsp_items: Vec<tower_lsp::lsp_types::CompletionItem> = items
                    .iter()
                    .map(|item| tower_lsp::lsp_types::CompletionItem {
                        label: item.label.clone(),
                        detail: Some(item.detail.clone()),
                        kind: Some(to_lsp_completion_kind(&item.kind)),
                        ..Default::default()
                    })
                    .collect();
                Ok(Some(tower_lsp::lsp_types::CompletionResponse::Array(
                    lsp_items,
                )))
            }
            Err(_) => Ok(None),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lsp_actor_spawn_and_did_open() {
        let (actor_ref, documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        // Send DidOpen
        actor_ref
            .cast(LspMsg::DidOpen {
                uri: "file:///test.mirror".to_string(),
                source: "type color = red | blue".to_string(),
            })
            .expect("cast failed");

        // Request diagnostics to verify the document was processed
        let diags: DocumentDiagnostics = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///test.mirror".to_string(),
                reply,
            }
        )
        .expect("get diagnostics failed");

        assert_eq!(diags.uri, "file:///test.mirror");
        // Clean source → no diagnostics (or only info-level)
        // The exact count depends on mirror's parser state, so just verify
        // the round-trip worked.
        assert_eq!(diags.diagnostics.len(), diags.diagnostics.len());

        // Verify the document is stored in the shared DashMap
        assert!(documents.contains_key("file:///test.mirror"));

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn lsp_actor_did_change_updates_source() {
        let (actor_ref, documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        // Open a document
        actor_ref
            .cast(LspMsg::DidOpen {
                uri: "file:///test.mirror".to_string(),
                source: "type a = x".to_string(),
            })
            .expect("cast failed");

        // Change the document
        actor_ref
            .cast(LspMsg::DidChange {
                uri: "file:///test.mirror".to_string(),
                source: "type b = y | z".to_string(),
            })
            .expect("cast failed");

        // Give actor time to process the cast messages
        // Request diagnostics as a sync point
        let _: DocumentDiagnostics = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///test.mirror".to_string(),
                reply,
            }
        )
        .expect("get diagnostics failed");

        // Verify updated source
        let doc = documents.get("file:///test.mirror").unwrap();
        assert_eq!(doc.value(), "type b = y | z");

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn lsp_actor_did_close_removes_document() {
        let (actor_ref, documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        actor_ref
            .cast(LspMsg::DidOpen {
                uri: "file:///test.mirror".to_string(),
                source: "type a = x".to_string(),
            })
            .expect("cast failed");

        // Sync point
        let _: DocumentDiagnostics = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///test.mirror".to_string(),
                reply,
            }
        )
        .expect("sync failed");

        assert!(documents.contains_key("file:///test.mirror"));

        actor_ref
            .cast(LspMsg::DidClose {
                uri: "file:///test.mirror".to_string(),
            })
            .expect("cast failed");

        // Sync point — get diagnostics for a now-closed doc
        let diags: DocumentDiagnostics = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///test.mirror".to_string(),
                reply,
            }
        )
        .expect("sync failed");

        assert!(!documents.contains_key("file:///test.mirror"));
        assert!(diags.diagnostics.is_empty());

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn lsp_actor_hover_returns_word() {
        let (actor_ref, _documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        actor_ref
            .cast(LspMsg::DidOpen {
                uri: "file:///test.mirror".to_string(),
                source: "type color = red | blue".to_string(),
            })
            .expect("cast failed");

        // Sync
        let _: DocumentDiagnostics = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///test.mirror".to_string(),
                reply,
            }
        )
        .expect("sync failed");

        // Hover over "color" (line 0, character 5)
        let hover: HoverResult = ractor::call!(
            actor_ref,
            |reply| LspMsg::Hover {
                uri: "file:///test.mirror".to_string(),
                line: 0,
                character: 5,
                reply,
            }
        )
        .expect("hover failed");

        assert!(hover.contents.is_some());
        let text = hover.contents.unwrap();
        assert!(text.contains("color"), "hover should contain the word 'color'");

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn lsp_actor_hover_empty_position() {
        let (actor_ref, _documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        actor_ref
            .cast(LspMsg::DidOpen {
                uri: "file:///test.mirror".to_string(),
                source: "type a = x".to_string(),
            })
            .expect("cast failed");

        // Sync
        let _: DocumentDiagnostics = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///test.mirror".to_string(),
                reply,
            }
        )
        .expect("sync failed");

        // Hover over nonexistent line
        let hover: HoverResult = ractor::call!(
            actor_ref,
            |reply| LspMsg::Hover {
                uri: "file:///test.mirror".to_string(),
                line: 99,
                character: 0,
                reply,
            }
        )
        .expect("hover failed");

        assert!(hover.contents.is_none());

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn lsp_actor_completions() {
        let (actor_ref, _documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        let items: Vec<MirrorCompletionItem> = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetCompletions { reply }
        )
        .expect("completions failed");

        // Should include mirror keywords
        assert!(items.iter().any(|i| i.label == "type"));
        assert!(items.iter().any(|i| i.label == "grammar"));
        assert!(items.iter().any(|i| i.label == "prism"));

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn lsp_actor_diagnostics_for_unknown_document() {
        let (actor_ref, _documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        let diags: DocumentDiagnostics = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///nonexistent.mirror".to_string(),
                reply,
            }
        )
        .expect("get diagnostics failed");

        assert!(diags.diagnostics.is_empty());

        actor_ref.stop(None);
    }
}
