// buffer creation/management

pub struct BufferManager {
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
    output: wgpu::Texture,
}
impl BufferManager {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
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
        Self { output }
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
