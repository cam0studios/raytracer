// buffer creation/management

use wgpu::util::DeviceExt;

use crate::{pipeline::Size, scene::Scene};

pub struct BufferManager {
    // static size
    pub vars: Vars, // Vars
    pub vars_buffer: wgpu::Buffer,
    pub active_ray_counter: wgpu::Buffer,  // atomic
    pub active_ray_indirect: wgpu::Buffer, // Indirect
    // scene dependent
    pub bvh_buffer: wgpu::Buffer,       // Bvh[# node]
    pub objects_buffer: wgpu::Buffer,   // Object[# object]
    pub materials_buffer: wgpu::Buffer, // Material[# material]
    // size dependent
    pub rays_buffer: wgpu::Buffer,          // Ray[# active]
    pub active_rays_buffer: wgpu::Buffer,   // u32[# active / 32] (packed bits)
    pub intersections_buffer: wgpu::Buffer, // Intersection[# active]
    pub output_buffer: wgpu::Texture,       // Texture[swidth x sheight]
    pub output_view: wgpu::TextureView,
}
impl BufferManager {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, scene: &Scene) -> Self {
        let size = Size(config.width, config.height);

        let vars = Vars {
            size,
            frame: 0,
            bounce: 0,
        };
        let vars_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vars Buffer"),
            contents: &vars.to_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let active_ray_counter = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Active Ray Counter"),
            mapped_at_creation: false,
            size: 4,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let active_ray_indirect = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Active Ray Counter"),
            mapped_at_creation: false,
            size: 16,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
        });

        let scene_dependent = Self::get_scene_dependent_buffers(device, scene);

        let size_dependent = Self::get_size_dependent_buffers(device, size);

        Self {
            vars,
            vars_buffer,
            active_ray_counter,
            active_ray_indirect,

            bvh_buffer: scene_dependent.bvh_buffer,
            objects_buffer: scene_dependent.objects_buffer,
            materials_buffer: scene_dependent.materials_buffer,

            rays_buffer: size_dependent.rays_buffer,
            active_rays_buffer: size_dependent.active_rays_buffer,
            intersections_buffer: size_dependent.intersections_buffer,
            output_buffer: size_dependent.output_buffer,
            output_view: size_dependent.output_view,
        }
    }

    fn get_scene_dependent_buffers(device: &wgpu::Device, scene: &Scene) -> SceneDependentBuffers {
        let mut bvh_raw: Vec<f32> = scene.bvh.to_raw();
        while bvh_raw.len() < 20 {
            bvh_raw.extend(vec![-1.0; 12]);
        }

        let bvh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BVH Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(bvh_raw.as_slice()),
        });

        let mut objects_raw: Vec<f32> = vec![];
        for object in &scene.objects {
            objects_raw.extend(object.to_raw());
        }
        while objects_raw.len() < 20 {
            objects_raw.extend(vec![-1.0; 16]);
        }

        let objects_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Objects Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(objects_raw.as_slice()),
        });

        let mut materials_raw: Vec<f32> = vec![];
        for material in &scene.materials {
            materials_raw.extend(material.to_raw());
        }
        while materials_raw.len() < 20 {
            materials_raw.extend(vec![-1.0; 16]);
        }

        let materials_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Materials Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&materials_raw.as_slice()),
        });

        SceneDependentBuffers {
            bvh_buffer,
            objects_buffer,
            materials_buffer,
        }
    }

    fn get_size_dependent_buffers(device: &wgpu::Device, size: Size) -> SizeDependentBuffers {
        let rays_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rays Buffer"),
            size: (4 * 12 * size.0 * size.1) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let active_rays_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Active Rays Buffer"),
            // size: (size.0 * size.1 + 31) as u64 / 4,
            size: (size.0 * size.1 * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let intersections_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Intersections Buffer"),
            size: (4 * 8 * size.0 * size.1) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let output_buffer = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Output Storage Texture"),
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::STORAGE_BINDING,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            mip_level_count: 1,
            sample_count: 1,
            view_formats: &[wgpu::TextureFormat::Rgba32Float],
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        });
        let output_view = output_buffer.create_view(&wgpu::TextureViewDescriptor::default());

        SizeDependentBuffers {
            rays_buffer,
            active_rays_buffer,
            intersections_buffer,
            output_buffer,
            output_view,
        }
    }

    pub fn write_vars(&self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        queue.write_buffer(&self.vars_buffer, 0, &self.vars.to_bytes());
    }

    pub fn write_scene(&mut self, device: &wgpu::Device, scene: &Scene) {
        let scene_dependent = BufferManager::get_scene_dependent_buffers(device, scene);
        self.bvh_buffer = scene_dependent.bvh_buffer;
        self.objects_buffer = scene_dependent.objects_buffer;
        self.materials_buffer = scene_dependent.materials_buffer;
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: Size) {
        let size_dependent = BufferManager::get_size_dependent_buffers(device, size);
        self.rays_buffer = size_dependent.rays_buffer;
        self.active_rays_buffer = size_dependent.active_rays_buffer;
        self.intersections_buffer = size_dependent.intersections_buffer;
        self.output_buffer = size_dependent.output_buffer;
        self.output_view = size_dependent.output_view;
    }
}

struct SceneDependentBuffers {
    bvh_buffer: wgpu::Buffer,
    objects_buffer: wgpu::Buffer,
    materials_buffer: wgpu::Buffer,
}

struct SizeDependentBuffers {
    rays_buffer: wgpu::Buffer,
    active_rays_buffer: wgpu::Buffer,
    intersections_buffer: wgpu::Buffer,
    output_buffer: wgpu::Texture,
    output_view: wgpu::TextureView,
}

pub struct Vars {
    pub size: Size,
    pub frame: u32,
    pub bounce: u32,
}
impl Vars {
    pub fn to_bytes(&self) -> [u8; 16] {
        let size_slice_binding = [self.size.0, self.size.1];
        let frame_slice_binding = [self.frame];
        let bounce_slice_binding = [self.bounce];
        let mut ret = [0 as u8; 16];
        ret[..8].copy_from_slice(bytemuck::cast_slice(&size_slice_binding));
        ret[8..12].copy_from_slice(bytemuck::cast_slice(&frame_slice_binding));
        ret[12..16].copy_from_slice(bytemuck::cast_slice(&bounce_slice_binding));
        ret
    }
}
