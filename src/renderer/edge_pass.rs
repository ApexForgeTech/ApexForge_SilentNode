use crate::domain::EdgeData;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

/// Single vertex for an edge segment (each edge = 6 verts / 2 tris forming a quad).
///
/// Layout (48 bytes):
///   [0]  position  vec3  (12)
///   [12] normal    vec3  (12)
///   [24] weight    f32   (4)
///   [28] activity  f32   (4)
///   [32] is_ghost  u32   (4)
///   [36] uv_along  f32   (4)
///   [40] highlight u32   (4)   0=normal 1=adjacent-to-selected 2=dimmed
///   [44] _pad      u32   (4)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EdgeVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub weight: f32,
    pub activity: f32,
    pub is_ghost: u32,
    pub uv_along: f32,
    pub highlight: u32,
    pub _pad: u32,
}

impl EdgeVertex {
    pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 28,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: 36,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 40,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

/// Build 6 EdgeVertex values (two triangles forming a quad) for one edge.
///
/// `highlight`: 0 = normal, 1 = adjacent to selected node, 2 = dimmed (not adjacent)
pub fn edge_to_vertices(
    edge: &EdgeData,
    src_pos: [f32; 3],
    dst_pos: [f32; 3],
    cam_up: Vec3,
    activity: f32,
    is_ghost: bool,
    highlight: u32,
) -> [EdgeVertex; 6] {
    let a = Vec3::from(src_pos);
    let b = Vec3::from(dst_pos);
    let dir = (b - a).normalize_or_zero();
    let normal = dir.cross(cam_up).normalize_or_zero();

    let make = |pos: Vec3, n: Vec3, uv: f32| EdgeVertex {
        position: pos.to_array(),
        normal: n.to_array(),
        weight: edge.weight,
        activity,
        is_ghost: is_ghost as u32,
        uv_along: uv,
        highlight,
        _pad: 0,
    };

    [
        make(a, -normal, 0.0),
        make(a, normal, 0.0),
        make(b, normal, 1.0),
        make(a, -normal, 0.0),
        make(b, normal, 1.0),
        make(b, -normal, 1.0),
    ]
}

pub struct EdgePipeline {
    pub pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    vertex_count: u32,
    capacity: u32,
}

impl EdgePipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        initial_capacity: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("edge_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/edge.wgsl").into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("edge_pipeline_layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("edge_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[EdgeVertex::vertex_layout()],
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

        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("edge_vertex_buf"),
            size: (initial_capacity as u64) * std::mem::size_of::<EdgeVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buf,
            vertex_count: 0,
            capacity: initial_capacity,
        }
    }

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[EdgeVertex]) {
        self.vertex_count = vertices.len() as u32;
        if vertices.is_empty() {
            return;
        }

        let required = vertices.len() as u32;
        if required > self.capacity {
            let new_cap = (required * 3 / 2).max(64);
            self.vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("edge_vertex_buf"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.capacity = new_cap;
        } else {
            queue.write_buffer(&self.vertex_buf, 0, bytemuck::cast_slice(vertices));
        }
    }

    pub fn draw<'rp>(
        &'rp self,
        pass: &mut wgpu::RenderPass<'rp>,
        camera_bind_group: &'rp wgpu::BindGroup,
    ) {
        if self.vertex_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}
