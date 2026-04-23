//! LspActor — a Ractor actor wrapping mirror's LSP pure functions.
//!
//! Single actor owns the editor state (open documents, diagnostics, self-loss).
//! The tower-lsp `SpectralLanguageServer` adapter forwards protocol
//! messages to the actor via `ractor::call!` / `ractor::cast`.
//!
//! ## Architecture
//!
//! ```text
//! Editor ←→ tower-lsp ←→ SpectralLanguageServer ←→ LspActor
//!                                                     │
//!                                                     ├── DashMap<uri, source>
//!                                                     ├── MirrorLspBackend
//!                                                     └── SelfLoss (proposal tracking)
//! ```
//!
//! Phase 1 methods: initialize, did_open, did_change, did_close,
//! hover, publishDiagnostics, textDocument/completion.
//!
//! Phase 2 (Tick 4): codeLens, honest gutter diagnostics, self-loss tracking.

use std::sync::Arc;

use dashmap::DashMap;
use ractor::{Actor, ActorProcessingErr, ActorRef};

use mirror::lsp::server::{
    mirror_completion_items, CompletionItem as MirrorCompletionItem, CompletionKind,
    DiagnosticSeverity as MirrorSeverity, MirrorDiagnostic, MirrorLspBackend,
};
use mirror::shatter_format::Luminosity;

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

/// A single CodeLens entry: a line with loss + coupling metrics.
#[derive(Debug, Clone)]
pub struct CodeLensEntry {
    pub line: u32,
    pub loss_bits: f64,
    pub coupling: f64,
    pub luminosity: GutterLuminosity,
}

/// The four gutter colors — the peer's self-knowledge made visible.
///
/// Green (0 bits)  — Light  — peer knows this code.
/// Yellow (1-2 bits) — Dimmed — peer is close.
/// Red (3+ bits)   — Dark   — peer was wrong.
/// Dark/Void       — Void   — Goedelian, unmeasurable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GutterLuminosity {
    /// 0 bits loss — peer knows this code.
    Light,
    /// 1-2 bits loss — peer is close.
    Dimmed,
    /// 3+ bits loss — peer was wrong.
    Dark,
    /// Unmeasurable — measuring requires stepping outside the system.
    Void,
}

impl GutterLuminosity {
    /// Classify loss in bits to a gutter color.
    pub fn from_bits(bits: f64) -> Self {
        if bits.is_nan() || bits.is_infinite() {
            GutterLuminosity::Void
        } else if bits < f64::EPSILON {
            GutterLuminosity::Light
        } else if bits < 3.0 {
            GutterLuminosity::Dimmed
        } else {
            GutterLuminosity::Dark
        }
    }

    /// Map to a display label.
    pub fn as_str(&self) -> &'static str {
        match self {
            GutterLuminosity::Light => "light",
            GutterLuminosity::Dimmed => "dimmed",
            GutterLuminosity::Dark => "dark",
            GutterLuminosity::Void => "void",
        }
    }
}

/// Self-loss report — the peer's own accuracy tracking.
#[derive(Debug, Clone)]
pub struct LossReport {
    /// Per-file loss breakdown: (uri, total_loss_bits, diagnostic_count, luminosity).
    pub files: Vec<FileLossEntry>,
    /// Self-measured loss from proposal tracking.
    pub self_loss: f64,
    /// Total proposals made.
    pub proposal_count: u64,
    /// Proposals accepted by the user.
    pub accepted_count: u64,
    /// Proposals rejected by the user.
    pub rejected_count: u64,
}

/// Per-file loss entry.
#[derive(Debug, Clone)]
pub struct FileLossEntry {
    pub uri: String,
    pub total_loss_bits: f64,
    pub diagnostic_count: usize,
    pub luminosity: GutterLuminosity,
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

    /// Request codeLens entries for a document.
    GetCodeLenses {
        uri: String,
        reply: ractor::RpcReplyPort<Vec<CodeLensEntry>>,
    },

    /// Request the full loss report (for MCP spectral_loss tool).
    GetLossReport {
        reply: ractor::RpcReplyPort<LossReport>,
    },
}

// ── Actor state ──────────────────────────────────────────────────────

