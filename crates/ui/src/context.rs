// context.rs — wgpu device/queue/instance, headless by default.

use wgpu::{
    Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor,
    Limits, PowerPreference, Queue, RequestAdapterOptions,
};

use crate::buffer::Buffer;

/// Owns the wgpu instance, adapter, device, and queue.
/// Created headless (no window surface) by default, making it safe for testing.
pub struct Context {
    /// Kept alive: wgpu instance must outlive the device. Not read directly.
    #[allow(dead_code)]
    pub(crate) instance: Instance,
    pub(crate) device: Device,
    pub(crate) queue: Queue,
}

impl Context {
    /// Create a headless context backed by whatever adapter wgpu can find.
    /// On macOS this is Metal; on Linux it may be Vulkan or DX12; on CI
    /// the `WGPU_BACKEND=gl` env var enables the GLES software path.
    pub fn new() -> Self {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Self {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::None,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("spectral-ui: no wgpu adapter found — set WGPU_BACKEND=gl for software fallback");

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("spectral-ui"),
                    required_features: Features::empty(),
                    required_limits: Limits::downlevel_defaults(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("spectral-ui: device creation failed");

        Self { instance, device, queue }
    }

    /// Allocate a storage buffer on the GPU and upload `data`.
    pub fn storage_buffer<T: bytemuck::Pod>(&mut self, data: &[T]) -> Buffer<T> {
        Buffer::new_storage(&self.device, data)
    }

    /// Allocate a vertex buffer on the GPU and upload `data`.
    pub fn vertex_buffer<T: bytemuck::Pod>(&mut self, data: &[T]) -> Buffer<T> {
        Buffer::new_vertex(&self.device, data)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
