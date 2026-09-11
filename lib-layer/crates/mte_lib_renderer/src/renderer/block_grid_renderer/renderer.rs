use std::iter;

use anyhow::Result;
use glam::Mat4;
use mte_macros::{vfs_include_bytes, vfs_include_str};
use wgpu::util::DeviceExt;
use wgpu::wgt::DrawIndexedIndirectArgs;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, BufferDescriptor, PipelineLayout, RenderPipeline, ShaderModule, TextureFormat, TextureView, VertexBufferLayout
};

use super::render_objects::texture::{DepthTexture, TextureArray2D};
use super::types::BlockVertex;
use crate::context::gpu_context::GpuContext;
use crate::context::render_context::RenderContext;
use crate::renderer::block_grid_renderer::inditect_buffer_manager::{
    BufferType, IndirectBufferManager
};
use crate::renderer::block_grid_renderer::render_objects::camera::{
    RotatableLens, WorldCameraUniform
};
use crate::renderer::block_grid_renderer::render_objects::outline::OutlineUniform;
use crate::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use crate::renderer::block_grid_renderer::types::vertex::SimpleVertex;

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub enum PipelineType {
    Shadow,
    Opaque,
    Skybox,
    Light,
    Quad,
    Text,
    Transparent,
}

pub enum BindGroupType {
    Texture,
    Camera,
    Light,
    Environment,
}

pub struct Renderer {
    pub render_context: RenderContext,

    pipelines: [RenderPipeline; 2],

    depth_texture_view: TextureView,

    pub block_static_render_data: BlockStaticRenderData,

    pub pass_1_perframe_bg: BindGroup,
    pub camera_buffer: Buffer,

    pub pass_2_resize_bg: BindGroup,
    pub proj_buffer: Buffer,

    pub matrix_bg: BindGroup,
    pub vector_bg: BindGroup,

    pub buffer_manager: IndirectBufferManager,
}