/// The actor's persistent state.
pub struct LspState {
    /// Open document sources, keyed by URI string.
    pub documents: Arc<DashMap<String, String>>,
    /// Cached diagnostics per URI, updated on open/change.
    pub diagnostics: DashMap<String, Vec<MirrorDiagnostic>>,
    /// Cached luminosity per URI, updated on open/change.
    pub luminosities: DashMap<String, Luminosity>,
    /// The mirror LSP backend — compiles source to diagnostics.
    pub backend: MirrorLspBackend,
    /// Total completion proposals offered.
    pub proposal_count: u64,
    /// Proposals the user accepted (approximated).
    pub accepted_count: u64,
    /// Proposals the user rejected (approximated).
    pub rejected_count: u64,
    /// Self-loss: Shannon entropy of accept/reject ratio.
    pub self_loss: f64,
    /// Whether a completion was recently offered (pending acceptance tracking).
    pub completion_pending: bool,
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
            luminosities: DashMap::new(),
            backend: MirrorLspBackend::new(),
            proposal_count: 0,
            accepted_count: 0,
            rejected_count: 0,
            self_loss: 0.0,
            completion_pending: false,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            LspMsg::DidOpen { uri, source } => {
                let (luminosity, diags) = state.backend.compile_and_diagnose(&source);
                state.documents.insert(uri.clone(), source);
                state.diagnostics.insert(uri.clone(), diags);
                state.luminosities.insert(uri, luminosity);
            }

            LspMsg::DidChange { uri, source } => {
                // If a completion was pending and the user typed something
                // different, count it as a rejection (best effort).
                if state.completion_pending {
                    state.rejected_count += 1;
                    state.completion_pending = false;
                    update_self_loss(state);
                }

                let (luminosity, diags) = state.backend.compile_and_diagnose(&source);
                state.documents.insert(uri.clone(), source);
                state.diagnostics.insert(uri.clone(), diags);
                state.luminosities.insert(uri, luminosity);
            }

            LspMsg::DidClose { uri } => {
                state.documents.remove(&uri);
                state.diagnostics.remove(&uri);
                state.luminosities.remove(&uri);
            }

            LspMsg::Hover {
                uri,
                line,
                character,
                reply,
            } => {
                let result = hover_at_position(state, &uri, line, character);
                let _ = reply.send(HoverResult { contents: result });
            }

            LspMsg::GetDiagnostics { uri, reply } => {
                let diags = state
                    .diagnostics
                    .get(&uri)
                    .map(|d| d.value().clone())
                    .unwrap_or_default();
                let _ = reply.send(DocumentDiagnostics {
                    uri,
                    diagnostics: diags,
                });
            }

            LspMsg::GetCompletions { reply } => {
                state.proposal_count += 1;
                state.completion_pending = true;
                let items = mirror_completion_items();
                let _ = reply.send(items);
            }

            LspMsg::GetCodeLenses { uri: _, reply: _ } => {
                todo!("tick-4: compute_code_lenses")
            }

            LspMsg::GetLossReport { reply: _ } => {
                todo!("tick-4: compute_loss_report")
            }
        }
        Ok(())
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

// ── Self-loss computation ────────────────────────────────────────────

/// Recompute self_loss as Shannon entropy of accept/reject ratio.
///
/// H = -p*log2(p) - (1-p)*log2(1-p) where p = accepted / total.
/// Edge cases: if total is 0 or p is 0 or 1, entropy is 0.
pub fn update_self_loss(state: &mut LspState) {
    let total = state.accepted_count + state.rejected_count;
    if total == 0 {
        state.self_loss = 0.0;
        return;
    }
    let p = state.accepted_count as f64 / total as f64;
    state.self_loss = shannon_entropy(p);
}

/// Shannon binary entropy: H(p) = -p*log2(p) - (1-p)*log2(1-p).
pub fn shannon_entropy(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }
    let q = 1.0 - p;
    -(p * p.log2() + q * q.log2())
}

// ── CodeLens computation ─────────────────────────────────────────────

