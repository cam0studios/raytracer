// wgpu compiling and encoding

use glam::Vec3;
use wesl::{StandardResolver, Wesl};

use crate::{
    buffers::BufferManager,
    scene::{Bvh, Lambertian, Primitive, Scene, Sphere},
    window::Context,
};

const CONSTS: Consts = Consts { bounces: 10 };

pub struct Pipeline {
    pub buffers: BufferManager,
    pub size: Size,
    pub pipelines: Pipelines,
    pub bind_group_layouts: BindGroupLayouts,
    pub bind_groups: BindGroups,
}
impl Pipeline {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        // todo: better location for scene
        let objects: Vec<Box<dyn Primitive>> = vec![
            Box::new(Sphere {
                center: Vec3::new(0.0, 1.1, 10.0),
                radius: 1.2,
                material: 0,
            }),
            Box::new(Sphere {
                center: Vec3::new(0.0, -0.2, 10.0),
                radius: 0.9,
                material: 0,
            }),
            Box::new(Sphere {
                center: Vec3::new(0.0, -1.3, 10.0),
                radius: 0.7,
                material: 0,
            }),
            Box::new(Sphere {
                center: Vec3::new(0.0, 101.5, 10.0),
                radius: 100.0,
                material: 1,
            }),
        ];
        let buffers = BufferManager::new(
            device,
            config,
            &Scene {
                bvh: Bvh::from_primitives(&objects),
                objects,
                materials: vec![
                    Box::new(Lambertian {
                        color: Vec3::new(1.0, 1.0, 1.0),
                    }),
                    Box::new(Lambertian {
                        color: Vec3::new(0.2, 0.6, 0.1),
                    }),
                ],
            },
        );

        let compiler = Wesl::new("src/shaders");

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

        let prep_indirect_shader_string = Self::get_shader_string(&compiler, "prep_indirect");
        let prep_indirect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Prep Indirect Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(prep_indirect_shader_string)),
        });
        let prep_indirect_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Prep Indirect Pipeline"),
                layout: None,
                module: &prep_indirect_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let extend_shader_string = Self::get_shader_string(&compiler, "extend");
        let extend_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Extend Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(extend_shader_string)),
        });
        let extend_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Extend Pipeline"),
            layout: None,
            module: &extend_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let shade_shader_string = Self::get_shader_string(&compiler, "shade");
        let shade_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shade Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(shade_shader_string)),
        });
        let shade_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Shade Pipeline"),
            layout: None,
            module: &shade_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // let compact_shader_string = Self::get_shader_string(&compiler, "compact");

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

        let pipelines = Pipelines {
            generate: generate_pipeline,
            prep_indirect: prep_indirect_pipeline,
            extend: extend_pipeline,
            shade: shade_pipeline,
            blit: blit_pipeline,
        };

        let bind_group_layouts = BindGroupLayouts {
            generate: pipelines.generate.get_bind_group_layout(0),
            prep_indirect: pipelines.prep_indirect.get_bind_group_layout(0),
            extend: pipelines.extend.get_bind_group_layout(0),
            shade: pipelines.shade.get_bind_group_layout(0),
            blit: pipelines.blit.get_bind_group_layout(0),
        };

        let bind_groups = Self::get_bind_groups(&bind_group_layouts, device, &buffers);

        Self {
            buffers,
            size: Size(config.width, config.height),
            pipelines,
            bind_group_layouts,
            bind_groups,
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

        // START

        self.buffers.vars.bounce = 0;
        self.buffers.write_vars(&context.device, &context.queue);

        // GENERATE PASS
        let mut generate_encoder =
            context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });
        {
            let mut generate_pass =
                generate_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Generate Pass"),
                    timestamp_writes: None,
                });
            generate_pass.set_pipeline(&self.pipelines.generate);
            generate_pass.set_bind_group(0, &self.bind_groups.generate, &[]);
            generate_pass.dispatch_workgroups(
                (self.size.f().0 / 8.0).ceil() as u32,
                (self.size.f().1 / 8.0).ceil() as u32,
                1,
            );
        }
        context
            .queue
            .submit(std::iter::once(generate_encoder.finish()));

        for bounce in 0..CONSTS.bounces {
            self.buffers.vars.bounce = bounce;
            self.buffers.write_vars(&context.device, &context.queue);
            let mut bounce_encoder =
                context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });

            // PREP INDIRECT PASS
            {
                let mut prep_indirect_pass =
                    bounce_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Prep Indirect Pass"),
                        timestamp_writes: None,
                    });
                prep_indirect_pass.set_pipeline(&self.pipelines.prep_indirect);
                prep_indirect_pass.set_bind_group(0, &self.bind_groups.prep_indirect, &[]);
                prep_indirect_pass.dispatch_workgroups(1, 1, 1);
            }
            // EXTEND PASS
            {
                let mut extend_pass =
                    bounce_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Extend Pass"),
                        timestamp_writes: None,
                    });
                extend_pass.set_pipeline(&self.pipelines.extend);
                extend_pass.set_bind_group(0, &self.bind_groups.extend, &[]);
                extend_pass.dispatch_workgroups_indirect(&self.buffers.active_ray_indirect, 0);
            }
            // SHADE PASS
            {
                let mut shade_pass =
                    bounce_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Shade Pass"),
                        timestamp_writes: None,
                    });
                shade_pass.set_pipeline(&self.pipelines.shade);
                shade_pass.set_bind_group(0, &self.bind_groups.shade, &[]);
                shade_pass.dispatch_workgroups_indirect(&self.buffers.active_ray_indirect, 0);
            }
            // COMPACT PASS
            // {}
            context
                .queue
                .submit(std::iter::once(bounce_encoder.finish()));
        }

        self.buffers.vars.frame += 1;
        self.buffers.write_vars(&context.device, &context.queue);

        // BLIT PASS
        let mut blit_encoder =
            context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });
        {
            let mut blit_pass = blit_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            blit_pass.set_pipeline(&self.pipelines.blit);
            blit_pass.set_bind_group(0, &self.bind_groups.blit, &[]);
            blit_pass.draw(0..3, 0..1);
        }
        context.queue.submit(std::iter::once(blit_encoder.finish()));

        context.queue.present(output);

        log::info!("frame {}", self.buffers.vars.frame);

        Ok(())
    }

    pub fn resize(&mut self, size: Size, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.size = size;
        self.buffers.resize(&device, self.size);
        self.buffers.vars.size = size;
        self.buffers.vars.frame = 0;
        self.buffers.write_vars(device, queue);
        self.bind_groups = Self::get_bind_groups(&self.bind_group_layouts, &device, &self.buffers);
    }

    pub fn get_bind_groups(
        layouts: &BindGroupLayouts,
        device: &wgpu::Device,
        buffers: &BufferManager,
    ) -> BindGroups {
        let generate = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Generate Bind Group"),
            layout: &layouts.generate,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.vars_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.rays_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.active_ray_counter.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.active_rays_buffer.as_entire_binding(),
                },
            ],
        });

        let prep_indirect = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Prep Indirect Bind Group"),
            layout: &layouts.prep_indirect,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.vars_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.active_ray_counter.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.active_ray_indirect.as_entire_binding(),
                },
            ],
        });

        let extend = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Extend Bind Group"),
            layout: &layouts.extend,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.vars_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.rays_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.active_rays_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.active_ray_indirect.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buffers.intersections_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: buffers.objects_buffer.as_entire_binding(),
                },
                // bvh
            ],
        });

        let shade = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shade Bind Group"),
            layout: &layouts.shade,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.vars_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.rays_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.active_rays_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.active_ray_counter.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buffers.active_ray_indirect.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: buffers.intersections_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: buffers.materials_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&buffers.output_view),
                },
            ],
        });

        // let compact

        let blit = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &layouts.blit,
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

        BindGroups {
            generate,
            prep_indirect,
            extend,
            shade,
            // compact,
            blit,
        }
    }
}

