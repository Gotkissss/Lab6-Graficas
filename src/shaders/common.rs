//! Funciones utilitarias compartidas por los fragment shaders.

use nalgebra_glm::Vec3;

/// Color a partir de un entero 0xRRGGBB.
pub fn hex(c: u32) -> Vec3 {
    Vec3::new(
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
    )
}

pub fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Rampa de color: interpola entre paradas (posición, color) ordenadas.
pub fn ramp(stops: &[(f32, Vec3)], x: f32) -> Vec3 {
    if x <= stops[0].0 {
        return stops[0].1;
    }
    for w in stops.windows(2) {
        let (a, b) = (w[0], w[1]);
        if x <= b.0 {
            return mix(a.1, b.1, (x - a.0) / (b.0 - a.0));
        }
    }
    stops[stops.len() - 1].1
}

/// Rotación alrededor del eje Y (para girar patrones con el tiempo).
pub fn rot_y(p: Vec3, a: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    Vec3::new(c * p.x + s * p.z, p.y, -s * p.x + c * p.z)
}

/// Rotación de Rodrigues alrededor de un eje unitario arbitrario.
pub fn rot_axis(p: Vec3, k: Vec3, a: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    p * c + k.cross(&p) * s + k * k.dot(&p) * (1.0 - c)
}

/// Dirección unitaria a partir de latitud y longitud (radianes).
pub fn from_lat_lon(lat: f32, lon: f32) -> Vec3 {
    Vec3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin())
}

/// Base tangente (T, B) ortonormal para una normal n.
pub fn tangent_frame(n: Vec3) -> (Vec3, Vec3) {
    let up = if n.y.abs() < 0.99 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let t = up.cross(&n).normalize();
    let b = n.cross(&t);
    (t, b)
}

/// Bump mapping procedural: perturba la normal n de una esfera con el
/// gradiente (diferencias finitas) del campo de altura `h`.
pub fn bump_normal(n: Vec3, strength: f32, eps: f32, h: impl Fn(Vec3) -> f32) -> Vec3 {
    let (t, b) = tangent_frame(n);
    let h0 = h(n);
    let ht = h((n + t * eps).normalize());
    let hb = h((n + b * eps).normalize());
    let dt = (ht - h0) / eps;
    let db = (hb - h0) / eps;
    (n - (t * dt + b * db) * strength).normalize()
}

/// Término especular Blinn-Phong.
pub fn blinn(n: Vec3, l: Vec3, v: Vec3, shininess: f32) -> f32 {
    let h = (l + v).normalize();
    n.dot(&h).max(0.0).powf(shininess)
}

/// Distancia t > 0 a la que un rayo que parte de p (dentro de la esfera de
/// radio r centrada en el origen) sale de esa esfera.
pub fn ray_sphere_exit(p: Vec3, d: Vec3, r: f32) -> f32 {
    let b = p.dot(&d);
    let c = p.dot(&p) - r * r;
    let disc = (b * b - c).max(0.0);
    -b + disc.sqrt()
}

/// Hash 2D barato en [0, 1) (fondo de estrellas).
pub fn hash2(x: f32, y: f32) -> f32 {
    let v = (x * 127.1 + y * 311.7).sin() * 43758.547;
    v - v.floor()
}
