//! detect — file grammar auto-detection and directory walking.
//!
//! Given a path, detect the grammar kind by extension. Walk a directory
//! tree respecting .gitignore, classify every file, produce a breakdown.
//!
//! This is phase 1 of gestalt auto-detection: no parse, just classification.

use std::path::{Path, PathBuf};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// GrammarKind — what grammar handles this file
// ---------------------------------------------------------------------------

/// The grammar that handles a detected file.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GrammarKind {
    /// Markdown document (.md, .mdx)
    Markdown,
    /// Gestalt native format (.gestalt)
    GestaltNative,
    /// Mirror grammar (.mirror)
    Mirror,
    /// Source code, with language tag
    Code(String),
    /// Configuration file, with format tag
    Config(String),
    /// Binary asset (image, pdf, etc.)
    Asset,
    /// Unknown file type
    Unknown,
}

/// A file detected during directory walk.
#[derive(Clone, Debug)]
pub struct DetectedFile {
    pub path: PathBuf,
    pub kind: GrammarKind,
}

/// Breakdown of detected file counts by grammar kind.
#[derive(Clone, Debug, Default)]
pub struct GestaltBreakdown {
    pub markdown: u32,
    pub code: u32,
    pub config: u32,
    pub asset: u32,
    pub gestalt_native: u32,
    pub mirror: u32,
    pub other: u32,
}

impl GestaltBreakdown {
    /// Total number of detected files.
    pub fn total(&self) -> u32 {
        self.markdown + self.code + self.config + self.asset
            + self.gestalt_native + self.mirror + self.other
    }

    /// Record a detected file kind.
    pub fn record(&mut self, kind: &GrammarKind) {
        match kind {
            GrammarKind::Markdown => self.markdown += 1,
            GrammarKind::GestaltNative => self.gestalt_native += 1,
            GrammarKind::Mirror => self.mirror += 1,
            GrammarKind::Code(_) => self.code += 1,
            GrammarKind::Config(_) => self.config += 1,
            GrammarKind::Asset => self.asset += 1,
            GrammarKind::Unknown => self.other += 1,
        }
    }
}

// ---------------------------------------------------------------------------
// detect_grammar — extension-based classification
// ---------------------------------------------------------------------------

/// Detect the grammar kind for a file by its extension.
pub fn detect_grammar(path: &Path) -> GrammarKind {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_lowercase(),
        None => return GrammarKind::Unknown,
    };

    match ext.as_str() {
        // Markdown
        "md" | "mdx" => GrammarKind::Markdown,

        // Gestalt native
        "gestalt" => GrammarKind::GestaltNative,

        // Mirror grammar
        "mirror" => GrammarKind::Mirror,

        // Code — Rust
        "rs" => GrammarKind::Code("rust".into()),
        // Code — Elixir
        "ex" | "exs" => GrammarKind::Code("elixir".into()),
        // Code — TypeScript
        "ts" | "tsx" => GrammarKind::Code("typescript".into()),
        // Code — JavaScript
        "js" | "jsx" | "mjs" | "cjs" => GrammarKind::Code("javascript".into()),
        // Code — Gleam
        "gleam" => GrammarKind::Code("gleam".into()),
        // Code — Python
        "py" => GrammarKind::Code("python".into()),
        // Code — Go
        "go" => GrammarKind::Code("go".into()),
        // Code — Ruby
        "rb" => GrammarKind::Code("ruby".into()),
        // Code — Shell
        "sh" | "bash" | "zsh" => GrammarKind::Code("shell".into()),
        // Code — Erlang
        "erl" | "hrl" => GrammarKind::Code("erlang".into()),
        // Code — Haskell
        "hs" => GrammarKind::Code("haskell".into()),
        // Code — C/C++
        "c" | "h" => GrammarKind::Code("c".into()),
        "cpp" | "hpp" | "cc" | "cxx" => GrammarKind::Code("cpp".into()),
        // Code — Java
        "java" => GrammarKind::Code("java".into()),
        // Code — Scala
        "scala" => GrammarKind::Code("scala".into()),
        // Code — Swift
        "swift" => GrammarKind::Code("swift".into()),
        // Code — Kotlin
        "kt" | "kts" => GrammarKind::Code("kotlin".into()),
        // Code — Lua
        "lua" => GrammarKind::Code("lua".into()),
        // Code — CSS/SCSS
        "css" | "scss" | "sass" | "less" => GrammarKind::Code("css".into()),
        // Code — HTML
        "html" | "htm" => GrammarKind::Code("html".into()),
        // Code — SQL
        "sql" => GrammarKind::Code("sql".into()),
        // Code — Fortran
        "f90" | "f95" | "f03" => GrammarKind::Code("fortran".into()),
        // Code — Zig
        "zig" => GrammarKind::Code("zig".into()),

        // Config — YAML
        "yaml" | "yml" => GrammarKind::Config("yaml".into()),
        // Config — TOML
        "toml" => GrammarKind::Config("toml".into()),
        // Config — JSON
        "json" => GrammarKind::Config("json".into()),
        // Config — Nix
        "nix" => GrammarKind::Config("nix".into()),
        // Config — INI
        "ini" | "cfg" => GrammarKind::Config("ini".into()),
        // Config — XML
        "xml" => GrammarKind::Config("xml".into()),
        // Config — Env
        "env" => GrammarKind::Config("env".into()),

        // Asset — Images
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" | "bmp" | "tiff" => {
            GrammarKind::Asset
        }
        // Asset — Documents
        "pdf" => GrammarKind::Asset,
        // Asset — Audio/Video
        "mp3" | "wav" | "ogg" | "mp4" | "webm" | "avi" => GrammarKind::Asset,
        // Asset — Fonts
        "woff" | "woff2" | "ttf" | "otf" | "eot" => GrammarKind::Asset,
        // Asset — Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" => GrammarKind::Asset,
        // Asset — Compiled
        "wasm" | "o" | "so" | "dylib" | "dll" | "exe" => GrammarKind::Asset,

        _ => GrammarKind::Unknown,
    }
}

