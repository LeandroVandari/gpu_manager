use std::sync::Arc;

use anyhow::{Result, bail};
use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Queue,
    RequestAdapterOptions, Surface, SurfaceConfiguration, TextureFormat, TextureUsages,
};
#[cfg(feature = "window")]
use winit::window::{Window, WindowAttributes};

pub struct GpuManager<SurfaceManager = ()> {
    surface_manager: SurfaceManager,
    device: Device,
    queue: Queue,
}

impl<SurfaceManager> GpuManager<SurfaceManager> {
    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    fn create_instance() -> Instance {
        let instance_desc = InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        };
        Instance::new(&instance_desc)
    }
}

impl GpuManager<()> {
    pub async fn simple() -> Result<Self> {
        let instance = Self::create_instance();
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                required_features: Features::empty(),
                ..Default::default()
            })
            .await?;

        Ok(Self {
            surface_manager: (),
            device,
            queue,
        })
    }
}

#[cfg(feature = "window")]
impl<'window> GpuManager<WindowManager<'window>> {
    pub async fn with_window(event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Self> {
        let instance = Self::create_instance();

        let window = Arc::new(Self::create_window(event_loop)?);
        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                required_features: Features::empty(),
                ..Default::default()
            })
            .await?;

        let config = Self::create_surface_configuration(&surface, &adapter, &window)?;
        surface.configure(&device, &config);

        Ok(Self {
            surface_manager: WindowManager {
                window,
                surface,
                config,
            },
            device,
            queue,
        })
    }

    pub fn config(&self) -> &SurfaceConfiguration {
        &self.surface_manager.config
    }

    pub fn surface(&self) -> &Surface<'window> {
        &self.surface_manager.surface
    }

    pub fn window(&self) -> Arc<Window> {
        self.surface_manager.window.clone()
    }

    fn create_window(
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<Window, winit::error::OsError> {
        event_loop.create_window(
            WindowAttributes::default()
                .with_maximized(true)
                .with_resizable(false)
                .with_title("Ray tracer"),
        )
    }

    fn create_surface_configuration(
        surface: &Surface,
        adapter: &Adapter,
        window: &Window,
    ) -> Result<SurfaceConfiguration> {
        let surface_caps = surface.get_capabilities(adapter);
        dbg!(&surface_caps);
        let usage = if surface_caps.usages.contains(TextureUsages::COPY_DST) {
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST
        } else {
            log::warn!("Surface can't be copy destination. Using compatibility mode.");
            TextureUsages::RENDER_ATTACHMENT
        };

        fn get_surface_format(available_formats: &[TextureFormat]) -> Result<TextureFormat> {
            let priority_formats = [
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Bgra8Unorm,
            ];
            for format in priority_formats {
                if available_formats.contains(&format) {
                    return Ok(format);
                }
            }
            bail!("Couldn't get supported surface format, exiting.");
        }

        let surface_format = get_surface_format(&surface_caps.formats)?;

        let size = window.inner_size();
        Ok(SurfaceConfiguration {
            usage,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        })
    }
}
#[cfg(feature = "window")]
pub struct WindowManager<'window> {
    window: Arc<Window>,
    surface: Surface<'window>,
    config: SurfaceConfiguration,
}
