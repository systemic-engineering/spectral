// RED stub
#[derive(Clone, Debug, PartialEq)]
pub struct Mote {
    pub position:    [f32; 2],
    pub radius:      f32,
    pub color:       [f32; 4],
    pub glow_radius: f32,
    pub energy:      f32,
}
