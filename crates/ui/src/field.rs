// RED stub
use crate::mote::Mote;

#[derive(Clone, Debug)]
pub struct Arc {
    pub from:     usize,
    pub to:       usize,
    pub strength: f32,
}

pub struct Field {
    pub motes:      Vec<Mote>,
    pub arcs:       Vec<Arc>,
    pub viewer_idx: usize,
}

impl Field {
    pub fn render(&self, _ctx: &mut crate::context::Context) -> Vec<u8> {
        todo!("RED: Field::render not yet implemented")
    }
}