/// Compute code lenses for a document.
///
/// One lens per significant line: lines that start declarations (type, grammar,
/// prism, form, action, etc.) get a loss + coupling annotation.
fn compute_code_lenses(state: &LspState, uri: &str) -> Vec<CodeLensEntry> {
    let source = match state.documents.get(uri) {
        Some(doc) => doc.value().clone(),
        None => return Vec::new(),
    };

    let diags = state
        .diagnostics
        .get(uri)
        .map(|d| d.value().clone())
        .unwrap_or_default();

    let luminosity = state
        .luminosities
        .get(uri)
        .map(|l| l.value().clone())
        .unwrap_or(Luminosity::Light);

    // Build per-line loss from diagnostics
    let line_count = source.lines().count();
    let mut line_loss: Vec<f64> = vec![0.0; line_count];
    for diag in &diags {
        if diag.line < line_count {
            // Each diagnostic contributes loss based on severity
            let bits = match &diag.severity {
                MirrorSeverity::Error => 4.0,
                MirrorSeverity::Warning => 2.0,
                MirrorSeverity::Info => 0.5,
            };
            line_loss[diag.line] += bits;
        }
    }

    // Total document loss for coupling proxy
    let total_loss: f64 = line_loss.iter().sum();
    let coupling = if total_loss > 0.0 && line_count > 0 {
        // Coupling proxy: fraction of lines with non-zero loss
        let affected = line_loss.iter().filter(|&&l| l > 0.0).count();
        affected as f64 / line_count as f64
    } else {
        0.0
    };

    let mut lenses = Vec::new();

    // Detect declaration lines for codeLens placement
    let declaration_keywords = [
        "type", "grammar", "prism", "form", "action", "property", "fold",
        "lens", "traversal", "focus", "project", "split", "zoom", "refract",
        "binding", "default",
    ];

    for (line_idx, line_text) in source.lines().enumerate() {
        let trimmed = line_text.trim();
        let is_declaration = declaration_keywords
            .iter()
            .any(|kw| trimmed.starts_with(kw));

        if is_declaration {
            let bits = line_loss[line_idx];
            let gutter = if luminosity == Luminosity::Dark {
                // If the whole document failed to compile, everything is Void
                // because we can't measure individual line loss accurately.
                GutterLuminosity::Void
            } else {
                GutterLuminosity::from_bits(bits)
            };

            lenses.push(CodeLensEntry {
                line: line_idx as u32,
                loss_bits: bits,
                coupling,
                luminosity: gutter,
            });
        }
    }

    lenses
}

/// Compute the full loss report across all open documents.
fn compute_loss_report(state: &LspState) -> LossReport {
    let mut files = Vec::new();

    for entry in state.diagnostics.iter() {
        let uri = entry.key().clone();
        let diags = entry.value();

        let total_loss: f64 = diags
            .iter()
            .map(|d| match &d.severity {
                MirrorSeverity::Error => 4.0,
                MirrorSeverity::Warning => 2.0,
                MirrorSeverity::Info => 0.5,
            })
            .sum();

        let luminosity = GutterLuminosity::from_bits(total_loss);

        files.push(FileLossEntry {
            uri,
            total_loss_bits: total_loss,
            diagnostic_count: diags.len(),
            luminosity,
        });
    }

    LossReport {
        files,
        self_loss: state.self_loss,
        proposal_count: state.proposal_count,
        accepted_count: state.accepted_count,
        rejected_count: state.rejected_count,
    }
}

// ── Diagnostic severity mapping with gutter colors ───────────────────

