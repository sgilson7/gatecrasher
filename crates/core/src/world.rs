//! The zone: one quarter of the outpost, the same for every mission.
//!
//! Everything solid is an axis-aligned box. The operative collides with them
//! as an upright cylinder, soldiers' eyes and bullets meet them as rays, and
//! portals open on their faces. The ground is flat at y = 0 and is typed
//! (grass, dirt, concrete, floor, sand) for the camo index.
//!
//! Conventions: metres in Q16, y up, north is −z. The outpost's wall runs at
//! ±80 m; the zone is 200 m square.

use crate::content::{Palette, Rgb};
use crate::fx::{self, ratio, ONE};
use crate::geom::{v3, V3};
use crate::mesh::Mesh;

/// Decimetres to Q16 metres: the layout below is written in decimetres.
pub const fn dm(x: i32) -> i32 {
    ratio(x, 10)
}

pub const HALF_ZONE: i32 = fx::int(100);
pub const WALL_AT: i32 = fx::int(80);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Grass = 0,
    Dirt = 1,
    Concrete = 2,
    Floor = 3,
    Sand = 4,
}

pub const SURFACES: usize = 5;

/// Shader styles (data/shaders/scene.frag).
pub const STYLE_PLAIN: i32 = 0;
pub const STYLE_PANEL: i32 = 1;
pub const STYLE_METAL: i32 = 2;
pub const STYLE_CAMO: i32 = 3;
pub const STYLE_GLOW: i32 = 4;
pub const STYLE_CRATE: i32 = 5;
pub const STYLE_GROUND: i32 = 6;
pub const STYLE_CLOAK: i32 = 7;
pub const STYLE_PORTAL: i32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Solid {
    pub lo: V3,
    pub hi: V3,
    pub colour: Rgb,
    pub style: i32,
    pub mesh: Mesh,
    /// blocks a soldier's sight
    pub opaque: bool,
    /// a portal can open on its faces
    pub portal: bool,
    /// a roof: what is under it is indoors
    pub roof: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bush {
    pub at: V3,
    pub r: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Patch {
    pub lo: V3,
    pub hi: V3,
}

/// A locker: a box you can stand in, or put a body in. `front` is the unit
/// vector it opens toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Locker {
    pub at: V3,
    pub front: V3,
    pub solid: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub lo: V3,
    pub hi: V3,
    pub surface: Surface,
}

/// Named places the missions refer to (data/missions.json).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub name: &'static str,
    pub at: V3,
}

#[derive(Debug, Clone, Default)]
pub struct World {
    pub solids: Vec<Solid>,
    pub bushes: Vec<Bush>,
    pub grass: Vec<Patch>,
    pub lockers: Vec<Locker>,
    pub regions: Vec<Region>,
    pub places: Vec<Place>,
    /// trees outside the wall: trunk solids are in `solids`; crowns here
    pub crowns: Vec<(V3, i32)>,
    /// the cage's barred door in the warehouse's back room
    pub cage_door: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub t: i32,
    pub at: V3,
    pub normal: V3,
    pub solid: Option<usize>,
}

struct Colours {
    concrete: Rgb,
    hq: Rgb,
    barracks: Rgb,
    warehouse: Rgb,
    prison: Rgb,
    roof: Rgb,
    crate_: Rgb,
    sandbag: Rgb,
    rock: Rgb,
    rock_hi: Rgb,
    metal: Rgb,
    locker: Rgb,
    bars: Rgb,
    trunk: Rgb,
    tank: Rgb,
    containers: Vec<Rgb>,
    truck: Rgb,
}

/// Builds the zone. Decimetres throughout; every coordinate below is a
/// layout decision (PLAN.md, "The zone").
pub fn build(p: &Palette) -> World {
    let c = Colours {
        concrete: p.get("concrete"),
        hq: p.get("hq"),
        barracks: p.get("barracks"),
        warehouse: p.get("warehouse"),
        prison: p.get("prison"),
        roof: p.get("roof"),
        crate_: p.get("crate"),
        sandbag: p.get("sandbag"),
        rock: p.get("rock"),
        rock_hi: p.get("rock_hi"),
        metal: p.get("metal"),
        locker: p.get("locker"),
        bars: p.get("bars"),
        trunk: p.get("trunk"),
        tank: p.get("tank"),
        containers: (0..p.count("containers"))
            .map(|k| p.pick("containers", k))
            .collect(),
        truck: p.get("truck"),
    };
    let mut w = World::default();
    regions(&mut w);
    perimeter(&mut w, &c);
    ridge(&mut w, &c);
    hq(&mut w, &c);
    barracks(&mut w, &c);
    warehouse(&mut w, &c);
    prison(&mut w, &c);
    comms_tower(&mut w, &c);
    guard_tower(&mut w, &c, -720, 720, "tower_sw");
    guard_tower(&mut w, &c, 720, -720, "tower_ne");
    yard(&mut w, &c);
    greenery(&mut w, &c);
    w
}

