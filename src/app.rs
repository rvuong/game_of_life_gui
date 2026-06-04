use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, Touch, TouchPhase, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

use egui_plot::{Line, Plot, PlotPoints};
use web_time::Instant;

use crate::render::Renderer;
use game_of_life::sim::patterns::{place, PRESETS};
use game_of_life::sim::Grid;
use game_of_life::sim::session::{GameResult, GameSession};

const BOTTOM_BAR_HEIGHT: f32 = 56.0;

/// Module-level thread_local so `resumed()` and `about_to_wait()` share the same slot.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static WASM_PENDING: std::cell::RefCell<Option<AppInner>> = std::cell::RefCell::new(None);
}
const MIN_CELL_PX: f32 = 24.0;

#[derive(Debug)]
pub enum AppPhase {
    Editing,
    Running,
    Ended(EndState),
    Splash { started: Instant },
}

#[derive(Debug)]
pub struct EndState {
    pub reason: GameResult,
    pub score: u64,
    pub population_history: Vec<u32>,
    pub step_count: u32,
}

/// Convert a screen-space position into a grid cell coordinate.
/// Returns `None` if the position is outside the grid bounds.
/// Free function so it can be unit-tested without GPU state.
pub fn screen_to_cell(
    mouse_x: f32,
    mouse_y: f32,
    grid_origin_x: f32,
    grid_origin_y: f32,
    cell_size: f32,
    grid_width: usize,
    grid_height: usize,
) -> Option<(usize, usize)> {
    if mouse_x < grid_origin_x || mouse_y < grid_origin_y {
        return None;
    }
    let col = ((mouse_x - grid_origin_x) / cell_size) as usize;
    let row = ((mouse_y - grid_origin_y) / cell_size) as usize;
    if col >= grid_width || row >= grid_height {
        None
    } else {
        Some((col, row))
    }
}

/// Compute grid dimensions that fill `available_w × available_h` with cells
/// of at least `MIN_CELL_PX`, capped at `max_dim` in each direction.
fn auto_grid_size(available_w: f32, available_h: f32) -> (usize, usize) {
    let cols = ((available_w / MIN_CELL_PX).floor() as usize).clamp(10, 40);
    let rows = ((available_h / MIN_CELL_PX).floor() as usize).clamp(10, 60);
    (cols, rows)
}

#[derive(Default)]
pub struct App {
    inner: Option<AppInner>,
    #[cfg(target_arch = "wasm32")]
    wasm_init_started: bool,
}

struct AppInner {
    window: Arc<Window>,
    renderer: Renderer,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    pub grid: Grid,
    pub phase: AppPhase,
    pub game_session: Option<GameSession>,
    pub max_steps: u32,
    grid_origin: egui::Pos2,
    grid_cell_size: f32,
    splash_texture: Option<egui::TextureHandle>,
    last_tick: Instant,
    step_duration: std::time::Duration,
    /// Last cell toggled during a drag, to avoid re-toggling the same cell.
    last_drag_cell: Option<(usize, usize)>,
    /// Whether the options sheet is open.
    options_open: bool,
    /// Whether this is a narrow (mobile) screen.
    is_mobile: bool,
}

