// use wgpu::RenderPipeline;

// use crate::gpu_context::GpuContext;

// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct TextUniforms {
//     // Матрица 4х4 (glam::Mat4 преобразуется в [[f32; 4]; 4])
//     pub matrix: [[f32; 4]; 4], 
//     // Цвет RGBA (например, [1.0, 1.0, 1.0, 1.0] для белого)
//     pub color: [f32; 4],       
//     // Тот самый px_range из генератора (например, 4.0)
//     pub px_range: f32,         
//     // Заглушка (Padding) для выравнивания структуры по границе 16 байт
//     pub _padding: [f32; 3],    
// }

// pub fn create_msdf_texture(
//     device: &wgpu::Device,
//     queue: &wgpu::Queue,
//     image: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
// ) -> wgpu::TextureView {
//     // 1. Конвертируем RGB в RGBA, так как WebGPU/wgpu не везде поддерживают чистый RGB формат
//     let rgba_image = image::DynamicImage::ImageRgb8(image.clone()).into_rgba8();
//     let (width, height) = rgba_image.dimensions();

//     // 2. Создаем текстуру на GPU
//     let texture_size = wgpu::Extent3d {
//         width,
//         height,
//         depth_or_array_layers: 1,
//     };

//     let texture = device.create_texture(&wgpu::TextureDescriptor {
//         label: Some("MSDF Font Texture"),
//         size: texture_size,
//         mip_level_count: 1,
//         sample_count: 1,
//         dimension: wgpu::TextureDimension::D2,
//         format: wgpu::TextureFormat::Rgba8Unorm, // Храним точные float-расстояния
//         usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
//         view_formats: &[],
//     });

//     // 3. Загружаем байты из оперативной памяти в видеопамять
//     queue.write_texture(
//         wgpu::TexelCopyTextureInfo {
//             texture: &texture,
//             mip_level: 0,
//             origin: wgpu::Origin3d::ZERO,
//             aspect: wgpu::TextureAspect::All,
//         },
//         &rgba_image,
//         wgpu::TexelCopyBufferLayout {
//             offset: 0,
//             bytes_per_row: Some(4 * width), // 4 байта на пиксель (RGBA)
//             rows_per_image: Some(height),
//         },
//         texture_size,
//     );

//     texture.create_view(&wgpu::TextureViewDescriptor::default())
// }


// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct TextVertex {
//     pub position: [f32; 3], // X, Y, Z в мировых координатах
//     pub uv: [f32; 2],       // Координаты текстуры атласа (0.0..1.0)
// }

// impl TextVertex {
//     // Описание разметки вершин для wgpu RenderPipeline
//     pub fn desc() -> wgpu::VertexBufferLayout<'static> {
//         wgpu::VertexBufferLayout {
//             array_stride: std::mem::size_of::<TextVertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &[
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 0,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
//                     shader_location: 1,
//                     format: wgpu::VertexFormat::Float32x2,
//                 },
//             ],
//         }
//     }
// }


// pub fn create_text_pipeline(
//     device: &wgpu::Device,
//     render_target_format: wgpu::TextureFormat, // Формат вашего экрана/кадра (например, Bgra8UnormSrgb)
// ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    
//     // 1. Загружаем и компилируем наш WGSL-шейдер
//     let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
//         label: Some("Text MSDF Shader"),
//         source: wgpu::ShaderSource::Wgsl(include_str!("../../../as.wgsl").into()),
//     });

//     // 2. Создаем Bind Group Layout (описываем, какие ресурсы видит шейдер в @group(0))
//     let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//         label: Some("Text Bind Group Layout"),
//         entries: &[
//             // @binding(0): Текстура атласа шрифта
//             wgpu::BindGroupLayoutEntry {
//                 binding: 0,
//                 visibility: wgpu::ShaderStages::FRAGMENT,
//                 ty: wgpu::BindingType::Texture {
//                     multisampled: false,
//                     view_dimension: wgpu::TextureViewDimension::D2,
//                     sample_type: wgpu::TextureSampleType::Float { filterable: true }, // Обязательно true для MSDF!
//                 },
//                 count: None,
//             },
//             // @binding(1): Линейный самплер
//             wgpu::BindGroupLayoutEntry {
//                 binding: 1,
//                 visibility: wgpu::ShaderStages::FRAGMENT,
//                 ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
//                 count: None,
//             },
//             // @binding(2): Юниформы (матрица MVP, цвет, px_range)
//             wgpu::BindGroupLayoutEntry {
//                 binding: 2,
//                 visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
//                 ty: wgpu::BindingType::Buffer {
//                     ty: wgpu::BufferBindingType::Uniform,
//                     has_dynamic_offset: false,
//                     min_binding_size: None,
//                 },
//                 count: None,
//             },
//         ],
//     });

