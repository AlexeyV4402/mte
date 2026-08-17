use anyhow::*;
use glam::Vec2;
use image::GenericImageView;
use wgpu::TextureView;

use crate::context::gpu_context::GpuContext;

pub struct DepthTexture;

impl DepthTexture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &'static str,
    ) -> TextureView {
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

        view
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

    pub fn init(
        &self,
        gpu_context: &GpuContext,
        rgba_bytes: &[u8],
    ) -> (wgpu::Texture, wgpu::TextureView) {
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