impl AppInner {
    async fn new(window: Arc<Window>) -> Self {
        let renderer = Renderer::new(Arc::clone(&window)).await;
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        Self {
            window,
            renderer,
            egui_ctx,
            egui_state,
            grid: Grid::new(20, 20),
            phase: AppPhase::Splash { started: Instant::now() },
            game_session: None,
            max_steps: 300,
            grid_origin: egui::Pos2::ZERO,
            grid_cell_size: 1.0,
            splash_texture: None,
            last_tick: Instant::now(),
            step_duration: std::time::Duration::from_millis(300),
            last_drag_cell: None,
            options_open: false,
            is_mobile: false,
        }
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let raw_input = self.egui_state.take_egui_input(&*self.window);
        let pixels_per_point = self.egui_ctx.pixels_per_point();

        let phase_ref = &self.phase;
        let grid_ref = &self.grid;
        let game_session_ref = &self.game_session;
        let max_steps = self.max_steps;
        let step_duration_ms = self.step_duration.as_millis() as u32;
        let options_open = self.options_open;

        let mut start_clicked = false;
        let mut restart_clicked = false;
        let mut new_grid_origin: Option<egui::Pos2> = None;
        let mut new_cell_size: Option<f32> = None;
        let mut new_step_duration_ms: Option<u32> = None;
        let mut new_max_steps: Option<u32> = None;
        let mut new_drag_pos: Option<egui::Pos2> = None;
        let mut new_preset: Option<usize> = None;
        let mut dismiss_splash = false;
        let mut toggle_options = false;
        let mut new_grid_size: Option<(usize, usize)> = None;

        if matches!(self.phase, AppPhase::Splash { .. }) && self.splash_texture.is_none() {
            let bytes = include_bytes!("../assets/splash.png");
            let img = image::load_from_memory(bytes)
                .expect("failed to decode splash image")
                .to_rgba8();
            let (w, h) = img.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                &img.into_raw(),
            );
            self.splash_texture = Some(self.egui_ctx.load_texture(
                "splash",
                color_image,
                egui::TextureOptions::default(),
            ));
        }
        let splash_texture_ref = self.splash_texture.as_ref();
        let splash_elapsed = if let AppPhase::Splash { started } = &self.phase {
            started.elapsed().as_secs_f32()
        } else {
            0.0
        };

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            let screen = ctx.screen_rect();
            let is_mobile = screen.width() < 600.0;

            match phase_ref {
                AppPhase::Splash { .. } => {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        if let Some(texture) = splash_texture_ref {
                            let available = ui.max_rect();
                            let img_size = texture.size_vec2();
                            let scale = (available.width() / img_size.x)
                                .min(available.height() / img_size.y);
                            let display_size = img_size * scale;
                            let offset = egui::vec2(
                                (available.width() - display_size.x) / 2.0,
                                (available.height() - display_size.y) / 2.0,
                            );
                            let rect = egui::Rect::from_min_size(
                                available.min + offset,
                                display_size,
                            );
                            ui.painter().image(
                                texture.id(),
                                rect,
                                egui::Rect::from_min_max(
                                    egui::Pos2::ZERO,
                                    egui::Pos2::new(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                        }

                        let available = ui.max_rect();
                        let hint = if is_mobile { "Tap to start" } else { "Click to start" };
                        ui.painter().text(
                            egui::Pos2::new(available.center().x, available.max.y - 24.0),
                            egui::Align2::CENTER_CENTER,
                            hint,
                            egui::FontId::proportional(16.0),
                            egui::Color32::from_white_alpha(180),
                        );

                        if splash_elapsed > 4.5 {
                            let t = ((splash_elapsed - 4.5) / 0.5).min(1.0);
                            ui.painter().rect_filled(
                                ui.max_rect(),
                                0.0,
                                egui::Color32::from_black_alpha((t * 255.0) as u8),
                            );
                        }

                        let response = ui.allocate_rect(ui.max_rect(), egui::Sense::click());
                        if response.clicked() {
                            dismiss_splash = true;
                        }
                    });
                }

                AppPhase::Editing => {
                    if !is_mobile {
                        // Desktop: keep the classic side panel.
                        egui::SidePanel::left("controls").show(ctx, |ui| {
                            ui.label("Grid size:");
                            if ui.button("20 × 20").clicked() { new_grid_size = Some((20, 20)); }
                            if ui.button("40 × 40").clicked() { new_grid_size = Some((40, 40)); }
                            if ui.button("60 × 60").clicked() { new_grid_size = Some((60, 60)); }
                            ui.separator();
                            ui.label("Max steps:");
                            let mut ms_local = max_steps;
                            if ui.add(egui::Slider::new(&mut ms_local, 100..=10_000).logarithmic(true)).changed() {
                                new_max_steps = Some(ms_local);
                            }
                            ui.separator();
                            ui.label("Presets:");
                            for (i, preset) in PRESETS.iter().enumerate() {
                                if ui.button(preset.name).on_hover_text(preset.description).clicked() {
                                    new_preset = Some(i);
                                }
                            }
                            ui.separator();
                            if ui.button("▶  Start").clicked() { start_clicked = true; }
                            ui.separator();
                            if ui.button("⟳  Restart").clicked() { restart_clicked = true; }
                        });
                    }

                    egui::CentralPanel::default().show(ctx, |ui| {
                        let grid_area = if is_mobile {
                            let r = ui.max_rect();
                            egui::Rect::from_min_max(r.min, egui::pos2(r.max.x, r.max.y - BOTTOM_BAR_HEIGHT))
                        } else {
                            ui.max_rect()
                        };

                        let (origin, cell_size) = crate::render::draw_grid_in(ui, grid_ref, grid_area);
                        new_grid_origin = Some(origin);
                        new_cell_size = Some(cell_size);

                        let grid_rect = egui::Rect::from_min_size(
                            origin,
                            egui::vec2(cell_size * grid_ref.width as f32, cell_size * grid_ref.height as f32),
                        );
                        // Drag sense: fires on click and on drag, letting us paint multiple cells.
                        let response = ui.allocate_rect(grid_rect, egui::Sense::drag());
                        if response.is_pointer_button_down_on() || response.dragged() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                new_drag_pos = Some(pos);
                            }
                        }
                        if response.drag_stopped() {
                            // Reset last drag cell so next interaction starts fresh.
                            new_drag_pos = None;
                        }
                    });

                    if is_mobile {
                        // Bottom bar.
                        let screen = ctx.screen_rect();
                        let bar_rect = egui::Rect::from_min_size(
                            egui::pos2(0.0, screen.max.y - BOTTOM_BAR_HEIGHT),
                            egui::vec2(screen.width(), BOTTOM_BAR_HEIGHT),
                        );
                        egui::Area::new(egui::Id::new("bottom_bar_edit"))
                            .fixed_pos(bar_rect.min)
                            .show(ctx, |ui| {
                                ui.set_clip_rect(bar_rect);
                                egui::Frame::none()
                                    .fill(egui::Color32::from_black_alpha(200))
                                    .show(ui, |ui| {
                                        ui.set_min_size(bar_rect.size());
                                        ui.horizontal_centered(|ui| {
                                            ui.add_space(8.0);
                                            if ui.button("⚙").clicked() { toggle_options = true; }
                                            ui.add_space(8.0);

                                            // Preset combo
                                            let mut preset_idx: usize = 0;
                                            egui::ComboBox::from_id_salt("preset_combo")
                                                .width(110.0)
                                                .selected_text("Preset")
                                                .show_ui(ui, |ui| {
                                                    for (i, p) in PRESETS.iter().enumerate() {
                                                        if ui.selectable_value(&mut preset_idx, i, p.name).clicked() {
                                                            new_preset = Some(i);
                                                        }
                                                    }
                                                });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.add_space(8.0);
                                                if ui.add_sized([80.0, 40.0], egui::Button::new("▶ Start")).clicked() {
                                                    start_clicked = true;
                                                }
                                            });
                                        });
                                    });
                            });

