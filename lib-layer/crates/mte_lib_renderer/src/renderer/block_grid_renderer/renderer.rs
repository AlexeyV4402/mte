use std::iter;

use anyhow::Result;
use mte_macros::{vfs_include_bytes, vfs_include_str};
use wgpu::{BindGroup, RenderPipeline};

use super::render_objects::camera::CameraSystem;
use super::types::Vertex;
use crate::context::render_context::RenderContext;
use crate::render_objects::texture::Texture;
use crate::renderer::block_grid_renderer::buffer_manager::BufferManager;

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

    pipelines: [RenderPipeline; 1],

    depth_texture: Texture,

    pub texture_bind_group: BindGroup,

    pub model_matrix_bind_group: BindGroup,

    pub camera_system: CameraSystem,

    pub buffer_manager: BufferManager,
}

impl Renderer {
    pub async fn new(render_context: RenderContext) -> Result<Renderer> {
        let gpu_context = &render_context.gpu_contexts[0];
        let window_context = &render_context.window_contexts[0];

        let buffer_manager = BufferManager::new(gpu_context);

        // Загрузка основного шейдера
        let shader_code_1 =
            vfs_include_str!("workspace://game-layer/assets/minecraft/shaders/opaque_new.wgsl");
        let shader = gpu_context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_code_1.into()),
            });

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

        let texture_bind_group_layout =
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
                    ],
                    label: Some("texture_bind_group_layout"),
                });

        let (_texture, texture_view) = init_texture_array(
            &gpu_context.device,
            &gpu_context.queue,
            vfs_include_bytes!("workspace://game-layer/crates/minecraft/content/blocks.pck"),
            16,
            6,
        );

        let texture_bind_group = gpu_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture_sampler),
                    },
                ],
                label: Some("texture_bind_group"),
            });

        let camera_bind_group_layout =
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
                    label: Some("camera_bind_group_layout"),
                });

        let model_matrix_bind_group_layout =
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
                    label: Some("model_matrix_bind_group_layout"),
                });

        let depth_texture = Texture::create_depth_texture(
            &gpu_context.device,
            &window_context.config,
            "depth_texture",
        );

        let model_matrix_bind_group =
            gpu_context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &model_matrix_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer_manager
                            .get_global_model_matrix_buffer()
                            .as_entire_binding(),
                    }],
                    label: Some("model_matrix_bind_group"),
                });

        let camera_system = CameraSystem::new(&render_context, &camera_bind_group_layout);

        // Создание раскладки конвеера
        let render_pipeline_layout =
            gpu_context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        Some(&texture_bind_group_layout),
                        Some(&camera_bind_group_layout),
                        Some(&model_matrix_bind_group_layout),
                    ],
                    immediate_size: 0,
                });

        let render_pipeline =
            gpu_context
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Render Pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[Vertex::desc()],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: window_context.config.format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        // cull_mode: None,
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
                });

        Ok(Renderer {
            render_context,
            pipelines: [render_pipeline],
            depth_texture,
            texture_bind_group,
            model_matrix_bind_group,
            camera_system,
            buffer_manager,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.render_context.resize(width, height);
            self.depth_texture = Texture::create_depth_texture(
                &self.render_context.gpu_contexts[0].device,
                &self.render_context.window_contexts[0].config,
                "depth_texture",
            );
        }
    }

    pub fn update(&mut self, _dt: std::time::Duration) {
        self.render_context.gpu_contexts[0].queue.write_buffer(
            &self.camera_system.buffer,
            0,
            bytemuck::cast_slice(&[self.camera_system.get_uniform().view_proj]),
        );
    }

    pub fn load_mesh(
        &mut self,
        mesh: (Vec<Vertex>, Vec<u32>),
        model_matrix: [[f32; 4]; 4],
    ) -> anyhow::Result<usize> {
        self.buffer_manager.load_mesh(mesh.0, mesh.1, model_matrix)
    }

    pub fn unload_mesh(&mut self, slot_id: usize) {
        self.buffer_manager.unload_mesh(slot_id);
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
                        view: &self.depth_texture.view,
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

                // encoder.clear_texture(texture, subresource_range);

                render_pass.set_pipeline(&self.pipelines[0]);

                render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
                render_pass.set_bind_group(1, &self.camera_system.bind_group, &[]);
                render_pass.set_bind_group(2, &self.model_matrix_bind_group, &[]);

                render_pass
                    .set_vertex_buffer(0, self.buffer_manager.get_global_vertex_buffer().slice(..));
                render_pass.set_index_buffer(
                    self.buffer_manager.get_global_index_buffer().slice(..),
                    wgpu::IndexFormat::Uint32,
                );

                render_pass.multi_draw_indexed_indirect(
                    &self.buffer_manager.gpu_indexed_indirect_buffer,
                    0,
                    self.buffer_manager.cpu_indexed_indirect_buffer.len() as u32,
                );
            }

            gpu_context.queue.submit(iter::once(encoder.finish()));
            output.present();
        }

        // if window_context.is_suboptimal {
        //     window_context.surface.configure(&gpu_context.device, &window_context.config);
        //     window_context.is_suboptimal = false;
        // }

        Ok(())
    }
}

pub fn init_texture_array(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rgba_bytes: &[u8],
    tile_size: u32,
    layer_count: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    // 1. Создаем текстуру нужного объема
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Voxel Texture Array"),
        size: wgpu::Extent3d {
            width: tile_size,
            height: tile_size,
            depth_or_array_layers: layer_count, // КОЛИЧЕСТВО СЛОЕВ (ТЕКСТУР)
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // 2. Заливаем наш плоский массив байт прямо в GPU
    queue.write_texture(
        // В современных версиях wgpu структура называется TexelCopyTextureInfo
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba_bytes,
        // Настройка разметки данных в памяти
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            // Сколько байт занимает одна строка пикселей (ширина * 4 байта RGBA)
            bytes_per_row: Some(tile_size * 4),
            // Сколько байт занимает весь один слой (высота)
            rows_per_image: Some(tile_size),
        },
        wgpu::Extent3d {
            width: tile_size,
            height: tile_size,
            depth_or_array_layers: layer_count,
        },
    );

    // 3. ВАЖНО: Создаем правильный View с типом D2Array
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Voxel Texture Array View"),
        format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        dimension: Some(wgpu::TextureViewDimension::D2Array), // УКАЗЫВАЕМ, ЧТО ЭТО МАССИВ
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: Some(layer_count),
        usage: None,
    });

    (texture, view)
}
