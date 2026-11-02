//! Modo de captura sin ventana: genera las imágenes y los cuadros de los
//! videos del README.
//!   cargo run --release -- --capture capture            (todo)
//!   cargo run --release -- --capture capture stills     (solo imágenes)
//!   cargo run --release -- --capture capture clips      (solo cuadros de video)
//!   cargo run --release -- --capture capture only:anim_helios

use crate::framebuffer::Framebuffer;
use crate::noise::Noises;
use crate::vertex::Vertex;
use crate::{render, scene, State, HEIGHT, WIDTH};
use std::fs;

struct Ctx<'a> {
    dir: String,
    verts: &'a [Vertex],
    noise: &'a Noises,
    fb: Framebuffer,
    /// Si no está vacío, solo se renderiza el clip con este nombre.
    only: String,
}

impl Ctx<'_> {
    fn shot(&mut self, name: &str, s: &State) {
        let inst = scene::build(s.selected, s.time);
        render::render(&mut self.fb, self.verts, &inst, &s.uniforms(), self.noise);
        self.fb.save_png(&format!("{}/{}.png", self.dir, name));
        println!("  {name}.png");
    }

    fn clip(&mut self, name: &str, frames: usize, base: State, f: impl Fn(&mut State, f32)) {
        if !self.only.is_empty() && self.only != name {
            return;
        }
        let d = format!("{}/{}", self.dir, name);
        fs::create_dir_all(&d).unwrap();
        for i in 0..frames {
            let mut s = State {
                ..clone_state(&base)
            };
            f(&mut s, i as f32 / frames as f32);
            let inst = scene::build(s.selected, s.time);
            render::render(&mut self.fb, self.verts, &inst, &s.uniforms(), self.noise);
            self.fb.save_png(&format!("{d}/f{i:04}.png"));
        }
        println!("  {name}/ ({frames} cuadros)");
    }
}

fn clone_state(s: &State) -> State {
    let mut n = State::new(s.selected);
    n.camera.yaw = s.camera.yaw;
    n.camera.pitch = s.camera.pitch;
    n.camera.distance = s.camera.distance;
    n.camera.target = s.camera.target;
    n.time = s.time;
    n.light_angle = s.light_angle;
    n.params = s.params;
    n
}

fn view(sel: usize, time: f32) -> State {
    let mut s = State::new(sel);
    s.time = time;
    s
}

