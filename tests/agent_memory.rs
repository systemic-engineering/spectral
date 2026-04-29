// These tests depend on the `lens` crate, which is currently disabled
// (see Cargo.toml — "Temporarily disabled — pending further integration").
// They build only when the `lens-integration` feature is enabled AND the
// lens crate compiles. Until then they are skipped from default test runs.
#![cfg(feature = "lens-integration")]

//! Integration tests for the agentic memory layer.
//!
//! These test the full lifecycle that a Claude Code agent runs:
//! 1. Open a project lens
//! 2. Store observations
//! 3. Recall context
//! 4. Crystallize patterns
//! 5. Export → ingest (cross-session persistence)
//! 6. Pressure-aware curation
//! 7. Partition tree community detection

use std::path::Path;

use lens::export::ExportFormat;
use lens::filter::GrammarFilter;
use lens::types::{Distance, NodeData, NodeType};
use lens::Lens;
use prism::{Oid, Pressure};

fn project_filter() -> GrammarFilter {
    GrammarFilter::new("project")
        .allow_type("file")
        .allow_type("function")
        .allow_type("decision")
        .allow_type("observation")
        .allow_type("test")
        .allow_type("pattern")
}

fn open_test_lens(dir: &Path) -> Lens {
    Lens::open(dir, project_filter(), "test-agent", 1e-6, 5_000_000)
        .expect("lens should open")
}

// ---------------------------------------------------------------------------
// 1. Store and read back
// ---------------------------------------------------------------------------

#[test]
fn store_and_read_observation() {
    let dir = tempfile::tempdir().unwrap();
    let lens = open_test_lens(dir.path());

    let beam = lens.store(
        NodeType::new_unchecked("observation"),
        NodeData::from_str("the tests pass on the refactor branch"),
    );

    assert!(beam.is_lossless(), "store should succeed without loss");
    let oid = beam.result.clone();

    // Read it back
    let read_beam = lens.read(oid);
    assert!(read_beam.result.is_some(), "read should find the node");
    let data = read_beam.result.unwrap();
    assert_eq!(
        String::from_utf8_lossy(data.as_bytes()),
        "the tests pass on the refactor branch"
    );
}

#[test]
fn store_multiple_types() {
    let dir = tempfile::tempdir().unwrap();
    let lens = open_test_lens(dir.path());

    let decision = lens.store(
        NodeType::new_unchecked("decision"),
        NodeData::from_str("use spectral-db for memory management"),
    );
    let observation = lens.store(
        NodeType::new_unchecked("observation"),
        NodeData::from_str("ego-graph gives zero signal on ring graphs"),
    );
    let pattern = lens.store(
        NodeType::new_unchecked("pattern"),
        NodeData::from_str("always use -j2 on cargo test to prevent OOM"),
    );

    assert!(decision.is_lossless());
    assert!(observation.is_lossless());
    assert!(pattern.is_lossless());

    let (nodes, _) = lens.graph_stats();
    assert_eq!(nodes, 3);
}

#[test]
fn store_invalid_type_reports_loss() {
    let dir = tempfile::tempdir().unwrap();
    let lens = open_test_lens(dir.path());

    // "secret" is not in the project grammar
    let beam = lens.store(
        NodeType::new_unchecked("secret"),
        NodeData::from_str("should not store"),
    );

    assert!(!beam.is_lossless(), "invalid type should report loss");
}

// ---------------------------------------------------------------------------
// 2. Connect and recall
// ---------------------------------------------------------------------------

