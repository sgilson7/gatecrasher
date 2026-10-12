//! gatecrasher_shim: moves values between the page and `gatecrasher_core::game`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core and a conversion of its arguments or result. The core speaks
//! Q16.16 integers and the shaders read most of them as integers too; the few
//! values WebGL wants as floats (the matrix, the uniforms, sound samples) are
//! divided by the core's own `ONE` here. `crates/core/tests/boundary.rs`
//! fails if a branch, a loop or the `?` operator appears here.

use gatecrasher_core::{fx, game};
use wasm_bindgen::prelude::*;

const ONE: f32 = fx::ONE as f32;

fn floats(v: Vec<i32>) -> Vec<f32> {
    v.into_iter().map(|x| x as f32 / ONE).collect()
}

#[wasm_bindgen]
pub struct Game {
    inner: game::Game,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(
        palette: &str,
        missions: &str,
        uniforms: &str,
        recruits: &str,
        saved: &str,
    ) -> Result<Game, JsValue> {
        game::Game::new(palette, missions, uniforms, recruits, saved)
            .map(|inner| Game { inner })
            .map_err(|e| JsValue::from_str(&e))
    }

    /// CSS size and device pixel ratio in; drawing-buffer size out.
    pub fn resize(&mut self, css_w: u32, css_h: u32, dpr: f64) -> Vec<i32> {
        self.inner
            .resize(css_w as i32, css_h as i32, (dpr * 100.0) as i32)
            .to_vec()
    }

    /// The page's clock in milliseconds, held action bits, mouse movement in CSS pixels.
    pub fn advance(&mut self, now_ms: f64, bits: u32, dx: f64, dy: f64) {
        self.inner.advance(
            (now_ms * 1000.0) as i64,
            bits,
            (dx * 100.0) as i32,
            (dy * 100.0) as i32,
        )
    }

    pub fn scenery(&self) -> Vec<i32> {
        self.inner.scenery()
    }

    pub fn scenery_version(&self) -> u32 {
        self.inner.scenery_version()
    }

    pub fn dynamic(&self) -> Vec<i32> {
        self.inner.dynamic()
    }

    pub fn draws(&self) -> Vec<u32> {
        self.inner.draws()
    }

    pub fn mats(&self) -> Vec<f32> {
        floats(self.inner.mats())
    }

    pub fn uniforms(&self) -> Vec<f32> {
        floats(self.inner.uniforms())
    }

    pub fn clear_color(&self) -> Vec<f32> {
        floats(self.inner.clear_color())
    }

    pub fn mesh(&self, id: u32) -> Vec<i32> {
        self.inner.mesh(id)
    }

    pub fn hud(&self) -> String {
        self.inner.hud_json()
    }

    pub fn menu(&self) -> String {
        self.inner.menu_json()
    }

    pub fn saved(&self) -> String {
        self.inner.saved()
    }

    pub fn choose(&mut self, k: u32) {
        self.inner.choose(k)
    }

    pub fn set_uniform(&mut self, k: u32) {
        self.inner.set_uniform(k)
    }

    pub fn deploy(&mut self) {
        self.inner.deploy()
    }

    pub fn pause(&mut self) {
        self.inner.pause()
    }

    pub fn resume(&mut self) {
        self.inner.resume()
    }

    pub fn retry(&mut self) {
        self.inner.retry()
    }

    pub fn to_title(&mut self) {
        self.inner.to_title()
    }

    /// For screenshots and the browser check (core `Game::stage`).
    pub fn stage(&mut self, x: i32, z: i32, yaw: i32) {
        self.inner.stage(x, z, yaw)
    }

    pub fn graceland(&mut self) {
        self.inner.graceland()
    }
}

#[wasm_bindgen]
pub fn numbers() -> String {
    game::numbers()
}

/// White noise for the page's sound, from the core's seeded generator.
#[wasm_bindgen]
pub fn noise(n: u32, seed: u32) -> Vec<f32> {
    floats(game::noise_samples(n, seed))
}
