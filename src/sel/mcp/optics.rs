//! Git-native optics — thin wrappers over `git2` for the six MCP tools
//! that collapse to single git operations.
//!
//! Each function operates on `refs/spectral/HEAD` (a symref to
//! `refs/spectral/heads/main`) and the per-node subtree layout under
//! `nodes/{oid}/{,.content,target_oid,...}` documented in
//! `docs/git-native-graph-plan.md` §6.
//!
//! These optics are pure git-level views: they do not consult the
//! in-memory `SpectralIndex`. After ref-mutating ops (`checkout`,
//! `cherrypick`) the calling `MemoryActor`'s in-memory state is stale
//! relative to the new HEAD. Reconciliation requires a server restart.
//! See the per-tool documentation in the response payload (`note` field).
//!
//! Path classification used by [`diff`]:
//! ```text
//!   nodes/{oid}/.content              -> node mutation
//!   nodes/{oid}/{target_oid}/...      -> edge mutation
//!   crystals/...                      -> crystal change
//!   profile | coords | manifold       -> metadata change
//! ```

use std::path::Path;

use git2::{Diff, DiffOptions, Oid, Repository, Sort};

use super::memory::{
    BlameEntry, BranchResult, BranchTip, CheckoutResult, CherrypickResult, DiffReport, EdgeRef,
    ThreadEntry,
};

const HEAD_REF: &str = "refs/spectral/HEAD";
const HEADS_PREFIX: &str = "refs/spectral/heads/";

// ── Helpers ──────────────────────────────────────────────────────────

/// Validate a spectral branch name against git ref-format rules.
///
/// Rejects names that libgit2 would reject opaquely, plus a few extras:
/// null bytes, leading `/`, `.lock` suffix, `@{` sequence, and
/// characters that are special in git revspec (`~^:?*[\\`).
fn validate_branch_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("branch name must not be empty".into());
    }
    if name.contains('\0') {
        return Err("branch name must not contain null bytes".into());
    }
    if name.starts_with('/') {
        return Err("branch name must not start with '/'".into());
    }
    if name.ends_with(".lock") {
        return Err("branch name must not end with '.lock'".into());
    }
    if name.contains("..") {
        return Err("branch name must not contain '..'".into());
    }
    if name.contains("@{") {
        return Err("branch name must not contain '@{'".into());
    }
    for ch in ['~', '^', ':', '?', '*', '[', '\\', ' '] {
        if name.contains(ch) {
            return Err(format!("branch name must not contain '{ch}'"));
        }
    }
    // Final guard: let libgit2 catch anything we missed.
    let full_ref = format!("{HEADS_PREFIX}{name}");
    if !git2::Reference::is_valid_name(&full_ref) {
        return Err(format!("'{name}' is not a valid git ref name"));
    }
    Ok(())
}

fn open_repo(repo_path: &Path) -> Result<Repository, String> {
    Repository::open(repo_path).map_err(|e| format!("open repo: {e}"))
}

/// Resolve a ref name or hex oid (or "HEAD", "HEAD~N") to a commit OID.
fn resolve_commit(repo: &Repository, name: &str) -> Result<Oid, String> {
    // Try revparse first (handles HEAD, HEAD~1, branch names, oids).
    // We rewrite "HEAD" to refs/spectral/HEAD since plain git HEAD doesn't
    // exist in spectral-only repos (and even when it does, we want the
    // spectral one).
    let rewritten = if name == "HEAD" {
        HEAD_REF.to_string()
    } else if let Some(rest) = name.strip_prefix("HEAD") {
        // HEAD~N or HEAD^... — splice the spectral ref in.
        format!("{}{}", HEAD_REF, rest)
    } else {
        name.to_string()
    };

    let obj = repo
        .revparse_single(&rewritten)
        .map_err(|e| format!("resolve '{name}': {e}"))?;
    obj.peel_to_commit()
        .map(|c| c.id())
        .map_err(|e| format!("not a commit '{name}': {e}"))
}