//     // 3. Создаем Layout для самого Pipeline
//     let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//         label: Some("Text Pipeline Layout"),
//         bind_group_layouts: &[Some(&bind_group_layout)],
//         immediate_size: 0,
//     });

//     // 4. Собираем Render Pipeline
//     let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
//         label: Some("Text Render Pipeline"),
//         layout: Some(&pipeline_layout),
        
//         // Настройки вершинного шейдера
//         vertex: wgpu::VertexState {
//             module: &shader,
//             entry_point: Some("vs_main"),
//             compilation_options: Default::default(),
//             buffers: &[TextVertex::desc()], // Передаем разметку нашей структуры вершины
//         },
        
//         // Настройки фрагментного шейдера и блендинга
//         fragment: Some(wgpu::FragmentState {
//             module: &shader,
//             entry_point: Some("fs_main"),
//             compilation_options: Default::default(),
//             targets: &[Some(wgpu::ColorTargetState {
//                 format: render_target_format,
//                 // Включаем классический Альфа-блендинг, чтобы текст плавно сглаживался с фоном
//                 blend: Some(wgpu::BlendState::ALPHA_BLENDING),
//                 write_mask: wgpu::ColorWrites::ALL,
//             })],
//         }),
        
//         // Как интерпретировать геометрию (отрисовка треугольниками)
//         primitive: wgpu::PrimitiveState {
//             topology: wgpu::PrimitiveTopology::TriangleList,
//             strip_index_format: None,
//             front_face: wgpu::FrontFace::Ccw, // Против часовой стрелки
//             cull_mode: None, // Отключаем куллинг, чтобы текст был виден с обеих сторон в 3D
//             unclipped_depth: false,
//             polygon_mode: wgpu::PolygonMode::Fill,
//             conservative: false,
//         },
        
//         // Если у вас в движке используется Z-буфер (тест глубины), укажите его настройки здесь.
//         // Для UI-текста поверх игры тест глубины обычно отключают или ставят Read-Only.
//         depth_stencil: Some(wgpu::DepthStencilState {
//             format: wgpu::TextureFormat::Depth32Float,
//             depth_write_enabled: Some(true),
//             depth_compare: Some(wgpu::CompareFunction::LessEqual),
//             stencil: wgpu::StencilState::default(),
//             bias: wgpu::DepthBiasState::default(),
//         }),
        
//         multisample: wgpu::MultisampleState {
//             count: 1,
//             mask: !0,
//             alpha_to_coverage_enabled: false,
//         },
//         cache: None,
//         multiview_mask: None,
//     });

//     (pipeline, bind_group_layout)
// }


// use std::iter;
// use std::sync::Arc;
// use glam::Mat4;
// use image::{ImageBuffer, Rgb};
// use wgpu::util::DeviceExt;
// use winit::application::ApplicationHandler;
// use winit::event::WindowEvent;
// use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
// use winit::window::{Window, WindowId};

// // --- СЛОЙ КОНТЕКСТА GPU И ОТРИСОВКИ ДВИЖКА ---
// pub struct Renderer {
//     gpu_context: GpuContext, // Теперь контекст скрыт внутри рендерера!
//     text_renderer: TextRenderer,
// }

// impl Renderer {
//     pub fn new(gpu_context: GpuContext, msdf_image: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> Self {
//         // Инициализируем подсистему шрифтов, передавая устройство и формат из контекста
//         let text_renderer = TextRenderer::new(
//             &gpu_context.device,
//             &gpu_context.queue,
//             gpu_context.config.format.add_srgb_suffix(), 
//             msdf_image
//         );

//         Self {
//             gpu_context,
//             text_renderer,
//         }
//     }

//     // Главная функция отрисовки кадра движка
// pub fn draw_frame(&self) {
//     self.gpu_context.window.request_redraw();

//     if !self.gpu_context.is_surface_configured {
//         return;
//     }

//     // 1. Получаем текущую текстуру кадра
//     let output = match self.gpu_context.surface.get_current_texture() {
//         wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
        
//         // КРИТИЧЕСКИЙ УЗЕЛ ДЛЯ WAYLAND:
//         wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
//             // Сначала ПРИНУДИТЕЛЬНО уничтожаем suboptimal-текстуру,
//             // чтобы освободить её внутренние семафоры Vulkan!
//             drop(surface_texture); 
            
//             // Только ТЕПЕРЬ, когда семафоры чисты, безопасно пересобираем Swapchain
//             self.gpu_context.surface.configure(&self.gpu_context.device, &self.gpu_context.config);
            
