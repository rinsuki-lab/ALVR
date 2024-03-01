use crate::{
    client_stream_pass::ClientStreamPass, convert, srgb_pass::SrgbPass, GlBackend, GraphicsContext,
};
use alvr_common::{anyhow::Result, glam::UVec2};
use std::{ffi::c_void, rc::Rc};
use wgpu::CommandEncoderDescriptor;

pub struct ClientStreamRenderer<T: Clone> {
    context: GraphicsContext<T>,
    input_swapchain_len: usize,
    srgb_pass: Option<SrgbPass>,
    client_stream_pass: ClientStreamPass,
}

impl ClientStreamRenderer<GlBackend> {
    pub fn new_gl(
        context: GraphicsContext<GlBackend>,
        input_swapchain_len: usize,
        output_view_resolution: UVec2,
        output_swapchain_gl: &[u32],
        skip_srgb_correction: bool,
    ) -> Result<Self> {
        let output_swapchain = output_swapchain_gl
            .iter()
            .map(|&texture| {
                convert::create_texture_from_gles(&context.device, texture, output_view_resolution)
            })
            .collect::<Vec<_>>();

        let client_stream_pass = ClientStreamPass::new(
            &context.device,
            input_swapchain_len,
            output_view_resolution,
            &output_swapchain,
        );

        let staging_swapchain = (0..output_swapchain_gl.len())
            .map(|idx| client_stream_pass.get_input_texture(idx))
            .collect::<Vec<_>>();

        let srgb_pass = SrgbPass::new(
            Rc::clone(&context.backend_handles.gl_context),
            &staging_swapchain,
            output_view_resolution, // todo: shoud get from client_stream_pass
            skip_srgb_correction,
        )?;

        Ok(Self {
            context: context.clone(),
            input_swapchain_len,
            srgb_pass: Some(srgb_pass),
            client_stream_pass,
        })
    }

    pub fn input_swapchain_gl(&self) -> Vec<u32> {
        if let Some(pass) = &self.srgb_pass {
            (0..self.input_swapchain_len)
                .map(|idx| pass.get_input_texture(idx).0.get())
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    }

    /// # Safety
    /// if buffer must either be null or point to a valid AHardwareBuffer
    pub unsafe fn render_from_buffer_gl(
        &mut self,
        buffer: *const c_void,
        input_swapchain_index: usize,
        output_swapchain_index: usize,
    ) {
        let egl_image = if !buffer.is_null() {
            if let Some(pass) = &self.srgb_pass {
                let gl_input_tex = pass.get_input_texture(input_swapchain_index);

                Some(
                    self.context
                        .bind_ahardwarebuffer_to_gl_ext_texture(buffer, gl_input_tex),
                )
            } else {
                None
            }
        } else {
            None
        };

        self.render_gl(
            input_swapchain_index,
            output_swapchain_index,
            buffer.is_null(),
        );

        if let Some(image) = egl_image {
            unsafe { self.context.destroy_image(image) };
        }
    }

    pub fn render_gl(
        &mut self,
        input_swapchain_index: usize,
        output_swapchain_index: usize,
        rerender_last: bool,
    ) {
        if !rerender_last {
            if let Some(pass) = self.srgb_pass.take() {
                pass.render(input_swapchain_index)
            }
        }

        self.render_no_color_correction(input_swapchain_index, output_swapchain_index)
    }
}

impl<T: Clone> ClientStreamRenderer<T> {
    pub fn render_no_color_correction(
        &mut self,
        input_swapchain_index: usize,
        output_swapchain_index: usize,
    ) {
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        self.client_stream_pass
            .render(&mut encoder, input_swapchain_index, output_swapchain_index);

        self.context.queue.submit(Some(encoder.finish()));
    }
}
