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
    fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material, uniforms: &'b wgpu::BindGroup);
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        material: &'b Material,
        uniforms: &'b wgpu::BindGroup,
    );
}
impl<'a, 'b> DrawModel<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material, uniforms: &'b wgpu::BindGroup) {
        self.draw_mesh_instanced(mesh, 0..1, material, uniforms);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        material: &'b Material,
        uniforms: &'b wgpu::BindGroup,
    ){
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..));
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, &uniforms, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
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
                if let Some(iter) = reader.read_positions() {
                    for (i, vertex_position) in iter.enumerate() {
                        vertices.push(ModelVertex {                            
                            position: vertex_position,
                            tex_coords: [0.0, 0.0],
                            normal: [0.0, 0.0, 0.0]
                        });
                    }
                }

                // Read tex_coords, put them in the struct as well.
                if let Some(iter) = reader.read_tex_coords(0) {                
                    for (i, tex_coords) in iter.into_f32().enumerate() {
                        vertices[i].tex_coords = tex_coords;
                    }
                }                    

                // Read vertex normals, put them in the struct.
                if let Some(iter) = reader.read_normals() {
                    for (i, normal) in iter.enumerate() {
                        vertices[i].normal = normal;
                    }
                }

                // Read indices.
                let mut indices = Vec::new();
                if let Some(iter) = reader.read_indices() {
                    for index in iter.into_u32() {
                        indices.push(index);
                    }
                }
                println!("Model is composed of {} triangles", indices.len() / 3);

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
                    // Hardcode that shit for now.
                    material: 0
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
            Some ( texture::Texture::from_gltf_image(
                device,
                queue,
                img,
                Some("diffuse_texture")
            ))
        } else {
            None
        };

        let normal_texture = if let Some(tex) = material.normal_texture() {
            let img = &images[tex.texture().index()];
            Some ( texture::Texture::from_gltf_image( device, queue, img, Some("normal_texture")) )
        } else { None };

        if let Some(ref t) = diffuse_texture {
            textures.push(t);
        }

        if let Some(ref t) = normal_texture {
            textures.push(t);
        }
        let (bind_group_layout, bind_group) = Material::create_bind_group_for_textures(textures, device);

        let name = material.name().unwrap_or("Very cool material name.").to_string();

        Self { name, diffuse_texture, normal_texture, bind_group_layout, bind_group, }
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
            ]
        }
    }
}