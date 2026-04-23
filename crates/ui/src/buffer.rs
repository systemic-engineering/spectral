// buffer.rs — typed GPU buffer (vertex, storage, uniform).

use std::marker::PhantomData;
use wgpu::{BufferDescriptor, BufferUsages, Device, MapMode};

/// A typed GPU buffer.
/// `T` must implement `bytemuck::Pod` so we can safely transmute slices.
pub struct Buffer<T: bytemuck::Pod> {
    pub(crate) raw: wgpu::Buffer,
    pub(crate) len: usize,
    _marker: PhantomData<T>,
}

impl<T: bytemuck::Pod> Buffer<T> {
    pub(crate) fn new_storage(device: &Device, data: &[T]) -> Self {
        use wgpu::util::DeviceExt;
        let raw = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("spectral-ui storage"),
            contents: bytemuck::cast_slice(data),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });
        Self { raw, len: data.len(), _marker: PhantomData }
    }

    pub(crate) fn new_vertex(device: &Device, data: &[T]) -> Self {
        use wgpu::util::DeviceExt;
        let raw = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("spectral-ui vertex"),
            contents: bytemuck::cast_slice(data),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });
        Self { raw, len: data.len(), _marker: PhantomData }
    }

    /// Number of elements in the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Read the buffer contents back to the CPU.
    /// Creates a staging buffer, copies GPU→staging, maps and reads.
    pub fn read_back(&self, ctx: &crate::context::Context) -> Vec<T> {
        let device = &ctx.device;
        let queue = &ctx.queue;

        let byte_size = (self.len * std::mem::size_of::<T>()) as u64;
        if byte_size == 0 {
            return Vec::new();
        }

        let staging = device.create_buffer(&BufferDescriptor {
            label: Some("spectral-ui staging"),
            size: byte_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("spectral-ui readback"),
        });
        encoder.copy_buffer_to_buffer(&self.raw, 0, &staging, 0, byte_size);
        queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(MapMode::Read, move |v| tx.send(v).unwrap());
        device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().expect("buffer map failed");

        let data = slice.get_mapped_range();
        let result: Vec<T> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging.unmap();
        result
    }
}
