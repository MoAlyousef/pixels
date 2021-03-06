use crate::renderers::{ScalingMatrix, ScalingRenderer};
use crate::SurfaceSize;
use crate::{Error, Pixels, PixelsContext, SurfaceTexture};
use raw_window_handle::HasRawWindowHandle;
use std::env;

/// A builder to help create customized pixel buffers.
pub struct PixelsBuilder<'req, 'dev, 'win, W: HasRawWindowHandle> {
    request_adapter_options: Option<wgpu::RequestAdapterOptions<'req>>,
    device_descriptor: wgpu::DeviceDescriptor<'dev>,
    backend: wgpu::BackendBit,
    width: u32,
    height: u32,
    _pixel_aspect_ratio: f64,
    present_mode: wgpu::PresentMode,
    surface_texture: SurfaceTexture<'win, W>,
    texture_format: wgpu::TextureFormat,
    render_texture_format: wgpu::TextureFormat,
}

impl<'req, 'dev, 'win, W: HasRawWindowHandle> PixelsBuilder<'req, 'dev, 'win, W> {
    /// Create a builder that can be finalized into a [`Pixels`] pixel buffer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pixels::PixelsBuilder;
    /// # let window = pixels_mocks::Rwh;
    /// # let surface_texture = pixels::SurfaceTexture::new(1024, 768, &window);
    /// let mut pixels = PixelsBuilder::new(256, 240, surface_texture)
    ///     .request_adapter_options(wgpu::RequestAdapterOptions {
    ///         power_preference: wgpu::PowerPreference::HighPerformance,
    ///         compatible_surface: None,
    ///     })
    ///     .enable_vsync(false)
    ///     .build()?;
    /// # Ok::<(), pixels::Error>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when `width` or `height` are 0.
    pub fn new(
        width: u32,
        height: u32,
        surface_texture: SurfaceTexture<'win, W>,
    ) -> PixelsBuilder<'req, 'dev, 'win, W> {
        assert!(width > 0);
        assert!(height > 0);

