//! CompilerActor — wraps mirror's compilation pipeline in a Ractor actor.
//!
//! Single actor owns a MirrorRuntime instance. Compilation requests go through
//! messages, ensuring thread-safe sequential access.

use std::path::PathBuf;

use ractor::{Actor, ActorProcessingErr, ActorRef};

use mirror::lsp::server::MirrorDiagnostic;
use mirror::mirror_runtime::MirrorRuntime;

// ── Reply types ──────────────────────────────────────────────────────

/// Result of a compilation operation.
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// Content-addressed OID of the compiled artifact (if successful).
    pub oid: Option<String>,
    /// Diagnostics produced during compilation.
    pub diagnostics: Vec<MirrorDiagnostic>,
    /// Whether compilation succeeded (possibly with warnings).
    pub success: bool,
}

// ── Messages ─────────────────────────────────────────────────────────

/// Messages the CompilerActor can receive.
pub enum CompilerMsg {
    /// Compile mirror source from a string.
    Compile {
        source: String,
        reply: ractor::RpcReplyPort<CompileResult>,
    },

    /// Compile a mirror source file from disk.
    CompileFile {
        path: PathBuf,
        reply: ractor::RpcReplyPort<CompileResult>,
    },
}

// ── Actor state ──────────────────────────────────────────────────────

/// The actor's persistent state: owns a MirrorRuntime.
pub struct CompilerState {
    pub runtime: MirrorRuntime,
}

// ── Actor ────────────────────────────────────────────────────────────

/// The CompilerActor: wraps mirror compilation in a Ractor actor.
///
/// Single ownership of the MirrorRuntime. All compilation goes through
/// actor messages — no shared state.
pub struct CompilerActor;

impl CompilerActor {
    /// Spawn a CompilerActor with a fresh MirrorRuntime.
    pub async fn spawn_new(
        name: Option<String>,
    ) -> Result<ActorRef<CompilerMsg>, ractor::SpawnErr> {
        let (actor_ref, _handle) = Actor::spawn(name, CompilerActor, ()).await?;
        Ok(actor_ref)
    }
}

#[ractor::async_trait]
impl Actor for CompilerActor {
    type Msg = CompilerMsg;
    type State = CompilerState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: (),
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(CompilerState {
            runtime: MirrorRuntime::new(),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            CompilerMsg::Compile { source, reply } => {
                let result = compile_source(&state.runtime, &source);
                let _ = reply.send(result);
            }
            CompilerMsg::CompileFile { path, reply } => {
                let result = compile_file(&state.runtime, &path);
                let _ = reply.send(result);
            }
        }
        Ok(())
    }
}

// ── Compilation helpers ─────────────────────────────────────────────

/// Compile mirror source text and return a CompileResult.
fn compile_source(runtime: &MirrorRuntime, source: &str) -> CompileResult {
    use mirror::lsp::server::loss_to_diagnostics;

    let compiled = runtime.compile_source(source);
    let loss = compiled.loss();
    let diagnostics = loss_to_diagnostics(&loss);
    let success = compiled.is_ok();
    let oid = if success {
        compiled.ok().map(|c| c.crystal().to_string())
    } else {
        None
    };

    CompileResult {
        oid,
        diagnostics,
        success,
    }
}

/// Compile a mirror file from disk and return a CompileResult.
fn compile_file(runtime: &MirrorRuntime, path: &PathBuf) -> CompileResult {
    use mirror::lsp::server::loss_to_diagnostics;

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return CompileResult {
                oid: None,
                diagnostics: vec![MirrorDiagnostic {
                    line: 0,
                    col: 0,
                    end_col: 0,
                    severity: mirror::lsp::server::DiagnosticSeverity::Error,
                    message: format!("failed to read {}: {}", path.display(), e),
                    code: Some("io_error".to_string()),
                }],
                success: false,
            };
        }
    };

    let compiled = runtime.compile_source(&source);
    let loss = compiled.loss();
    let diagnostics = loss_to_diagnostics(&loss);
    let success = compiled.is_ok();
    let oid = if success {
        compiled.ok().map(|c| c.crystal().to_string())
    } else {
        None
    };

    CompileResult {
        oid,
        diagnostics,
        success,
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn compiler_actor_spawn_and_compile_valid_source() {
        let actor_ref = CompilerActor::spawn_new(None)
            .await
            .expect("spawn failed");

        let result: CompileResult = ractor::call!(
            actor_ref,
            |reply| CompilerMsg::Compile {
                source: "type color = red | blue".to_string(),
                reply,
            }
        )
        .expect("compile call failed");

        assert!(result.success, "valid source should compile successfully");
        assert!(result.oid.is_some(), "successful compile should produce an OID");

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn compiler_actor_compile_file_nonexistent() {
        let actor_ref = CompilerActor::spawn_new(None)
            .await
            .expect("spawn failed");

        let result: CompileResult = ractor::call!(
            actor_ref,
            |reply| CompilerMsg::CompileFile {
                path: PathBuf::from("/tmp/nonexistent_mirror_file.mirror"),
                reply,
            }
        )
        .expect("compile_file call failed");

        assert!(!result.success, "nonexistent file should fail");
        assert!(result.oid.is_none());
        assert!(!result.diagnostics.is_empty(), "should have error diagnostics");

        actor_ref.stop(None);
    }
}
