use crate::renderer::texture::Texture;

pub struct Material {
    pub name: String,
    pub diffuse_texture: Option<Texture>,
    pub metallic_roughness_texture: Option<Texture>,
    pub occlusion_texture: Option<Texture>,
    pub normal_texture: Option<Texture>,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    fn _create_bind_group_for_textures(
        textures: Vec<&Texture>,
        device: &wgpu::Device,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        // Define texture bindgroup layout, and the bind group.
        let mut layout_desc_entries = Vec::new();
        let mut bind_group_entries = Vec::new();

        for (i, t) in textures.iter().enumerate() {
            layout_desc_entries.push(wgpu::BindGroupLayoutEntry {
                binding: (i * 2) as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            });
            layout_desc_entries.push(wgpu::BindGroupLayoutEntry {
                binding: (i * 2 + 1) as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler {
                    comparison: false,
                    filtering: true,
                },
                count: None,
            });

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: (i * 2) as u32,
                resource: wgpu::BindingResource::TextureView(&t.view),
            });

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: (i * 2 + 1) as u32,
                resource: wgpu::BindingResource::Sampler(&t.sampler),
            });
        }

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &layout_desc_entries.as_slice(),
            label: Some("bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &bind_group_entries,
            label: Some("bind_group"),
        });

        (bind_group_layout, bind_group)
    }

    fn create_bind_group_with_layout(
        textures: Vec<&Texture>,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        let mut bind_group_entries = Vec::new();
        for (i, t) in textures.iter().enumerate() {
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: (i * 2) as u32,
                resource: wgpu::BindingResource::TextureView(&t.view),
            });

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: (i * 2 + 1) as u32,
                resource: wgpu::BindingResource::Sampler(&t.sampler),
            });
        }
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &bind_group_entries,
            label: Some("bind_group"),
        })
    }

    pub fn from_gltf(
        material: gltf::material::Material,
        images: &Vec<gltf::image::Data>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let mut textures = Vec::new();

        let pbr_mr = material.pbr_metallic_roughness();
        let diffuse_texture = if let Some(tex) = pbr_mr.base_color_texture() {
            let img = &images[tex.texture().index()];
            Some(Texture::from_gltf_image(
                device,
                queue,
                img,
                Some("diffuse_texture"),
            ))
        } else {
            None
        };

        let metallic_roughness_texture = if let Some(tex) = pbr_mr.metallic_roughness_texture() {
            let img = &images[tex.texture().index()];
            Some(Texture::from_gltf_image(
                device,
                queue,
                img,
                Some("metallic_roughness_texture"),
            ))
        } else {
            None
        };

        let normal_texture = if let Some(tex) = material.normal_texture() {
            let img = &images[tex.texture().index()];
            Some(Texture::from_gltf_image(
                device,
                queue,
                img,
                Some("normal_texture"),
            ))
        } else {
            None
        };

        let occlusion_texture = if let Some(tex) = material.occlusion_texture() {
            let img = &images[tex.texture().index()];
            Some(Texture::from_gltf_image(
                device,
                queue,
                img,
                Some("occlusion_texture"),
            ))
        } else {
            None
        };

        // Figure out what textures are present which we need to request binds for.
        if let Some(ref t) = diffuse_texture {
            textures.push(t);
        }

        if let Some(ref t) = normal_texture {
            textures.push(t);
        }
        //let (bind_group_layout, bind_group) = Material::create_bind_group_for_textures(textures, device);
        let bind_group =
            Material::create_bind_group_with_layout(textures, device, bind_group_layout);

        let name = material
            .name()
            .unwrap_or("Very cool material name.")
            .to_string();

        Self {
            name,
            diffuse_texture,
            metallic_roughness_texture,
            occlusion_texture,
            normal_texture,
            bind_group,
        }
    }
}
