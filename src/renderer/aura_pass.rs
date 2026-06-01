use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// GPU uniform matching `AuraUniform` in `shaders/aura.wgsl` exactly.
/// Layout: 64 bytes (4 × 16-byte rows — valid std140).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct AuraUniform {
    pub color_primary: [f32; 4],   // 16
    pub color_secondary: [f32; 4], // 16
    pub intensity: f32,            // 4
    pub turbulence: f32,           // 4
    pub pulse_rate: f32,           // 4
    pub time: f32,                 // 4
    /// Cognitive season: 0.0=spring, 1.0=summer, 2.0=autumn, 3.0=winter
    pub season: f32, // 4
    /// Oracle signal strength (0=none, >0 = shooting star effect active)
    pub oracle_pulse: f32, // 4
    /// Void zone count normalised 0..1 (darkens nebula where voids cluster)
    pub void_density: f32, // 4
    pub _pad: f32,                 // 4  (alignment)
}

impl Default for AuraUniform {
    fn default() -> Self {
        Self {
            color_primary: [0.03, 0.05, 0.10, 1.0],
            color_secondary: [0.35, 0.90, 0.55, 1.0],
            intensity: 0.35,
            turbulence: 0.05,
            pulse_rate: 0.2,
            time: 0.0,
            season: 0.0,
            oracle_pulse: 0.0,
            void_density: 0.0,
            _pad: 0.0,
        }
    }
}

pub struct AuraPass {
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl AuraPass {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("aura_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/aura.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("aura_bgl"),
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

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("aura_uniform_buf"),
            contents: bytemuck::bytes_of(&AuraUniform::default()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("aura_bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("aura_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("aura_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_aura",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_aura",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
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

        Self {
            pipeline,
            uniform_buf,
            bind_group,
            bind_group_layout,
        }
    }

    /// Upload updated uniform to GPU.
    pub fn update(&self, queue: &wgpu::Queue, uniform: &AuraUniform) {
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(uniform));
    }

    /// Draw the full-screen background (3 vertices = one covering triangle).
    pub fn draw<'rp>(&'rp self, pass: &mut wgpu::RenderPass<'rp>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}
