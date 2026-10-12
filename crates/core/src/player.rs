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
    pub vault: Option<(V3, V3, i32)>,
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
        if let Some((from, to, t)) = self.vault {
            let t = t + dt;
            let k = fx::div(t.min(VAULT_TIME), VAULT_TIME);
            let lift = fx::mul(fx::sin(fx::mul(k, fx::HALF)), ratio(5, 10));
            self.pos = from.lerp(to, k).add(v3(0, lift, 0));
            self.vault = if t >= VAULT_TIME {
                None
            } else {
                Some((from, to, t))
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
        if self.dive > 0 {
            self.dive -= dt;
            if self.dive <= 0 {
                self.stance = Stance::Prone;
            }
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
        self.vel = v3(fx::div(p.x - self.pos.x, dt), 0, fx::div(p.z - self.pos.z, dt));
        self.pos = p;
        // footsteps
        if moving && !self.airborne {
            self.walk_phase = (self.walk_phase + fx::mul(speed, dt) * 4) & (fx::TURN - 1);
            self.step_t -= dt;
            if self.step_t <= 0 {
                self.step_t = STEP_EVERY;
                out.step = true;
                let base = match (self.gait, self.stance) {
                    (Gait::Sprint, _) => NOISE_SPRINT,
                    (_, Stance::Prone) => NOISE_PRONE,
                    (_, Stance::Crouch) => NOISE_CROUCH,
                    (Gait::Run, _) => NOISE_RUN,
                    _ => NOISE_CROUCH,
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

    /// Space: vault a ledge ahead if there is one, dive out of a sprint,
    /// or stand up. Returns true for a vault.
    pub fn jump(&mut self, world: &World) -> bool {
        if self.airborne || self.vault.is_some() || self.boxed {
            return false;
        }
        let ahead = self.pos.add(facing(self.yaw, 0).scale(ratio(8, 10)));
        let top = world.ground(ahead, RADIUS / 2, self.pos.y + VAULT_MAX - STEP);
        if top > self.pos.y + STEP && top <= self.pos.y + VAULT_MAX {
            let to = v3(ahead.x, top, ahead.z);
            if !world.blocked(to, RADIUS, top + ONE / 20, top + HEIGHT_CROUCH) {
                self.vault = Some((self.pos, to, 0));
                if self.stance == Stance::Prone {
                    self.stance = Stance::Crouch;
                }
                return true;
            }
        }
        if self.gait == Gait::Sprint && self.stance == Stance::Stand {
            self.push = facing(self.yaw, 0).scale(DIVE_SPEED);
            self.dive = DIVE_TIME;
            return false;
        }
        if self.stance != Stance::Stand {
            self.set_stance(world, Stance::Stand);
        }
        false
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
