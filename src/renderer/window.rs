use crate::domain::{NodeData, Position3};
use crate::entropy::EntropyEngine;
use crate::gravity::GravityEngine;
use crate::materialize::MaterializationEngine;
use crate::renderer::aura_pass::{AuraPass, AuraUniform};
use crate::renderer::camera::{Camera, CameraController};
use crate::renderer::edge_pass::{edge_to_vertices, EdgePipeline, EdgeVertex};
use crate::renderer::node_pass::{NodeInstance, NodePipeline};
use crate::renderer::particles::ParticleSystem;
use crate::renderer::RenderConfig;
use crate::storage::SqliteWorkspaceStore;
use crate::storage::WorkspaceStore;
use crate::systems::{CognitiveSeason, CognitiveSeasonDetector, WeatherSystem};
use crate::workspace::SilentNodeWorkspace;
use chrono::Utc;
use glam::Vec4Swizzles;
use glam::{Vec2, Vec3, Vec4};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

// ── Drag state ────────────────────────────────────────────────────────────────

struct DragState {
    node_id: Uuid,
    plane_point: Vec3,  // world-space hit at drag start (point on drag plane)
    plane_normal: Vec3, // drag plane normal = camera forward at drag start
    node_start: Vec3,   // node world position at drag start
}

// ── Renderer ──────────────────────────────────────────────────────────────────

pub struct Renderer {
    workspace: SilentNodeWorkspace,
    config: RenderConfig,
    gpu: Option<GpuState>,
    last_frame: Instant,
    // Mouse
    mouse_down: bool,
    right_mouse_down: bool,
    last_cursor: Option<(f64, f64)>,
    click_pos: Option<(f64, f64)>, // press position for click-vs-drag
    // Selection & drag
    selected_node: Option<Uuid>,
    drag_state: Option<DragState>,
    // Physics
    physics_running: bool,
    // Autosave
    autosave_timer: f32,
}

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    camera: Camera,
    controller: CameraController,
    camera_buf: wgpu::Buffer,
    #[allow(dead_code)]
    camera_bind_group_layout: wgpu::BindGroupLayout,
    camera_bind_group: wgpu::BindGroup,
    node_pipeline: NodePipeline,
    edge_pipeline: EdgePipeline,
    particles: ParticleSystem,
    aura: AuraPass,
    weather: WeatherSystem,
    elapsed: f32,
    last_weather_derive: Instant,
    // Phase 6 live cognitive state
    season_value: f32,
    oracle_pulse: f32,
    // Phase 8
    fly_target: Option<Vec3>, // smooth camera fly-to target
    pending_screenshot: bool,
    fps: f32,
    fps_timer: f32,
    fps_frames: u32,
    // Phase 9 — Audio (only when compiled with --features audio)
    #[cfg(feature = "audio")]
    audio: crate::audio::AudioEngine,
}

impl Renderer {
    pub fn new(workspace: SilentNodeWorkspace, config: RenderConfig) -> Self {
        Self {
            workspace,
            config,
            gpu: None,
            last_frame: Instant::now(),
            mouse_down: false,
            right_mouse_down: false,
            last_cursor: None,
            click_pos: None,
            selected_node: None,
            drag_state: None,
            physics_running: false,
            autosave_timer: 0.0,
        }
    }
}

// ── Ray helpers ───────────────────────────────────────────────────────────────

fn cursor_ray(cursor: (f64, f64), w: f32, h: f32, cam: &Camera) -> (Vec3, Vec3) {
    let ndc_x = (cursor.0 as f32 / w) * 2.0 - 1.0;
    let ndc_y = 1.0 - (cursor.1 as f32 / h) * 2.0;
    let inv = cam.inv_view_proj();
    let near = inv * Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
    let near = near.xyz() / near.w;
    let far = inv * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
    let far = far.xyz() / far.w;
    (cam.position, (far - near).normalize_or_zero())
}