impl World {
    fn add(&mut self, lo: V3, hi: V3, colour: Rgb, style: i32) -> usize {
        self.solids.push(Solid {
            lo,
            hi,
            colour,
            style,
            mesh: Mesh::Cube,
            opaque: true,
            portal: true,
            roof: false,
        });
        self.solids.len() - 1
    }

    /// A box in decimetres: x0..x1, z0..z1, y0..y1.
    #[allow(clippy::too_many_arguments)]
    fn dbox(
        &mut self,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        y0: i32,
        y1: i32,
        colour: Rgb,
        style: i32,
    ) -> usize {
        self.add(
            v3(dm(x0.min(x1)), dm(y0), dm(z0.min(z1))),
            v3(dm(x0.max(x1)), dm(y1), dm(z0.max(z1))),
            colour,
            style,
        )
    }

    fn place(&mut self, name: &'static str, x: i32, y: i32, z: i32) {
        self.places.push(Place {
            name,
            at: v3(dm(x), dm(y), dm(z)),
        });
    }

    pub fn place_at(&self, name: &str) -> Option<V3> {
        self.places.iter().find(|p| p.name == name).map(|p| p.at)
    }

    /// A wall along x (`along_x`) or z from `a` to `b` at `at`, thickness
    /// `t`, height `h`, with door gaps (centre, width, height) cut out and a
    /// lintel over each.
    #[allow(clippy::too_many_arguments)]
    fn wall(
        &mut self,
        along_x: bool,
        at: i32,
        a: i32,
        b: i32,
        t: i32,
        h: i32,
        doors: &[(i32, i32, i32)],
        colour: Rgb,
        style: i32,
    ) {
        let mut cuts: Vec<(i32, i32, i32)> = doors
            .iter()
            .map(|&(cen, wd, dh)| (cen - wd / 2, cen + wd / 2, dh))
            .collect();
        cuts.sort();
        let mut from = a;
        let seg = |w: &mut World, s: i32, e: i32, y0: i32, y1: i32| {
            if e <= s || y1 <= y0 {
                return;
            }
            if along_x {
                w.dbox(s, at - t / 2, e, at + t / 2, y0, y1, colour, style);
            } else {
                w.dbox(at - t / 2, s, at + t / 2, e, y0, y1, colour, style);
            }
        };
        for (s, e, dh) in cuts {
            seg(self, from, s, 0, h);
            seg(self, s, e, dh, h);
            from = e;
        }
        seg(self, from, b, 0, h);
    }

    /// A building: four walls with doors on each side (south, north, west,
    /// east; centre and width in decimetres), and a roof.
    #[allow(clippy::too_many_arguments)]
    fn building(
        &mut self,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        h: i32,
        doors: [&[(i32, i32, i32)]; 4],
        colour: Rgb,
        roof: Rgb,
        style: i32,
    ) {
        let t = 4;
        self.wall(true, z1, x0, x1, t, h, doors[0], colour, style);
        self.wall(true, z0, x0, x1, t, h, doors[1], colour, style);
        self.wall(false, x0, z0 + t / 2, z1 - t / 2, t, h, doors[2], colour, style);
        self.wall(false, x1, z0 + t / 2, z1 - t / 2, t, h, doors[3], colour, style);
        let r = self.dbox(x0 - 3, z0 - 3, x1 + 3, z1 + 3, h, h + 3, roof, STYLE_PLAIN);
        self.solids[r].roof = true;
        self.regions.push(Region {
            lo: v3(dm(x0), 0, dm(z0)),
            hi: v3(dm(x1), 0, dm(z1)),
            surface: Surface::Floor,
        });
    }

    fn locker(&mut self, x: i32, z: i32, front: V3, colour: Rgb) {
        // 10 dm wide, 20 tall, 6 deep, its back to the wall
        let (hx, hz) = if front.z != 0 { (5, 3) } else { (3, 5) };
        let s = self.dbox(x - hx, z - hz, x + hx, z + hz, 0, 21, colour, STYLE_METAL);
        self.solids[s].portal = false;
        self.lockers.push(Locker {
            at: v3(dm(x), 0, dm(z)),
            front,
            solid: s,
        });
    }

    fn crate_(&mut self, x: i32, z: i32, size: i32, y: i32, c: &Colours) {
        let s = self.dbox(
            x - size / 2,
            z - size / 2,
            x + size / 2,
            z + size / 2,
            y,
            y + size,
            c.crate_,
            STYLE_CRATE,
        );
        self.solids[s].portal = false;
    }

