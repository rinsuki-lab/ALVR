use alvr_common::glam::UVec2;
use std::num::NonZeroU32;
use wgpu::*;

struct InputResource {
    input: Texture,
    bind_group: BindGroup,
}

// This pass contains FFR, and eye split (everything except sRGB correction)
pub struct ClientStreamPass {
    pipeline: RenderPipeline,
    input_resource_swapchain: Vec<InputResource>,
    output_swapchain: Vec<TextureView>,
}

impl ClientStreamPass {
    // note: swapchain must be of size 3 and contain texture arrays of depth 2
    pub fn new(
        device: &Device,
        input_swapchain_len: usize,
        output_view_resolution: UVec2,
        output_swapchain: &[Texture],
    ) -> Self {
        let label = Some("stream_pass");

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label,
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &device.create_shader_module(ShaderModuleDescriptor {
                    label,
                    source: ShaderSource::Wgsl(
                        include_str!("../shaders/client/render_vert.wgsl").into(),
                    ),
                }),
                entry_point: "main",
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &device.create_shader_module(ShaderModuleDescriptor {
                    label,
                    source: ShaderSource::Wgsl(
                        include_str!("../shaders/client/render_frag.wgsl").into(),
                    ),
                }),
                entry_point: "main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: Some(NonZeroU32::new(2).unwrap()),
        });

        let input_resource_swapchain = (0..input_swapchain_len)
            .map(|_| {
                let input = device.create_texture(&TextureDescriptor {
                    label,
                    // todo: size should be derived
                    size: Extent3d {
                        width: output_view_resolution.x,
                        height: output_view_resolution.y,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });

                let bind_group = device.create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &bind_group_layout,
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(
                            &input.create_view(&Default::default()),
                        ),
                    }],
                });

                InputResource { input, bind_group }
            })
            .collect();

        let output_swapchain = output_swapchain
            .iter()
            .map(|output| output.create_view(&Default::default()))
            .collect();

        Self {
            pipeline,
            input_resource_swapchain,
            output_swapchain,
        }
    }

    pub fn get_input_texture(&self, swapchain_index: usize) -> &Texture {
        &self.input_resource_swapchain[swapchain_index].input
    }

    pub fn render(
        &self,
        encoder: &mut CommandEncoder,
        input_swapchain_index: usize,
        output_swapchain_index: usize,
    ) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.output_swapchain[output_swapchain_index],
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(
            0,
            &self.input_resource_swapchain[input_swapchain_index].bind_group,
            &[],
        );
        // pass.set_push_constants(ShaderStages::VERTEX, offset, data)
        pass.draw(0..4, 0..1)
    }
}