impl Renderer {
    pub async fn new(render_context: RenderContext, args: RendererCreateArgs) -> Result<Renderer> {
        let gpu_context = &render_context.gpu_contexts[0];
        let window_context = &render_context.window_contexts[0];

        let buffer_manager = IndirectBufferManager::new(gpu_context);

        let (block_data, block_data_bgl) = BlockStaticRenderData::new(&gpu_context, args);

        let opaque_shader_code =
            vfs_include_str!("workspace://game-layer/assets/minecraft/shaders/opaque_new.wgsl");
        let opaque_shader = gpu_context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("World Shader"),
                source: wgpu::ShaderSource::Wgsl(opaque_shader_code.into()),
            });

        let hand_shader_code =
            vfs_include_str!("workspace://game-layer/assets/minecraft/shaders/hand.wgsl");
        let hand_shader = gpu_context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Hand Shader"),
                source: wgpu::ShaderSource::Wgsl(hand_shader_code.into()),
            });

        let one_uniform_buffer_bgl =
            gpu_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("one_uniform_buffer_bgl"),
                });

        let model_data_bgl =
            gpu_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("model_data_bgl"),
                });

        let matrix_bg = gpu_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &model_data_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_manager
                        .get_global_buffer::<{ BufferType::Matrix as usize }>()
                        .as_entire_binding(),
                }],
                label: Some("matrix_bg"),
            });

        let vector_bg = gpu_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &model_data_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_manager
                        .get_global_buffer::<{ BufferType::Vector as usize }>()
                        .as_entire_binding(),
                }],
                label: Some("vector_bg"),
            });

        let camera_buffer = gpu_context.device.create_buffer(&BufferDescriptor {
            label: Some("Camera Buffer"),
            size: size_of::<WorldCameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pass_1_perframe_bg = gpu_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &one_uniform_buffer_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
                label: Some("pass_1_perframe_bg"),
            });

        let proj_buffer = gpu_context.device.create_buffer(&BufferDescriptor {
            label: Some("Projection Buffer"),
            size: size_of::<[[f32; 4]; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pass_2_resize_bg = gpu_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &one_uniform_buffer_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: proj_buffer.as_entire_binding(),
                }],
                label: Some("pass_2_resize_bg"),
            });

        let render_pipeline_layout =
            gpu_context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        Some(&block_data_bgl),
                        Some(&one_uniform_buffer_bgl),
                        Some(&model_data_bgl),
                    ],
                    immediate_size: 0,
                });

        let layer_0_render_pipeline = Self::create_standart_render_pipeline(
            gpu_context,
            "Layer 0 Render Pipeline",
            &render_pipeline_layout,
            &opaque_shader,
            &[Some(BlockVertex::desc())],
            window_context.config.format,
        );
        let hand_render_pipeline = Self::create_standart_render_pipeline(
            gpu_context,
            "Hand Render Pipeline",
            &render_pipeline_layout,
            &hand_shader,
            &[Some(BlockVertex::desc())],
            window_context.config.format,
        );

        let depth_texture_view =
            DepthTexture::create(&gpu_context.device, &window_context.config, "depth_texture");

        Ok(Renderer {
            render_context,
            pipelines: [layer_0_render_pipeline, hand_render_pipeline],
            depth_texture_view,
            block_static_render_data: block_data,
            pass_1_perframe_bg,
            camera_buffer,
            pass_2_resize_bg,
            proj_buffer,
            matrix_bg,
            vector_bg,
            buffer_manager,
        })
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let gpu_context = &self.render_context.gpu_contexts[0];
        let window_context = &self.render_context.window_contexts[0];

        if !window_context.is_surface_configured {
            return Ok(());
        }
        {
            let output = match window_context.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
                wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                    // println!("CurrentSurfaceTexture::Suboptimal");
                    // window_context.surface.configure(&gpu_context.device, &window_context.config);
                    surface_texture
                }
                wgpu::CurrentSurfaceTexture::Timeout
                | wgpu::CurrentSurfaceTexture::Occluded
                | wgpu::CurrentSurfaceTexture::Validation => {
                    // println!("CurrentSurfaceTexture::TOV");
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    // println!("CurrentSurfaceTexture::Outdated");
                    window_context
                        .surface
                        .configure(&gpu_context.device, &window_context.config);
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    anyhow::bail!("Lost device");
                }
            };

            let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(window_context.config.format.add_srgb_suffix()),
                ..Default::default()
            });

            let mut encoder =
                gpu_context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });

            self.buffer_manager
                .prepare_buffers(&gpu_context.queue, &mut encoder);

            let global_vertex_buffer = self
                .buffer_manager
                .get_global_buffer::<{ BufferType::Vertex as usize }>()
                .slice(..);
            let global_index_buffer = self
                .buffer_manager
                .get_global_buffer::<{ BufferType::Index as usize }>()
                .slice(..);

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_texture_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });

                render_pass.set_pipeline(&self.pipelines[0]);

                render_pass.set_bind_group(0, &self.block_static_render_data.bind_group, &[]);
                render_pass.set_bind_group(1, &self.pass_1_perframe_bg, &[]);
                render_pass.set_bind_group(2, &self.vector_bg, &[]);

                render_pass.set_vertex_buffer(0, global_vertex_buffer);
                render_pass.set_index_buffer(global_index_buffer, wgpu::IndexFormat::Uint32);

                render_pass.multi_draw_indexed_indirect(
                    &self.buffer_manager.gpu_indexed_indirect_buffer,
                    size_of::<DrawIndexedIndirectArgs>() as u64,
                    self.buffer_manager.cpu_indexed_indirect_buffer.len() as u32 - 1,
                );
            }

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_texture_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });

                render_pass.set_pipeline(&self.pipelines[1]);

                render_pass.set_bind_group(0, &self.block_static_render_data.bind_group, &[]);
                render_pass.set_bind_group(1, &self.pass_2_resize_bg, &[]);
                render_pass.set_bind_group(2, &self.matrix_bg, &[]);

                render_pass.set_vertex_buffer(0, global_vertex_buffer);
                render_pass.set_index_buffer(global_index_buffer, wgpu::IndexFormat::Uint32);

                render_pass.multi_draw_indexed_indirect(
                    &self.buffer_manager.gpu_indexed_indirect_buffer,
                    0,
                    1,
                );
            }

            gpu_context.queue.submit(iter::once(encoder.finish()));
            // output.present();
        }

        // if window_context.is_suboptimal {
        //     window_context.surface.configure(&gpu_context.device, &window_context.config);
        //     window_context.is_suboptimal = false;
        // }

        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32, lens: &mut RotatableLens) {
        if width > 0 && height > 0 {
            self.render_context.resize(width, height);
            lens.aspect = width as f32 / height as f32;
            self.depth_texture_view = DepthTexture::create(
                &self.render_context.gpu_contexts[0].device,
                &self.render_context.window_contexts[0].config,
                "depth_texture",
            );
            self.render_context.gpu_contexts[0].queue.write_buffer(
                &self.proj_buffer,
                0,
                bytemuck::cast_slice(&lens.get_proj_mat().to_cols_array_2d()),
            );
        }
    }

    pub fn frame(&mut self, pass_1_camera_uniform: WorldCameraUniform) {
        self.render_context.gpu_contexts[0].queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[pass_1_camera_uniform]),
        );
    }

    pub fn load_chunk(
        &mut self,
        primitive: BlockIndexedPrimitive,
        position: [i32; 4],
    ) -> anyhow::Result<usize> {
        self.buffer_manager.load_chunk(primitive, position)
    }

    pub fn unload(&mut self, slot_id: usize) {
        self.buffer_manager.unload(slot_id);
    }

    pub fn create_standart_render_pipeline(
        gpu_context: &GpuContext,
        label: &'static str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        buffers: &[Option<VertexBufferLayout>],
        format: TextureFormat,
    ) -> RenderPipeline {
        gpu_context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    // cull_mode: Some(wgpu::Face::Back),
                    cull_mode: None,
                    // polygon_mode: wgpu::PolygonMode::Line,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            })
    }

    pub fn create_line_render_pipeline(
        gpu_context: &GpuContext,
        label: &'static str,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        buffers: &[Option<VertexBufferLayout>],
        format: TextureFormat,
    ) -> RenderPipeline {
        gpu_context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    // cull_mode: Some(wgpu::Face::Back),
                    cull_mode: None,
                    // polygon_mode: wgpu::PolygonMode::Line,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            })
    }
}