    fn sandbags(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, c: &Colours) {
        let s = self.dbox(x0, z0, x1, z1, 0, 10, c.sandbag, STYLE_PLAIN);
        self.solids[s].portal = false;
    }

    fn bush(&mut self, x: i32, z: i32, r: i32) {
        self.bushes.push(Bush {
            at: v3(dm(x), 0, dm(z)),
            r: dm(r),
            h: dm(13),
        });
    }

    // --- queries ---------------------------------------------------------------

    /// The ground type underfoot: the last region that holds the point.
    pub fn surface_at(&self, p: V3) -> Surface {
        let mut s = Surface::Grass;
        for r in &self.regions {
            if p.x >= r.lo.x && p.x <= r.hi.x && p.z >= r.lo.z && p.z <= r.hi.z {
                s = r.surface;
            }
        }
        s
    }

    /// Opens the cage: its door drops out of the world.
    pub fn open_cage(&mut self) {
        let s = &mut self.solids[self.cage_door];
        s.lo = v3(s.lo.x, -fx::int(50), s.lo.z);
        s.hi = v3(s.hi.x, -fx::int(50), s.hi.z);
    }

    pub fn cage_open(&self) -> bool {
        self.solids.get(self.cage_door).is_some_and(|s| s.hi.y < 0)
    }

    /// Whether a point stands in a bush.
    pub fn in_bush(&self, p: V3) -> bool {
        self.bushes.iter().any(|b| {
            let d = v3(p.x - b.at.x, 0, p.z - b.at.z).len();
            d < b.r && p.y < b.h
        })
    }

    pub fn in_grass(&self, p: V3) -> bool {
        p.y < dm(5)
            && self
                .grass
                .iter()
                .any(|g| p.x >= g.lo.x && p.x <= g.hi.x && p.z >= g.lo.z && p.z <= g.hi.z)
    }

    /// Under a roof or a platform: indoors for the camo index, and no sky
    /// for a fulton balloon.
    pub fn covered(&self, p: V3) -> bool {
        self.solids
            .iter()
            .any(|s| s.lo.y > p.y + dm(15) && p.x > s.lo.x && p.x < s.hi.x && p.z > s.lo.z && p.z < s.hi.z)
    }

    pub fn indoors(&self, p: V3) -> bool {
        self.solids
            .iter()
            .any(|s| s.roof && p.x > s.lo.x && p.x < s.hi.x && p.z > s.lo.z && p.z < s.hi.z && s.lo.y > p.y)
    }

    /// The height of the highest top under a disc that the feet can stand on
    /// from `feet` (no higher than a step above them); the ground is 0.
    pub fn ground(&self, p: V3, r: i32, feet: i32) -> i32 {
        let mut g = 0;
        for s in &self.solids {
            if s.hi.y > feet + crate::balance::STEP || s.hi.y <= g {
                continue;
            }
            if circle_box(p, r, s) {
                g = s.hi.y;
            }
        }
        g
    }

    /// Pushes a standing cylinder (feet `p`, radius `r`, height `h`) out of
    /// every box it overlaps, sideways only. Returns the corrected position.
    pub fn collide(&self, mut p: V3, r: i32, h: i32) -> V3 {
        for _ in 0..3 {
            let mut moved = false;
            for s in &self.solids {
                if s.hi.y <= p.y + crate::balance::STEP || s.lo.y >= p.y + h {
                    continue;
                }
                let cx = p.x.clamp(s.lo.x, s.hi.x);
                let cz = p.z.clamp(s.lo.z, s.hi.z);
                let (dx, dz) = (p.x - cx, p.z - cz);
                let d2 = dx as i64 * dx as i64 + dz as i64 * dz as i64;
                if d2 >= r as i64 * r as i64 {
                    continue;
                }
                if d2 == 0 {
                    // centre inside the box: out through the nearest side
                    let out = [
                        (p.x - s.lo.x, v3(-1, 0, 0)),
                        (s.hi.x - p.x, v3(1, 0, 0)),
                        (p.z - s.lo.z, v3(0, 0, -1)),
                        (s.hi.z - p.z, v3(0, 0, 1)),
                    ];
                    let (d, n) = out.iter().min_by_key(|o| o.0).copied().unwrap_or((0, V3::ZERO));
                    p.x += n.x * (d + r);
                    p.z += n.z * (d + r);
                } else {
                    let d = fx::isqrt(d2 as u64) as i32;
                    let push = r - d;
                    p.x += fx::mul(fx::div(dx, d), push);
                    p.z += fx::mul(fx::div(dz, d), push);
                }
                moved = true;
            }
            if !moved {
                break;
            }
        }
        let lim = HALF_ZONE - dm(10);
        p.x = p.x.clamp(-lim, lim);
        p.z = p.z.clamp(-lim, lim);
        p
    }

