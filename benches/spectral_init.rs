// spectral_init.rs -- benchmark suite for `spectral init`.
//
// [Taut] Two-tier snapshot benchmarks on real data.
//
// Part 1: Synthetic scaling (1, 10, 50, 100 .mirror files)
// Part 2: Two-tier snapshot isolation (FNV-1a vs CoincidenceHash)
// Part 3: Real repo benchmarks (systemic.engineering, identity)
// Part 4: Graph shape comparison across repos
//
// The cascade ratio tells you how much uncertainty the system can hold
// between measurements. Higher ratio = wider aperture. Not Hz. Aperture.
//
// Apache-2.0

use std::path::Path;
use std::time::Instant;

use spectral::apache2::init::{init_identity, serialize_init_state, InitSnapshot};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a temporary directory with N .mirror files.
fn make_mirror_dir(count: usize) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir creation failed");
    for i in 0..count {
        let filename = format!("{:02}-grammar-{}.mirror", i, i);
        let content = format!(
            "grammar @bench_{} {{\n  in @benchmark\n  type result_{} = str\n}}\n",
            i, i
        );
        std::fs::write(dir.path().join(&filename), content)
            .expect("write .mirror file failed");
    }
    dir
}

/// Scan a real directory for .mirror files, return sorted (name, content) pairs.
/// Same logic as init_identity but returns the file list for isolated benchmarks.
fn scan_mirror_files(path: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".mirror") {
                let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
                files.push((name, content));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// Scan a directory tree recursively for all files, return count and total bytes.
fn scan_directory_stats(path: &Path) -> (usize, usize) {
    let mut file_count = 0usize;
    let mut total_bytes = 0usize;
    fn walk(path: &Path, count: &mut usize, bytes: &mut usize) {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    // Skip .git, .spectral, node_modules
                    if name.starts_with('.') || name == "node_modules" {
                        continue;
                    }
                    walk(&p, count, bytes);
                } else if p.is_file() {
                    *count += 1;
                    if let Ok(meta) = p.metadata() {
                        *bytes += meta.len() as usize;
                    }
                }
            }
        }
    }
    walk(path, &mut file_count, &mut total_bytes);
    (file_count, total_bytes)
}

// ===========================================================================
// Part 1: Synthetic init scaling
// ===========================================================================

#[divan::bench]
fn init_identity_1_file(bencher: divan::Bencher) {
    let dir = make_mirror_dir(1);
    bencher.bench_local(|| {
        divan::black_box(init_identity(dir.path()));
    });
}

#[divan::bench]
fn init_identity_10_files(bencher: divan::Bencher) {
    let dir = make_mirror_dir(10);
    bencher.bench_local(|| {
        divan::black_box(init_identity(dir.path()));
    });
}

#[divan::bench]
fn init_identity_50_files(bencher: divan::Bencher) {
    let dir = make_mirror_dir(50);
    bencher.bench_local(|| {
        divan::black_box(init_identity(dir.path()));
    });
}

#[divan::bench]
fn init_identity_100_files(bencher: divan::Bencher) {
    let dir = make_mirror_dir(100);
    bencher.bench_local(|| {
        divan::black_box(init_identity(dir.path()));
    });
}

#[divan::bench]
fn session_init(bencher: divan::Bencher) {
    bencher.bench_local(|| {
        let dir = tempfile::tempdir().expect("tempdir");
        divan::black_box(init_identity(dir.path()));
    });
}

// [Seam] Edge cases
#[divan::bench]
fn init_identity_empty_dir(bencher: divan::Bencher) {
    let dir = tempfile::tempdir().expect("tempdir");
    bencher.bench_local(|| {
        divan::black_box(init_identity(dir.path()));
    });
}

#[divan::bench]
fn init_identity_nonexistent(bencher: divan::Bencher) {
    let path = Path::new("/tmp/spectral-bench-nonexistent-dir-12345");
    bencher.bench_local(|| {
        divan::black_box(init_identity(path));
    });
}

// ===========================================================================
// Part 2: Two-tier snapshot isolation (fast vs full on synthetic data)
// ===========================================================================

#[divan::bench(args = [1, 10, 50, 100])]
fn snapshot_fast_only(bencher: divan::Bencher, n: usize) {
    let dir = make_mirror_dir(n);
    let files = scan_mirror_files(dir.path());
    let bytes = serialize_init_state(&files);
    bencher.bench_local(|| {
        // Isolate just the FNV-1a path
        let fast = fnv1a_bench(&bytes);
        divan::black_box(fast);
    });
}

#[divan::bench(args = [1, 10, 50, 100])]
fn snapshot_full_only(bencher: divan::Bencher, n: usize) {
    let dir = make_mirror_dir(n);
    let files = scan_mirror_files(dir.path());
    let bytes = serialize_init_state(&files);
    bencher.bench_local(|| {
        // Isolate just the CoincidenceHash path
        let full = prism::oid::Oid::hash(&bytes);
        divan::black_box(full);
    });
}

