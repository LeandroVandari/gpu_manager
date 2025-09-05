use std::sync::Arc;

use anyhow::{Result, bail};
use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Queue,
    RequestAdapterOptions, Surface, SurfaceConfiguration, TextureFormat, TextureUsages,
};
#[cfg(feature = "window")]
use winit::window::{Window, WindowAttributes};

/// Manages Device creation and basic configuration.
///
/// This is the main struct provided by this crate. In order to obtain a [`GpuManager`] instance, use
/// [`GpuManager::simple`] or [`GpuManager::with_window`].
pub struct GpuManager<SurfaceManager = ()> {
    surface_manager: SurfaceManager,
    device: Device,
    queue: Queue,
}

impl<SurfaceManager> GpuManager<SurfaceManager> {
    /// Returns a reference to the contained [`wgpu::Device`].
    pub fn device(&self) -> &Device {
        &self.device
    }
    /// Returns a reference to the contained [`wgpu::Queue`].
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    fn create_instance() -> Instance {
        log::trace!("Creating wgpu Instance...");
        let instance_desc = InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        };
        Instance::new(&instance_desc)
    }
}

impl GpuManager<()> {
    /// Creates a [`GpuManager`] *without* window display capabilities.
    ///
    /// To be used without a window.
    ///
    /// Since creating an [`Adapter`] is async, this is also an async function.
    ///
    /// # Examples
    /// ```
    /// use gpu_manager::GpuManager;
    ///
    /// let manager = pollster::block_on(GpuManager::simple()).unwrap();
    /// ```
    ///
    /// # Errors
    /// This will error if [`Adapter`] or [`Device`] creation fail.
    pub async fn simple() -> Result<Self> {
        let instance = Self::create_instance();
        log::trace!("Creating wgpu Adapter...");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
            .await?;
        log::trace!("Creating wgpu Device...");
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
    /// Creates a [`GpuManager`] along with a [`Window`] that it will be able to display to.
    ///
    /// Since creating an [`Adapter`] is async, this is also async.
    ///
    /// Call this inside the [`ApplicationHandler::resumed`](winit::application::ApplicationHandler::resumed) function.
    ///
    /// # Errors
    /// This will error if 1) [`Adapter`] or [`Device`] creation fail, or 2) [`Surface`] configuration fails.
    pub async fn with_window(event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Self> {
        let instance = Self::create_instance();

        let window = Arc::new(Self::create_window(event_loop)?);
        log::trace!("Creating Surface...");
        let surface = instance.create_surface(window.clone())?;
        log::trace!("Creating wgpu Adapter...");
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;
        log::trace!("Creating wgpu Device...");
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                required_features: Features::empty(),
                ..Default::default()
            })
            .await?;

        let config = Self::create_surface_configuration(&surface, &adapter, &window)?;
        log::trace!("Configuring Surface...");
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

    /// Returns a reference to the contained [`SurfaceConfiguration`].
    pub fn config(&self) -> &SurfaceConfiguration {
        &self.surface_manager.config
    }

    /// Returns a reference to the contained [`Surface`].
    pub fn surface(&self) -> &Surface<'window> {
        &self.surface_manager.surface
    }

    /// Returns a counted reference to the contained [`Window`].
    pub fn window(&self) -> Arc<Window> {
        self.surface_manager.window.clone()
    }

    fn create_window(
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<Window, winit::error::OsError> {
        log::trace!("Creating window...");
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

        let surface_caps = surface.get_capabilities(adapter);
        log::trace!("Surface capabilities:\n{surface_caps:#?}");
        let usage = if surface_caps.usages.contains(TextureUsages::COPY_DST) {
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST
        } else {
            log::warn!("Surface can't be copy destination. Using compatibility mode.");
            TextureUsages::RENDER_ATTACHMENT
        };

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
/// Manages [`Window`] specific attributes, not needed when drawing to a file, for example.
pub struct WindowManager<'window> {
    window: Arc<Window>,
    surface: Surface<'window>,
    config: SurfaceConfiguration,
}