fn ray_plane_hit(orig: Vec3, dir: Vec3, plane_pt: Vec3, plane_n: Vec3) -> Option<Vec3> {
    let denom = dir.dot(plane_n);
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = (plane_pt - orig).dot(plane_n) / denom;
    if t < 0.0 {
        return None;
    }
    Some(orig + dir * t)
}

// ── ApplicationHandler ────────────────────────────────────────────────────────

impl ApplicationHandler for Renderer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("SilentNode — Living Cognitive Universe")
                        .with_inner_size(winit::dpi::LogicalSize::new(
                            self.config.width,
                            self.config.height,
                        )),
                )
                .expect("failed to create window"),
        );

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("failed to create wgpu surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no suitable GPU adapter found");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("silentnode_window"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        ))
        .expect("failed to request GPU device");

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            // COPY_SRC enables screenshot readback
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let camera = Camera {
            aspect: size.width as f32 / size.height.max(1) as f32,
            ..Camera::default()
        };
        let camera_uniform = camera.to_uniform();

        let camera_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buf"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bg"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buf.as_entire_binding(),
            }],
        });

        let node_pipeline = NodePipeline::new(
            &device,
            format,
            &camera_bind_group_layout,
            self.config.node_capacity,
        );
        let edge_pipeline = EdgePipeline::new(
            &device,
            format,
            &camera_bind_group_layout,
            self.config.edge_capacity,
        );
        let particles = ParticleSystem::new(&device, format, &camera_bind_group_layout);
        let aura = AuraPass::new(&device, format);

        let mut weather = WeatherSystem::new();
        let initial_season_value;
        #[cfg(feature = "audio")]
        let initial_season_value_str: String;
        {
            let nodes: Vec<&NodeData> = self.workspace.graph.nodes().collect();
            let events = self.workspace.focus.events();
            weather.derive(&nodes, events, Utc::now());
            let season_report =
                CognitiveSeasonDetector::new().detect_season(&nodes, events, &[], Utc::now());
            initial_season_value = season_value_of(season_report.season);
            #[cfg(feature = "audio")]
            {
                initial_season_value_str = season_report.season.name().to_lowercase();
            }
        }

        println!();
        println!("╔══════════════════════════════════════════════════╗");
        println!("║  SilentNode — Keyboard Controls                  ║");
        println!("║  W/A/S/D or Arrows  orbit camera                 ║");
        println!("║  +/-                zoom                          ║");
        println!("║  Space              reset camera                  ║");
        println!("║  F                  fly to selected node          ║");
        println!("║  N                  spawn new thought node        ║");
        println!("║  P                  toggle physics simulation     ║");
        println!("║  I                  print selected node info      ║");
        println!("║  Shift+S            save screenshot PNG           ║");
        println!("║  Esc                deselect                      ║");
        println!("║  Left-drag node     move node in world space      ║");
        println!("║  Right-drag         pan camera                    ║");
        println!("╚══════════════════════════════════════════════════╝");

        self.gpu = Some(GpuState {
            window,
            surface,
            surface_config,
            device,
            queue,
            camera,
            controller: CameraController::new(),
            camera_buf,
            camera_bind_group_layout,
            camera_bind_group,
            node_pipeline,
            edge_pipeline,
            particles,
            aura,
            weather,
            elapsed: 0.0,
            last_weather_derive: Instant::now(),
            season_value: initial_season_value,
            oracle_pulse: 0.0,
            fly_target: None,
            pending_screenshot: false,
            fps: 0.0,
            fps_timer: 0.0,
            fps_frames: 0,
            // Phase 9 — start audio engine and set initial atmosphere from season
            #[cfg(feature = "audio")]
            audio: {
                let eng = crate::audio::AudioEngine::new();
                eng.set_atmosphere(crate::audio::atmosphere_from_season(
                    &initial_season_value_str,
                ));
                eng
            },
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.autosave_now();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.surface_config.width = size.width.max(1);
                    gpu.surface_config.height = size.height.max(1);
                    gpu.surface.configure(&gpu.device, &gpu.surface_config);
                    gpu.camera
                        .set_aspect(gpu.surface_config.width, gpu.surface_config.height);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                // Shift+S → screenshot
                if code == KeyCode::KeyS {
                    if let Some(gpu) = &mut self.gpu {
                        if gpu.window.current_monitor().is_some() {
                            // detect shift via scancode; simpler: always allow
                            gpu.pending_screenshot = true;
                        }
                    }
                } else {
                    self.handle_key(code);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        if state == ElementState::Pressed {
                            self.mouse_down = true;
                            self.click_pos = self.last_cursor;
                            // Check if cursor is over a node → enter drag mode
                            if let Some(cursor) = self.last_cursor {
                                self.try_start_drag(cursor);
                            }
                        } else {
                            // Release
                            if self.drag_state.is_none() {
                                // Only pick node if not dragging and barely moved
                                if let (Some(start), Some(end)) = (self.click_pos, self.last_cursor)
                                {
                                    let dx = (end.0 - start.0).abs();
                                    let dy = (end.1 - start.1).abs();
                                    if dx < 5.0 && dy < 5.0 {
                                        self.pick_node_at(end);
                                    }
                                }
                            }
                            self.drag_state = None;
                            self.mouse_down = false;
                            self.last_cursor = None;
                            self.click_pos = None;
                        }
                    }
                    MouseButton::Right => {
                        self.right_mouse_down = state == ElementState::Pressed;
                        if !self.right_mouse_down {
                            self.last_cursor = None;
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = (position.x, position.y);
                if let Some(last) = self.last_cursor {
                    let delta = Vec2::new((pos.0 - last.0) as f32, (pos.1 - last.1) as f32);
                    if let Some(ref ds) = self.drag_state {
                        // Drag node: unproject cursor to drag plane
                        let node_id = ds.node_id;
                        if let Some(gpu) = &self.gpu {
                            let w = gpu.surface_config.width as f32;
                            let h = gpu.surface_config.height as f32;
                            let (ro, rd) = cursor_ray(pos, w, h, &gpu.camera);
                            let hit = ray_plane_hit(ro, rd, ds.plane_point, ds.plane_normal);
                            if let Some(hit_pt) = hit {
                                let new_pos = ds.node_start + (hit_pt - ds.plane_point);
                                if let Some(node) = self.workspace.graph.get_node_mut(node_id) {
                                    node.position = Position3 {
                                        x: new_pos.x,
                                        y: new_pos.y,
                                        z: new_pos.z,
                                    };
                                }
                            }
                        }
                    } else if self.mouse_down {
                        // Left drag on empty space → orbit
                        if let Some(gpu) = &mut self.gpu {
                            gpu.controller.orbit(delta);
                        }
                    } else if self.right_mouse_down {
                        // Right drag → pan
                        if let Some(gpu) = &mut self.gpu {
                            gpu.controller.pan(delta * Vec2::new(-1.0, 1.0));
                        }
                    }
                }
                self.last_cursor = Some(pos);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 0.10,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.005,
                };
                if let Some(gpu) = &mut self.gpu {
                    gpu.controller.zoom(1.0 + scroll);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();
            }
            _ => {}
        }

        if let Some(gpu) = &self.gpu {
            gpu.window.request_redraw();
        }
    }
}

