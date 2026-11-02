//! Generadores de ruido compartidos por todos los shaders.
//! Todos usan frecuencia 1.0: cada shader escala las coordenadas por su cuenta,
//! así la frecuencia de cada patrón queda visible como parámetro en el shader.

use fastnoise_lite::{
    CellularDistanceFunction, CellularReturnType, FastNoiseLite, FractalType, NoiseType,
};
use nalgebra_glm::Vec3;

pub struct Noises {
    /// OpenSimplex2 FBm de 6 octavas: relieve / continentes.
    pub terrain: FastNoiseLite,
    /// OpenSimplex2 FBm de 5 octavas: nubes y turbulencia.
    pub clouds: FastNoiseLite,
    /// OpenSimplex2 FBm de 3 octavas: distorsión de dominio (domain warping).
    pub warp: FastNoiseLite,
    /// OpenSimplex2 FBm de 3 octavas: detalles finos (humedad, ciudades...).
    pub detail: FastNoiseLite,
    /// OpenSimplex2 sin fractal: patrones suaves de baja frecuencia.
    pub simplex: FastNoiseLite,
    /// Celular, distancia F2 - F1: ~0 en los bordes de cada celda (grietas, granulación).
    pub cell_edge: FastNoiseLite,
    /// Celular, distancia F1: distancia al centro de la celda (cráteres).
    pub cell_dist: FastNoiseLite,
    /// Celular, valor aleatorio por celda (biomas, máscara de cráteres).
    pub cell_value: FastNoiseLite,
}

fn simplex(seed: i32, octaves: i32) -> FastNoiseLite {
    let mut n = FastNoiseLite::with_seed(seed);
    n.set_noise_type(Some(NoiseType::OpenSimplex2));
    n.set_frequency(Some(1.0));
    if octaves > 1 {
        n.set_fractal_type(Some(FractalType::FBm));
        n.set_fractal_octaves(Some(octaves));
        n.set_fractal_lacunarity(Some(2.0));
        n.set_fractal_gain(Some(0.5));
    }
    n
}

fn cellular(seed: i32, ret: CellularReturnType) -> FastNoiseLite {
    let mut n = FastNoiseLite::with_seed(seed);
    n.set_noise_type(Some(NoiseType::Cellular));
    n.set_frequency(Some(1.0));
    n.set_cellular_distance_function(Some(CellularDistanceFunction::Euclidean));
    n.set_cellular_return_type(Some(ret));
    n.set_cellular_jitter(Some(1.0));
    n
}

impl Noises {
    pub fn new() -> Self {
        Noises {
            terrain: simplex(1337, 6),
            clouds: simplex(4242, 5),
            warp: simplex(777, 3),
            detail: simplex(2024, 3),
            simplex: simplex(99, 1),
            cell_edge: cellular(31, CellularReturnType::Distance2Sub),
            cell_dist: cellular(57, CellularReturnType::Distance),
            cell_value: cellular(57, CellularReturnType::CellValue),
        }
    }
}

/// Muestra un generador en un punto 3D. Devuelve ~[-1, 1].
#[inline]
pub fn n3(n: &FastNoiseLite, p: Vec3) -> f32 {
    n.get_noise_3d(p.x, p.y, p.z)
}

/// Igual que `n3` pero remapeado a [0, 1].
#[inline]
pub fn n01(n: &FastNoiseLite, p: Vec3) -> f32 {
    (n.get_noise_3d(p.x, p.y, p.z) * 0.5 + 0.5).clamp(0.0, 1.0)
}

/// Vector de distorsión 3D (domain warping) a partir de un solo generador.
#[inline]
pub fn warp3(n: &FastNoiseLite, p: Vec3) -> Vec3 {
    Vec3::new(
        n3(n, p),
        n3(n, p + Vec3::new(31.7, 11.3, -7.1)),
        n3(n, p + Vec3::new(-19.1, 47.9, 23.3)),
    )
}
