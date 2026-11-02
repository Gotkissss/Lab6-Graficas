//! ANILLOS de Zephyr: la misma esfera aplastada por el vertex shader en un
//! disco delgado; el fragment shader descarta el centro y las divisiones.
//!
//! Capas de color:
//!   1. Anillo C interior (polvo oscuro y tenue).
//!   2. Anillo B brillante (hielo crema).
//!   3. División de Cassini y división de Encke (descartadas).
//!   4. Anillo A exterior + ringlets finos (senos de alta frecuencia).
//!   5. Grumos que orbitan + sombra del planeta sobre los anillos.

use super::common::*;
use super::Surface;
use crate::noise::*;
use crate::render::Uniforms;
use crate::scene::Instance;
use nalgebra_glm::Vec3;

const INNER: f32 = 1.28; // radio interior (en radios del planeta)
const OUTER: f32 = 2.32; // radio exterior
const CASSINI: f32 = 1.93; // posición de la división principal

/// Densidad 0..1 de los anillos según el radio (sin ruido: se usa también para
/// descartar fragmentos durante la rasterización y para la sombra en el planeta).
pub fn density(r: f32) -> f32 {
    if r < INNER || r > OUTER {
        return 0.0;
    }
    let edges = smoothstep(INNER, INNER + 0.05, r) * (1.0 - smoothstep(OUTER - 0.06, OUTER, r));
    let c_ring = mix_f(0.35, 1.0, smoothstep(1.48, 1.56, r)); // anillo C tenue
    let cassini = smoothstep(0.03, 0.05, (r - CASSINI).abs());
    let encke = smoothstep(0.006, 0.014, (r - 2.2).abs());
    let ringlets = 0.78 + 0.12 * (r * 95.0).sin() + 0.1 * (r * 37.0 + 1.0).sin();
    edges * c_ring * cassini * encke * ringlets
}

fn mix_f(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn shade(s: &Surface, u: &Uniforms, nz: &Noises, _inst: &Instance, _all: &[Instance]) -> Vec3 {
    let r = super::ring_radius(&s.obj);
    let dens = density(r);

    let mut col = ramp(
        &[
            (1.28, hex(0x5e4f42)),
            (1.50, hex(0x8f7a62)),
            (1.62, hex(0xd9c8a6)),
            (1.85, hex(0xf1e6cc)),
            (1.98, hex(0xb59e7d)),
            (2.15, hex(0xcdb894)),
            (2.32, hex(0x8a755d)),
        ],
        r,
    );
    // Grumos que orbitan (más rápido cerca del planeta, como Kepler).
    let ang = s.obj.z.atan2(s.obj.x) - u.time * 0.35 / r.powf(1.5);
    let clump = n3(
        &nz.detail,
        Vec3::new(ang.cos() * 6.0, ang.sin() * 6.0, r * 22.0),
    );
    col *= 0.82 + 0.25 * dens + 0.12 * clump;

    // Iluminación: el disco es fino, se ilumina por ambos lados.
    let illum = 0.25 + 0.75 * s.l.y.abs();
    // Dispersión hacia delante cuando miramos contra la luz.
    let forward = (-s.v.dot(&s.l)).max(0.0).powi(6) * 0.4;

    // Sombra del planeta: ¿el rayo hacia la luz intersecta la esfera unitaria?
    let q =
        Vec3::new(s.obj.x, 0.0, s.obj.z) / crate::scene::SPHERE_RADIUS * crate::scene::RING_SCALE;
    let b = q.dot(&s.l);
    let c = q.dot(&q) - 1.0;
    let disc = b * b - c;
    let shadow = if b < 0.0 {
        1.0 - smoothstep(-0.02, 0.06, disc) * 0.92
    } else {
        1.0
    };

    col * (illum * shadow + forward) * (0.55 + 0.45 * dens)
}