// ── Interaction helpers ───────────────────────────────────────────────────────

impl Renderer {
    fn handle_key(&mut self, code: KeyCode) {
        const ORBIT: f32 = 3.0;
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                if let Some(g) = &mut self.gpu {
                    g.controller.orbit(Vec2::new(0.0, -ORBIT));
                }
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                if let Some(g) = &mut self.gpu {
                    g.controller.orbit(Vec2::new(0.0, ORBIT));
                }
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                if let Some(g) = &mut self.gpu {
                    g.controller.orbit(Vec2::new(-ORBIT, 0.0));
                }
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                if let Some(g) = &mut self.gpu {
                    g.controller.orbit(Vec2::new(ORBIT, 0.0));
                }
            }
            KeyCode::Equal | KeyCode::NumpadAdd => {
                if let Some(g) = &mut self.gpu {
                    g.controller.zoom(0.92);
                }
            }
            KeyCode::Minus | KeyCode::NumpadSubtract => {
                if let Some(g) = &mut self.gpu {
                    g.controller.zoom(1.08);
                }
            }
            KeyCode::Space => {
                if let Some(g) = &mut self.gpu {
                    g.camera = Camera::default();
                    g.camera
                        .set_aspect(g.surface_config.width, g.surface_config.height);
                    g.fly_target = None;
                }
            }
            KeyCode::KeyF => {
                if let Some(sel_id) = self.selected_node {
                    if let Some(node) = self.workspace.graph.get_node(sel_id) {
                        let p = node.position;
                        if let Some(g) = &mut self.gpu {
                            g.fly_target = Some(Vec3::new(p.x, p.y, p.z));
                        }
                    }
                }
            }
            KeyCode::KeyN => {
                // Spawn new thought node near camera target
                let target = self
                    .gpu
                    .as_ref()
                    .map(|g| g.camera.target)
                    .unwrap_or(Vec3::ZERO);
                let offset = Vec3::new(
                    (rand_f32() - 0.5) * 4.0,
                    (rand_f32() - 0.5) * 4.0,
                    (rand_f32() - 0.5) * 4.0,
                );
                let pos = target + offset;
                let label = format!("Thought {}", chrono::Utc::now().format("%H:%M:%S"));
                let engine = MaterializationEngine::new();
                match self.workspace.materialize_thought(&engine, &label) {
                    Ok(result) => {
                        // Set position to camera vicinity
                        if let Some(node) = self.workspace.graph.get_node_mut(result.node_id) {
                            node.position = Position3 {
                                x: pos.x,
                                y: pos.y,
                                z: pos.z,
                            };
                        }
                        self.selected_node = Some(result.node_id);
                        println!("\n[spawn] new node: {} — {}", result.node_id, label);
                        // Phase 9 — audio: contagion ripple when new node appears
                        #[cfg(feature = "audio")]
                        if let Some(gpu) = &self.gpu {
                            gpu.audio
                                .trigger_event(crate::audio::AudioEvent::GhostEmergence);
                        }
                    }
                    Err(e) => eprintln!("\n[spawn] error: {e}"),
                }
            }
            KeyCode::KeyP => {
                self.physics_running = !self.physics_running;
                println!(
                    "\n[physics] {}",
                    if self.physics_running { "ON" } else { "OFF" }
                );
            }
            KeyCode::KeyI => {
                if let Some(sel_id) = self.selected_node {
                    if let Some(node) = self.workspace.graph.get_node(sel_id) {
                        println!("\n─── Selected Node ──────────────────────────────");
                        println!("  ID:       {}", node.id);
                        println!("  Content:  {}", node.content);
                        println!("  Type:     {:?}", node.node_type);
                        println!("  Entropy:  {:.3}", node.entropy);
                        println!("  Gravity:  {:.3}", node.gravity);
                        println!("  Velocity: {:.3}", node.velocity);
                        println!("  Degree:   {}", self.workspace.graph.degree(sel_id));
                        println!("  Void:     {}", node.is_void);
                        println!("───────────────────────────────────────────────");
                    }
                } else {
                    println!("\n[info] no node selected — left-click a node");
                }
            }
            // V — send selected node to void (+ audio event)
            KeyCode::KeyV => {
                if let Some(sel_id) = self.selected_node {
                    match self.workspace.send_to_void(sel_id) {
                        Ok(()) => {
                            println!("\n[void] node {} sent to void", sel_id);
                            #[cfg(feature = "audio")]
                            if let Some(gpu) = &self.gpu {
                                gpu.audio.trigger_event(crate::audio::AudioEvent::VoidEntry);
                            }
                        }
                        Err(e) => eprintln!("\n[void] error: {e}"),
                    }
                }
            }
            // U — extract selected node from void (+ audio event)
            KeyCode::KeyU => {
                if let Some(sel_id) = self.selected_node {
                    match self.workspace.extract_from_void(sel_id) {
                        Ok(()) => {
                            println!("\n[void] node {} extracted from void", sel_id);
                            #[cfg(feature = "audio")]
                            if let Some(gpu) = &self.gpu {
                                gpu.audio.trigger_event(crate::audio::AudioEvent::VoidExit);
                            }
                        }
                        Err(e) => eprintln!("\n[void] error: {e}"),
                    }
                }
            }
            KeyCode::Escape => {
                self.selected_node = None;
            }
            _ => {}
        }
    }

    fn try_start_drag(&mut self, cursor: (f64, f64)) {
        let gpu = match &self.gpu {
            Some(g) => g,
            None => return,
        };
        let w = gpu.surface_config.width as f32;
        let h = gpu.surface_config.height as f32;
        let vp = gpu.camera.view_proj();
        let cam_fwd = gpu.camera.forward();

        let ndc_x = (cursor.0 as f32 / w) * 2.0 - 1.0;
        let ndc_y = 1.0 - (cursor.1 as f32 / h) * 2.0;

        let mut best_id: Option<Uuid> = None;
        let mut best_dist: f32 = f32::MAX;

        for node in self.workspace.graph.nodes() {
            let clip = vp * Vec4::new(node.position.x, node.position.y, node.position.z, 1.0);
            if clip.w <= 0.0 {
                continue;
            }
            let nx = clip.x / clip.w;
            let ny = clip.y / clip.w;
            let threshold =
                ((node.gravity * 0.5 + 0.5).clamp(0.3, 4.0) / clip.w * 2.0 + 0.04).max(0.06);
            let d = ((nx - ndc_x).powi(2) + (ny - ndc_y).powi(2)).sqrt();
            if d < threshold && d < best_dist {
                best_dist = d;
                best_id = Some(node.id);
            }
        }

        if let Some(id) = best_id {
            let node = self.workspace.graph.get_node(id).unwrap();
            let node_pos = Vec3::new(node.position.x, node.position.y, node.position.z);
            let (ray_orig, ray_dir) = cursor_ray(cursor, w, h, &gpu.camera);
            let hit = ray_plane_hit(ray_orig, ray_dir, node_pos, cam_fwd).unwrap_or(node_pos);
            self.drag_state = Some(DragState {
                node_id: id,
                plane_point: hit,
                plane_normal: cam_fwd,
                node_start: node_pos,
            });
        }
    }

    fn pick_node_at(&mut self, cursor: (f64, f64)) {
        let gpu = match &self.gpu {
            Some(g) => g,
            None => return,
        };
        let w = gpu.surface_config.width as f32;
        let h = gpu.surface_config.height as f32;
        let vp = gpu.camera.view_proj();

        let ndc_x = (cursor.0 as f32 / w) * 2.0 - 1.0;
        let ndc_y = 1.0 - (cursor.1 as f32 / h) * 2.0;

        let mut best_id: Option<Uuid> = None;
        let mut best_dist: f32 = f32::MAX;

        for node in self.workspace.graph.nodes() {
            let clip = vp * Vec4::new(node.position.x, node.position.y, node.position.z, 1.0);
            if clip.w <= 0.0 {
                continue;
            }
            let nx = clip.x / clip.w;
            let ny = clip.y / clip.w;
            let threshold =
                ((node.gravity * 0.5 + 0.5).clamp(0.3, 4.0) / clip.w * 2.0 + 0.04).max(0.06);
            let d = ((nx - ndc_x).powi(2) + (ny - ndc_y).powi(2)).sqrt();
            if d < threshold && d < best_dist {
                best_dist = d;
                best_id = Some(node.id);
            }
        }

        if let Some(id) = best_id {
            self.selected_node = Some(id);
            if let Some(node) = self.workspace.graph.get_node(id) {
                println!("\n[select] {} — {}", node.id, node.content);
            }
        } else {
            self.selected_node = None;
        }
    }

    fn autosave_now(&self) {
        if let Some(ref path) = self.config.autosave_path {
            match SqliteWorkspaceStore::new(path) {
                Ok(mut store) => {
                    let snap = self.workspace.snapshot();
                    match store.save_snapshot(&snap) {
                        Ok(_) => println!("\n[autosave] workspace saved to {}", path.display()),
                        Err(e) => eprintln!("\n[autosave] save error: {e}"),
                    }
                }
                Err(e) => eprintln!("\n[autosave] store error: {e}"),
            }
        }
    }
}

