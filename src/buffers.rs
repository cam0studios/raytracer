// buffer creation/management

use wgpu::util::DeviceExt;

pub struct BufferManager {
    pub vars: Vec<f32>, // Vars
    pub vars_buffer: wgpu::Buffer,
    // bvh: Vec<f32>, // Bvh[# node]
    // bvh_buffer: wgpu::Buffer,
    // scene: Vec<f32>, // Object[# object]
    // scene_buffer: wgpu::Buffer,
    // materials: Vec<f32>, // Material[# material]
    // materials_buffer: wgpu::Buffer,
    // rays_buffer: wgpu::Buffer, // Ray[# active]
    // active_rays_buffer: wgpu::Buffer, // bool[# active] OR bitmap
    // active_ray_counter: wgpu::Buffer, // Indirect
    // intersections_buffer: wgpu::Buffer, // Intersection[# active]
    pub output: wgpu::Texture, // Texture[swidth x sheight]
    pub output_view: wgpu::TextureView,
}
impl BufferManager {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let vars: Vec<f32> = vec![config.width as f32, config.height as f32];
        let vars_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vars Buffer"),
            contents: bytemuck::cast_slice(&vars),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Output Storage Texture"),
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::STORAGE_BINDING,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            mip_level_count: 1,
            sample_count: 1,
            view_formats: &config.view_formats,
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
        });
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            vars,
            vars_buffer,
            output,
            output_view,
        }
    }

    // pub fn resize(&mut self) {}
}