#[derive(Clone, Copy)]
pub struct RendererCreateArgs {
    pub block_properties: &'static [u8],
    pub layer_count: u32,
    pub outline_vertices: &'static [u8],
}

pub struct BlockStaticRenderData {
    pub bind_group: BindGroup,
}

impl BlockStaticRenderData {
    pub fn new(gpu_context: &GpuContext, args: RendererCreateArgs) -> (Self, BindGroupLayout) {
        let texture_sampler = gpu_context.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Voxel Block Sampler"),
            // Включаем бесконечный повтор текстуры по всем осям координат
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            // Включаем Nearest-фильтрацию для сохранения четкого пиксель-арта
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            // Настройка мипмаппинга (Nearest убирает размытие блоков на расстоянии)
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            // Остальные параметры оставляем дефолтными
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let bind_group_layout =
            gpu_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2Array,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    label: Some("block_static_render_data_bind_group_layout"),
                });

        let block_properties_buffer = gpu_context.device.create_buffer(&BufferDescriptor {
            label: Some("Block Properties Buffer"),
            size: (size_of::<u64>() * args.block_properties.len()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture_view = TextureArray2D::init(
            &gpu_context,
            vfs_include_bytes!("workspace://game-layer/crates/minecraft/content/000001"),
            16,
            args.layer_count,
        );

        let bind_group = gpu_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(
                            block_properties_buffer.as_entire_buffer_binding(),
                        ),
                    },
                ],
                label: Some("texture_bind_group"),
            });

        gpu_context
            .queue
            .write_buffer(&block_properties_buffer, 0, args.block_properties);
        (Self { bind_group }, bind_group_layout)
    }
}
