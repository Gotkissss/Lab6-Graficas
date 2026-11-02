//! Vertex shader, prueba de descarte y despacho de fragment shaders.

pub mod common;
mod gas;
mod ice;
mod lava;
mod moon;
mod nyx;
mod rings;
mod rocky;
mod star;

use crate::framebuffer::Fragment;
use crate::noise::Noises;
use crate::render::Uniforms;
use crate::scene::{Instance, ShaderKind, RING_SCALE, SPHERE_RADIUS};
use crate::vertex::Vertex;
use nalgebra_glm::Vec3;

/// Entrada común a todos los fragment shaders (en espacio del objeto).
pub struct Surface {
    /// Dirección unitaria del fragmento sobre la esfera = normal geométrica.
    pub p: Vec3,
    /// Posición original del modelo (sin normalizar).
    pub obj: Vec3,
    /// Dirección hacia la luz (espacio del objeto).
    pub l: Vec3,
    /// Dirección hacia la cámara (espacio del objeto).
    pub v: Vec3,
}

/// Vertex shader: los vértices NO se deforman; la matriz de modelo de cada
/// instancia (en render.rs) los lleva al mundo. Los anillos usan una matriz de
/// escala no uniforme (ver scene.rs) en lugar de otra geometría.
pub fn vertex_shader(v: &Vertex, _inst: &Instance) -> Vec3 {
    v.position
}

/// Descarte de fragmentos (equivalente a `discard` en GLSL). Solo los anillos
/// descartan: el centro del disco (donde está el planeta) y las divisiones vacías.
pub fn discard(inst: &Instance, obj: &Vec3) -> bool {
    match inst.kind {
        ShaderKind::Rings => rings::density(ring_radius(obj)) < 0.06,
        _ => false,
    }
}

/// Radio del anillo (en radios del planeta) para una posición del modelo.
pub fn ring_radius(obj: &Vec3) -> f32 {
    (obj.x * obj.x + obj.z * obj.z).sqrt() / SPHERE_RADIUS * RING_SCALE
}

pub fn fragment_shader(
    f: &Fragment,
    inst: &Instance,
    all: &[Instance],
    u: &Uniforms,
    nz: &Noises,
) -> Vec3 {
    let to_obj = inst.rot.transpose();
    let l_world = (u.light_pos - f.world)
        .try_normalize(1e-6)
        .unwrap_or(Vec3::new(0.0, 1.0, 0.0));
    let v_world = (u.cam_pos - f.world).normalize();
    let s = Surface {
        p: f.obj.normalize(),
        obj: f.obj,
        l: to_obj * l_world,
        v: to_obj * v_world,
    };
    match inst.kind {
        ShaderKind::Star => star::shade(&s, u, nz),
        ShaderKind::Rocky => rocky::shade(&s, u, nz),
        ShaderKind::Moon => moon::shade(&s, u, nz),
        ShaderKind::GasGiant => gas::shade(&s, u, nz),
        ShaderKind::Rings => rings::shade(&s, u, nz, inst, all),
        ShaderKind::Lava => lava::shade(&s, u, nz),
        ShaderKind::Ice => ice::shade(&s, u, nz),
        ShaderKind::Alien => nyx::shade(&s, u, nz),
    }
}
