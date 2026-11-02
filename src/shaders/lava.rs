//! IGNIS — mundo de lava (planeta extra 1).
//!
//! Capas de color:
//!   1. Corteza de basalto (FBm oscuro con relieve).
//!   2. Grietas incandescentes (bordes de celdas Voronoi con domain warping).
//!   3. Lagos de lava fundida con turbulencia que fluye.
//!   4. Depósitos de azufre amarillo alrededor de las grietas.
//!   5. Bruma de humo rojizo en el borde.
//! Efectos: material animado (pulso del calor), día/noche (la lava domina de noche),
//! respuesta distinta a la luz (corteza difusa con bump vs. lava emisiva).

use super::common::*;
use super::Surface;
use crate::noise::*;
use crate::render::Uniforms;
use nalgebra_glm::Vec3;

const CRACK_FREQ: f32 = 4.5; // densidad de las placas
const CRACK_WIDTH: f32 = 0.09; // grosor de las grietas
const LAKE_THRESHOLD: f32 = 0.70; // mayor = menos lagos
const PULSE_SPEED: f32 = 2.2; // velocidad del pulso de calor

pub fn shade(s: &Surface, u: &Uniforms, nz: &Noises) -> Vec3 {
    let t = u.time;
    let p = s.p;
    let rot = |q: Vec3| rot_y(q, -t * 0.03);
    let ps = rot(p);
    let w = warp3(&nz.warp, ps * 2.0);

    // 2. Grietas: distancia al borde de la celda (~0 en el borde).
    let edge_of = |q: Vec3| {
        n3(
            &nz.cell_edge,
            q * CRACK_FREQ + warp3(&nz.warp, q * 2.0) * 0.35,
        ) + 1.0
    };
    let e = edge_of(ps);
    let crack = 1.0 - smoothstep(0.0, CRACK_WIDTH, e);

    // 1. Corteza.
    let crust_n = n01(&nz.terrain, ps * 3.0);
    let mut crust = mix(hex(0x15100e), hex(0x4a3a31), crust_n);
    // 4. Azufre: halo justo fuera de la grieta.
    let sulfur = smoothstep(CRACK_WIDTH, CRACK_WIDTH + 0.06, e)
        * (1.0 - smoothstep(CRACK_WIDTH + 0.08, CRACK_WIDTH + 0.2, e))
        * smoothstep(0.1, 0.4, n3(&nz.detail, ps * 8.0));
    crust = mix(crust, hex(0xb9a032), sulfur * 0.6);

    // 3. Lagos de lava con turbulencia animada.
    let lake = smoothstep(
        LAKE_THRESHOLD,
        LAKE_THRESHOLD + 0.08,
        n01(&nz.simplex, ps * 1.3 + Vec3::new(4.0, 0.0, 0.0)),
    );
    let molten = n01(
        &nz.clouds,
        ps * 5.0 + w * 0.8 + Vec3::new(0.0, t * 0.12, t * 0.05),
    );

    // Pulso de calor (material animado).
    let pulse = 0.72 + 0.28 * (t * PULSE_SPEED + w.x * 8.0).sin();
    let heat = (crack * (0.55 + 0.45 * molten) * pulse).max(lake * (0.45 + 0.55 * molten));
    let lava = ramp(
        &[
            (0.0, hex(0x2a0503)),
            (0.35, hex(0xa11a02)),
            (0.6, hex(0xff6a00)),
            (0.82, hex(0xffcc33)),
            (1.0, hex(0xfff6c8)),
        ],
        heat,
    );

    // Corteza con bump (las placas se levantan, las grietas se hunden).
    let n = if u.params.bump {
        bump_normal(p, 0.04 * u.params.bump_strength, 0.004, |q| {
            let q = rot(q);
            smoothstep(0.0, 0.25, edge_of(q)) * 0.6 + n01(&nz.terrain, q * 3.0) * 0.4
        })
    } else {
        p
    };
    let ndl = n.dot(&s.l).max(0.0);
    let night = 1.0 - smoothstep(-0.15, 0.2, p.dot(&s.l));
    let surface_mask = (1.0 - lake).max(0.0) * (1.0 - crack * 0.8);
    let mut col = crust * (0.03 + 0.95 * ndl) * surface_mask;
    // La lava emite luz propia; de noche brilla todavía más.
    col += lava * heat * (1.0 + 0.6 * night);

    // 5. Bruma de humo.
    let mu = p.dot(&s.v).max(0.0);
    col += hex(0xff4a1a) * (1.0 - mu).powi(3) * 0.45;
    col
}
