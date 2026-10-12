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
//!
//! A portal's opening is its own layer. With both portals open, each opening
//! is a window: its pixels are marked in the stencil, their depth is pushed
//! to the far plane, and the scene is drawn again inside them from a second
//! camera standing behind the other portal (Portal's and Splitgate's trick),
//! with everything behind that portal's face cut away. Then the opening's
//! own depth is put back so the see-through shapes sort against it.

use crate::content::Rgb;
use crate::fx::{self, ONE};
use crate::geom::{v3, Affine, Mat4, V3};
use crate::mesh::Mesh;

pub const INSTANCE_VALUES: usize = 16;
/// buffer, mesh, byte offset, count, pass, view, stencil value
pub const DRAW_VALUES: usize = 7;
/// for each view: the view-projection matrix, the eye, and the clip plane
/// (normal, offset: points with `n·p < offset` are cut away)
pub const MAT_VALUES: usize = 23;
/// the main view and the view through each portal
pub const VIEWS: usize = 3;
pub const BYTES_PER_VALUE: usize = 4;

pub const BUF_SCENERY: u32 = 0;
pub const BUF_DYNAMIC: u32 = 1;

pub const PASS_SOLID: u32 = 0;
pub const PASS_INK: u32 = 1;
pub const PASS_TINT: u32 = 2;
/// a portal's opening into the stencil, nothing drawn
pub const PASS_MARK: u32 = 3;
/// the marked pixels' depth to the far plane
pub const PASS_SEAL: u32 = 4;
/// the opening's own depth back
pub const PASS_UNSEAL: u32 = 5;

pub const LAYER_INKED: usize = 0;
pub const LAYER_PLAIN: usize = 1;
pub const LAYER_TINT: usize = 2;
/// the two portals' openings (a swirl when no view shows through)
pub const LAYER_HOLE: usize = 3;
pub const LAYERS: usize = 5;

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

/// The draws for both buffers: every fill, then every outline; then, for
/// each portal with a view through it, the window and the scene inside it;
/// then every see-through shape. `through[k]` says whether portal k shows
/// a view (view k + 1, stencil value k + 1).
pub fn draws(
    scenery: &[(usize, u32, u32, u32)],
    dynamic: &[(usize, u32, u32, u32)],
    through: [bool; 2],
) -> Vec<u32> {
    let mut out = Vec::new();
    let sets = [(BUF_SCENERY, scenery), (BUF_DYNAMIC, dynamic)];
    let mut emit = |pass: u32, view: u32, stencil: u32, pick: &dyn Fn(usize) -> bool| {
        for (buf, runs) in sets {
            for &(layer, mesh, offset, count) in runs.iter() {
                if pick(layer) {
                    out.extend_from_slice(&[buf, mesh, offset, count, pass, view, stencil]);
                }
            }
        }
    };
    let swirl = |layer: usize| layer >= LAYER_HOLE && !through[layer - LAYER_HOLE];
    emit(PASS_SOLID, 0, 0, &|l| {
        l == LAYER_INKED || l == LAYER_PLAIN || swirl(l)
    });
    emit(PASS_INK, 0, 0, &|l| l == LAYER_INKED);
    for (k, &open) in through.iter().enumerate() {
        if !open {
            continue;
        }
        let (view, st) = (k as u32 + 1, k as u32 + 1);
        let hole = move |l: usize| l == LAYER_HOLE + k;
        emit(PASS_MARK, 0, st, &hole);
        emit(PASS_SEAL, 0, st, &hole);
        // inside the window this opening is a swirl (one level deep); the
        // one you look out of is not drawn at all
        let other = LAYER_HOLE + 1 - k;
        emit(PASS_SOLID, view, st, &|l| {
            l == LAYER_INKED || l == LAYER_PLAIN || (l >= LAYER_HOLE && l != other)
        });
        emit(PASS_INK, view, st, &|l| l == LAYER_INKED);
        emit(PASS_UNSEAL, 0, st, &hole);
    }
    emit(PASS_TINT, 0, 0, &|l| l == LAYER_TINT);
    out
}

/// A camera: where it is, where it looks, its vertical field of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Camera {
    pub eye: V3,
    pub fwd: V3,
    pub fov: i32,
    pub far: i32,
    /// which way is up on screen: the world's up, except through a floor portal
    pub up: V3,
}

/// A clip plane that cuts nothing away.
pub const NO_CLIP: [i32; 4] = [0, 0, 0, -fx::int(10000)];

impl Camera {
    pub fn matrix(&self, aspect: i32) -> Mat4 {
        let p = Mat4::perspective(self.fov, aspect, NEAR, self.far);
        p.mul(&Mat4::look(self.eye, self.fwd, self.up))
    }

    pub fn mat_values(&self, aspect: i32, clip: [i32; 4]) -> [i32; MAT_VALUES] {
        let m = self.matrix(aspect).0;
        let mut out = [0; MAT_VALUES];
        out[..16].copy_from_slice(&m);
        out[16] = self.eye.x;
        out[17] = self.eye.y;
        out[18] = self.eye.z;
        out[19..].copy_from_slice(&clip);
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
        let d = draws(&runs, &[], [false, false]);
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
    fn with_a_view_through_a_portal_the_window_is_marked_sealed_drawn_in_and_unsealed() {
        let mut b = Batch::new();
        let a = boxed(V3::ZERO, v3(ONE, ONE, ONE));
        b.push(LAYER_INKED, Mesh::Cube, &a, [ONE; 3], 0);
        b.push(LAYER_HOLE, Mesh::Cyl, &a, [ONE; 3], 0);
        b.push(LAYER_HOLE + 1, Mesh::Cyl, &a, [ONE; 3], 0);
        let (_, runs) = b.finish();
        let shut: Vec<u32> = draws(&[], &runs, [false, false])
            .chunks(DRAW_VALUES)
            .map(|c| c[4])
            .collect();
        assert_eq!(
            shut,
            vec![PASS_SOLID, PASS_SOLID, PASS_SOLID, PASS_INK],
            "both openings are swirls"
        );
        let d = draws(&[], &runs, [true, false]);
        let steps: Vec<(u32, u32, u32)> = d.chunks(DRAW_VALUES).map(|c| (c[4], c[5], c[6])).collect();
        assert_eq!(
            steps,
            vec![
                (PASS_SOLID, 0, 0),
                (PASS_SOLID, 0, 0),
                (PASS_INK, 0, 0),
                (PASS_MARK, 0, 1),
                (PASS_SEAL, 0, 1),
                (PASS_SOLID, 1, 1),
                (PASS_SOLID, 1, 1),
                (PASS_INK, 1, 1),
                (PASS_UNSEAL, 0, 1),
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
            up: V3::UP,
        };
        let (x, y) = cam.screen(ONE, v3(0, int(2), -int(10))).unwrap();
        assert!((x - int(50)).abs() < ONE && (y - int(50)).abs() < ONE, "{x} {y}");
        assert!(cam.screen(ONE, v3(0, int(2), int(10))).is_none());
    }
}
