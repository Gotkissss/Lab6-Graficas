//! TERRA NOVA — planeta rocoso con océanos, biomas, hielo, nubes y ciudades.
//!
//! Capas de color:
//!   1. Océano: profundo / poco profundo según la altura bajo el nivel del mar.
//!   2. Biomas: playa, pasto, bosque, desierto (humedad + latitud), roca, nieve.
//!   3. Casquetes polares (latitud + ruido).
//!   4. Luces de ciudades (solo en el lado nocturno).
//!   5. Nubes (máscara FBm propia que rota más rápido que el terreno).
//!   6. Atmósfera (resplandor de Fresnel en el borde).
//! Efectos: bump mapping, océano especular vs. continente mate, sombras de nubes.

use super::common::*;
use super::Surface;
use crate::noise::*;
use crate::render::Uniforms;
use nalgebra_glm::Vec3;

// ---- Parámetros de apariencia ----
const SEA_LEVEL: f32 = -0.06; // subirlo inunda continentes
const CONTINENT_FREQ: f32 = 1.3; // menor = continentes más grandes
const SURFACE_SPIN: f32 = 0.05; // rad/s de rotación del terreno
const CLOUD_SPIN: f32 = 0.11; // rad/s de las nubes (distinto al terreno)
const CLOUD_HEIGHT: f32 = 0.06; // altura de la capa de nubes (en radios): separa las sombras
const OCEAN_SHININESS: f32 = 90.0; // tamaño del brillo especular del océano

/// Campo de altura procedural: continentes con domain warping + cordilleras.
fn height(q: Vec3, nz: &Noises) -> f32 {
    let w = warp3(&nz.warp, q * 1.2);
    let base = n3(&nz.terrain, q * CONTINENT_FREQ + w * 0.45) * 1.25 + 0.05;
    // Cordilleras "ridged": 1 - |ruido|, solo tierra adentro.
    let ridge = 1.0 - n3(&nz.detail, q * 4.0 + w).abs();
    base + ridge * ridge * 0.25 * smoothstep(0.05, 0.35, base)
}

/// Densidad de nubes (0..1) en una dirección de la esfera de nubes.
pub fn cloud_density(q: Vec3, t: f32, coverage: f32, nz: &Noises) -> f32 {
    let qc = rot_y(q, -t * CLOUD_SPIN);
    let w = warp3(&nz.warp, qc * 1.8 + Vec3::new(0.0, 0.0, t * 0.02));
    // Estirado en latitud: nubes alargadas como frentes y bandas.
    let c = n01(
        &nz.clouds,
        Vec3::new(qc.x * 2.4, qc.y * 4.0, qc.z * 2.4) + w * 0.7,
    );
    let thr = 0.78 - coverage * 0.45; // cobertura: baja el umbral
    smoothstep(thr, thr + 0.16, c)
}

