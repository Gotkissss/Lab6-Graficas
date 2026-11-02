//! Instancias de la MISMA esfera. Cada instancia solo cambia su matriz de
//! modelo y el shader que la colorea.

use nalgebra_glm::{self as glm, Mat3, Mat4, Vec3};

/// Radio de la esfera proporcionada (sus vértices están a ~0.866 del origen).
pub const SPHERE_RADIUS: f32 = 0.866;
/// Escala horizontal de la matriz de modelo de los anillos (radio externo ≈ 2.4 radios del planeta).
pub const RING_SCALE: f32 = 2.5;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShaderKind {
    Star,
    Rocky,
    Moon,
    GasGiant,
    Rings,
    Lava,
    Ice,
    Alien,
}

#[derive(Clone, Copy)]
pub struct Instance {
    pub kind: ShaderKind,
    pub center: Vec3,
    pub radius: f32,
    /// Inclinación del cuerpo (los anillos usan la misma que su planeta).
    pub rot: Mat3,
    pub model: Mat4,
}

impl Instance {
    pub fn new(kind: ShaderKind, center: Vec3, radius: f32, tilt_deg: f32) -> Self {
        let rot4 = glm::rotate(
            &Mat4::identity(),
            tilt_deg.to_radians(),
            &Vec3::new(0.0, 0.0, 1.0),
        );
        let s = radius / SPHERE_RADIUS;
        // Los anillos son la MISMA esfera con una escala no uniforme en la matriz
        // de modelo: muy delgada en Y y ancha en XZ (un disco). Los vértices del
        // modelo no se modifican; todo el detalle lo pone el fragment shader.
        let scale = if kind == ShaderKind::Rings {
            Vec3::new(s * RING_SCALE, s * 0.002, s * RING_SCALE)
        } else {
            Vec3::new(s, s, s)
        };
        let model = glm::translate(&Mat4::identity(), &center)
            * rot4
            * glm::scale(&Mat4::identity(), &scale);
        let rot = glm::mat4_to_mat3(&rot4);
        Instance {
            kind,
            center,
            radius,
            rot,
            model,
        }
    }
}

pub const BODY_NAMES: [&str; 7] = [
    "Sistema completo",
    "Helios (estrella)",
    "Terra Nova (rocoso) + Selene (luna)",
    "Zephyr (gigante gaseoso) + anillos",
    "Ignis (mundo de lava)",
    "Glacies (mundo de hielo)",
    "Nyx (mundo bioluminiscente)",
];

/// Distancia de cámara recomendada para cada vista.
pub fn default_distance(sel: usize) -> f32 {
    match sel {
        0 => 12.0,
        2 => 4.6,
        3 => 5.0,
        _ => 3.3,
    }
}

/// Construye la escena de la vista seleccionada (0 = sistema, 1..6 = cuerpos).
pub fn build(sel: usize, time: f32) -> Vec<Instance> {
    use ShaderKind::*;
    let o = Vec3::zeros();
    let moon_at = |planet: Vec3, dist: f32, r: f32| {
        let a = time * 0.35 + 1.2;
        Instance::new(
            Moon,
            planet + Vec3::new(a.cos() * dist, a.sin() * dist * 0.18, a.sin() * dist),
            r,
            0.0,
        )
    };
    match sel {
        1 => vec![Instance::new(Star, o, 1.0, 0.0)],
        2 => vec![Instance::new(Rocky, o, 1.0, 0.0), moon_at(o, 2.0, 0.27)],
        3 => vec![
            Instance::new(GasGiant, o, 1.0, 22.0),
            Instance::new(Rings, o, 1.0, 22.0),
        ],
        4 => vec![Instance::new(Lava, o, 1.0, 0.0)],
        5 => vec![Instance::new(Ice, o, 1.0, 0.0)],
        6 => vec![Instance::new(Alien, o, 1.0, 0.0)],
        _ => {
            // Vista general: la estrella en el centro y los planetas alrededor.
            let rocky = Vec3::new(-4.6, 0.0, -1.2);
            let gas = Vec3::new(5.3, 0.0, -1.8);
            vec![
                Instance::new(Star, o, 1.6, 0.0),
                Instance::new(Lava, Vec3::new(-2.6, 0.0, -0.3), 0.45, 0.0),
                Instance::new(Rocky, rocky, 0.65, 0.0),
                moon_at(rocky, 1.05, 0.18),
                Instance::new(Alien, Vec3::new(2.9, 0.0, -0.6), 0.55, 0.0),
                Instance::new(GasGiant, gas, 0.95, 22.0),
                Instance::new(Rings, gas, 0.95, 22.0),
                Instance::new(Ice, Vec3::new(-6.8, 0.0, -3.4), 0.6, 0.0),
            ]
        }
    }
}
