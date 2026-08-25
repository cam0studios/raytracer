// buffer creation/management

use glam::{Mat4, Vec3, vec3};
use wgpu::util::DeviceExt;

use crate::{
    pipeline::Size,
    scene::{Bvh, Material, Primitive, Scene},
};

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

        let vars = Vars::new(size);

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
        let mut bvh_raw: Vec<[u8; 32]> = scene.bvhs.iter().map(Bvh::raw).collect();
        while bvh_raw.len() < 2 {
            bvh_raw.push(Bvh::empty_raw());
        }

        let bvh_contents: &[u8] = bytemuck::cast_slice(&bvh_raw);
        let bvh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BVH Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bvh_contents,
        });

        let mut objects_raw: Vec<[u8; 64]> = scene.primitives.iter().map(Primitive::raw).collect();
        while objects_raw.len() < 2 {
            objects_raw.push(Primitive::empty_raw());
        }

        let objects_contents: &[u8] = bytemuck::cast_slice(&objects_raw);
        let objects_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Objects Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: objects_contents,
        });

        let mut materials_raw: Vec<[u8; 64]> = scene.materials.iter().map(Material::raw).collect();
        while materials_raw.len() < 2 {
            materials_raw.push(Material::empty_raw());
        }

        let materials_contents: &[u8] = bytemuck::cast_slice(&materials_raw);
        let materials_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Materials Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: materials_contents,
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
            size: (4 * 16 * size.0 * size.1) as u64,
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

    pub fn write_vars(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.vars_buffer, 0, &self.vars.to_bytes());
    }

    pub fn clear(&mut self, queue: &wgpu::Queue) {
        self.vars.frame = 0;
        self.vars.sample_idx += 1;
        self.write_vars(queue);

        let data: Vec<f32> = vec![0.0; (self.vars.size.0 * self.vars.size.1 * 4) as usize];

        queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &self.output_buffer,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(data.as_slice()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.vars.size.0 * 16),
                rows_per_image: Some(self.vars.size.1),
            },
            wgpu::Extent3d {
                width: self.vars.size.0,
                height: self.vars.size.1,
                depth_or_array_layers: 1,
            },
        );
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
    pub camera_pos: Vec3,
    pub frame: u32,
    pub camera_dir: Vec3,
    pub bounce: u32,
    pub size: Size,
    pub sample_idx: u32,
    matrix: Mat4,
}
impl Vars {
    pub fn new(size: Size) -> Self {
        Self {
            camera_pos: vec3(0.0, 0.0, 0.0),
            camera_dir: vec3(0.0, 0.0, 1.0),
            frame: 0,
            bounce: 0,
            sample_idx: 0,
            size,
            matrix: Mat4::default(),
        }
    }
    pub fn to_bytes(&self) -> [u8; 128] {
        let mut ret = [0u8; 128];
        ret[00..64].copy_from_slice(bytemuck::cast_slice(&self.matrix.to_cols_array()));
        ret[64..72].copy_from_slice(bytemuck::cast_slice(&[self.size.0, self.size.1]));
        ret[72..76].copy_from_slice(bytemuck::cast_slice(&[self.frame]));
        ret[76..80].copy_from_slice(bytemuck::cast_slice(&[self.bounce]));
        ret[80..84].copy_from_slice(bytemuck::cast_slice(&[self.sample_idx]));
        ret[84..128].copy_from_slice(bytemuck::cast_slice(&[0f32; 11]));
        ret
    }
    pub fn update_matrix(&mut self) {
        let camera_right = self.camera_dir.cross(vec3(0.0, 1.0, 0.0));
        let camera_up = camera_right.cross(self.camera_dir);
        self.matrix = Mat4::from_cols_array_2d(&[
            [
                camera_right.x,
                camera_up.x,
                self.camera_dir.x,
                self.camera_pos.x,
            ],
            [
                camera_right.y,
                camera_up.y,
                self.camera_dir.y,
                self.camera_pos.y,
            ],
            [
                camera_right.z,
                camera_up.z,
                self.camera_dir.z,
                self.camera_pos.z,
            ],
            [0.0, 0.0, 0.0, 1.0],
        ]);
    }
}
