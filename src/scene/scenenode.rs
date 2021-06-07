use cgmath::{
    Vector3,
    Matrix4,
    Quaternion,
    Rotation3,
};


// Consider all these fields LOCAL ONLY!!!
//  The "world matrix" aka Instance will be calculated when something
//    changes and needs to be synced to gpu
#[derive(Clone)]
pub struct SceneNode {
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

    pub fn _new_instance_node(model_id: usize, instance_id: usize) -> Self {
        SceneNode {
            position: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::from_axis_angle(Vector3::unit_x(), cgmath::Deg(0.0)),
            scale: 1.0,
            model_id: Some(model_id),
            instance_id: Some(instance_id),
            children: vec![],
            changed: true,
        }
    }

    // Sets the children of the removed node to this node.
    pub fn _remove_child(&mut self, id: usize) {
        let mut new_children = vec![];
        if let Some(child) = self.children.get_mut(id) {
            new_children.append(&mut child.children);
            self.children.remove(id);
        } else {
            println!("Attemping to remove child which isn't there.");
        }

        self.children.append(&mut new_children);
    }

    pub fn add_child(&mut self, child: SceneNode) {
        self.children.push(child);
    }

    pub fn _set_parent(self, other: &mut SceneNode) {
        other.add_child(self);
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
    pub fn collect_changed(&mut self, parent_mat: Matrix4<f32>) -> Vec<(Option<usize>, Option<usize>, Matrix4<f32>)> {
        let mut result = vec![];
        let mat = Matrix4::from(self.rotation) * Matrix4::from_translation(self.position) * Matrix4::from_scale(self.scale);
        let accumulated_mat = parent_mat * mat;

        if self.changed {
            result.push((self.model_id, self.instance_id, accumulated_mat));
            // Don't forget to unset this flag :))
            self.changed = false;
        }

        for c in &mut self.children {
            result.append(&mut c.collect_changed(accumulated_mat));
        }

        result
    }

    pub fn _translate<T: Into<f32>>(&mut self, x: T, y: T, z: T) {
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
        self.rotation = self.rotation * rotation;
        self.changed = true;
        self.update_children();
    }
}

impl Default for SceneNode {
    fn default() -> Self {
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
}