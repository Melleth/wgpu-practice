use crate::model::*;
use crate::renderer::light::*;

// TODO: Adding a scene graph would be really nice.
//  - Scene graph nodes should be seperate from the models...
//      - But contain a ref to a model?
//      - That way I can just adjust the scene graph each frame and
//        collect all changed nodes and use the refs to figure out what needs to
//        be synced?
//          - Maybe an enum NodeChange would help? 
pub struct Scene {
    pub models: Vec<Model>,
    _lights: Vec<Light>,
}

impl Scene {
    pub fn empty() -> Self {
        Self {
            models: vec![],
            _lights: vec![],
        }
    }

    pub fn add_model(&mut self, model: Model) {
        self.models.push(model);
    } 


    pub fn _add_light(&mut self, light: Light) {
        self._lights.push(light);
    } 

    pub fn add_instance_of(&mut self, id: usize) {
        self.models[id].add_instance();

        // TODO: come up with some Scene::sync fn that'll sync all resources that need to be sync'ed.
        //  Needs rework of resource ownership, I think...
        self.models[id].instance_resource.sync_gpu();
    }

    pub fn remove_instance_of(&mut self, id: usize) {
        // No need to sync, because we can just call draw_indexed with a smaller range?
        self.models[id].remove_instance();
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
            render_pass.draw_model_instanced(&m, 0..m.get_num_instances() as u32, &self.uniform_bind_group, &self.light_bind_group);
        }

        drop(render_pass);
        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}