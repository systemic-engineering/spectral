// spectral-ui: GPU rendering layer for the spectral eigenboard.
//
// The GPU holds the superposition. The CPU collapses it.
// That's the architecture.
//
// Apache-2.0

pub mod context;
pub mod buffer;
pub mod program;
pub mod pass;
pub mod mote;
pub mod field;
pub mod superposition;

pub use context::Context;
pub use buffer::Buffer;
pub use program::Program;
pub use pass::RenderPass;
pub use mote::Mote;
pub use field::{Field, Arc};
pub use superposition::{DeviceState, Snapshot, SpectralGpu};