// ---------------------------------------------------------------------------
// walk_detected — directory walking with .gitignore respect
// ---------------------------------------------------------------------------

/// Walk a directory tree, classify files, respect .gitignore.
/// Returns detected files and a breakdown.
///
/// Uses a manual walk that checks .gitignore patterns. Skips hidden
/// directories (.git, .hg, etc.) and common build artifact directories.
pub fn walk_detected(root: &Path) -> (Vec<DetectedFile>, GestaltBreakdown) {
    let mut files = Vec::new();
    let mut breakdown = GestaltBreakdown::default();

    // Load .gitignore patterns if present
    let gitignore_patterns = load_gitignore(root);

    walk_recursive(root, root, &gitignore_patterns, &mut files, &mut breakdown);

    (files, breakdown)
}

/// Directories to always skip (hidden and build artifacts).
fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".hg" | ".svn" | ".direnv"
            | "node_modules" | "target" | "build" | "dist"
            | "_build" | ".build" | "__pycache__"
            | ".cache" | ".npm" | ".yarn"
            | ".spectral" | ".next" | ".nuxt"
            | "vendor" | "deps"
    )
}

/// Load .gitignore patterns from a root directory.
/// Returns a list of glob patterns that should be ignored.
fn load_gitignore(root: &Path) -> Vec<String> {
    let gitignore_path = root.join(".gitignore");
    match std::fs::read_to_string(gitignore_path) {
        Ok(content) => content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#')
            })
            .map(|line| line.trim().to_string())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Check if a path relative to root matches any gitignore pattern.
fn is_gitignored(relative: &Path, patterns: &[String]) -> bool {
    let rel_str = relative.to_string_lossy();
    for pattern in patterns {
        let pat = pattern.trim_end_matches('/');
        // Simple pattern matching: check if any path component matches
        if rel_str == pat {
            return true;
        }
        // Check directory prefix
        if rel_str.starts_with(&format!("{}/", pat)) {
            return true;
        }
        // Check if any component matches
        for component in relative.components() {
            let comp = component.as_os_str().to_string_lossy();
            if comp == pat {
                return true;
            }
            // Glob-like: *.ext pattern
            if let Some(ext_pat) = pattern.strip_prefix("*.") {
                if let Some(ext) = Path::new(comp.as_ref()).extension() {
                    if ext.to_string_lossy() == ext_pat {
                        return true;
                    }
                }
            }
        }
        // Direct glob: *.ext at root level
        if let Some(ext_pat) = pattern.strip_prefix("*.") {
            if let Some(ext) = relative.extension() {
                if ext.to_string_lossy() == ext_pat {
                    return true;
                }
            }
        }
    }
    false
}

