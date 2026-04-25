// Integration tests for spectral-ui
// RED phase: these tests define the contract before implementation exists.
// They will compile-fail / panic until the GREEN phase is complete.

use spectral_ui::{Arc, Context, Field, Mote};

// Test 1: Context::new() completes without panicking (headless)
#[test]
fn context_new_headless_no_panic() {
    let ctx = Context::new();
    // If we get here, the headless adapter was found and device was created.
    // On CI without GPU, wgpu falls back to the Vulkan/Metal null backend or
    // WGPU_ADAPTER_NAME env can force a software adapter.
    drop(ctx);
}

// Test 2: storage_buffer round-trips bytes through GPU
#[test]
fn storage_buffer_roundtrip() {
    let mut ctx = Context::new();
    let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
    let buf = ctx.storage_buffer(&data);
    let out = buf.read_back(&ctx);
    assert_eq!(out.len(), data.len(), "roundtrip length mismatch");
    for (a, b) in data.iter().zip(out.iter()) {
        assert!((a - b).abs() < 1e-6, "roundtrip value mismatch: {a} vs {b}");
    }
}

// Test 3: Mote renders to a non-empty pixel buffer (headless render_texture)
#[test]
fn mote_renders_to_pixel_buffer() {
    let mut ctx = Context::new();
    let mote = Mote {
        position: [0.0, 0.0],
        radius: 0.3,
        color: [0.2, 0.8, 1.0, 1.0],
        glow_radius: 0.5,
        energy: 1.0,
    };
    let field = Field {
        motes: vec![mote],
        arcs: vec![],
        viewer_idx: 0,
    };
    let pixels = field.render(&mut ctx);
    assert!(!pixels.is_empty(), "pixel buffer must be non-empty");
    // Rendered image should have at least some non-zero pixels (the mote itself)
    let has_nonzero = pixels.iter().any(|&b| b > 0);
    assert!(has_nonzero, "rendered mote must produce non-zero pixels");
}

// Test 4: Field with 3 motes renders without panic
#[test]
fn field_three_motes_no_panic() {
    let mut ctx = Context::new();
    let field = Field {
        motes: vec![
            Mote { position: [-0.5, 0.0], radius: 0.2, color: [1.0, 0.2, 0.2, 1.0], glow_radius: 0.35, energy: 0.8 },
            Mote { position: [0.0, 0.5], radius: 0.15, color: [0.2, 1.0, 0.2, 1.0], glow_radius: 0.3, energy: 0.6 },
            Mote { position: [0.5, -0.3], radius: 0.25, color: [0.2, 0.2, 1.0, 1.0], glow_radius: 0.4, energy: 0.9 },
        ],
        arcs: vec![
            Arc { from: 0, to: 1, strength: 0.5 },
            Arc { from: 1, to: 2, strength: 0.3 },
        ],
        viewer_idx: 0,
    };
    let pixels = field.render(&mut ctx);
    assert!(!pixels.is_empty(), "3-mote field must produce a pixel buffer");
}

// Test 5: Additive blending — two overlapping motes produce brighter result
// than either mote individually.
#[test]
fn additive_blending_two_motes_brighter() {
    let mut ctx = Context::new();

    // Single mote at center
    let single = Field {
        motes: vec![Mote {
            position: [0.0, 0.0],
            radius: 0.3,
            color: [0.5, 0.5, 0.5, 1.0],
            glow_radius: 0.5,
            energy: 0.5,
        }],
        arcs: vec![],
        viewer_idx: 0,
    };
    let single_pixels = single.render(&mut ctx);

    // Two identical overlapping motes at center
    let double = Field {
        motes: vec![
            Mote { position: [0.0, 0.0], radius: 0.3, color: [0.5, 0.5, 0.5, 1.0], glow_radius: 0.5, energy: 0.5 },
            Mote { position: [0.0, 0.0], radius: 0.3, color: [0.5, 0.5, 0.5, 1.0], glow_radius: 0.5, energy: 0.5 },
        ],
        arcs: vec![],
        viewer_idx: 0,
    };
    let double_pixels = double.render(&mut ctx);

    // Sum of pixel values in the double render should exceed single
    let single_sum: u64 = single_pixels.iter().map(|&b| b as u64).sum();
    let double_sum: u64 = double_pixels.iter().map(|&b| b as u64).sum();
    assert!(
        double_sum > single_sum,
        "additive blending: two overlapping motes ({double_sum}) should be brighter than one ({single_sum})"
    );
}

// Test 6: Empty field renders a fully-black (or near-black) buffer
#[test]
fn empty_field_renders_black() {
    let mut ctx = Context::new();
    let field = Field { motes: vec![], arcs: vec![], viewer_idx: 0 };
    let pixels = field.render(&mut ctx);
    assert!(!pixels.is_empty(), "even empty field must return a buffer");
    let max_val = pixels.iter().copied().max().unwrap_or(0);
    assert!(max_val < 10, "empty field should render near-black, got max pixel {max_val}");
}