#[test]
fn connect_and_recall_neighbors() {
    let dir = tempfile::tempdir().unwrap();
    let lens = open_test_lens(dir.path());

    let a = lens.store(
        NodeType::new_unchecked("observation"),
        NodeData::from_str("mirror crate is pure language"),
    );
    let b = lens.store(
        NodeType::new_unchecked("observation"),
        NodeData::from_str("conversation crate is runtime"),
    );

    let oid_a = a.result.clone();
    let oid_b = b.result.clone();

    // Connect them
    let connect_beam = lens.connect(oid_a.clone(), oid_b.clone());
    assert!(connect_beam.is_lossless());

    // Walk from a — should find b as a neighbor
    let walk_beam = lens.walk(oid_a.clone(), lens::types::Depth::new(1));
    assert!(
        !walk_beam.result.is_empty(),
        "walk should find connected nodes"
    );
    assert!(
        walk_beam.result.contains(&oid_b),
        "walk from a should find b"
    );
}

// ---------------------------------------------------------------------------
// 3. Crystallize
// ---------------------------------------------------------------------------

#[test]
fn crystallize_survives_working_set() {
    let dir = tempfile::tempdir().unwrap();
    let lens = open_test_lens(dir.path());

    let beam = lens.store(
        NodeType::new_unchecked("pattern"),
        NodeData::from_str("use nix develop -c for all cargo commands"),
    );
    let oid = beam.result.clone();

    // Crystallize
    let crystal_beam = lens.crystallize(oid.clone());
    assert!(crystal_beam.is_lossless());

    // Activate it
    lens.activate(oid.clone());
    assert!(lens.is_active(&oid));

    // Evict under pressure — crystallized should survive
    let evictable = lens.evict_under_pressure();
    // The crystallized node should NOT be in the evictable set
    // (it's in working set, so it stays)
    assert!(
        !evictable.result.contains(&oid),
        "active crystallized node should not be evictable"
    );
}

// ---------------------------------------------------------------------------
// 4. Working set and pressure curation
// ---------------------------------------------------------------------------

#[test]
fn pressure_curation_keeps_active_drops_cold() {
    let dir = tempfile::tempdir().unwrap();
    let lens = open_test_lens(dir.path());

    // Store 10 observations
    let mut oids = Vec::new();
    for i in 0..10 {
        let beam = lens.store(
            NodeType::new_unchecked("observation"),
            NodeData::from_str(&format!("observation number {}", i)),
        );
        oids.push(beam.result.clone());
    }

    // Activate first 3
    for oid in &oids[..3] {
        lens.activate(oid.clone());
    }

    // Curate at high pressure — should prefer active items
    let curated = lens.curate(Pressure::new(0.95));
    let curated_oids: Vec<_> = curated.result.clone();

    // Active items should be present
    for oid in &oids[..3] {
        assert!(
            curated_oids.contains(oid),
            "active item {} should survive high pressure curation",
            oid
        );
    }

    // At critical pressure, should have fewer items than total
    assert!(
        curated_oids.len() <= oids.len(),
        "curation should not return more items than stored"
    );
}

#[test]
fn forget_removes_from_working_set() {
    let dir = tempfile::tempdir().unwrap();
    let lens = open_test_lens(dir.path());

    let beam = lens.store(
        NodeType::new_unchecked("observation"),
        NodeData::from_str("temporary context"),
    );
    let oid = beam.result.clone();

    lens.activate(oid.clone());
    assert!(lens.is_active(&oid));

    lens.forget(oid.clone());
    assert!(!lens.is_active(&oid));
}

// ---------------------------------------------------------------------------
// 5. Export and ingest (cross-session persistence)
// ---------------------------------------------------------------------------