fn walk_recursive(
    root: &Path,
    current: &Path,
    patterns: &[String],
    files: &mut Vec<DetectedFile>,
    breakdown: &mut GestaltBreakdown,
) {
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut entries_vec: Vec<_> = entries.flatten().collect();
    entries_vec.sort_by_key(|e| e.file_name());

    for entry in entries_vec {
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        // Skip hidden files/dirs (starting with .)
        if name.starts_with('.') {
            continue;
        }

        // Compute relative path for gitignore matching
        let relative = match path.strip_prefix(root) {
            Ok(r) => r.to_path_buf(),
            Err(_) => continue,
        };

        // Check gitignore
        if is_gitignored(&relative, patterns) {
            continue;
        }

        // Use entry.file_type() which does NOT follow symlinks.
        // This prevents infinite recursion through symlinked directory trees.
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if file_type.is_symlink() {
            // Don't follow symlinks — they can cause cycles and
            // point to trees outside the repo (e.g. visibility/protected/)
            continue;
        }

        if file_type.is_dir() {
            // Skip known artifact directories
            if should_skip_dir(&name) {
                continue;
            }
            walk_recursive(root, &path, patterns, files, breakdown);
        } else if file_type.is_file() {
            let kind = detect_grammar(&path);
            breakdown.record(&kind);
            files.push(DetectedFile {
                path: path.clone(),
                kind,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Markdown shape extraction (lightweight, no full parse)
// ---------------------------------------------------------------------------

/// Lightweight shape extracted from a markdown file.
#[derive(Clone, Debug, Default)]
pub struct MarkdownShape {
    pub heading_count: u32,
    pub paragraph_count: u32,
    pub word_count: u32,
    pub link_count: u32,
    pub wiki_link_targets: Vec<String>,
}

/// Extract lightweight shape from markdown content without full parse.
pub fn extract_markdown_shape(content: &str) -> MarkdownShape {
    let mut shape = MarkdownShape::default();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            shape.heading_count += 1;
        } else if !trimmed.is_empty() {
            shape.paragraph_count += 1;
            shape.word_count += trimmed.split_whitespace().count() as u32;
        }

        // Count links
        let mut rest = trimmed;
        while let Some(pos) = rest.find("[[") {
            rest = &rest[pos + 2..];
            if let Some(end) = rest.find("]]") {
                let target = &rest[..end];
                // Handle pipe syntax: [[Target|Display]]
                let actual_target = target.split('|').next().unwrap_or(target);
                shape.wiki_link_targets.push(actual_target.to_string());
                shape.link_count += 1;
                rest = &rest[end + 2..];
            } else {
                break;
            }
        }

        // Count markdown links [text](url)
        let mut rest2 = trimmed;
        while let Some(pos) = rest2.find("](") {
            shape.link_count += 1;
            rest2 = &rest2[pos + 2..];
        }
    }

    shape
}

/// Lightweight shape extracted from a code file.
#[derive(Clone, Debug, Default)]
pub struct CodeShape {
    pub function_count: u32,
    pub type_count: u32,
    pub import_count: u32,
    pub line_count: u32,
}

/// Extract lightweight shape from code content.
/// Uses grep-level heuristics, not a full parser.
pub fn extract_code_shape(content: &str, language: &str) -> CodeShape {
    let mut shape = CodeShape::default();
    shape.line_count = content.lines().count() as u32;

    for line in content.lines() {
        let trimmed = line.trim();
        match language {
            "rust" => {
                if trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("fn ")
                    || trimmed.starts_with("pub(crate) fn ")
                    || trimmed.starts_with("pub(super) fn ")
                {
                    shape.function_count += 1;
                }
                if trimmed.starts_with("pub struct ")
                    || trimmed.starts_with("struct ")
                    || trimmed.starts_with("pub enum ")
                    || trimmed.starts_with("enum ")
                    || trimmed.starts_with("pub type ")
                    || trimmed.starts_with("type ")
                    || trimmed.starts_with("pub trait ")
                    || trimmed.starts_with("trait ")
                {
                    shape.type_count += 1;
                }
                if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
                    shape.import_count += 1;
                }
            }
            "elixir" => {
                if trimmed.starts_with("def ") || trimmed.starts_with("defp ") {
                    shape.function_count += 1;
                }
                if trimmed.starts_with("defmodule ")
                    || trimmed.starts_with("defstruct")
                    || trimmed.starts_with("@type ")
                {
                    shape.type_count += 1;
                }
                if trimmed.starts_with("import ") || trimmed.starts_with("alias ") || trimmed.starts_with("use ") {
                    shape.import_count += 1;
                }
            }
            "typescript" | "javascript" => {
                if trimmed.starts_with("function ")
                    || trimmed.starts_with("export function ")
                    || trimmed.starts_with("export default function ")
                    || trimmed.starts_with("async function ")
                    || trimmed.starts_with("export async function ")
                    || trimmed.contains("=> {")
                    || trimmed.contains("=> (")
                {
                    shape.function_count += 1;
                }
                if trimmed.starts_with("interface ")
                    || trimmed.starts_with("export interface ")
                    || trimmed.starts_with("type ")
                    || trimmed.starts_with("export type ")
                    || trimmed.starts_with("class ")
                    || trimmed.starts_with("export class ")
                {
                    shape.type_count += 1;
                }
                if trimmed.starts_with("import ") {
                    shape.import_count += 1;
                }
            }
            "gleam" => {
                if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
                    shape.function_count += 1;
                }
                if trimmed.starts_with("pub type ") || trimmed.starts_with("type ") {
                    shape.type_count += 1;
                }
                if trimmed.starts_with("import ") {
                    shape.import_count += 1;
                }
            }
            "python" => {
                if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                    shape.function_count += 1;
                }
                if trimmed.starts_with("class ") {
                    shape.type_count += 1;
                }
                if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                    shape.import_count += 1;
                }
            }
            _ => {
                // Generic heuristics: count lines with common function patterns
                if trimmed.starts_with("fn ")
                    || trimmed.starts_with("def ")
                    || trimmed.starts_with("func ")
                    || trimmed.starts_with("function ")
                    || trimmed.starts_with("pub fn ")
                {
                    shape.function_count += 1;
                }
                if trimmed.starts_with("import ") || trimmed.starts_with("use ") {
                    shape.import_count += 1;
                }
            }
        }
    }

    shape
}

