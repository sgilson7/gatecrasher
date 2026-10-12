//! Portals (Splitgate's): two doors, opened by shooting a wall or a floor
//! you can see. Walk into one and you come out of the other, turned by the
//! difference between their faces, with your speed carried through: fall
//! into a floor portal and you are thrown out of the other one.
//!
//! A portal opens only on a face big enough to hold it, with room in front
//! to stand. Ordinary soldiers cannot see them; elites' lenses can (the
//! soldier module decides who looks).

use crate::balance::*;
use crate::fx::{self, ratio, ONE};
use crate::geom::{v3, V3};
use crate::world::World;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Portal {
    /// the middle of the opening, just off the face
    pub at: V3,
    /// the face's outward normal: an axis, ±x, ±z or +y
    pub n: V3,
}

impl Portal {
    pub fn floor(&self) -> bool {
        self.n.y > 0
    }

    /// The heading a person faces walking out of it (wall portals).
    pub fn heading(&self) -> i32 {
        fx::atan2(-self.n.x, -self.n.z)
    }

    /// The height of a wall portal's bottom edge, or a floor portal's face.
    pub fn bottom(&self) -> i32 {
        if self.floor() {
            self.at.y
        } else {
            self.at.y - PORTAL_H / 2
        }
    }

    /// Unit vector along the face, across the opening (wall portals).
    pub fn across(&self) -> V3 {
        v3(-self.n.z, 0, self.n.x)
    }

    /// The opening's axes: right, up and out. A floor portal's up is north.
    pub fn frame(&self) -> (V3, V3, V3) {
        let up = if self.floor() { v3(0, 0, -ONE) } else { V3::UP };
        (up.cross(self.n), up, self.n)
    }

    /// Where a ray from `o` along `d` (unit) passes through the opening,
    /// going in from the front: the distance along it.
    pub fn crossing(&self, o: V3, d: V3, max: i32) -> Option<i32> {
        let dn = d.dot(self.n);
        if dn >= 0 {
            return None;
        }
        let t = fx::div(self.at.sub(o).dot(self.n), dn);
        if t < 0 || t > max {
            return None;
        }
        let rel = o.add(d.scale(t)).sub(self.at);
        let (right, up, _) = self.frame();
        let (w, h) = if self.floor() {
            (PORTAL_FLOOR, PORTAL_FLOOR)
        } else {
            (PORTAL_W, PORTAL_H)
        };
        // the opening is an ellipse
        let x = fx::div(rel.dot(right), w / 2);
        let y = fx::div(rel.dot(up), h / 2);
        (fx::mul(x, x) + fx::mul(y, y) < ONE).then_some(t)
    }
}

/// A direction going into `from` comes out of `to` turned so: what points
/// into `from` points out of `to`, and up stays up for two wall portals.
pub fn carry_dir(from: &Portal, to: &Portal, d: V3) -> V3 {
    let (ra, ua, na) = from.frame();
    let (rb, ub, nb) = to.frame();
    // half a turn about the up axis between the two
    rb.scale(-d.dot(ra))
        .add(ub.scale(d.dot(ua)))
        .add(nb.scale(-d.dot(na)))
}

