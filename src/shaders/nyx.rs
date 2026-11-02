//! NYX — mundo alienígena bioluminiscente (planeta extra 3).
//!
//! Capas de color:
//!   1. Mares violeta (profundo / poco profundo).
//!   2. Biomas por celda Voronoi: selva turquesa, llanuras de hongos lima,
//!      desiertos de esporas ámbar y arrecifes carmesí.
//!   3. Ríos de luz en los bordes entre biomas.
//!   4. Bioluminiscencia pulsante en el lado nocturno (ondas que viajan).
//!   5. Bruma violeta de la atmósfera.
//! Efectos: día/noche, respuesta distinta a la luz (mar especular rosado, tierra con luz envolvente).

use super::common::*;
use super::Surface;
use crate::noise::*;
use crate::render::Uniforms;
use nalgebra_glm::Vec3;

const BIOME_FREQ: f32 = 3.2; // tamaño de los biomas
const RIVER_WIDTH: f32 = 0.03; // grosor de los ríos de luz
const GLOW_SPEED: f32 = 3.0; // velocidad de las ondas bioluminiscentes

pub fn shade(s: &Surface, u: &Uniforms, nz: &Noises) -> Vec3 {
    let t = u.time;
    let p = s.p;
    let ps = rot_y(p, -t * 0.04);
    let w = warp3(&nz.warp, ps * 1.4) * 0.3;

    let h = n3(&nz.terrain, ps * 1.8 + w);
    let is_sea = h < -0.08;
    let bq = ps * BIOME_FREQ + w;
    let biome = n3(&nz.cell_value, bq);
    let edge = n3(&nz.cell_edge, bq) + 1.0;

    let (col, glow_mask) = if is_sea {
        (
            mix(hex(0x55307a), hex(0x140a2e), smoothstep(-0.08, -0.45, h)),
            0.0,
        )
    } else {
        let c = ramp(
            &[
                (-1.0, hex(0x0e4f4a)),
                (-0.35, hex(0x16695e)),
                (-0.1, hex(0x6fae4a)),
                (0.2, hex(0x86b84e)),
                (0.35, hex(0xc08a43)),
                (0.6, hex(0xc99a55)),
                (0.75, hex(0x8e2d4a)),
                (1.0, hex(0x6e2140)),
            ],
            biome,
        );
        let tex = 0.75 + 0.45 * n01(&nz.detail, ps * 20.0);
        // Variación dentro de cada bioma para que no sean planos.
        let c = mix(
            c,
            c.component_mul(&hex(0xb0d8c8)),
            n01(&nz.terrain, ps * 6.0) * 0.5,
        );
        let river = 1.0 - smoothstep(0.0, RIVER_WIDTH, edge);
        let jungle = 1.0 - smoothstep(-0.4, -0.2, biome);
        let spots = smoothstep(0.35, 0.6, n3(&nz.detail, ps * 35.0)) * jungle;
        (
            mix(c * tex, hex(0x1f9e98), river * 0.6),
            river + spots * 0.7,
        )
    };

    let ndl = p.dot(&s.l);
    let day = smoothstep(-0.1, 0.25, ndl);
    let mut out = if is_sea {
        let spec = blinn(p, s.l, s.v, 60.0) * smoothstep(0.0, 0.1, ndl);
        col * (0.03 + 0.97 * ndl.max(0.0)) + hex(0xff9ad8) * spec * 0.8
    } else {
        // Luz envolvente ("wrap lighting"): la vegetación dispersa la luz.
        col * (0.03 + 0.97 * ((ndl + 0.3) / 1.3).max(0.0))
    };

    // Bioluminiscencia: ondas concéntricas que recorren los ríos y la selva.
    if u.params.night_lights {
        let wave = 0.55 + 0.45 * (t * GLOW_SPEED - n01(&nz.simplex, ps * 2.0) * 18.0).sin();
        out += hex(0x3ff5e0) * glow_mask * wave * (1.0 - day) * 1.2;
    }

    let mu = p.dot(&s.v).max(0.0);
    out += hex(0xb78cff) * (1.0 - mu).powf(2.5) * (0.1 + 0.6 * day);
    out
}