    /// Whether a standing cylinder overlaps any box (between `y0` and `y1`).
    pub fn blocked(&self, p: V3, r: i32, y0: i32, y1: i32) -> bool {
        self.solids
            .iter()
            .any(|s| s.hi.y > y0 && s.lo.y < y1 && circle_box(p, r, s))
    }

    /// The first box face a ray meets within `max` metres. `d` is a unit
    /// vector. `want` filters which boxes count. The ground plane counts too.
    pub fn ray(&self, o: V3, d: V3, max: i32, want: impl Fn(&Solid) -> bool) -> Option<Hit> {
        let end = o.add(d.scale(max));
        let (blo, bhi) = (
            v3(o.x.min(end.x), o.y.min(end.y), o.z.min(end.z)),
            v3(o.x.max(end.x), o.y.max(end.y), o.z.max(end.z)),
        );
        let mut best: Option<Hit> = None;
        let mut best_t = max;
        if d.y < 0 && o.y >= 0 {
            let t = fx::div(o.y, -d.y);
            if t <= best_t {
                best_t = t;
                best = Some(Hit {
                    t,
                    at: v3(o.x + fx::mul(d.x, t), 0, o.z + fx::mul(d.z, t)),
                    normal: V3::UP,
                    solid: None,
                });
            }
        }
        for (i, s) in self.solids.iter().enumerate() {
            if s.hi.x < blo.x
                || s.lo.x > bhi.x
                || s.hi.y < blo.y
                || s.lo.y > bhi.y
                || s.hi.z < blo.z
                || s.lo.z > bhi.z
            {
                continue;
            }
            if !want(s) {
                continue;
            }
            if let Some((t, n)) = slab(o, d, s.lo, s.hi) {
                if t < best_t {
                    best_t = t;
                    best = Some(Hit {
                        t,
                        at: o.add(d.scale(t)),
                        normal: n,
                        solid: Some(i),
                    });
                }
            }
        }
        best
    }

    /// Whether nothing opaque stands between two points.
    pub fn clear(&self, a: V3, b: V3) -> bool {
        let d = b.sub(a);
        let len = d.len();
        if len == 0 {
            return true;
        }
        let dir = d.norm();
        match self.ray(a, dir, len, |s| s.opaque) {
            Some(h) => h.t >= len - dm(1),
            None => true,
        }
    }
}

/// Whether a disc in the xz plane overlaps a box's footprint.
pub fn circle_box(p: V3, r: i32, s: &Solid) -> bool {
    let cx = p.x.clamp(s.lo.x, s.hi.x);
    let cz = p.z.clamp(s.lo.z, s.hi.z);
    let (dx, dz) = ((p.x - cx) as i64, (p.z - cz) as i64);
    dx * dx + dz * dz < r as i64 * r as i64
}

/// Ray against a box (the slab method). Returns the entry distance and the
/// face normal, or `None`; a ray starting inside returns nothing.
pub fn slab(o: V3, d: V3, lo: V3, hi: V3) -> Option<(i32, V3)> {
    let mut tmin = i64::MIN;
    let mut tmax = i64::MAX;
    let mut normal = V3::ZERO;
    let axes = [
        (o.x, d.x, lo.x, hi.x, 0),
        (o.y, d.y, lo.y, hi.y, 1),
        (o.z, d.z, lo.z, hi.z, 2),
    ];
    for (oa, da, la, ha, k) in axes {
        if da == 0 {
            if oa < la || oa > ha {
                return None;
            }
            continue;
        }
        let t1 = (((la - oa) as i64) << 16) / da as i64;
        let t2 = (((ha - oa) as i64) << 16) / da as i64;
        let (near, far, sign) = if t1 < t2 { (t1, t2, -1) } else { (t2, t1, 1) };
        if near > tmin {
            tmin = near;
            normal = match k {
                0 => v3(sign * ONE, 0, 0),
                1 => v3(0, sign * ONE, 0),
                _ => v3(0, 0, sign * ONE),
            };
        }
        tmax = tmax.min(far);
        if tmin > tmax {
            return None;
        }
    }
    if tmin < 0 || tmin > i32::MAX as i64 {
        return None;
    }
    Some((tmin as i32, normal))
}

// --- the layout --------------------------------------------------------------------

fn regions(w: &mut World) {
    let r = |w: &mut World, x0: i32, z0: i32, x1: i32, z1: i32, s: Surface| {
        w.regions.push(Region {
            lo: v3(dm(x0), 0, dm(z0)),
            hi: v3(dm(x1), 0, dm(z1)),
            surface: s,
        })
    };
    // outside: grass; the beach to the east
    r(w, -1000, -1000, 1000, 1000, Surface::Grass);
    r(w, 900, -1000, 1000, 1000, Surface::Sand);
    // inside the wall: packed dirt, with concrete roads and two grassy corners
    r(w, -800, -800, 800, 800, Surface::Dirt);
    r(w, -60, -460, 60, 800, Surface::Concrete);
    r(w, -800, -60, 800, 60, Surface::Concrete);
    r(w, -380, 80, -60, 240, Surface::Concrete);
    r(w, -790, -120, -600, 300, Surface::Grass);
    r(w, 520, 520, 790, 790, Surface::Grass);
    r(w, 300, 300, 600, 460, Surface::Concrete);
}

