pub mod aura_pass;
pub mod camera;
pub mod edge_pass;
pub mod node_pass;
pub mod particles;
pub mod window;

pub use aura_pass::{AuraPass, AuraUniform};
pub use camera::{Camera, CameraController, CameraUniform};
pub use edge_pass::{edge_to_vertices, EdgePipeline, EdgeVertex};
pub use node_pass::{NodeInstance, NodePipeline};
pub use particles::{Particle, ParticleSystem};
pub use window::launch;

use crate::workspace::SilentNodeWorkspace;
use glam::Vec3;
use wgpu::util::DeviceExt;

/// Configuration for rendering (headless or windowed).
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub width: u32,
    pub height: u32,
    pub background_color: wgpu::Color,
    pub node_capacity: u32,
    pub edge_capacity: u32,
    /// If set, the renderer autosaves the workspace to this SQLite path every 30 s.
    pub autosave_path: Option<std::path::PathBuf>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            background_color: wgpu::Color {
                r: 0.03,
                g: 0.05,
                b: 0.10,
                a: 1.0,
            },
            node_capacity: 10_000,
            edge_capacity: 60_000,
            autosave_path: None,
        }
    }
}

/// Headless GPU device — used for tests and offline rendering.
pub struct RenderDevice {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: RenderConfig,
    pub camera: Camera,
    camera_buf: wgpu::Buffer,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_bind_group: wgpu::BindGroup,
}

impl RenderDevice {
    pub async fn new_headless(config: RenderConfig) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("silentnode_headless"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .ok()?;

        let camera = Camera {
            aspect: config.width as f32 / config.height as f32,
            ..Camera::default()
        };

        let camera_uniform = camera.to_uniform();
        let camera_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_uniform_buf"),
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

        Some(Self {
            device,
            queue,
            config,
            camera,
            camera_buf,
            camera_bind_group_layout,
            camera_bind_group,
        })
    }

    pub fn update_camera(&self) {
        let uniform = self.camera.to_uniform();
        self.queue
            .write_buffer(&self.camera_buf, 0, bytemuck::bytes_of(&uniform));
    }

    /// Render workspace to raw RGBA8 bytes. Used for tests and PNG export.
    pub fn render_to_buffer(&mut self, workspace: &SilentNodeWorkspace) -> Vec<u8> {
        let cfg = &self.config;

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen_tex"),
            size: wgpu::Extent3d {
                width: cfg.width,
                height: cfg.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let aura_pass = AuraPass::new(&self.device, format);
        let mut node_p = NodePipeline::new(
            &self.device,
            format,
            &self.camera_bind_group_layout,
            cfg.node_capacity,
        );
        let mut edge_p = EdgePipeline::new(
            &self.device,
            format,
            &self.camera_bind_group_layout,
            cfg.edge_capacity,
        );

        let nodes: Vec<NodeInstance> = workspace
            .graph
            .nodes()
            .map(NodeInstance::from_node)
            .collect();

        let cam_up = Vec3::Y;
        let mut edge_verts: Vec<EdgeVertex> = Vec::new();
        for edge in workspace.graph.edges() {
            if let (Some(src), Some(dst)) = (
                workspace.graph.get_node(edge.source_id),
                workspace.graph.get_node(edge.target_id),
            ) {
                let sp = [src.position.x, src.position.y, src.position.z];
                let dp = [dst.position.x, dst.position.y, dst.position.z];
                let ghost = src.is_ghost || dst.is_ghost;
                edge_verts.extend_from_slice(&edge_to_vertices(
                    edge,
                    sp,
                    dp,
                    cam_up,
                    edge.weight,
                    ghost,
                    0,
                ));
            }
        }

        self.update_camera();
        node_p.upload(&self.device, &self.queue, &nodes);
        edge_p.upload(&self.device, &self.queue, &edge_verts);

        // Default aura uniform for headless render
        aura_pass.update(&self.queue, &AuraUniform::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(cfg.background_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            aura_pass.draw(&mut rpass);
            edge_p.draw(&mut rpass, &self.camera_bind_group);
            node_p.draw(&mut rpass, &self.camera_bind_group);
        }

        let bytes_per_row = align_to(cfg.width * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback_buf"),
            size: (bytes_per_row * cfg.height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback_buf,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(cfg.height),
                },
            },
            wgpu::Extent3d {
                width: cfg.width,
                height: cfg.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = readback_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .expect("readback closed")
            .expect("readback failed");

        let mapped = slice.get_mapped_range();
        let mut out = Vec::with_capacity((cfg.width * cfg.height * 4) as usize);
        for row in 0..cfg.height as usize {
            let start = row * bytes_per_row as usize;
            out.extend_from_slice(&mapped[start..start + cfg.width as usize * 4]);
        }
        out
    }
}

fn align_to(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}

/// Frame stats reported after each render tick.
#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    pub node_count: u32,
    pub edge_count: u32,
    pub particle_count: u32,
    pub frame_time_ms: f32,
}
