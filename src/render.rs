//! Pipeline del software renderer:
//!   1. vertex shader (por vértice)          -> shaders::vertex_shader
//!   2. ensamblado de triángulos + rasterización con coordenadas baricéntricas
//!   3. prueba de profundidad (z-buffer)      -> el fragmento más cercano se guarda
//!   4. fragment shader (por píxel visible)   -> shaders::fragment_shader
//!   5. fondo de estrellas + halo de la estrella

use crate::framebuffer::{to_u32, Fragment, Framebuffer, NO_INSTANCE};
use crate::noise::Noises;
use crate::scene::{Instance, ShaderKind};
use crate::shaders::{self, common::*};
use crate::vertex::{TransformedVertex, Vertex};
use nalgebra_glm::{Mat4, Vec3, Vec4};
use rayon::prelude::*;

/// Parámetros ajustables en tiempo real (teclado) que leen los shaders.
#[derive(Clone, Copy)]
pub struct Params {
    pub cloud_coverage: f32,
    pub bump_strength: f32,
    pub parallax_scale: f32,
    pub storm_strength: f32,
    pub clouds: bool,
    pub cloud_shadows: bool,
    pub bump: bool,
    pub parallax: bool,
    pub storm: bool,
    pub night_lights: bool,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            cloud_coverage: 0.5,
            bump_strength: 1.0,
            parallax_scale: 0.05,
            storm_strength: 4.0,
            clouds: true,
            cloud_shadows: true,
            bump: true,
            parallax: true,
            storm: true,
            night_lights: true,
        }
    }
}

pub struct Uniforms {
    pub view: Mat4,
    pub projection: Mat4,
    pub width: f32,
    pub height: f32,
    pub time: f32,
    pub cam_pos: Vec3,
    pub light_pos: Vec3,
    pub params: Params,
}

fn transform(v: &Vertex, inst: &Instance, u: &Uniforms) -> TransformedVertex {
    let local = shaders::vertex_shader(v, inst);
    let world4 = inst.model * Vec4::new(local.x, local.y, local.z, 1.0);
    let clip = u.projection * u.view * world4;
    let inv_w = 1.0 / clip.w;
    let ndc = Vec3::new(clip.x * inv_w, clip.y * inv_w, clip.z * inv_w);
    TransformedVertex {
        screen: Vec3::new(
            (ndc.x + 1.0) * 0.5 * u.width,
            (1.0 - ndc.y) * 0.5 * u.height,
            ndc.z,
        ),
        inv_w,
        world: world4.xyz(),
        obj: v.position,
    }
}

#[inline]
fn edge(a: &Vec3, b: &Vec3, px: f32, py: f32) -> f32 {
    (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x)
}

fn rasterize(fb: &mut Framebuffer, t: [TransformedVertex; 3], id: u16, inst: &Instance) {
    let (a, b, c) = (&t[0].screen, &t[1].screen, &t[2].screen);
    if t.iter().any(|v| v.inv_w <= 0.0) {
        return; // detrás de la cámara
    }
    let area = edge(a, b, c.x, c.y);
    if area.abs() < 1e-8 {
        return;
    }
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as i32;
    let max_x = a.x.max(b.x).max(c.x).ceil().min(fb.width as f32 - 1.0) as i32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as i32;
    let max_y = a.y.max(b.y).max(c.y).ceil().min(fb.height as f32 - 1.0) as i32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let w0 = edge(b, c, px, py) / area;
            let w1 = edge(c, a, px, py) / area;
            let w2 = edge(a, b, px, py) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let depth = w0 * a.z + w1 * b.z + w2 * c.z;
            let idx = y as usize * fb.width + x as usize;
            if depth >= fb.fragments[idx].depth {
                continue;
            }
            // Interpolación con corrección de perspectiva.
            let p0 = w0 * t[0].inv_w;
            let p1 = w1 * t[1].inv_w;
            let p2 = w2 * t[2].inv_w;
            let s = 1.0 / (p0 + p1 + p2);
            let obj = (t[0].obj * p0 + t[1].obj * p1 + t[2].obj * p2) * s;
            if shaders::discard(inst, &obj) {
                continue;
            }
            let world = (t[0].world * p0 + t[1].world * p1 + t[2].world * p2) * s;
            fb.fragments[idx] = Fragment {
                depth,
                world,
                obj,
                instance: id,
            };
        }
    }
}

