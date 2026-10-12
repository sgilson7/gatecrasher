//! A soldier: what he sees and hears, and what he does about it.
//!
//! The model is MGS5's, as the brief asks. Sight builds an awareness meter
//! (0–100) faster the closer and plainer you are; the camo index scales how
//! far he can see you. At 30 he is suspicious and turns to look; at 60 he
//! walks over to check; at 100 he has spotted you. Sound sends him to where
//! it came from. A body, a portal (if he wears the lenses) or a balloon
//! sends him too. Spotting you starts reflex mode in the game; then combat,
//! in which every soldier who can see you aims and fires.

use crate::balance::*;
use crate::fx::{self, deg, int, ONE};
use crate::geom::{facing, v3, V3};
use crate::nav::{self, Nav};
use crate::rng::Rng;
use crate::world::World;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Life {
    Awake,
    /// a dart in the body: on his feet a moment longer
    Dart(i32),
    Asleep(i32),
    Stunned(i32),
    Dead,
    /// tied up: never wakes by himself; another soldier can untie him
    Bound,
    /// taken away by a fulton balloon
    Gone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mind {
    Patrol,
    Suspicious {
        at: V3,
        t: i32,
    },
    Investigate {
        at: V3,
        t: i32,
    },
    Search {
        at: V3,
        t: i32,
    },
    Combat,
    /// walking to a downed soldier to wake or untie him, then doing it
    Help {
        buddy: usize,
        t: i32,
    },
    /// walking to a portal to shut it
    Portal {
        which: usize,
        t: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Normal,
    Caution(i32),
    Evasion(i32),
    Combat(i32),
}

#[derive(Debug, Clone)]
pub struct Soldier {
    pub pos: V3,
    pub yaw: i32,
    /// where his eyes point, relative to his body
    pub look: i32,
    pub life: Life,
    pub mind: Mind,
    pub awareness: i32,
    pub route: Vec<(V3, i32)>,
    pub route_i: usize,
    pub wait: i32,
    pub path: Vec<V3>,
    pub path_goal: Option<V3>,
    pub perch: bool,
    pub home_yaw: i32,
    pub elite: bool,
    pub officer: bool,
    pub target: bool,
    pub knows: Vec<usize>,
    pub talked: bool,
    pub marked: bool,
    pub mark_t: i32,
    pub aim: i32,
    pub fire_cd: i32,
    pub held: bool,
    pub carried: bool,
    pub on_mule: bool,
    pub locker: Option<usize>,
    /// bodies he has already reacted to, so one body is one reaction
    pub seen_bodies: Vec<usize>,
    pub moving: bool,
    pub walk_phase: i32,
    pub id: usize,
    pub name_seed: u64,
}

/// What a soldier can perceive of the player this tick, made by the game.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub pos: V3,
    /// the point a soldier must see: the chest, at the stance's height
    pub chest: V3,
    /// how plainly he can be seen, percent (100 − camo index)
    pub vis: i32,
    pub hidden: bool,
    pub moving: bool,
    pub sprinting: bool,
    pub stance_hit: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Body {
    pub idx: usize,
    pub at: V3,
    pub dead: bool,
    pub bound: bool,
}

pub struct Ctx<'a> {
    pub world: &'a World,
    pub nav: &'a Nav,
    pub player: Target,
    pub phase: Phase,
    pub lkp: V3,
    pub bodies: &'a [Body],
    pub portals: [Option<V3>; 2],
    pub balloons: &'a [V3],
    pub tick: u32,
    pub reflex: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// awareness reached 100
    Spotted(usize),
    /// awareness passed the suspicious line (for the "?" sound)
    Noticed(usize),
    /// he sees you in combat: the last known position moves
    Sees(V3),
    Shot {
        from: V3,
        hit: bool,
    },
    FoundDead(usize),
    Woke {
        by: usize,
        buddy: usize,
    },
    ShutPortal(usize),
}

pub const EYE: i32 = fx::ratio(16, 10);

impl Soldier {
    pub fn new(id: usize, route: Vec<(V3, i32)>, yaw: i32, perch: bool) -> Soldier {
        let pos = route.first().map(|r| r.0).unwrap_or(V3::ZERO);
        Soldier {
            pos,
            yaw,
            look: 0,
            life: Life::Awake,
            mind: Mind::Patrol,
            awareness: 0,
            route,
            route_i: 0,
            wait: 0,
            path: Vec::new(),
            path_goal: None,
            perch,
            home_yaw: yaw,
            elite: false,
            officer: false,
            target: false,
            knows: Vec::new(),
            talked: false,
            marked: false,
            mark_t: 0,
            aim: 0,
            fire_cd: 0,
            held: false,
            carried: false,
            on_mule: false,
            locker: None,
            seen_bodies: Vec::new(),
            moving: false,
            walk_phase: 0,
            id,
            name_seed: id as u64,
        }
    }

    pub fn awake(&self) -> bool {
        matches!(self.life, Life::Awake | Life::Dart(_))
    }

    /// Lying on the ground: asleep, stunned, dead or tied up.
    pub fn down(&self) -> bool {
        matches!(
            self.life,
            Life::Asleep(_) | Life::Stunned(_) | Life::Dead | Life::Bound
        )
    }

    /// Free to act: awake, not in your arms, not tied to the mule.
    pub fn free(&self) -> bool {
        self.awake() && !self.held && !self.carried && !self.on_mule
    }

    pub fn eyes(&self) -> V3 {
        self.pos.add(v3(0, EYE, 0))
    }

    pub fn gaze(&self) -> i32 {
        self.yaw + self.look
    }

    /// The sight range for the phase, before camouflage.
    pub fn sight(phase: Phase) -> i32 {
        match phase {
            Phase::Normal => SIGHT_NORMAL,
            Phase::Caution(_) | Phase::Evasion(_) => SIGHT_CAUTION,
            Phase::Combat(_) => SIGHT_COMBAT,
        }
    }

    /// Whether a point is inside his fan of sight and in the open.
    pub fn can_see(&self, world: &World, p: V3, range: i32, wide: bool) -> bool {
        let eyes = self.eyes();
        let d = p.sub(eyes);
        let dist = d.len();
        if dist > range {
            return false;
        }
        let bearing = fx::atan2(-d.x, -d.z);
        let off = fx::wrap(bearing - self.gaze()).abs();
        let fan = if dist < NEAR_SIGHT || wide {
            FAN_NEAR
        } else {
            FAN_FAR
        };
        if off > fan {
            return false;
        }
        world.clear(eyes, p)
    }

    /// One tick of sight on the player; returns whether he sees you.
    pub fn watch(&mut self, ctx: &Ctx, dt: i32, out: &mut Vec<Event>) -> bool {
        let t = &ctx.player;
        if t.hidden {
            return false;
        }
        let combat = matches!(ctx.phase, Phase::Combat(_));
        let range = fx::mul(Soldier::sight(ctx.phase), fx::div(t.vis.clamp(5, 160), 100));
        let eyes = self.eyes();
        let dist = t.chest.dist(eyes);
        let mut sees = self.can_see(ctx.world, t.chest, range, combat);
        // a man right behind him is felt, unless he is still and in a box
        if !sees && dist < PRESENCE && (t.moving || t.vis > 40) && ctx.world.clear(eyes, t.chest) {
            sees = true;
        }
        if !sees {
            return false;
        }
        if combat {
            self.awareness = int(100);
            out.push(Event::Sees(t.pos));
            return true;
        }
        let q = fx::div((range - dist).max(0), range.max(1));
        let rate = int(NOTICE_BASE) + NOTICE_GAIN * fx::mul(q, q);
        let before = self.awareness;
        self.awareness = (self.awareness + fx::mul(rate, dt)).min(int(100));
        if before < int(SUSPICIOUS_AT) && self.awareness >= int(SUSPICIOUS_AT) {
            out.push(Event::Noticed(self.id));
        }
        if self.awareness >= int(SPOTTED_AT) {
            out.push(Event::Spotted(self.id));
        } else if self.awareness >= int(INVESTIGATE_AT) {
            if !matches!(self.mind, Mind::Investigate { .. }) {
                self.mind = Mind::Investigate { at: t.pos, t: 0 };
                self.path_goal = None;
            } else if let Mind::Investigate { at, .. } = &mut self.mind {
                *at = t.pos;
            }
        } else if self.awareness >= int(SUSPICIOUS_AT)
            && matches!(self.mind, Mind::Patrol | Mind::Search { .. })
        {
            self.mind = Mind::Suspicious {
                at: t.pos,
                t: LOOK_AROUND,
            };
        }
        true
    }

    /// A sound reaches him from `at`.
    pub fn hear(&mut self, at: V3, gunshot: bool) {
        if !self.free() {
            return;
        }
        match self.mind {
            Mind::Combat => {}
            Mind::Help { .. } | Mind::Portal { .. } if !gunshot => {}
            _ => {
                self.awareness = self.awareness.max(int(40));
                self.mind = Mind::Investigate { at, t: 0 };
                self.path_goal = None;
            }
        }
    }

    pub fn alert(&mut self) {
        if self.free() {
            self.mind = Mind::Combat;
            self.awareness = int(100);
            self.path_goal = None;
        }
    }

    /// Back to normal duty once combat or caution ends.
    pub fn stand_down(&mut self, searching: Option<V3>) {
        if !self.free() {
            return;
        }
        self.aim = 0;
        self.path_goal = None;
        self.mind = match searching {
            Some(at) => Mind::Search { at, t: SEARCH_TIME },
            None => Mind::Patrol,
        };
        self.awareness = self.awareness.min(int(20));
    }

    /// Turns the body toward a heading at the soldier's turning rate.
    fn turn_to(&mut self, heading: i32, dt: i32) {
        let d = fx::wrap(heading - self.yaw);
        let step = fx::mul(SOLDIER_TURN, dt);
        self.yaw += d.clamp(-step, step);
        self.yaw &= fx::TURN - 1;
    }

    fn face(&mut self, p: V3, dt: i32) {
        let d = p.sub(self.pos);
        if d.len_xz() > ONE / 4 {
            self.turn_to(fx::atan2(-d.x, -d.z), dt);
        }
    }

    /// Walks toward `goal` along the nav grid; returns true on arrival.
    fn walk_to(&mut self, ctx: &Ctx, goal: V3, speed: i32, dt: i32) -> bool {
        if self.perch {
            return true;
        }
        let flat = v3(goal.x, 0, goal.z);
        if v3(self.pos.x, 0, self.pos.z).dist(flat) < fx::ratio(6, 10) {
            self.path.clear();
            return true;
        }
        let stale = self.path_goal.is_none_or(|g| g.dist(flat) > int(2)) || self.path.is_empty();
        if stale {
            // a new route at most every quarter second per soldier
            if !(ctx.tick as usize + self.id).is_multiple_of(15)
                && self.path_goal.is_some()
                && !self.path.is_empty()
            {
                // keep the old route a little longer
            } else {
                self.path = ctx.nav.path(ctx.world, self.pos, flat).unwrap_or_default();
                self.path_goal = Some(flat);
                if self.path.is_empty() {
                    return true;
                }
            }
        }
        let (p, heading) = nav::follow(self.pos, &mut self.path, speed, dt);
        self.moving = p != self.pos;
        self.pos = p;
        if let Some(h) = heading {
            self.turn_to(h, dt * 2);
        }
        self.walk_phase = (self.walk_phase + fx::mul(speed, dt) * 3) & (fx::TURN - 1);
        self.path.is_empty() && v3(self.pos.x, 0, self.pos.z).dist(flat) < int(1)
    }

    /// One tick of behaviour for a free, awake soldier.
    pub fn think(&mut self, ctx: &Ctx, rng: &mut Rng, dt: i32, sees: bool, out: &mut Vec<Event>) {
        self.moving = false;
        if self.fire_cd > 0 {
            self.fire_cd -= dt;
        }
        // the eyes sweep while he stands about
        let sweep = fx::mul(
            deg(35),
            fx::sin((ctx.tick as i32 * 180 + self.id as i32 * 9000) & (fx::TURN - 1)),
        );
        match self.mind {
            Mind::Patrol => {
                self.look = sweep;
                self.awareness = (self.awareness - fx::mul(int(NOTICE_DECAY), dt)).max(0);
                self.patrol(ctx, dt);
            }
            Mind::Suspicious { at, t } => {
                self.look = 0;
                self.face(at, dt);
                if !sees {
                    self.awareness = (self.awareness - fx::mul(int(NOTICE_DECAY), dt)).max(0);
                }
                let t = t - dt;
                self.mind = if t <= 0 && self.awareness < int(SUSPICIOUS_AT) {
                    Mind::Patrol
                } else {
                    Mind::Suspicious { at, t: t.max(0) }
                };
            }
            Mind::Investigate { at, t } => {
                self.look = 0;
                if !sees {
                    self.awareness =
                        (self.awareness - fx::mul(int(NOTICE_DECAY / 2), dt)).max(int(SUSPICIOUS_AT));
                }
                if t > 0 || self.walk_to(ctx, at, JOG, dt) {
                    self.look = sweep * 2;
                    let t = t + dt;
                    self.mind = if t >= LOOK_AROUND {
                        self.awareness = int(SUSPICIOUS_AT) / 2;
                        match ctx.phase {
                            Phase::Normal => Mind::Patrol,
                            _ => Mind::Search { at, t: SEARCH_TIME },
                        }
                    } else {
                        Mind::Investigate { at, t }
                    };
                }
            }
            Mind::Search { at, t } => {
                self.look = sweep * 2;
                let t = t - dt;
                if t <= 0 {
                    self.mind = Mind::Patrol;
                    self.path_goal = None;
                } else {
                    // wander from point to point about the place
                    if self.path.is_empty() && (ctx.tick as usize + self.id * 7).is_multiple_of(90) {
                        let p = at.add(v3(rng.range(-8, 8) * ONE, 0, rng.range(-8, 8) * ONE));
                        self.path_goal = None;
                        self.mind = Mind::Search { at, t };
                        self.walk_to(ctx, p, WALK, dt);
                    } else if !self.path.is_empty() {
                        let (p, h) = nav::follow(self.pos, &mut self.path, WALK, dt);
                        self.moving = p != self.pos;
                        self.pos = p;
                        if let Some(h) = h {
                            self.turn_to(h, dt * 2);
                        }
                        self.walk_phase = (self.walk_phase + fx::mul(WALK, dt) * 3) & (fx::TURN - 1);
                    }
                    self.mind = Mind::Search { at, t };
                }
            }
            Mind::Combat => self.fight(ctx, rng, dt, sees, out),
            Mind::Help { buddy, t } => {
                let Some(b) = ctx.bodies.iter().find(|b| b.idx == buddy) else {
                    self.mind = Mind::Search {
                        at: self.pos,
                        t: SEARCH_TIME,
                    };
                    return;
                };
                if t > 0 || self.walk_to(ctx, b.at, JOG, dt) {
                    self.face(b.at, dt);
                    let need = if b.bound { UNTIE_TIME } else { WAKE_BUDDY };
                    let t = t + dt;
                    if t >= need {
                        out.push(Event::Woke { by: self.id, buddy });
                        self.mind = Mind::Search {
                            at: b.at,
                            t: SEARCH_TIME,
                        };
                    } else {
                        self.mind = Mind::Help { buddy, t };
                    }
                }
            }
            Mind::Portal { which, t } => {
                let Some(p) = ctx.portals[which] else {
                    self.mind = Mind::Search {
                        at: self.pos,
                        t: SEARCH_TIME,
                    };
                    return;
                };
                let stand = v3(p.x, 0, p.z);
                let arrived = t > 0 || self.walk_to(ctx, stand, JOG, dt) || self.pos.dist(stand) < int(4);
                if arrived {
                    self.face(p, dt);
                    let t = t + dt;
                    if t >= int(2) {
                        out.push(Event::ShutPortal(which));
                        self.mind = Mind::Search {
                            at: stand,
                            t: SEARCH_TIME,
                        };
                    } else {
                        self.mind = Mind::Portal { which, t };
                    }
                } else if (ctx.tick as usize + self.id).is_multiple_of(120) && self.path.is_empty() {
                    // nowhere to stand near it: shut it from where he is, by radio
                    out.push(Event::ShutPortal(which));
                    self.mind = Mind::Search {
                        at: self.pos,
                        t: SEARCH_TIME,
                    };
                }
            }
        }
    }

    fn patrol(&mut self, ctx: &Ctx, dt: i32) {
        if self.route.is_empty() {
            return;
        }
        if self.perch || self.route.len() == 1 {
            let (home, _) = self.route[0];
            if !self.perch && self.pos.dist(home) > ONE {
                self.walk_to(ctx, home, WALK, dt);
                return;
            }
            self.turn_to(self.home_yaw, dt);
            return;
        }
        if self.wait > 0 {
            self.wait -= dt;
            return;
        }
        let (goal, pause) = self.route[self.route_i];
        if self.walk_to(ctx, goal, WALK, dt) {
            self.route_i = (self.route_i + 1) % self.route.len();
            self.wait = pause;
            self.path_goal = None;
        }
    }

    fn fight(&mut self, ctx: &Ctx, rng: &mut Rng, dt: i32, sees: bool, out: &mut Vec<Event>) {
        self.look = 0;
        let p = &ctx.player;
        let d = p.chest.dist(self.eyes());
        if sees && d < GUN_RANGE {
            self.face(p.pos, dt * 2);
            self.aim += dt;
            let need = if self.elite { AIM_TIME_ELITE } else { AIM_TIME };
            if self.aim >= need && self.fire_cd <= 0 && !ctx.reflex {
                self.fire_cd = FIRE_EVERY;
                let mut chance = HIT_CLOSE - HIT_PER_10M * (d / int(10)) - p.stance_hit;
                if p.sprinting {
                    chance -= HIT_SPRINT;
                } else if p.moving {
                    chance -= HIT_MOVING;
                }
                let hit = rng.chance(chance.max(HIT_FLOOR));
                out.push(Event::Shot {
                    from: self.eyes(),
                    hit,
                });
            }
            return;
        }
        self.aim = (self.aim - dt).max(0);
        if !self.perch {
            self.walk_to(ctx, ctx.lkp, RUN, dt);
        } else {
            self.face(ctx.lkp, dt);
        }
    }

    /// Looks for bodies, portals and balloons. Called every few ticks.
    pub fn scan(&mut self, ctx: &Ctx) {
        if !self.free() || matches!(self.mind, Mind::Combat | Mind::Help { .. }) {
            return;
        }
        for b in ctx.bodies {
            if b.idx == self.id || self.seen_bodies.contains(&b.idx) {
                continue;
            }
            if self.can_see(ctx.world, b.at.add(v3(0, fx::ratio(3, 10), 0)), BODY_SIGHT, false) {
                self.seen_bodies.push(b.idx);
                self.awareness = self.awareness.max(int(50));
                self.mind = Mind::Help { buddy: b.idx, t: 0 };
                self.path_goal = None;
                return;
            }
        }
        if self.elite && !matches!(self.mind, Mind::Portal { .. }) {
            for (k, p) in ctx.portals.iter().enumerate() {
                if let Some(p) = p {
                    if self.can_see(ctx.world, *p, PORTAL_SIGHT, false) {
                        self.awareness = self.awareness.max(int(50));
                        self.mind = Mind::Portal { which: k, t: 0 };
                        self.path_goal = None;
                        return;
                    }
                }
            }
        }
        for b in ctx.balloons {
            if matches!(self.mind, Mind::Patrol) && self.can_see(ctx.world, *b, BALLOON_SIGHT, false) {
                self.awareness = self.awareness.max(int(SUSPICIOUS_AT + 5));
                self.mind = Mind::Suspicious {
                    at: *b,
                    t: LOOK_AROUND,
                };
                return;
            }
        }
    }

    /// The timers of a soldier who is down. Returns true when he wakes.
    pub fn sleep_tick(&mut self, dt: i32) -> bool {
        match self.life {
            Life::Dart(t) => {
                self.life = if t - dt <= 0 {
                    Life::Asleep(SLEEP_TIME)
                } else {
                    Life::Dart(t - dt)
                };
                false
            }
            Life::Asleep(t) | Life::Stunned(t) => {
                if t - dt <= 0 {
                    self.life = Life::Awake;
                    self.held = false;
                    true
                } else {
                    self.life = match self.life {
                        Life::Asleep(_) => Life::Asleep(t - dt),
                        _ => Life::Stunned(t - dt),
                    };
                    false
                }
            }
            _ => false,
        }
    }

    /// A ray against his head and body: the distance along it, and whether
    /// it was the head. `d` is a unit vector.
    pub fn hit_by(&self, o: V3, d: V3) -> Option<(i32, bool)> {
        if self.life == Life::Gone || self.carried || self.on_mule || self.locker.is_some() {
            return None;
        }
        let parts: Vec<(V3, i32, bool)> = if self.down() {
            vec![(
                self.pos.add(v3(0, fx::ratio(25, 100), 0)),
                BODY_RADIUS + ONE / 4,
                false,
            )]
        } else {
            vec![
                (self.pos.add(v3(0, fx::ratio(165, 100), 0)), HEAD_RADIUS, true),
                (self.pos.add(v3(0, fx::ratio(120, 100), 0)), BODY_RADIUS, false),
                (self.pos.add(v3(0, fx::ratio(70, 100), 0)), BODY_RADIUS, false),
                (
                    self.pos.add(v3(0, fx::ratio(30, 100), 0)),
                    BODY_RADIUS * 3 / 4,
                    false,
                ),
            ]
        };
        let mut best: Option<(i32, bool)> = None;
        for (c, r, head) in parts {
            if let Some(t) = ray_sphere(o, d, c, r) {
                if best.is_none_or(|b| t < b.0) {
                    best = Some((t, head));
                }
            }
        }
        best
    }
}

/// Distance along a unit ray to a sphere, if it meets it ahead.
pub fn ray_sphere(o: V3, d: V3, c: V3, r: i32) -> Option<i32> {
    let oc = c.sub(o);
    let along = oc.dot(d);
    if along < 0 {
        return None;
    }
    let closest = o.add(d.scale(along));
    let miss = closest.dist(c);
    if miss > r {
        return None;
    }
    let back = fx::isqrt((r as i64 * r as i64 - miss as i64 * miss as i64) as u64) as i32;
    Some((along - back).max(0))
}

/// The unit vector a soldier faces.
pub fn ahead(s: &Soldier) -> V3 {
    facing(s.gaze(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Palette;
    use crate::world;

    fn setup() -> (World, Nav) {
        let w = world::build(&Palette::parse(include_str!("../../../data/palette.json")).unwrap());
        let n = Nav::build(&w);
        (w, n)
    }

    fn target(pos: V3, vis: i32) -> Target {
        Target {
            pos,
            chest: pos.add(v3(0, int(1), 0)),
            vis,
            hidden: false,
            moving: false,
            sprinting: false,
            stance_hit: 0,
        }
    }

    fn ctx<'a>(w: &'a World, n: &'a Nav, t: Target, phase: Phase) -> Ctx<'a> {
        Ctx {
            world: w,
            nav: n,
            player: t,
            phase,
            lkp: t.pos,
            bodies: &[],
            portals: [None, None],
            balloons: &[],
            tick: 0,
            reflex: false,
        }
    }

    /// A soldier in the open yard at the origin, looking north (−z).
    fn soldier() -> Soldier {
        Soldier::new(0, vec![(v3(0, 0, int(30)), 0)], 0, false)
    }

    /// Seconds of watching until spotted, or None within 10 s.
    fn time_to_spot(s: &mut Soldier, c: &Ctx) -> Option<i32> {
        let mut out = Vec::new();
        for tick in 0..600 {
            s.watch(c, DT, &mut out);
            if out.iter().any(|e| matches!(e, Event::Spotted(_))) {
                return Some(tick);
            }
        }
        None
    }

    #[test]
    fn closer_is_seen_faster_and_out_of_range_is_not_seen() {
        let (w, n) = setup();
        let near = time_to_spot(
            &mut soldier(),
            &ctx(&w, &n, target(v3(0, 0, int(22)), 100), Phase::Normal),
        )
        .unwrap();
        let far = time_to_spot(
            &mut soldier(),
            &ctx(&w, &n, target(v3(0, 0, int(8)), 100), Phase::Normal),
        )
        .unwrap();
        // he stands at z = 30 looking toward −z: z = 22 is 8 m from him, z = 8 is 22 m
        assert!(far > near, "8 m: {near} ticks, 22 m: {far} ticks");
        let gone = time_to_spot(
            &mut soldier(),
            &ctx(&w, &n, target(v3(0, 0, -int(20)), 100), Phase::Normal),
        );
        assert_eq!(gone, None, "50 m away is beyond 34 m sight");
    }

    #[test]
    fn camouflage_shortens_how_far_he_sees() {
        let (w, n) = setup();
        // 20 m ahead: plain (100) is seen, a 60% camo index (vis 40: 13.6 m) is not
        let plain = time_to_spot(
            &mut soldier(),
            &ctx(&w, &n, target(v3(0, 0, int(10)), 100), Phase::Normal),
        );
        let hidden = time_to_spot(
            &mut soldier(),
            &ctx(&w, &n, target(v3(0, 0, int(10)), 40), Phase::Normal),
        );
        assert!(plain.is_some());
        assert_eq!(hidden, None);
    }

    #[test]
    fn he_does_not_see_behind_him_unless_you_are_right_there() {
        let (w, n) = setup();
        let behind = time_to_spot(
            &mut soldier(),
            &ctx(&w, &n, target(v3(0, 0, int(36)), 100), Phase::Normal),
        );
        assert_eq!(behind, None, "6 m behind");
        let mut t = target(v3(0, 0, int(31)), 100);
        t.moving = true;
        assert!(
            time_to_spot(&mut soldier(), &ctx(&w, &n, t, Phase::Normal)).is_some(),
            "1 m behind, moving"
        );
    }

    #[test]
    fn walls_hide_you_and_lockers_hide_you() {
        let (w, n) = setup();
        // a soldier inside the gate looking south at a man outside the wall
        let mut s = Soldier::new(0, vec![(v3(int(30), 0, int(76)), 0)], fx::HALF, false);
        let t = target(v3(int(30), 0, int(86)), 100);
        assert_eq!(time_to_spot(&mut s, &ctx(&w, &n, t, Phase::Normal)), None);
        let mut t = target(v3(0, 0, int(22)), 100);
        t.hidden = true;
        assert_eq!(time_to_spot(&mut soldier(), &ctx(&w, &n, t, Phase::Normal)), None);
    }

    #[test]
    fn suspicion_comes_before_spotting() {
        let (w, n) = setup();
        let mut s = soldier();
        let c = ctx(&w, &n, target(v3(0, 0, int(2)), 100), Phase::Normal);
        let mut out = Vec::new();
        let mut order = Vec::new();
        for _ in 0..600 {
            s.watch(&c, DT, &mut out);
            for e in out.drain(..) {
                order.push(e);
            }
            if order.iter().any(|e| matches!(e, Event::Spotted(_))) {
                break;
            }
        }
        assert!(matches!(order.first(), Some(Event::Noticed(0))), "{order:?}");
        assert!(matches!(order.last(), Some(Event::Spotted(0))));
    }

    #[test]
    fn a_noise_sends_him_to_look_and_he_walks_there() {
        let (w, n) = setup();
        let mut s = soldier();
        let at = v3(int(8), 0, int(30));
        s.hear(at, false);
        assert!(matches!(s.mind, Mind::Investigate { .. }));
        let c = ctx(&w, &n, target(v3(int(90), 0, int(90)), 100), Phase::Normal);
        let mut rng = Rng::new(1);
        let mut out = Vec::new();
        for _ in 0..600 {
            s.think(&c, &mut rng, DT, false, &mut out);
        }
        assert!(s.pos.dist(at) < int(2), "{:?}", s.pos);
    }

    #[test]
    fn in_combat_he_aims_then_fires_and_a_close_standing_target_is_usually_hit() {
        let (w, n) = setup();
        let mut hits = 0;
        let mut shots = 0;
        let mut rng = Rng::new(5);
        for _ in 0..50 {
            let mut s = soldier();
            s.alert();
            let c = ctx(&w, &n, target(v3(0, 0, int(25)), 100), Phase::Combat(0));
            let mut out = Vec::new();
            let mut first = None;
            for tick in 0..120 {
                let sees = s.watch(&c, DT, &mut out);
                s.think(&c, &mut rng, DT, sees, &mut out);
                if let Some(Event::Shot { hit, .. }) = out.iter().find(|e| matches!(e, Event::Shot { .. })) {
                    first.get_or_insert(tick);
                    shots += 1;
                    if *hit {
                        hits += 1;
                    }
                    break;
                }
            }
            // he aims for 0.9 s first
            assert!(first.unwrap() >= 53, "{first:?}");
        }
        assert_eq!(shots, 50);
        assert!(hits > 30 && hits < 50, "{hits} of 50 at 5 m");
    }

    #[test]
    fn a_dart_drops_him_after_a_moment_and_he_sleeps_five_minutes() {
        let mut s = soldier();
        s.life = Life::Dart(DART_DELAY);
        let mut ticks = 0;
        while matches!(s.life, Life::Dart(_)) {
            s.sleep_tick(DT);
            ticks += 1;
        }
        // 2.5 s; a sixtieth of a second is a hair short in Q16
        assert!((150..=151).contains(&ticks), "{ticks}");
        let mut woke = 0;
        for t in 0..(301 * 60) {
            if s.sleep_tick(DT) {
                woke = t;
                break;
            }
        }
        assert!((299 * 60..=300 * 60 + 10).contains(&woke), "{woke}");
    }

    #[test]
    fn rays_find_the_head_before_the_body() {
        let s = soldier();
        let o = v3(0, fx::ratio(165, 100), int(40));
        let (t, head) = s.hit_by(o, v3(0, 0, -ONE)).unwrap();
        assert!(head);
        assert!((t - int(10) + HEAD_RADIUS).abs() < ONE / 50);
        let (_, head) = s.hit_by(v3(0, ONE, int(40)), v3(0, 0, -ONE)).unwrap();
        assert!(!head);
        assert!(s.hit_by(v3(int(2), ONE, int(40)), v3(0, 0, -ONE)).is_none());
    }
}
