use crate::renderer::{
    texture::Texture,
    instance::{Instance, InstanceRaw},
    Renderer,
    resource::{Resource, ResourceType},
};

use anyhow::*;
use std::path::Path;
use std::ops::Range;
use wgpu::util::DeviceExt;

pub trait Vertex {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a>;
}

// Each time I look at these traits I hate them some more.
//  TODO: remove? just pass the draw functions some renderpass?
//   - Should get rid of all the lifetime annotations.
//   - Maybe ruins the potential of parallel draw invocations?
//      - Is that even a thing/worthwhile?
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
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
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
            self.set_vertex_buffer(1, model.instance_resource.get_gpu_buffer().slice(..));
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
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
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
    // For a singular model this would be resource.cpu_buffer.len() == 1 
    //  vector containing just a model matrix
    pub instance_resource: Resource<InstanceRaw>,
}

impl Model {
    pub fn load<P: AsRef<Path>>(
        renderer: &Renderer,
        path: P
    ) -> Result<Self> {
        let queue = &renderer.queue;
        let device = &renderer.device;

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
                        queue,
                        &renderer.default_bind_group_layout,
                    )
                );

                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                let mut _vertices = Vec::new();                

                
                // Read positions, put them in a ModelVertex struct.
                _vertices = if let Some(pos_iter) = reader.read_positions() {
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
                        contents: bytemuck::cast_slice(&_vertices),
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

        // This is old code. If we want to render without a scene graph, we might need it again.
        // let mut instances: = Vec::new();
        // let position = Vector3::new(0.0, 0.0, 0.0);
        // let rotation = Quaternion::from_axis_angle(Vector3::unit_z(), cgmath::Deg(0.0));
        // instances.push(Instance{position, rotation, scale: 1.0});
        // let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        //
        // Create instance resource.
        // let instance_resource = Resource::new_with_data(
        //     device.clone(), queue.clone(),
        //     instance_data,
        //     ResourceType::Vertex
        // );

        let instance_resource = Resource::new_sized(
            device.clone(),
            queue.clone(), 
            1,
            ResourceType::Vertex
        );
            

        Ok ( Self { meshes, materials, instance_resource})
    }

    pub fn add_instance(&mut self) {
        // For now we'll default the new instances to be positioned next to the
        //  previous instance.
        let prev = self.instance_resource.get_cpu_length() - 1;
        let mut new = if let Some(prev_raw_instance) = self.instance_resource.local_at(prev) {
            Instance::from(prev_raw_instance)
        } else {
            Instance::default()
        };

        new.position.x += 1.0;

        self.instance_resource.add_to_buffer(vec![new.to_raw()]);
    }

    pub fn _change_instance(&mut self, id: usize, instance: Instance) {
        if let Some(i) = self.instance_resource._mut_local_at(id) {
            *i = instance.to_raw();
        }
    }

    pub fn change_instance_raw(&mut self, id: usize, instance_raw: cgmath::Matrix4<f32>) {
        if let Some(i) = self.instance_resource._mut_local_at(id) {
            *i = InstanceRaw { model: instance_raw.into() };
        }
    }

    pub fn _remove_instance(&mut self) {
        // Remove the last instance for testing purposes.
        self.instance_resource._remove_from_buffer(self.instance_resource.get_cpu_length() - 1);
    }

    pub fn get_num_instances(&self) -> usize {
        self.instance_resource.get_cpu_length()
    }

}

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
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float{ filterable: true },
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
                        filtering: true,
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

    fn create_bind_group_with_layout(
        textures: Vec<&Texture>,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        let mut bind_group_entries = Vec::new();
        for (i, t) in textures.iter().enumerate() {
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
        device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &bind_group_entries,
                label: Some("bind_group"),
            }
        )
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
            Some(Texture::from_gltf_image(device, queue, img, Some("diffuse_texture")))
        } else {
            None
        };

        let metallic_roughness_texture = if let Some(tex) = pbr_mr.metallic_roughness_texture() {
            let img = &images[tex.texture().index()];
            Some(Texture::from_gltf_image(device, queue, img, Some("metallic_roughness_texture")))
        } else { None };
        
        let normal_texture = if let Some(tex) = material.normal_texture() {
            let img = &images[tex.texture().index()];
            Some(Texture::from_gltf_image(device, queue, img, Some("normal_texture")))
        } else { None };

        let occlusion_texture = if let Some(tex) = material.occlusion_texture() {
            let img = &images[tex.texture().index()];
            Some(Texture::from_gltf_image(device, queue, img, Some("occlusion_texture")))
        } else { None };

        // Figure out what textures are present which we need to request binds for.
        if let Some(ref t) = diffuse_texture {
            textures.push(t);
        }

        if let Some(ref t) = normal_texture {
            textures.push(t);
        }
        //let (bind_group_layout, bind_group) = Material::create_bind_group_for_textures(textures, device);
        let bind_group = Material::create_bind_group_with_layout(textures, device, bind_group_layout);

        let name = material.name().unwrap_or("Very cool material name.").to_string();

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

// TODO: Do I want to use the renderer::Resource abstraction for mesh buffer?
//  - Seems unnessecary as mesh vertices, indices dont really change...
//  - Would ruin the draw code.
//  - Yeah probably a bad idea.
//  - It might be worthwhile to employ resources for animation skinning though... If I get around to it.
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

impl Vertex for ModelVertex {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3, },
                wgpu::VertexAttribute { offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress, shader_location: 1, format: wgpu::VertexFormat::Float32x2, },
                wgpu::VertexAttribute { offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress, shader_location: 2, format: wgpu::VertexFormat::Float32x3, },
                wgpu::VertexAttribute { offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress, shader_location: 3, format: wgpu::VertexFormat::Float32x3, },
                wgpu::VertexAttribute { offset: std::mem::size_of::<[f32; 11]>() as wgpu::BufferAddress, shader_location: 4, format: wgpu::VertexFormat::Float32x3, },
            ]
        }
    }
}