pub fn render(
    fb: &mut Framebuffer,
    sphere: &[Vertex],
    instances: &[Instance],
    u: &Uniforms,
    noise: &Noises,
) {
    fb.clear();

    // Etapas 1-3: vertex shader + rasterización + z-buffer.
    for (id, inst) in instances.iter().enumerate() {
        let transformed: Vec<TransformedVertex> =
            sphere.iter().map(|v| transform(v, inst, u)).collect();
        for tri in transformed.chunks_exact(3) {
            rasterize(fb, [tri[0], tri[1], tri[2]], id as u16, inst);
        }
    }

    // Datos de pantalla de la(s) estrella(s) para el halo.
    let halos: Vec<(f32, f32, f32)> = instances
        .iter()
        .filter(|i| i.kind == ShaderKind::Star)
        .map(|i| screen_circle(i, u))
        .collect();

    // Etapas 4-5: fragment shader en paralelo, una vez por píxel.
    let w = fb.width;
    let frags = &fb.fragments;
    fb.color.par_iter_mut().enumerate().for_each(|(i, out)| {
        let f = &frags[i];
        let (x, y) = ((i % w) as f32, (i / w) as f32);
        let color = if f.instance == NO_INSTANCE {
            background(x, y, &halos, u.time)
        } else {
            let inst = &instances[f.instance as usize];
            shaders::fragment_shader(f, inst, instances, u, noise)
        };
        *out = to_u32(color);
    });
}

/// Centro y radio aproximado en pantalla de una esfera.
fn screen_circle(inst: &Instance, u: &Uniforms) -> (f32, f32, f32) {
    let to_screen = |p: Vec3| {
        let c = u.projection * u.view * Vec4::new(p.x, p.y, p.z, 1.0);
        Vec3::new(
            (c.x / c.w + 1.0) * 0.5 * u.width,
            (1.0 - c.y / c.w) * 0.5 * u.height,
            0.0,
        )
    };
    let view_inv = u.view.try_inverse().unwrap_or(Mat4::identity());
    let right = (view_inv * Vec4::new(1.0, 0.0, 0.0, 0.0)).xyz();
    let c = to_screen(inst.center);
    let e = to_screen(inst.center + right * inst.radius);
    (c.x, c.y, (e - c).norm())
}

/// Fondo: estrellas lejanas pseudoaleatorias + halo/corona de la estrella.
fn background(x: f32, y: f32, halos: &[(f32, f32, f32)], time: f32) -> Vec3 {
    let mut col = hex(0x03040a);
    let h = hash2(x, y);
    if h > 0.9975 {
        let tw = 0.6 + 0.4 * (time * 2.0 + h * 500.0).sin();
        let tint = mix(hex(0xaac4ff), hex(0xfff1d6), hash2(y, x));
        col += tint * ((h - 0.9975) / 0.0025) * tw;
    }
    for &(cx, cy, r) in halos {
        let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / r.max(1.0);
        if d > 1.0 {
            let ang = (y - cy).atan2(x - cx);
            let rays =
                0.85 + 0.15 * (ang * 9.0 + time * 0.6).sin() * (ang * 5.0 - time * 0.4).cos();
            let glow = (-(d - 1.0) * 3.2).exp() * rays;
            col += hex(0xff8a2a) * glow * 0.9 + hex(0xffe2a0) * (-(d - 1.0) * 9.0).exp() * 0.6;
        }
    }
    col
}