/// Where a point near `from` lands, seen from `to`.
pub fn carry_point(from: &Portal, to: &Portal, p: V3) -> V3 {
    to.at.add(carry_dir(from, to, p.sub(from.at)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fail {
    Nothing,
    TooSmall,
    Ceiling,
    NoRoom,
}

/// Opens a portal where a ray from `o` along `d` first meets a portal-able
/// face, fitted inside the face. `other` is the portal of the other colour,
/// which this one may not overlap.
pub fn place(world: &World, o: V3, d: V3, other: Option<&Portal>) -> Result<Portal, Fail> {
    let hit = world.ray(o, d, PORTAL_RANGE, |s| s.opaque).ok_or(Fail::Nothing)?;
    let n = hit.normal;
    if n.y < 0 {
        return Err(Fail::Ceiling);
    }
    let lift = ONE / 50;
    let portal = match hit.solid {
        None => Portal {
            at: v3(hit.at.x, lift, hit.at.z),
            n,
        },
        Some(i) => {
            let s = &world.solids[i];
            if !s.portal {
                return Err(Fail::TooSmall);
            }
            let fit = |x: i32, lo: i32, hi: i32, size: i32| -> Option<i32> {
                if hi - lo < size {
                    None
                } else {
                    Some(x.clamp(lo + size / 2, hi - size / 2))
                }
            };
            if n.y > 0 {
                let x = fit(hit.at.x, s.lo.x, s.hi.x, PORTAL_FLOOR).ok_or(Fail::TooSmall)?;
                let z = fit(hit.at.z, s.lo.z, s.hi.z, PORTAL_FLOOR).ok_or(Fail::TooSmall)?;
                Portal {
                    at: v3(x, s.hi.y + lift, z),
                    n,
                }
            } else {
                let y = fit(hit.at.y, s.lo.y, s.hi.y, PORTAL_H).ok_or(Fail::TooSmall)?;
                if n.x != 0 {
                    let z = fit(hit.at.z, s.lo.z, s.hi.z, PORTAL_W).ok_or(Fail::TooSmall)?;
                    let x = if n.x > 0 { s.hi.x + lift } else { s.lo.x - lift };
                    Portal { at: v3(x, y, z), n }
                } else {
                    let x = fit(hit.at.x, s.lo.x, s.hi.x, PORTAL_W).ok_or(Fail::TooSmall)?;
                    let z = if n.z > 0 { s.hi.z + lift } else { s.lo.z - lift };
                    Portal { at: v3(x, y, z), n }
                }
            }
        }
    };
    // room to come out: a person-sized space in front of it
    let (stand, h) = if portal.floor() {
        (portal.at, HEIGHT_STAND)
    } else {
        (
            v3(portal.at.x, portal.bottom(), portal.at.z).add(portal.n.scale(PORTAL_EXIT)),
            HEIGHT_CROUCH,
        )
    };
    if world.blocked(stand, RADIUS - ratio(5, 100), stand.y + ONE / 10, stand.y + h) {
        return Err(Fail::NoRoom);
    }
    if let Some(q) = other {
        if q.n == portal.n && q.at.dist(portal.at) < PORTAL_W + ONE / 4 {
            return Err(Fail::NoRoom);
        }
    }
    Ok(portal)
}

/// Whether a person at `pos` (feet) moving along `vel` (horizontal, m/s)
/// steps into the portal this tick.
pub fn enters(p: &Portal, pos: V3, vel: V3, vy: i32) -> bool {
    if p.floor() {
        let d = v3(pos.x - p.at.x, 0, pos.z - p.at.z).len();
        return d < PORTAL_FLOOR / 2 - ratio(2, 10)
            && pos.y - p.at.y < ratio(3, 10)
            && pos.y - p.at.y > -ONE
            && vy <= 0;
    }
    let rel = pos.sub(p.at);
    let along = v3(rel.x, 0, rel.z).dot(p.n);
    let side = rel.dot(p.across()).abs();
    let feet = pos.y - p.bottom();
    along < PORTAL_ENTER
        && along > -ratio(5, 10)
        && side < PORTAL_W / 2 - ratio(15, 100)
        && feet > -ratio(4, 10)
        && feet < ratio(9, 10)
        && vel.dot(p.n) < 0
}

/// Where you come out of `to` after going into `from`: feet position, the
/// heading change, the horizontal push and the upward speed.
pub struct Exit {
    pub pos: V3,
    pub turn: i32,
    pub push: V3,
    pub vy: i32,
}

pub fn exit(from: &Portal, to: &Portal, yaw: i32, vel: V3, vy: i32) -> Exit {
    // speed going in: along the wall portal's face, or falling into a floor
    let speed = if from.floor() { -vy } else { -vel.dot(from.n) }.max(0);
    if to.floor() {
        let turn = 0;
        return Exit {
            pos: to.at.add(v3(0, ratio(15, 100), 0)),
            turn,
            push: crate::geom::facing(yaw, 0).scale(ratio(15, 10)),
            vy: speed.max(int_speed(5)),
        };
    }
    let turn = if from.floor() {
        fx::wrap(to.heading() - yaw)
    } else {
        // turn so that walking into `from` becomes walking out of `to`
        fx::wrap(to.heading() - (from.heading() + fx::HALF))
    };
    Exit {
        pos: v3(to.at.x, to.bottom() + ONE / 50, to.at.z).add(to.n.scale(PORTAL_EXIT)),
        turn,
        push: to.n.scale(if from.floor() { speed } else { 0 }),
        vy: 0,
    }
}

const fn int_speed(n: i32) -> i32 {
    fx::int(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Palette;
    use crate::fx::{deg, int};
    use crate::geom::facing;
    use crate::world;

    fn w() -> World {
        world::build(&Palette::parse(include_str!("../../../data/palette.json")).unwrap())
    }

    #[test]
    fn a_portal_opens_on_the_wall_you_aim_at_facing_you() {
        let w = w();
        // standing south of the south wall, looking north at it
        let p = place(&w, v3(int(30), dm16(), int(90)), facing(0, 0), None).unwrap();
        assert_eq!(p.n, v3(0, 0, ONE));
        assert!(p.at.z > int(80));
        // its bottom on the ground: the wall is 3.6 m tall and the portal 2.4 m
        assert!(p.bottom() >= 0 && p.bottom() < int(1), "{}", p.bottom());
    }

    fn dm16() -> i32 {
        ratio(16, 10)
    }

    #[test]
    fn the_ground_takes_a_floor_portal_and_a_ceiling_takes_none() {
        let w = w();
        let p = place(&w, v3(int(10), int(2), int(10)), facing(0, -deg(45)), None).unwrap();
        assert!(p.floor());
        // inside the HQ looking straight up at the roof
        assert_eq!(
            place(&w, v3(-int(10), ONE, -int(55)), v3(0, ONE, 0), None),
            Err(Fail::Ceiling)
        );
    }

    #[test]
    fn a_crate_is_too_small_and_a_locker_will_not_take_one() {
        let w = w();
        // the crate at (10, 10), 1.2 m: from the south, at its middle
        let r = place(&w, v3(int(10), ratio(6, 10), int(14)), facing(0, 0), None);
        assert_eq!(r, Err(Fail::TooSmall));
    }

    #[test]
    fn from_the_ridge_a_portal_opens_on_the_comms_hut_just_over_the_deck() {
        let w = w();
        let eye = v3(int(33), ratio(42, 10) + ratio(16, 10), -int(94));
        let aim = v3(int(30), int(19), -int(60)).sub(eye).norm();
        let p = place(&w, eye, aim, None).unwrap();
        assert_eq!(p.n, v3(0, 0, -ONE), "{p:?}");
        let out = v3(p.at.x, p.bottom(), p.at.z).add(p.n.scale(PORTAL_EXIT));
        // you come out over the deck (16.4 m) and drop onto it
        assert!(out.y > ratio(164, 10) && out.y < int(19), "{:?}", out);
        assert!(out.z > -int(62) && out.x > int(28) && out.x < int(38), "{out:?}");
    }

    #[test]
    fn walking_into_a_wall_portal_turns_you_to_face_out_of_the_other() {
        // a portal on a wall facing south, another on a wall facing east
        let a = Portal {
            at: v3(0, int(1), 0),
            n: v3(0, 0, ONE),
        };
        let b = Portal {
            at: v3(int(20), ratio(12, 10), 0),
            n: v3(ONE, 0, 0),
        };
        // walking north (heading 0) into `a`
        let pos = v3(0, 0, ratio(4, 10));
        let vel = v3(0, 0, -int(4));
        assert!(enters(&a, pos, vel, 0));
        assert!(!enters(&a, pos, vel.neg(), 0), "walking away does not count");
        let e = exit(&a, &b, 0, vel, 0);
        // facing east now: heading 270°
        assert!(fx::wrap(e.turn - deg(270)).abs() < deg(1), "{}", e.turn);
        assert!(e.pos.x > b.at.x);
    }

    #[test]
    fn what_goes_into_one_portal_comes_out_of_the_other_turned_the_same_way_you_are() {
        let a = Portal {
            at: v3(0, int(1), 0),
            n: v3(0, 0, ONE),
        };
        let b = Portal {
            at: v3(int(20), ratio(12, 10), 0),
            n: v3(ONE, 0, 0),
        };
        // into a is out of b
        assert_eq!(carry_dir(&a, &b, a.n.neg()), b.n);
        // the turn matches walking through (heading 0 into a becomes 270°)
        let e = exit(&a, &b, 0, v3(0, 0, -int(4)), 0);
        let walk = carry_dir(&a, &b, facing(0, 0));
        assert_eq!(walk, facing(e.turn, 0), "{walk:?}");
        assert_eq!(carry_dir(&a, &b, V3::UP), V3::UP);
        // and is not a mirror: heading west into a (90°) turns 270° to north
        assert_eq!(carry_dir(&a, &b, v3(-ONE, 0, 0)), v3(0, 0, -ONE));
        // a point a metre in front of a is a metre behind b
        let p = carry_point(&a, &b, a.at.add(a.n));
        assert_eq!(p, b.at.sub(b.n));
        // floor to floor: falling in comes up out of the other
        let f = Portal {
            at: v3(0, ONE / 50, 0),
            n: V3::UP,
        };
        let g = Portal {
            at: v3(int(9), ONE / 50, 0),
            n: V3::UP,
        };
        assert_eq!(carry_dir(&f, &g, V3::UP.neg()), V3::UP);
    }

    #[test]
    fn a_ray_crosses_the_opening_from_the_front_and_misses_it_beside_or_behind() {
        let a = Portal {
            at: v3(0, int(1), 0),
            n: v3(0, 0, ONE),
        };
        let north = v3(0, 0, -ONE);
        assert_eq!(a.crossing(v3(0, int(1), int(5)), north, int(50)), Some(int(5)));
        assert_eq!(
            a.crossing(v3(int(2), int(1), int(5)), north, int(50)),
            None,
            "beside"
        );
        assert_eq!(
            a.crossing(v3(0, int(1), -int(5)), north.neg(), int(50)),
            None,
            "behind"
        );
        assert_eq!(
            a.crossing(v3(0, int(1), int(5)), north, int(3)),
            None,
            "out of range"
        );
    }

    #[test]
    fn falling_into_a_floor_portal_throws_you_out_of_the_other_as_fast() {
        let a = Portal {
            at: v3(0, ONE / 50, 0),
            n: V3::UP,
        };
        let b = Portal {
            at: v3(int(20), ONE / 50, 0),
            n: V3::UP,
        };
        assert!(enters(&a, v3(0, 0, 0), V3::ZERO, -int(9)));
        let e = exit(&a, &b, 0, V3::ZERO, -int(9));
        assert_eq!(e.vy, int(9));
        let wall = Portal {
            at: v3(0, int(3), 0),
            n: v3(0, 0, ONE),
        };
        let e = exit(&a, &wall, 0, V3::ZERO, -int(9));
        assert_eq!(e.push, v3(0, 0, int(9)), "out of a wall, flung forward");
    }
}
