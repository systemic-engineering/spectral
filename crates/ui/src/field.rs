// field.rs — Field: collection of Motes + coupling arcs, full scene renderer.

use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindingResource, BufferUsages, Color,
    CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureViewDescriptor,
};

use crate::{
    context::Context,
    mote::{Mote, MoteGpu},
    pass::{make_render_texture, readback_texture, RENDER_FORMAT},
    program::{Program, ProgramKind},
};

/// A directed coupling arc between two motes.
#[derive(Clone, Debug)]
pub struct Arc {
    /// Index into `Field::motes` for the start of the arc.
    pub from: usize,
    /// Index into `Field::motes` for the end of the arc.
    pub to: usize,
    /// Coupling strength: drives arc opacity and width. Range [0, 1].
    pub strength: f32,
}

/// GPU layout for one arc.
/// std430: vec2=8, f32=4. Total: 8+8+4+12pad = 32 bytes. Align=8. 32=4×8.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ArcGpu {
    pub from_pos:  [f32; 2],   // offset  0
    pub to_pos:    [f32; 2],   // offset  8
    pub strength:  f32,         // offset 16
    pub _pad0:     f32,         // offset 20
    pub _pad1:     f32,         // offset 24
    pub _pad2:     f32,         // offset 28 → total 32
}

/// The complete eigenboard scene: motes, arcs, and viewport config.
pub struct Field {
    /// The glowing nodes to render.
    pub motes: Vec<Mote>,
    /// Coupling arcs between motes.
    pub arcs: Vec<Arc>,
    /// Which mote index is "the player" (reserved for camera follow in windowed mode).
    pub viewer_idx: usize,
}

impl Field {
    /// Render the field to an RGBA8 byte buffer (headless, 512×512).
    ///
    /// Returns `Vec<u8>` in row-major RGBA order, length = width × height × 4.
    pub fn render(&self, ctx: &mut Context) -> Vec<u8> {
        let device = &ctx.device;
        let queue = &ctx.queue;

        let mote_gpu_data: Vec<MoteGpu> = self.motes.iter().map(MoteGpu::from).collect();
        let arc_gpu_data: Vec<ArcGpu> = self.arcs.iter().map(|a| {
            let from_pos = self.motes.get(a.from).map(|m| m.position).unwrap_or([0.0, 0.0]);
            let to_pos   = self.motes.get(a.to).map(|m| m.position).unwrap_or([0.0, 0.0]);
            ArcGpu { from_pos, to_pos, strength: a.strength, _pad0: 0.0, _pad1: 0.0, _pad2: 0.0 }
        }).collect();

        let texture = make_render_texture(device);
        let view = texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("spectral-ui field render"),
        });

        // Clear to transparent black (all channels 0, including alpha).
        // Additive blending accumulates on top of this baseline.
        let transparent_black = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
        {
            let _clear = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("spectral-ui clear"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations { load: LoadOp::Clear(transparent_black), store: StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        // Arc pass (drawn first, under motes)
        if !arc_gpu_data.is_empty() {
            let arc_program = Program::new(device, ProgramKind::Arc, RENDER_FORMAT);
            let arc_buf = Self::upload_storage(device, &arc_gpu_data);
            let arc_bind = device.create_bind_group(&BindGroupDescriptor {
                label: Some("spectral-ui arcs"),
                layout: &arc_program.bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(arc_buf.as_entire_buffer_binding()),
                }],
            });
            {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("spectral-ui arc pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(&arc_program.pipeline);
                pass.set_bind_group(0, &arc_bind, &[]);
                pass.draw(0..((arc_gpu_data.len() as u32) * 6), 0..1);
            }
        }

        // Mote pass (drawn on top)
        if !mote_gpu_data.is_empty() {
            let mote_program = Program::new(device, ProgramKind::Mote, RENDER_FORMAT);
            let mote_buf = Self::upload_storage(device, &mote_gpu_data);
            let mote_bind = device.create_bind_group(&BindGroupDescriptor {
                label: Some("spectral-ui motes"),
                layout: &mote_program.bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(mote_buf.as_entire_buffer_binding()),
                }],
            });
            {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("spectral-ui mote pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(&mote_program.pipeline);
                pass.set_bind_group(0, &mote_bind, &[]);
                // 6 vertices per mote (one quad = 2 triangles)
                pass.draw(0..((mote_gpu_data.len() as u32) * 6), 0..1);
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
        readback_texture(device, queue, &texture)
    }

    fn upload_storage<T: bytemuck::Pod>(device: &wgpu::Device, data: &[T]) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("spectral-ui scene data"),
            contents: bytemuck::cast_slice(data),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        })
    }
}
