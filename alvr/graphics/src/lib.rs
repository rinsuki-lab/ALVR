mod client_stream_pass;
mod client_stream_renderer;
mod convert;
mod srgb_pass;
mod lobby;

pub use convert::*;
pub use wgpu;
pub use client_stream_renderer::*;

use std::sync::Arc;
use wgpu::*;

#[derive(Clone)]
pub struct GraphicsContext<B: Clone = ()> {
    pub instance: Arc<Instance>,
    pub adapter: Arc<Adapter>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub backend_handles: B,
}