        PixelsBuilder {
            request_adapter_options: None,
            device_descriptor: wgpu::DeviceDescriptor::default(),
            backend: wgpu::BackendBit::PRIMARY,
            width,
            height,
            _pixel_aspect_ratio: 1.0,
            present_mode: wgpu::PresentMode::Fifo,
            surface_texture,
            texture_format: wgpu::TextureFormat::Rgba8UnormSrgb,
            render_texture_format: wgpu::TextureFormat::Bgra8UnormSrgb,
        }
    }

    /// Add options for requesting a [`wgpu::Adapter`].
    pub fn request_adapter_options(
        mut self,
        request_adapter_options: wgpu::RequestAdapterOptions<'req>,
    ) -> PixelsBuilder<'req, 'dev, 'win, W> {
        self.request_adapter_options = Some(request_adapter_options);
        self
    }

    /// Add options for requesting a [`wgpu::Device`].
    pub fn device_descriptor(
        mut self,
        device_descriptor: wgpu::DeviceDescriptor<'dev>,
    ) -> PixelsBuilder<'req, 'dev, 'win, W> {
        self.device_descriptor = device_descriptor;
        self
    }

    /// Set which backends wgpu will attempt to use.
    ///
    /// The default value of this is [`wgpu::BackendBit::PRIMARY`], which enables
    /// the well supported backends for wgpu.
    pub fn wgpu_backend(mut self, backend: wgpu::BackendBit) -> PixelsBuilder<'req, 'dev, 'win, W> {
        self.backend = backend;
        self
    }

    /// Set the pixel aspect ratio to simulate non-square pixels.
    ///
    /// This setting enables a render pass that horizontally scales the pixel buffer by the given
    /// factor.
    ///
    /// E.g. set this to `8.0 / 7.0` for an 8:7 pixel aspect ratio.
    ///
    /// # Panics
    ///
    /// The aspect ratio must be > 0.
    ///
    /// # Warning
    ///
    /// This documentation is hidden because support for pixel aspect ratio is incomplete.
    #[doc(hidden)]
    pub fn pixel_aspect_ratio(
        mut self,
        pixel_aspect_ratio: f64,
    ) -> PixelsBuilder<'req, 'dev, 'win, W> {
        assert!(pixel_aspect_ratio > 0.0);

        self._pixel_aspect_ratio = pixel_aspect_ratio;
        self
    }

    /// Enable or disable Vsync.
    ///
    /// Vsync is enabled by default.
    ///
    /// The `wgpu` present mode will be set to `Fifo` when Vsync is enabled, or `Immediate` when
    /// Vsync is disabled. To set the present mode to `Mailbox` or another value, use the
    /// [`PixelsBuilder::present_mode`] method.
    pub fn enable_vsync(mut self, enable_vsync: bool) -> PixelsBuilder<'req, 'dev, 'win, W> {
        self.present_mode = if enable_vsync {
            wgpu::PresentMode::Fifo
        } else {
            wgpu::PresentMode::Immediate
        };
        self
    }

    /// Set the `wgpu` present mode.
    ///
    /// This differs from [`PixelsBuilder::enable_vsync`] by allowing the present mode to be set to
    /// any value.
    pub fn present_mode(
        mut self,
        present_mode: wgpu::PresentMode,
    ) -> PixelsBuilder<'req, 'dev, 'win, W> {
        self.present_mode = present_mode;
        self
    }

    /// Set the texture format.
    ///
    /// The default value is [`wgpu::TextureFormat::Rgba8UnormSrgb`], which is 4 unsigned bytes in
    /// `RGBA` order using the SRGB color space. This is typically what you want when you are
    /// working with color values from popular image editing tools or web apps.
    pub fn texture_format(
        mut self,
        texture_format: wgpu::TextureFormat,
    ) -> PixelsBuilder<'req, 'dev, 'win, W> {
        self.texture_format = texture_format;
        self
    }

    /// Set the render texture format.
    ///
    /// The default value is [`wgpu::TextureFormat::Bgra8UnormSrgb`], which is 4 unsigned bytes in
    /// `BGRA` order using the SRGB color space. This format depends on the hardware/platform the
    /// pixel buffer is rendered to/for.
    pub fn render_texture_format(
        mut self,
        texture_format: wgpu::TextureFormat,
    ) -> PixelsBuilder<'req, 'dev, 'win, W> {
        self.render_texture_format = texture_format;
        self
    }

    /// Create a pixel buffer from the options builder.
    ///
    /// # Errors
    ///
    /// Returns an error when a [`wgpu::Adapter`] cannot be found.
    pub fn build(self) -> Result<Pixels, Error> {
        let instance = wgpu::Instance::new(self.backend);

        // TODO: Use `options.pixel_aspect_ratio` to stretch the scaled texture
        let surface = unsafe { instance.create_surface(self.surface_texture.window) };
        let compatible_surface = Some(&surface);
        let adapter = instance.request_adapter(&self.request_adapter_options.map_or_else(
            || wgpu::RequestAdapterOptions {
                compatible_surface,
                power_preference: get_default_power_preference(),
            },
            |rao| wgpu::RequestAdapterOptions {
                compatible_surface: rao.compatible_surface.or(compatible_surface),
                power_preference: rao.power_preference,
            },
        ));
        let adapter = pollster::block_on(adapter).ok_or(Error::AdapterNotFound)?;

        let (mut device, queue) =
            pollster::block_on(adapter.request_device(&self.device_descriptor, None))
                .map_err(Error::DeviceNotFound)?;

        let present_mode = self.present_mode;

        // Create swap chain
        let surface_size = self.surface_texture.size;
        let swap_chain = create_swap_chain(
            &mut device,
            &surface,
            self.render_texture_format,
            &surface_size,
            present_mode,
        );

        // Create the backing texture
        let (scaling_matrix_inverse, texture_extent, texture, scaling_renderer, pixels_buffer_size) =
            create_backing_texture(
                &device,
                // Backing texture values
                self.width,
                self.height,
                self.texture_format,
                // Render texture values
                &surface_size,
                self.render_texture_format,
            );

        // Create the pixel buffer
        let mut pixels = Vec::with_capacity(pixels_buffer_size);
        pixels.resize_with(pixels_buffer_size, Default::default);

        // Instantiate the Pixels struct
        let context = PixelsContext {
            device,
            queue,
            surface,
            swap_chain,
            texture,
            texture_extent,
            texture_format: self.texture_format,
            texture_format_size: get_texture_format_size(self.texture_format),
            scaling_renderer,
        };

        Ok(Pixels {
            context,
            surface_size,
            present_mode,
            pixels,
            scaling_matrix_inverse,
            render_texture_format: self.render_texture_format,
        })
    }
}

