use image::GenericImageView;
use anyhow::*;
use std::path::Path;
use std::num::NonZeroU32;

#[derive(Debug)]
pub struct Texture {
    pub texture : wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor, label: &str) -> Self {
        let size = wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        };

        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self { texture, view, sampler, }
    }

    pub fn _from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str
    ) -> Result<Self> {
        let img = match image::load_from_memory_with_format(bytes, image::ImageFormat::Png) {
            Ok(i) => i,
            Err(e) => {
                println!("{}", &e.to_string());
                panic!["Kapot"];
            }
        };
        Self::_from_image(device, queue, &img, Some(label))
    }

    pub fn from_gltf_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &gltf::image::Data,
        label: Option<&str>,
    ) -> Self {
        let texture_size = wgpu::Extent3d {
            width: img.width,
            height: img.height,
            depth_or_array_layers: 1,
        };

        use gltf::image::Format;

        let format = wgpu::TextureFormat::Rgba8Unorm;
        let converted_rgba: Vec<u8>;
        match img.format {
            Format::R8G8B8 => {
                // Wgpu seems to not support rgb without alpha, so add a byte for each set of rgb
                converted_rgba = img.pixels.iter().enumerate().step_by(3).map(|(i, _)| {
                    vec![
                        img.pixels[i + 0],
                        img.pixels[i + 1],
                        img.pixels[i + 2],
                        255 as u8,
                    ]
                }).collect::<Vec<Vec<u8>>>().into_iter().flatten().collect::<Vec<u8>>();
            },
            _ => panic!["Unsupported gltf::image::Format in texture::from_gltf_image(device: &wgpu::Device, queue: &wgpu::Queue, img: &gltf::image::Data, label: Option<&str>)"],
        };

        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: format,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
                label: label,
        });


        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
           &converted_rgba,
           wgpu::ImageDataLayout {
               offset: 0,
               bytes_per_row: NonZeroU32::new(4 * img.width),
               rows_per_image: NonZeroU32::new(img.height),
           },
           texture_size
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self { texture, view, sampler }
    }

    pub fn _from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers:1,
        };

        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
                label: label,
            }
        );

        // Use the queue to copy the pixel data to GPU.
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            // Pixel data.
            &rgba,
            // Define the layout of the texture:
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * dimensions.0),
                rows_per_image: NonZeroU32::new(dimensions.1),
            },
            texture_size
        );

        // Define texture view and sampler.
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self { texture, view, sampler })
    }

    pub fn _load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
    ) -> Result<Self> {
        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str();
        let img = image::open(path)?;
        Self::_from_image(device, queue, &img, label)
    }
}