/// Classify a path into one of: ("node", oid), ("edge", "{from}->{to}"),
/// ("crystal", path), ("metadata", path), ("other", path).
///
/// Handles both the Phase-4 layout (`nodes/{oid}/...`) and the legacy flat
/// layout where per-node subtrees lived at the tree root (`{oid}/...`).
/// A flat entry is recognised when the first path segment is exactly 40
/// hex characters — the spectral content OID.
fn classify_path(path: &str) -> (&'static str, String) {
    // Helper: split "oid/rest" where oid is 40 hex chars.
    fn split_oid_path(s: &str) -> Option<(&str, Option<&str>)> {
        let mut it = s.splitn(2, '/');
        let oid = it.next()?;
        if oid.len() == 40 && oid.bytes().all(|b| b.is_ascii_hexdigit()) {
            Some((oid, it.next()))
        } else {
            None
        }
    }

    if let Some(rest) = path.strip_prefix("nodes/") {
        // Phase-4 layout: nodes/{oid}/...
        let mut iter = rest.splitn(2, '/');
        let oid = iter.next().unwrap_or("").to_string();
        match iter.next() {
            Some(".content") | None => ("node", oid),
            Some(rest) => {
                let target = rest.split('/').next().unwrap_or("").to_string();
                ("edge", format!("{oid}->{target}"))
            }
        }
    } else if let Some(rest) = path.strip_prefix("crystals/") {
        ("crystal", rest.to_string())
    } else if path == "profile" || path == "coords" || path == "manifold" {
        ("metadata", path.to_string())
    } else if let Some((oid, tail)) = split_oid_path(path) {
        // Legacy flat layout: {oid}/... at tree root (pre-Phase-4).
        match tail {
            Some(".content") | None => ("node", oid.to_string()),
            Some(rest) => {
                let target = rest.split('/').next().unwrap_or("").to_string();
                ("edge", format!("{oid}->{target}"))
            }
        }
    } else {
        ("other", path.to_string())
    }
}

// ── memory_diff ──────────────────────────────────────────────────────

/// Compute a structured diff between two commits.
///
/// Defaults: `from = HEAD~1`, `to = HEAD`. If HEAD has no parent the
/// "from" tree is treated as empty (everything in HEAD is "added").
pub fn diff(
    repo_path: &Path,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<DiffReport, String> {
    let repo = open_repo(repo_path)?;

    let to_oid = match to {
        Some(s) => resolve_commit(&repo, s)?,
        None => resolve_commit(&repo, HEAD_REF)?,
    };
    let to_commit = repo
        .find_commit(to_oid)
        .map_err(|e| format!("find to commit: {e}"))?;
    let to_tree = to_commit
        .tree()
        .map_err(|e| format!("to tree: {e}"))?;

    let from_tree_opt = match from {
        Some(s) => {
            let oid = resolve_commit(&repo, s)?;
            let c = repo
                .find_commit(oid)
                .map_err(|e| format!("find from commit: {e}"))?;
            Some(c.tree().map_err(|e| format!("from tree: {e}"))?)
        }
        None => match to_commit.parent(0) {
            Ok(parent) => Some(parent.tree().map_err(|e| format!("parent tree: {e}"))?),
            Err(_) => None, // no parent — root commit
        },
    };

    let mut opts = DiffOptions::new();
    opts.include_typechange(true);
    let diff: Diff = repo
        .diff_tree_to_tree(from_tree_opt.as_ref(), Some(&to_tree), Some(&mut opts))
        .map_err(|e| format!("diff_tree_to_tree: {e}"))?;

    let from_label = from
        .map(|s| s.to_string())
        .unwrap_or_else(|| "HEAD~1".into());
    let to_label = to.map(|s| s.to_string()).unwrap_or_else(|| "HEAD".into());

    let mut report = DiffReport {
        from: from_label,
        to: to_label,
        added_nodes: Vec::new(),
        removed_nodes: Vec::new(),
        changed_nodes: Vec::new(),
        added_edges: Vec::new(),
        removed_edges: Vec::new(),
        added_crystals: Vec::new(),
        removed_crystals: Vec::new(),
        metadata_changed: Vec::new(),
    };

    diff.foreach(
        &mut |delta, _| {
            let status = delta.status();
            // Prefer new_file path; fall back to old.
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            if path.is_empty() {
                return true;
            }

            let (kind, key) = classify_path(&path);
            match (kind, status) {
                ("node", git2::Delta::Added) => report.added_nodes.push(key),
                ("node", git2::Delta::Deleted) => report.removed_nodes.push(key),
                ("node", _) => report.changed_nodes.push(key),
                ("edge", git2::Delta::Deleted) => {
                    if let Some(e) = parse_edge_key(&key) {
                        report.removed_edges.push(e);
                    }
                }
                ("edge", _) => {
                    if let Some(e) = parse_edge_key(&key) {
                        report.added_edges.push(e);
                    }
                }
                ("crystal", git2::Delta::Deleted) => report.removed_crystals.push(key),
                ("crystal", _) => report.added_crystals.push(key),
                ("metadata", _) => report.metadata_changed.push(key),
                _ => {}
            }
            true
        },
        None,
        None,
        None,
    )
    .map_err(|e| format!("diff foreach: {e}"))?;

    // Dedup (multiple files under one node directory should not double-count
    // the node itself; classify_path returns the same key for them).
    dedup(&mut report.added_nodes);
    dedup(&mut report.removed_nodes);
    dedup(&mut report.changed_nodes);
    dedup(&mut report.added_crystals);
    dedup(&mut report.removed_crystals);
    dedup(&mut report.metadata_changed);
    dedup_edges(&mut report.added_edges);
    dedup_edges(&mut report.removed_edges);

    Ok(report)
}

fn parse_edge_key(key: &str) -> Option<EdgeRef> {
    let mut it = key.splitn(2, "->");
    let from = it.next()?.to_string();
    let to = it.next()?.to_string();
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some(EdgeRef { from, to })
}

fn dedup(v: &mut Vec<String>) {
    v.sort();
    v.dedup();
}

fn dedup_edges(v: &mut Vec<EdgeRef>) {
    v.sort_by(|a, b| (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str())));
    v.dedup_by(|a, b| a.from == b.from && a.to == b.to);
}

