//! What the page draws, and in what order.
//!
//! Two instance buffers: the zone, which never moves (uploaded once, and
//! again only when a cage door opens), and the few hundred things rebuilt
//! each frame (people, the mule, portals, sight cones, tracers, balloons).
//! An instance is a 3×4 transform and a colour with a style, as Q16
//! integers; the shader reads them as integers and scales them.
//!
//! Each shape belongs to a layer: inked (filled, then outlined in plum by an
//! inverted hull, like the TikZ art), plain (filled, no outline: the ground)
//! or tint (see-through, drawn last: sight cones and the landing zone's
//! glow). The core lists the draws; the page sets the GL state each asks for.

use crate::content::Rgb;
use crate::fx::{self, ONE};
use crate::geom::{v3, Affine, Mat4, V3};
use crate::mesh::Mesh;

pub const INSTANCE_VALUES: usize = 16;
/// buffer, mesh, byte offset, count, pass
pub const DRAW_VALUES: usize = 5;
/// the view-projection matrix and the eye
pub const MAT_VALUES: usize = 19;
pub const BYTES_PER_VALUE: usize = 4;

pub const BUF_SCENERY: u32 = 0;
pub const BUF_DYNAMIC: u32 = 1;

pub const PASS_SOLID: u32 = 0;
pub const PASS_INK: u32 = 1;
pub const PASS_TINT: u32 = 2;

pub const LAYER_INKED: usize = 0;
pub const LAYER_PLAIN: usize = 1;
pub const LAYER_TINT: usize = 2;
pub const LAYERS: usize = 3;

pub const MESH_COUNT: u32 = Mesh::ALL.len() as u32;
pub const NEAR: i32 = ONE / 10;

/// A finished batch: its values, and for each layer and mesh (layer, mesh,
/// byte offset, count).
pub type Finished = (Vec<i32>, Vec<(usize, u32, u32, u32)>);

#[derive(Debug, Clone)]
pub struct Batch {
    lists: Vec<Vec<i32>>,
}

fn slot(layer: usize, mesh: Mesh) -> usize {
    layer * Mesh::ALL.len() + mesh as usize
}

pub fn instance(a: &Affine, rgb: Rgb, style: i32) -> [i32; INSTANCE_VALUES] {
    [
        a.c0.x, a.c0.y, a.c0.z, a.t.x, a.c1.x, a.c1.y, a.c1.z, a.t.y, a.c2.x, a.c2.y, a.c2.z, a.t.z, rgb[0],
        rgb[1], rgb[2], style,
    ]
}

impl Default for Batch {
    fn default() -> Self {
        Self::new()
    }
}

impl Batch {
    pub fn new() -> Batch {
        Batch {
            lists: vec![Vec::new(); LAYERS * Mesh::ALL.len()],
        }
    }

    /// `style` is a shader style number; it reaches the shader as a whole
    /// number in the colour's fourth channel.
    pub fn push(&mut self, layer: usize, mesh: Mesh, a: &Affine, rgb: Rgb, style: i32) {
        self.lists[slot(layer, mesh)].extend_from_slice(&instance(a, rgb, fx::int(style)));
    }

    /// A see-through shape; `alpha` in Q16.
    pub fn tint(&mut self, mesh: Mesh, a: &Affine, rgb: Rgb, alpha: i32) {
        self.lists[slot(LAYER_TINT, mesh)].extend_from_slice(&instance(a, rgb, alpha));
    }

    pub fn count(&self) -> usize {
        self.lists.iter().map(|l| l.len() / INSTANCE_VALUES).sum()
    }

    pub fn finish(self) -> Finished {
        let mut values = Vec::new();
        let mut runs = Vec::new();
        for layer in 0..LAYERS {
            for mesh in Mesh::ALL {
                let v = &self.lists[slot(layer, mesh)];
                if v.is_empty() {
                    continue;
                }
                let offset = (values.len() * BYTES_PER_VALUE) as u32;
                runs.push((layer, mesh as u32, offset, (v.len() / INSTANCE_VALUES) as u32));
                values.extend_from_slice(v);
            }
        }
        (values, runs)
    }
}