fn perimeter(w: &mut World, c: &Colours) {
    let (h, t) = (36, 6);
    // south: the main gate; east: a side door; west: a drain to crawl through
    w.wall(
        true,
        800,
        -800,
        800,
        t,
        h,
        &[(0, 100, h)],
        c.concrete,
        STYLE_PANEL,
    );
    w.wall(true, -800, -800, 800, t, h, &[], c.concrete, STYLE_PANEL);
    w.wall(
        false,
        800,
        -797,
        797,
        t,
        h,
        &[(300, 24, 24)],
        c.concrete,
        STYLE_PANEL,
    );
    w.wall(
        false,
        -800,
        -797,
        797,
        t,
        h,
        &[(-290, 20, 8)],
        c.concrete,
        STYLE_PANEL,
    );
    w.place("gate", 0, 0, 820);
    w.place("drain", -820, 0, -290);
    w.place("side_door", 820, 0, 300);
}

fn ridge(w: &mut World, c: &Colours) {
    // three terraces north of the wall, each a vault high, looking down into
    // the yard: the place to open a portal on a roof from
    for (k, (z0, z1)) in [(-880, -840), (-920, -880), (-1000, -920)].iter().enumerate() {
        let h = 14 * (k as i32 + 1);
        let col = if k % 2 == 0 { c.rock } else { c.rock_hi };
        w.dbox(-640, *z0, 420, *z1, 0, h, col, STYLE_PLAIN);
    }
    w.place("ridge", -200, 42, -960);
}

fn hq(w: &mut World, c: &Colours) {
    // x −24..8, z −66..−46 m; the west room has lockers, the east room is the office
    let h = 42;
    w.building(
        -240,
        -660,
        80,
        -460,
        h,
        [&[(-140, 24, 24), (0, 24, 24)], &[], &[], &[(-560, 24, 24)]],
        c.hq,
        c.roof,
        STYLE_PANEL,
    );
    w.wall(false, -60, -658, -462, 4, h, &[(-560, 24, 24)], c.hq, STYLE_PANEL);
    for k in 0..6 {
        w.locker(-220 + k * 12, -652, v3(0, 0, ONE), c.locker);
    }
    // the office: a desk and a filing cabinet
    let d = w.dbox(10, -620, 40, -600, 0, 8, c.crate_, STYLE_CRATE);
    w.solids[d].portal = false;
    let f = w.dbox(60, -650, 74, -640, 0, 16, c.metal, STYLE_METAL);
    w.solids[f].portal = false;
    w.place("hq_office", 25, 0, -580);
    w.place("hq_desk", 25, 8, -610);
    w.place("hq_west", -150, 0, -560);
    w.place("hq_door", -140, 0, -440);
}

fn barracks(w: &mut World, c: &Colours) {
    // x −70..−46, z −30..−14 m
    w.building(
        -700,
        -300,
        -460,
        -140,
        36,
        [&[(-580, 24, 24)], &[], &[], &[(-220, 24, 24)]],
        c.barracks,
        c.roof,
        STYLE_PANEL,
    );
    for k in 0..10 {
        w.locker(-680 + k * 12, -292, v3(0, 0, ONE), c.locker);
    }
    // bunks along the south wall
    for k in 0..4 {
        let x = -680 + k * 50;
        let b = w.dbox(x, -190, x + 30, -150, 0, 6, c.metal, STYLE_METAL);
        w.solids[b].portal = false;
    }
    w.place("barracks", -580, 0, -220);
    w.place("barracks_door", -580, 0, -120);
}

