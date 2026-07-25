// wgpu compiling and encoding
use wesl::{StandardResolver, Wesl};

use crate::window::Context;

pub struct Pipeline {
    // pub generate_pipeline: wgpu::ComputePipeline,
    // pub extend_pipeline: wgpu::ComputePipeline,
    // pub sort_pipeline: wgpu::ComputePipeline,
    // pub prep_indirect_material_pipeline: wgpu::ComputePipeline,
    // pub lambertian_pipeline: wgpu::ComputePipeline,
    // pub dielectric_pipeline: wgpu::ComputePipeline,
    // pub emissive_pipeline: wgpu::ComputePipeline,
    // pub miss_pipeline: wgpu::ComputePipeline,
    // pub prep_indirect_extend_pipeline: wgpu::ComputePipeline,
    pub blit_pipeline: wgpu::RenderPipeline,
}
impl Pipeline {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let compiler = Wesl::new("src/shaders");
        // let generate_shader_string = Self::get_shader_string(&compiler, "generate");
        // let extend_shader_string = Self::get_shader_string(&compiler, "extend");
        // let sort_shader_string = Self::get_shader_string(&compiler, "sort");
        // let prep_indirect_shader_string = Self::get_shader_string(&compiler, "prep_indirect");
        // let lambertian_shader_string = Self::get_shader_string(&compiler, "lambertian");
        // let dielectric_shader_string = Self::get_shader_string(&compiler, "dielectric");
        // let emissive_shader_string = Self::get_shader_string(&compiler, "emissive");
        // let miss_shader_string = Self::get_shader_string(&compiler, "miss");
        let blit_shader_string = Self::get_shader_string(&compiler, "blit");
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(blit_shader_string)),
        });
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            cache: None,
            multiview_mask: None,
        });
        Self { blit_pipeline }
    }

    fn get_shader_string(compiler: &Wesl<StandardResolver>, name: &str) -> String {
        compiler
            .compile(&("package::".to_string() + name).parse().unwrap())
            .inspect_err(|e| eprintln!("WESL error: {e}"))
            .unwrap()
            .to_string()
    }

    pub fn render(&self, context: &Context) -> anyhow::Result<()> {
        let output = match context.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                context.surface.configure(&context.device, &context.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("lost device!");
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.blit_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        // submit will accept anything that implements IntoIter
        context.queue.submit(std::iter::once(encoder.finish()));
        context.queue.present(output);

        Ok(())
    }
}

/*

CPU: write scene data

FOR EVERY FRAME:
    camera data  --- generate.wesl -->  ray buffer + active indices+counter
    CPU: write extend indirect buffer

    FOR EVERY BOUNCE:
        ray buffer, scene  >---------------------------------- extend.wesl --------->  intersection buffer
        CPU: clear material counters
        intersection buffer  >-------------------------------- sort.wesl ----------->  material indices+counters
        material indirect buffer  >--------------------------- prep_indirect.wesl -->  material indirect buffer
        lambertian indices+counter, ray buffer, materials  >-- lambertian.wesl ----->  new active indices buffer
        dielectric indices+counter, ray buffer, materials  >-- dielectric.wesl ----->  new active indices buffer
        emissive indices+counter, ray buffer, materials  >---- emissive.wesl ------->  output buffer
        miss indices+counter, ray buffer  >------------------- miss.wesl ----------->  output buffer
        extend indirect buffer  >----------------------------- prep_indirect.wesl -->  extend indirect buffer
        CPU: swap old and new active indices buffers and counters, clear used one

    output buffer, frame count  --- blit.wesl -->  output image

*/
