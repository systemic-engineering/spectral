//! spectral join — boot identity, enter conversation.

use colored::Colorize;
use rustyline::DefaultEditor;

use crate::apache2::init::init_identity;
use crate::sel::llm::{format_system_prompt_with_context, ClaudeClient, Message, Role};

pub fn join(path: &std::path::Path) -> Result<(), String> {
    // 1. Load identity
    let init_result = match init_identity(path) {
        terni::Imperfect::Success(r) => r,
        terni::Imperfect::Partial(r, loss) => {
            eprintln!(
                "{}",
                format!(
                    "identity loaded with {} warnings",
                    loss.grammars_with_warnings
                )
                .yellow()
            );
            r
        }
        terni::Imperfect::Failure(err, _) => {
            return Err(format!("failed to load identity: {}", err));
        }
    };

    // 2. Derive name from bias chain (first entry or "spectral")
    let name = init_result
        .bias_chain
        .first()
        .unwrap_or("spectral")
        .to_string();

    // 3. Build system prompt with file contents
    let system = format_system_prompt_with_context(&name, &init_result.bias_chain, &init_result.files);

    // 4. Build Claude client
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;
    let model = std::env::var("SPECTRAL_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
    let client = ClaudeClient::new(api_key, model.clone());

    // 5. Welcome
    eprintln!(
        "{}",
        format!(
            "spectral join — {} — bias: {}",
            name,
            init_result.bias_chain.ordering().join(" => ")
        )
        .green()
    );
    eprintln!("{}", format!("model: {} | /quit to exit", model).dimmed());
    eprintln!();

    // 6. Prompt loop
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut messages: Vec<Message> = Vec::new();
    let mut rl = DefaultEditor::new().map_err(|e| format!("readline: {}", e))?;

    loop {
        let prompt = format!("{} ", "spectral>".dimmed());
        match rl.readline(&prompt) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed == "/quit" || trimmed == "/q" {
                    break;
                }

                let _ = rl.add_history_entry(trimmed);

                // Commands
                if trimmed.starts_with('/') {
                    match trimmed {
                        "/status" => {
                            eprintln!(
                                "  {} {}",
                                "bias:".dimmed(),
                                init_result.bias_chain.ordering().join(" => ")
                            );
                            eprintln!("  {} {}", "turns:".dimmed(), messages.len());
                            eprintln!("  {} {}", "model:".dimmed(), client.model());
                        }
                        "/clear" => {
                            messages.clear();
                            eprintln!("{}", "  conversation cleared".yellow());
                        }
                        other => {
                            eprintln!("  {} {}", "unknown command:".red(), other);
                        }
                    }
                    continue;
                }

                // Send to Claude
                messages.push(Message::new(Role::User, trimmed));

                match rt.block_on(client.send(&system, &messages)) {
                    Ok(response) => {
                        println!("\n{}\n", response);
                        messages.push(Message::new(Role::Assistant, &response));

                        // Status line after each turn
                        let turns = messages.len() / 2;
                        eprintln!(
                            "{}",
                            format!(
                                "  turns: {} | bias: {} | model: {}",
                                turns,
                                init_result
                                    .bias_chain
                                    .first()
                                    .unwrap_or("?"),
                                client.model()
                            )
                            .dimmed()
                        );
                    }
                    Err(e) => {
                        eprintln!("{}", format!("  error: {}", e).red());
                        messages.pop(); // remove failed user message
                    }
                }
            }
            Err(_) => break, // ctrl-d
        }
    }

    eprintln!("\n{}", "session ended.".dimmed());
    Ok(())
}

/// Register a directory as a spectral peer in ~/.spectral.
/// Returns a summary string for display.
pub fn join_peer(path: &std::path::Path) -> Result<String, String> {
    todo!()
}

#[cfg(test)]
mod peer_tests {
    use super::*;
    use std::env;

    #[test]
    fn join_peer_registers_in_home_spectral() {
        let tmp = tempfile::tempdir().unwrap();
        unsafe { env::set_var("HOME", tmp.path()) };

        // Source dir with a .spectral/manifest.json
        let src = tempfile::tempdir().unwrap();
        let spec_dir = src.path().join(".spectral");
        std::fs::create_dir_all(&spec_dir).unwrap();
        std::fs::write(
            spec_dir.join("manifest.json"),
            serde_json::to_vec(&vec!["oid-a", "oid-b"]).unwrap(),
        )
        .unwrap();

        let result = join_peer(src.path()).unwrap();
        assert!(result.contains("joined"), "should report joining: {}", result);

        let home_spectral = tmp.path().join(".spectral");
        let registry = crate::sel::peer::load_registry(&home_spectral);
        assert_eq!(registry.peers.len(), 1);

        let refs_raw = std::fs::read(home_spectral.join("refs.json")).unwrap();
        let refs: Vec<crate::sel::peer::RefPacket> =
            serde_json::from_slice(&refs_raw).unwrap();
        assert_eq!(refs.len(), 2);
    }
}
