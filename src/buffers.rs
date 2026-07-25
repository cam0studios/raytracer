// buffer creation/management

use wgpu::util::DeviceExt;

pub struct BufferManager {
    pub vars: Vec<f32>,
    pub vars_buffer: wgpu::Buffer,
    // bvh: Vec<f32>,
    // bvh_buffer: wgpu::Buffer,
    // scene: Vec<f32>,
    // scene_buffer: wgpu::Buffer,
    // materials: Vec<f32>,
    // materials_buffer: wgpu::Buffer,
    // rays_buffer: wgpu::Buffer,
    // active_rays_buffer: wgpu::Buffer,
    // active_ray_counter_a: wgpu::Buffer,
    // active_ray_counter_b: wgpu::Buffer,
    // intersections_buffer: wgpu::Buffer,
    // material_rays_buffer: wgpu::Buffer,
    // material_counter_buffer: wgpu::Buffer,
    pub output: wgpu::Texture,
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

/*

bvh: BVH[]
scene: Object[]
materials: Material[]
rays: Ray[]
active ray indices a+b: u32[]
active counters a+b: Indirect
intersections: Intersection[]
material indices (x4): u32[]
material counters: Indirect x4
output buffer: texture f32

*/