// ── memory_blame ─────────────────────────────────────────────────────

/// Walk `refs/spectral/HEAD` and return commits that touched
/// `nodes/{oid}/` (or the legacy flat `{oid}/`). The chain is returned
/// newest-first. `fiedler_at_commit` is populated from the `profile` blob
/// at the commit's tree root when present (written by spectral-db Phase 4).
pub fn blame(repo_path: &Path, oid: &str) -> Result<Vec<BlameEntry>, String> {
    let repo = open_repo(repo_path)?;

    // Try Phase-4 path first; fall back to legacy flat path.
    let nested_prefix = format!("nodes/{oid}");

    let mut walk = repo
        .revwalk()
        .map_err(|e| format!("revwalk: {e}"))?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
        .map_err(|e| format!("set_sorting: {e}"))?;
    if walk.push_ref(HEAD_REF).is_err() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for rev in walk {
        let commit_id = match rev {
            Ok(id) => id,
            Err(e) => {
                eprintln!("spectral blame: revwalk error (skipping): {e}");
                continue;
            }
        };
        let commit = match repo.find_commit(commit_id) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("spectral blame: find_commit {commit_id} failed (skipping): {e}");
                continue;
            }
        };
        let tree = match commit.tree() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("spectral blame: tree for {commit_id} failed (skipping): {e}");
                continue;
            }
        };

        // Check both Phase-4 nested path and legacy flat path.
        let cur_subtree_oid = subtree_oid(&repo, &tree, &nested_prefix)
            .or_else(|| subtree_oid(&repo, &tree, oid));
        if cur_subtree_oid.is_none() {
            continue;
        }

        // Only emit commits where the subtree changed relative to the parent.
        let parent_subtree_oid = match commit.parent(0) {
            Ok(parent) => match parent.tree() {
                Ok(pt) => subtree_oid(&repo, &pt, &nested_prefix)
                    .or_else(|| subtree_oid(&repo, &pt, oid)),
                Err(_) => None,
            },
            Err(_) => None,
        };

        if cur_subtree_oid != parent_subtree_oid {
            let sig = commit.author();
            // Extract fiedler from the `profile` blob at the tree root (Phase 4).
            let fiedler_at_commit = read_fiedler_from_tree(&repo, &tree);
            out.push(BlameEntry {
                commit_oid: commit_id.to_string(),
                message: commit.summary().unwrap_or("").to_string(),
                timestamp: commit.time().seconds(),
                author: format!(
                    "{} <{}>",
                    sig.name().unwrap_or(""),
                    sig.email().unwrap_or("")
                ),
                fiedler_at_commit,
            });
        }
    }
    Ok(out)
}

fn subtree_oid(repo: &Repository, tree: &git2::Tree<'_>, path: &str) -> Option<Oid> {
    let entry = tree.get_path(std::path::Path::new(path)).ok()?;
    // Ensure it's a tree entry, not a blob.
    let _ = entry.to_object(repo).ok()?.peel_to_tree().ok()?;
    Some(entry.id())
}

