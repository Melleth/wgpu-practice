use crate::model::*;
use crate::camera::*;

struct Scene {
    camera: Vec<Camera>,
    models: Vec<Model>,
    lights: Vec<Light>,

    active_camera: Option<&Camera>,
}

impl Scene {
    pub fn empty() -> Self {
        Self {
            camera: vec![],
            models: vec![],
            lights: vec![],
        }
    }

    pub fn add_model(&mut self, model: Model) {
        self.models.push(model);
    } 

    pub fn add_camera(&mut self, camera: camera) {
        self.cameras.push(camera);
    } 

    pub fn add_light(&mut self, light: camera) {
        self.lights.push(light);
    } 

    pub fn set_camera_active(&mut self, id: usize) {
        if id < self.cameras.len() {
            self.active_camera = Some(&self.camers[i]);
        } else {
            println!("Failed to swich to camera {} as it is not present in scene.", id);
        }
    }
}

pub trait DrawScene<'a, 'b> where 'b: 'a {
    fn draw_scene(&mut self, scene &'b Scene, uniforms: &'b BindGroup)
}


impl DrawScene<'a, 'b> for wgpu::RenderPass<'a> {
    fn draw_scene(&mut self, scene &'b Scene) {
        if let Some(camera) == self.active_camera {
            
        }
    }
}