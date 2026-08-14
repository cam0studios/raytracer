// window creation/management, wgpu setup

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use crate::pipeline::{Pipeline, Size};

pub struct Context {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
}

pub struct State {
    pub context: Context,
    pub window: Arc<Window>,
    pipeline: Pipeline,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            })
            .await?;

        let mut limits = wgpu::Limits::default();
        limits.max_storage_buffer_binding_size = 2u64.pow(29);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: limits,
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb()) // todo: decide if sRGB?
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };

        let pipeline = Pipeline::new(&device, &config);

        Ok(Self {
            window,
            context: Context {
                surface,
                device,
                queue,
                config,
                is_surface_configured: false,
            },
            pipeline,
        })
    }

    pub fn resize(&mut self, size: Size) {
        if size.0 > 0 && size.1 > 0 {
            self.context.config.width = size.0;
            self.context.config.height = size.1;
            self.context
                .surface
                .configure(&self.context.device, &self.context.config);

            if !self.context.is_surface_configured {
                log::info!("Window configured");
                self.window.set_visible(true);
                self.context.is_surface_configured = true;
            }

            self.pipeline
                .resize(size, &self.context.device, &self.context.queue);
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.context.is_surface_configured {
            return Ok(());
        }

        self.pipeline.render(&self.context)?;

        Ok(())
    }
}

pub struct WindowManager {
    state: Option<State>,
}

impl WindowManager {
    pub fn new() -> Self {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll); // switch to wait?

        let mut this = Self { state: None };
        event_loop.run_app(&mut this).unwrap();

        this
    }
}

impl ApplicationHandler for WindowManager {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Raytracer")
            .with_visible(true); // todo: hidden while launching?

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.state = Some(pollster::block_on(State::new(window)).unwrap());

        log::info!("Window created");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                log::info!("Window closed");
            }
            WindowEvent::Resized(size) => {
                state.resize(Size(size.width, size.height));
            }
            WindowEvent::RedrawRequested => match state.render() {
                Ok(_) => {}
                Err(e) => {
                    log::error!("{e}");
                    event_loop.exit();
                }
            },
            _ => (),
        }
    }
}