//             // Пропускаем этот дефектный кадр, экран обновится на следующем тике Poll
//             return;
//         }
//         wgpu::CurrentSurfaceTexture::Timeout

//         | wgpu::CurrentSurfaceTexture::Occluded
//         | wgpu::CurrentSurfaceTexture::Validation => {
//             return;
//         }
//         wgpu::CurrentSurfaceTexture::Outdated => {
//             // Для Outdated текстуры в комплекте нет, поэтому configure можно вызывать сразу
//             self.gpu_context.surface.configure(&self.gpu_context.device, &self.gpu_context.config);
//             return;
//         }
//         wgpu::CurrentSurfaceTexture::Lost => {
//             return;
//         }
//     };

//     // 2. Если мы здесь — у нас на 100% чистая, валидная текстура Success
//     let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
//         format: Some(self.gpu_context.config.format.add_srgb_suffix()),
//         ..Default::default()
//     });

//     let mut encoder = self.gpu_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
//         label: Some("Render Encoder"),
//     });
        
//     {
//         let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//             label: Some("Render Pass"),
//             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                 view: &view, // Вывод идет на экран
//                 resolve_target: None,
//                 ops: wgpu::Operations {
//                     load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
//                     store: wgpu::StoreOp::Store,
//                 },
//                 depth_slice: None,
//             })],
//             depth_stencil_attachment: None,
//             occlusion_query_set: None,
//             timestamp_writes: None,
//             multiview_mask: None,
//         });

//         // Отрисовка нашего MSDF-текста
//         self.text_renderer.render(&mut render_pass);
//     }

//     // 3. Отправляем команды и презентуем кадр
//     self.gpu_context.queue.submit(std::iter::once(encoder.finish()));
//     output.present();
// }



//     // Метод для обработки изменения размеров окна
//     pub fn resize(&mut self, new_width: u32, new_height: u32) {
//         if new_width > 0 && new_height > 0 {
//             // Обновляем размеры в конфиге
//             self.gpu_context.config.width = new_width;
//             self.gpu_context.config.height = new_height;
            
//             // ПОВТОРНО КОНФИГУРИРУЕМ ПОВЕРХНОСТЬ
//             self.gpu_context.surface.configure(
//                 &self.gpu_context.device, 
//                 &self.gpu_context.config
//             );
//         }
//     }
// }

// // --- ПОДСИСТЕМА ШРИФТОВ (Остается сфокусированной только на тексте) ---
// pub struct TextRenderer {
//     pipeline: wgpu::RenderPipeline,
//     bind_group: wgpu::BindGroup,
//     vertex_buffer: wgpu::Buffer,
//     index_buffer: wgpu::Buffer,
//     uniform_buffer: wgpu::Buffer,
// }

// impl TextRenderer {
//     pub fn new(
//         device: &wgpu::Device,
//         queue: &wgpu::Queue,
//         texture_format: wgpu::TextureFormat,
//         msdf_image: &ImageBuffer<Rgb<u8>, Vec<u8>>,
//     ) -> Self {
//         let vertices: [TextVertex; 4] = [
//             TextVertex { position: [0.0, 1.0, 0.0], uv: [0.0, 0.0] },
//             TextVertex { position: [0.0, 0.0, 0.0], uv: [0.0, 1.0] },
//             TextVertex { position: [1.0, 0.0, 0.0], uv: [1.0, 1.0] },
//             TextVertex { position: [1.0, 1.0, 0.0], uv: [1.0, 0.0] },
//         ];
//         let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

//         let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Text Vertex Buffer"),
//             contents: bytemuck::cast_slice(&vertices),
//             usage: wgpu::BufferUsages::VERTEX,
//         });

//         let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Text Index Buffer"),
//             contents: bytemuck::cast_slice(&indices),
//             usage: wgpu::BufferUsages::INDEX,
//         });

//         let msdf_view = create_msdf_texture(device, queue, msdf_image);
//         let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
//             label: Some("MSDF Text Linear Sampler"),
//             address_mode_u: wgpu::AddressMode::ClampToEdge,
//             address_mode_v: wgpu::AddressMode::ClampToEdge,
//             mag_filter: wgpu::FilterMode::Linear,
//             min_filter: wgpu::FilterMode::Linear,
//             ..Default::default()
//         });

//         let initial_uniforms = TextUniforms {
//             matrix: Mat4::IDENTITY.to_cols_array_2d(),
//             color: [1.0, 1.0, 1.0, 1.0],
//             px_range: 4.0,
//             _padding: [0.0; 3],
//         };
//         let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Text Uniform Buffer"),
//             contents: bytemuck::cast_slice(&[initial_uniforms]),
//             usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
//         });

//         let (pipeline, layout) = create_text_pipeline(device, texture_format);

