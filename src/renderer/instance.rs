use cgmath::prelude::*;
use cgmath::{Matrix3, Matrix4, Quaternion, Vector3};

#[derive(Clone, Copy, Debug)]
pub struct Instance {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: f32,
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        let model = Matrix4::from_translation(self.position)
            * Matrix4::from(self.rotation)
            * Matrix4::from_scale(self.scale);

        let inverse_model = model.invert().unwrap();

        InstanceRaw {
            model: model.into(),
            inverse_model: inverse_model.into(),
        }
    }
}

impl From<InstanceRaw> for Instance {
    fn from(raw: InstanceRaw) -> Self {
        let m: Matrix4<f32> = raw.model.into();
        // Rotation scale 3x3 submatrix, transpose to row major.
        let rs = Matrix3::from_cols(m.x.truncate(), m.y.truncate(), m.z.truncate()).transpose();
        // Assume uniform xyz scaling
        let scale = rs.x.dot(rs.x).sqrt();
        // Pull rotation, but don't forget to transpose rs back to column major.
        let rotation: Quaternion<f32> = ((1.0 / scale) * rs.transpose()).into();
        let position = m.w.truncate();
        Instance {
            position,
            rotation,
            scale,
        }
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::from_axis_angle(Vector3::unit_z(), cgmath::Deg(0.0)),
            scale: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
    pub inverse_model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // model starts here
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // inverse_model starts here.
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 20]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 24]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 28]>() as wgpu::BufferAddress,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
