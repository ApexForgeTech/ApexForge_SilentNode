use crate::domain::{NodeData, NodeType};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Per-instance GPU data for a single node.
///
/// Layout (72 bytes total, 4-byte aligned):
///   [0]  position    vec3  (12)
///   [12] radius      f32   (4)
///   [16] color       vec4  (16)
///   [32] entropy     f32   (4)
///   [36] gravity_mass f32  (4)
///   [40] velocity_mag f32  (4)
///   [44] node_type   u32   (4)
///   [48] civ_color   vec4  (16)   civilization membership color
///   [64] flags       u32   (4)    bit0=void, bit1=crystal, bit2=shadow, bit3=oracle_target
///   [68] _pad        u32   (4)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct NodeInstance {
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 4],
    pub entropy: f32,
    pub gravity_mass: f32,
    pub velocity_mag: f32,
    pub node_type: u32,
    pub civ_color: [f32; 4],
    pub flags: u32,
    pub _pad: u32,
}

impl NodeInstance {
    pub fn from_node(node: &NodeData) -> Self {
        let color = node_type_color(node.node_type);
        let radius = (0.5 + node.gravity * 0.3).clamp(0.3, 4.0);
        let civ_color = civilization_color(node.civilization_id);
        let mut flags: u32 = 0;
        if node.is_void {
            flags |= 1;
        }
        if node.is_fossil {
            flags |= 2;
        }

        Self {
            position: [node.position.x, node.position.y, node.position.z],
            radius,
            color,
            entropy: node.entropy,
            gravity_mass: node.gravity,
            velocity_mag: node.velocity,
            node_type: node_type_id(node.node_type),
            civ_color,
            flags,
            _pad: 0,
        }
    }

    /// Mark this instance as an oracle-anticipated target (for rendering).
    pub fn with_oracle_target(mut self) -> Self {
        self.flags |= 8;
        self
    }

    /// Mark this instance as a digital shadow.
    pub fn with_shadow_flag(mut self) -> Self {
        self.flags |= 4;
        self
    }

    pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 36,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 40,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 44,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

pub struct NodePipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub instance_buf: wgpu::Buffer,
    pub instance_count: u32,
    capacity: u32,
}

impl NodePipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        initial_capacity: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("node_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/node.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("node_pipeline_layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("node_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[NodeInstance::vertex_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("node_instance_buf"),
            size: (initial_capacity as u64) * std::mem::size_of::<NodeInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            instance_buf,
            instance_count: 0,
            capacity: initial_capacity,
        }
    }

    /// Upload node instances from graph nodes. Reallocates buffer if needed.
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, nodes: &[NodeInstance]) {
        self.instance_count = nodes.len() as u32;
        if nodes.is_empty() {
            return;
        }

        let required = nodes.len() as u32;
        if required > self.capacity {
            // Grow buffer with 50% headroom
            let new_cap = (required * 3 / 2).max(64);
            self.instance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("node_instance_buf"),
                contents: bytemuck::cast_slice(nodes),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.capacity = new_cap;
        } else {
            queue.write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(nodes));
        }
    }

    pub fn draw<'rp>(
        &'rp self,
        pass: &mut wgpu::RenderPass<'rp>,
        camera_bind_group: &'rp wgpu::BindGroup,
    ) {
        if self.instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buf.slice(..));
        // 6 vertices per billboard quad, drawn instance_count times
        pass.draw(0..6, 0..self.instance_count);
    }
}

fn node_type_color(nt: NodeType) -> [f32; 4] {
    match nt {
        NodeType::Idea => [0.35, 0.65, 1.00, 1.0], // electric blue
        NodeType::Memory => [0.72, 0.38, 1.00, 1.0], // violet
        NodeType::Project => [0.25, 0.95, 0.55, 1.0], // emerald
        NodeType::Person => [1.00, 0.38, 0.38, 1.0], // coral red
        NodeType::Artifact => [0.75, 0.72, 0.68, 1.0], // warm silver
        NodeType::Media => [1.00, 0.88, 0.28, 1.0], // gold
        NodeType::Process => [0.28, 1.00, 0.85, 1.0], // cyan-teal
        NodeType::World => [0.42, 0.92, 0.42, 1.0], // vivid green
        NodeType::Ghost => [0.28, 0.28, 0.33, 0.38], // dim slate
        NodeType::Fossil => [0.58, 0.50, 0.32, 0.65], // aged amber
        NodeType::Other => [0.58, 0.64, 0.72, 1.0], // slate
    }
}

/// Deterministic civilization color from UUID (golden-angle hue stepping).
fn civilization_color(civ_id: Option<uuid::Uuid>) -> [f32; 4] {
    match civ_id {
        None => [0.0, 0.0, 0.0, 0.0],
        Some(id) => {
            // Use the last 4 bytes of the UUID as a u32 seed
            let bytes = id.as_bytes();
            let seed = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
            // Golden-ratio hue stepping: hue = (seed * 0.618) % 1
            let hue = ((seed as f64 * 0.618_033_988_749_895) % 1.0) as f32;
            // HSV to RGB (S=0.7, V=0.9)
            let (r, g, b) = hsv_to_rgb(hue, 0.70, 0.90);
            [r, g, b, 1.0]
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let i = (h * 6.0).floor() as u32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

fn node_type_id(nt: NodeType) -> u32 {
    match nt {
        NodeType::Idea => 0,
        NodeType::Memory => 1,
        NodeType::Project => 2,
        NodeType::Person => 3,
        NodeType::Artifact => 4,
        NodeType::Media => 5,
        NodeType::Process => 6,
        NodeType::World => 7,
        NodeType::Ghost => 8,
        NodeType::Fossil => 9,
        NodeType::Other => 10,
    }
}
