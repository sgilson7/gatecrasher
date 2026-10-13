//! Shapes for everything drawn: the zone (once), and each frame the people,
//! the mule, portals, sight cones, tracers and balloons. People are a dozen
//! boxes posed by a few angles; the look is the TikZ art's: flat fills and a
//! plum outline (the ink pass).

use crate::balance::{DIVE_LIFT, GRAVITY, KICK_EVERY};
use crate::content::{Palette, Rgb};
use crate::fx::{self, deg, int, ratio, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{boxed, spanning, Batch, LAYER_HOLE, LAYER_INKED, LAYER_PLAIN};
use crate::rng::Rng;
use crate::world::{self, World, STYLE_GLOW, STYLE_GROUND, STYLE_PLAIN, STYLE_PORTAL};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    /// standing or walking; `aim` holds the pistol out
    Upright {
        walk: i32,
        moving: bool,
        aim: bool,
    },
    Crouch {
        walk: i32,
        moving: bool,
        aim: bool,
    },
    /// face down, along the heading
    Prone {
        walk: i32,
    },
    /// on his back: asleep, stunned, dead
    Lying,
    /// tied up on his back, rope round him
    Bound,
    /// in a headlock: leaning back
    Held,
    /// held up at gunpoint: both hands high
    HandsUp,
    /// astride the mule: legs apart and forward
    Ride {
        aim: bool,
    },
    /// kicking a sleeping buddy awake; `t` is how long he has been at it
    Kick {
        t: i32,
    },
    /// hanging from a ledge by both hands, legs swinging as she shimmies
    Hang {
        walk: i32,
    },
    /// in the air in a dive out of a sprint, `t` after leaving the ground:
    /// tipping from a run's lean to flat, arms out ahead
    Dive {
        t: i32,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Look {
    pub body: Rgb,
    pub dark: Rgb,
    pub skin: Rgb,
    pub boots: Rgb,
    /// helmet, beret, hair or nothing
    pub hat: Hat,
    pub goggles: Option<Rgb>,
    pub style: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum Hat {
    Helmet(Rgb),
    Beret(Rgb),
    Hair(Rgb),
}

#[allow(clippy::too_many_arguments)]
fn part(b: &mut Batch, base: &Affine, pivot: V3, angle: i32, centre: V3, size: V3, rgb: Rgb, style: i32) {
    let a = base
        .then(&Affine::translate(pivot))
        .then(&Affine::rot_x(angle))
        .then(&Affine::translate(centre.sub(pivot)))
        .then(&Affine::scale(size));
    b.push(LAYER_INKED, Mesh::Cube, &a, rgb, style);
}

/// A person standing at `pos` (feet) facing `yaw`.
pub fn person(b: &mut Batch, pos: V3, yaw: i32, pose: Pose, look: &Look) {
    let mut base = Affine::translate(pos).then(&Affine::rot_y(yaw));
    let (walk, moving, aim, crouch) = match pose {
        Pose::Upright { walk, moving, aim } => (walk, moving, aim, false),
        Pose::Crouch { walk, moving, aim } => (walk, moving, aim, true),
        Pose::Prone { walk } => (walk, true, false, false),
        Pose::Ride { aim } => (0, false, aim, false),
        _ => (0, false, false, false),
    };
    match pose {
        Pose::Prone { .. } => {
            // the whole figure tipped forward onto the ground
            base = base
                .then(&Affine::translate(v3(0, ratio(14, 100), 0)))
                .then(&Affine::rot_x(-fx::QUARTER));
        }
        Pose::Lying | Pose::Bound => {
            base = base
                .then(&Affine::translate(v3(0, ratio(14, 100), 0)))
                .then(&Affine::rot_x(fx::QUARTER));
        }
        Pose::Held => {
            base = base.then(&Affine::rot_x(deg(12)));
        }
        Pose::Dive { t } => {
            // turning about the hip from a run's lean to flat over the time in
            // the air, sinking and sliding so that the last frame is Prone's
            let air = fx::div(2 * DIVE_LIFT, GRAVITY);
            let f = fx::div(t, air).clamp(0, ONE);
            let lean = deg(20) + fx::mul(fx::QUARTER - deg(20), f);
            let hip = ratio(84, 100);
            base = base
                .then(&Affine::translate(v3(
                    0,
                    hip - fx::mul(hip - ratio(14, 100), f),
                    -fx::mul(hip, f),
                )))
                .then(&Affine::rot_x(-lean))
                .then(&Affine::translate(v3(0, -hip, 0)));
        }
        _ => {}
    }
    let swing = if moving {
        fx::mul(deg(32), fx::sin(walk))
    } else {
        0
    };
    // a kick: the right leg swings forward and back, once a kick
    let kick = match pose {
        Pose::Kick { t } => {
            let phase = fx::div(t % KICK_EVERY, KICK_EVERY) * (fx::TURN / ONE);
            fx::mul(deg(55), fx::sin(phase & (fx::TURN - 1)).max(0))
        }
        _ => 0,
    };
    let drop = if crouch { ratio(42, 100) } else { 0 };
    let hip = v3(0, ratio(84, 100) - drop, 0);
    // legs
    let astride = matches!(pose, Pose::Ride { .. });
    for (side, sgn) in [(-1, 1), (1, -1)] {
        let x = if astride { ratio(26, 100) } else { ratio(11, 100) } * side;
        if crouch {
            // thigh forward, shin down
            part(
                b,
                &base,
                v3(x, hip.y, 0),
                fx::QUARTER - deg(10) + swing * sgn / 3,
                v3(x, hip.y - ratio(22, 100), 0),
                v3(ratio(16, 100), ratio(44, 100), ratio(18, 100)),
                look.body,
                look.style,
            );
            part(
                b,
                &base,
                v3(x, ratio(21, 100), -ratio(36, 100)),
                0,
                v3(x, ratio(21, 100), -ratio(36, 100)),
                v3(ratio(15, 100), ratio(42, 100), ratio(17, 100)),
                look.dark,
                look.style,
            );
        } else {
            let swing = if astride {
                deg(35)
            } else if side == 1 {
                swing * sgn + kick
            } else {
                swing * sgn
            };
            part(
                b,
                &base,
                v3(x, hip.y, 0),
                swing,
                v3(x, ratio(44, 100), 0),
                v3(ratio(16, 100), ratio(80, 100), ratio(18, 100)),
                look.body,
                look.style,
            );
            part(
                b,
                &base,
                v3(x, hip.y, 0),
                swing,
                v3(x, ratio(6, 100), -ratio(4, 100)),
                v3(ratio(17, 100), ratio(12, 100), ratio(26, 100)),
                look.boots,
                0,
            );
        }
    }
    let lean = if crouch { deg(18) } else { 0 };
    let chest = v3(0, hip.y + ratio(30, 100), 0);
    // torso, tipped forward when crouching
    part(
        b,
        &base,
        hip,
        -lean,
        chest,
        v3(ratio(42, 100), ratio(60, 100), ratio(25, 100)),
        look.body,
        look.style,
    );
    let neck = v3(0, hip.y + ratio(62, 100), -fx::mul(fx::sin(lean), ratio(62, 100)));
    let head = neck.add(v3(0, ratio(14, 100), 0));
    part(
        b,
        &base,
        head,
        0,
        head,
        v3(ratio(22, 100), ratio(24, 100), ratio(22, 100)),
        look.skin,
        0,
    );
    match look.hat {
        Hat::Helmet(c) => part(
            b,
            &base,
            head,
            0,
            head.add(v3(0, ratio(12, 100), 0)),
            v3(ratio(29, 100), ratio(11, 100), ratio(31, 100)),
            c,
            0,
        ),
        Hat::Beret(c) => part(
            b,
            &base,
            head,
            deg(-8),
            head.add(v3(ratio(3, 100), ratio(13, 100), 0)),
            v3(ratio(27, 100), ratio(7, 100), ratio(27, 100)),
            c,
            0,
        ),
        Hat::Hair(c) => {
            part(
                b,
                &base,
                head,
                0,
                head.add(v3(0, ratio(9, 100), ratio(3, 100))),
                v3(ratio(24, 100), ratio(9, 100), ratio(24, 100)),
                c,
                0,
            );
            part(
                b,
                &base,
                head,
                0,
                head.add(v3(0, -ratio(4, 100), ratio(12, 100))),
                v3(ratio(16, 100), ratio(22, 100), ratio(6, 100)),
                c,
                0,
            );
        }
    }
    if let Some(lens) = look.goggles {
        part(
            b,
            &base,
            head,
            0,
            head.add(v3(0, ratio(2, 100), -ratio(12, 100))),
            v3(ratio(24, 100), ratio(7, 100), ratio(4, 100)),
            lens,
            STYLE_GLOW,
        );
    }
    // arms
    let shoulder_y = hip.y + ratio(54, 100);
    for (side, sgn) in [(-1, -1), (1, 1)] {
        let x = ratio(28, 100) * side;
        let sh = v3(x, shoulder_y, -fx::mul(fx::sin(lean), ratio(54, 100)));
        let angle = if aim && side == 1 {
            fx::QUARTER
        } else if matches!(pose, Pose::HandsUp) {
            fx::HALF - deg(12)
        } else if matches!(pose, Pose::Hang { .. }) {
            // both hands on the ledge above her head
            fx::HALF - deg(8)
        } else if matches!(pose, Pose::Dive { .. }) {
            // reaching ahead, over the head, as she flies
            fx::HALF - deg(20)
        } else if matches!(pose, Pose::Kick { .. }) {
            // arms out a little for balance
            deg(14) * sgn
        } else if matches!(pose, Pose::Bound) {
            0
        } else {
            swing * sgn
        };
        part(
            b,
            &base,
            sh,
            angle,
            sh.sub(v3(0, ratio(28, 100), 0)),
            v3(ratio(12, 100), ratio(56, 100), ratio(12, 100)),
            look.body,
            look.style,
        );
        if aim && side == 1 {
            // the pistol, at the end of the arm
            part(
                b,
                &base,
                sh,
                angle,
                sh.sub(v3(0, ratio(60, 100), ratio(5, 100))),
                v3(ratio(6, 100), ratio(22, 100), ratio(10, 100)),
                [ONE / 6, ONE / 7, ONE / 5],
                0,
            );
        }
    }
    if matches!(pose, Pose::Bound) {
        for k in 0..3 {
            part(
                b,
                &base,
                hip,
                0,
                v3(0, hip.y + ratio(10 + 20 * k, 100), 0),
                v3(ratio(46, 100), ratio(4, 100), ratio(28, 100)),
                [ONE * 89 / 100, ONE * 80 / 100, ONE * 60 / 100],
                0,
            );
        }
    }
}

/// A rifle lying on the ground where a soldier dropped it.
pub fn rifle(b: &mut Batch, at: V3, yaw: i32) {
    let base = Affine::translate(at).then(&Affine::rot_y(yaw + fx::QUARTER / 3));
    let gun = [ONE / 6, ONE / 7, ONE / 5];
    b.push(
        LAYER_INKED,
        Mesh::Cube,
        &base.then(&boxed(
            v3(0, ratio(4, 100), 0),
            v3(ratio(8, 100), ratio(8, 100), ratio(95, 100)),
        )),
        gun,
        0,
    );
    b.push(
        LAYER_INKED,
        Mesh::Cube,
        &base.then(&boxed(
            v3(0, ratio(4, 100), ratio(10, 100)),
            v3(ratio(7, 100), ratio(7, 100), ratio(20, 100)),
        )),
        gun,
        0,
    );
}

/// A person draped over a shoulder (carried) or a mule's back.
pub fn draped(b: &mut Batch, at: V3, yaw: i32, look: &Look) {
    let base = Affine::translate(at)
        .then(&Affine::rot_y(yaw + fx::QUARTER))
        .then(&Affine::rot_z(fx::QUARTER));
    let lift = |c: V3, s: V3, rgb: Rgb, st: i32, b: &mut Batch| {
        b.push(LAYER_INKED, Mesh::Cube, &base.then(&boxed(c, s)), rgb, st);
    };
    lift(
        v3(0, ratio(10, 100), 0),
        v3(ratio(40, 100), ratio(60, 100), ratio(24, 100)),
        look.body,
        look.style,
        b,
    );
    lift(
        v3(0, -ratio(38, 100), 0),
        v3(ratio(22, 100), ratio(22, 100), ratio(22, 100)),
        look.skin,
        0,
        b,
    );
    lift(
        v3(ratio(10, 100), ratio(70, 100), 0),
        v3(ratio(16, 100), ratio(70, 100), ratio(18, 100)),
        look.body,
        look.style,
        b,
    );
    lift(
        v3(-ratio(10, 100), ratio(70, 100), 0),
        v3(ratio(16, 100), ratio(70, 100), ratio(18, 100)),
        look.dark,
        look.style,
        b,
    );
}

pub struct MuleLook {
    pub coat: Rgb,
    pub light: Rgb,
    pub dark: Rgb,
    pub bandana: Rgb,
    pub saddle: Rgb,
}

/// The Memphis Mule.
pub fn mule(b: &mut Batch, pos: V3, yaw: i32, walk: i32, moving: bool, charging: bool, l: &MuleLook) {
    let base = Affine::translate(pos).then(&Affine::rot_y(yaw));
    let swing = if moving {
        fx::mul(deg(if charging { 40 } else { 26 }), fx::sin(walk))
    } else {
        0
    };
    let leg = |b: &mut Batch, x: i32, z: i32, a: i32| {
        part(
            b,
            &base,
            v3(x, ratio(95, 100), z),
            a,
            v3(x, ratio(50, 100), z),
            v3(ratio(15, 100), ratio(92, 100), ratio(16, 100)),
            l.coat,
            0,
        );
        part(
            b,
            &base,
            v3(x, ratio(95, 100), z),
            a,
            v3(x, ratio(5, 100), z),
            v3(ratio(17, 100), ratio(10, 100), ratio(18, 100)),
            l.dark,
            0,
        );
    };
    let w = ratio(22, 100);
    leg(b, -w, -ratio(55, 100), swing);
    leg(b, w, -ratio(55, 100), -swing);
    leg(b, -w, ratio(55, 100), -swing);
    leg(b, w, ratio(55, 100), swing);
    // barrel, belly, saddlebags
    part(
        b,
        &base,
        V3::ZERO,
        0,
        v3(0, ratio(118, 100), 0),
        v3(ratio(66, 100), ratio(60, 100), ratio(150, 100)),
        l.coat,
        0,
    );
    part(
        b,
        &base,
        V3::ZERO,
        0,
        v3(0, ratio(92, 100), 0),
        v3(ratio(52, 100), ratio(14, 100), ratio(110, 100)),
        l.light,
        0,
    );
    part(
        b,
        &base,
        V3::ZERO,
        0,
        v3(0, ratio(150, 100), ratio(5, 100)),
        v3(ratio(70, 100), ratio(8, 100), ratio(60, 100)),
        l.saddle,
        0,
    );
    for s in [-1, 1] {
        part(
            b,
            &base,
            V3::ZERO,
            0,
            v3(ratio(40, 100) * s, ratio(120, 100), ratio(10, 100)),
            v3(ratio(16, 100), ratio(36, 100), ratio(44, 100)),
            l.saddle,
            0,
        );
    }
    // neck and head, lowered when he charges
    let dip = if charging { deg(50) } else { 0 };
    let nk = v3(0, ratio(140, 100), -ratio(65, 100));
    part(
        b,
        &base,
        nk,
        deg(35) - dip,
        nk.add(v3(0, ratio(20, 100), -ratio(12, 100))),
        v3(ratio(30, 100), ratio(62, 100), ratio(34, 100)),
        l.coat,
        0,
    );
    part(
        b,
        &base,
        nk,
        deg(35) - dip,
        nk.add(v3(0, ratio(8, 100), -ratio(8, 100))),
        v3(ratio(36, 100), ratio(14, 100), ratio(38, 100)),
        l.bandana,
        0,
    );
    let hd = nk.add(v3(0, ratio(52, 100), -ratio(36, 100)));
    part(
        b,
        &base,
        nk,
        -dip,
        hd,
        v3(ratio(28, 100), ratio(30, 100), ratio(56, 100)),
        l.coat,
        0,
    );
    part(
        b,
        &base,
        nk,
        -dip,
        hd.add(v3(0, -ratio(4, 100), -ratio(30, 100))),
        v3(ratio(24, 100), ratio(22, 100), ratio(14, 100)),
        l.light,
        0,
    );
    for s in [-1, 1] {
        // the ears: long, a little splayed
        part(
            b,
            &base,
            nk,
            -dip,
            hd.add(v3(ratio(10, 100) * s, ratio(36, 100), ratio(14, 100))),
            v3(ratio(8, 100), ratio(46, 100), ratio(12, 100)),
            l.coat,
            0,
        );
    }
    part(
        b,
        &base,
        v3(0, ratio(135, 100), ratio(75, 100)),
        deg(-25) + swing / 2,
        v3(0, ratio(105, 100), ratio(85, 100)),
        v3(ratio(7, 100), ratio(60, 100), ratio(7, 100)),
        l.dark,
        0,
    );
}

/// A portal: a glowing oval rim and its dark swirl.
/// A portal: a glowing rim, and the opening (`k`'s own layer: a swirl, or a
/// window when the page draws the view through it).
#[allow(clippy::too_many_arguments)]
pub fn portal(b: &mut Batch, k: usize, at: V3, n: V3, rim: Rgb, w: i32, h: i32, open: i32) {
    let rot = if n.y > 0 {
        Affine::IDENTITY
    } else if n.z != 0 {
        Affine::rot_x(if n.z > 0 { fx::QUARTER } else { -fx::QUARTER })
    } else {
        Affine::rot_z(if n.x > 0 { -fx::QUARTER } else { fx::QUARTER })
    };
    let (sw, sh) = if n.y > 0 { (w, w) } else { (w, h) };
    let sw = fx::mul(sw, open);
    let sh = fx::mul(sh, open);
    let base = Affine::translate(at).then(&rot);
    b.push(
        LAYER_INKED,
        Mesh::Cyl,
        &base.then(&Affine::scale(v3(sw, ratio(4, 100), sh))),
        rim,
        STYLE_GLOW,
    );
    let inner = base
        .then(&Affine::translate(v3(0, ratio(2, 100), 0)))
        .then(&Affine::scale(v3(sw * 4 / 5, ratio(4, 100), sh * 4 / 5)));
    b.push(LAYER_HOLE + k, Mesh::Cyl, &inner, rim, STYLE_PORTAL);
}

/// A sight cone on the ground: thin slices out to wherever a wall stops it.
#[allow(clippy::too_many_arguments)]
pub fn cone(
    b: &mut Batch,
    world: &World,
    eyes: V3,
    ground: i32,
    gaze: i32,
    half: i32,
    range: i32,
    slices: i32,
    rgb: Rgb,
    alpha: i32,
) {
    let step = half * 2 / slices;
    let tan = fx::div(fx::sin(step / 2), fx::cos(step / 2));
    for k in 0..slices {
        let a = gaze - half + step * k + step / 2;
        let d = crate::geom::facing(a, 0);
        let len = world
            .ray(eyes, d, range, |s| s.opaque)
            .map(|h| h.t)
            .unwrap_or(range);
        if len < ONE / 2 {
            continue;
        }
        let width = fx::mul(tan, len) * 2 + ONE / 50;
        let t = Affine::translate(v3(eyes.x, ground + ratio(6, 100), eyes.z))
            .then(&Affine::rot_y(a))
            .then(&Affine::scale(v3(width, ONE, len)));
        b.tint(Mesh::Tri, &t, rgb, alpha);
    }
}

pub fn tracer(b: &mut Batch, from: V3, to: V3, rgb: Rgb) {
    let d = to.sub(from);
    let len = d.len();
    if len == 0 {
        return;
    }
    let a = Affine::translate(from.add(d.scale(ONE / 2)))
        .then(&Affine::looking(d.norm()))
        .then(&Affine::scale(v3(ratio(3, 100), ratio(3, 100), len)));
    b.push(LAYER_PLAIN, Mesh::Cube, &a, rgb, STYLE_GLOW);
}

/// A fulton balloon at `top` with its line down to `bottom`.
pub fn balloon(b: &mut Batch, top: V3, bottom: V3, rgb: Rgb) {
    b.push(
        LAYER_INKED,
        Mesh::Sphere,
        &boxed(top, v3(ratio(14, 10), ratio(17, 10), ratio(14, 10))),
        rgb,
        0,
    );
    let mid = top.add(bottom).scale(ONE / 2);
    let len = top.y - bottom.y;
    if len > 0 {
        b.push(
            LAYER_PLAIN,
            Mesh::Cube,
            &boxed(mid, v3(ratio(3, 100), len, ratio(3, 100))),
            [ONE * 9 / 10; 3],
            0,
        );
    }
}

/// A charge stuck to the mech, its light blinking.
pub fn charge(b: &mut Batch, at: V3, blink: bool, rgb: Rgb) {
    b.push(
        LAYER_INKED,
        Mesh::Cube,
        &boxed(
            at.add(v3(0, ratio(1, 10), 0)),
            v3(ratio(4, 10), ratio(2, 10), ratio(3, 10)),
        ),
        rgb,
        0,
    );
    if blink {
        b.push(
            LAYER_PLAIN,
            Mesh::Sphere,
            &boxed(
                at.add(v3(0, ratio(25, 100), 0)),
                v3(ratio(12, 100), ratio(12, 100), ratio(12, 100)),
            ),
            rgb,
            STYLE_GLOW,
        );
    }
}

/// A charge going off `t` seconds ago: a fireball that swells for two
/// seconds, then a fire that flickers on.
pub fn blast(b: &mut Batch, at: V3, t: i32, fire: Rgb, smoke: Rgb) {
    let swell = ONE * 2;
    if t < swell {
        let k = fx::div(t, swell);
        let r = fx::mul(int(11), k) + ONE;
        b.push(
            LAYER_PLAIN,
            Mesh::Sphere,
            &boxed(at, v3(r, r, r)),
            fire,
            STYLE_GLOW,
        );
    }
    let flick = fx::sin((fx::mul(t, int(9)) * (fx::TURN / ONE)) & (fx::TURN - 1));
    let r = int(2) + fx::mul(ratio(5, 10), flick);
    b.push(
        LAYER_PLAIN,
        Mesh::Sphere,
        &boxed(at.add(v3(0, ONE, 0)), v3(r, r * 3 / 2, r)),
        fire,
        STYLE_GLOW,
    );
    // smoke climbing out of it
    for k in 0..3 {
        let rise = (t / 2 + int(k * 4)) % int(12);
        let r = int(2) + rise / 4;
        b.push(
            LAYER_INKED,
            Mesh::Sphere,
            &boxed(at.add(v3(0, int(3) + rise, 0)), v3(r, r, r)),
            smoke,
            0,
        );
    }
}

/// The helicopter: cabin, tail, skids, and a turning rotor. `at` is under its skids.
pub fn helicopter(b: &mut Batch, at: V3, yaw: i32, rotor: i32, body: Rgb, dark: Rgb) {
    let base = Affine::translate(at).then(&Affine::rot_y(yaw));
    let part = |b: &mut Batch, mesh: Mesh, c: V3, s: V3, rgb: Rgb| {
        b.push(LAYER_INKED, mesh, &base.then(&boxed(c, s)), rgb, 0);
    };
    part(
        b,
        Mesh::Sphere,
        v3(0, ratio(19, 10), 0),
        v3(int(3), ratio(26, 10), int(6)),
        body,
    );
    part(
        b,
        Mesh::Cube,
        v3(0, ratio(22, 10), int(6)),
        v3(ratio(7, 10), ratio(7, 10), int(7)),
        body,
    );
    part(
        b,
        Mesh::Cube,
        v3(0, ratio(33, 10), ratio(95, 10)),
        v3(ratio(2, 10), int(2), ratio(12, 10)),
        dark,
    );
    part(
        b,
        Mesh::Cube,
        v3(0, ratio(19, 10), -ratio(22, 10)),
        v3(ratio(24, 10), ratio(9, 10), ratio(4, 10)),
        dark,
    );
    for side in [-1, 1] {
        part(
            b,
            Mesh::Cube,
            v3(side * ratio(12, 10), ratio(1, 10), 0),
            v3(ratio(15, 100), ratio(15, 100), int(5)),
            dark,
        );
        part(
            b,
            Mesh::Cube,
            v3(side * ratio(12, 10), ratio(6, 10), 0),
            v3(ratio(1, 10), ratio(9, 10), ratio(1, 10)),
            dark,
        );
    }
    part(
        b,
        Mesh::Cube,
        v3(0, ratio(34, 10), 0),
        v3(ratio(3, 10), ratio(4, 10), ratio(3, 10)),
        dark,
    );
    for k in 0..2 {
        let blade = base
            .then(&Affine::translate(v3(0, ratio(37, 10), 0)))
            .then(&Affine::rot_y(rotor + k * fx::QUARTER))
            .then(&Affine::scale(v3(ratio(3, 10), ratio(6, 100), int(12))));
        b.push(LAYER_PLAIN, Mesh::Cube, &blade, dark, 0);
    }
}

/// A cardboard box over a crouching person.
pub fn cardboard(b: &mut Batch, pos: V3, yaw: i32, rgb: Rgb) {
    let base = Affine::translate(pos).then(&Affine::rot_y(yaw));
    b.push(
        LAYER_INKED,
        Mesh::Cube,
        &base.then(&boxed(
            v3(0, ratio(52, 100), 0),
            v3(ratio(90, 100), ratio(100, 100), ratio(110, 100)),
        )),
        rgb,
        world::STYLE_CRATE,
    );
    // a hand-hole and a flap
    let dark = [rgb[0] * 2 / 3, rgb[1] * 2 / 3, rgb[2] * 2 / 3];
    b.push(
        LAYER_PLAIN,
        Mesh::Cube,
        &base.then(&boxed(
            v3(0, ratio(80, 100), -ratio(555, 1000)),
            v3(ratio(24, 100), ratio(8, 100), ratio(2, 100)),
        )),
        dark,
        0,
    );
}

/// Every shape of the zone that never moves.
pub fn scenery(world: &World, p: &Palette) -> Batch {
    let mut b = Batch::new();
    // the sea, the island's ground, then each region's ground over it
    b.push(
        LAYER_PLAIN,
        Mesh::Ground,
        &boxed(v3(0, -ratio(4, 10), 0), v3(int(1400), ONE, int(1400))),
        p.get("sea"),
        STYLE_GROUND,
    );
    let ground_rgb = |s: world::Surface| match s {
        world::Surface::Grass => p.get("grass"),
        world::Surface::Dirt => p.get("dirt"),
        world::Surface::Concrete => p.get("concrete"),
        world::Surface::Floor => p.get("floor"),
        world::Surface::Sand => p.get("sand"),
    };
    for (k, r) in world.regions.iter().enumerate() {
        let y = ratio(k as i32 + 1, 400);
        let lo = v3(r.lo.x.max(-world::HALF_ZONE), y, r.lo.z.max(-world::HALF_ZONE));
        let hi = v3(r.hi.x.min(world::HALF_ZONE), y, r.hi.z.min(world::HALF_ZONE));
        let size = hi.sub(lo);
        let a = boxed(
            v3(lo.x + size.x / 2, y, lo.z + size.z / 2),
            v3(size.x, ONE, size.z),
        );
        b.push(LAYER_PLAIN, Mesh::Ground, &a, ground_rgb(r.surface), STYLE_GROUND);
    }
    // the island's edge: a low bank down to the sea
    for (c, s) in [
        (
            v3(0, -ratio(2, 10), world::HALF_ZONE + int(3)),
            v3(int(212), ratio(4, 10), int(6)),
        ),
        (
            v3(0, -ratio(2, 10), -world::HALF_ZONE - int(3)),
            v3(int(212), ratio(4, 10), int(6)),
        ),
        (
            v3(world::HALF_ZONE + int(3), -ratio(2, 10), 0),
            v3(int(6), ratio(4, 10), int(200)),
        ),
        (
            v3(-world::HALF_ZONE - int(3), -ratio(2, 10), 0),
            v3(int(6), ratio(4, 10), int(200)),
        ),
    ] {
        b.push(LAYER_PLAIN, Mesh::Cube, &boxed(c, s), p.get("sand"), STYLE_GROUND);
    }
    for s in &world.solids {
        if s.hi.y <= s.lo.y {
            continue;
        }
        b.push(LAYER_INKED, s.mesh, &spanning(s.lo, s.hi), s.colour, s.style);
    }
    // bushes: a knot of blobs
    let mut rng = Rng::new(77);
    for (k, bush) in world.bushes.iter().enumerate() {
        for j in 0..4 {
            let a = fx::TURN * j / 4 + rng.range(0, 4000);
            let off = fx::mul(bush.r, ratio(45, 100));
            let c = v3(
                bush.at.x + fx::mul(fx::cos(a), off),
                bush.h / 2 + rng.range(0, ONE / 5),
                bush.at.z + fx::mul(fx::sin(a), off),
            );
            let r = fx::mul(bush.r, ratio(rng.range(95, 125), 100));
            b.push(
                LAYER_INKED,
                Mesh::Blob,
                &boxed(c, v3(r, bush.h, r)),
                p.pick("bush", k + j as usize),
                0,
            );
        }
    }
    // tall grass: blades
    for (k, g) in world.grass.iter().enumerate() {
        let (w, d) = ((g.hi.x - g.lo.x) / ONE, (g.hi.z - g.lo.z) / ONE);
        let n = (w * d * 2 / 3).min(900);
        for j in 0..n {
            let x = g.lo.x + rng.range(0, w * 100) * ONE / 100;
            let z = g.lo.z + rng.range(0, d * 100) * ONE / 100;
            let h = ratio(rng.range(55, 95), 100);
            let a = Affine::translate(v3(x, h / 2, z))
                .then(&Affine::rot_y(rng.range(0, fx::TURN)))
                .then(&Affine::scale(v3(ratio(35, 100), h, ratio(10, 100))));
            b.push(
                LAYER_PLAIN,
                Mesh::Cone,
                &a,
                p.pick("grass_tall", k + j as usize),
                STYLE_PLAIN,
            );
        }
    }
    for (k, (c, r)) in world.crowns.iter().enumerate() {
        b.push(
            LAYER_INKED,
            Mesh::Blob,
            &boxed(*c, v3(*r * 2, *r * 3 / 2, *r * 2)),
            p.pick("crown", k),
            0,
        );
    }
    // clouds over the sea, as in the art
    for k in 0..14 {
        let a = fx::TURN * k / 14 + rng.range(0, 2000);
        let dist = int(320 + rng.range(0, 140));
        let at = v3(
            fx::mul(fx::cos(a), dist),
            int(70 + rng.range(0, 50)),
            fx::mul(fx::sin(a), dist),
        );
        for j in 0..3 {
            let c = at.add(v3(int(j * 14 - 14), int(rng.range(0, 6)), 0));
            let r = int(16 + rng.range(0, 10));
            b.push(
                LAYER_INKED,
                Mesh::Blob,
                &boxed(c, v3(r * 2, r, r * 2)),
                p.get("sky_hi"),
                STYLE_GLOW,
            );
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> Look {
        Look {
            body: [ONE / 2; 3],
            dark: [ONE / 3; 3],
            skin: [ONE; 3],
            boots: [0; 3],
            hat: Hat::Helmet([ONE / 4; 3]),
            goggles: Some([0, ONE, ONE]),
            style: 0,
        }
    }

    #[test]
    fn a_standing_person_is_about_one_point_eight_metres_and_a_prone_one_lies_low() {
        let tops = |pose: Pose| {
            let mut b = Batch::new();
            person(&mut b, V3::ZERO, 0, pose, &look());
            let (v, _) = b.finish();
            // each instance's translation y (value 7) plus half its height at most
            v.chunks(16)
                .map(|c| c[7] + c[5].abs() / 2 + c[9].abs() / 2 + c[4].abs() / 2)
                .max()
                .unwrap()
        };
        let stand = tops(Pose::Upright {
            walk: 0,
            moving: false,
            aim: false,
        });
        assert!(stand > ratio(17, 10) && stand < ratio(20, 10), "{stand}");
        let prone = tops(Pose::Prone { walk: 0 });
        assert!(prone < ratio(6, 10), "{prone}");
    }

    #[test]
    fn the_zone_draws_every_solid_and_some_grass() {
        let p = Palette::parse(include_str!("../../../data/palette.json")).unwrap();
        let w = world::build(&p);
        let b = scenery(&w, &p);
        assert!(b.count() > w.solids.len() + 500, "{}", b.count());
    }
}
