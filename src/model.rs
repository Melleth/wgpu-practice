use crate::texture;
use anyhow::*;
use std::path::Path;
use std::ops::Range;
use wgpu::util::DeviceExt;

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

pub trait DrawModel<'a, 'b>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup
    );

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        material: &'b Material,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup
    );

    fn draw_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );

    
}
impl<'a, 'b> DrawModel<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup
    ) {
        self.draw_mesh_instanced(mesh, 0..1, material, uniforms, light);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        material: &'b Material,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup
    ){
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..));
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, &uniforms, &[]);
        self.set_bind_group(2, &light, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, uniforms, light);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, instances.clone(), material, uniforms, light);
        }
    }
}

pub trait DrawLight<'a, 'b>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b Mesh,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) where
        'b: 'a;

    fn draw_light_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_light_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLight<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b Mesh,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_light_mesh_instanced(mesh, 0..1, uniforms, light);
    }

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..));
        self.set_bind_group(0, uniforms, &[]);
        self.set_bind_group(1, light, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_light_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_light_model_instanced(model, 0..1, uniforms, light);
    }
    fn draw_light_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_light_mesh_instanced(mesh, instances.clone(), uniforms, light);
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
    tangent: [f32; 3],
    bitangent: [f32; 3],
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P
    ) -> Result<Self> {
        let (document, buffers, images) = gltf::import(path.as_ref())?;

        let mut meshes = Vec::new();
        let mut materials = Vec::new();

        for mesh in document.meshes() {
            for primitive in mesh.primitives() {

                // Deal with material.
                materials.push(
                    Material::from_gltf(
                        primitive.material(),
                        &images,
                        device,
                        queue
                    )
                );

                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                let mut vertices = Vec::new();                

                
                // Read positions, put them in a ModelVertex struct.
                vertices = if let Some(pos_iter) = reader.read_positions() {
                    if let Some(tc_iter) = reader.read_tex_coords(0) {
                        if let Some(n_iter) = reader.read_normals() {
                            if let Some(tangent_iter) = reader.read_tangents() {
                                pos_iter.zip(
                                    tc_iter.into_f32().zip(
                                        n_iter.zip(
                                            tangent_iter))).map(|(p, (tc, (n, t)))| {
                                                let tangent = cgmath::Vector3::from([t[0], t[1], t[2]]);
                                                let normal = cgmath::Vector3::from(n);
                                                let bitangent = tangent.cross(normal);
                                                ModelVertex {
                                                    position: p,
                                                    tex_coords: tc,
                                                    normal: n,
                                                    tangent: tangent.into(),
                                                    bitangent: bitangent.into(),
                                                }
                                            }).collect()
                            } else { Vec::new() }
                        } else { Vec::new() }
                    } else { Vec::new() }
                } else { Vec::new() };

                // Read indices.
                let mut indices = Vec::new();
                if let Some(iter) = reader.read_indices() {
                    for index in iter.into_u32() {
                        indices.push(index);
                    }
                }

                // Create buffers.
                let vertex_buffer = device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{:?} Vertex Buffer", path.as_ref())),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsage::VERTEX,
                    }
                );

                let index_buffer = device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{:?} Index Buffer", path.as_ref())),
                        contents: bytemuck::cast_slice(&indices),
                        usage: wgpu::BufferUsage::INDEX,
                    }
                );

                meshes.push(Mesh {
                    name: mesh.name().unwrap_or("Cool mesh name").to_string(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: indices.len() as u32,
                    material: materials.len() - 1
                });
            }
        }
        Ok ( Self { meshes, materials })
    }

    pub fn get_bind_group_layouts<'a>(&'a self) -> Vec<&'a wgpu::BindGroupLayout> {
        let mut bgls = Vec::new();
        for m in &self.materials {
            bgls.push(&m.bind_group_layout);
        }
        bgls
    }
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: Option<texture::Texture>,
    pub metallic_roughness_texture: Option<texture::Texture>,
    pub occlusion_texture: Option<texture::Texture>,
    pub normal_texture: Option<texture::Texture>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    fn create_bind_group_for_textures(
        textures: Vec<&texture::Texture>,
        device: &wgpu::Device
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        // Define texture bindgroup layout, and the bind group.
        let mut layout_desc_entries = Vec::new();
        let mut bind_group_entries = Vec::new();

        for (i, t) in textures.iter().enumerate() {
            layout_desc_entries.push(
                wgpu::BindGroupLayoutEntry {
                    binding: (i * 2) as u32,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                    },
                    count: None,
                }
            );
            layout_desc_entries.push(
                wgpu::BindGroupLayoutEntry {
                    binding: (i * 2 + 1) as u32,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                    },
                    count: None,
                }
            );

            bind_group_entries.push(
                wgpu::BindGroupEntry {
                    binding: (i * 2) as u32,
                    resource: wgpu::BindingResource::TextureView(&t.view)
                }
            );

            bind_group_entries.push(
                wgpu::BindGroupEntry {
                    binding: (i * 2 + 1) as u32,
                    resource: wgpu::BindingResource::Sampler(&t.sampler)
                }
            );
        }

        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &layout_desc_entries.as_slice(),
                label: Some("bind_group_layout"),
            }
        );

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &bind_group_entries,
                label: Some("bind_group"),
            }
        );

        (bind_group_layout, bind_group)
    }

    pub fn from_gltf(
        material: gltf::material::Material,
        images: &Vec<gltf::image::Data>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let mut textures = Vec::new();

        let pbr_mr = material.pbr_metallic_roughness();
        let diffuse_texture = if let Some(tex) = pbr_mr.base_color_texture() {
            let img = &images[tex.texture().index()];
            Some(texture::Texture::from_gltf_image(device, queue, img, Some("diffuse_texture")))
        } else {
            None
        };

        let metallic_roughness_texture = if let Some(tex) = pbr_mr.metallic_roughness_texture() {
            let img = &images[tex.texture().index()];
            Some(texture::Texture::from_gltf_image(device, queue, img, Some("metallic_roughness_texture")))
        } else { None };
        
        let normal_texture = if let Some(tex) = material.normal_texture() {
            let img = &images[tex.texture().index()];
            Some(texture::Texture::from_gltf_image(device, queue, img, Some("normal_texture")))
        } else { None };

        let occlusion_texture = if let Some(tex) = material.occlusion_texture() {
            let img = &images[tex.texture().index()];
            Some(texture::Texture::from_gltf_image(device, queue, img, Some("occlusion_texture")))
        } else { None };

        // Figure out what textures are present which we need to request binds for.
        if let Some(ref t) = diffuse_texture {
            textures.push(t);
        }

        if let Some(ref t) = normal_texture {
            textures.push(t);
        }
        let (bind_group_layout, bind_group) = Material::create_bind_group_for_textures(textures, device);

        let name = material.name().unwrap_or("Very cool material name.").to_string();

        Self { 
            name,
            diffuse_texture,
            metallic_roughness_texture,
            occlusion_texture,
            normal_texture,
            bind_group_layout,
            bind_group,
        }
    }
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        let vert_atr_arr = wgpu::vertex_attr_array![
                0 => Float3,
                1 => Float2,
                2 => Float3
            ];

        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float3, },
                wgpu::VertexAttributeDescriptor { offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress, shader_location: 1, format: wgpu::VertexFormat::Float2, },
                wgpu::VertexAttributeDescriptor { offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress, shader_location: 2, format: wgpu::VertexFormat::Float3, },
                wgpu::VertexAttributeDescriptor { offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress, shader_location: 3, format: wgpu::VertexFormat::Float3, },
                wgpu::VertexAttributeDescriptor { offset: std::mem::size_of::<[f32; 11]>() as wgpu::BufferAddress, shader_location: 4, format: wgpu::VertexFormat::Float3, },

            ]
        }
    }
}