fn warehouse(w: &mut World, c: &Colours) {
    // x 26..62, z −34..−6 m; a back room behind a partition at x = 52
    let h = 75;
    w.building(
        260,
        -340,
        620,
        -60,
        h,
        [&[(420, 80, 50)], &[], &[(-200, 24, 24)], &[]],
        c.warehouse,
        c.roof,
        STYLE_METAL,
    );
    w.wall(
        false,
        520,
        -338,
        -62,
        4,
        h,
        &[(-200, 24, 24)],
        c.warehouse,
        STYLE_METAL,
    );
    // crates in the hall
    for (x, z, s, y) in [
        (300, -300, 14, 0),
        (316, -300, 14, 0),
        (300, -300, 14, 14),
        (340, -150, 12, 0),
        (460, -280, 14, 0),
        (460, -264, 14, 0),
        (460, -280, 14, 14),
        (480, -120, 12, 0),
        (380, -230, 12, 0),
    ] {
        w.crate_(x, z, s, y, c);
    }
    // the cage in the back room: bars a soldier can see through
    let bars = |w: &mut World, x0: i32, z0: i32, x1: i32, z1: i32| {
        let s = w.dbox(x0, z0, x1, z1, 0, 26, c.bars, STYLE_METAL);
        w.solids[s].opaque = false;
        w.solids[s].portal = false;
    };
    bars(w, 550, -300, 590, -298);
    bars(w, 550, -262, 590, -260);
    bars(w, 588, -300, 590, -260);
    w.cage_door = w.solids.len();
    bars(w, 550, -298, 552, -262);
    // the west side is the cage door, opened in mission 1 (missions.json)
    w.place("cage", 570, 0, -280);
    w.place("cage_door", 551, 0, -280);
    w.place("warehouse", 420, 0, -200);
    w.place("warehouse_back", 570, 0, -160);
}

fn prison(w: &mut World, c: &Colours) {
    // x −70..−48, z 34..52 m; three cells along the west wall
    w.building(
        -700,
        340,
        -480,
        520,
        34,
        [&[], &[], &[], &[(440, 24, 24)]],
        c.prison,
        c.roof,
        STYLE_PANEL,
    );
    for z in [400, 460] {
        w.dbox(-698, z - 2, -620, z + 2, 0, 34, c.prison, STYLE_PANEL);
    }
    for (z0, z1) in [(342, 398), (402, 458), (462, 518)] {
        let mid = (z0 + z1) / 2;
        for (a, b) in [(z0, mid - 8), (mid + 8, z1)] {
            let s = w.dbox(-622, a, -618, b, 0, 34, c.bars, STYLE_METAL);
            w.solids[s].opaque = false;
            w.solids[s].portal = false;
        }
    }
    w.place("prison", -560, 0, 440);
    w.place("cell_2", -660, 0, 430);
}

fn comms_tower(w: &mut World, c: &Colours) {
    // a concrete shaft 16 m tall with a railed deck and no stairs. The hut on
    // the deck faces north: from the ridge its wall is in sight, and a
    // portal on it opens just over the deck (BRIEF: portals reach places).
    w.dbox(300, -600, 360, -540, 0, 160, c.concrete, STYLE_PANEL);
    w.dbox(280, -620, 380, -520, 160, 164, c.metal, STYLE_METAL);
    let rail = |w: &mut World, x0: i32, z0: i32, x1: i32, z1: i32| {
        let s = w.dbox(x0, z0, x1, z1, 164, 174, c.metal, STYLE_METAL);
        w.solids[s].opaque = false;
        w.solids[s].portal = false;
    };
    rail(w, 280, -620, 380, -618);
    rail(w, 280, -522, 380, -520);
    rail(w, 280, -618, 282, -522);
    rail(w, 378, -618, 380, -522);
    w.dbox(282, -600, 320, -540, 164, 204, c.concrete, STYLE_PANEL);
    let a = w.dbox(358, -538, 362, -534, 164, 280, c.metal, STYLE_METAL);
    w.solids[a].portal = false;
    w.place("comms_top", 345, 164, -580);
    w.place("comms", 330, 0, -500);
}

fn guard_tower(w: &mut World, c: &Colours, x: i32, z: i32, name: &'static str) {
    for (dx, dz) in [(-16, -16), (16, -16), (-16, 16), (16, 16)] {
        let s = w.dbox(
            x + dx - 2,
            z + dz - 2,
            x + dx + 2,
            z + dz + 2,
            0,
            88,
            c.metal,
            STYLE_METAL,
        );
        w.solids[s].portal = false;
        w.solids[s].opaque = false;
    }
    w.dbox(x - 20, z - 20, x + 20, z + 20, 60, 64, c.metal, STYLE_METAL);
    // waist-high armour round the deck, leaving gaps at the corners
    for (x0, z0, x1, z1) in [
        (-18, -20, 18, -18),
        (-18, 18, 18, 20),
        (-20, -18, -18, 18),
        (18, -18, 20, 18),
    ] {
        let s = w.dbox(x + x0, z + z0, x + x1, z + z1, 64, 74, c.sandbag, STYLE_PLAIN);
        w.solids[s].opaque = false;
    }
    let r = w.dbox(x - 22, z - 22, x + 22, z + 22, 88, 91, c.roof, STYLE_PLAIN);
    w.solids[r].portal = false;
    w.place(name, x, 64, z);
}

