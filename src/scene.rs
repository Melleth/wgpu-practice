use crate::model::*;
use crate::renderer::light::*;

use std::time::Duration;
use std::ops::Range;

use cgmath::{
    Vector3,
    Quaternion,
    Rotation3,
};

// Theorizing different types of syncs I'll need.
enum SyncJob {
    Instance {model_id: usize, instance_id: usize},
    Instances { model_id: usize, instance_ids: Range<usize> },
    Vertex,
    Index,
    Animation
}


struct SceneNode {
    position: Vector3<f32>,
    rotation: Quaternion<f32>,
    model_id: Option<usize>,
    instance_id: Option<usize>,
    children: Vec<SceneNode>
}

impl SceneNode {
    // A root node is just an empty node with no model and instances!
    pub fn new_root() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::from_axis_angle(Vector3::unit_x(), cgmath::Deg(0.0)),
            model_id: None,
            instance_id: None,
            children: vec![],
        }
    }

    pub fn add_child(&mut self, child: SceneNode) {
        self.children.push(child);
    }
}

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
    sync_queue: Vec<SyncJob>,
    graph: Option<SceneNode>,
}

impl Scene {
    pub fn empty() -> Self {
        Self {
            models: vec![],
            _lights: vec![],
            sync_queue: vec![],
            graph: None,
        }
    }

    pub fn add_model(&mut self, model: Model) {
        self.models.push(model);

        if self.models.len() == 1 {
            // If this model is the first to be added, it becomes a child of the scene root
            let mut scene_root = SceneNode::new_root();
            let model_node = SceneNode {
                position: Vector3::new(0.0, 0.0, 0.0),
                rotation: Quaternion::from_axis_angle(Vector3::unit_x(), cgmath::Deg(0.0)),
                model_id: Some(0),
                instance_id: None,
                children: vec![],
            };

            scene_root.add_child(model_node);
            self.graph = Some(scene_root);
        }
    } 


    pub fn _add_light(&mut self, light: Light) {
        self._lights.push(light);
    } 

    pub fn add_instance_of(&mut self, model_id: usize) {
        self.models[model_id].add_instance();
        let instance_id = self.models[model_id].get_num_instances() - 1;

        // TODO: come up with some Scene::sync fn that'll sync all resources that need to be sync'ed.
        //  Needs rework of resource ownership, I think...
        //self.models[id].instance_resource.sync_gpu();
        self.sync_queue.push(SyncJob::Instance { model_id, instance_id });
    }

    pub fn remove_instance_of(&mut self, id: usize) {
        // No need to sync, because we can just call draw_indexed with a smaller range?
        self.models[id].remove_instance();
    }

    // Stub of what the sync fn could look like.
    // TODO: shouldn't sync whole instance resource on a singular change. WIP
    fn sync_scene_gpu(&mut self) {
        for job in &self.sync_queue {
            match job {
                SyncJob::Instance{model_id, ..} => {
                    self.models[*model_id].instance_resource.sync_gpu();
                },
                _ => unimplemented!["Scene::sync_scene_gpu not implementend SyncJob case!"],
            }
        }

        self.sync_queue.clear();
    }

    // fn unpdate stub that would also handle animations, scenegraph updates etc.
    //  For now it just calls sync
    pub fn update(&mut self, _dt: Duration) {
        self.sync_scene_gpu();
    }
}

pub trait DrawScene {
    fn draw_scene(&mut self, scene: &Scene) -> Result<(), wgpu::SwapChainError>;
}


impl DrawScene for crate::renderer::Renderer {
    // Draws all models and their instances.
    fn draw_scene(&mut self, scene: &Scene) -> Result<(), wgpu::SwapChainError> {
        // TODO: figure out how to deal with this renderpass initialization boilerplate code.
        //  - I tried putting the code in a renderer::get_forward_pass() -> Result<(RndrPss, CmndEnc, SwpChTx), err>
        //      - But it had borrow issues as &frame.fiew is used by render_pass...
        //      - THERE HAS TO BE a "clean" way of abstracting this away tho?
        //      - Maybe do some Drop trait for a render_pass abstraction, keeping ownership to some Renderer field AND,
        //          borrowing the render_pass abstraction to draw fns that request some render_pass abstraction?
        //          - Dropping it could perhaps also mean "submit cmd encoder to queue"? <-- prolly pretty unintuitive to any1 but me tho
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