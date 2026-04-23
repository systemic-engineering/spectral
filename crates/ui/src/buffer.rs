// RED stub
use std::marker::PhantomData;

pub struct Buffer<T: bytemuck::Pod> {
    _marker: PhantomData<T>,
}

impl<T: bytemuck::Pod> Buffer<T> {
    pub fn len(&self) -> usize { todo!() }
    pub fn is_empty(&self) -> bool { todo!() }
    pub fn read_back(&self, _ctx: &crate::context::Context) -> Vec<T> { todo!() }
}