fn yard(w: &mut World, c: &Colours) {
    // shipping containers, some two high: walls to put portals on
    let mut k = 0;
    for (x, z, stack) in [
        (-340, 120, 2),
        (-270, 120, 1),
        (-200, 120, 2),
        (-340, 170, 1),
        (-200, 170, 1),
        (-130, 210, 1),
    ] {
        for level in 0..stack {
            let col = c.containers[k % c.containers.len().max(1)];
            k += 1;
            w.dbox(
                x - 30,
                z - 12,
                x + 30,
                z + 12,
                level * 26,
                level * 26 + 26,
                col,
                STYLE_METAL,
            );
        }
    }
    // fuel tanks
    for x in [380, 450, 520] {
        let s = w.dbox(x - 24, 360, x + 24, 408, 0, 42, c.tank, STYLE_METAL);
        w.solids[s].mesh = Mesh::Cyl;
        w.solids[s].portal = false;
    }
    // sandbag nests inside the gate and round the tower
    w.sandbags(-110, 700, -60, 712, c);
    w.sandbags(60, 700, 110, 712, c);
    w.sandbags(150, 380, 270, 392, c);
    w.sandbags(-60, -400, 60, -388, c);
    // trucks
    for (x, z, along_x) in [(-20, 300, false), (120, -220, true), (700, 380, false)] {
        let (hx, hz) = if along_x { (30, 12) } else { (12, 30) };
        w.dbox(x - hx, z - hz, x + hx, z + hz, 4, 30, c.truck, STYLE_METAL);
        let (cx, cz) = if along_x { (x + hx + 8, z) } else { (x, z - hz - 8) };
        let s = w.dbox(cx - 9, cz - 9, cx + 9, cz + 9, 4, 24, c.truck, STYLE_PLAIN);
        w.solids[s].portal = false;
    }
    // crates about the yard
    for (x, z, s) in [
        (100, 100, 12),
        (112, 100, 12),
        (100, 112, 12),
        (-440, 300, 12),
        (240, 620, 12),
        (252, 620, 12),
        (640, 140, 14),
        (-300, -420, 12),
        (-288, -420, 12),
        (300, 140, 12),
        (640, -500, 12),
        (-500, 640, 12),
    ] {
        w.crate_(x, z, s, 0, c);
    }
    w.place("yard", 0, 0, 0);
    w.place("tanks", 450, 0, 430);
    w.place("containers", -270, 0, 150);
}

