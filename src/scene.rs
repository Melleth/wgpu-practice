use crate::model::*;
use crate::renderer::light::*;
use crate::renderer::{
    instance::Instance,
    instance::InstanceRaw,
};

use std::time::Duration;
use std::ops::Range;

use cgmath::{
    Vector3,
    Matrix4,
    Quaternion,
    Rotation3,
};

// Theorizing different types of syncs I'll need.
#[derive(Debug)]
#[allow(dead_code)]
enum SyncJob {
    Instance {model_id: usize, instance_id: usize},
    // When we adjust multiple instances at once (e.g. grpah parent changes position)
    //  we can try to update the resource with one call.
    Instances { model_id: usize, instance_ids: Range<usize> },
    Vertex,
    Index,
    Animation
}

// Consider all these fields LOCAL ONLY!!!
//  The "world matrix" aka Instance will be calculated when something
//    changes and needs to be synced to gpu
struct SceneNode {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: f32,
    pub model_id: Option<usize>,
    pub instance_id: Option<usize>,
    pub children: Vec<SceneNode>,
    pub changed: bool,
}

impl SceneNode {
    // A root node is just an empty node with no model and instances!
    pub fn new_root() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::from_axis_angle(Vector3::unit_x(), cgmath::Deg(0.0)),
            scale: 1.0,
            model_id: None,
            instance_id: None,
            children: vec![],
            changed: false,
        }
    }

    pub fn new_instance_node(model_id: usize, instance_id: usize) -> Self {
        SceneNode {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::from_axis_angle(Vector3::unit_x(), cgmath::Deg(0.0)),
            scale: 10.0,
            model_id: Some(model_id),
            instance_id: Some(instance_id),
            children: vec![],
            changed: true,
        }
    }

    pub fn add_child(&mut self, child: SceneNode) {
        self.children.push(child);
    }

    // For now sets the changed flag, because propagating the values is
    //  handled by SceneNode::collect_changed(). This is probably obsolete but
    //  current design of fetching sync jobs relies on this flag.
    //  TODO: come up with a nicer way that doesn't traverse the tree on each change
    //      --> possible sync job can be inferred by changed status of parent.
    pub fn update_children(
        &mut self,
        //_parent_pos: Vector3<f32>,
        //_parent_rot: Quaternion<f32>,
        //_parent_scale: f32,
    ) {
        for child in &mut self.children {
            child.changed = true;
            child.update_children();
        }
    }

    // Collects all changed node model and instance ids and their new world views as instances.
    pub fn collect_changed(&mut self, parent_instance: Instance) -> Vec<(Option<usize>, Option<usize>, Instance)> {
        let mut result = vec![];

        // Construct world-v parameters.
        let mat = Matrix4::from(Instance {
            position: self.position,
            rotation: self.rotation,
            scale: self.scale,
        }.to_raw().model);

        let parent_mat = Matrix4::from(parent_instance.to_raw().model);
        let accumulated_mat = parent_mat * mat;
        let accumulated_instance = Instance::from(InstanceRaw { model: accumulated_mat.into() });

        if self.changed {
            result.push((self.model_id, self.instance_id, accumulated_instance));
            // Don't forget to unset this flag :))
            self.changed = false;
        }

        for n in &mut self.children {
            result.append(&mut n.collect_changed(accumulated_instance));
        }

        result
    }

    pub fn translate<T: Into<f32>>(&mut self, x: T, y: T, z: T) {
        self.position.x += x.into();
        self.position.y += y.into();
        self.position.z += z.into();
        self.changed = true;
        self.update_children();
    }

    pub fn _set_scale<T: Into<f32>>(&mut self, scale: T) {
        self.scale = scale.into();
        self.changed = true;
        self.update_children()
    }

    pub fn rotate(&mut self, rotation: Quaternion<f32>) {
        self.rotation = self.rotation * rotation;// self.rotation;
        self.changed = true;
        self.update_children();
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
    graph: SceneNode,
}

impl Scene {
    pub fn empty() -> Self {
        Self {
            models: vec![],
            _lights: vec![],
            sync_queue: vec![],
            graph: SceneNode::new_root(),
        }
    }

    pub fn add_model(&mut self, model: Model) {
        self.models.push(model);

        if self.models.len() == 1 {
            // If this model is the first to be added, it becomes a child of the scene root
            let model_node = SceneNode {
                position: Vector3::new(0.0, 0.0, 0.0),
                rotation: Quaternion::from_axis_angle(Vector3::unit_x(), cgmath::Deg(0.0)),
                scale: 10.0,
                model_id: Some(0),
                instance_id: Some(0),
                children: vec![],
                changed: true,
            };
            self.graph.add_child(model_node);
        }
    } 


    pub fn _add_light(&mut self, light: Light) {
        self._lights.push(light);
    } 

    // Creates a new instance of a previously loaded model and adds it to the
    //  scene graph root node children.
    pub fn add_instance_of(&mut self, model_id: usize) {
        self.models[model_id].add_instance();
        let instance_id = self.models[model_id].get_num_instances() - 1;

        let mut scene_node = SceneNode::new_instance_node(model_id, instance_id);

        // Offset the node a bit so we ccan actually see them
        scene_node.position.x = instance_id as f32 * 1.0;
        
        self.graph.add_child(scene_node);

        self.sync_queue.push(SyncJob::Instance { model_id, instance_id });
    }

    pub fn _remove_instance_of(&mut self, id: usize) {
        // No need to sync, because we can just call draw_indexed with a smaller range?
        self.models[id]._remove_instance();
    }

    // Stub of collecting sync jobs.
    fn collect_sync_jobs(&mut self) {
        // Construct root instance, so we can propagate changes down the scene graph/tree
        let root_instance = Instance {
            position: self.graph.position,
            rotation: self.graph.rotation,
            scale: self.graph.scale,
        };

        let changed =  self.graph.collect_changed(root_instance);
        // for i in &changed {
        //     dbg!(i);
        // }

        for (mid, iid, instance) in changed {
            if let Some(model_id) = mid {
                if let Some(instance_id) = iid {
                    self.models[model_id].change_instance(instance_id, instance);
                    self.sync_queue.push(SyncJob::Instance { model_id, instance_id });
                }
            }
        }
    }

    // Stub of what the sync fn could look like.
    // TODO: shouldn't sync whole instance resource on a singular change. WIP
    //  - Also syncs the resource multile times on changes of seperate instances of the resource.
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
        //self.graph.scale += 0.1;
        self.graph.rotate(Quaternion::from_axis_angle(Vector3::unit_z(), cgmath::Deg(1.0)));
        self.graph.translate(0.0, 0.0001, 0.0);
        self.graph.update_children();
        self.graph.changed = true;
        self.collect_sync_jobs();
        self.sync_scene_gpu();
    }

    // Rotates root node and its childeren
    pub fn _set_rotation_of_node(&mut self, rotation: Quaternion<f32>) {
        self.graph.rotate(rotation);
        self.graph.update_children();
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