// ── Render loop ───────────────────────────────────────────────────────────────

impl Renderer {
    fn render_frame(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        // ── Physics tick ─────────────────────────────────────────────────────
        if self.physics_running {
            let gravity_engine = GravityEngine::new();
            let entropy_engine = EntropyEngine::new();
            self.workspace.step_gravity(&gravity_engine, dt);
            // Tick entropy every ~2 s to avoid overwhelming decay
            self.autosave_timer += dt;
            if self.autosave_timer > 2.0 {
                self.workspace.tick_entropy(&entropy_engine);
            }
        } else {
            self.autosave_timer += dt;
        }

        // ── Autosave every 30 s ───────────────────────────────────────────────
        if self.autosave_timer >= 30.0 {
            self.autosave_timer = 0.0;
            self.autosave_now();
        }

        // ── Re-derive weather / season every 5 s ─────────────────────────────
        let should_rederive = self
            .gpu
            .as_ref()
            .map(|g| now.duration_since(g.last_weather_derive).as_secs_f32() > 5.0)
            .unwrap_or(false);

        if should_rederive {
            let nodes: Vec<&NodeData> = self.workspace.graph.nodes().collect();
            let events = self.workspace.focus.events();
            let season_report =
                CognitiveSeasonDetector::new().detect_season(&nodes, events, &[], Utc::now());
            let sv = season_value_of(season_report.season);
            let oracle_signals = self.workspace.oracle_signals();
            let op = oracle_signals
                .iter()
                .map(|s| s.strength)
                .fold(0.0_f32, f32::max);
            if let Some(gpu) = self.gpu.as_mut() {
                gpu.weather.derive(&nodes, events, Utc::now());
                gpu.last_weather_derive = now;
                gpu.season_value = sv;
                gpu.oracle_pulse = op;
                // Phase 9 — update audio atmosphere from live workspace state
                #[cfg(feature = "audio")]
                self.workspace.derive_audio_atmosphere(&gpu.audio);
            }
        }

        if self.gpu.is_none() {
            return;
        }

        // void_density must be computed before mutable gpu borrow
        let void_density = {
            let vc = self.workspace.graph.nodes().filter(|n| n.is_void).count();
            let tc = self.workspace.graph.node_count().max(1);
            (vc as f32 / tc as f32).clamp(0.0, 1.0)
        };

        let gpu = self.gpu.as_mut().unwrap();

        // ── FPS counter ───────────────────────────────────────────────────────
        gpu.fps_frames += 1;
        gpu.fps_timer += dt;
        if gpu.fps_timer >= 1.0 {
            gpu.fps = gpu.fps_frames as f32 / gpu.fps_timer;
            gpu.fps_timer = 0.0;
            gpu.fps_frames = 0;
        }

        // ── Smooth fly-to ─────────────────────────────────────────────────────
        if let Some(fly_tgt) = gpu.fly_target {
            gpu.camera.target = gpu.camera.target.lerp(fly_tgt, 0.07);
            if gpu.camera.target.distance(fly_tgt) < 0.05 {
                gpu.camera.target = fly_tgt;
                gpu.fly_target = None;
            }
        }

        gpu.elapsed += dt;
        gpu.weather.tick(dt);

        // ── Aura uniform ──────────────────────────────────────────────────────
        let aura_uniform = AuraUniform {
            color_primary: gpu.weather.blended_primary(),
            color_secondary: gpu.weather.blended_secondary(),
            intensity: gpu.weather.blended_intensity(),
            turbulence: gpu.weather.blended_turbulence(),
            pulse_rate: gpu.weather.blended_pulse_rate(),
            time: gpu.elapsed,
            season: gpu.season_value,
            oracle_pulse: gpu.oracle_pulse,
            void_density,
            _pad: 0.0,
        };
        gpu.aura.update(&gpu.queue, &aura_uniform);

        // ── Camera ───────────────────────────────────────────────────────────
        gpu.controller.update(&mut gpu.camera);
        let mut cam_uniform = gpu.camera.to_uniform();
        cam_uniform.time = gpu.elapsed;
        gpu.queue
            .write_buffer(&gpu.camera_buf, 0, bytemuck::bytes_of(&cam_uniform));

        // ── Node instances ────────────────────────────────────────────────────
        let selected = self.selected_node;
        // Collect adjacent nodes to selected (for edge highlighting)
        let adjacent: std::collections::HashSet<Uuid> = if let Some(sel) = selected {
            self.workspace
                .graph
                .edges()
                .filter_map(|e| {
                    if e.source_id == sel {
                        Some(e.target_id)
                    } else if e.target_id == sel {
                        Some(e.source_id)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            std::collections::HashSet::new()
        };
        let has_selection = selected.is_some();

        let nodes: Vec<NodeInstance> = self
            .workspace
            .graph
            .nodes()
            .map(|n| {
                let mut inst = NodeInstance::from_node(n);
                if selected == Some(n.id) {
                    inst.flags |= 16;
                }
                inst
            })
            .collect();

        // ── Edge vertices with highlight ──────────────────────────────────────
        let cam_up = Vec3::Y;
        let mut edge_verts: Vec<EdgeVertex> = Vec::new();
        for edge in self.workspace.graph.edges() {
            if let (Some(src), Some(dst)) = (
                self.workspace.graph.get_node(edge.source_id),
                self.workspace.graph.get_node(edge.target_id),
            ) {
                let sp = [src.position.x, src.position.y, src.position.z];
                let dp = [dst.position.x, dst.position.y, dst.position.z];
                let is_ghost = src.is_ghost || dst.is_ghost;

                let highlight = if !has_selection {
                    0u32 // no selection → all normal
                } else if selected == Some(edge.source_id)
                    || selected == Some(edge.target_id)
                    || adjacent.contains(&edge.source_id)
                    || adjacent.contains(&edge.target_id)
                {
                    1u32 // adjacent → highlight
                } else {
                    2u32 // not adjacent → dim
                };

                let verts =
                    edge_to_vertices(edge, sp, dp, cam_up, edge.weight, is_ghost, highlight);
                edge_verts.extend_from_slice(&verts);
            }
        }

        let gpu = self.gpu.as_mut().unwrap();
        gpu.node_pipeline.upload(&gpu.device, &gpu.queue, &nodes);
        gpu.edge_pipeline
            .upload(&gpu.device, &gpu.queue, &edge_verts);

        let particle_dt = dt * gpu.weather.blended_particle_speed();
        gpu.particles.update_params(&gpu.queue, particle_dt);
        gpu.particles.upload(&gpu.queue);

        // ── Acquire surface frame ─────────────────────────────────────────────
        let surface_tex = match gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                gpu.surface.configure(&gpu.device, &gpu.surface_config);
                return;
            }
            Err(e) => {
                eprintln!("surface error: {e}");
                return;
            }
        };

        let view = surface_tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        gpu.particles.compute(&mut encoder);
        gpu.particles.copy_to_render(&mut encoder);

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.config.background_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            gpu.aura.draw(&mut rpass);
            gpu.edge_pipeline.draw(&mut rpass, &gpu.camera_bind_group);
            gpu.node_pipeline.draw(&mut rpass, &gpu.camera_bind_group);
            gpu.particles.draw(&mut rpass, &gpu.camera_bind_group);
        }

        // ── Screenshot readback (Shift+S) ─────────────────────────────────────
        let do_screenshot = gpu.pending_screenshot;
        if do_screenshot {
            gpu.pending_screenshot = false;
        }

        let screenshot_staging: Option<(wgpu::Buffer, u32, u32, u32)> = if do_screenshot {
            let w = gpu.surface_config.width;
            let h = gpu.surface_config.height;
            let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let bpr = ((w * 4) + align - 1) / align * align;
            let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("screenshot_buf"),
                size: (bpr * h) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                surface_tex.texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &buf,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(bpr),
                        rows_per_image: Some(h),
                    },
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            Some((buf, w, h, bpr))
        } else {
            None
        };