/// The draws for both buffers: every fill, then every outline, then every
/// see-through shape.
pub fn draws(scenery: &[(usize, u32, u32, u32)], dynamic: &[(usize, u32, u32, u32)]) -> Vec<u32> {
    let mut out = Vec::new();
    let sets = [(BUF_SCENERY, scenery), (BUF_DYNAMIC, dynamic)];
    for (pass, layers) in [
        (PASS_SOLID, [LAYER_INKED, LAYER_PLAIN]),
        (PASS_INK, [LAYER_INKED, LAYER_INKED]),
        (PASS_TINT, [LAYER_TINT, LAYER_TINT]),
    ] {
        for (buf, runs) in sets {
            for &(layer, mesh, offset, count) in runs.iter() {
                if layer == layers[0] || (layers[1] != layers[0] && layer == layers[1]) {
                    out.extend_from_slice(&[buf, mesh, offset, count, pass]);
                }
            }
        }
    }
    out
}

/// A camera: where it is, where it looks, its vertical field of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Camera {
    pub eye: V3,
    pub fwd: V3,
    pub fov: i32,
    pub far: i32,
}

impl Camera {
    pub fn matrix(&self, aspect: i32) -> Mat4 {
        let p = Mat4::perspective(self.fov, aspect, NEAR, self.far);
        p.mul(&Mat4::look(self.eye, self.fwd, V3::UP))
    }

    pub fn mat_values(&self, aspect: i32) -> [i32; MAT_VALUES] {
        let m = self.matrix(aspect).0;
        let mut out = [0; MAT_VALUES];
        out[..16].copy_from_slice(&m);
        out[16] = self.eye.x;
        out[17] = self.eye.y;
        out[18] = self.eye.z;
        out
    }

    /// Where a world point lands, in percent (Q16) from the left and the
    /// top of the screen; `None` behind the camera.
    pub fn screen(&self, aspect: i32, p: V3) -> Option<(i32, i32)> {
        let [x, y, _, w] = self.matrix(aspect).project(p);
        if w <= ONE / 100 {
            return None;
        }
        Some(((fx::div(x, w) + ONE) * 50, (ONE - fx::div(y, w)) * 50))
    }
}

/// A box standing on `at` (the middle of its foot) of a size.
pub fn standing(at: V3, size: V3) -> Affine {
    Affine::translate(at.add(v3(0, size.y / 2, 0))).then(&Affine::scale(size))
}

pub fn boxed(at: V3, size: V3) -> Affine {
    Affine::translate(at).then(&Affine::scale(size))
}

/// A box between two corners.
pub fn spanning(lo: V3, hi: V3) -> Affine {
    let size = hi.sub(lo);
    boxed(v3(lo.x + size.x / 2, lo.y + size.y / 2, lo.z + size.z / 2), size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::{deg, int};
    use crate::geom::facing;

    #[test]
    fn fills_come_first_then_outlines_then_tints() {
        let mut b = Batch::new();
        let a = boxed(V3::ZERO, v3(ONE, ONE, ONE));
        b.tint(Mesh::Tri, &a, [ONE; 3], ONE / 2);
        b.push(LAYER_PLAIN, Mesh::Ground, &a, [ONE; 3], 0);
        b.push(LAYER_INKED, Mesh::Cube, &a, [ONE; 3], 0);
        let (values, runs) = b.finish();
        assert_eq!(values.len(), 3 * INSTANCE_VALUES);
        let d = draws(&runs, &[]);
        let passes: Vec<u32> = d.chunks(DRAW_VALUES).map(|c| c[4]).collect();
        let meshes: Vec<u32> = d.chunks(DRAW_VALUES).map(|c| c[1]).collect();
        assert_eq!(passes, vec![PASS_SOLID, PASS_SOLID, PASS_INK, PASS_TINT]);
        assert_eq!(
            meshes,
            vec![
                Mesh::Cube as u32,
                Mesh::Ground as u32,
                Mesh::Cube as u32,
                Mesh::Tri as u32
            ]
        );
    }

    #[test]
    fn a_point_ahead_lands_mid_screen_and_one_behind_lands_nowhere() {
        let cam = Camera {
            eye: v3(0, int(2), 0),
            fwd: facing(0, 0),
            fov: deg(60),
            far: int(300),
        };
        let (x, y) = cam.screen(ONE, v3(0, int(2), -int(10))).unwrap();
        assert!((x - int(50)).abs() < ONE && (y - int(50)).abs() < ONE, "{x} {y}");
        assert!(cam.screen(ONE, v3(0, int(2), int(10))).is_none());
    }
}