pub(crate) fn create_swap_chain(
    device: &mut wgpu::Device,
    surface: &wgpu::Surface,
    format: wgpu::TextureFormat,
    surface_size: &SurfaceSize,
    present_mode: wgpu::PresentMode,
) -> wgpu::SwapChain {
    device.create_swap_chain(
        &surface,
        &wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format,
            width: surface_size.width,
            height: surface_size.height,
            present_mode,
        },
    )
}

pub(crate) fn create_backing_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    backing_texture_format: wgpu::TextureFormat,
    surface_size: &SurfaceSize,
    render_texture_format: wgpu::TextureFormat,
) -> (
    ultraviolet::Mat4,
    wgpu::Extent3d,
    wgpu::Texture,
    ScalingRenderer,
    usize,
) {
    let scaling_matrix_inverse = ScalingMatrix::new(
        (width as f32, height as f32),
        (surface_size.width as f32, surface_size.height as f32),
    )
    .transform
    .inversed();

    let texture_extent = wgpu::Extent3d {
        width,
        height,
        depth: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pixels_source_texture"),
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: backing_texture_format,
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let scaling_renderer = ScalingRenderer::new(
        device,
        &texture_view,
        &texture_extent,
        surface_size,
        render_texture_format,
    );

    let texture_format_size = get_texture_format_size(backing_texture_format);
    let pixels_buffer_size = ((width * height) as f32 * texture_format_size) as usize;

    (
        scaling_matrix_inverse,
        texture_extent,
        texture,
        scaling_renderer,
        pixels_buffer_size,
    )
}

#[rustfmt::skip]
#[inline]
const fn get_texture_format_size(texture_format: wgpu::TextureFormat) -> f32 {
    use wgpu::TextureFormat::*;

    // TODO: Use constant arithmetic when supported.
    // See: https://github.com/rust-lang/rust/issues/57241
    match texture_format {
        // 8-bit formats, 8 bits per component
        R8Unorm
        | R8Snorm
        | R8Uint
        | R8Sint => 1.0, // 8.0 / 8.0

        // 16-bit formats, 8 bits per component
        R16Uint
        | R16Sint
        | R16Float
        | Rg8Unorm
        | Rg8Snorm
        | Rg8Uint
        | Rg8Sint => 2.0, // 16.0 / 8.0

        // 32-bit formats, 8 bits per component
        R32Uint
        | R32Sint
        | R32Float
        | Rg16Uint
        | Rg16Sint
        | Rg16Float
        | Rgba8Unorm
        | Rgba8UnormSrgb
        | Rgba8Snorm
        | Rgba8Uint
        | Rgba8Sint
        | Bgra8Unorm
        | Bgra8UnormSrgb
        | Rgb10a2Unorm
        | Rg11b10Float
        | Depth32Float
        | Depth24Plus
        | Depth24PlusStencil8 => 4.0, // 32.0 / 8.0

        // 64-bit formats, 8 bits per component
        Rg32Uint
        | Rg32Sint
        | Rg32Float
        | Rgba16Uint
        | Rgba16Sint
        | Rgba16Float => 8.0, // 64.0 / 8.0

        // 128-bit formats, 8 bits per component
        Rgba32Uint
        | Rgba32Sint
        | Rgba32Float => 16.0, // 128.0 / 8.0

        // Compressed formats

        // 4x4 blocks, 8 bytes per block
        Bc1RgbaUnorm
        | Bc1RgbaUnormSrgb
        | Bc4RUnorm
        | Bc4RSnorm
        | Etc2RgbUnorm
        | Etc2RgbUnormSrgb
        | Etc2RgbA1Unorm
        | Etc2RgbA1UnormSrgb
        | EacRUnorm
        | EacRSnorm => 0.5, // 4.0 * 4.0 / 8.0

        // 4x4 blocks, 16 bytes per block
        Bc2RgbaUnorm
        | Bc2RgbaUnormSrgb
        | Bc3RgbaUnorm
        | Bc3RgbaUnormSrgb
        | Bc5RgUnorm
        | Bc5RgSnorm
        | Bc6hRgbUfloat
        | Bc6hRgbSfloat
        | Bc7RgbaUnorm
        | Bc7RgbaUnormSrgb
        | Etc2RgbA8Unorm
        | Etc2RgbA8UnormSrgb
        | EtcRgUnorm
        | EtcRgSnorm
        | Astc4x4RgbaUnorm
        | Astc4x4RgbaUnormSrgb => 1.0, // 4.0 * 4.0 / 16.0

        // 5x4 blocks, 16 bytes per block
        Astc5x4RgbaUnorm
        | Astc5x4RgbaUnormSrgb => 1.25, // 5.0 * 4.0 / 16.0

        // 5x5 blocks, 16 bytes per block
        Astc5x5RgbaUnorm
        | Astc5x5RgbaUnormSrgb => 1.5625, // 5.0 * 5.0 / 16.0

        // 6x5 blocks, 16 bytes per block
        Astc6x5RgbaUnorm
        | Astc6x5RgbaUnormSrgb => 1.875, // 6.0 * 5.0 / 16.0

        // 6x6 blocks, 16 bytes per block
        Astc6x6RgbaUnorm
        | Astc6x6RgbaUnormSrgb => 2.25, // 6.0 * 6.0 / 16.0

        // 8x5 blocks, 16 bytes per block
        Astc8x5RgbaUnorm
        | Astc8x5RgbaUnormSrgb => 2.5, // 8.0 * 5.0 / 16.0

        // 8x6 blocks, 16 bytes per block
        Astc8x6RgbaUnorm
        | Astc8x6RgbaUnormSrgb => 3.0, // 8.0 * 6.0 / 16.0

        // 8x8 blocks, 16 bytes per block
        Astc8x8RgbaUnorm
        | Astc8x8RgbaUnormSrgb => 4.0, // 8.0 * 8.0 / 16.0

        // 10x5 blocks, 16 bytes per block
        Astc10x5RgbaUnorm
        | Astc10x5RgbaUnormSrgb => 3.125, // 10.0 * 5.0 / 16.0

        // 10x6 blocks, 16 bytes per block
        Astc10x6RgbaUnorm
        | Astc10x6RgbaUnormSrgb => 3.75, // 10.0 * 6.0 / 16.0

        // 10x8 blocks, 16 bytes per block
        Astc10x8RgbaUnorm
        | Astc10x8RgbaUnormSrgb => 5.0, // 10.0 * 8.0 / 16.0

        // 10x10 blocks, 16 bytes per block
        Astc10x10RgbaUnorm
        | Astc10x10RgbaUnormSrgb => 6.25, // 10.0 * 10.0 / 16.0

        // 12x10 blocks, 16 bytes per block
        Astc12x10RgbaUnorm
        | Astc12x10RgbaUnormSrgb => 7.5, // 12.0 * 10.0 / 16.0

        // 12x12 blocks, 16 bytes per block
        Astc12x12RgbaUnorm
        | Astc12x12RgbaUnormSrgb => 9.0, // 12.0 * 12.0 / 16.0
    }
}

fn get_default_power_preference() -> wgpu::PowerPreference {
    env::var("PIXELS_HIGH_PERF").map_or_else(
        |_| {
            env::var("PIXELS_LOW_POWER").map_or(wgpu::PowerPreference::default(), |_| {
                wgpu::PowerPreference::LowPower
            })
        },
        |_| wgpu::PowerPreference::HighPerformance,
    )
}
