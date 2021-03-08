use crate::model::*;
use crate::camera::*;
use crate::renderer::light::*;

pub struct Scene {
    models: Vec<Model>,
    lights: Vec<Light>,
}

impl Scene {
    pub fn empty() -> Self {
        Self {
            models: vec![],
            lights: vec![],
        }
    }

    pub fn add_model(&mut self, model: Model) {
        self.models.push(model);
    } 


    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    } 

    pub fn add_instance_of(&mut self, id: usize, device: &wgpu::Device) {
        self.models[id].add_instance(device);
    }
}

pub trait DrawScene {
    fn draw_scene(&mut self, scene: &Scene) -> Result<(), wgpu::SwapChainError>;
}


impl DrawScene for crate::renderer::Renderer {
    fn draw_scene(&mut self, scene: &Scene) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain
            .get_current_frame()?
            .output;
        
        // Encoders can create a commandbuffer.
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Scene render pass"),
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

        render_pass.set_pipeline(&self.render_pipeline);
        for m in &scene.models {
            render_pass.draw_model_instanced(&m, 0..m.instances.len() as u32, &self.uniform_bind_group, &self.light_bind_group);
        }
        drop(render_pass);
        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}