#[divan::bench(args = [1, 10, 50, 100])]
fn snapshot_both_tiers(bencher: divan::Bencher, n: usize) {
    let dir = make_mirror_dir(n);
    let files = scan_mirror_files(dir.path());
    let bytes = serialize_init_state(&files);
    bencher.bench_local(|| {
        divan::black_box(InitSnapshot::capture(&bytes));
    });
}

/// Inline FNV-1a for isolated benchmarking (no Oid construction overhead).
fn fnv1a_bench(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;
    let mut hash = FNV_OFFSET;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ===========================================================================
// Part 3: Real repo benchmarks
// ===========================================================================

// The identity repo at /Users/reed/identity/ has .mirror files at the root.
// systemic.engineering at /Users/reed/dev/systemic.engineering/ may or may not.
// We benchmark both paths: init_identity (file discovery + bias chain + snapshot)
// and isolated snapshot (just the hash).

const IDENTITY_REPO: &str = "/Users/reed/identity";
const SYSTEMIC_REPO: &str = "/Users/reed/dev/systemic.engineering";

// --- Identity repo ---

#[divan::bench]
fn bench_init_identity_repo(bencher: divan::Bencher) {
    let path = Path::new(IDENTITY_REPO);
    if !path.exists() { return; }
    bencher.bench_local(|| {
        divan::black_box(init_identity(path));
    });
}

#[divan::bench]
fn bench_fast_snapshot_identity(bencher: divan::Bencher) {
    let path = Path::new(IDENTITY_REPO);
    if !path.exists() { return; }
    let files = scan_mirror_files(path);
    if files.is_empty() { return; }
    let bytes = serialize_init_state(&files);
    eprintln!("[identity] {} .mirror files, {} state bytes", files.len(), bytes.len());
    bencher.bench_local(|| {
        divan::black_box(fnv1a_bench(&bytes));
    });
}

#[divan::bench]
fn bench_full_snapshot_identity(bencher: divan::Bencher) {
    let path = Path::new(IDENTITY_REPO);
    if !path.exists() { return; }
    let files = scan_mirror_files(path);
    if files.is_empty() { return; }
    let bytes = serialize_init_state(&files);
    bencher.bench_local(|| {
        divan::black_box(prism::oid::Oid::hash(&bytes));
    });
}

#[divan::bench]
fn bench_cascade_ratio_identity(bencher: divan::Bencher) {
    let path = Path::new(IDENTITY_REPO);
    if !path.exists() { return; }
    let files = scan_mirror_files(path);
    if files.is_empty() { return; }
    let bytes = serialize_init_state(&files);
    bencher.bench_local(|| {
        let fast_start = Instant::now();
        let _fast = fnv1a_bench(&bytes);
        let fast_ns = fast_start.elapsed().as_nanos() as u64;

        let full_start = Instant::now();
        let _full = prism::oid::Oid::hash(&bytes);
        let full_ns = full_start.elapsed().as_nanos() as u64;

        let ratio = if fast_ns > 0 { full_ns as f64 / fast_ns as f64 } else { 0.0 };
        divan::black_box(ratio);
    });
}

// --- systemic.engineering repo ---

#[divan::bench]
fn bench_init_systemic_engineering(bencher: divan::Bencher) {
    let path = Path::new(SYSTEMIC_REPO);
    if !path.exists() { return; }
    bencher.bench_local(|| {
        divan::black_box(init_identity(path));
    });
}

#[divan::bench]
fn bench_fast_snapshot_systemic(bencher: divan::Bencher) {
    let path = Path::new(SYSTEMIC_REPO);
    if !path.exists() { return; }
    let files = scan_mirror_files(path);
    if files.is_empty() { return; }
    let bytes = serialize_init_state(&files);
    eprintln!("[systemic] {} .mirror files, {} state bytes", files.len(), bytes.len());
    bencher.bench_local(|| {
        divan::black_box(fnv1a_bench(&bytes));
    });
}

#[divan::bench]
fn bench_full_snapshot_systemic(bencher: divan::Bencher) {
    let path = Path::new(SYSTEMIC_REPO);
    if !path.exists() { return; }
    let files = scan_mirror_files(path);
    if files.is_empty() { return; }
    let bytes = serialize_init_state(&files);
    bencher.bench_local(|| {
        divan::black_box(prism::oid::Oid::hash(&bytes));
    });
}

#[divan::bench]
fn bench_cascade_ratio_systemic(bencher: divan::Bencher) {
    let path = Path::new(SYSTEMIC_REPO);
    if !path.exists() { return; }
    let files = scan_mirror_files(path);
    if files.is_empty() { return; }
    let bytes = serialize_init_state(&files);
    bencher.bench_local(|| {
        let fast_start = Instant::now();
        let _fast = fnv1a_bench(&bytes);
        let fast_ns = fast_start.elapsed().as_nanos() as u64;

        let full_start = Instant::now();
        let _full = prism::oid::Oid::hash(&bytes);
        let full_ns = full_start.elapsed().as_nanos() as u64;

        let ratio = if fast_ns > 0 { full_ns as f64 / fast_ns as f64 } else { 0.0 };
        divan::black_box(ratio);
    });
}

// ===========================================================================
// Part 4: Directory scan -- repo structure profiling
// ===========================================================================

#[divan::bench]
fn bench_directory_scan_systemic(bencher: divan::Bencher) {
    let path = Path::new(SYSTEMIC_REPO);
    if !path.exists() { return; }
    bencher.bench_local(|| {
        divan::black_box(scan_directory_stats(path));
    });
}

#[divan::bench]
fn bench_directory_scan_identity(bencher: divan::Bencher) {
    let path = Path::new(IDENTITY_REPO);
    if !path.exists() { return; }
    bencher.bench_local(|| {
        divan::black_box(scan_directory_stats(path));
    });
}

// ===========================================================================
// Part 5: Graph shape comparison (printed to stderr on first run)
// ===========================================================================

/// Not a benchmark -- a one-shot comparison that prints repo fingerprints.
/// Run with `cargo bench -- graph_shape_comparison` to see the output.
#[divan::bench]
fn graph_shape_comparison(bencher: divan::Bencher) {
    let identity_path = Path::new(IDENTITY_REPO);
    let systemic_path = Path::new(SYSTEMIC_REPO);

    // Collect stats once
    let id_files = if identity_path.exists() {
        scan_mirror_files(identity_path)
    } else {
        vec![]
    };
    let sys_files = if systemic_path.exists() {
        scan_mirror_files(systemic_path)
    } else {
        vec![]
    };

    let id_dir_stats = if identity_path.exists() {
        scan_directory_stats(identity_path)
    } else {
        (0, 0)
    };
    let sys_dir_stats = if systemic_path.exists() {
        scan_directory_stats(systemic_path)
    } else {
        (0, 0)
    };

    let id_bytes = serialize_init_state(&id_files);
    let sys_bytes = serialize_init_state(&sys_files);

    // Compute snapshots for comparison
    let id_snap = if !id_files.is_empty() {
        Some(InitSnapshot::capture(&id_bytes))
    } else {
        None
    };
    let sys_snap = if !sys_files.is_empty() {
        Some(InitSnapshot::capture(&sys_bytes))
    } else {
        None
    };

    // Print comparison (once, outside the benchmark loop)
    eprintln!();
    eprintln!("=== Graph Shape Comparison ===");
    eprintln!();
    eprintln!("  identity repo ({}):", IDENTITY_REPO);
    eprintln!("    .mirror files:  {}", id_files.len());
    eprintln!("    state bytes:    {}", id_bytes.len());
    eprintln!("    total files:    {}", id_dir_stats.0);
    eprintln!("    total bytes:    {}", id_dir_stats.1);
    if let Some(ref snap) = id_snap {
        eprintln!("    fast oid:       {}", snap.fast_oid);
        eprintln!("    full oid:       {}", snap.full_oid);
    }
    eprintln!();
    eprintln!("  systemic.engineering ({}):", SYSTEMIC_REPO);
    eprintln!("    .mirror files:  {}", sys_files.len());
    eprintln!("    state bytes:    {}", sys_bytes.len());
    eprintln!("    total files:    {}", sys_dir_stats.0);
    eprintln!("    total bytes:    {}", sys_dir_stats.1);
    if let Some(ref snap) = sys_snap {
        eprintln!("    fast oid:       {}", snap.fast_oid);
        eprintln!("    full oid:       {}", snap.full_oid);
    }
    eprintln!();

    // Density ratio: .mirror files / total files (how identity-dense is the repo?)
    if id_dir_stats.0 > 0 {
        let density = id_files.len() as f64 / id_dir_stats.0 as f64;
        eprintln!("  identity density: {:.4} ({} / {})",
            density, id_files.len(), id_dir_stats.0);
    }
    if sys_dir_stats.0 > 0 {
        let density = sys_files.len() as f64 / sys_dir_stats.0 as f64;
        eprintln!("  systemic density: {:.4} ({} / {})",
            density, sys_files.len(), sys_dir_stats.0);
    }
    eprintln!();
    eprintln!("  state byte ratio: identity/systemic = {:.2}",
        if sys_bytes.len() > 0 { id_bytes.len() as f64 / sys_bytes.len() as f64 } else { 0.0 });
    eprintln!("=== End Comparison ===");
    eprintln!();

    // Benchmark: the comparison itself is trivially fast (just the print)
    bencher.bench_local(|| {
        // Noop -- the real work is the eprintln above
        divan::black_box(());
    });
}

fn main() {
    divan::main();
}
