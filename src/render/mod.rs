use std::sync::Arc;
use winit::{dpi::PhysicalSize, window::Window};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Draw the game grid into `area` using egui's Painter.
/// Returns `(grid_origin, cell_size)` in logical pixels.
pub fn draw_grid_in(ui: &mut egui::Ui, grid: &game_of_life::sim::Grid, area: egui::Rect) -> (egui::Pos2, f32) {
    let cell_size = (area.width() / grid.width as f32)
        .min(area.height() / grid.height as f32)
        .floor()
        .max(4.0);

    let grid_w = cell_size * grid.width as f32;
    let grid_h = cell_size * grid.height as f32;

    let origin = area.min
        + egui::vec2(
            (area.width() - grid_w) / 2.0,
            (area.height() - grid_h) / 2.0,
        );

    let painter = ui.painter();

    for y in 0..grid.height {
        for x in 0..grid.width {
            let top_left = origin + egui::vec2(x as f32 * cell_size, y as f32 * cell_size);
            let rect = egui::Rect::from_min_size(
                top_left,
                egui::vec2(cell_size - 1.0, cell_size - 1.0),
            );
            let color = if grid.get(x as i32, y as i32) {
                egui::Color32::from_rgb(102, 204, 102)
            } else {
                egui::Color32::from_rgb(51, 51, 51)
            };
            painter.rect_filled(rect, 0.0, color);
        }
    }

    (origin, cell_size)
}

/// Convenience wrapper — draws into all available ui space.
#[allow(dead_code)]
pub fn draw_grid(ui: &mut egui::Ui, grid: &game_of_life::sim::Grid) -> (egui::Pos2, f32) {
    let area = egui::Rect::from_min_size(ui.cursor().min, ui.available_size());
    draw_grid_in(ui, grid, area)
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    egui_renderer: egui_wgpu::Renderer,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        #[cfg(not(target_arch = "wasm32"))]
        let backends = wgpu::Backends::all();
        #[cfg(target_arch = "wasm32")]
        let backends = wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        #[cfg(not(target_arch = "wasm32"))]
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("failed to create wgpu surface");

        #[cfg(target_arch = "wasm32")]
        let surface = {
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("game-canvas"))
                .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                .expect("canvas#game-canvas not found");
            instance
                .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
                .expect("failed to create wgpu surface")
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no suitable GPU adapter found");

        #[cfg(not(target_arch = "wasm32"))]
        let limits = wgpu::Limits::default();
        #[cfg(target_arch = "wasm32")]
        let limits = wgpu::Limits::downlevel_webgl2_defaults();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("failed to create wgpu device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // dithering=false: plain rendering, no ordered dithering
        let egui_renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);

        Self { surface, device, queue, config, egui_renderer }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(
        &mut self,
        tris: Vec<egui::ClippedPrimitive>,
        textures_delta: egui::TexturesDelta,
        pixels_per_point: f32,
    ) {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(e) => {
                log::warn!("dropped frame: {e}");
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame encoder"),
        });

        for (id, delta) in &textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point,
        };

        let extra_cmds =
            self.egui_renderer
                .update_buffers(&self.device, &self.queue, &mut encoder, &tris, &screen);

        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("main pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.06,
                                g: 0.06,
                                b: 0.06,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();

            self.egui_renderer.render(&mut pass, &tris, &screen);
        }

        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue
            .submit(extra_cmds.into_iter().chain(std::iter::once(encoder.finish())));
        frame.present();
    }
}