pub struct Consts {
    bounces: u32,
}

#[derive(Debug)]
pub struct Size(pub u32, pub u32);
impl Size {
    pub fn f(&self) -> (f32, f32) {
        (self.0 as f32, self.1 as f32)
    }
}
impl Clone for Size {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}
impl Copy for Size {}

pub struct Pipelines {
    generate: wgpu::ComputePipeline,
    prep_indirect: wgpu::ComputePipeline,
    extend: wgpu::ComputePipeline,
    shade: wgpu::ComputePipeline,
    // compact: wgpu::ComputePipeline
    blit: wgpu::RenderPipeline,
}
pub struct BindGroups {
    generate: wgpu::BindGroup,
    prep_indirect: wgpu::BindGroup,
    extend: wgpu::BindGroup,
    shade: wgpu::BindGroup,
    // compact: wgpu::BindGroup,
    blit: wgpu::BindGroup,
}
pub struct BindGroupLayouts {
    generate: wgpu::BindGroupLayout,
    prep_indirect: wgpu::BindGroupLayout,
    extend: wgpu::BindGroupLayout,
    shade: wgpu::BindGroupLayout,
    // compact: wgpu::BindGroupLayout,
    blit: wgpu::BindGroupLayout,
}

/*

CPU: write scene

FOR EVERY FRAME {

    CPU: write frame vars

    vars  ----------------------------------------------------------------------------------------------------------------------------- generate.wesl (width x height) -->  ray buffer [# active], active ray counter(next), active rays buffer [# active]

    FOR EVERY BOUNCE {
        CPU: write bounce vars

        vars, active ray counter(next), active ray counter  >-------------------------------------------------------------------------- prep_indirect.wesl (1) ---------->  active ray counter (indirect format), active ray counter (next reset)

        vars, ray buffer [# active], active rays buffer [# active], scene  >----------------------------------------------------------- extend.wesl (# active) ---------->  intersection buffer [# active]

        vars, ray buffer [# active], active rays buffer [# active], active ray counter, intersection buffer [# active], materials  >--- shade.wesl (# active) ----------->  ray buffer [# active], active rays buffer [# active], output buffer
        // split into substeps, separate accumulation, and/or separate materials

        vars, ray buffer [# active], active rays buffer [# active]  >------------------------------------------------------------------ compact.wesl (# active) --------->  ray buffer [# next active], active rays buffer [# next active]
        // split into count active, compute offsets, scatter into compacted?
        // not compact every bounce?
    }

    vars, output buffer  >------------------------------------------------------------------------------------------------------------- blit.wesl (render) -------------->  output image

}

*/
