pub mod resource;
pub mod instance;
pub mod texture;
pub mod light;

use std::time::Duration;
use std::sync::Arc;

use crate::camera::{Camera, CameraController, Projection};
use crate::model::{Vertex, ModelVertex};

use resource::{
    Resource,
    ResourceType,
};

use instance::{
    InstanceRaw,
};

use texture::{
    Texture,
};
use light::{
    Light,
};

use cgmath::prelude::*;
use cgmath::{Vector3, Matrix4, Quaternion};

use winit::{
    event::*,
    window::{Window},
};

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (camera.projection.calculate_matrix() * camera.calculate_matrix()).into();
    }
}

pub struct Renderer {
    surface: wgpu::Surface,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    pub clear_color: wgpu::Color,
    pub render_pipeline: wgpu::RenderPipeline,
    pub light_render_pipeline: wgpu::RenderPipeline,
    camera_controller: CameraController,
    pub camera: Camera,
    pub uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub depth_texture: texture::Texture,
    light: Light,
    light_buffer: wgpu::Buffer,
    pub light_bind_group: wgpu::BindGroup,
    mouse_pressed: bool,
    pub default_bind_group_layout: wgpu::BindGroupLayout,
}

impl Renderer {
    pub async fn new (window: &Window) -> Self {
        let size = window.inner_size();
        // Handle to gpu
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            },
        ).await.unwrap();

        // Use the adapter to request device and queue.
        //  You can view available features through device.features()
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Device descriptor"),
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ).await.unwrap();

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        // Define the camera.
        let camera_controller = CameraController::new(4.0, 0.4);
        let mut camera = Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection = Projection::new(sc_desc.width, sc_desc.height, cgmath::Deg(45.0), 0.1, 100.0);
        camera.projection = projection;


        // Uniform definitons start here
        let uniforms = Uniforms::new();
        let _uniform_resource = Resource::new_with_data(
            Arc::clone(&device),
            Arc::clone(&queue),
            vec![uniforms],
            ResourceType::Uniform,
        );

        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }
        );

        let uniform_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries : &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: Some("uniform_bind_group_layout"),
            }
        );

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("uniform_bind_group"),
        });

        let depth_texture = texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

        let clear_color = wgpu::Color::BLACK;


        // Light stuff starts here.
        // TODO: move light stuff to scene, add support for multiple lights and figure out
        //  How to set up a deffered pipeline. <-- will take some research.
        let light = Light {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
        };

        let light_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light vertex buffer"),
                contents: bytemuck::cast_slice(&[light]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::VERTEX,
            }
        );

        let light_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            }
        );

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: None,
        });

        // Load precompiled shaders (see build.rs), set up render pipeline.
        let vs_module = device.create_shader_module(&wgpu::include_spirv!("shader_src/shader.vert.spv"));
        let fs_module = device.create_shader_module(&wgpu::include_spirv!("shader_src/shader.frag.spv"));

        // Get the pre-defined bindgroup layouts. Not sure if pre-defining is the way, but so far so good.
        let default_bind_group_layout = Self::default_bindgroup_layout(&device);
        let mut bind_group_layouts = vec![&default_bind_group_layout];
        bind_group_layouts.push(&uniform_bind_group_layout);
        bind_group_layouts.push(&light_bind_group_layout);

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: bind_group_layouts.as_slice(),
            push_constant_ranges: &[],
        });

        let render_pipeline = Self::create_render_pipeline(
            &device,
            &render_pipeline_layout,
            sc_desc.format,
            Some(texture::Texture::DEPTH_FORMAT),
            &vs_module,
            &fs_module
        );

        let light_render_pipeline = {
            let light_render_pipeline_layout = device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Light Pipeline Layout"),
                    bind_group_layouts: &[
                        &uniform_bind_group_layout,
                        &light_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                }
            );

            let light_vs_module = device.create_shader_module(&wgpu::include_spirv!("shader_src/light.vert.spv"));
            let light_fs_module = device.create_shader_module(&wgpu::include_spirv!("shader_src/light.frag.spv"));

            Self::create_render_pipeline(
                &device, 
                &light_render_pipeline_layout, 
                sc_desc.format, 
                Some(Texture::DEPTH_FORMAT), 
                &light_vs_module, 
                &light_fs_module,
            )
        };



        // Instancing stuff starts here.
        //const NUM_INSTANCES_PER_ROW: u32 = 50;
        //const _NUM_INSTANCES: u32 = NUM_INSTANCES_PER_ROW * NUM_INSTANCES_PER_ROW;
        //const INSTANCE_DISPLACEMENT: Vector3<f32> = Vector3::new(NUM_INSTANCES_PER_ROW as f32 * 0.5, 0.0, NUM_INSTANCES_PER_ROW as f32 * 0.5); 
        //let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
        //    (0..NUM_INSTANCES_PER_ROW).map(move |x| {
        //        let position = Vector3 { x: x as f32, y: 0.0, z: z as f32} - INSTANCE_DISPLACEMENT;
        //        let rotation = if position.is_zero() {
        //            Quaternion::from_axis_angle(Vector3::unit_z(), cgmath::Deg(0.0))
        //        } else {
        //            cgmath::Quaternion::from_axis_angle(position.clone().normalize(), cgmath::Deg(45.0))
        //        };
        //        Instance {
        //            position, rotation,
        //        }
        //    })
        //}).collect::<Vec<_>>();

        //let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();


        Self { 
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            clear_color,
            render_pipeline,
            light_render_pipeline,
            camera,
            camera_controller,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            depth_texture,
            light,
            light_buffer,
            light_bind_group,
            mouse_pressed: false,
            default_bind_group_layout,
        }
    }

    pub fn resize(&mut self, new_size: Option<winit::dpi::PhysicalSize<u32>>) {
        // This fn also deals with "lost" swap chain, so pick the previously known
        //  size as a default value if we don't provide any.
        let ns = if let Some(x) = new_size {
            x
        } else {
            self.size
        };

        // Update the new size to state and the swapchain descriptor.
        self.size = ns;
        self.sc_desc.width = ns.width;
        self.sc_desc.height = ns.height;

        // Recreate textures that are screen space buffers. (depth buffer e.g.)
        self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);

        // Update the projection
        self.camera.projection.resize(ns.width, ns.height);
    }


    pub fn input(&mut self, event: &WindowEvent) -> bool {
        // I'd merge this to Renderer::input_mouse_movement because splitting the mouse and keyboard handling
        //  makes no sense, but unfortunately: https://github.com/rust-windowing/winit/issues/1470
        match event {
            WindowEvent::KeyboardInput {
                input,
                ..
            } => {
                self.camera_controller.process_keyboard(input.virtual_keycode.unwrap(), input.state)
            }
            _ => false,
        }
    }

    pub fn input_mouse_movement(&mut self, event: &DeviceEvent) -> bool{
        // See State::input why this is seperate.
        match event {
            DeviceEvent::MouseMotion {
                delta
            } => {
                if self.mouse_pressed {
                    self.camera_controller.process_mouse(delta.0, delta.1);
                }
                true
            }
            DeviceEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(&delta);
                true
            }
            DeviceEvent::Button {
                button: 1,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false
        }
    }

    pub fn update(&mut self, dt: Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.uniforms.update_view_proj(&self.camera);

        // Update the light
        let old_position: Vector3<_> = self.light.position.into();
        self.light.position = (Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(60.0 * dt.as_secs_f32())) * old_position).into();

        self.queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[self.light]));
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));
    }


    // For now this isn't really used as all the handles to the resources are owned by scene.
    //  Maybe come up with something to keep scene an abstract representation of the scene
    //  without any handles to gpu buffers etc? How?... Want to prevent introducing
    //  refference lifetime bloat.
    pub fn _render(&mut self) -> Result<(), wgpu::SwapChainError> {
        // Analog of ogl fbo I guess?
        let frame = self.swap_chain
            .get_current_frame()?
            .output;
        
        // Encoders can create a commandbuffer.
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        let mut _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Default Renderpass"),
            color_attachments: &[
                wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: true,
                    }
                }
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                attachment: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });


        // Draw light (as an avocado... :o )
        // render_pass.set_pipeline(&self.light_render_pipeline);
        // render_pass.set_vertex_buffer(1, self.light_buffer.slice(..));

        // render_pass.draw_light_model(
        //     &self.gltf_model,
        //     &self.uniform_bind_group,
        //     &self.light_bind_group,
        // );

        // render_pass.set_pipeline(&self.render_pipeline);
        // // Set the "model_matrix" atribute:
        //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        // // Draw mesh instances.
        // render_pass.draw_model_instanced(
        //     &self.gltf_model,
        //     0..self.instances.len() as u32,
        //     &self.uniform_bind_group,
        //     &self.light_bind_group
        // );


        drop(_render_pass);
        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[ModelVertex::layout(), InstanceRaw::layout()]
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[
                    wgpu::ColorTargetState {
                        format: color_format,
                        color_blend: wgpu::BlendState::REPLACE,
                        alpha_blend: wgpu::BlendState::REPLACE,
                        write_mask: wgpu::ColorWrite::ALL,
                    },
                ],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                strip_index_format: None,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: depth_format.map(|format| {
                wgpu::DepthStencilState {
                    format: format,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                    clamp_depth: false,
                }
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            }
        })
    }

    pub fn _get_size(&self) -> (u32, u32) {
        (self.sc_desc.width, self.sc_desc.height)
    }

    // Creates a default layout for 4 texture views and their samplers.
    fn default_bindgroup_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let layout_entries = (0..2).step_by(1).map(|i| {
            vec![
                wgpu::BindGroupLayoutEntry {
                    binding: (i * 2) as u32,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float{ filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: (i * 2 + 1) as u32,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None
                }
            ]
        }).collect::<Vec<Vec<_>>>().into_iter().flatten().collect::<Vec<wgpu::BindGroupLayoutEntry>>();

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &layout_entries,
            label: Some("default_bindgroup_lauout")
        })
    }

}