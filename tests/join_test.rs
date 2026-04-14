//! Tests for sel::llm and sel::join types.
//! Requires --features sel.
#![cfg(feature = "sel")]

use spectral::sel::llm::{ClaudeClient, Message, Role, format_system_prompt};
use spectral::apache2::identity::BiasChain;

#[test]
fn llm_message_construction() {
    let msg = Message::new(Role::User, "hello");
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.content, "hello");
}

#[test]
fn client_construction() {
    let client = ClaudeClient::new("test-key".into(), "claude-sonnet-4-20250514".into());
    assert_eq!(client.model(), "claude-sonnet-4-20250514");
}

#[test]
fn system_prompt_includes_bias_chain() {
    let chain = BiasChain::new(vec![
        "narrative".into(),
        "identity".into(),
        "knowledge".into(),
    ]);
    let prompt = format_system_prompt("Reed", &chain);
    assert!(prompt.contains("Reed"));
    assert!(prompt.contains("narrative"));
    assert!(prompt.contains("identity"));
    assert!(prompt.contains("knowledge"));
}

#[test]
fn system_prompt_with_context_includes_files() {
    use spectral::sel::llm::format_system_prompt_with_context;

    let chain = BiasChain::new(vec!["narrative".into()]);
    let files = vec![
        ("00-narrative.mirror".to_string(), "I am the story.".to_string()),
        ("01-identity.mirror".to_string(), "I am who I am.".to_string()),
    ];
    let prompt = format_system_prompt_with_context("Reed", &chain, &files);
    assert!(prompt.contains("I am the story."));
    assert!(prompt.contains("I am who I am."));
    assert!(prompt.contains("00-narrative.mirror"));
}