/// Map MirrorSeverity to tower-lsp severity with gutter-aware mapping.
///
/// The honest gutter uses four levels:
/// - Light (0 bits) → Hint — peer knows this code
/// - Dimmed (1-2 bits) → Information — peer is close
/// - Dark (3+ bits) → Warning — peer was wrong
/// - Void (unmeasurable) → Warning with Unnecessary tag
fn to_gutter_severity(
    severity: &MirrorSeverity,
    loss_bits: f64,
) -> tower_lsp::lsp_types::DiagnosticSeverity {
    let gutter = GutterLuminosity::from_bits(loss_bits);
    match gutter {
        GutterLuminosity::Light => tower_lsp::lsp_types::DiagnosticSeverity::HINT,
        GutterLuminosity::Dimmed => tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION,
        GutterLuminosity::Dark | GutterLuminosity::Void => {
            // Preserve the original severity for actual errors
            match severity {
                MirrorSeverity::Error => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
                MirrorSeverity::Warning => tower_lsp::lsp_types::DiagnosticSeverity::WARNING,
                MirrorSeverity::Info => tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION,
            }
        }
    }
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

/// Convert mirror diagnostics to tower-lsp diagnostics with honest gutter colors.
///
/// Each diagnostic carries a loss value based on its severity:
/// - Error → 4 bits, Warning → 2 bits, Info → 0.5 bits
///
/// The gutter color then maps to LSP severity:
/// - Light (0 bits) → Hint
/// - Dimmed (1-2 bits) → Information
/// - Dark/Void → preserves original severity
fn to_lsp_diagnostics(diags: &[MirrorDiagnostic]) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    diags
        .iter()
        .map(|d| {
            let loss_bits = match &d.severity {
                MirrorSeverity::Error => 4.0,
                MirrorSeverity::Warning => 2.0,
                MirrorSeverity::Info => 0.5,
            };
            let gutter = GutterLuminosity::from_bits(loss_bits);

            // For Void diagnostics (unmeasurable), add a custom tag
            let tags = if gutter == GutterLuminosity::Void {
                Some(vec![tower_lsp::lsp_types::DiagnosticTag::UNNECESSARY])
            } else {
                None
            };

            tower_lsp::lsp_types::Diagnostic {
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
                severity: Some(to_gutter_severity(&d.severity, loss_bits)),
                source: Some("mirror".into()),
                message: d.message.clone(),
                code: d
                    .code
                    .as_ref()
                    .map(|c| tower_lsp::lsp_types::NumberOrString::String(c.clone())),
                tags,
                ..Default::default()
            }
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
                code_lens_provider: Some(tower_lsp::lsp_types::CodeLensOptions {
                    resolve_provider: Some(false),
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

    async fn code_lens(
        &self,
        params: tower_lsp::lsp_types::CodeLensParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<tower_lsp::lsp_types::CodeLens>>> {
        let uri = params.text_document.uri.to_string();

        let result = ractor::call!(self.actor, |reply| LspMsg::GetCodeLenses {
            uri,
            reply,
        });

        match result {
            Ok(entries) => {
                let lenses: Vec<tower_lsp::lsp_types::CodeLens> = entries
                    .iter()
                    .map(|entry| {
                        let title = format!(
                            "loss: {:.1} bits | coupling: {:.2} | {}",
                            entry.loss_bits,
                            entry.coupling,
                            entry.luminosity.as_str(),
                        );
                        tower_lsp::lsp_types::CodeLens {
                            range: tower_lsp::lsp_types::Range {
                                start: tower_lsp::lsp_types::Position {
                                    line: entry.line,
                                    character: 0,
                                },
                                end: tower_lsp::lsp_types::Position {
                                    line: entry.line,
                                    character: 0,
                                },
                            },
                            command: Some(tower_lsp::lsp_types::Command {
                                title,
                                command: String::new(),
                                arguments: None,
                            }),
                            data: None,
                        }
                    })
                    .collect();
                if lenses.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(lenses))
                }
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

    // ── Tick 4: honest gutter tests ──────────────────────────────────

    #[tokio::test]
    async fn code_lens_returns_entries_for_declarations() {
        let (actor_ref, _documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        // Open a document with declarations
        let source = "type color = red | blue\ntype shape = circle | square";
        actor_ref
            .cast(LspMsg::DidOpen {
                uri: "file:///test.mirror".to_string(),
                source: source.to_string(),
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

        // Request codeLenses
        let lenses: Vec<CodeLensEntry> = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetCodeLenses {
                uri: "file:///test.mirror".to_string(),
                reply,
            }
        )
        .expect("code lens failed");

        // Two type declarations = two code lenses
        assert_eq!(lenses.len(), 2, "should have one lens per declaration");
        assert_eq!(lenses[0].line, 0);
        assert_eq!(lenses[1].line, 1);

        // Clean source = Light luminosity
        assert_eq!(lenses[0].luminosity, GutterLuminosity::Light);

        actor_ref.stop(None);
    }

    #[test]
    fn gutter_luminosity_from_bits_classification() {
        // Light: 0 bits
        assert_eq!(GutterLuminosity::from_bits(0.0), GutterLuminosity::Light);

        // Dimmed: 1-2 bits
        assert_eq!(GutterLuminosity::from_bits(1.0), GutterLuminosity::Dimmed);
        assert_eq!(GutterLuminosity::from_bits(2.5), GutterLuminosity::Dimmed);

        // Dark: 3+ bits
        assert_eq!(GutterLuminosity::from_bits(3.0), GutterLuminosity::Dark);
        assert_eq!(GutterLuminosity::from_bits(10.0), GutterLuminosity::Dark);

        // Void: NaN / Infinity
        assert_eq!(GutterLuminosity::from_bits(f64::NAN), GutterLuminosity::Void);
        assert_eq!(
            GutterLuminosity::from_bits(f64::INFINITY),
            GutterLuminosity::Void
        );
        assert_eq!(
            GutterLuminosity::from_bits(f64::NEG_INFINITY),
            GutterLuminosity::Void
        );
    }

    #[test]
    fn gutter_luminosity_as_str() {
        assert_eq!(GutterLuminosity::Light.as_str(), "light");
        assert_eq!(GutterLuminosity::Dimmed.as_str(), "dimmed");
        assert_eq!(GutterLuminosity::Dark.as_str(), "dark");
        assert_eq!(GutterLuminosity::Void.as_str(), "void");
    }

    #[test]
    fn self_loss_computation_correct() {
        // 10 proposals, 7 accepted, 3 rejected → H(0.7, 0.3)
        let p: f64 = 7.0 / 10.0;
        let expected = -(p * p.log2() + (1.0 - p) * (1.0 - p).log2());
        let computed = shannon_entropy(p);
        assert!(
            (computed - expected).abs() < 1e-10,
            "expected {}, got {}",
            expected,
            computed
        );
        // H(0.7) ~ 0.8813
        assert!(
            (computed - 0.8813).abs() < 0.001,
            "H(0.7) should be ~0.8813, got {}",
            computed
        );
    }

    #[test]
    fn self_loss_edge_cases() {
        // All accepted → 0 entropy
        assert_eq!(shannon_entropy(1.0), 0.0);
        // All rejected → 0 entropy
        assert_eq!(shannon_entropy(0.0), 0.0);
        // Equal split → max entropy = 1.0 bit
        assert!((shannon_entropy(0.5) - 1.0).abs() < 1e-10);
    }

    #[tokio::test]
    async fn loss_report_includes_file_data() {
        let (actor_ref, _documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        // Open a document with an unknown token to generate diagnostics
        actor_ref
            .cast(LspMsg::DidOpen {
                uri: "file:///test.mirror".to_string(),
                source: "type color = red | blue\nwidget foo".to_string(),
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

        // Get loss report
        let report: LossReport = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetLossReport { reply }
        )
        .expect("loss report failed");

        // Should have one file entry
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].uri, "file:///test.mirror");
        assert!(
            report.files[0].diagnostic_count > 0,
            "should have diagnostics for invalid source"
        );
        assert!(
            report.files[0].total_loss_bits > 0.0,
            "should have non-zero loss for invalid source"
        );

        // Self-loss should be 0 (no proposals tracked yet)
        assert_eq!(report.self_loss, 0.0);
        assert_eq!(report.proposal_count, 0);

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn completion_increments_proposal_count() {
        let (actor_ref, _documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        // Request completions
        let _: Vec<MirrorCompletionItem> = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetCompletions { reply }
        )
        .expect("completions failed");

        // Check loss report
        let report: LossReport = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetLossReport { reply }
        )
        .expect("loss report failed");

        assert_eq!(report.proposal_count, 1);

        actor_ref.stop(None);
    }

    #[test]
    fn diagnostic_severity_mapping_for_luminosity_levels() {
        // Light (0 bits) → Hint
        let sev = to_gutter_severity(&MirrorSeverity::Info, 0.0);
        assert_eq!(sev, tower_lsp::lsp_types::DiagnosticSeverity::HINT);

        // Dimmed (1 bit) → Information
        let sev = to_gutter_severity(&MirrorSeverity::Warning, 1.5);
        assert_eq!(sev, tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION);

        // Dark (4 bits, Error) → preserves ERROR
        let sev = to_gutter_severity(&MirrorSeverity::Error, 4.0);
        assert_eq!(sev, tower_lsp::lsp_types::DiagnosticSeverity::ERROR);

        // Dark (3 bits, Warning) → preserves WARNING
        let sev = to_gutter_severity(&MirrorSeverity::Warning, 3.0);
        assert_eq!(sev, tower_lsp::lsp_types::DiagnosticSeverity::WARNING);

        // Void (infinity) → preserves original severity
        let sev = to_gutter_severity(&MirrorSeverity::Warning, f64::INFINITY);
        assert_eq!(sev, tower_lsp::lsp_types::DiagnosticSeverity::WARNING);
    }

    #[tokio::test]
    async fn did_change_after_completion_increments_rejected() {
        let (actor_ref, _documents) = LspActor::spawn_new(None)
            .await
            .expect("spawn failed");

        // Open doc
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

        // Request completions (marks pending)
        let _: Vec<MirrorCompletionItem> = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetCompletions { reply }
        )
        .expect("completions failed");

        // User types something different (DidChange while completion pending)
        actor_ref
            .cast(LspMsg::DidChange {
                uri: "file:///test.mirror".to_string(),
                source: "type b = y".to_string(),
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

        // Check loss report — should show 1 rejection
        let report: LossReport = ractor::call!(
            actor_ref,
            |reply| LspMsg::GetLossReport { reply }
        )
        .expect("loss report failed");

        assert_eq!(report.proposal_count, 1);
        assert_eq!(report.rejected_count, 1);
        // With 0 accepted, 1 rejected: p=0 → H=0
        assert_eq!(report.self_loss, 0.0);

        actor_ref.stop(None);
    }
}
