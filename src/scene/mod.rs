pub mod scenenode;

use crate::model::*;
use crate::renderer::light::*;
use scenenode::*;

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

pub struct Scene {
    pub models: Vec<Model>,
    _lights: Vec<Light>,
    sync_queue: Vec<SyncJob>,
    graph: SceneNode,
}

impl Scene {
    // creates a scene graph with a sun, earth and moon for demonstration purposes.
    //  (provided a base model is already added.)
    pub fn make_galaxy(&mut self) {
        if self.models.len() == 1 {
            // Create positioning nodes.
            let mut solar_system = SceneNode::default();
            let mut planet_orbit = SceneNode {  position: Vector3::new(2.0, 0.0, 0.0), ..Default::default() };
            let mut moon_orbit = SceneNode {  position: Vector3::new(0.75, 0.0, 0.0), ..Default::default() };


            // Create instances and retrieve their ids.
            let sun = self.add_instance(0);
            let earth = self.add_instance(0);
            let moon = self.add_instance(0);

            // Add the instances as nodes to their positioning nodes.
            solar_system.add_child(SceneNode { model_id: Some(0), instance_id: Some(sun), scale: 10.0, ..Default::default()});
            planet_orbit.add_child(SceneNode {model_id: Some(0), instance_id: Some(earth), scale: 5.0, ..Default::default()});
            moon_orbit.add_child(SceneNode { model_id: Some(0), instance_id: Some(moon), scale: 2.0, ..Default::default()});

            // Set up final scene graph
            planet_orbit.add_child(moon_orbit);
            solar_system.add_child(planet_orbit);

            self.graph = solar_system;
        } else {
            panic!("No base model found!");
        }
    }

    // Hardcode some node changes that demonstrate a moon orbiting a planet
    //  which is orbiting a star. (they're all avaocado rn bite me)
    pub fn animate_galaxy(&mut self, _dt: Duration) {
        // Fetch the nodes by getting mutable refs to the slices.
        // It's a bit dumb but necessary(?) as we can't just do multiple mutable borrows of
        //  individual items in a vector. #JustRustThings
        let (sun, planet_orbit) = self.graph.children.split_at_mut(1);
        planet_orbit[0].rotate(Quaternion::from_axis_angle(Vector3::unit_z(), cgmath::Deg(0.1)));
        let (_planet, moon_orbit) = planet_orbit[0].children.split_at_mut(1);

        // rotate the nodes... needs indices because we're getting slices form split_at_mut.. *<|8D
        sun[0].rotate(Quaternion::from_axis_angle(Vector3::unit_x(), cgmath::Deg(1.0)));
        moon_orbit[0].rotate(Quaternion::from_axis_angle(Vector3::unit_z(), cgmath::Deg(-2.0)));
    }

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
    } 


    pub fn _add_light(&mut self, light: Light) {
        self._lights.push(light);
    } 

    // Creates a new instance of a previously loaded model and returns the instance_id,
    //  This can be used to add it to scene graph.
    pub fn add_instance(&mut self, model_id: usize) -> usize {
        self.models[model_id].add_instance();
        let instance_id = self.models[model_id].get_num_instances() - 1;
        self.sync_queue.push(SyncJob::Instance { model_id, instance_id });
        instance_id
    }

    pub fn _make_instance_child_of(&mut self, model_id: usize, instance_id: usize, id_chain: Vec<usize>) {
        let mut node = &mut self.graph;
        for i in id_chain {
            node = &mut node.children[i];
        }
        node.add_child(SceneNode::_new_instance_node(model_id, instance_id));
        dbg!(&node.children.len());
    }



    pub fn _remove_instance_of(&mut self, id: usize) {
        // No need to sync, because we can just call draw_indexed with a smaller range?
        self.models[id]._remove_instance();
    }

    // Stub of collecting sync jobs.
    fn collect_sync_jobs(&mut self) {
        // Construct root world matrix, so we can propagate it through the tree.
        let root_mat = Matrix4::from(self.graph.rotation) * Matrix4::from_translation(self.graph.position) * Matrix4::from_scale(self.graph.scale);
        let changed = self.graph.collect_changed(root_mat);
        
        // Collect instance sync jobs
        let mut instance_syncs: Vec<Vec<(usize, Matrix4<f32>)>> = vec![vec![]; self.models.len()];
        for (mid, iid, instance) in changed {
            // A change only needs to be synced if there are resources
            //  associated with it. Otherwise it's just a local graph change which
            //  has now propagated through the tree.
            if let (Some(model_id), Some(instance_id)) = (mid, iid) {
                instance_syncs[model_id].push((instance_id, instance));
            }
        }

        // TODO: Collecting consecutive insance ids to use sliced buffer writes would go
        //  here I guess. Just need to profile if worth.
        for (model_id, model) in instance_syncs.iter().enumerate() {
            for (instance_id, instance) in model {
                self.models[model_id].change_instance_raw(*instance_id, *instance);
            }
            // Instances of a model are on the same resource so we can just sync.
            //  once for now. TODO: fix syncing the same resource multiple times on
            //    seperate instance chances.
            self.sync_queue.push(SyncJob::Instance{model_id, instance_id: 0});
        }
    }

    // Stub of what the sync fn could look like.
    // TODO: shouldn't sync whole instance resource on a singular change. WIP
    //  - Also syncs the resource multile times on changes of seperate instances of the resource. >:(
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
    //  For now it just collects sync jobs and syncs.
    pub fn update(&mut self, _dt: Duration) {
        self.collect_sync_jobs();
        self.sync_scene_gpu();
    }

    pub fn _set_scale<T: Into<f32>>(&mut self, scale: T) {
        self.graph._set_scale(scale.into());
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
                wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: true,
                    }
                }
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
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