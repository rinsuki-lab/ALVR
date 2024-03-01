use crate::GraphicsContext;
use alvr_common::anyhow::Result;
use rend3::{
    graph::RenderGraph, types::Handedness, ExtendedAdapterInfo, InstanceAdapterDevice, Renderer,
    RendererProfile, ShaderPreProcessor, Vendor,
};
use std::sync::Arc;
use wgpu::Texture;

struct LobbyRenderer {
    renderer: Renderer,
    swapchain: Vec<Texture>,
}

impl LobbyRenderer {
    pub fn new<T: Clone>(
        graphics_context: GraphicsContext<T>,
        swapchain: Vec<Texture>,
    ) -> Result<Self> {
        let iad = InstanceAdapterDevice {
            instance: Arc::clone(&graphics_context.instance),
            adapter: Arc::clone(&graphics_context.adapter),
            device: Arc::clone(&graphics_context.device),
            queue: Arc::clone(&graphics_context.queue),
            profile: RendererProfile::CpuDriven,
            info: ExtendedAdapterInfo {
                name: graphics_context.adapter.get_info().name,
                vendor: match graphics_context.adapter.get_info().vendor {
                    0x1002 => Vendor::Amd,
                    0x10DE => Vendor::Nv,
                    0x13B5 => Vendor::Arm,
                    0x1414 => Vendor::Microsoft,
                    0x14E4 => Vendor::Broadcom,
                    0x5143 => Vendor::Qualcomm,
                    0x8086 => Vendor::Intel,
                    v => Vendor::Unknown(v as usize),
                },
                device: graphics_context.adapter.get_info().device as usize,
                device_type: graphics_context.adapter.get_info().device_type,
                backend: graphics_context.adapter.get_info().backend,
            },
        };

        let renderer = Renderer::new(iad, Handedness::Right, None)?;

        let shader_preprocessor = ShaderPreProcessor::new();
        // rend3_rou

        let fnjdks = surface.get_current_texture().unwrap();

        let graph = RenderGraph::new();

        graph.add_imported_render_target(texture, layers, mips, viewport);

        Ok(Self {})
    }

    pub fn render(&self, swapchain_index: usize) {
        let mut graph = RenderGraph::new();
        graph.add_imported_render_target(self.swapchain[swapchain_index], 0..2, 0..1, viewport)

    }
}
