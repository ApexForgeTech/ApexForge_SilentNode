use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    pub aspect: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 50.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov_y: 45_f32.to_radians(),
            near: 0.1,
            far: 2000.0,
            aspect: 1.0,
        }
    }
}

impl Camera {
    pub fn new(position: Vec3, target: Vec3, aspect: f32) -> Self {
        Self {
            position,
            target,
            aspect,
            ..Default::default()
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }

    pub fn view_proj(&self) -> Mat4 {
        self.proj_matrix() * self.view_matrix()
    }

    pub fn to_uniform(&self) -> CameraUniform {
        CameraUniform {
            view_proj: self.view_proj().to_cols_array_2d(),
            camera_pos: self.position.to_array(),
            time: 0.0,
        }
    }

    pub fn set_aspect(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height.max(1) as f32;
    }

    /// Unit vector from camera toward target.
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize_or_zero()
    }

    /// Inverse view-projection matrix — used for unprojecting cursor to world space.
    pub fn inv_view_proj(&self) -> Mat4 {
        self.view_proj().inverse()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    /// Elapsed seconds — injected by the render loop; used by node/edge shaders for animation.
    pub time: f32,
}

#[derive(Debug, Clone)]
pub struct CameraController {
    pub pan_speed: f32,
    pub zoom_speed: f32,
    pub orbit_speed: f32,
    pub inertia: f32,

    // accumulated velocity for inertia
    pan_vel: glam::Vec2,
    zoom_vel: f32,
    orbit_vel: glam::Vec2,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            pan_speed: 0.05,
            zoom_speed: 1.1,
            orbit_speed: 0.005,
            inertia: 0.88,
            pan_vel: glam::Vec2::ZERO,
            zoom_vel: 0.0,
            orbit_vel: glam::Vec2::ZERO,
        }
    }
}

impl CameraController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pan(&mut self, delta: glam::Vec2) {
        self.pan_vel += delta * self.pan_speed;
    }

    pub fn zoom(&mut self, factor: f32) {
        self.zoom_vel += (factor - 1.0) * self.zoom_speed;
    }

    pub fn orbit(&mut self, delta: glam::Vec2) {
        self.orbit_vel += delta * self.orbit_speed;
    }

    /// Apply accumulated velocity to camera, then decay with inertia.
    pub fn update(&mut self, camera: &mut Camera) {
        // Orbit: rotate position around target
        if self.orbit_vel.length_squared() > 1e-8 {
            let arm = camera.position - camera.target;
            let yaw = Mat4::from_rotation_y(-self.orbit_vel.x);
            let right = arm.cross(camera.up).normalize();
            let pitch = Mat4::from_axis_angle(right, -self.orbit_vel.y);
            let rotated = (yaw * pitch).transform_vector3(arm);
            camera.position = camera.target + rotated;
        }

        // Pan: move both position and target in the view plane
        if self.pan_vel.length_squared() > 1e-8 {
            let view_right = (camera.target - camera.position)
                .cross(camera.up)
                .normalize();
            let view_up = view_right
                .cross(camera.target - camera.position)
                .normalize();
            let offset = view_right * (-self.pan_vel.x) + view_up * self.pan_vel.y;
            camera.position += offset;
            camera.target += offset;
        }

        // Zoom: move toward/away from target
        if self.zoom_vel.abs() > 1e-6 {
            let arm = camera.position - camera.target;
            let new_len = (arm.length() * (1.0 - self.zoom_vel)).clamp(1.0, 5000.0);
            camera.position = camera.target + arm.normalize() * new_len;
            self.zoom_vel = 0.0;
        }

        // Apply inertia decay
        self.pan_vel *= self.inertia;
        self.orbit_vel *= self.inertia;
    }
}
