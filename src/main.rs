//! Laboratorio: planetas y estrellas con shaders (software renderer en CPU).
//!
//! Ejecutar:  cargo run --release
//! Capturas:  cargo run --release -- --capture capture

mod camera;
mod capture;
mod framebuffer;
mod noise;
mod obj;
mod render;
mod scene;
mod shaders;
mod vertex;

use camera::{projection_matrix, Camera};
use framebuffer::Framebuffer;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nalgebra_glm::Vec3;
use render::{Params, Uniforms};
use std::time::Instant;

pub const WIDTH: usize = 800;
pub const HEIGHT: usize = 600;

/// Estado de la aplicación que se puede controlar con el teclado.
pub struct State {
    pub selected: usize,
    pub camera: Camera,
    pub time: f32,
    pub light_angle: f32,
    pub params: Params,
}

impl State {
    pub fn new(selected: usize) -> Self {
        State {
            selected,
            camera: Camera {
                target: Vec3::zeros(),
                yaw: 0.0,
                pitch: 0.12,
                distance: scene::default_distance(selected),
            },
            time: 0.0,
            light_angle: 0.0,
            params: Params::default(),
        }
    }

    /// Posición de la luz: en la vista general es la estrella; en las vistas
    /// individuales es un "sol" lejano que se puede rotar con Z / X.
    pub fn light_pos(&self) -> Vec3 {
        if self.selected == 0 {
            Vec3::zeros()
        } else {
            shaders::common::rot_y(Vec3::new(1.0, 0.32, 0.55).normalize(), self.light_angle) * 60.0
        }
    }

    pub fn uniforms(&self) -> Uniforms {
        Uniforms {
            view: self.camera.view_matrix(),
            projection: projection_matrix(WIDTH as f32, HEIGHT as f32),
            width: WIDTH as f32,
            height: HEIGHT as f32,
            time: self.time,
            cam_pos: self.camera.eye(),
            light_pos: self.light_pos(),
            params: self.params,
        }
    }
}

fn main() {
    let sphere = obj::Obj::load("assets/sphere.obj").expect("No se encontró assets/sphere.obj");
    let noise = noise::Noises::new();

    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--capture") {
        let dir = args.get(i + 1).cloned().unwrap_or_else(|| "capture".into());
        capture::run(&dir, &sphere.vertices, &noise);
        return;
    }

    let mut fb = Framebuffer::new(WIDTH, HEIGHT);
    let mut window = Window::new(
        "Laboratorio de shaders",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .expect("No se pudo crear la ventana");

    let mut state = State::new(1);
    let mut paused = false;
    let mut last = Instant::now();
    let mut fps_timer = 0.0;
    let mut frames = 0;
    print_help();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = last.elapsed().as_secs_f32().min(0.1);
        last = Instant::now();
        if !paused {
            state.time += dt;
        }
        handle_input(&window, &mut state, &mut paused, dt);

        let instances = scene::build(state.selected, state.time);
        render::render(
            &mut fb,
            &sphere.vertices,
            &instances,
            &state.uniforms(),
            &noise,
        );
        window.update_with_buffer(&fb.color, WIDTH, HEIGHT).unwrap();

        frames += 1;
        fps_timer += dt;
        if fps_timer > 0.5 {
            let p = &state.params;
            window.set_title(&format!(
                "[{}] {}  |  {:.0} fps  |  nubes {:.2}  bump {:.1}  parallax {:.3}  tormenta {:.1}{}",
                state.selected,
                scene::BODY_NAMES[state.selected],
                frames as f32 / fps_timer,
                p.cloud_coverage,
                if p.bump { p.bump_strength } else { 0.0 },
                if p.parallax { p.parallax_scale } else { 0.0 },
                if p.storm { p.storm_strength } else { 0.0 },
                if paused { "  (pausa)" } else { "" },
            ));
            frames = 0;
            fps_timer = 0.0;
        }
    }
}

