//! spectral init — compile identity into a spectral-db graph.

use std::path::Path;
use terni::{Imperfect, Loss};
use super::identity::BiasChain;
use super::loss::InitLoss;

/// Result of initializing identity from a directory of .mirror files.
#[derive(Debug)]
pub struct InitResult {
    pub bias_chain: BiasChain,
    pub mirror_files_found: u32,
    pub files: Vec<(String, String)>,
}

/// Read directory, find .mirror files, derive bias chain from filename order.
/// "00-narrative.mirror" -> "narrative" in the bias chain.
/// Returns Success (all clean), Partial (some warnings), Failure (no files).
pub fn init_identity(path: &Path) -> Imperfect<InitResult, String, InitLoss> {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            return Imperfect::Failure(
                format!("cannot read directory: {}", e),
                InitLoss::total(),
            );
        }
    };

    let mut mirror_files: Vec<(String, String)> = Vec::new();

    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(".mirror") {
            let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
            mirror_files.push((file_name, content));
        }
    }

    if mirror_files.is_empty() {
        return Imperfect::Failure(
            "no .mirror files found".to_string(),
            InitLoss { grammars_compiled: 0, grammars_with_warnings: 0 },
        );
    }

    // Sort by filename to get numbered ordering
    mirror_files.sort_by(|a, b| a.0.cmp(&b.0));

    // Derive bias chain: "00-narrative.mirror" -> "narrative"
    let ordering: Vec<String> = mirror_files
        .iter()
        .map(|(name, _)| {
            let stem = name.trim_end_matches(".mirror");
            // Strip leading number prefix like "00-"
            if let Some(pos) = stem.find('-') {
                let prefix = &stem[..pos];
                if prefix.chars().all(|c| c.is_ascii_digit()) {
                    return stem[pos + 1..].to_string();
                }
            }
            stem.to_string()
        })
        .collect();

    let count = mirror_files.len() as u32;

    Imperfect::Success(InitResult {
        bias_chain: BiasChain::new(ordering),
        mirror_files_found: count,
        files: mirror_files,
    })
}