/// Read the Fiedler value from the `profile` blob at the tree root.
///
/// The blob format (Phase 4) is:
/// ```text
/// spectral-profile\0fiedler: <f64>\n...
/// ```
/// Returns `None` if the blob is absent or the header cannot be parsed.
fn read_fiedler_from_tree(repo: &Repository, tree: &git2::Tree<'_>) -> Option<f64> {
    let entry = tree.get_name("profile")?;
    let blob = repo.find_blob(entry.id()).ok()?;
    let content = std::str::from_utf8(blob.content()).ok()?;
    // Header: "spectral-profile\0fiedler: <value>\n..."
    // After the null byte, find "fiedler: ".
    let after_magic = content.find('\0').map(|i| &content[i + 1..]).unwrap_or(content);
    for line in after_magic.lines() {
        if let Some(val) = line.strip_prefix("fiedler: ") {
            return val.trim().parse::<f64>().ok();
        }
    }
    None
}

// ── memory_branch ────────────────────────────────────────────────────

/// Create a branch at HEAD or list all branches under
/// `refs/spectral/heads/`.
pub fn branch(repo_path: &Path, name: Option<&str>) -> Result<BranchResult, String> {
    let repo = open_repo(repo_path)?;

    match name {
        Some(branch_name) => {
            validate_branch_name(branch_name)?;
            let head_oid = resolve_commit(&repo, HEAD_REF)?;
            let ref_name = format!("{HEADS_PREFIX}{branch_name}");
            repo.reference(&ref_name, head_oid, true, "memory_branch: create at HEAD")
                .map_err(|e| format!("create ref: {e}"))?;
            Ok(BranchResult::Created {
                ref_name,
                commit_oid: head_oid.to_string(),
            })
        }
        None => {
            let mut tips = Vec::new();
            let pattern = format!("{HEADS_PREFIX}*");
            let refs = repo
                .references_glob(&pattern)
                .map_err(|e| format!("references_glob: {e}"))?;
            for r in refs.flatten() {
                let full = match r.name() {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                let short = full.strip_prefix(HEADS_PREFIX).unwrap_or(&full).to_string();
                let target = match r.resolve().ok().and_then(|rr| rr.target()) {
                    Some(o) => o.to_string(),
                    None => continue,
                };
                tips.push(BranchTip {
                    name: short,
                    commit_oid: target,
                });
            }
            tips.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(BranchResult::List { branches: tips })
        }
    }
}

// ── memory_checkout ──────────────────────────────────────────────────

/// Repoint `refs/spectral/HEAD` to `refs/spectral/heads/{name}`.
///
/// **Note:** the in-memory `SpectralDb` state held by the calling
/// `MemoryActor` is now stale relative to the new branch tip. The
/// simplest reconciliation is to restart the server (a fresh
/// `SpectralDb::open` rebuilds the index from the new HEAD's tree).
/// This is signalled via the `note` field on the response.
pub fn checkout(repo_path: &Path, name: &str) -> Result<CheckoutResult, String> {
    let repo = open_repo(repo_path)?;
    validate_branch_name(name)?;
    let target_ref = format!("{HEADS_PREFIX}{name}");
    let target_commit = repo
        .find_reference(&target_ref)
        .map_err(|_| format!("branch '{name}' does not exist (no {target_ref})"))?
        .resolve()
        .map_err(|e| format!("resolve {target_ref}: {e}"))?
        .target()
        .ok_or_else(|| format!("{target_ref} has no target"))?;

    repo.reference_symbolic(HEAD_REF, &target_ref, true, "memory_checkout")
        .map_err(|e| format!("update symref: {e}"))?;

    Ok(CheckoutResult {
        branch: name.to_string(),
        commit_oid: target_commit.to_string(),
        note: "in-memory state is stale; restart the server to reload at the new HEAD".into(),
    })
}

// ── memory_thread ────────────────────────────────────────────────────

/// Walk the topic-note thread for `topic`. Tries both
/// `refs/spectral/notes/topics/{topic}` and `refs/spectral/notes/{topic}`;
/// uses the first that yields any entries. Returns chronological
/// (oldest-first) entries.
///
/// Uses `repo.notes()` to iterate only note-bearing commits — O(k) where k
/// is the note count, not O(N) over the full commit history.
pub fn thread(repo_path: &Path, topic: &str) -> Result<Vec<ThreadEntry>, String> {
    let candidates = [
        format!("refs/spectral/notes/topics/{topic}"),
        format!("refs/spectral/notes/{topic}"),
    ];

    for notes_ref in &candidates {
        let entries = collect_notes(repo_path, notes_ref)?;
        if !entries.is_empty() {
            return Ok(entries);
        }
    }
    Ok(Vec::new())
}

/// Iterate all notes under `notes_ref` using the notes iterator (O(k)).
/// Returns entries sorted oldest-first.
fn collect_notes(repo_path: &Path, notes_ref: &str) -> Result<Vec<ThreadEntry>, String> {
    let repo = open_repo(repo_path)?;

    let notes_iter = match repo.notes(Some(notes_ref)) {
        Ok(iter) => iter,
        Err(_) => return Ok(Vec::new()), // ref doesn't exist
    };

    let mut out = Vec::new();
    for item in notes_iter {
        let (note_id, annotated_id) = match item {
            Ok(pair) => pair,
            Err(_) => continue,
        };
        let note = match repo.find_blob(note_id) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let body = std::str::from_utf8(note.content())
            .unwrap_or("")
            .to_string();
        let ts = repo
            .find_commit(annotated_id)
            .map(|c| c.time().seconds())
            .unwrap_or(0);
        out.push(ThreadEntry {
            commit_oid: annotated_id.to_string(),
            timestamp: ts,
            body,
        });
    }

    // Oldest first.
    out.sort_by_key(|e| e.timestamp);
    Ok(out)
}

// ── memory_cherrypick ────────────────────────────────────────────────

/// Replay a same-repo `commit_oid`'s tree changes onto the current
/// `refs/spectral/HEAD`. Cross-repo cherry-pick is not yet implemented.
///
/// Implementation: we use `git2::Repository::cherrypick_commit` to
/// produce an in-memory index, write the merged tree, and write a new
/// commit on top of HEAD. The HEAD ref is then advanced under the
/// active branch (the symref's target).
///
/// Conflicts cause a hard error — we do not write conflict markers.
pub fn cherrypick(repo_path: &Path, commit_oid: &str) -> Result<CherrypickResult, String> {
    let repo = open_repo(repo_path)?;

    let pick_oid = Oid::from_str(commit_oid)
        .map_err(|e| format!("invalid commit oid '{commit_oid}': {e}"))?;
    let pick = repo
        .find_commit(pick_oid)
        .map_err(|e| format!("find pick commit: {e}"))?;

    let head_ref_resolved = repo
        .find_reference(HEAD_REF)
        .map_err(|e| format!("find HEAD: {e}"))?
        .resolve()
        .map_err(|e| format!("resolve HEAD: {e}"))?;
    let head_branch_name = head_ref_resolved
        .name()
        .ok_or_else(|| "HEAD has no resolved name".to_string())?
        .to_string();
    let head_oid = head_ref_resolved
        .target()
        .ok_or_else(|| "HEAD has no target".to_string())?;
    let head_commit = repo
        .find_commit(head_oid)
        .map_err(|e| format!("find HEAD commit: {e}"))?;

    // Cherry-pick into an in-memory index. Mainline of 1 = first parent.
    let mut idx = repo
        .cherrypick_commit(&pick, &head_commit, 0, None)
        .map_err(|e| format!("cherrypick_commit: {e}"))?;
    if idx.has_conflicts() {
        return Err("memory_cherrypick: conflicts (none-merge resolution required)".into());
    }
    let new_tree_oid = idx
        .write_tree_to(&repo)
        .map_err(|e| format!("write_tree_to: {e}"))?;
    let new_tree = repo
        .find_tree(new_tree_oid)
        .map_err(|e| format!("find new tree: {e}"))?;

    // Use the repo's configured identity; fall back to the project standard.
    let sig = repo
        .signature()
        .or_else(|_| git2::Signature::now("Reed", "reed@systemic.engineer"))
        .map_err(|e| format!("signature: {e}"))?;
    let summary = pick.summary().unwrap_or("cherry-pick");
    let msg = format!("memory_cherrypick: {summary}\n\ncherry picked from {pick_oid}");

    // Write the new commit. Update the active branch ref directly via
    // `repo.reference(...)`. Using the resolved branch name (not the
    // symref) ensures HEAD continues to symbolically point to the same
    // branch.
    let new_commit_oid = repo
        .commit(None, &sig, &sig, &msg, &new_tree, &[&head_commit])
        .map_err(|e| format!("commit: {e}"))?;
    repo.reference(
        &head_branch_name,
        new_commit_oid,
        true,
        "memory_cherrypick: advance HEAD",
    )
    .map_err(|e| format!("update {head_branch_name}: {e}"))?;

    Ok(CherrypickResult {
        source_commit: pick_oid.to_string(),
        new_head: new_commit_oid.to_string(),
        note: "in-memory state is stale; restart the server to reload at the new HEAD".into(),
    })
}