        gpu.queue.submit(std::iter::once(encoder.finish()));

        // ── Flush screenshot after submit ─────────────────────────────────────
        if let Some((staging, w, h, bpr)) = screenshot_staging {
            let slice = staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            gpu.device.poll(wgpu::Maintain::Wait);
            if rx.recv().is_ok() {
                let raw = slice.get_mapped_range();
                let mut pixels: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
                for row in 0..h {
                    let start = (row * bpr) as usize;
                    pixels.extend_from_slice(&raw[start..start + (w * 4) as usize]);
                }
                drop(raw);
                // Swap BGRA → RGBA if the surface format is BGRA
                let is_bgra = matches!(
                    gpu.surface_config.format,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
                );
                if is_bgra {
                    for chunk in pixels.chunks_exact_mut(4) {
                        chunk.swap(0, 2);
                    }
                }
                let fname = format!(
                    "screenshot_{}.png",
                    chrono::Utc::now().format("%Y%m%d_%H%M%S")
                );
                match image::RgbaImage::from_raw(w, h, pixels) {
                    Some(img) => {
                        if let Err(e) = img.save(&fname) {
                            eprintln!("\n[screenshot] save error: {e}");
                        } else {
                            println!("\n[screenshot] saved → {fname}");
                        }
                    }
                    None => eprintln!("\n[screenshot] failed to build image buffer"),
                }
            }
        }

