use nalgebra_glm::Vec3;

/// Vértice tal como viene del modelo (la normal se conserva aunque los shaders
/// usan la dirección normalizada de la posición, que en una esfera es la normal suave).
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

/// Vértice después del vertex shader.
#[derive(Clone, Copy, Debug)]
pub struct TransformedVertex {
    /// Posición en pantalla (x, y en píxeles; z = profundidad NDC).
    pub screen: Vec3,
    /// 1/w para interpolación con corrección de perspectiva.
    pub inv_w: f32,
    /// Posición en el mundo.
    pub world: Vec3,
    /// Posición original en espacio del objeto (sin deformar).
    pub obj: Vec3,
}
