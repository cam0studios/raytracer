// wgpu compiling and encoding

use glam::Vec2;
use wesl::{StandardResolver, Wesl};

use crate::{buffers::BufferManager, window::Context};

const CONSTS: Consts = Consts { bounces: 1 };

pub struct Pipeline {
    pub generate_pipeline: wgpu::ComputePipeline,
    pub generate_bind_group: wgpu::BindGroup,
    // pub prep_indirect_pipeline: wgpu::ComputePipeline,
    // pub prep_indirect_bind_group: wgpu::BindGroup,
    // pub extend_pipeline: wgpu::ComputePipeline,
    // pub extend_bind_group: wgpu::BindGroup,
    // pub shade_pipeline: wgpu::ComputePipeline,
    // pub shade_bind_group: wgpu::BindGroup,
    // pub compact_pipeline: wgpu::ComputePipeline,
    // pub compact_bind_group: wgpu::BindGroup,
    pub blit_pipeline: wgpu::RenderPipeline,
    pub blit_bind_group: wgpu::BindGroup,
    pub buffers: BufferManager,
    pub size: Vec2,
}
impl Pipeline {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let buffers = BufferManager::new(device, config);

        let compiler = Wesl::new("src/shaders");

        // GENERATE SHADER
        let generate_shader_string = Self::get_shader_string(&compiler, "generate");
        let generate_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Generate Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(generate_shader_string)),
        });
        let generate_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Generate Pipeline"),
            layout: None,
            module: &generate_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let generate_bind_group_layout = generate_pipeline.get_bind_group_layout(0);
        let generate_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Generate Bind Group"),
            layout: &generate_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.vars_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&buffers.output_view),
                },
            ],
        });

        // let prep_indirect_shader_string = Self::get_shader_string(&compiler, "prep_indirect");
        // let extend_shader_string = Self::get_shader_string(&compiler, "extend");
        // let shade_shader_string = Self::get_shader_string(&compiler, "shade");
        // let compact_shader_string = Self::get_shader_string(&compiler, "compact");

        // BLIT SHADER
        let blit_shader_string = Self::get_shader_string(&compiler, "blit");
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(blit_shader_string)),
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: None,
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
        let blit_bind_group_layout = blit_pipeline.get_bind_group_layout(0);
        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.vars_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&buffers.output_view),
                },
            ],
        });

        Self {
            generate_pipeline,
            generate_bind_group,
            blit_pipeline,
            blit_bind_group,
            buffers,
            size: Vec2 {
                x: config.width as f32,
                y: config.height as f32,
            },
        }
    }

    fn get_shader_string(compiler: &Wesl<StandardResolver>, name: &str) -> String {
        compiler
            .compile(&("package::".to_string() + name).parse().unwrap())
            .inspect_err(|e| eprintln!("WESL error: {e}"))
            .unwrap()
            .to_string()
    }

    // RENDER LOOP
    pub fn render(&mut self, context: &Context) -> anyhow::Result<()> {
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

        // START

        // GENERATE PASS
        {
            let mut generate_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Generate Pass"),
                timestamp_writes: None,
            });
            generate_pass.set_pipeline(&self.generate_pipeline);
            generate_pass.set_bind_group(0, &self.generate_bind_group, &[]);
            generate_pass.dispatch_workgroups(
                (self.size.x / 8.0).ceil() as u32,
                (self.size.y / 8.0).ceil() as u32,
                1,
            );
        }
        for bounce in 0..CONSTS.bounces {
            self.buffers.vars.bounce = bounce;
            self.buffers.write_vars(&context.device, &context.queue);
            // PREP INDIRECT PASS
            // {}
            // EXTEND PASS
            // {}
            // SHADE PASS
            // {}
            // COMPACT PASS
            // {}
        }
        // BLIT PASS
        {
            let mut blit_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blit Pass"),
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
            blit_pass.set_pipeline(&self.blit_pipeline);
            blit_pass.set_bind_group(0, &self.blit_bind_group, &[]);
            blit_pass.draw(0..3, 0..1);
        }

        context.queue.submit(std::iter::once(encoder.finish()));
        context.queue.present(output);

        Ok(())
    }
}

pub struct Consts {
    bounces: u32,
}

/*

CPU: write scene

FOR EVERY FRAME {

    CPU: write frame vars

    vars  ---------------------------------------------------------------------------------------------------- generate.wesl (width x height) -->  ray buffer dense [# active], active ray counter(next)

    FOR EVERY BOUNCE {
        CPU: write bounce vars

        vars, active ray counter(next), active ray counter  >-------------------------------------------------- prep_indirect.wesl (1) ---------->  active ray counter (indirect format), active ray counter (next reset)

        vars, ray buffer dense [# active], scene  >------------------------------------------------------------ extend.wesl (# active) ---------->  intersection buffer [# active]

        vars, ray buffer dense [# active], active ray counter, intersection buffer [# active], materials  >---- shade.wesl (# active) ----------->  ray buffer sparse [# active], active rays buffer [# active], output buffer
        // split into substeps?

        vars, ray buffer sparse [# active], active rays buffer [# active]  >----------------------------------- compact.wesl (# active) --------->  ray buffer dense [# next active]
        // split into count active, compute offsets, scatter into compacted?
        // not compact every bounce?
    }

    vars, output buffer  >------------------------------------------------------------------------------------- blit.wesl ----------------------->  output image

}

*/