//         let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             label: Some("Text Bind Group"),
//             layout: &layout,
//             entries: &[
//                 wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&msdf_view) },
//                 wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&linear_sampler) },
//                 wgpu::BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
//             ],
//         });

//         Self { pipeline, bind_group, vertex_buffer, index_buffer, uniform_buffer }
//     }

//     pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
//         render_pass.set_pipeline(&self.pipeline);
//         render_pass.set_bind_group(0, &self.bind_group, &[]);
//         render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
//         render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
//         render_pass.draw_indexed(0..6, 0, 0..1);
//     }
// }

// // --- СИСТЕМНЫЙ СЛОЙ ОКНА (WINIT) ---

// pub struct AppState {
//     window: Arc<Window>,
//     renderer: Renderer,
// }

// pub struct App {
//     state: Option<AppState>,
//     // 🔥 Обертываем картинку в Arc, чтобы её можно было безопасно читать многократно
//     msdf_img_data: Arc<ImageBuffer<Rgb<u8>, Vec<u8>>>,
// }

// impl ApplicationHandler for App {
//     // 1. Метод resumed теперь СТРОГО только создает окно в ОС, если его еще нет
//     fn resumed(&mut self, event_loop: &ActiveEventLoop) {
//         if self.state.is_some() { return; }
        
//         // Создаем окно, но графику пока не трогаем — ждем первого Resized от Wayland
//         let window = Arc::new(
//             event_loop
//                 .create_window(winit::window::Window::default_attributes().with_title("My Engine"))
//                 .unwrap()
//         );
        
//         // Чтобы зафиксировать окно, временно сохраняем его, а рендерер соберем в Resized
//         // (Для удобства инициализации оставим это поле Option внутри AppState)
//     }

//     fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
//         match event {
//             WindowEvent::CloseRequested => {
//                 event_loop.exit();
//             }
            
//             // 🔥 КЛЮЧЕВОЙ УЗЕЛ: Безопасный запуск графического конвейера
//             WindowEvent::Resized(new_size) => {
//                 if new_size.width == 0 || new_size.height == 0 { return; }

//                 if self.state.is_none() {
//                     // Метод resumed гарантированно отработал, Wayland создал дескриптор окна.
//                     // Создаем окно заново, если winit стер промежуточное состояние, 
//                     // либо используем ActiveEventLoop для сборки контекста:
//                     let window = Arc::new(
//                         event_loop
//                             .create_window(winit::window::Window::default_attributes().with_title("My Engine"))
//                             .unwrap()
//                     );

//                     println!("Окно готово ({:?}). Безопасно инициализируем wgpu...", new_size);

//                     let mut gpu_context = pollster::block_on(GpuContext::new(Arc::clone(&window))).unwrap();
                    
//                     // 🔥 МЫ БОЛЬШЕ НЕ ДЕЛАЕМ .take()! Мы просто клонируем Arc-ссылку!
//                     // Это zero-copy операция (дублируется только указатель, а не мегабайты пикселей)
//                     let img_ref = Arc::clone(&self.msdf_img_data);

//                     // Конфигурируем буфер обмена под честный стартовый размер окна
//                     gpu_context.config.width = new_size.width;
//                     gpu_context.config.height = new_size.height;
//                     gpu_context.surface.configure(&gpu_context.device, &gpu_context.config);
//                     gpu_context.is_surface_configured = true;

//                     // Собираем наш Renderer, передавая ему ссылку на картинку
//                     let renderer = Renderer::new(gpu_context, &img_ref);

//                     self.state = Some(AppState {
//                         window,
//                         renderer,
//                     });
//                 } else {
//                     // Обычный ресайз окна пользователем в процессе игры
//                     if let Some(state) = &mut self.state {
//                         state.renderer.resize(new_size.width, new_size.height);
//                     }
//                 }
//             }

//             WindowEvent::RedrawRequested => {
//                 if let Some(state) = &self.state {
//                     state.renderer.draw_frame();
//                 }
//             }
            
//             _ => {}
//         }
//     }

//     fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
//         if let Some(state) = &self.state {
//             state.window.request_redraw();
//         }
//     }
// }

// // --- ИЗМЕНЕНИЕ ФУНКЦИИ RUN ---
// pub fn run() {
//     let a = include_bytes!("../../.././aboba.jpg");
//     let img = image::load_from_memory(a).expect("Не удалось декодировать JPEG");
//     let b = img.to_rgb8();

//     let event_loop = EventLoop::new().unwrap();
//     event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

//     let mut app = App {
//         state: None,
//         msdf_img_data: Arc::new(b), // Упаковываем картинку в Arc на самом старте!
//     };

//     event_loop.run_app(&mut app).unwrap();
// }


