//! ZEPHYR — gigante gaseoso con bandas "líquidas" y una tormenta que las curva.
//!
//! Capas de color:
//!   1. Familias de bandas (crema, ocre, óxido, chocolate) según la latitud distorsionada.
//!   2. Claro/oscuro dentro de cada banda + estrías finas a lo largo de la longitud.
//!   3. Tormenta: núcleo rojizo + anillo claro que la rodea.
//!   4. Bruma polar azul grisácea.
//!   5. Sombra proyectada por los anillos + bruma del borde.
//! Efectos: material animado (flujo de bandas), vórtice localizado.

use super::common::*;
use super::rings;
use super::Surface;
use crate::noise::*;
use crate::render::Uniforms;
use nalgebra_glm::Vec3;

// ---- Parámetros de apariencia ----
const BAND_COUNT: f32 = 7.0; // número de bandas
const TURBULENCE: f32 = 0.55; // cuánto ondulan los bordes de las bandas
const FLOW_SPEED: f32 = 0.12; // velocidad del flujo de las bandas
const STORM_LAT: f32 = -0.42; // latitud del vórtice (rad)
const STORM_RADIUS: f32 = 0.28; // radio de influencia del vórtice (rad)

/// Flujo zonal: cada banda se desplaza hacia el este u oeste. Usamos la técnica
/// de "flow map" con dos fases que se mezclan, para que el ruido fluya sin
/// estirarse infinitamente con el tiempo.
fn liquid_turbulence(q: Vec3, t: f32, nz: &Noises) -> f32 {
    let speed = (q.y * 11.0).sin() * 0.9 + (q.y * 5.0).cos() * 0.4; // dirección por banda
    let period = 1.0 / FLOW_SPEED;
    let ph1 = (t / period).fract();
    let ph2 = (t / period + 0.5).fract();
    let sample = |ph: f32, off: f32| {
        let r = rot_y(q, speed * ph * 0.9 + off);
        let w = warp3(&nz.warp, Vec3::new(r.x * 2.0, r.y * 7.0, r.z * 2.0));
        n3(
            &nz.clouds,
            Vec3::new(r.x * 3.0, r.y * 10.0, r.z * 3.0) + w * 0.8,
        )
    };
    let wgt = 1.0 - (2.0 * ph1 - 1.0).abs();
    sample(ph1, 0.0) * wgt + sample(ph2, 2.7) * (1.0 - wgt)
}

pub fn shade(s: &Surface, u: &Uniforms, nz: &Noises) -> Vec3 {
    let t = u.time;
    let prm = &u.params;
    let p = s.p;
    let ps = rot_y(p, -t * 0.05);

    // ---- Vórtice: rotación local alrededor del centro de la tormenta ----
    let center = from_lat_lon(STORM_LAT, 1.7);
    let d = ps.dot(&center).clamp(-1.0, 1.0).acos(); // distancia angular
    let fall = (-(d / STORM_RADIUS).powi(2)).exp();
    let mut q = ps;
    if prm.storm {
        // Giro máximo en el centro y nulo lejos: las bandas se curvan en espiral.
        let swirl = prm.storm_strength * fall + t * 0.6 * fall * fall;
        q = rot_axis(ps, center, swirl);
    }

    // ---- Capas 1 y 2: bandas ----
    let turb = liquid_turbulence(q, t, nz);
    let lat = q.y
        + turb * TURBULENCE * 0.12
        + n3(&nz.simplex, q.component_mul(&Vec3::new(1.5, 3.0, 1.5))) * 0.04;
    let b = lat * BAND_COUNT;
    let family = (b * 0.55 + 1.3).sin() * 0.5 + 0.5; // tipo de banda
    let shade_in = (b * std::f32::consts::PI).sin() * 0.5 + 0.5; // claro/oscuro
    let mut col = ramp(
        &[
            (0.00, hex(0x4e2c1f)),
            (0.30, hex(0x9c4f2e)),
            (0.55, hex(0xcf9a62)),
            (0.80, hex(0xeedcb6)),
            (1.00, hex(0xf7efdc)),
        ],
        family * 0.75 + shade_in * 0.25,
    );
    let streaks = n3(&nz.detail, Vec3::new(q.x * 4.0, q.y * 60.0, q.z * 4.0));
    col *= 0.9 + 0.12 * streaks + 0.1 * turb;

    // ---- Capa 3: tormenta ----
    if prm.storm {
        let core = 1.0 - smoothstep(STORM_RADIUS * 0.3, STORM_RADIUS * 0.75, d);
        let eye = mix(
            hex(0xd8552e),
            hex(0x7a2416),
            smoothstep(0.0, STORM_RADIUS * 0.6, d) * 0.6 + turb * 0.3,
        );
        let halo = (-((d - STORM_RADIUS * 0.85) / (STORM_RADIUS * 0.15)).powi(2)).exp();
        col = mix(col, eye, core * 0.9);
        col = mix(col, hex(0xfff0d0), halo * 0.35);
    }

    // ---- Capa 4: bruma polar ----
    let pole = smoothstep(0.72, 0.95, q.y.abs());
    col = mix(col, hex(0x8a9bab) * (0.85 + 0.2 * turb), pole * 0.8);

    // ---- Iluminación ----
    let ndl = p.dot(&s.l);
    let diff = smoothstep(-0.08, 0.6, ndl) * 0.85 + ndl.max(0.0) * 0.15;
    // Sombra de los anillos: rayo hacia la luz contra el plano ecuatorial (y = 0).
    let mut shadow = 1.0;
    if s.l.y.abs() > 1e-4 {
        let tr = -p.y / s.l.y;
        if tr > 0.0 {
            let hit = p + s.l * tr;
            let r = (hit.x * hit.x + hit.z * hit.z).sqrt();
            shadow = 1.0 - 0.75 * rings::density(r);
        }
    }
    let mu = p.dot(&s.v).max(0.0);
    col *= 0.03 + 0.97 * diff * shadow;
    col *= 0.6 + 0.4 * mu.powf(0.3); // oscurecimiento de borde

    // ---- Capa 5: bruma del borde ----
    col += hex(0xffcf94) * (1.0 - mu).powi(3) * smoothstep(-0.2, 0.4, ndl) * 0.35;
    col
}