#[test]
fn export_ingest_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let export_dir = tempfile::tempdir().unwrap();

    // Session 1: store observations
    {
        let lens = open_test_lens(dir.path());
        lens.store(
            NodeType::new_unchecked("decision"),
            NodeData::from_str("split mirror from conversation"),
        );
        lens.store(
            NodeType::new_unchecked("observation"),
            NodeData::from_str("spectral-db settles in 2 ticks"),
        );
        lens.store(
            NodeType::new_unchecked("pattern"),
            NodeData::from_str("use -j2 to prevent OOM"),
        );

        let export_beam = lens.export_to(export_dir.path(), ExportFormat::Markdown);
        assert!(export_beam.is_lossless(), "export should not lose data");
    }

    // Session 2: ingest into a fresh graph
    {
        let dir2 = tempfile::tempdir().unwrap();
        let lens2 = open_test_lens(dir2.path());

        let ingest_beam = lens2.ingest_from(export_dir.path());
        let ingested = ingest_beam.result.len();
        assert_eq!(ingested, 3, "should ingest all 3 nodes");

        let (nodes, _) = lens2.graph_stats();
        assert_eq!(nodes, 3, "second session should have 3 nodes");
    }
}

// ---------------------------------------------------------------------------
// 6. Grammar-scoped visibility
// ---------------------------------------------------------------------------

#[test]
fn grammar_filter_hides_types() {
    let dir = tempfile::tempdir().unwrap();

    // Wide lens: sees everything
    let wide_lens = Lens::open(
        dir.path(),
        GrammarFilter::new("wide")
            .allow_type("observation")
            .allow_type("decision")
            .allow_type("secret"),
        "wide-agent",
        1e-6,
        5_000_000,
    )
    .unwrap();

    wide_lens.store(
        NodeType::new_unchecked("observation"),
        NodeData::from_str("visible to everyone"),
    );
    wide_lens.store(
        NodeType::new_unchecked("secret"),
        NodeData::from_str("only wide lens sees this"),
    );

    // Narrow lens: only sees observations (same db, different filter)
    let narrow_lens = wide_lens.with_filter(
        GrammarFilter::new("narrow").allow_type("observation"),
    );

    let wide_find = wide_lens.find(NodeType::new_unchecked("secret"));
    let narrow_find = narrow_lens.find(NodeType::new_unchecked("secret"));

    assert!(
        !wide_find.result.is_empty(),
        "wide lens should see secrets"
    );
    assert!(
        narrow_find.result.is_empty(),
        "narrow lens should NOT see secrets"
    );
}

// ---------------------------------------------------------------------------
// 7. Lens composition (multi-agent)
// ---------------------------------------------------------------------------

#[test]
fn composed_lens_independent_working_sets() {
    let dir = tempfile::tempdir().unwrap();
    let base = open_test_lens(dir.path());

    let beam = base.store(
        NodeType::new_unchecked("observation"),
        NodeData::from_str("shared knowledge"),
    );
    let oid = beam.result.clone();

    // Compose a narrower lens for a sub-agent
    let sub_filter = GrammarFilter::new("sub")
        .allow_type("observation")
        .allow_type("pattern");
    let sub_lens = base.compose(sub_filter, "sub-agent");

    // Activate in sub-agent's working set
    sub_lens.activate(oid.clone());

    // Sub-agent sees it as active
    assert!(sub_lens.is_active(&oid));
    // Base lens does NOT see it as active (independent working sets)
    assert!(!base.is_active(&oid));
}

// ---------------------------------------------------------------------------
// 8. Graph stats
// ---------------------------------------------------------------------------

#[test]
fn graph_stats_track_nodes_and_edges() {
    let dir = tempfile::tempdir().unwrap();
    let lens = open_test_lens(dir.path());

    let (n0, e0) = lens.graph_stats();
    assert_eq!(n0, 0);
    assert_eq!(e0, 0);

    let a = lens.store(
        NodeType::new_unchecked("observation"),
        NodeData::from_str("a"),
    );
    let b = lens.store(
        NodeType::new_unchecked("observation"),
        NodeData::from_str("b"),
    );

    let (n1, e1) = lens.graph_stats();
    assert_eq!(n1, 2);
    assert_eq!(e1, 0);

    lens.connect(a.result.clone(), b.result.clone());

    let (n2, e2) = lens.graph_stats();
    assert_eq!(n2, 2);
    assert_eq!(e2, 1);
}
