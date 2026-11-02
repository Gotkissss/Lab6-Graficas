//! HELIOS — estrella. Emisiva: NO usa la luz externa.
//!
//! Capas de color:
//!   1. Temperatura base: FBm con domain warping que "hierve" con el tiempo.
//!   2. Granulación: ruido celular (F2-F1) -> celdas brillantes y bordes oscuros.
//!   3. Manchas solares: umbra (casi negra) + penumbra filamentosa.
//!   4. Fáculas: anillos más calientes alrededor de las manchas.
//!   5. Oscurecimiento de borde (limb darkening) + borde rojizo.

use super::common::*;
use super::Surface;
use crate::noise::*;
use crate::render::Uniforms;
use nalgebra_glm::Vec3;

// ---- Parámetros de apariencia ----
const BOIL_SPEED: f32 = 0.08; // velocidad del hervor de la temperatura base
const GRANULE_FREQ: f32 = 18.0; // tamaño de la granulación (mayor = celdas más pequeñas)
const SPOT_THRESHOLD: f32 = 0.70; // umbral de las manchas (menor = más manchas)
const LIMB_POWER: f32 = 0.45; // intensidad del oscurecimiento de borde

pub fn shade(s: &Surface, u: &Uniforms, nz: &Noises) -> Vec3 {
    let t = u.time;
    let p = s.p;

    // 1. Temperatura base con domain warping animado.
    let w = warp3(&nz.warp, p * 1.6 + Vec3::new(t * 0.05, 0.0, -t * 0.04));
    let temp = n01(
        &nz.clouds,
        p * 2.6 + w * 0.9 + Vec3::new(0.0, t * BOIL_SPEED, t * 0.03),
    );

    // 2. Granulación (celdas de convección) que se desplazan lentamente.
    let g = n3(&nz.cell_edge, rot_y(p, t * 0.04) * GRANULE_FREQ + w * 0.6);
    let granule = smoothstep(-1.0, -0.75, g); // 0 en los bordes, 1 en el centro de la celda

    // 3. Manchas solares (ruido de baja frecuencia + filamentos de la penumbra).
    let sp = rot_y(p, t * 0.02);
    let spot = n01(&nz.simplex, sp * 2.3) * 0.85 + n01(&nz.detail, sp * 7.0) * 0.15;
    let umbra = smoothstep(SPOT_THRESHOLD + 0.12, SPOT_THRESHOLD + 0.17, spot);
    let filaments = 0.6 + 0.4 * (n3(&nz.detail, sp * 40.0) * 6.0).sin();
    let penumbra = smoothstep(SPOT_THRESHOLD + 0.04, SPOT_THRESHOLD + 0.11, spot);

    // 4. Fáculas: banda caliente justo antes de la penumbra.
    let faculae = smoothstep(SPOT_THRESHOLD - 0.06, SPOT_THRESHOLD + 0.02, spot) * (1.0 - penumbra);

    let mut heat = 0.30 + temp * 0.45 + granule * 0.14;
    heat += faculae * 0.18;
    heat -= penumbra * (0.28 + 0.1 * filaments);
    heat -= umbra * 0.45;

    let mut col = ramp(
        &[
            (0.00, hex(0x1a0300)),
            (0.25, hex(0x6e0f00)),
            (0.42, hex(0xc42800)),
            (0.58, hex(0xff6a00)),
            (0.74, hex(0xffb22e)),
            (0.88, hex(0xffe9a0)),
            (1.00, hex(0xfffbe8)),
        ],
        heat,
    );

    // 5. Limb darkening: el borde es más oscuro y rojizo.
    let mu = p.dot(&s.v).max(0.0);
    col = mix(col.component_mul(&hex(0xff5a1a)), col, mu.powf(0.35));
    col *= 0.35 + 0.75 * mu.powf(LIMB_POWER);
    col * 1.1
}
