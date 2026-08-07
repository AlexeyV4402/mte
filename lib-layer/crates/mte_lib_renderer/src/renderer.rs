use std::{fmt, iter};

use anyhow::Result;
use mte_macros::{vfs_include_bytes, vfs_include_str};
use wgpu::wgt::DrawIndexedIndirectArgs;
use wgpu::{BindGroup, Buffer, RenderPipeline};

use crate::context::render_context::RenderContext;
use crate::render_objects::camera::CameraSystem;
use crate::render_objects::texture::{self, Texture};
use crate::types::Vertex;

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

// Создаешь обертку и вешаешь на нее derive(Clone, Copy)
#[derive(Clone, Copy)]
pub struct DebugDrawArgs(pub DrawIndexedIndirectArgs);

// Вручную реализуешь трейт Debug для своей обертки
impl fmt::Debug for DebugDrawArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawIndexedIndirectArgs")
            .field("index_count", &self.0.index_count)
            .field("instance_count", &self.0.instance_count)
            .field("first_index", &self.0.first_index)
            .field("base_vertex", &self.0.base_vertex)
            .field("base_instance", &self.0.first_instance)
            .finish()
    }
}

pub const GLOBAL_BUFFER_SECTION_COUNT: usize = 64;

pub const GLOBAL_BUFFER_INDIRECT_CAPACITY: usize =
    size_of::<DrawIndexedIndirectArgs>() * GLOBAL_BUFFER_SECTION_COUNT;

pub const GLOBAL_BUFFER_VERTEX_PER_SECTION: u32 = 196608;
pub const GLOBAL_BUFFER_INDEX_PER_SECTION: u32 =
    (GLOBAL_BUFFER_VERTEX_PER_SECTION as f32 * 1.5) as u32;

pub const GLOBAL_VERTEX_BUFFER_SECTION_CAPACITY: usize =
    (size_of::<Vertex>() * GLOBAL_BUFFER_VERTEX_PER_SECTION as usize) as usize;
pub const GLOBAL_VERTEX_BUFFER_CAPACITY: usize =
    GLOBAL_VERTEX_BUFFER_SECTION_CAPACITY * GLOBAL_BUFFER_SECTION_COUNT;

const _: () = assert!(
    GLOBAL_VERTEX_BUFFER_CAPACITY <= 268435456,
    "Запрещено создание буферов больше 256 МБ"
);

pub const GLOBAL_INDEX_BUFFER_SECTION_CAPACITY: usize =
    (size_of::<u32>() * GLOBAL_BUFFER_INDEX_PER_SECTION as usize) as usize;
pub const GLOBAL_INDEX_BUFFER_CAPACITY: usize =
    GLOBAL_INDEX_BUFFER_SECTION_CAPACITY * GLOBAL_BUFFER_SECTION_COUNT;

const _: () = assert!(
    GLOBAL_INDEX_BUFFER_CAPACITY <= 268435456,
    "Запрещено создание буферов больше 256 МБ"
);

const GLOBAL_MATRIX_BUFFER_CAPACITY: usize =
    GLOBAL_BUFFER_SECTION_COUNT as usize * std::mem::size_of::<[[f32; 4]; 4]>();

const _: () = assert!(
    GLOBAL_MATRIX_BUFFER_CAPACITY <= 268435456,
    "Запрещено создание буферов больше 256 МБ"
);

pub struct Renderer {
    pub render_context: RenderContext,
    pipelines: [RenderPipeline; 1],
    depth_texture: Texture,
    pub diffuse_bind_group: BindGroup,
    pub camera_system: CameraSystem,

    pub gpu_indexed_indirect_buffer: Buffer,
    pub cpu_indexed_indirect_buffer: Vec<DrawIndexedIndirectArgs>,

    pub global_index_buffer: Buffer,
    pub global_vertex_buffer: Buffer,
    pub global_matrix_buffer: Buffer,

    pub global_buffer_free_slots: Vec<usize>,
}

impl Renderer {
    pub async fn new(render_context: RenderContext) -> Result<Renderer> {
        let gpu_context = &render_context.gpu_contexts[0];
        let window_context = &render_context.window_contexts[0];

        // Загрузка основного шейдера
        let shader_code_1 =
            vfs_include_str!("workspace://game-layer/assets/minecraft/shaders/opaque.wgsl");
        let shader = gpu_context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_code_1.into()),
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
                                view_dimension: wgpu::TextureViewDimension::D2,
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

        let diffuse_bytes = vfs_include_bytes!(
            "workspace://game-layer/assets/minecraft/textures/fasn-kukuruza.jpg"
        );

        let diffuse_texture = texture::Texture::from_bytes(
            &gpu_context.device,
            &gpu_context.queue,
            diffuse_bytes,
            "fasn-kukuruza.jpg",
            false,
        )
        .unwrap();

