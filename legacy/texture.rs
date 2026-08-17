use anyhow::*;
use glam::Vec2;
use image::GenericImageView;

use crate::context::gpu_context::GpuContext;

pub struct Texture {
    #[allow(unused)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: wgpu::Extent3d,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &'static str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[Self::DEPTH_FORMAT],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            size,
        }
    }

    #[allow(dead_code)]
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        is_normal_map: bool,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label), is_normal_map)
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
        is_normal_map: bool,
    ) -> Result<Self> {
        let dimensions = img.dimensions();
        let rgba = img.to_rgba8();

        let format = if is_normal_map {
            wgpu::TextureFormat::Rgba8Unorm
        } else {
            wgpu::TextureFormat::Rgba8UnormSrgb
        };
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let size = wgpu::Extent3d {
            width: img.width(),
            height: img.height(),
            depth_or_array_layers: 1,
        };
        let texture = Self::create_2d_texture(
            device,
            size.width,
            size.height,
            format,
            usage,
            wgpu::FilterMode::Linear,
            label,
        );

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        Ok(texture)
    }

    pub(crate) fn create_2d_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        mag_filter: wgpu::FilterMode,
        label: Option<&str>,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        Self::create_texture(
            device,
            label,
            size,
            format,
            usage,
            wgpu::TextureDimension::D2,
            mag_filter,
        )
    }

    pub fn create_texture(
        device: &wgpu::Device,
        label: Option<&str>,
        size: wgpu::Extent3d,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        dimension: wgpu::TextureDimension,
        mag_filter: wgpu::FilterMode,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format,
            usage,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            size, // NEW!
        }
    }
}

pub struct CubeTexture {
    texture: wgpu::Texture,
    sampler: wgpu::Sampler,
    view: wgpu::TextureView,
}

impl CubeTexture {
    pub fn create_2d(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        mip_level_count: u32,
        usage: wgpu::TextureUsages,
        mag_filter: wgpu::FilterMode,
        label: Option<&str>,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                // A cube has 6 sides, so we need 6 layers
                depth_or_array_layers: 6,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label,
            dimension: Some(wgpu::TextureViewDimension::Cube),
            array_layer_count: Some(6),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            sampler,
            view,
        }
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
}

pub struct TextureArray2D;

impl TextureArray2D {
    pub fn init(
        gpu_context: &GpuContext,
        rgba_bytes: &[u8],
        tile_size: u32,
        layer_count: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = gpu_context.device.create_texture(&wgpu::TextureDescriptor {
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

        gpu_context.queue.write_texture(
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
}

pub struct TextureAtlas {
    pub atlas_size: Vec2,
}

impl TextureAtlas {
    #[inline]
    pub fn get_uv_rect(&self, pixel_min: Vec2, pixel_size: Vec2) -> (Vec2, Vec2) {
        // Делаем branchless деление векторов за один такт
        // Инвертируем размер атласа, чтобы заменить тяжелое деление на быстрое умножение
        let inv_size = Vec2::ONE / self.atlas_size;

        let uv_min = pixel_min * inv_size;
        let uv_max = (pixel_min + pixel_size) * inv_size;

        (uv_min, uv_max)
    }

    pub fn init(&self, gpu_context: &GpuContext, rgba_bytes: &[u8]) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = gpu_context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Voxel Texture Array"),
            size: wgpu::Extent3d {
                width: self.atlas_size.x as u32,
                height: self.atlas_size.y as u32,
                depth_or_array_layers: 1, // КОЛИЧЕСТВО СЛОЕВ (ТЕКСТУР)
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        gpu_context.queue.write_texture(
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
                bytes_per_row: Some(self.atlas_size.x as u32 * 4),
                // Сколько байт занимает весь один слой (высота)
                rows_per_image: Some(self.atlas_size.y as u32),
            },
            wgpu::Extent3d {
                width: self.atlas_size.x as u32,
                height: self.atlas_size.y as u32,
                depth_or_array_layers: 1,
            },
        );

        // 3. ВАЖНО: Создаем правильный View с типом D2Array
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Voxel Texture Array View"),
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            dimension: Some(wgpu::TextureViewDimension::D2), // УКАЗЫВАЕМ, ЧТО ЭТО МАССИВ
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            usage: None,
        });

        (texture, view)
    }
}
