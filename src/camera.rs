use nalgebra_glm::{self as glm, Mat4, Vec3};

/// Cámara orbital: gira alrededor del objetivo con yaw/pitch y se acerca con `distance`.
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Camera {
    pub fn eye(&self) -> Vec3 {
        self.target
            + Vec3::new(
                self.pitch.cos() * self.yaw.sin(),
                self.pitch.sin(),
                self.pitch.cos() * self.yaw.cos(),
            ) * self.distance
    }

    pub fn view_matrix(&self) -> Mat4 {
        glm::look_at(&self.eye(), &self.target, &Vec3::new(0.0, 1.0, 0.0))
    }
}

pub fn projection_matrix(width: f32, height: f32) -> Mat4 {
    glm::perspective(width / height, 45f32.to_radians(), 0.1, 100.0)
}