        let diffuse_bind_group = gpu_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view), // CHANGED!
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler), // CHANGED!
                    },
                ],
                label: Some("diffuse_bind_group"),
            });

        let camera_bind_group_layout =
            gpu_context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            // КРИТИЧЕСКИ ОШИБКА БЫЛА ТУТ: Обязательно добавляем VERTEX!
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                // Указываем, что это Storage буфер только для чтения (read)
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    label: Some("camera_bind_group_layout"),
                });

        let depth_texture = Texture::create_depth_texture(
            &gpu_context.device,
            &window_context.config,
            "depth_texture",
        );

        let global_matrix_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Global Matrix Buffer"),
            size: GLOBAL_MATRIX_BUFFER_CAPACITY as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_system = CameraSystem::new(
            &render_context,
            &camera_bind_group_layout,
            &global_matrix_buffer,
        );

        let global_vertex_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: GLOBAL_VERTEX_BUFFER_CAPACITY as u64,
            mapped_at_creation: false,
        });

        let global_index_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            size: GLOBAL_INDEX_BUFFER_CAPACITY as u64,
            mapped_at_creation: false,
        });

        let cpu_indexed_indirect_buffer: Vec<DrawIndexedIndirectArgs> =
            vec![DrawIndexedIndirectArgs::default(); GLOBAL_BUFFER_SECTION_COUNT];

        let gpu_indexed_indirect_buffer =
            gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Indirect Buffer"),
                usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
                size: GLOBAL_BUFFER_INDIRECT_CAPACITY as u64,
                mapped_at_creation: false,
            });

        // Создание раскладки конвеера
        let render_pipeline_layout =
            gpu_context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        Some(&texture_bind_group_layout),
                        Some(&camera_bind_group_layout),
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
            diffuse_bind_group: diffuse_bind_group,
            camera_system,
            gpu_indexed_indirect_buffer,
            cpu_indexed_indirect_buffer,
            global_index_buffer,
            global_vertex_buffer,
            global_buffer_free_slots: (0..GLOBAL_BUFFER_SECTION_COUNT).collect(),
            global_matrix_buffer,
        })
    }

    pub fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        topology: wgpu::PrimitiveTopology, // NEW!
        shader: wgpu::ShaderModuleDescriptor,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(shader);

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{:?}", shader)),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: vertex_layouts,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology, // NEW!
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual), // UDPATED!
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // tells wgpu to render to just specific texture layers.
            multiview_mask: None,
            cache: None,
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
        let gpu_context = &self.render_context.gpu_contexts[0];
        if mesh.0.len() > GLOBAL_BUFFER_VERTEX_PER_SECTION as usize
            || mesh.1.len() > GLOBAL_BUFFER_INDEX_PER_SECTION as usize
        {
            return Err(anyhow::anyhow!(
                "{} вершин дано;\n{} вершин допустимо;\n{} индексов дано;\n{} индексов допустимо;",
                mesh.0.len(),
                GLOBAL_BUFFER_VERTEX_PER_SECTION,
                mesh.1.len(),
                GLOBAL_BUFFER_INDEX_PER_SECTION
            ));
        }

        let free = self
            .global_buffer_free_slots
            // .remove(0);
            .pop()
            .ok_or(anyhow::anyhow!("Нет свободных слотов в VRAM"))?;

        let vertex_offset_bytes = (free * GLOBAL_VERTEX_BUFFER_SECTION_CAPACITY) as u64;
        let index_offset_bytes = (free * GLOBAL_INDEX_BUFFER_SECTION_CAPACITY) as u64;
        let matrix_offset_bytes = free as u64 * std::mem::size_of::<[[f32; 4]; 4]>() as u64;

        gpu_context.queue.write_buffer(
            &self.global_vertex_buffer,
            vertex_offset_bytes,
            bytemuck::cast_slice(&mesh.0),
        );

        gpu_context.queue.write_buffer(
            &self.global_index_buffer,
            index_offset_bytes,
            bytemuck::cast_slice(&mesh.1),
        );

        gpu_context.queue.write_buffer(
            &self.global_matrix_buffer,
            matrix_offset_bytes,
            bytemuck::cast_slice(&[model_matrix]),
        );

        self.cpu_indexed_indirect_buffer[free] = DrawIndexedIndirectArgs {
            index_count: mesh.1.len() as u32,
            instance_count: 1, // Рисуем 1 экземпляр чанка
            first_index: free as u32 * GLOBAL_BUFFER_INDEX_PER_SECTION, // С какого индекса в штуках начинать рендер [7]
            base_vertex: (free as u32 * GLOBAL_BUFFER_VERTEX_PER_SECTION) as i32, // Какое число прибавлять к индексам на GPU [5, 8]
            first_instance: free as u32,
        };

        Ok(free)
    }

    pub fn unload_mesh(&mut self, slot_id: usize) {
        self.global_buffer_free_slots.push(slot_id);

        self.cpu_indexed_indirect_buffer[slot_id] = DrawIndexedIndirectArgs {
            index_count: 0,    // Видеокарта увидит 0 и мгновенно пропустит этот слот
            instance_count: 0, // На всякий случай зануляем и инстансы
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        };
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let gpu_context = &self.render_context.gpu_contexts[0];
        let window_context = &self.render_context.window_contexts[0];

        gpu_context.queue.write_buffer(
            &self.gpu_indexed_indirect_buffer,
            0,
            bytemuck::cast_slice(&self.cpu_indexed_indirect_buffer),
        );

        // window_context.window.request_redraw();

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

                render_pass.set_pipeline(&self.pipelines[0]);
                render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
                render_pass.set_bind_group(1, &self.camera_system.bind_group, &[]);

                render_pass.set_vertex_buffer(0, self.global_vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    self.global_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );

                render_pass.multi_draw_indexed_indirect(
                    &self.gpu_indexed_indirect_buffer,
                    0,
                    self.cpu_indexed_indirect_buffer.len() as u32,
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
