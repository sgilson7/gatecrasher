//! The operative: how she moves (MGS5's stances, sprint, vault and dive),
//! the noise each movement makes, and the camo index (MGS3's) that says how
//! well she blends in.

use crate::balance::*;
use crate::fx::{self, ratio, ONE};
use crate::geom::{facing, v3, V3};
use crate::mission::Uniform;
use crate::world::{Surface, World};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stance {
    Stand,
    Crouch,
    Prone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gait {
    Still,
    Walk,
    Run,
    Sprint,
}

/// The walk cycle after moving at `speed` for `dt`: a full turn of the
/// phase is one stride, whose length grows with speed (`balance::STRIDE`).
pub fn stride(phase: i32, speed: i32, dt: i32, base: i32, grow: i32) -> i32 {
    let length = base + fx::mul(grow, speed.max(0));
    (phase + fx::div(fx::mul(speed.max(0), dt), length.max(ONE / 10))) & (fx::TURN - 1)
}

/// Hanging from a ledge, Uncharted style: the leap up to it, the hang
/// (shimmy left and right), and the climb up onto the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ledge {
    Grab { from: V3, t: i32 },
    Hang,
    Up { to: V3, t: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hang {
    /// flat unit vector into the wall
    pub dir: V3,
    /// the height of the ledge
    pub top: i32,
    /// her feet while hanging
    pub at: V3,
    pub stage: Ledge,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub pos: V3,
    pub vy: i32,
    pub yaw: i32,
    pub stance: Stance,
    pub gait: Gait,
    /// horizontal velocity this tick, m/s
    pub vel: V3,
    /// a shove that fades: a dive, a portal's fling
    pub push: V3,
    pub airborne: bool,
    pub fall_top: i32,
    /// a vault: from, to, time so far, and how high the arc lifts
    pub vault: Option<(V3, V3, i32, i32)>,
    pub hang: Option<Hang>,
    pub dive: i32,
    pub step_t: i32,
    pub walk_phase: i32,
    pub boxed: bool,
    pub locker: Option<usize>,
}

/// What one tick of movement produced.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Moved {
    /// a footstep or a landing: where, and how far it carries
    pub noise: Option<(V3, i32)>,
    pub fell_to_death: bool,
    pub vaulted: bool,
    pub step: bool,
}

impl Player {
    pub fn new(pos: V3, yaw: i32) -> Player {
        Player {
            pos,
            vy: 0,
            yaw,
            stance: Stance::Crouch,
            gait: Gait::Still,
            vel: V3::ZERO,
            push: V3::ZERO,
            airborne: false,
            fall_top: pos.y,
            vault: None,
            hang: None,
            dive: 0,
            step_t: 0,
            walk_phase: 0,
            boxed: false,
            locker: None,
        }
    }

    pub fn height(&self) -> i32 {
        if self.boxed {
            return ratio(10, 10);
        }
        match self.stance {
            Stance::Stand => HEIGHT_STAND,
            Stance::Crouch => HEIGHT_CROUCH,
            Stance::Prone => HEIGHT_PRONE,
        }
    }

    /// The point a soldier must see.
    pub fn chest(&self) -> V3 {
        self.pos.add(v3(0, self.height() * 3 / 4, 0))
    }

    /// Whether there is headroom to take a taller stance.
    pub fn room_for(&self, world: &World, s: Stance) -> bool {
        let h = match s {
            Stance::Stand => HEIGHT_STAND,
            Stance::Crouch => HEIGHT_CROUCH,
            Stance::Prone => HEIGHT_PRONE,
        };
        !world.blocked(
            self.pos,
            RADIUS - ONE / 20,
            self.pos.y + STEP + ONE / 50,
            self.pos.y + h,
        )
    }

    pub fn set_stance(&mut self, world: &World, s: Stance) {
        if s == self.stance || self.room_for(world, s) {
            self.stance = s;
        }
    }

    /// One tick. `wish` is the flat direction asked for (unit or zero),
    /// `speed` the most the situation allows, `face` a heading to hold
    /// (aiming) or None to turn toward the movement.
    pub fn step(
        &mut self,
        world: &World,
        wish: V3,
        sprint: bool,
        cap: i32,
        face: Option<i32>,
        dt: i32,
    ) -> Moved {
        let mut out = Moved::default();
        if self.hang.is_some() {
            self.hang_step(world, wish, dt);
            return out;
        }
        if let Some((from, to, t, arc)) = self.vault {
            let t = t + dt;
            let k = fx::div(t.min(VAULT_TIME), VAULT_TIME);
            let lift = fx::mul(fx::sin(fx::mul(k, fx::HALF)), arc);
            self.pos = from.lerp(to, k).add(v3(0, lift, 0));
            self.vault = if t >= VAULT_TIME {
                None
            } else {
                Some((from, to, t, arc))
            };
            if self.vault.is_none() {
                self.pos = to;
                self.fall_top = to.y;
            }
            self.vel = V3::ZERO;
            return out;
        }
        let moving = wish != V3::ZERO;
        let speed = if !moving {
            0
        } else if self.boxed {
            SPEED_BOX
        } else {
            match self.stance {
                Stance::Prone => SPEED_PRONE,
                Stance::Crouch => SPEED_CROUCH,
                Stance::Stand if sprint => SPEED_SPRINT,
                Stance::Stand => SPEED_RUN,
            }
        }
        .min(cap);
        self.gait = if !moving {
            Gait::Still
        } else if speed >= SPEED_SPRINT {
            Gait::Sprint
        } else if speed >= SPEED_RUN {
            Gait::Run
        } else {
            Gait::Walk
        };
        let diving = self.dive > 0;
        if diving {
            self.dive -= dt;
        }
        // turn the body
        let want = face.or(if moving {
            Some(fx::atan2(-wish.x, -wish.z))
        } else {
            None
        });
        if let Some(h) = want {
            let d = fx::wrap(h - self.yaw);
            let max = fx::mul(TURN_RATE, dt);
            self.yaw = (self.yaw + d.clamp(-max, max)) & (fx::TURN - 1);
        }
        self.vel = wish.scale(speed).add(self.push);
        let mut p = self.pos.add(self.vel.scale(dt));
        p = world.collide(p, RADIUS, self.height());
        // the push fades, faster on the ground
        let fade = if self.airborne { ratio(1, 2) } else { int_(6) };
        let pl = self.push.len();
        if pl > 0 {
            let left = (pl - fx::mul(fade, dt)).max(0);
            self.push = self.push.scale(fx::div(left, pl));
        }
        // up and down
        let ground = world.ground(p, RADIUS * 2 / 3, p.y);
        if p.y > ground + ONE / 50 || self.vy > 0 {
            if !self.airborne {
                self.airborne = true;
                self.fall_top = p.y;
            }
            self.vy -= fx::mul(GRAVITY, dt);
            p.y += fx::mul(self.vy, dt);
            self.fall_top = self.fall_top.max(p.y);
            if p.y <= ground {
                p.y = ground;
                let drop = self.fall_top - ground;
                self.vy = 0;
                self.airborne = false;
                if drop > FALL_DEADLY {
                    out.fell_to_death = true;
                } else if drop > int_(2) {
                    out.noise = Some((p, NOISE_LAND));
                }
            }
        } else {
            p.y = ground;
            self.vy = 0;
            self.airborne = false;
        }
        // a dive ends face down where she lands
        if diving && (!self.airborne || self.dive <= 0) {
            self.dive = 0;
            self.stance = Stance::Prone;
        }
        self.vel = v3(fx::div(p.x - self.pos.x, dt), 0, fx::div(p.z - self.pos.z, dt));
        self.pos = p;
        // footsteps
        if moving && !self.airborne {
            self.walk_phase = stride(self.walk_phase, speed, dt, STRIDE, STRIDE_GROW);
            self.step_t -= dt;
            if self.step_t <= 0 {
                self.step_t = STEP_EVERY;
                out.step = true;
                let base = match (self.gait, self.stance) {
                    (Gait::Sprint, _) => NOISE_SPRINT,
                    (_, Stance::Prone) => NOISE_PRONE,
                    (_, Stance::Crouch) => NOISE_SNEAK,
                    (Gait::Run, _) => NOISE_RUN,
                    _ => NOISE_WALK,
                };
                let r = match world.surface_at(p) {
                    Surface::Floor => base * 6 / 5,
                    Surface::Grass | Surface::Sand => base * 4 / 5,
                    _ => base,
                };
                if r > 0 && out.noise.is_none() {
                    out.noise = Some((p, r));
                }
            }
        } else {
            self.step_t = 0;
        }
        out
    }

    /// The ledge in front of her feet `at` along `dir`: how far ahead its
    /// face is and how high its top, if it is a top she could reach (no higher
    /// than a hang) with room above it.
    pub fn ledge(world: &World, at: V3, dir: V3) -> Option<(i32, i32)> {
        let mut d = ratio(3, 10);
        while d <= int_(1) {
            let probe = at.add(dir.scale(d));
            let top = world.ground(probe, ONE / 20, at.y + HANG_MAX - STEP);
            let above = world.blocked(probe, ONE / 20, top + ONE / 50, top + HEIGHT_CROUCH);
            if world.blocked(probe, ONE / 20, at.y + STEP, at.y + HANG_MAX + HEIGHT_CROUCH) {
                return if top > at.y + STEP && !above {
                    Some((d, top))
                } else {
                    None
                };
            }
            d += ONE / 20;
        }
        None
    }

    /// Space: vault a ledge ahead (over it if it is low and thin), leap up and
    /// grab a higher one, climb up from a hang, dive out of a sprint, or stand
    /// up. Returns true for a vault or a grab (they make a little noise).
    pub fn jump(&mut self, world: &World) -> bool {
        if let Some(h) = self.hang {
            if h.stage == Ledge::Hang {
                self.climb_up(world);
            }
            return false;
        }
        if self.dive > 0 {
            return false;
        }
        if self.airborne || self.vault.is_some() || self.boxed {
            return false;
        }
        let dir = facing(self.yaw, 0);
        if let Some((d, top)) = Player::ledge(world, self.pos, dir) {
            let rise = top - self.pos.y;
            if rise <= MANTLE_OVER && self.gait != Gait::Still {
                // on the move, over a low, thin thing: find where it ends and land beyond
                let mut e = d + ONE / 10;
                while e <= d + MANTLE_DEPTH {
                    let probe = self.pos.add(dir.scale(e));
                    if world.ground(probe, ONE / 20, top) < top - STEP {
                        let land = self.pos.add(dir.scale(e + RADIUS + ONE / 10));
                        let g = world.ground(land, RADIUS, top);
                        let to = v3(land.x, g, land.z);
                        if g < top - STEP && !world.blocked(to, RADIUS, g + STEP, g + HEIGHT_STAND) {
                            let mid = (self.pos.y + g) / 2;
                            let arc = (top + ratio(35, 100) - mid).max(ratio(5, 10));
                            self.vault = Some((self.pos, to, 0, arc));
                            self.stance = Stance::Stand;
                            return true;
                        }
                        break;
                    }
                    e += ONE / 10;
                }
            }
            if rise <= VAULT_MAX {
                let ahead = self.pos.add(dir.scale(d + RADIUS + ONE / 10));
                let to = v3(ahead.x, top, ahead.z);
                if !world.blocked(to, RADIUS, top + ONE / 20, top + HEIGHT_CROUCH) {
                    self.vault = Some((self.pos, to, 0, ratio(5, 10)));
                    if self.stance == Stance::Prone {
                        self.stance = Stance::Crouch;
                    }
                    return true;
                }
            } else if self.stance != Stance::Prone {
                // too high to vault: leap and hang from it
                let foot = self.pos.add(dir.scale(d - RADIUS - ONE / 50));
                let at = v3(foot.x, top - HANG_REACH, foot.z);
                if !world.blocked(at, RADIUS - ONE / 10, at.y + ONE / 10, top - ONE / 20) {
                    self.hang = Some(Hang {
                        dir,
                        top,
                        at,
                        stage: Ledge::Grab { from: self.pos, t: 0 },
                    });
                    self.stance = Stance::Stand;
                    self.yaw = fx::atan2(-dir.x, -dir.z);
                    return true;
                }
            }
        }
        if self.gait == Gait::Sprint && self.stance == Stance::Stand {
            self.push = facing(self.yaw, 0).scale(DIVE_SPEED);
            self.vy = DIVE_LIFT;
            self.dive = DIVE_TIME;
            return false;
        }
        if self.stance != Stance::Stand {
            self.set_stance(world, Stance::Stand);
        }
        false
    }

    pub fn hanging(&self) -> bool {
        self.hang.is_some()
    }

    /// From a hang: up onto the top, if there is room to crouch there.
    pub fn climb_up(&mut self, world: &World) -> bool {
        let Some(mut h) = self.hang else {
            return false;
        };
        let to = h.at.add(h.dir.scale(RADIUS * 2 + ratio(15, 100)));
        let to = v3(to.x, h.top, to.z);
        if world.blocked(to, RADIUS - ONE / 20, h.top + ONE / 50, h.top + HEIGHT_CROUCH) {
            return false;
        }
        h.stage = Ledge::Up { to, t: 0 };
        self.hang = Some(h);
        true
    }

    /// Lets go of the ledge.
    pub fn let_go(&mut self) {
        if let Some(h) = self.hang {
            if h.stage == Ledge::Hang {
                self.hang = None;
                self.pos = h.at.sub(h.dir.scale(ONE / 10));
                self.fall_top = self.pos.y;
            }
        }
    }

    fn hang_step(&mut self, world: &World, wish: V3, dt: i32) {
        let Some(mut h) = self.hang else {
            return;
        };
        self.vel = V3::ZERO;
        self.vy = 0;
        self.push = V3::ZERO;
        self.dive = 0;
        self.airborne = false;
        self.gait = Gait::Still;
        match h.stage {
            Ledge::Grab { from, t } => {
                let t = t + dt;
                let k = fx::div(t.min(GRAB_TIME), GRAB_TIME);
                self.pos = from.lerp(h.at, k);
                h.stage = if t >= GRAB_TIME {
                    Ledge::Hang
                } else {
                    Ledge::Grab { from, t }
                };
            }
            Ledge::Hang => {
                let right = v3(-h.dir.z, 0, h.dir.x);
                let along = fx::mul(wish.x, right.x) + fx::mul(wish.z, right.z);
                let into = fx::mul(wish.x, h.dir.x) + fx::mul(wish.z, h.dir.z);
                if into > ratio(7, 10) {
                    self.hang = Some(h);
                    self.climb_up(world);
                    return;
                }
                if into < -ratio(7, 10) {
                    self.hang = Some(h);
                    self.let_go();
                    return;
                }
                if along.abs() > ONE / 2 {
                    let step = right.scale(fx::mul(SHIMMY, dt) * along.signum());
                    let next = h.at.add(step);
                    // the ledge must go on, at the same height, with nothing on it
                    let probe = next.add(h.dir.scale(RADIUS + ONE / 10));
                    let top = world.ground(probe, ONE / 20, h.top - STEP);
                    let ok = (top - h.top).abs() < ONE / 10
                        && !world.blocked(probe, ONE / 20, h.top + ONE / 50, h.top + HEIGHT_CROUCH)
                        && !world.blocked(next, RADIUS - ONE / 10, next.y + ONE / 10, h.top - ONE / 20);
                    if ok {
                        h.at = next;
                        self.walk_phase = (self.walk_phase + fx::mul(fx::TURN / 2, dt)) & (fx::TURN - 1);
                    }
                }
                self.pos = h.at;
            }
            Ledge::Up { to, t } => {
                let t = t + dt;
                let k = fx::div(t.min(CLIMB_TIME), CLIMB_TIME);
                let rise = ratio(6, 10);
                self.pos = if k < rise {
                    let a = fx::div(k, rise);
                    v3(h.at.x, h.at.y + fx::mul(h.top - h.at.y, a), h.at.z)
                } else {
                    let a = fx::div(k - rise, ONE - rise);
                    let flat = v3(h.at.x, h.top, h.at.z).lerp(to, a);
                    v3(flat.x, h.top, flat.z)
                };
                if t >= CLIMB_TIME {
                    self.pos = to;
                    self.fall_top = to.y;
                    self.hang = None;
                    self.stance = Stance::Crouch;
                    self.set_stance(world, Stance::Stand);
                    return;
                }
                h.stage = Ledge::Up { to, t };
            }
        }
        self.fall_top = self.pos.y;
        self.hang = Some(h);
    }
}

const fn int_(n: i32) -> i32 {
    fx::int(n)
}

/// What the camo index is made of, for the HUD and for sight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Camo {
    pub index: i32,
    pub surface: Surface,
}