/// Count of top-level keys in a config file (simple heuristic).
pub fn extract_config_key_count(content: &str, format: &str) -> u32 {
    match format {
        "yaml" | "toml" | "ini" => {
            content
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty()
                        && !trimmed.starts_with('#')
                        && !trimmed.starts_with("//")
                        && !trimmed.starts_with('[')
                        && trimmed.contains(':')
                        || trimmed.contains('=')
                })
                .count() as u32
        }
        "json" => {
            // Count top-level keys by counting lines with ": "
            content
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    trimmed.contains("\": ") && !trimmed.starts_with("//")
                })
                .count() as u32
        }
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // -- detect_grammar unit tests --

    #[test]
    fn detect_grammar_markdown() {
        assert_eq!(detect_grammar(Path::new("doc.md")), GrammarKind::Markdown);
        assert_eq!(detect_grammar(Path::new("page.mdx")), GrammarKind::Markdown);
    }

    #[test]
    fn detect_grammar_code_rust() {
        assert_eq!(
            detect_grammar(Path::new("lib.rs")),
            GrammarKind::Code("rust".into())
        );
    }

    #[test]
    fn detect_grammar_code_elixir() {
        assert_eq!(
            detect_grammar(Path::new("server.ex")),
            GrammarKind::Code("elixir".into())
        );
        assert_eq!(
            detect_grammar(Path::new("test.exs")),
            GrammarKind::Code("elixir".into())
        );
    }

    #[test]
    fn detect_grammar_code_typescript() {
        assert_eq!(
            detect_grammar(Path::new("app.ts")),
            GrammarKind::Code("typescript".into())
        );
        assert_eq!(
            detect_grammar(Path::new("component.tsx")),
            GrammarKind::Code("typescript".into())
        );
    }

    #[test]
    fn detect_grammar_config_yaml() {
        assert_eq!(
            detect_grammar(Path::new("config.yaml")),
            GrammarKind::Config("yaml".into())
        );
        assert_eq!(
            detect_grammar(Path::new("config.yml")),
            GrammarKind::Config("yaml".into())
        );
    }

    #[test]
    fn detect_grammar_config_toml() {
        assert_eq!(
            detect_grammar(Path::new("Cargo.toml")),
            GrammarKind::Config("toml".into())
        );
    }

    #[test]
    fn detect_grammar_config_json() {
        assert_eq!(
            detect_grammar(Path::new("package.json")),
            GrammarKind::Config("json".into())
        );
    }

    #[test]
    fn detect_grammar_config_nix() {
        assert_eq!(
            detect_grammar(Path::new("flake.nix")),
            GrammarKind::Config("nix".into())
        );
    }

    #[test]
    fn detect_grammar_asset_png() {
        assert_eq!(detect_grammar(Path::new("logo.png")), GrammarKind::Asset);
    }

    #[test]
    fn detect_grammar_asset_pdf() {
        assert_eq!(detect_grammar(Path::new("doc.pdf")), GrammarKind::Asset);
    }

    #[test]
    fn detect_grammar_asset_svg() {
        assert_eq!(detect_grammar(Path::new("icon.svg")), GrammarKind::Asset);
    }

    #[test]
    fn detect_grammar_unknown() {
        assert_eq!(detect_grammar(Path::new("file.xyz")), GrammarKind::Unknown);
    }

    #[test]
    fn detect_grammar_no_extension() {
        assert_eq!(detect_grammar(Path::new("Makefile")), GrammarKind::Unknown);
    }

    #[test]
    fn detect_grammar_gestalt() {
        assert_eq!(
            detect_grammar(Path::new("piece.gestalt")),
            GrammarKind::GestaltNative
        );
    }

    #[test]
    fn detect_grammar_mirror() {
        assert_eq!(
            detect_grammar(Path::new("00-narrative.mirror")),
            GrammarKind::Mirror
        );
    }

    // -- walk_directory tests --

    #[test]
    fn walk_directory_counts_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "# Hello").unwrap();
        fs::write(dir.path().join("b.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("c.yaml"), "key: value").unwrap();
        fs::write(dir.path().join("d.png"), &[0x89, 0x50, 0x4E, 0x47]).unwrap();
        fs::write(dir.path().join("e.txt"), "hello").unwrap();

        let (files, breakdown) = walk_detected(dir.path());
        assert_eq!(files.len(), 5);
        assert_eq!(breakdown.markdown, 1);
        assert_eq!(breakdown.code, 1);
        assert_eq!(breakdown.config, 1);
        assert_eq!(breakdown.asset, 1);
        assert_eq!(breakdown.other, 1);
        assert_eq!(breakdown.total(), 5);
    }

    #[test]
    fn walk_directory_skips_hidden() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("visible.md"), "# Hello").unwrap();
        let hidden = dir.path().join(".hidden");
        fs::create_dir(&hidden).unwrap();
        fs::write(hidden.join("secret.md"), "# Secret").unwrap();

        let (files, _) = walk_detected(dir.path());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, dir.path().join("visible.md"));
    }

    #[test]
    fn walk_directory_skips_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), "ignored.md\nbuild/\n").unwrap();
        fs::write(dir.path().join("kept.md"), "# Kept").unwrap();
        fs::write(dir.path().join("ignored.md"), "# Ignored").unwrap();
        let build_dir = dir.path().join("build");
        fs::create_dir(&build_dir).unwrap();
        fs::write(build_dir.join("artifact.rs"), "fn main() {}").unwrap();

        let (files, breakdown) = walk_detected(dir.path());
        // Only kept.md should be found (build/ dir is in skip list too)
        assert_eq!(files.len(), 1);
        assert_eq!(breakdown.markdown, 1);
    }

    #[test]
    fn walk_directory_skips_node_modules() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("app.ts"), "export default {}").unwrap();
        let nm = dir.path().join("node_modules");
        fs::create_dir(&nm).unwrap();
        fs::write(nm.join("dep.js"), "module.exports = {}").unwrap();

        let (files, _) = walk_detected(dir.path());
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn walk_directory_recurses_into_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("root.md"), "# Root").unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("child.md"), "# Child").unwrap();
        let deep = sub.join("deep");
        fs::create_dir(&deep).unwrap();
        fs::write(deep.join("leaf.md"), "# Leaf").unwrap();

        let (files, breakdown) = walk_detected(dir.path());
        assert_eq!(files.len(), 3);
        assert_eq!(breakdown.markdown, 3);
    }

    #[test]
    fn walk_directory_gitignore_glob_pattern() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();
        fs::write(dir.path().join("app.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("debug.log"), "log data").unwrap();

        let (files, _) = walk_detected(dir.path());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].kind, GrammarKind::Code("rust".into()));
    }

    #[test]
    fn walk_directory_skips_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("real.md"), "# Real file").unwrap();
        let real_sub = dir.path().join("real_sub");
        fs::create_dir(&real_sub).unwrap();
        fs::write(real_sub.join("deep.md"), "# Deep file").unwrap();

        // Create a symlink to real_sub (would cause double-counting if followed)
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&real_sub, dir.path().join("link_sub")).unwrap();
            // Also create a file symlink
            std::os::unix::fs::symlink(
                dir.path().join("real.md"),
                dir.path().join("link.md"),
            ).unwrap();
        }

        let (files, breakdown) = walk_detected(dir.path());
        // Should only find real.md and deep.md, not the symlinked versions
        assert_eq!(files.len(), 2, "should skip symlinks, found {} files", files.len());
        assert_eq!(breakdown.markdown, 2);
    }

    // -- breakdown tests --

    #[test]
    fn breakdown_records_correctly() {
        let mut b = GestaltBreakdown::default();
        b.record(&GrammarKind::Markdown);
        b.record(&GrammarKind::Markdown);
        b.record(&GrammarKind::Code("rust".into()));
        b.record(&GrammarKind::Config("yaml".into()));
        b.record(&GrammarKind::Asset);
        b.record(&GrammarKind::GestaltNative);
        b.record(&GrammarKind::Unknown);

        assert_eq!(b.markdown, 2);
        assert_eq!(b.code, 1);
        assert_eq!(b.config, 1);
        assert_eq!(b.asset, 1);
        assert_eq!(b.gestalt_native, 1);
        assert_eq!(b.other, 1);
        assert_eq!(b.total(), 7);
    }

    // -- markdown shape extraction --

    #[test]
    fn extract_markdown_shape_basic() {
        let content = "# Title\n\nParagraph one.\n\nParagraph two.\n";
        let shape = extract_markdown_shape(content);
        assert_eq!(shape.heading_count, 1);
        assert_eq!(shape.paragraph_count, 2);
        assert!(shape.word_count > 0);
    }

    #[test]
    fn extract_markdown_shape_wiki_links() {
        let content = "See [[Target]] and [[Other|Display]] for details.\n";
        let shape = extract_markdown_shape(content);
        assert_eq!(shape.wiki_link_targets.len(), 2);
        assert_eq!(shape.wiki_link_targets[0], "Target");
        assert_eq!(shape.wiki_link_targets[1], "Other");
    }

    #[test]
    fn extract_markdown_shape_empty() {
        let shape = extract_markdown_shape("");
        assert_eq!(shape.heading_count, 0);
        assert_eq!(shape.paragraph_count, 0);
        assert_eq!(shape.word_count, 0);
    }

    // -- code shape extraction --

    #[test]
    fn extract_code_shape_rust() {
        let content = "use std::path::Path;\n\npub fn hello() {}\nfn helper() {}\n\npub struct Foo {}\nenum Bar {}\n";
        let shape = extract_code_shape(content, "rust");
        assert_eq!(shape.function_count, 2);
        assert_eq!(shape.type_count, 2);
        assert_eq!(shape.import_count, 1);
    }

    #[test]
    fn extract_code_shape_elixir() {
        let content = "defmodule MyApp do\n  def hello, do: :ok\n  defp helper, do: :ok\nend\n";
        let shape = extract_code_shape(content, "elixir");
        assert_eq!(shape.function_count, 2);
        assert_eq!(shape.type_count, 1);
    }

    #[test]
    fn extract_code_shape_typescript() {
        let content = "import React from 'react'\n\nexport interface Props {}\nexport function App() {}\n";
        let shape = extract_code_shape(content, "typescript");
        assert_eq!(shape.function_count, 1);
        assert_eq!(shape.type_count, 1);
        assert_eq!(shape.import_count, 1);
    }

    // -- config key count --

    #[test]
    fn extract_config_key_count_yaml() {
        let content = "name: spectral\nversion: 0.1.0\n# comment\ndescription: test\n";
        let count = extract_config_key_count(content, "yaml");
        assert!(count >= 3);
    }
}
