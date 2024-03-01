use alvr_common::{anyhow::Result, glam::UVec2, ToAny };
use alvr_graphics::{ClientStreamRenderer, GlBackend, GraphicsContext};
use std::{cell::RefCell, ffi::c_void};
use alvr_session::FoveatedEncodingConfig;

// ---- OpenGL ----

thread_local! {
    pub static GRAPHICS_CONTEXT_GL: RefCell<Option<GraphicsContext<GlBackend>>> = RefCell::new(None);
    pub static CLIENT_STREAM_RENDERER_GL: RefCell<Option<ClientStreamRenderer<GlBackend>>> = RefCell::new(None);
}

pub fn initialize_gl() {
    GRAPHICS_CONTEXT_GL.with_borrow_mut(|ctx| *ctx = Some(GraphicsContext::new_gles()));
}

pub fn resume_gl(preferred_view_resolution: UVec2, swapchain_textures: &[u32]) -> Result<()> {
    Ok(())
}

pub fn initialize_stream_renderer_gl(
    input_swapchain_len: usize,
    output_view_resolution: UVec2,
    output_swapchain_gl: &[u32],
    foveation: Option<FoveatedEncodingConfig>, // todo
    skip_srgb_correction: bool,
) -> Result<()> {
    CLIENT_STREAM_RENDERER_GL.with_borrow_mut(|renderer| -> Result<()> {
        *renderer = Some(ClientStreamRenderer::new_gl(
            GRAPHICS_CONTEXT_GL
                .with_borrow(|ctx| ctx.as_ref().cloned())
                .to_any()?,
            input_swapchain_len,
            output_view_resolution,
            output_swapchain_gl,
            skip_srgb_correction,
        )?);

        Ok(())
    })
}

pub fn input_swapchain_gl() -> Vec<u32> {
    CLIENT_STREAM_RENDERER_GL.with_borrow(|renderer| {
        renderer
            .as_ref()
            .map(|renderer| renderer.input_swapchain_gl())
            .unwrap_or_default()
    })
}

pub fn render_gl(
    input_swapchain_index: usize,
    output_swapchain_index: usize,
    rerender_last: bool, // AKA reproject
) {
    CLIENT_STREAM_RENDERER_GL.with_borrow_mut(|renderer| {
        if let Some(renderer) = renderer.as_mut() {
            renderer.render_gl(input_swapchain_index, output_swapchain_index, rerender_last)
        }
    })
}

pub fn render_no_color_correction_gl(input_swapchain_index: usize, output_swapchain_index: usize) {
    CLIENT_STREAM_RENDERER_GL.with_borrow_mut(|renderer| {
        if let Some(renderer) = renderer.as_mut() {
            renderer.render_no_color_correction(input_swapchain_index, output_swapchain_index)
        }
    })
}

pub unsafe fn render_from_buffer_gl(
    buffer: *const c_void,
    input_swapchain_index: usize,
    output_swapchain_index: usize,
) {
    CLIENT_STREAM_RENDERER_GL.with_borrow_mut(|renderer| {
        if let Some(renderer) = renderer.as_mut() {
            renderer.render_from_buffer_gl(buffer, input_swapchain_index, output_swapchain_index);
        }
    })
}

pub fn destroy_gl() {
    CLIENT_STREAM_RENDERER_GL.with_borrow_mut(|renderer| *renderer = None);
    GRAPHICS_CONTEXT_GL.with_borrow_mut(|context| *context = None);
}