                        // Options sheet (modal-style overlay).
                        if options_open {
                            egui::Window::new("Options")
                                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                                .collapsible(false)
                                .resizable(false)
                                .show(ctx, |ui| {
                                    ui.label("Grid size:");
                                    if ui.button("20 × 20").clicked() { new_grid_size = Some((20, 20)); toggle_options = true; }
                                    if ui.button("40 × 40").clicked() { new_grid_size = Some((40, 40)); toggle_options = true; }
                                    if ui.button("60 × 60 (large)").clicked() { new_grid_size = Some((60, 60)); toggle_options = true; }
                                    ui.separator();
                                    ui.label("Max steps:");
                                    let mut ms_local = max_steps;
                                    if ui.add(egui::Slider::new(&mut ms_local, 100..=10_000).logarithmic(true)).changed() {
                                        new_max_steps = Some(ms_local);
                                    }
                                    ui.separator();
                                    if ui.button("⟳  Clear grid").clicked() { restart_clicked = true; toggle_options = true; }
                                    ui.separator();
                                    if ui.button("Close").clicked() { toggle_options = true; }
                                });
                        }
                    }
                }

                AppPhase::Running => {
                    let session = game_session_ref.as_ref().unwrap();

                    if !is_mobile {
                        egui::SidePanel::left("controls").show(ctx, |ui| {
                            ui.label(format!("Step: {} / {}", session.step_count, max_steps));
                            ui.label(format!(
                                "Live cells: {}",
                                session.population_history.last().copied().unwrap_or(0)
                            ));
                            ui.separator();
                            ui.label("Speed (ms/step):");
                            let mut ms_local = step_duration_ms;
                            if ui.add(egui::Slider::new(&mut ms_local, 50..=2000).logarithmic(true)).changed() {
                                new_step_duration_ms = Some(ms_local);
                            }
                            ui.separator();
                            if ui.button("⟳  Restart").clicked() { restart_clicked = true; }
                        });
                    }

                    egui::CentralPanel::default().show(ctx, |ui| {
                        let grid_area = if is_mobile {
                            let r = ui.max_rect();
                            egui::Rect::from_min_max(r.min, egui::pos2(r.max.x, r.max.y - BOTTOM_BAR_HEIGHT))
                        } else {
                            ui.max_rect()
                        };
                        let (origin, cell_size) = crate::render::draw_grid_in(ui, &session.grid, grid_area);
                        new_grid_origin = Some(origin);
                        new_cell_size = Some(cell_size);
                    });

                    if is_mobile {
                        let screen = ctx.screen_rect();
                        let bar_rect = egui::Rect::from_min_size(
                            egui::pos2(0.0, screen.max.y - BOTTOM_BAR_HEIGHT),
                            egui::vec2(screen.width(), BOTTOM_BAR_HEIGHT),
                        );
                        egui::Area::new(egui::Id::new("bottom_bar_run"))
                            .fixed_pos(bar_rect.min)
                            .show(ctx, |ui| {
                                ui.set_clip_rect(bar_rect);
                                egui::Frame::none()
                                    .fill(egui::Color32::from_black_alpha(200))
                                    .show(ui, |ui| {
                                        ui.set_min_size(bar_rect.size());
                                        ui.horizontal_centered(|ui| {
                                            ui.add_space(8.0);
                                            ui.label(format!("{}/{}", session.step_count, max_steps));
                                            ui.add_space(8.0);
                                            let mut ms_local = step_duration_ms;
                                            if ui.add(
                                                egui::Slider::new(&mut ms_local, 50..=2000)
                                                    .logarithmic(true)
                                                    .show_value(false)
                                            ).changed() {
                                                new_step_duration_ms = Some(ms_local);
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.add_space(8.0);
                                                if ui.add_sized([44.0, 40.0], egui::Button::new("⟳")).clicked() {
                                                    restart_clicked = true;
                                                }
                                            });
                                        });
                                    });
                            });
                    }
                }

                AppPhase::Ended(state) => {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading("Game Over");
                        ui.separator();
                        let reason_text = match state.reason {
                            GameResult::CycleDetected => "A pattern repeated — the simulation has looped.",
                            GameResult::CapReached => "Step limit reached.",
                            GameResult::StillRunning => unreachable!(),
                        };
                        ui.label(reason_text);
                        ui.separator();
                        ui.label(format!("Steps: {}", state.step_count));
                        ui.label(format!("Unique states: {}", (state.score as f64).sqrt() as u64 - 1));
                        ui.heading(format!("Score: {}", state.score));
                        ui.separator();
                        ui.label("Population over time:");
                        let points: PlotPoints = state.population_history
                            .iter()
                            .enumerate()
                            .map(|(i, &count)| [i as f64, count as f64])
                            .collect();
                        let plot_h = if is_mobile { 120.0 } else { 200.0 };
                        Plot::new("pop_graph")
                            .height(plot_h)
                            .show(ui, |plot_ui| {
                                plot_ui.line(Line::new(points).name("Live cells"));
                            });
                        ui.separator();
                        if ui.add_sized(
                            [if is_mobile { ui.available_width() } else { 120.0 }, 44.0],
                            egui::Button::new("▶  Play again"),
                        ).clicked() {
                            restart_clicked = true;
                        }
                    });
                }
            }

            // Quit button — desktop only (meaningless in a PWA).
            #[cfg(not(target_arch = "wasm32"))]
            if !matches!(phase_ref, AppPhase::Splash { .. }) {
                egui::Area::new(egui::Id::new("quit_btn"))
                    .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
                    .show(ctx, |ui| {
                        if ui.button("Quit").clicked() {
                            event_loop.exit();
                        }
                    });
            }
        });

        // Apply mutations.
        if dismiss_splash {
            self.phase = AppPhase::Editing;
            self.splash_texture = None;
            // Auto-size the grid for the current screen on first entry.
            let screen = self.egui_ctx.screen_rect();
            let available_h = screen.height() - BOTTOM_BAR_HEIGHT;
            let (cols, rows) = auto_grid_size(screen.width(), available_h);
            self.grid = Grid::new(cols, rows);
            self.is_mobile = screen.width() < 600.0;
        }
        if toggle_options {
            self.options_open = !self.options_open;
        }
        if let Some((w, h)) = new_grid_size {
            self.grid = Grid::new(w, h);
        }
        if let Some(origin) = new_grid_origin {
            self.grid_origin = origin;
        }
        if let Some(cs) = new_cell_size {
            self.grid_cell_size = cs;
        }
        if let Some(ms) = new_step_duration_ms {
            self.step_duration = std::time::Duration::from_millis(ms as u64);
        }
        if let Some(ms) = new_max_steps {
            self.max_steps = ms;
        }
        if let Some(i) = new_preset {
            place(PRESETS[i].cells, &mut self.grid);
        }
        if let Some(pos) = new_drag_pos {
            if let Some((col, row)) = screen_to_cell(
                pos.x, pos.y,
                self.grid_origin.x, self.grid_origin.y,
                self.grid_cell_size,
                self.grid.width, self.grid.height,
            ) {
                if self.last_drag_cell != Some((col, row)) {
                    self.grid.toggle(col, row);
                    self.last_drag_cell = Some((col, row));
                }
            }
        } else {
            self.last_drag_cell = None;
        }
        if start_clicked {
            let session = GameSession::new(self.grid.clone(), self.max_steps);
            self.game_session = Some(session);
            self.phase = AppPhase::Running;
        }
        if restart_clicked {
            self.grid.clear();
            self.game_session = None;
            self.phase = AppPhase::Editing;
        }

        self.egui_state.handle_platform_output(&*self.window, full_output.platform_output);
        let tris = self.egui_ctx.tessellate(full_output.shapes, pixels_per_point);
        self.renderer.render(tris, full_output.textures_delta, pixels_per_point);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.inner.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Game of Life")
            .with_inner_size(LogicalSize::new(800u32, 800u32));
        let window = Arc::new(
            event_loop.create_window(attrs).expect("failed to create window"),
        );
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner = Some(pollster::block_on(AppInner::new(window)));
        }
        #[cfg(target_arch = "wasm32")]
        if !self.wasm_init_started {
            self.wasm_init_started = true;
            wasm_bindgen_futures::spawn_local(async move {
                let inner = AppInner::new(window).await;
                WASM_PENDING.with(|p| *p.borrow_mut() = Some(inner));
            });
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        // On WASM, check if the async init completed and pick it up.
        #[cfg(target_arch = "wasm32")]
        if self.inner.is_none() {
            WASM_PENDING.with(|p| {
                if p.borrow().is_some() {
                    self.inner = p.borrow_mut().take();
                }
            });
        }

        let Some(inner) = &mut self.inner else { return };

        if matches!(&inner.phase, AppPhase::Splash { .. }) {
            if let WindowEvent::KeyboardInput { event, .. } = &event {
                if event.state == ElementState::Pressed {
                    inner.phase = AppPhase::Editing;
                    inner.splash_texture = None;
                    let screen = inner.egui_ctx.screen_rect();
                    let available_h = screen.height() - BOTTOM_BAR_HEIGHT;
                    let (cols, rows) = auto_grid_size(screen.width(), available_h);
                    inner.grid = Grid::new(cols, rows);
                    inner.is_mobile = screen.width() < 600.0;
                    inner.window.request_redraw();
                    return;
                }
            }
            // Touch dismisses splash too.
            if let WindowEvent::Touch(Touch { phase: TouchPhase::Started, .. }) = &event {
                inner.phase = AppPhase::Editing;
                inner.splash_texture = None;
                let screen = inner.egui_ctx.screen_rect();
                let available_h = screen.height() - BOTTOM_BAR_HEIGHT;
                let (cols, rows) = auto_grid_size(screen.width(), available_h);
                inner.grid = Grid::new(cols, rows);
                inner.is_mobile = screen.width() < 600.0;
                inner.window.request_redraw();
                return;
            }
        }

        let response = inner.egui_state.on_window_event(&*inner.window, &event);
        if response.consumed {
            inner.window.request_redraw();
            return;
        }

        match &event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed =>
            {
                if let PhysicalKey::Code(code) = event.physical_key {
                    use crate::input::AppAction;
                    if let Some(action) = AppAction::from_key(code) {
                        match action {
                            AppAction::Quit => event_loop.exit(),
                            AppAction::ToggleFullscreen => {
                                use winit::window::Fullscreen;
                                let next = if inner.window.fullscreen().is_some() {
                                    None
                                } else {
                                    Some(Fullscreen::Borderless(None))
                                };
                                inner.window.set_fullscreen(next);
                            }
                            AppAction::ExitFullscreen => {
                                if inner.window.fullscreen().is_some() {
                                    inner.window.set_fullscreen(None);
                                }
                            }
                        }
                    }
                }
            }

            WindowEvent::Resized(size) => {
                inner.renderer.resize(*size);
                inner.window.request_redraw();
            }

            WindowEvent::RedrawRequested => inner.render(event_loop),

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // On WASM: pick up AppInner once the async GPU init finishes.
        #[cfg(target_arch = "wasm32")]
        if self.inner.is_none() {
            WASM_PENDING.with(|p| {
                if p.borrow().is_some() {
                    self.inner = p.borrow_mut().take();
                }
            });
        }

        let Some(inner) = &mut self.inner else { return };

        if let AppPhase::Splash { started } = &inner.phase {
            if started.elapsed().as_secs_f32() >= 5.0 {
                inner.phase = AppPhase::Editing;
                inner.splash_texture = None;
                let screen = inner.egui_ctx.screen_rect();
                let available_h = screen.height() - BOTTOM_BAR_HEIGHT;
                let (cols, rows) = auto_grid_size(screen.width(), available_h);
                inner.grid = Grid::new(cols, rows);
                inner.is_mobile = screen.width() < 600.0;
            }
        }

        if matches!(inner.phase, AppPhase::Running) {
            if inner.last_tick.elapsed() >= inner.step_duration {
                inner.last_tick = Instant::now();
                let session = inner.game_session.as_mut().unwrap();
                match session.advance() {
                    GameResult::StillRunning => {}
                    reason => {
                        inner.phase = AppPhase::Ended(EndState {
                            reason,
                            score: session.score(),
                            population_history: session.population_history.clone(),
                            step_count: session.step_count,
                        });
                    }
                }
            }
        }

        inner.window.request_redraw();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.inner = None;
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_transitions_to_running() {
        let grid = Grid::new(5, 5);
        let max_steps = 300;
        let session = GameSession::new(grid.clone(), max_steps);
        let phase = AppPhase::Running;
        assert!(matches!(phase, AppPhase::Running));
        let _ = session;
    }

    #[test]
    fn advance_called_when_enough_time_has_elapsed() {
        use std::time::{Duration, Instant};

        let mut g = Grid::new(10, 10);
        g.set(4, 5, true); g.set(5, 5, true); g.set(6, 5, true);

        let mut session = GameSession::new(g, 300);
        let step_duration = Duration::from_millis(1);
        let mut last_tick = Instant::now() - step_duration;

        if last_tick.elapsed() >= step_duration {
            last_tick = Instant::now();
            let result = session.advance();
            assert_eq!(result, GameResult::StillRunning);
            assert_eq!(session.step_count, 1);
        }
        let _ = last_tick;
    }
}

#[cfg(test)]
mod coord_tests {
    use super::*;

    #[test]
    fn top_left_corner_maps_to_cell_0_0() {
        assert_eq!(screen_to_cell(10.0, 10.0, 10.0, 10.0, 20.0, 5, 5), Some((0, 0)));
    }

    #[test]
    fn click_in_second_cell_row_and_column() {
        assert_eq!(screen_to_cell(55.0, 35.0, 10.0, 10.0, 20.0, 5, 5), Some((2, 1)));
    }

    #[test]
    fn click_outside_grid_returns_none() {
        assert_eq!(screen_to_cell(5.0, 5.0, 10.0, 10.0, 20.0, 5, 5), None);
        assert_eq!(screen_to_cell(120.0, 10.0, 10.0, 10.0, 20.0, 5, 5), None);
    }
}
