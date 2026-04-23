// RED stub — implementation pending
use crate::buffer::Buffer;

pub struct Context;

impl Context {
    pub fn new() -> Self { todo!("RED: Context::new not yet implemented") }
    pub fn storage_buffer<T: bytemuck::Pod>(&mut self, _data: &[T]) -> Buffer<T> { todo!() }
    pub fn vertex_buffer<T: bytemuck::Pod>(&mut self, _data: &[T]) -> Buffer<T> { todo!() }
}

impl Default for Context {
    fn default() -> Self { Self::new() }
}
