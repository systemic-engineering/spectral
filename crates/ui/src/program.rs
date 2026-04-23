// program.rs — WGSL shader compilation + pipeline state.

use wgpu::{
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType,
    ColorTargetState, ColorWrites, Device, FragmentState, PipelineLayoutDescriptor,
    RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource,
    ShaderStages, TextureFormat, VertexState,
    BlendState, BlendComponent, BlendFactor, BlendOperation,
};

/// A compiled WGSL shader program with its render pipeline.
pub struct Program {
    pub(crate) pipeline: RenderPipeline,
    pub(crate) bind_group_layout: BindGroupLayout,
}

/// Whether this program is for motes or arcs.
pub enum ProgramKind {
    Mote,
    Arc,
}

impl Program {
    /// Compile a WGSL program for the given kind.
    /// `format` is the target texture format (e.g. `Rgba8Unorm`).
    pub fn new(device: &Device, kind: ProgramKind, format: TextureFormat) -> Self {
        let (source, label) = match kind {
            ProgramKind::Mote => (include_str!("wgsl/mote.wgsl"), "mote"),
            ProgramKind::Arc => (include_str!("wgsl/arc.wgsl"), "arc"),
        };

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label),
            source: ShaderSource::Wgsl(source.into()),
        });

        // Storage buffer binding for mote/arc data
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Additive blending: src + dst (no alpha weighting on dst).
        // This makes overlapping motes accumulate brightness naturally.
        let additive = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        };

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[], // data comes from storage buffer, not vertex buffer
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(additive),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self { pipeline, bind_group_layout }
    }
}
