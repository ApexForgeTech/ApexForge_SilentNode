use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

/// GPU-resident particle — matches the WGSL `Particle` struct layout exactly.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Particle {
    pub position: [f32; 3],
    pub lifetime: f32,
    pub velocity: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
    pub dest: [f32; 3],
    pub node_affinity: f32,
}

impl Particle {
    /// A contagion spark emitted from `src` toward `dst`.
    pub fn contagion_spark(src: Vec3, dst: Vec3, color: [f32; 4]) -> Self {
        let jitter = Vec3::new(
            (rand_f32() - 0.5) * 2.0,
            (rand_f32() - 0.5) * 2.0,
            (rand_f32() - 0.5) * 2.0,
        );
        Self {
            position: src.to_array(),
            lifetime: 1.5 + rand_f32() * 0.5,
            velocity: (jitter * 0.5).to_array(),
            size: 0.08 + rand_f32() * 0.06,
            color,
            dest: dst.to_array(),
            node_affinity: 0.8,
        }
    }

    pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // lifetime
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32,
                },
                // size
                wgpu::VertexAttribute {
                    offset: 28,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
                // color
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct SimParams {
    delta_time: f32,
    gravity_strength: f32,
    drag: f32,
    _pad: f32,
}

pub struct ParticleSystem {
    /// CPU-side particle list (source of truth between frames)
    pub particles: Vec<Particle>,
    storage_buf: wgpu::Buffer,
    render_buf: wgpu::Buffer,
    params_buf: wgpu::Buffer,
    compute_bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
    pub render_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    capacity: usize,
}

impl ParticleSystem {
    pub const MAX_PARTICLES: usize = 8192;

    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        // Compile shaders
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particles_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/particles.wgsl").into()),
        });

        // Storage buffer (compute R/W + copy to render)
        let storage_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_storage"),
            size: (Self::MAX_PARTICLES * std::mem::size_of::<Particle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Render-only vertex buffer (COPY_DST from storage each frame)
        let render_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_render"),
            size: (Self::MAX_PARTICLES * std::mem::size_of::<Particle>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle_params"),
            contents: bytemuck::bytes_of(&SimParams {
                delta_time: 0.016,
                gravity_strength: 3.0,
                drag: 1.5,
                _pad: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Compute bind group layout
        let compute_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle_compute_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle_compute_bg"),
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: storage_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        let compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle_compute_layout"),
            bind_group_layouts: &[&compute_bgl],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("particle_compute"),
            layout: Some(&compute_layout),
            module: &shader,
            entry_point: "cs_update",
            compilation_options: Default::default(),
            cache: None,
        });

        // Render pipeline uses the shared camera bind group at group 0 (same as nodes/edges).
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("particle_render_layout"),
                bind_group_layouts: &[camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particle_render"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_particle",
                buffers: &[Particle::vertex_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_particle",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One, // additive for glow
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
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
            particles: Vec::new(),
            storage_buf,
            render_buf,
            params_buf,
            compute_bind_group,
            compute_pipeline,
            render_pipeline,
            capacity: Self::MAX_PARTICLES,
        }
    }

    /// Spawn new particles, clamping to MAX_PARTICLES by dropping oldest.
    pub fn emit(&mut self, new_particles: impl IntoIterator<Item = Particle>) {
        for p in new_particles {
            if self.particles.len() >= Self::MAX_PARTICLES {
                self.particles.remove(0);
            }
            self.particles.push(p);
        }
    }

    /// Upload current CPU particles to GPU storage buffer.
    pub fn upload(&self, queue: &wgpu::Queue) {
        if self.particles.is_empty() {
            return;
        }
        let data = bytemuck::cast_slice(&self.particles);
        let capped = &data[..data
            .len()
            .min(Self::MAX_PARTICLES * std::mem::size_of::<Particle>())];
        queue.write_buffer(&self.storage_buf, 0, capped);
    }

    /// Update sim params (delta_time changes each frame).
    pub fn update_params(&self, queue: &wgpu::Queue, delta_time: f32) {
        queue.write_buffer(
            &self.params_buf,
            0,
            bytemuck::bytes_of(&SimParams {
                delta_time,
                gravity_strength: 3.0,
                drag: 1.5,
                _pad: 0.0,
            }),
        );
    }

    /// Dispatch compute pass to advance particle physics on GPU.
    pub fn compute<'a>(&'a self, encoder: &mut wgpu::CommandEncoder) {
        if self.particles.is_empty() {
            return;
        }
        let workgroups = (self.particles.len() as u32 + 63) / 64;
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("particle_compute_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.compute_pipeline);
        pass.set_bind_group(0, &self.compute_bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    /// Copy storage → render buffer so the vertex shader sees updated positions.
    pub fn copy_to_render(&self, encoder: &mut wgpu::CommandEncoder) {
        let byte_len = (self.particles.len() * std::mem::size_of::<Particle>()) as u64;
        if byte_len == 0 {
            return;
        }
        encoder.copy_buffer_to_buffer(&self.storage_buf, 0, &self.render_buf, 0, byte_len);
    }

    pub fn draw<'rp>(
        &'rp self,
        pass: &mut wgpu::RenderPass<'rp>,
        camera_bind_group: &'rp wgpu::BindGroup,
    ) {
        if self.particles.is_empty() {
            return;
        }
        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.render_buf.slice(..));
        pass.draw(0..6, 0..self.particles.len() as u32);
    }

    /// Remove dead particles from CPU list (called once per frame after compute readback).
    pub fn prune_dead(&mut self) {
        self.particles.retain(|p| p.lifetime > 0.0);
    }
}

/// LCG-based stateless float in [0,1] — avoids pulling in rand crate.
fn rand_f32() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    static SEED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let s = SEED.fetch_add(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(1),
        std::sync::atomic::Ordering::Relaxed,
    );
    let v = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (v >> 33) as f32 / (u32::MAX as f32)
}