fn handle_input(window: &Window, s: &mut State, paused: &mut bool, dt: f32) {
    let pressed = |k| window.is_key_pressed(k, KeyRepeat::No);
    let views = [
        Key::Key0,
        Key::Key1,
        Key::Key2,
        Key::Key3,
        Key::Key4,
        Key::Key5,
        Key::Key6,
    ];
    for (i, k) in views.iter().enumerate() {
        if pressed(*k) {
            s.selected = i;
            s.camera.distance = scene::default_distance(i);
        }
    }
    if pressed(Key::Tab) {
        s.selected = (s.selected + 1) % views.len();
        s.camera.distance = scene::default_distance(s.selected);
    }

    // Cámara orbital.
    let speed = 1.5 * dt;
    if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
        s.camera.yaw -= speed;
    }
    if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
        s.camera.yaw += speed;
    }
    if window.is_key_down(Key::Up) {
        s.camera.pitch = (s.camera.pitch + speed).min(1.4);
    }
    if window.is_key_down(Key::Down) {
        s.camera.pitch = (s.camera.pitch - speed).max(-1.4);
    }
    if window.is_key_down(Key::W) {
        s.camera.distance = (s.camera.distance - 2.0 * dt).max(1.4);
    }
    if window.is_key_down(Key::S) {
        s.camera.distance = (s.camera.distance + 2.0 * dt).min(30.0);
    }
    // Luz.
    if window.is_key_down(Key::Z) {
        s.light_angle -= speed;
    }
    if window.is_key_down(Key::X) {
        s.light_angle += speed;
    }

    let p = &mut s.params;
    if window.is_key_down(Key::RightBracket) {
        p.cloud_coverage = (p.cloud_coverage + 0.4 * dt).min(1.0);
    }
    if window.is_key_down(Key::LeftBracket) {
        p.cloud_coverage = (p.cloud_coverage - 0.4 * dt).max(0.0);
    }
    if window.is_key_down(Key::Equal) {
        p.bump_strength = (p.bump_strength + dt).min(4.0);
    }
    if window.is_key_down(Key::Minus) {
        p.bump_strength = (p.bump_strength - dt).max(0.0);
    }
    if window.is_key_down(Key::Period) {
        p.parallax_scale = (p.parallax_scale + 0.04 * dt).min(0.2);
    }
    if window.is_key_down(Key::Comma) {
        p.parallax_scale = (p.parallax_scale - 0.04 * dt).max(0.0);
    }
    if window.is_key_down(Key::M) {
        p.storm_strength = (p.storm_strength + 2.0 * dt).min(10.0);
    }
    if window.is_key_down(Key::N) {
        p.storm_strength = (p.storm_strength - 2.0 * dt).max(0.0);
    }
    if pressed(Key::C) {
        p.clouds = !p.clouds;
    }
    if pressed(Key::H) {
        p.cloud_shadows = !p.cloud_shadows;
    }
    if pressed(Key::B) {
        p.bump = !p.bump;
    }
    if pressed(Key::P) {
        p.parallax = !p.parallax;
    }
    if pressed(Key::T) {
        p.storm = !p.storm;
    }
    if pressed(Key::L) {
        p.night_lights = !p.night_lights;
    }
    if pressed(Key::Space) {
        *paused = !*paused;
    }
    if pressed(Key::R) {
        let sel = s.selected;
        *s = State::new(sel);
    }
}

fn print_help() {
    println!("=== Laboratorio de shaders: planetas y estrellas ===");
    println!("0 sistema | 1 Helios | 2 Terra Nova + luna | 3 Zephyr + anillos");
    println!("4 Ignis | 5 Glacies | 6 Nyx | Tab: siguiente");
    println!("Flechas / A D: orbitar cámara   W / S: acercar / alejar");
    println!("Z / X: mover la luz   Espacio: pausa   R: reiniciar vista");
    println!("[ ]: cobertura de nubes   C: nubes   H: sombras de nubes");
    println!("- =: fuerza del bump   B: bump on/off");
    println!(", .: escala del parallax   P: parallax on/off");
    println!("N M: fuerza de la tormenta   T: tormenta on/off   L: luces nocturnas");
}
