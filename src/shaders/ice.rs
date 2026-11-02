//! GLACIES — mundo de hielo (planeta extra 2), con PARALLAX dependiente de la vista.
//!
//! Capas de color:
//!   1. Campos de nieve (blanco mate).
//!   2. Glaciares azul hielo.
//!   3. Mares congelados turquesa con placas agrietadas.
//!   4. Grietas profundas (crevasses) azul oscuro.
//!   5. Auroras animadas en los polos (más visibles de noche).
//! Efectos: parallax (las grietas se ven "hundidas" y se desplazan al mover la cámara),
//! respuesta distinta a la luz (hielo/mar brillante con especular vs. nieve mate).

use super::common::*;
use super::Surface;
use crate::noise::*;
use crate::render::Uniforms;
use nalgebra_glm::Vec3;

const CREVASSE_FREQ: f32 = 3.5; // densidad de las grietas
const PARALLAX_STEPS: usize = 10; // pasos del "steep parallax"
const AURORA_LAT: f32 = 0.80; // latitud (seno) de la aurora

/// Campo de altura procedural: 1 = superficie, 0 = fondo de la grieta.
fn height(q: Vec3, nz: &Noises) -> f32 {
    let w = warp3(&nz.warp, q * 1.5) * 0.35;
    let ridge = n3(&nz.simplex, q * CREVASSE_FREQ + w).abs(); // 0 sobre la línea de la grieta
    let crev = smoothstep(0.0, 0.09, ridge);
    let plates = n3(&nz.cell_edge, q * 3.0 + w) + 1.0;
    let plate_crack = 0.4 + 0.6 * smoothstep(0.0, 0.05, plates);
    (crev * plate_crack) * 0.85 + n01(&nz.detail, q * 6.0) * 0.15
}

pub fn shade(s: &Surface, u: &Uniforms, nz: &Noises) -> Vec3 {
    let t = u.time;
    let prm = &u.params;
    let p = s.p;
    let v = s.v;
    let rot = |q: Vec3| rot_y(q, -t * 0.03);

    // ---- Parallax: marcha a lo largo del rayo de vista dentro de la capa de hielo ----
    // Avanzamos capa por capa hasta que el rayo queda "debajo" del campo de altura.
    let mut q = p;
    if prm.parallax {
        let ndv = p.dot(&v).max(0.2);
        let vt = v - p * p.dot(&v); // componente tangente de la vista
        let step_dir = -vt / ndv * prm.parallax_scale; // desplazamiento total a profundidad 1
        let dz = 1.0 / PARALLAX_STEPS as f32;
        let mut layer = 0.0;
        for _ in 0..PARALLAX_STEPS {
            let cand = (p + step_dir * layer).normalize();
            if 1.0 - height(rot(cand), nz) <= layer {
                break;
            }
            layer += dz;
        }
        q = (p + step_dir * layer).normalize();
    }

    let qs = rot(q);
    let h = height(qs, nz);
    let cavity = 1.0 - h;

    // ---- Capas 1-3 ----
    let snow = smoothstep(-0.05, 0.25, n3(&nz.terrain, qs * 2.2));
    let sea = smoothstep(
        0.15,
        0.25,
        -n3(&nz.terrain, qs * 1.1 + Vec3::new(7.0, 0.0, 0.0)),
    );
    let mut col = mix(hex(0x9fd3f2), hex(0xf7fbff), snow); // glaciar -> nieve
    col = mix(col, hex(0x5fcfc8), sea * 0.85); // mar congelado
                                               // ---- Capa 4: grietas profundas ----
    col = mix(
        col,
        mix(hex(0x2e7fc4), hex(0x06224a), smoothstep(0.3, 0.9, cavity)),
        smoothstep(0.15, 0.55, cavity),
    );

    // ---- Iluminación: nieve mate, hielo y mar brillantes ----
    let n = if prm.bump {
        bump_normal(p, 0.007 * prm.bump_strength, 0.004, |x| height(rot(x), nz))
    } else {
        p
    };
    let ndl = n.dot(&s.l).max(0.0);
    let shiny = (1.0 - snow) * 0.6 + sea * 0.4;
    let spec = blinn(n, s.l, v, 70.0) * shiny * 0.7;
    let mut out = col * (0.03 + 0.97 * ndl) * (1.0 - cavity * 0.35) + hex(0xffffff) * spec;

    // ---- Capa 5: auroras ----
    let lat = qs.y.abs();
    let lon = qs.z.atan2(qs.x);
    let wave = 0.04 * (lon * 5.0 + t * 0.8).sin()
        + 0.02 * n3(&nz.detail, qs * 4.0 + Vec3::new(t * 0.2, 0.0, 0.0));
    let band = (-((lat - AURORA_LAT + wave) / 0.05).powi(2)).exp();
    let curtain = 0.5 + 0.5 * (lon * 28.0 + t * 2.5 + n3(&nz.simplex, qs * 6.0) * 5.0).sin();
    let night = 1.0 - smoothstep(-0.2, 0.25, p.dot(&s.l));
    let aurora = mix(
        hex(0x2dff9a),
        hex(0xc54dff),
        smoothstep(-0.03, 0.05, lat - AURORA_LAT + wave),
    );
    out += aurora * band * curtain * (0.25 + 0.9 * night);

    // Atmósfera fría.
    let mu = p.dot(&v).max(0.0);
    out += hex(0x9fdcff) * (1.0 - mu).powi(3) * 0.5 * smoothstep(-0.3, 0.3, p.dot(&s.l));
    out
}