pub fn shade(s: &Surface, u: &Uniforms, nz: &Noises) -> Vec3 {
    let t = u.time;
    let prm = &u.params;
    let (p, l, v) = (s.p, s.l, s.v);
    let surf = |q: Vec3| rot_y(q, -t * SURFACE_SPIN);

    let ps = surf(p);
    let h = height(ps, nz);
    let elev = h - SEA_LEVEL;
    let lat = ps.y.abs();
    let is_land = elev > 0.0;

    // ---- Capa 3: hielo polar ----
    let ice = smoothstep(0.80, 0.86, lat + 0.07 * n3(&nz.detail, ps * 5.0));

    // ---- Capas 1 y 2: océano o biomas ----
    let base;
    if is_land {
        let moist = n3(&nz.detail, ps * 2.2 + Vec3::new(5.0, 1.0, 3.0));
        let desert = smoothstep(0.05, 0.3, 0.25 - (lat - 0.3).abs() - moist * 0.6);
        let mut c = mix(hex(0x5f8f3a), hex(0x1f4f2a), smoothstep(-0.2, 0.35, moist)); // pasto -> bosque
        c = mix(c, hex(0xc9a35c), desert); // desierto
        c = mix(hex(0xd8c58e), c, smoothstep(0.0, 0.035, elev)); // playa
        c = mix(c, hex(0x6b5a4b), smoothstep(0.32, 0.48, elev)); // roca
        c = mix(c, hex(0xf2f2ee), smoothstep(0.62, 0.7, elev)); // nieve de cumbres
        base = c * (0.88 + 0.24 * n01(&nz.detail, ps * 22.0));
    } else {
        let depth = -elev;
        let waves = n3(&nz.detail, ps * 30.0 + Vec3::new(t * 0.3, 0.0, 0.0)) * 0.03;
        base = mix(
            hex(0x2aa7b8),
            hex(0x0a2552),
            smoothstep(0.0, 0.35, depth + waves),
        )
        .component_mul(&mix(
            hex(0xffffff),
            hex(0x8fb0ff),
            smoothstep(0.3, 0.9, depth),
        ));
    }
    let albedo = mix(base, hex(0xeaf4ff), ice);

    // ---- Normal con bump mapping (solo tierra firme) ----
    let mut n = p;
    if is_land && prm.bump {
        n = bump_normal(p, 0.02 * prm.bump_strength, 0.004, |q| {
            height(surf(q), nz).max(SEA_LEVEL)
        });
    }

    // ---- Iluminación diferente por región ----
    let ndl = n.dot(&l).max(0.0);
    let geo = p.dot(&l);
    let day = smoothstep(-0.12, 0.18, geo);

    // Sombras de nubes: el punto donde el rayo HACIA LA LUZ cruza la capa de nubes.
    let mut shadow = 1.0;
    if prm.clouds && prm.cloud_shadows && geo > -0.25 {
        let tl = ray_sphere_exit(p, l, 1.0 + CLOUD_HEIGHT);
        let qs = (p + l * tl).normalize();
        shadow = 1.0 - 0.8 * cloud_density(qs, t, prm.cloud_coverage, nz);
    }

    let mut col = albedo * (0.025 + 0.975 * ndl * shadow);
    if !is_land || ice > 0.5 {
        // Océano / hielo: reflejo especular + reflejo del cielo (Fresnel).
        let spec_strength = if is_land { 0.35 } else { 1.0 };
        let shin = if is_land { 30.0 } else { OCEAN_SHININESS };
        let spec = blinn(p, l, v, shin) * spec_strength * shadow * smoothstep(0.0, 0.1, geo);
        let fres = (1.0 - p.dot(&v).max(0.0)).powi(5);
        col += hex(0xfff4dc) * spec + hex(0x4f87c8) * fres * day * 0.35;
    }
    // (La tierra firme se queda mate: solo difuso, sin término especular.)

    // ---- Capa 4: luces de ciudades en el lado nocturno ----
    if prm.night_lights && is_land && ice < 0.3 && elev < 0.3 {
        let cluster = smoothstep(-0.1, 0.35, n3(&nz.simplex, ps * 3.5));
        let city = smoothstep(0.25, 0.65, n3(&nz.detail, ps * 18.0)) * cluster;
        let night = 1.0 - day;
        col += hex(0xffc46b) * city * night * 3.0;
    }

    // ---- Capa 5: nubes (se ven donde el rayo de VISTA cruza la capa) ----
    if prm.clouds {
        let tv = ray_sphere_exit(p, v, 1.0 + CLOUD_HEIGHT);
        let qv = (p + v * tv).normalize();
        let dens = cloud_density(qv, t, prm.cloud_coverage, nz);
        let lit = smoothstep(-0.15, 0.35, qv.dot(&l));
        let cloud_col = mix(hex(0x0b0e16), mix(hex(0xb7c0cf), hex(0xffffff), dens), lit);
        col = mix(col, cloud_col, dens * 0.95);
    }

    // ---- Capa 6: atmósfera ----
    let mu = p.dot(&v).max(0.0);
    let rim = (1.0 - mu).powf(2.5);
    col += hex(0x4d9bff) * rim * (0.08 + 0.7 * day);
    col
}
