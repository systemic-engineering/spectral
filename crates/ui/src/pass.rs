// pass.rs — render pass execution and pixel readback.

use wgpu::{
    Device, Extent3d, ImageCopyBuffer, ImageCopyTexture,
    ImageDataLayout, Origin3d, Queue,
    Texture, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages,
    BufferUsages, BufferDescriptor,
};

pub use wgpu::RenderPass;

/// The output texture used for headless rendering.
/// 512x512 RGBA8 — enough resolution to test blending while keeping tests fast.
pub const RENDER_WIDTH: u32 = 512;
pub const RENDER_HEIGHT: u32 = 512;
pub const RENDER_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

/// Allocate a render target texture.
pub fn make_render_texture(device: &Device) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some("spectral-ui render target"),
        size: Extent3d {
            width: RENDER_WIDTH,
            height: RENDER_HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: RENDER_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// Read a rendered texture back to CPU bytes (RGBA8, row-major).
pub fn readback_texture(device: &Device, queue: &Queue, texture: &Texture) -> Vec<u8> {
    // wgpu requires rows to be aligned to 256 bytes
    let bytes_per_pixel = 4u32;
    let unpadded_row = RENDER_WIDTH * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_row = (unpadded_row + align - 1) / align * align;

    let staging = device.create_buffer(&BufferDescriptor {
        label: Some("spectral-ui readback staging"),
        size: (padded_row * RENDER_HEIGHT) as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("spectral-ui readback encoder"),
    });
    encoder.copy_texture_to_buffer(
        ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        ImageCopyBuffer {
            buffer: &staging,
            layout: ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(RENDER_HEIGHT),
            },
        },
        Extent3d { width: RENDER_WIDTH, height: RENDER_HEIGHT, depth_or_array_layers: 1 },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().expect("texture readback map failed");

    let data = slice.get_mapped_range();
    // Strip row padding: collect only the unpadded portion of each row
    let mut out = Vec::with_capacity((unpadded_row * RENDER_HEIGHT) as usize);
    for row in 0..RENDER_HEIGHT as usize {
        let start = row * padded_row as usize;
        let end = start + unpadded_row as usize;
        out.extend_from_slice(&data[start..end]);
    }
    drop(data);
    staging.unmap();
    out
}