        surface_tex.present();
        gpu.particles.prune_dead();

        // ── HUD terminal status line ──────────────────────────────────────────
        let nc = self.workspace.graph.node_count();
        let ec = self.workspace.graph.edge_count();
        let fps = gpu.fps;
        let sel = self
            .selected_node
            .and_then(|id| self.workspace.graph.get_node(id))
            .map(|n| n.content.chars().take(22).collect::<String>())
            .unwrap_or_else(|| "—".to_string());
        let phys = if self.physics_running { "ON" } else { "OFF" };
        print!("\r[FPS:{fps:5.1}] [N:{nc:4}] [E:{ec:5}] [Phys:{phys}] [Sel:{sel}]   ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn season_value_of(s: CognitiveSeason) -> f32 {
    match s {
        CognitiveSeason::Spring => 0.0,
        CognitiveSeason::Summer => 1.0,
        CognitiveSeason::Autumn => 2.0,
        CognitiveSeason::Winter => 3.0,
    }
}

/// Fast pseudo-random f32 in [0,1] using thread-local LCG (no deps needed).
fn rand_f32() -> f32 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = Cell::new(0x9e3779b97f4a7c15);
    }
    STATE.with(|s| {
        let mut x = s.get().wrapping_add(0x6c62272e07bb0142);
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        x ^= x >> 31;
        s.set(x);
        (x >> 33) as f32 / (u32::MAX as f32)
    })
}

/// Launch the windowed GPU renderer. Blocks until the window is closed.
pub fn launch(workspace: SilentNodeWorkspace, config: RenderConfig) {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = Renderer::new(workspace, config);
    event_loop.run_app(&mut app).expect("event loop failed");
}