/// MGS3's camo index in percent: the uniform against the ground underfoot,
/// the stance, the movement, the cover; a box or active camo instead.
pub fn camo_index(p: &Player, world: &World, uniform: &Uniform, cloak: bool) -> Camo {
    let surface = if world.indoors(p.pos) {
        Surface::Floor
    } else {
        world.surface_at(p.pos)
    };
    let moving = p.gait != Gait::Still;
    let mut index = if p.boxed {
        if moving {
            CAMO_BOX_MOVING
        } else {
            CAMO_BOX_STILL
        }
    } else {
        let stance = match p.stance {
            Stance::Stand => CAMO_STAND,
            Stance::Crouch => CAMO_CROUCH,
            Stance::Prone => CAMO_PRONE,
        };
        let motion = match p.gait {
            Gait::Still => CAMO_STILL,
            Gait::Walk => CAMO_WALK,
            Gait::Run => CAMO_RUN,
            Gait::Sprint => CAMO_SPRINT,
        };
        let low = p.stance != Stance::Stand;
        let cover = if world.in_bush(p.pos) {
            if low {
                CAMO_BUSH_LOW
            } else {
                CAMO_BUSH_STAND
            }
        } else if world.in_grass(p.pos) {
            match p.stance {
                Stance::Prone => CAMO_GRASS_PRONE,
                Stance::Crouch => CAMO_GRASS_CROUCH,
                Stance::Stand => 0,
            }
        } else {
            0
        };
        let shade = if world.indoors(p.pos) { CAMO_INDOORS } else { 0 };
        uniform.matches[surface as usize] + stance + motion + cover + shade
    };
    if cloak {
        index = index.max(CAMO_ACTIVE);
    }
    Camo {
        index: index.clamp(CAMO_MIN, CAMO_MAX),
        surface,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Palette;
    use crate::fx::int;
    use crate::mission::parse_uniforms;
    use crate::world;

    /// Strides a second at a speed: one second of movement in 60 ticks.
    fn cadence(speed: i32) -> i32 {
        let mut phase = 0;
        let mut turns = 0;
        for _ in 0..60 {
            let next = stride(phase, speed, ONE / 60, STRIDE, STRIDE_GROW);
            if next < phase {
                turns += 1;
            }
            phase = next;
        }
        turns * 100 + phase * 100 / fx::TURN
    }

    #[test]
    fn legs_stride_at_a_human_pace_walking_running_and_sprinting() {
        // people take about one stride (two steps) a second walking and
        // about one and a half running; never the blur of four a metre
        for (speed, lo, hi) in [(WALK, 70, 120), (SPEED_RUN, 110, 160), (SPEED_SPRINT, 120, 170)] {
            let c = cadence(speed);
            assert!(
                c >= lo && c <= hi,
                "{} m/s: {} strides a second (×100)",
                speed / 65536,
                c
            );
        }
    }

    fn w() -> World {
        world::build(&Palette::parse(include_str!("../../../data/palette.json")).unwrap())
    }

    fn uniforms() -> Vec<Uniform> {
        parse_uniforms(include_str!("../../../data/uniforms.json")).unwrap()
    }

    fn run(p: &mut Player, w: &World, wish: V3, sprint: bool, ticks: usize) -> Vec<Moved> {
        (0..ticks)
            .map(|_| p.step(w, wish, sprint, int(99), None, DT))
            .collect()
    }

    #[test]
    fn each_stance_moves_at_its_own_speed() {
        let w = w();
        for (stance, sprint, speed) in [
            (Stance::Stand, true, SPEED_SPRINT),
            (Stance::Stand, false, SPEED_RUN),
            (Stance::Crouch, false, SPEED_CROUCH),
            (Stance::Prone, false, SPEED_PRONE),
        ] {
            let mut p = Player::new(v3(0, 0, int(30)), 0);
            p.stance = stance;
            run(&mut p, &w, v3(0, 0, -ONE), sprint, 60);
            let went = int(30) - p.pos.z;
            assert!((went - speed).abs() < ONE / 20, "{stance:?}: {went} vs {speed}");
        }
    }

    #[test]
    fn sprinting_is_heard_much_further_than_crawling() {
        let w = w();
        let loudest = |stance, sprint| {
            let mut p = Player::new(v3(0, 0, int(30)), 0);
            p.stance = stance;
            run(&mut p, &w, v3(0, 0, -ONE), sprint, 120)
                .iter()
                .filter_map(|m| m.noise.map(|n| n.1))
                .max()
                .unwrap_or(0)
        };
        assert_eq!(loudest(Stance::Stand, true), NOISE_SPRINT, "on concrete");
        assert_eq!(loudest(Stance::Prone, false), 0);
        assert!(loudest(Stance::Crouch, false) < NOISE_RUN);
    }

    #[test]
    fn the_uniform_that_matches_the_ground_hides_you_best() {
        let w = w();
        let u = uniforms();
        let mut p = Player::new(v3(-int(90), 0, 0), 0);
        p.stance = Stance::Crouch;
        // on the western grass: woodland beats splitter
        let wood = camo_index(&p, &w, &u[0], false).index;
        let split = camo_index(&p, &w, &u[1], false).index;
        assert!(wood > split + 40, "{wood} vs {split}");
        // on the concrete road it is the other way round
        p.pos = v3(0, 0, int(30));
        assert!(camo_index(&p, &w, &u[1], false).index > camo_index(&p, &w, &u[0], false).index);
    }

    #[test]
    fn crawling_in_a_bush_or_cloaking_raises_the_index_and_sprinting_lowers_it() {
        let w = w();
        let u = uniforms();
        let mut p = Player::new(v3(-int(77), 0, -int(10)), 0);
        p.stance = Stance::Prone;
        let bush = camo_index(&p, &w, &u[0], false).index;
        p.pos = v3(0, 0, int(30));
        let open = camo_index(&p, &w, &u[0], false).index;
        assert!(bush > open + 40, "{bush} vs {open}");
        p.stance = Stance::Stand;
        p.gait = Gait::Sprint;
        assert!(camo_index(&p, &w, &u[0], false).index < 0);
        assert_eq!(camo_index(&p, &w, &u[0], true).index, CAMO_ACTIVE);
    }

    #[test]
    fn space_vaults_a_crate_and_out_of_a_sprint_dives() {
        let w = w();
        // facing the crate at (10, 10) from the south
        let mut p = Player::new(v3(int(10), 0, int(11)), 0);
        p.stance = Stance::Stand;
        assert!(p.jump(&w));
        run(&mut p, &w, V3::ZERO, false, 40);
        assert_eq!(p.pos.y, ratio(12, 10), "on top of it");
        let mut p = Player::new(v3(0, 0, int(40)), 0);
        p.stance = Stance::Stand;
        run(&mut p, &w, v3(0, 0, -ONE), true, 10);
        assert!(!p.jump(&w));
        run(&mut p, &w, V3::ZERO, false, 40);
        assert_eq!(p.stance, Stance::Prone);
    }

    /// Flat ground and one wall across x, `h` high, from z = −`deep` to 0.
    fn wall(h: i32, deep: i32) -> World {
        let mut w = World::default();
        w.add(v3(-int(5), 0, -deep), v3(int(5), h, 0), [0, 0, 0], 0);
        w
    }

    #[test]
    fn on_the_move_space_vaults_clean_over_a_low_thin_wall() {
        let w = wall(ratio(1, 1), ratio(5, 10));
        // walking north (−z) at it from the south
        let mut p = Player::new(v3(0, 0, ratio(8, 10)), 0);
        p.stance = Stance::Stand;
        run(&mut p, &w, v3(0, 0, -ONE), false, 5);
        assert!(p.jump(&w));
        run(&mut p, &w, V3::ZERO, false, 60);
        assert!(p.pos.z < -ratio(5, 10), "landed beyond it: z {}", p.pos.z);
        assert_eq!(p.pos.y, 0);
        // standing still, the same press puts her on top instead
        let mut p = Player::new(v3(0, 0, ratio(8, 10)), 0);
        p.stance = Stance::Stand;
        assert!(p.jump(&w));
        run(&mut p, &w, V3::ZERO, false, 60);
        assert_eq!(p.pos.y, ONE, "on top");
    }

    #[test]
    fn a_ledge_too_high_to_vault_is_leapt_at_hung_from_shimmied_along_and_climbed() {
        let top = ratio(23, 10);
        let w = wall(top, int(3));
        let mut p = Player::new(v3(0, 0, ratio(7, 10)), 0);
        p.stance = Stance::Stand;
        assert!(p.jump(&w), "grabs the ledge");
        run(&mut p, &w, V3::ZERO, false, 30);
        assert!(
            matches!(
                p.hang,
                Some(Hang {
                    stage: Ledge::Hang,
                    ..
                })
            ),
            "{:?}",
            p.hang
        );
        assert_eq!(p.pos.y, top - HANG_REACH, "hanging by the hands");
        // she hangs: no fall
        run(&mut p, &w, V3::ZERO, false, 120);
        assert!(p.hanging());
        // shimmy east along it
        let x0 = p.pos.x;
        run(&mut p, &w, v3(ONE, 0, 0), false, 60);
        assert!(
            p.pos.x - x0 > ratio(9, 10),
            "moved {} along the ledge",
            p.pos.x - x0
        );
        // up and over the top
        run(&mut p, &w, v3(0, 0, -ONE), false, 1);
        run(&mut p, &w, V3::ZERO, false, 70);
        assert!(!p.hanging());
        assert_eq!(p.pos.y, top, "on top of the wall");
        assert_eq!(p.stance, Stance::Stand);
    }

    #[test]
    fn she_lets_go_of_a_ledge_and_cannot_reach_one_too_high() {
        let w = wall(ratio(23, 10), int(3));
        let mut p = Player::new(v3(0, 0, ratio(7, 10)), 0);
        p.stance = Stance::Stand;
        p.jump(&w);
        run(&mut p, &w, V3::ZERO, false, 30);
        p.let_go();
        run(&mut p, &w, V3::ZERO, false, 60);
        assert!(!p.hanging());
        assert_eq!(p.pos.y, 0);
        let w = wall(ratio(35, 10), int(3));
        let mut p = Player::new(v3(0, 0, ratio(7, 10)), 0);
        p.stance = Stance::Stand;
        assert!(!p.jump(&w));
        assert!(!p.hanging());
    }

    #[test]
    fn a_long_fall_kills_and_a_short_one_is_only_loud() {
        let w = w();
        let mut p = Player::new(v3(0, int(20), int(30)), 0);
        assert!(run(&mut p, &w, V3::ZERO, false, 200)
            .iter()
            .any(|m| m.fell_to_death));
        let mut p = Player::new(v3(0, int(4), int(30)), 0);
        let moves = run(&mut p, &w, V3::ZERO, false, 200);
        assert!(!moves.iter().any(|m| m.fell_to_death));
        assert!(moves.iter().any(|m| m.noise.is_some()));
    }
}
