//! SELENE — luna del planeta rocoso. Misma esfera, escalada y trasladada.
//!
//! Capas de color:
//!   1. Regolito base (FBm gris cálido).
//!   2. Tierras altas claras vs. "mares" de basalto oscuro.
//!   3. Cráteres en dos escalas (ruido celular F1): fondo oscuro + borde claro.
//!   4. Eyecta/rayos brillantes alrededor de los cráteres grandes.
//! Efecto: bump mapping del campo de altura de cráteres (relieve que responde a la luz).

use super::common::*;
use super::Surface;
use crate::noise::*;
use crate::render::Uniforms;
use nalgebra_glm::Vec3;

const CRATER_FREQ: f32 = 4.5; // cráteres grandes
const SMALL_CRATER_FREQ: f32 = 13.0; // cráteres pequeños
const MARIA_THRESHOLD: f32 = 0.15; // menor = más mares oscuros

/// Perfil de un cráter: cuenco hundido + borde elevado. Devuelve (altura, borde, cuenco).
fn crater(q: Vec3, freq: f32, nz: &Noises) -> (f32, f32, f32) {
    let d = n3(&nz.cell_dist, q * freq) + 1.0; // distancia al centro de la celda
    let cv = n3(&nz.cell_value, q * freq); // valor aleatorio de la celda
    let exists = smoothstep(-0.45, -0.25, cv);
    let r = 0.24 + 0.1 * cv;
    let bowl = 1.0 - smoothstep(0.0, r, d);
    let rim = (-((d - r) / 0.06).powi(2)).exp();
    (
        (rim * 0.5 - bowl * bowl * 0.9) * exists,
        rim * exists,
        bowl * exists,
    )
}

fn height(q: Vec3, nz: &Noises) -> f32 {
    let (big, _, _) = crater(q, CRATER_FREQ, nz);
    let (small, _, _) = crater(q + Vec3::new(3.1, 1.7, 4.2), SMALL_CRATER_FREQ, nz);
    big + small * 0.45 + n3(&nz.detail, q * 9.0) * 0.08
}

pub fn shade(s: &Surface, u: &Uniforms, nz: &Noises) -> Vec3 {
    let p = s.p;

    // 1. Regolito base.
    let mut c = mix(hex(0x7f7b74), hex(0xb9b4aa), n01(&nz.terrain, p * 3.0));
    // 2. Mares de basalto.
    let maria = smoothstep(
        MARIA_THRESHOLD,
        MARIA_THRESHOLD + 0.15,
        n3(&nz.simplex, p * 1.4 + Vec3::new(2.0, 0.0, 0.0)),
    );
    c = mix(c, hex(0x45464c), maria * 0.85);
    // 3. Cráteres.
    let (_, rim, bowl) = crater(p, CRATER_FREQ, nz);
    let (_, rim2, bowl2) = crater(p + Vec3::new(3.1, 1.7, 4.2), SMALL_CRATER_FREQ, nz);
    c = mix(c, hex(0x5d5a54), bowl * 0.7 + bowl2 * 0.4);
    c = mix(c, hex(0xe0dccf), rim * 0.55 + rim2 * 0.3);
    // 4. Rayos de eyecta.
    let rays = smoothstep(
        0.55,
        0.9,
        n01(&nz.detail, p.component_mul(&Vec3::new(18.0, 3.0, 18.0))),
    );
    c = mix(c, hex(0xd6d2c6), rays * 0.25 * (1.0 - maria));

    // Relieve procedural (bump mapping).
    let n = if u.params.bump {
        bump_normal(p, 0.05 * u.params.bump_strength, 0.004, |q| height(q, nz))
    } else {
        p
    };
    let ndl = n.dot(&s.l).max(0.0);
    c * (0.02 + 1.05 * ndl)
}