fn greenery(w: &mut World, c: &Colours) {
    // bushes inside, along the west wall and in the south-east corner
    for k in 0..8 {
        w.bush(-770, -100 + k * 50, 16);
    }
    for (x, z) in [(-700, 120), (-650, 260), (-740, 300), (-560, 600), (-430, 560)] {
        w.bush(x, z, 15);
    }
    for (x, z) in [
        (560, 560),
        (620, 600),
        (700, 560),
        (740, 680),
        (600, 720),
        (680, 760),
        (540, 660),
    ] {
        w.bush(x, z, 16);
    }
    for (x, z) in [(140, 520), (-160, 480), (380, -500), (700, -300), (420, 200)] {
        w.bush(x, z, 14);
    }
    // outside: the approaches
    for k in 0..14 {
        w.bush(-880 + (k % 2) * 40, -400 + k * 60, 18);
    }
    for k in 0..12 {
        w.bush(-700 + k * 120, 860 + (k % 3) * 30, 18);
    }
    for (x, z) in [
        (860, -200),
        (880, 120),
        (860, 520),
        (880, -600),
        (300, 940),
        (-300, 940),
    ] {
        w.bush(x, z, 18);
    }
    let grass = |w: &mut World, x0: i32, z0: i32, x1: i32, z1: i32| {
        w.grass.push(Patch {
            lo: v3(dm(x0), 0, dm(z0)),
            hi: v3(dm(x1), 0, dm(z1)),
        })
    };
    grass(w, -600, 840, -200, 980);
    grass(w, -980, -400, -840, 400);
    grass(w, 560, 560, 780, 780);
    grass(w, 200, 840, 600, 960);
    grass(w, -780, 0, -680, 280);
    // trees outside the wall
    for (k, (x, z)) in [
        (-940, -700),
        (-900, -520),
        (-960, 560),
        (-900, 700),
        (-700, 940),
        (-460, 920),
        (-100, 960),
        (120, 900),
        (520, 920),
        (700, 960),
        (-960, 120),
        (-960, -120),
        (600, -960),
        (760, -900),
    ]
    .into_iter()
    .enumerate()
    {
        let s = w.dbox(x - 3, z - 3, x + 3, z + 3, 0, 50, c.trunk, STYLE_PLAIN);
        w.solids[s].mesh = Mesh::Cyl;
        w.solids[s].portal = false;
        w.solids[s].opaque = false;
        let k = k as i32;
        w.crowns
            .push((v3(dm(x), dm(60 + (k % 3) * 6), dm(z)), dm(30 + (k % 2) * 8)));
    }
    // the landing zones
    w.place("lz_west", -920, 0, 640);
    w.place("lz_ridge", 600, 0, -920);
    w.place("lz_beach", 940, 0, -100);
    w.place("start_west", -940, 0, 300);
    w.place("start_ridge", -500, 42, -960);
    w.place("start_south", 380, 0, 960);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::int;

    pub fn world() -> World {
        build(&Palette::parse(include_str!("../../../data/palette.json")).unwrap())
    }

    #[test]
    fn a_ray_meets_the_near_face_of_a_box_with_its_outward_normal() {
        let s = Solid {
            lo: v3(int(2), 0, -ONE),
            hi: v3(int(3), int(2), ONE),
            colour: [0; 3],
            style: 0,
            mesh: Mesh::Cube,
            opaque: true,
            portal: true,
            roof: false,
        };
        let (t, n) = slab(v3(0, ONE, 0), v3(ONE, 0, 0), s.lo, s.hi).unwrap();
        assert_eq!(t, int(2));
        assert_eq!(n, v3(-ONE, 0, 0));
        assert!(
            slab(v3(0, int(3), 0), v3(ONE, 0, 0), s.lo, s.hi).is_none(),
            "over the top"
        );
        assert!(
            slab(v3(0, ONE, 0), v3(-ONE, 0, 0), s.lo, s.hi).is_none(),
            "pointing away"
        );
    }

    #[test]
    fn the_perimeter_wall_stops_you_and_the_gate_lets_you_through() {
        let w = world();
        // walking north into the south wall at x = 30 m
        let p = w.collide(v3(int(30), 0, int(80) + dm(4)), crate::balance::RADIUS, dm(18));
        assert!(
            p.z >= int(80) + dm(3) + crate::balance::RADIUS - 2,
            "pushed back out: {}",
            p.z
        );
        // the gate at x = 0 is open
        let p = w.collide(v3(0, 0, int(80)), crate::balance::RADIUS, dm(18));
        assert_eq!(p.z, int(80));
    }

    #[test]
    fn the_drain_lets_a_crawl_through_and_stops_a_crouch() {
        let w = world();
        let at = v3(-int(80), 0, -dm(290));
        assert!(
            !w.blocked(at, crate::balance::RADIUS, dm(5), dm(5)),
            "prone fits under"
        );
        assert!(
            w.blocked(at, crate::balance::RADIUS, dm(5), dm(11)),
            "crouching does not"
        );
    }

    #[test]
    fn soldiers_cannot_see_through_a_wall_but_can_through_bars() {
        let w = world();
        let outside = v3(int(30), dm(16), int(85));
        let inside = v3(int(30), dm(16), int(75));
        assert!(!w.clear(outside, inside));
        assert!(w.clear(v3(int(30), dm(16), int(70)), inside));
        // through the cage bars in the warehouse's back room
        assert!(w.clear(v3(dm(570), dm(10), dm(-250)), v3(dm(570), dm(10), dm(-270))));
    }

    #[test]
    fn the_ground_type_follows_the_regions() {
        let w = world();
        assert_eq!(w.surface_at(v3(int(-90), 0, 0)), Surface::Grass);
        assert_eq!(w.surface_at(v3(0, 0, int(30))), Surface::Concrete);
        assert_eq!(w.surface_at(v3(int(30), 0, int(-60))), Surface::Dirt);
        assert_eq!(w.surface_at(v3(int(-10), 0, int(-55))), Surface::Floor);
        assert_eq!(w.surface_at(v3(int(95), 0, 0)), Surface::Sand);
    }

    #[test]
    fn the_roofs_cover_the_rooms_and_the_yard_is_open_sky() {
        let w = world();
        assert!(w.indoors(v3(int(-10), 0, int(-55))));
        assert!(w.covered(v3(int(-10), 0, int(-55))));
        assert!(!w.indoors(v3(0, 0, 0)));
        assert!(!w.covered(v3(0, 0, 0)));
    }

    #[test]
    fn you_can_stand_on_a_crate_from_its_top_but_not_step_onto_it_from_the_ground() {
        let w = world();
        let crate_top = v3(dm(100), 0, dm(100));
        assert_eq!(w.ground(crate_top, dm(2), 0), 0, "too high to step onto");
        assert_eq!(w.ground(crate_top, dm(2), dm(12)), dm(12));
    }

    #[test]
    fn every_place_the_missions_name_is_open_ground_or_a_deck() {
        let w = world();
        for p in &w.places {
            if p.name == "cage_door" {
                continue;
            }
            assert!(
                !w.blocked(p.at, dm(3), p.at.y + dm(2), p.at.y + dm(17)),
                "{} at {:?} is inside something",
                p.name,
                p.at
            );
        }
    }
}