pub fn run(dir: &str, verts: &[Vertex], noise: &Noises) {
    let what = std::env::args().last().unwrap_or_default();
    fs::create_dir_all(dir).unwrap();
    let mut c = Ctx {
        dir: dir.into(),
        verts,
        noise,
        fb: Framebuffer::new(WIDTH, HEIGHT),
        only: String::new(),
    };
    if let Some(name) = what.strip_prefix("only:") {
        c.only = name.to_string();
    }

    if what != "clips" && c.only.is_empty() {
        println!("Capturas:");
        let names = [
            "sistema",
            "helios",
            "terra_nova",
            "zephyr",
            "ignis",
            "glacies",
            "nyx",
        ];
        for (i, n) in names.iter().enumerate() {
            c.shot(n, &view(i, 6.0));
        }
        // Lado nocturno (luces de ciudad, lava, bioluminiscencia, auroras).
        for (i, n) in [
            (2, "terra_nova_noche"),
            (4, "ignis_noche"),
            (5, "glacies_noche"),
            (6, "nyx_noche"),
        ] {
            let mut s = view(i, 6.0);
            s.light_angle = 2.3;
            c.shot(n, &s);
        }
        // Cobertura de nubes.
        for cov in [0.2f32, 0.5, 0.85] {
            let mut s = view(2, 6.0);
            s.params.cloud_coverage = cov;
            s.camera.distance = 3.2;
            c.shot(&format!("nubes_cobertura_{:02}", (cov * 100.0) as i32), &s);
        }
        // Bump mapping: con y sin relieve.
        for (i, n) in [(2, "terra_nova"), (4, "ignis"), (5, "glacies")] {
            let mut s = view(i, 6.0);
            s.camera.yaw = 0.75; // vemos el terminador: luz rasante que resalta el relieve
            s.camera.distance = if i == 2 { 2.6 } else { 2.4 };
            s.params.clouds = false;
            c.shot(&format!("bump_on_{n}"), &s);
            s.params.bump = false;
            c.shot(&format!("bump_off_{n}"), &s);
        }
        // Luna de cerca con bump.
        let mut s = view(2, 6.0);
        let moon = scene::build(2, 6.0)[1];
        s.camera.target = moon.center;
        s.camera.distance = 1.1;
        s.camera.yaw = -1.2;
        s.light_angle = -1.6;
        c.shot("selene", &s);
        // Sombras de nubes: con y sin.
        let mut s = view(2, 6.0);
        s.camera.distance = 2.2;
        s.camera.yaw = 1.15;
        s.params.cloud_coverage = 0.45;
        c.shot("sombras_on", &s);
        s.params.cloud_shadows = false;
        c.shot("sombras_off", &s);
        // Tormenta: con y sin.
        let mut s = view(3, 6.0);
        s.camera.distance = 3.4;
        s.camera.yaw = -0.6;
        s.camera.pitch = -0.25;
        s.light_angle = -1.0;
        c.shot("tormenta_on", &s);
        s.params.storm = false;
        c.shot("tormenta_off", &s);
    }

    if what != "stills" {
        println!("Clips:");
        c.clip("anim_helios", 90, view(1, 0.0), |s, k| s.time = k * 9.0);
        c.clip(
            "anim_terra_nubes",
            90,
            {
                let mut s = view(2, 0.0);
                s.camera.distance = 3.0;
                s
            },
            |s, k| s.time = k * 30.0,
        );
        c.clip(
            "anim_terra_luz",
            90,
            {
                let mut s = view(2, 6.0);
                s.camera.distance = 3.0;
                s
            },
            |s, k| {
                s.light_angle = -1.0 + k * 3.6;
            },
        );
        c.clip(
            "anim_zephyr",
            90,
            {
                let mut s = view(3, 0.0);
                s.camera.pitch = 0.3;
                s
            },
            |s, k| s.time = k * 18.0,
        );
        c.clip(
            "anim_tormenta",
            90,
            {
                let mut s = view(3, 0.0);
                s.camera.distance = 3.2;
                s.camera.yaw = -0.6;
                s.camera.pitch = -0.25;
                s.light_angle = -1.0;
                s
            },
            |s, k| s.time = k * 12.0,
        );
        c.clip(
            "anim_ignis",
            90,
            {
                let mut s = view(4, 0.0);
                s.light_angle = 1.6;
                s
            },
            |s, k| s.time = k * 9.0,
        );
        c.clip(
            "anim_glacies_parallax",
            90,
            {
                let mut s = view(5, 6.0);
                s.camera.distance = 1.9;
                s.light_angle = 0.4;
                s
            },
            |s, k| {
                s.camera.yaw = (k * std::f32::consts::TAU).sin() * 0.45;
            },
        );
        c.clip(
            "anim_glacies_sin_parallax",
            90,
            {
                let mut s = view(5, 6.0);
                s.camera.distance = 1.9;
                s.light_angle = 0.4;
                s.params.parallax = false;
                s
            },
            |s, k| {
                s.camera.yaw = (k * std::f32::consts::TAU).sin() * 0.45;
            },
        );
        c.clip(
            "anim_glacies_aurora",
            90,
            {
                let mut s = view(5, 0.0);
                s.light_angle = 2.4;
                s.camera.pitch = 0.5;
                s
            },
            |s, k| s.time = k * 9.0,
        );
        c.clip(
            "anim_nyx",
            90,
            {
                let mut s = view(6, 0.0);
                s.light_angle = 2.0;
                s
            },
            |s, k| s.time = k * 9.0,
        );
        c.clip("anim_sistema", 120, view(0, 0.0), |s, k| {
            s.time = k * 18.0;
            s.camera.yaw = k * 0.8 - 0.4;
            s.camera.pitch = 0.35;
        });
    }
}
