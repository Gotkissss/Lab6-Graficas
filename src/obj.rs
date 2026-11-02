//! Carga del modelo OBJ (la esfera proporcionada). Solo se leen posiciones y
//! normales; la misma geometría se reutiliza para TODOS los cuerpos.

use crate::vertex::Vertex;
use nalgebra_glm::Vec3;

pub struct Obj {
    pub vertices: Vec<Vertex>,
}

impl Obj {
    pub fn load(path: &str) -> Result<Self, tobj::LoadError> {
        let (models, _) = tobj::load_obj(
            path,
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
        )?;

        let mut vertices = Vec::new();
        for model in models {
            let mesh = &model.mesh;
            for &i in &mesh.indices {
                let i = i as usize;
                let position = Vec3::new(
                    mesh.positions[3 * i],
                    mesh.positions[3 * i + 1],
                    mesh.positions[3 * i + 2],
                );
                let normal = if mesh.normals.len() >= 3 * i + 3 {
                    Vec3::new(
                        mesh.normals[3 * i],
                        mesh.normals[3 * i + 1],
                        mesh.normals[3 * i + 2],
                    )
                } else {
                    position.normalize()
                };
                vertices.push(Vertex { position, normal });
            }
        }
        Ok(Obj { vertices })
    }
}
