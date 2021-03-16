use std::sync::Arc;
use std::ops::Range;
use bytemuck::{ Pod, Zeroable };
use wgpu::util::DeviceExt;

// Used to create a mapping between types and buffer usages.
//  May be very obsolete, don't really know.
#[derive(Clone, Copy)]
pub enum ResourceType {
    Vertex,
    Index,
    Uniform,
}

impl From<ResourceType> for wgpu::BufferUsage {
    fn from(item: ResourceType) -> wgpu::BufferUsage {
        match item {
            ResourceType::Vertex => wgpu::BufferUsage::VERTEX,
            ResourceType::Index => wgpu::BufferUsage::INDEX,
            ResourceType::Uniform => wgpu::BufferUsage::UNIFORM,
        }
    }
}

// Abstraction that wraps GPU buffers.
// Also holds Arc to the device and queue so we can conveniently
//  perform operations on the resources on the GPU.
pub struct Resource<T: Pod + Zeroable> {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    // This field represents the Resource data but in CPU ram.
    cpu_buffer: Vec<T>,
    // The gpu counterpart
    gpu_buffer: wgpu::Buffer,
    // Always represents n of T that can fit in the gpu_buffer.
    size: usize,
    resource_type: ResourceType,
}

impl<T: Pod + Zeroable> Resource<T> {
    // Constructs a new Resource with a size
    pub fn _new_sized(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        size: usize,
        resource_type: ResourceType,
    ) -> Self {
        let usage = wgpu::BufferUsage::from(resource_type) | wgpu::BufferUsage::COPY_DST;
        let cpu_buffer: Vec<T> = Vec::with_capacity(size);
        let gpu_buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label: Some("Nicely sized buffer"),
            size: (size * std::mem::size_of::<T>()) as wgpu::BufferAddress,
            // Wether the mem block is accesible by ArrayBuffer (according to spec...?)
            mapped_at_creation: false,
            usage: usage,
        });

        Self {
            device,
            queue,
            cpu_buffer,
            gpu_buffer,
            size,
            resource_type
        }
    }

    pub fn new_with_data(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        cpu_buffer: Vec<T>,
        resource_type: ResourceType,
    ) -> Self where T: Pod + Zeroable, {
        let usage = wgpu::BufferUsage::from(resource_type) | wgpu::BufferUsage::COPY_DST;
        let size = cpu_buffer.len();

        let gpu_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, // This will bite us in the ass while debugging somewhere along the line I guess.
            contents: bytemuck::cast_slice(&cpu_buffer),
            usage
        });

        Self {
            device,
            queue,
            cpu_buffer,
            gpu_buffer,
            size,
            resource_type,
        }
    }

    pub fn add_to_buffer(&mut self, items: Vec<T>) {
        self.cpu_buffer.extend(&items);
    }

    pub fn remove_from_buffer(&mut self, id: usize) {
        self.cpu_buffer.remove(id);
    }

    pub fn sync_gpu(&mut self) {
        if self.size < self.cpu_buffer.len() {
            // Recreate the gpu_buffer with twice the size to prevent overflow.
            self.size *= 2;
            self.gpu_buffer.destroy();
            let usage = wgpu::BufferUsage::from(self.resource_type) | wgpu::BufferUsage::COPY_DST;

            self.gpu_buffer = self.device.create_buffer(&wgpu::BufferDescriptor{
                label: Some("Nicely sized buffer"),
                size: (self.size * std::mem::size_of::<T>()) as wgpu::BufferAddress,
                mapped_at_creation: false,
                usage: usage,
            });
        }

        self.queue.write_buffer(&self.gpu_buffer, 0 as wgpu::BufferAddress, bytemuck::cast_slice(&self.cpu_buffer));

    }

    pub fn get_gpu_buffer(&self) -> &wgpu::Buffer {
        &self.gpu_buffer
    }

    pub fn _partial_sync_gpu(&mut self, range: Range<usize>, offset: usize) {
        self.queue.write_buffer(&self.gpu_buffer, offset as wgpu::BufferAddress, bytemuck::cast_slice(&self.cpu_buffer[range]));
    }

    pub fn _mut_local_at(&mut self, id: usize) -> Option<&mut T> {
        if id < self.cpu_buffer.len() { Some(&mut self.cpu_buffer[id]) } else { None }
    }

    pub fn local_at(&self, id:usize) -> Option<T> {
        if id < self.cpu_buffer.len() { Some(self.cpu_buffer[id]) } else { None }
    }

    pub fn get_cpu_length(&self) -> usize {
        self.cpu_buffer.len()
    }

    pub fn _get_gpu_length(&self) -> usize {
        self.size
    }
}