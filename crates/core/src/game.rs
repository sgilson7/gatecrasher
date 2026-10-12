//! The game: screens, one mission at a time, and everything that happens
//! between the operative, the soldiers, the mule and the zone.
//!
//! The page calls `advance` once a display frame with its clock, the held
//! action bits and the mouse movement. The simulation runs in fixed ticks of
//! a sixtieth of a second (a fifth of that in reflex mode). After each frame
//! the page reads what to draw and the HUD as JSON.

use crate::balance::*;
use crate::content::{Palette, Rgb};
use crate::draw::{self, Hat, Look, MuleLook, Pose};
use crate::fx::{self, deg, int, ratio, ONE};
use crate::geom::{facing, right_of, v3, V3};
use crate::mesh::{self, Mesh};
use crate::mission::{self, Kind, Mission, Recruits, Reveal, Uniform};
use crate::nav::{self, Nav};
use crate::player::{self, Gait, Player, Stance};
use crate::portal::{self, Portal};
use crate::render::{self, Batch, Camera, LAYER_INKED};
use crate::rng::Rng;
use crate::soldier::{Body, Ctx, Event, Life, Mind, Phase, Soldier, Target};
use crate::world::{self, World, STYLE_CAMO, STYLE_CLOAK, STYLE_GLOW};
use serde_json::{json, Value};
use std::collections::BTreeMap;

// action bits (data/controls.json names them; numbers() reports them)
pub const FORWARD: u32 = 1;
pub const BACK: u32 = 1 << 1;
pub const LEFT: u32 = 1 << 2;
pub const RIGHT: u32 = 1 << 3;
pub const SPRINT: u32 = 1 << 4;
pub const CROUCH: u32 = 1 << 5;
pub const PRONE: u32 = 1 << 6;
pub const JUMP: u32 = 1 << 7;
pub const USE: u32 = 1 << 8;
pub const CQC: u32 = 1 << 9;
pub const FIRE: u32 = 1 << 10;
pub const AIM: u32 = 1 << 11;
pub const ROUNDS: u32 = 1 << 12;
pub const PORTAL_A: u32 = 1 << 13;
pub const PORTAL_B: u32 = 1 << 14;
pub const CLOSE: u32 = 1 << 15;
pub const CLOAK: u32 = 1 << 16;
pub const BOX: u32 = 1 << 17;
pub const UNIFORM: u32 = 1 << 18;
pub const WHISTLE: u32 = 1 << 19;
pub const RAM: u32 = 1 << 20;
pub const TIE: u32 = 1 << 21;
pub const FULTON: u32 = 1 << 22;
pub const FOCUS: u32 = 1 << 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Title,
    Brief,
    Play,
    Paused,
    Failed,
    Complete,
    Graceland,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Title => "title",
            Mode::Brief => "brief",
            Mode::Play => "play",
            Mode::Paused => "paused",
            Mode::Failed => "failed",
            Mode::Complete => "complete",
            Mode::Graceland => "graceland",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recruit {
    pub name: String,
    pub unit: usize,
    pub grade: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub rank: String,
    pub secs: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Carry {
    Soldier(usize),
    Commando,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoldAct {
    Grip,
    Talk(i32),
    Choke(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MuleMode {
    Follow,
    Come,
    Charge(usize),
}

#[derive(Debug, Clone)]
struct Mule {
    pos: V3,
    yaw: i32,
    mode: MuleMode,
    path: Vec<V3>,
    goal: Option<V3>,
    load: Option<usize>,
    talk: i32,
    sniff: i32,
    walk: i32,
    moving: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Who {
    Soldier(usize),
    Commando,
}

#[derive(Debug, Clone, Copy)]
struct Balloon {
    who: Who,
    at: V3,
    t: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prisoner {
    Caged,
    Free,
    Carried,
    Gone,
}

#[derive(Debug, Clone, Copy)]
struct Tracer {
    from: V3,
    to: V3,
    t: i32,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub done: bool,
    pub cause: &'static str,
    pub rank: String,
    pub secs: i32,
    pub alerts: i32,
    pub kills: i32,
    pub recruits: usize,
    pub intel: usize,
}

#[derive(Debug, Clone)]
struct Run {
    m: usize,
    world: World,
    p: Player,
    cam_yaw: i32,
    cam_pitch: i32,
    soldiers: Vec<Soldier>,
    mule: Mule,
    portals: [Option<Portal>; 2],
    portal_cd: i32,
    phase: Phase,
    lkp: V3,
    reflex: Option<(usize, i32)>,
    rng: Rng,
    elapsed: i32,
    alerts: i32,
    kills: i32,
    revealed: Vec<bool>,
    target_known: bool,
    places: Vec<V3>,
    prisoner: Prisoner,
    prisoner_at: V3,
    case_taken: bool,
    balloons: Vec<Balloon>,
    tracers: Vec<Tracer>,
    hold: Option<(usize, HoldAct)>,
    carry: Option<Carry>,
    uniform: usize,
    changing: i32,
    cloak: bool,
    battery: i32,
    battery_max: i32,
    lethal: bool,
    ammo: [i32; 2],
    fultons: i32,
    fire_cd: i32,
    aiming: bool,
    mark: Option<(usize, i32)>,
    recruited: Vec<Recruit>,
    outcome: Option<Outcome>,
    sniff_range: i32,
    tick: u32,
    said: Option<(String, i32)>,
    camo: i32,
}

pub struct Game {
    palette: Palette,
    base: World,
    nav: Nav,
    missions: Vec<Mission>,
    uniforms: Vec<Uniform>,
    recruits: Recruits,
    roster: Vec<Recruit>,
    records: BTreeMap<String, Record>,
    uniform: usize,
    mode: Mode,
    chosen: usize,
    run: Option<Run>,
    clock: Option<i64>,
    acc: i64,
    prev_bits: u32,
    pending: u32,
    size: (i32, i32),
    scenery_vals: Vec<i32>,
    scenery_runs: Vec<(usize, u32, u32, u32)>,
    scenery_version: u32,
    dynamic_vals: Vec<i32>,
    dynamic_runs: Vec<(usize, u32, u32, u32)>,
    cam: Camera,
    orbit: i32,
    sounds: Vec<&'static str>,
    toasts: Vec<String>,
    names: Rng,
    scenery_cage_open: bool,
}

fn secs_text(secs: i32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

impl Game {
    pub fn new(
        palette: &str,
        missions: &str,
        uniforms: &str,
        recruits: &str,
        saved: &str,
    ) -> Result<Game, String> {
        let palette = Palette::parse(palette)?;
        let base = world::build(&palette);
        let nav = Nav::build(&base);
        let missions = mission::parse_missions(missions, &base)?;
        let uniforms = mission::parse_uniforms(uniforms)?;
        let recruits = mission::parse_recruits(recruits)?;
        let mut g = Game {
            palette,
            base,
            nav,
            missions,
            uniforms,
            recruits,
            roster: Vec::new(),
            records: BTreeMap::new(),
            uniform: 0,
            mode: Mode::Title,
            chosen: 0,
            run: None,
            clock: None,
            acc: 0,
            prev_bits: 0,
            pending: 0,
            size: (1280, 720),
            scenery_vals: Vec::new(),
            scenery_runs: Vec::new(),
            scenery_version: 0,
            dynamic_vals: Vec::new(),
            dynamic_runs: Vec::new(),
            cam: Camera {
                eye: v3(0, int(60), int(120)),
                fwd: facing(0, -deg(20)),
                fov: FOV,
                far: FAR,
            },
            orbit: 0,
            sounds: Vec::new(),
            toasts: Vec::new(),
            names: Rng::new(9),
            scenery_cage_open: false,
        };
        g.load(saved);
        g.rebuild_scenery();
        g.frame();
        Ok(g)
    }

    fn load(&mut self, saved: &str) {
        let Ok(v) = serde_json::from_str::<Value>(saved) else {
            return;
        };
        if let Some(r) = v["roster"].as_array() {
            for x in r {
                if let (Some(name), Some(unit), Some(grade)) =
                    (x["name"].as_str(), x["unit"].as_u64(), x["grade"].as_u64())
                {
                    self.roster.push(Recruit {
                        name: name.to_string(),
                        unit: (unit as usize).min(2),
                        grade: (grade as usize).min(self.recruits.grades.len() - 1),
                    });
                }
            }
        }
        if let Some(r) = v["records"].as_object() {
            for (k, x) in r {
                if let (Some(rank), Some(secs)) = (x["rank"].as_str(), x["secs"].as_i64()) {
                    self.records.insert(
                        k.clone(),
                        Record {
                            rank: rank.to_string(),
                            secs: secs as i32,
                        },
                    );
                }
            }
        }
        if let Some(u) = v["uniform"].as_u64() {
            self.uniform = (u as usize).min(self.uniforms.len() - 1);
        }
        self.names = Rng::new(9 + self.roster.len() as u64 * 31);
    }

    pub fn saved(&self) -> String {
        let roster: Vec<Value> = self
            .roster
            .iter()
            .map(|r| json!({"name": r.name, "unit": r.unit, "grade": r.grade}))
            .collect();
        let records: serde_json::Map<String, Value> = self
            .records
            .iter()
            .map(|(k, r)| (k.clone(), json!({"rank": r.rank, "secs": r.secs})))
            .collect();
        json!({"roster": roster, "records": records, "uniform": self.uniform}).to_string()
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    // --- Graceland: what the staff gives ---------------------------------------------

    /// Each unit's level, 0..5: a point per grade step per recruit, six to a level.
    pub fn unit_levels(&self) -> [i32; 3] {
        let mut pts = [0i32; 3];
        for r in &self.roster {
            pts[r.unit] += r.grade as i32 + 1;
        }
        pts.map(|p| (p / 6).min(5))
    }

    // --- the screens --------------------------------------------------------------------

    pub fn choose(&mut self, k: u32) {
        if matches!(self.mode, Mode::Title | Mode::Brief) && (k as usize) < self.missions.len() {
            self.chosen = k as usize;
            self.mode = Mode::Brief;
        }
    }

    pub fn set_uniform(&mut self, k: u32) {
        if (k as usize) < self.uniforms.len() {
            self.uniform = k as usize;
        }
    }

    pub fn graceland(&mut self) {
        if matches!(self.mode, Mode::Title | Mode::Complete | Mode::Failed) {
            self.mode = Mode::Graceland;
            self.run = None;
            self.rebuild_scenery();
        }
    }

    pub fn to_title(&mut self) {
        self.mode = Mode::Title;
        self.run = None;
        self.rebuild_scenery();
    }

    pub fn pause(&mut self) {
        if self.mode == Mode::Play {
            self.mode = Mode::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.mode == Mode::Paused {
            self.mode = Mode::Play;
            self.clock = None;
        }
    }

    /// For screenshots and the browser check: puts the operative at `x`, `z`
    /// metres facing `yaw` degrees, and the mule beside her.
    pub fn stage(&mut self, x: i32, z: i32, yaw: i32) {
        if let Some(r) = self.run.as_mut() {
            let at = v3(int(x), 0, int(z));
            r.p.pos = v3(at.x, r.world.ground(at, RADIUS, int(30)), at.z);
            r.p.yaw = deg(yaw);
            r.cam_yaw = deg(yaw);
            r.mule.pos = v3(at.x, 0, at.z).sub(right_of(deg(yaw)).scale(int(2)));
            r.mule.path.clear();
        }
    }

    pub fn retry(&mut self) {
        if matches!(self.mode, Mode::Failed | Mode::Complete | Mode::Paused) {
            self.deploy();
        }
    }

    /// Starts the chosen mission: the soldiers in their places, everything fresh.
    pub fn deploy(&mut self) {
        let m = &self.missions[self.chosen];
        let levels = self.unit_levels();
        let mut world = self.base.clone();
        let mut soldiers = Vec::new();
        for (k, s) in m.soldiers.iter().enumerate() {
            let mut sol = Soldier::new(k, s.route.clone(), s.yaw, s.perch);
            sol.elite = s.elite;
            sol.officer = s.officer;
            sol.target = s.target;
            sol.knows = s.knows.clone();
            sol.name_seed = m.seed + k as u64;
            soldiers.push(sol);
        }
        let mut p = Player::new(m.start, m.start_yaw);
        p.pos.y = world.ground(p.pos, RADIUS, p.pos.y + STEP);
        // beside her on the left, out of the camera's way over her right shoulder
        let mule_at = m.start.sub(right_of(m.start_yaw).scale(int(3)));
        let mule_y = world.ground(mule_at, ONE / 2, mule_at.y + STEP);
        let prisoner_at = m.target_place.unwrap_or(V3::ZERO);
        if m.kind != Kind::Rescue {
            // the cage stands empty and open in the other missions
            world.open_cage();
        }
        let tranq = TRANQ_ROUNDS + 2 * levels[1];
        let battery = BATTERY + fx::mul(BATTERY, ratio(8 * levels[2], 100));
        self.run = Some(Run {
            m: self.chosen,
            world,
            p,
            cam_yaw: m.start_yaw,
            cam_pitch: -deg(8),
            soldiers,
            mule: Mule {
                pos: v3(mule_at.x, mule_y, mule_at.z),
                yaw: m.start_yaw,
                mode: MuleMode::Follow,
                path: Vec::new(),
                goal: None,
                load: None,
                talk: 0,
                sniff: 0,
                walk: 0,
                moving: false,
            },
            portals: [None, None],
            portal_cd: 0,
            phase: Phase::Normal,
            lkp: V3::ZERO,
            reflex: None,
            rng: Rng::new(m.seed),
            elapsed: 0,
            alerts: 0,
            kills: 0,
            revealed: vec![false; m.intel.len()],
            target_known: false,
            places: Vec::new(),
            prisoner: if m.kind == Kind::Rescue {
                Prisoner::Caged
            } else {
                Prisoner::Gone
            },
            prisoner_at,
            case_taken: m.kind != Kind::Recover,
            balloons: Vec::new(),
            tracers: Vec::new(),
            hold: None,
            carry: None,
            uniform: self.uniform,
            changing: 0,
            cloak: false,
            battery,
            battery_max: battery,
            lethal: false,
            ammo: [tranq, LETHAL_ROUNDS],
            fultons: FULTONS,
            fire_cd: 0,
            aiming: false,
            mark: None,
            recruited: Vec::new(),
            outcome: None,
            sniff_range: MULE_SNIFF + int(4) * levels[0],
            tick: 0,
            said: None,
            camo: 0,
        });
        self.mode = Mode::Play;
        self.clock = None;
        self.acc = 0;
        self.prev_bits = 0;
        self.rebuild_scenery();
        self.sounds.push("deploy");
    }

    fn rebuild_scenery(&mut self) {
        let w = self.run.as_ref().map(|r| &r.world).unwrap_or(&self.base);
        self.scenery_cage_open = w.cage_open();
        let (v, runs) = draw::scenery(w, &self.palette).finish();
        self.scenery_vals = v;
        self.scenery_runs = runs;
        self.scenery_version += 1;
    }

    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr100: i32) -> [i32; 2] {
        let w = (css_w * dpr100 / 100).clamp(1, 4096);
        let h = (css_h * dpr100 / 100).clamp(1, 4096);
        self.size = (w, h);
        [w, h]
    }

    fn aspect(&self) -> i32 {
        fx::div(int(self.size.0.max(1)), int(self.size.1.max(1)))
    }

    // --- the clock ----------------------------------------------------------------------

    pub fn advance(&mut self, now_us: i64, bits: u32, dx: i32, dy: i32) {
        self.sounds.clear();
        self.toasts.clear();
        let elapsed = match self.clock {
            Some(c) => (now_us - c).clamp(0, TICK_US * MAX_TICKS_PER_FRAME as i64),
            None => TICK_US,
        };
        self.clock = Some(now_us);
        let live = self.mode == Mode::Play && bits & FOCUS != 0;
        if live {
            if let Some(r) = self.run.as_mut() {
                r.cam_yaw = (r.cam_yaw - dx * LOOK / 100) & (fx::TURN - 1);
                r.cam_pitch = (r.cam_pitch - dy * LOOK / 100).clamp(PITCH_MIN, PITCH_MAX);
            }
        }
        if self.mode != Mode::Play {
            self.orbit = (self.orbit + 6) & (fx::TURN - 1);
        }
        self.acc += elapsed;
        let mut n = 0;
        while self.acc >= TICK_US && n < MAX_TICKS_PER_FRAME {
            self.acc -= TICK_US;
            n += 1;
            if live {
                let b = bits | self.pending;
                self.pending = 0;
                self.tick(b);
                if self.mode != Mode::Play {
                    break;
                }
            }
        }
        if n == MAX_TICKS_PER_FRAME {
            self.acc = 0;
        }
        if !live {
            self.prev_bits = 0;
            self.pending = 0;
        } else if n == 0 {
            self.pending |= bits;
        }
        self.frame();
    }

    fn tick(&mut self, bits: u32) {
        let pressed = bits & !self.prev_bits;
        self.prev_bits = bits & !FOCUS;
        let Some(mut r) = self.run.take() else {
            return;
        };
        let m = &self.missions[r.m];
        let mut sounds = Vec::new();
        let mut toasts = Vec::new();
        let dt = if r.reflex.is_some() { DT / REFLEX_SLOW } else { DT };
        r.tick = r.tick.wrapping_add(1);
        r.elapsed += dt;
        player_tick(
            &mut r,
            m,
            &self.uniforms,
            &self.nav,
            bits,
            pressed,
            dt,
            &mut sounds,
            &mut toasts,
        );
        if r.outcome.is_none() {
            soldiers_tick(&mut r, &self.nav, dt, &mut sounds, &mut toasts);
            mule_tick(&mut r, &self.nav, m, dt, &mut sounds, &mut toasts);
            world_tick(
                &mut r,
                m,
                dt,
                &mut sounds,
                &mut toasts,
                &self.recruits,
                &mut self.names,
            );
        }
        if let Some(o) = r.outcome.clone() {
            if o.done {
                let rec = Record {
                    rank: o.rank.clone(),
                    secs: o.secs,
                };
                let better = self.records.get(&m.id).is_none_or(|old| {
                    rank_value(&rec.rank) > rank_value(&old.rank)
                        || (rec.rank == old.rank && rec.secs < old.secs)
                });
                if better {
                    self.records.insert(m.id.clone(), rec);
                }
                self.roster.extend(r.recruited.iter().cloned());
                self.mode = Mode::Complete;
                sounds.push("complete");
            } else {
                self.mode = Mode::Failed;
            }
        }
        self.sounds.extend(sounds);
        self.toasts.extend(toasts);
        let rebuild = r.world.cage_open() != self.scenery_cage_open;
        self.run = Some(r);
        if rebuild {
            self.rebuild_scenery();
        }
    }
}

fn rank_value(r: &str) -> i32 {
    match r {
        "S" => 4,
        "A" => 3,
        "B" => 2,
        _ => 1,
    }
}

// --- the operative's tick ----------------------------------------------------------------

const SAID_TIME: i32 = int(6);
const LZ_RADIUS: i32 = int(7);

fn head_height(p: &Player) -> i32 {
    p.height() * 17 / 20
}

/// The third-person camera: over the right shoulder, pulled in front of
/// any wall behind; from inside a locker, looking out through its vents.
fn camera(r: &Run) -> Camera {
    let fwd = facing(r.cam_yaw, r.cam_pitch);
    if let Some(k) = r.p.locker {
        let l = &r.world.lockers[k];
        return Camera {
            eye: l
                .at
                .add(v3(0, ratio(16, 10), 0))
                .add(l.front.scale(ratio(35, 100))),
            fwd,
            fov: FOV,
            far: FAR,
        };
    }
    let (back, side, fov) = if r.aiming {
        (CAM_BACK_AIM, CAM_SIDE_AIM, FOV_AIM)
    } else {
        (CAM_BACK, CAM_SIDE, FOV)
    };
    let pivot = r.p.pos.add(v3(0, head_height(&r.p), 0));
    let want = pivot
        .add(right_of(r.cam_yaw).scale(side))
        .add(v3(0, CAM_UP / 2, 0))
        .sub(fwd.scale(back));
    let d = want.sub(pivot);
    let len = d.len();
    let dir = d.norm();
    let eye = match r.world.ray(pivot, dir, len, |s| s.opaque) {
        Some(h) if h.t < len => pivot.add(dir.scale((h.t - ratio(25, 100)).max(ratio(2, 10)))),
        _ => want,
    };
    Camera {
        eye,
        fwd,
        fov,
        far: FAR,
    }
}

/// The crosshair's ray: from the camera, started level with the operative
/// so nothing behind her is hit.
fn aim_ray(r: &Run) -> (V3, V3) {
    let cam = camera(r);
    let pivot = r.p.pos.add(v3(0, head_height(&r.p), 0));
    let along = pivot.sub(cam.eye).dot(cam.fwd).max(0);
    (cam.eye.add(cam.fwd.scale(along)), cam.fwd)
}

/// What the crosshair is on: the nearest soldier before any wall, and the
/// point it meets.
fn aimed(r: &Run, range: i32) -> (Option<usize>, V3, bool) {
    let (o, d) = aim_ray(r);
    let wall = r
        .world
        .ray(o, d, range, |s| s.opaque)
        .map(|h| h.t)
        .unwrap_or(range);
    let mut best: Option<(usize, i32, bool)> = None;
    for (i, s) in r.soldiers.iter().enumerate() {
        if s.held {
            continue;
        }
        if let Some((t, head)) = s.hit_by(o, d) {
            if t < wall && best.is_none_or(|b| t < b.1) {
                best = Some((i, t, head));
            }
        }
    }
    match best {
        Some((i, t, head)) => (Some(i), o.add(d.scale(t)), head),
        None => (None, o.add(d.scale(wall)), false),
    }
}

fn caution(r: &mut Run) {
    r.phase = match r.phase {
        Phase::Normal | Phase::Caution(_) => Phase::Caution(CAUTION_TIME),
        p => p,
    };
}

fn start_combat(r: &mut Run, sounds: &mut Vec<&'static str>) {
    if !matches!(r.phase, Phase::Combat(_)) {
        r.alerts += 1;
    }
    r.phase = Phase::Combat(0);
    r.reflex = None;
    r.lkp = r.p.pos;
    for s in r.soldiers.iter_mut() {
        s.alert();
    }
    sounds.push("alert");
}

/// A sound at `at` carrying `radius`: every free soldier inside it (less far
/// through walls) goes to look.
fn noise(r: &mut Run, at: V3, radius: i32, gunshot: bool) {
    if radius <= 0 {
        return;
    }
    let combat = matches!(r.phase, Phase::Combat(_));
    let ear = at.add(v3(0, ONE, 0));
    for s in r.soldiers.iter_mut() {
        if !s.free() {
            continue;
        }
        let d = s.eyes().dist(ear);
        if d > radius {
            continue;
        }
        let reach = if r.world.clear(ear, s.eyes()) {
            radius
        } else {
            radius * NOISE_WALLED / 100
        };
        if d < reach {
            s.hear(at, gunshot);
            if combat && gunshot {
                r.lkp = at;
            }
        }
    }
}

/// A soldier dropped asleep: those near who see him go to wake him.
fn sleeper_dropped(r: &mut Run, i: usize, sounds: &mut Vec<&'static str>) {
    let at = r.soldiers[i].pos.add(v3(0, ratio(3, 10), 0));
    let mut any = false;
    for (k, s) in r.soldiers.iter_mut().enumerate() {
        if k == i || !s.free() || matches!(s.mind, Mind::Combat) {
            continue;
        }
        if s.pos.dist(at) < SLEEPER_NOTICE && s.can_see(&r.world, at, SLEEPER_NOTICE, true) {
            s.seen_bodies.push(i);
            s.awareness = s.awareness.max(int(50));
            s.mind = Mind::Help { buddy: i, t: 0 };
            s.path_goal = None;
            any = true;
        }
    }
    if any {
        sounds.push("huh");
    }
}

fn knock_out(r: &mut Run, i: usize, life: Life) {
    let s = &mut r.soldiers[i];
    s.life = life;
    s.held = false;
    s.aim = 0;
    if let Some((k, _)) = r.reflex {
        if k == i {
            r.reflex = None;
        }
    }
    if matches!(r.hold, Some((h, _)) if h == i) {
        r.hold = None;
    }
}

fn kill(r: &mut Run, i: usize) {
    if !r.soldiers[i].target {
        r.kills += 1;
    }
    knock_out(r, i, Life::Dead);
}

fn objective_done(r: &Run, m: &Mission) -> bool {
    match m.kind {
        Kind::Rescue => r.prisoner == Prisoner::Gone || r.carry == Some(Carry::Commando),
        Kind::Eliminate => r
            .soldiers
            .iter()
            .any(|s| s.target && matches!(s.life, Life::Dead | Life::Gone)),
        Kind::Recover => r.case_taken,
    }
}

fn reveal(r: &mut Run, m: &Mission, id: usize, toasts: &mut Vec<String>, sounds: &mut Vec<&'static str>) {
    if r.revealed[id] {
        return;
    }
    r.revealed[id] = true;
    toasts.push(m.intel[id].text.clone());
    sounds.push("intel");
    match m.intel[id].reveals {
        Reveal::Target => {
            r.target_known = true;
            for s in r.soldiers.iter_mut() {
                if s.target {
                    s.marked = true;
                }
            }
        }
        Reveal::Elites => {
            for s in r.soldiers.iter_mut() {
                if s.elite {
                    s.marked = true;
                }
            }
        }
        Reveal::Place(p) => r.places.push(p),
        Reveal::None => {}
    }
}

/// A soldier talks (or does not). On the mule he nearly always does, and
/// tells more.
fn interrogate(
    r: &mut Run,
    m: &Mission,
    i: usize,
    on_mule: bool,
    toasts: &mut Vec<String>,
    sounds: &mut Vec<&'static str>,
) {
    let chance = if on_mule { MULE_TALK_CHANCE } else { TALK_CHANCE };
    if r.soldiers[i].talked && !on_mule {
        r.said = Some(("@talk_again".into(), SAID_TIME));
        return;
    }
    r.soldiers[i].talked = true;
    if !r.rng.chance(chance) {
        r.said = Some(("@talk_nothing".into(), SAID_TIME));
        return;
    }
    let mut knows = r.soldiers[i].knows.clone();
    if on_mule && knows.iter().all(|&k| r.revealed[k]) {
        // the mule is persuasive: he gives up something he only overheard
        let left: Vec<usize> = (0..m.intel.len()).filter(|&k| !r.revealed[k]).collect();
        if !left.is_empty() {
            knows.push(left[r.rng.range(0, left.len() as i32 - 1) as usize]);
        }
    }
    let range = if on_mule { int(60) } else { TALK_MARK_RANGE };
    let here = r.soldiers[i].pos;
    let mut marked = 0;
    if knows.is_empty() || on_mule {
        for (k, s) in r.soldiers.iter_mut().enumerate() {
            if k != i && s.life != Life::Gone && s.pos.dist(here) < range && !s.marked {
                s.marked = true;
                marked += 1;
            }
        }
    }
    if knows.is_empty() {
        r.said = Some((
            if marked > 0 { "@talk_marks" } else { "@talk_alone" }.into(),
            SAID_TIME,
        ));
        sounds.push("intel");
        return;
    }
    r.said = Some((m.intel[knows[0]].text.clone(), SAID_TIME));
    for k in knows {
        reveal(r, m, k, toasts, sounds);
    }
}

fn nearest_soldier(r: &Run, at: V3, reach: i32, ok: impl Fn(&Soldier) -> bool) -> Option<usize> {
    let mut best: Option<(usize, i32)> = None;
    for (i, s) in r.soldiers.iter().enumerate() {
        if !ok(s) || (s.pos.y - at.y).abs() > int(1) {
            continue;
        }
        let d = v3(s.pos.x - at.x, 0, s.pos.z - at.z).len();
        if d < reach && best.is_none_or(|b| d < b.1) {
            best = Some((i, d));
        }
    }
    best.map(|b| b.0)
}

fn lying(s: &Soldier) -> bool {
    s.down() && !s.carried && !s.on_mule && s.locker.is_none() && s.life != Life::Gone
}

fn nearest_locker(r: &Run, at: V3) -> Option<usize> {
    r.world
        .lockers
        .iter()
        .enumerate()
        .filter(|(_, l)| l.at.add(l.front.scale(ratio(8, 10))).dist(v3(at.x, 0, at.z)) < REACH)
        .min_by_key(|(_, l)| l.at.dist(at))
        .map(|(k, _)| k)
}

fn locker_taken(r: &Run, k: usize) -> bool {
    r.soldiers.iter().any(|s| s.locker == Some(k))
}

/// What E would do right now, for the prompt and for the press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Use {
    Nothing,
    LeaveLocker,
    Talk,
    Stash(usize),
    Drop,
    OpenCage,
    CarryPrisoner,
    TakeCase,
    CarryBody(usize),
    Hide(usize),
}

fn what_use(r: &Run, m: &Mission) -> Use {
    if r.p.locker.is_some() {
        return Use::LeaveLocker;
    }
    if let Some((_, HoldAct::Grip)) = r.hold {
        return Use::Talk;
    }
    if r.hold.is_some() {
        return Use::Nothing;
    }
    let at = r.p.pos;
    if let Some(c) = r.carry {
        if let (Carry::Soldier(_), Some(k)) = (c, nearest_locker(r, at)) {
            if !locker_taken(r, k) {
                return Use::Stash(k);
            }
        }
        return Use::Drop;
    }
    if m.kind == Kind::Rescue {
        if r.prisoner == Prisoner::Caged {
            if let Some(door) = r.world.place_at("cage_door") {
                if door.dist(v3(at.x, 0, at.z)) < int(2) {
                    return Use::OpenCage;
                }
            }
        }
        if r.prisoner == Prisoner::Free && r.prisoner_at.dist(at) < REACH {
            return Use::CarryPrisoner;
        }
    }
    if !r.case_taken {
        if let Some(c) = m.target_place {
            if c.dist(at) < REACH {
                return Use::TakeCase;
            }
        }
    }
    if let Some(i) = nearest_soldier(r, at, REACH, lying) {
        return Use::CarryBody(i);
    }
    if !r.p.boxed {
        if let Some(k) = nearest_locker(r, at) {
            if !locker_taken(r, k) {
                return Use::Hide(k);
            }
        }
    }
    Use::Nothing
}

fn fulton_target(r: &Run) -> Option<Who> {
    if let Some(Carry::Soldier(i)) = r.carry {
        if r.soldiers[i].life != Life::Dead {
            return Some(Who::Soldier(i));
        }
    }
    if r.carry == Some(Carry::Commando)
        || (r.prisoner == Prisoner::Free && r.prisoner_at.dist(r.p.pos) < REACH)
    {
        return Some(Who::Commando);
    }
    if let Some(i) = r.mule.load {
        if r.mule.pos.dist(r.p.pos) < MULE_TIE_REACH {
            return Some(Who::Soldier(i));
        }
    }
    nearest_soldier(r, r.p.pos, REACH, |s| lying(s) && s.life != Life::Dead).map(Who::Soldier)
}

#[allow(clippy::too_many_arguments)]
fn player_tick(
    r: &mut Run,
    m: &Mission,
    uniforms: &[Uniform],
    nav: &Nav,
    bits: u32,
    pressed: u32,
    dt: i32,
    sounds: &mut Vec<&'static str>,
    toasts: &mut Vec<String>,
) {
    let _ = nav;
    let on = |b: u32| pressed & b != 0;
    if let Some((_, t)) = r.said.as_mut() {
        *t -= dt;
    }
    if r.said.as_ref().is_some_and(|s| s.1 <= 0) {
        r.said = None;
    }
    r.portal_cd = (r.portal_cd - dt).max(0);
    r.fire_cd = (r.fire_cd - dt).max(0);
    if r.cloak {
        r.battery -= dt;
        if r.battery <= 0 {
            r.battery = 0;
            r.cloak = false;
            sounds.push("cloak_off");
        }
    } else {
        r.battery = (r.battery + fx::mul(BATTERY_RECHARGE, dt)).min(r.battery_max);
    }
    // in a locker: only leaving it
    if let Some(k) = r.p.locker {
        if on(USE) {
            let l = r.world.lockers[k];
            r.p.pos = l.at.add(l.front.scale(ratio(9, 10)));
            r.p.yaw = fx::atan2(-l.front.x, -l.front.z);
            r.p.locker = None;
            r.p.stance = Stance::Crouch;
            sounds.push("locker");
        }
        r.camo = CAMO_MAX;
        return;
    }
    if r.changing > 0 {
        r.changing -= dt;
        if r.changing <= 0 {
            r.uniform = (r.uniform + 1) % uniforms.len();
            toasts.push(format!("@uniform:{}", uniforms[r.uniform].name));
            sounds.push("uniform");
        }
    }
    let busy = r.hold.is_some() || r.carry.is_some();
    // toggles
    if on(ROUNDS) {
        r.lethal = !r.lethal;
        sounds.push("switch");
    }
    if on(CLOAK) {
        if r.cloak {
            r.cloak = false;
            sounds.push("cloak_off");
        } else if r.battery > ONE {
            r.cloak = true;
            sounds.push("cloak_on");
        }
    }
    if on(BOX) && !busy {
        r.p.boxed = !r.p.boxed;
        r.p.stance = Stance::Crouch;
        sounds.push("box");
    }
    if on(UNIFORM) && !busy && !r.p.boxed && r.changing <= 0 {
        r.changing = UNIFORM_TIME;
    }
    if on(CROUCH) {
        if let Some((i, HoldAct::Grip)) = r.hold {
            // throw him down: out cold for a while
            knock_out(r, i, Life::Stunned(STUN_THROW));
            let p = r.p.pos.add(facing(r.p.yaw, 0).scale(ratio(9, 10)));
            r.soldiers[i].pos = v3(p.x, r.p.pos.y, p.z);
            noise(r, r.p.pos, NOISE_THROW, false);
            sounds.push("throw");
        } else if !busy && !r.p.boxed {
            let s = match r.p.stance {
                Stance::Stand => Stance::Crouch,
                _ => Stance::Stand,
            };
            r.p.set_stance(&r.world, s);
        }
    }
    if on(PRONE) && !busy && !r.p.boxed {
        let s = if r.p.stance == Stance::Prone {
            Stance::Crouch
        } else {
            Stance::Prone
        };
        r.p.set_stance(&r.world, s);
    }
    if busy {
        r.p.set_stance(&r.world, Stance::Stand);
        r.p.boxed = false;
    }
    if on(JUMP) && r.hold.is_none() && r.carry.is_none() && r.p.jump(&r.world) {
        noise(r, r.p.pos, NOISE_VAULT, false);
        sounds.push("vault");
    }
    r.aiming = bits & AIM != 0 && !busy && !r.p.boxed;
    // moving
    let mut fwd = 0;
    let mut side = 0;
    if bits & FORWARD != 0 {
        fwd += 1;
    }
    if bits & BACK != 0 {
        fwd -= 1;
    }
    if bits & RIGHT != 0 {
        side += 1;
    }
    if bits & LEFT != 0 {
        side -= 1;
    }
    let still = r.changing > 0 || matches!(r.hold, Some((_, HoldAct::Talk(_) | HoldAct::Choke(_))));
    let wish = if still || (fwd == 0 && side == 0) {
        V3::ZERO
    } else {
        facing(r.cam_yaw, 0)
            .scale(int(fwd))
            .add(right_of(r.cam_yaw).scale(int(side)))
            .norm()
    };
    let cap = if r.carry.is_some() {
        SPEED_CARRY
    } else if r.hold.is_some() {
        SPEED_HOLD
    } else {
        int(99)
    };
    let face = if r.aiming { Some(r.cam_yaw) } else { None };
    let moved = r.p.step(&r.world, wish, bits & SPRINT != 0, cap, face, dt);
    if moved.fell_to_death {
        fail(r, "fell");
        return;
    }
    if let Some((at, rad)) = moved.noise {
        noise(r, at, rad, false);
    }
    if moved.step {
        sounds.push(if matches!(r.p.gait, Gait::Run | Gait::Sprint) {
            "step"
        } else {
            "step_soft"
        });
    }
    // portals: walking through
    if r.portal_cd <= 0 {
        if let [Some(a), Some(b)] = r.portals {
            for (from, to) in [(a, b), (b, a)] {
                // the way she is trying to go: a wall stops the step but not the intent
                let intent = if wish != V3::ZERO {
                    wish.scale(r.p.vel.len().max(SPEED_CROUCH)).add(r.p.push)
                } else {
                    r.p.vel
                };
                if portal::enters(&from, r.p.pos, intent, r.p.vy) {
                    let e = portal::exit(&from, &to, r.p.yaw, intent, r.p.vy);
                    r.p.pos = e.pos;
                    r.p.yaw = (r.p.yaw + e.turn) & (fx::TURN - 1);
                    r.cam_yaw = (r.cam_yaw + e.turn) & (fx::TURN - 1);
                    r.p.push = e.push;
                    r.p.vy = e.vy;
                    r.p.airborne = true;
                    r.p.fall_top = r.p.pos.y;
                    r.portal_cd = PORTAL_COOLDOWN;
                    sounds.push("warp");
                    break;
                }
            }
        }
    }
    // portals: opening and shutting
    for (k, bit) in [(0, PORTAL_A), (1, PORTAL_B)] {
        if on(bit) && r.hold.is_none() {
            let (o, d) = aim_ray(r);
            match portal::place(&r.world, o, d, r.portals[1 - k].as_ref()) {
                Ok(p) => {
                    r.portals[k] = Some(p);
                    sounds.push(if k == 0 { "portal_a" } else { "portal_b" });
                }
                Err(_) => sounds.push("portal_fail"),
            }
        }
    }
    if on(CLOSE) {
        if let Some((i, HoldAct::Grip)) = r.hold {
            kill(r, i);
            sounds.push("kill");
        } else if r.portals.iter().any(|p| p.is_some()) {
            r.portals = [None, None];
            sounds.push("portal_shut");
        }
    }
    // the pistol
    if on(FIRE) && r.fire_cd <= 0 && !busy {
        shoot(r, sounds);
    }
    // marking: holding the sights on a soldier
    if r.aiming {
        let (who, _, _) = aimed(r, MARK_RANGE);
        r.mark = match (who, r.mark) {
            (Some(i), Some((j, t))) if i == j => Some((i, t + dt)),
            (Some(i), _) => Some((i, 0)),
            _ => None,
        };
        if let Some((i, t)) = r.mark {
            if t >= MARK_TIME && !r.soldiers[i].marked {
                r.soldiers[i].marked = true;
                sounds.push("mark");
            }
        }
    } else {
        r.mark = None;
    }
    // CQC
    if on(CQC) {
        cqc(r, sounds, toasts);
    }
    if let Some((i, act)) = r.hold {
        let ahead = r.p.pos.add(facing(r.p.yaw, 0).scale(ratio(55, 100)));
        r.soldiers[i].pos = v3(ahead.x, r.p.pos.y, ahead.z);
        r.soldiers[i].yaw = r.p.yaw;
        match act {
            HoldAct::Talk(t) => {
                if t + dt >= INTERROGATE_TIME {
                    r.hold = Some((i, HoldAct::Grip));
                    interrogate(r, m, i, false, toasts, sounds);
                } else {
                    r.hold = Some((i, HoldAct::Talk(t + dt)));
                }
            }
            HoldAct::Choke(t) => {
                if t + dt >= CHOKE_TIME {
                    knock_out(r, i, Life::Asleep(SLEEP_TIME));
                    sounds.push("thud");
                } else {
                    r.hold = Some((i, HoldAct::Choke(t + dt)));
                }
            }
            HoldAct::Grip => {}
        }
    }
    if on(USE) {
        use_press(r, m, sounds, toasts);
    }
    if on(FULTON) {
        fulton(r, sounds, toasts);
    }
    if on(TIE) {
        tie(r, sounds, toasts);
    }
    if on(WHISTLE) {
        if r.mule.mode != MuleMode::Come {
            r.mule.mode = MuleMode::Come;
            r.mule.goal = None;
        }
        sounds.push("whistle");
    }
    if on(RAM) {
        let (who, _, _) = aimed(r, MULE_RAM_RANGE);
        let target = who.filter(|&i| r.soldiers[i].free()).or_else(|| {
            let mut best: Option<(usize, i32)> = None;
            for (i, s) in r.soldiers.iter().enumerate() {
                let d = s.pos.sub(r.p.pos);
                let off = fx::wrap(fx::atan2(-d.x, -d.z) - r.cam_yaw).abs();
                if s.free() && d.len() < int(30) && off < deg(25) && best.is_none_or(|b| d.len() < b.1) {
                    best = Some((i, d.len()));
                }
            }
            best.map(|b| b.0)
        });
        match target {
            Some(i) if r.mule.load.is_none() && r.soldiers[i].pos.y < ONE => {
                r.mule.mode = MuleMode::Charge(i);
                r.mule.goal = None;
                sounds.push("bray");
            }
            _ => toasts.push("@mule_no_target".into()),
        }
    }
    // carried things follow
    if let Some(c) = r.carry {
        let shoulder = r.p.pos.add(v3(0, ratio(15, 10), 0));
        match c {
            Carry::Soldier(i) => r.soldiers[i].pos = shoulder,
            Carry::Commando => r.prisoner_at = shoulder,
        }
    }
    let u = &uniforms[r.uniform.min(uniforms.len() - 1)];
    r.camo = player::camo_index(&r.p, &r.world, u, r.cloak).index;
    // the landing zone
    let lz = v3(m.lz.x, 0, m.lz.z);
    if v3(r.p.pos.x, 0, r.p.pos.z).dist(lz) < LZ_RADIUS && objective_done(r, m) {
        if r.carry == Some(Carry::Commando) {
            r.prisoner = Prisoner::Gone;
        }
        finish(r);
    }
}

fn fail(r: &mut Run, cause: &'static str) {
    r.outcome = Some(Outcome {
        done: false,
        cause,
        rank: String::new(),
        secs: r.elapsed / ONE,
        alerts: r.alerts,
        kills: r.kills,
        recruits: 0,
        intel: r.revealed.iter().filter(|x| **x).count(),
    });
}

fn finish(r: &mut Run) {
    let secs = r.elapsed / ONE;
    let rank = if r.alerts == 0 && r.kills == 0 && secs < 900 {
        "S"
    } else if r.alerts <= 1 && r.kills <= 2 {
        "A"
    } else if r.alerts <= 3 {
        "B"
    } else {
        "C"
    };
    r.outcome = Some(Outcome {
        done: true,
        cause: "",
        rank: rank.into(),
        secs,
        alerts: r.alerts,
        kills: r.kills,
        recruits: r.recruited.len(),
        intel: r.revealed.iter().filter(|x| **x).count(),
    });
}

fn shoot(r: &mut Run, sounds: &mut Vec<&'static str>) {
    let k = if r.lethal { 1 } else { 0 };
    if r.ammo[k] <= 0 {
        sounds.push("empty");
        r.fire_cd = FIRE_COOLDOWN;
        return;
    }
    r.ammo[k] -= 1;
    r.fire_cd = FIRE_COOLDOWN;
    r.p.boxed = false;
    let (who, at, head) = aimed(r, PISTOL_RANGE);
    let gun =
        r.p.pos
            .add(v3(0, ratio(13, 10), 0))
            .add(right_of(r.p.yaw).scale(ratio(3, 10)));
    r.tracers.push(Tracer {
        from: gun,
        to: at,
        t: TRACER_TIME,
    });
    if r.lethal {
        sounds.push("lethal");
        r.cloak = false;
        let here = r.p.pos;
        noise(r, here, NOISE_LETHAL, true);
    } else {
        sounds.push("tranq");
    }
    match who {
        Some(i) => {
            sounds.push("hit");
            let s = &r.soldiers[i];
            if r.lethal {
                if s.life != Life::Dead && s.life != Life::Gone {
                    kill(r, i);
                }
            } else if s.awake() {
                if head {
                    knock_out(r, i, Life::Asleep(SLEEP_TIME));
                    sleeper_dropped(r, i, sounds);
                } else if s.life == Life::Awake {
                    let me = r.p.pos;
                    let s = &mut r.soldiers[i];
                    s.life = Life::Dart(DART_DELAY);
                    s.awareness = s.awareness.max(int(55));
                    if !matches!(s.mind, Mind::Combat) {
                        s.mind = Mind::Suspicious {
                            at: me,
                            t: LOOK_AROUND,
                        };
                    }
                }
            }
        }
        None => noise(r, at, NOISE_RICOCHET, false),
    }
}

fn cqc(r: &mut Run, sounds: &mut Vec<&'static str>, toasts: &mut Vec<String>) {
    if let Some((i, HoldAct::Grip)) = r.hold {
        r.hold = Some((i, HoldAct::Choke(0)));
        sounds.push("choke");
        return;
    }
    if r.hold.is_some() || r.carry.is_some() || r.p.boxed {
        return;
    }
    let me = r.p.pos;
    let Some(i) = nearest_soldier(r, me, THROW_REACH, |s| s.free()) else {
        return;
    };
    let s = &r.soldiers[i];
    let d = me.sub(s.pos);
    let off = fx::wrap(fx::atan2(-d.x, -d.z) - s.gaze()).abs();
    let near = d.len_xz() < GRAB_REACH;
    let aware = matches!(s.mind, Mind::Combat) || s.awareness >= int(SPOTTED_AT);
    if near && off > GRAB_BEHIND && !aware {
        r.soldiers[i].held = true;
        r.soldiers[i].path.clear();
        r.hold = Some((i, HoldAct::Grip));
        if let Some((k, _)) = r.reflex {
            if k == i {
                r.reflex = None;
            }
        }
        r.cloak = false;
        sounds.push("grab");
    } else {
        // from the front: take the rifle off him and put him on the ground
        knock_out(r, i, Life::Stunned(STUN_THROW));
        noise(r, me, NOISE_THROW, false);
        sounds.push("throw");
        toasts.push("@disarmed".into());
    }
}

fn use_press(r: &mut Run, m: &Mission, sounds: &mut Vec<&'static str>, toasts: &mut Vec<String>) {
    match what_use(r, m) {
        Use::Nothing | Use::LeaveLocker => {}
        Use::Talk => {
            if let Some((i, _)) = r.hold {
                r.hold = Some((i, HoldAct::Talk(0)));
                sounds.push("talk");
            }
        }
        Use::Stash(k) => {
            if let Some(Carry::Soldier(i)) = r.carry {
                let s = &mut r.soldiers[i];
                s.carried = false;
                s.locker = Some(k);
                s.pos = r.world.lockers[k].at;
                r.carry = None;
                sounds.push("locker");
            }
        }
        Use::Drop => {
            let at = r.p.pos.add(facing(r.p.yaw, 0).scale(ratio(8, 10)));
            let at = v3(at.x, r.world.ground(at, RADIUS / 2, r.p.pos.y + STEP), at.z);
            match r.carry {
                Some(Carry::Soldier(i)) => {
                    r.soldiers[i].carried = false;
                    r.soldiers[i].pos = at;
                }
                Some(Carry::Commando) => {
                    r.prisoner = Prisoner::Free;
                    r.prisoner_at = at;
                }
                None => {}
            }
            r.carry = None;
            sounds.push("drop");
        }
        Use::OpenCage => {
            r.world.open_cage();
            r.prisoner = Prisoner::Free;
            r.target_known = true;
            toasts.push("@prisoner_found".into());
            sounds.push("cage");
        }
        Use::CarryPrisoner => {
            r.prisoner = Prisoner::Carried;
            r.carry = Some(Carry::Commando);
            sounds.push("lift");
        }
        Use::TakeCase => {
            r.case_taken = true;
            r.target_known = true;
            toasts.push("@case_taken".into());
            sounds.push("pickup");
        }
        Use::CarryBody(i) => {
            r.soldiers[i].carried = true;
            r.carry = Some(Carry::Soldier(i));
            sounds.push("lift");
        }
        Use::Hide(k) => {
            r.p.locker = Some(k);
            r.p.boxed = false;
            r.cloak = false;
            sounds.push("locker");
        }
    }
}

fn fulton(r: &mut Run, sounds: &mut Vec<&'static str>, toasts: &mut Vec<String>) {
    let Some(who) = fulton_target(r) else {
        toasts.push("@fulton_nothing".into());
        return;
    };
    let at = match who {
        Who::Soldier(i) => r.soldiers[i].pos,
        Who::Commando => r.prisoner_at,
    };
    let at = if r.carry.is_some() {
        r.p.pos.add(facing(r.p.yaw, 0).scale(ONE))
    } else {
        at
    };
    if r.world.covered(v3(at.x, at.y.min(r.p.pos.y), at.z)) || r.world.covered(r.p.pos) {
        toasts.push("@fulton_roof".into());
        return;
    }
    if r.fultons <= 0 {
        toasts.push("@fulton_none".into());
        return;
    }
    r.fultons -= 1;
    match who {
        Who::Soldier(i) => {
            let s = &mut r.soldiers[i];
            s.carried = false;
            s.held = false;
            if s.on_mule {
                s.on_mule = false;
                r.mule.load = None;
            }
            s.life = Life::Gone;
            if matches!(r.carry, Some(Carry::Soldier(j)) if j == i) {
                r.carry = None;
            }
        }
        Who::Commando => {
            r.prisoner = Prisoner::Gone;
            if r.carry == Some(Carry::Commando) {
                r.carry = None;
            }
        }
    }
    r.balloons.push(Balloon {
        who,
        at: v3(at.x, at.y.max(0), at.z),
        t: 0,
    });
    sounds.push("fulton");
}

fn tie(r: &mut Run, sounds: &mut Vec<&'static str>, toasts: &mut Vec<String>) {
    let me = r.p.pos;
    if let Some(i) = r.mule.load {
        if r.mule.pos.dist(me) < MULE_TIE_REACH {
            let s = &mut r.soldiers[i];
            s.on_mule = false;
            s.pos = r.mule.pos.add(right_of(r.mule.yaw).scale(int(1)));
            s.pos.y = 0;
            r.mule.load = None;
            sounds.push("drop");
        }
        return;
    }
    if let Some(Carry::Soldier(i)) = r.carry {
        if r.soldiers[i].life != Life::Dead {
            r.soldiers[i].carried = false;
            r.carry = None;
        }
    }
    let Some(i) = nearest_soldier(r, me, REACH * 2, |s| {
        (lying(s) || s.carried) && s.life != Life::Dead
    }) else {
        toasts.push("@tie_nothing".into());
        return;
    };
    if r.mule.pos.dist(r.soldiers[i].pos) > MULE_TIE_REACH * 2 {
        toasts.push("@mule_far".into());
        return;
    }
    let s = &mut r.soldiers[i];
    s.life = Life::Bound;
    s.on_mule = true;
    s.carried = false;
    r.mule.load = Some(i);
    r.mule.talk = INTERROGATE_TIME;
    r.mule.mode = MuleMode::Follow;
    sounds.push("bray");
    sounds.push("tie");
}

// --- the soldiers' tick ------------------------------------------------------------------

fn target_of(r: &Run) -> Target {
    let p = &r.p;
    Target {
        pos: p.pos,
        chest: p.chest(),
        vis: (100 - r.camo).clamp(5, 160),
        hidden: p.locker.is_some(),
        moving: p.gait != Gait::Still,
        sprinting: p.gait == Gait::Sprint,
        stance_hit: match p.stance {
            Stance::Stand => 0,
            Stance::Crouch => HIT_CROUCH,
            Stance::Prone => HIT_PRONE,
        },
    }
}

fn soldiers_tick(r: &mut Run, nav: &Nav, dt: i32, sounds: &mut Vec<&'static str>, toasts: &mut Vec<String>) {
    let player = target_of(r);
    let bodies: Vec<Body> = r
        .soldiers
        .iter()
        .enumerate()
        .filter(|(_, s)| lying(s) && !r.world.in_bush(s.pos))
        .map(|(i, s)| Body {
            idx: i,
            at: s.pos,
            dead: s.life == Life::Dead,
            bound: s.life == Life::Bound,
        })
        .collect();
    let portals = [r.portals[0].map(|p| p.at), r.portals[1].map(|p| p.at)];
    let balloons: Vec<V3> = r
        .balloons
        .iter()
        .map(|b| b.at.add(v3(0, int(3) + balloon_rise(b.t), 0)))
        .collect();
    let mut events = Vec::new();
    let mut dropped = Vec::new();
    let mut woke = Vec::new();
    let mut seen = false;
    {
        let ctx = Ctx {
            world: &r.world,
            nav,
            player,
            phase: r.phase,
            lkp: r.lkp,
            bodies: &bodies,
            portals,
            balloons: &balloons,
            tick: r.tick,
            reflex: r.reflex.is_some(),
        };
        let rng = &mut r.rng;
        for (i, s) in r.soldiers.iter_mut().enumerate() {
            if s.life == Life::Gone {
                continue;
            }
            let before = s.life;
            if s.sleep_tick(dt) {
                woke.push(i);
            }
            if matches!(before, Life::Dart(_)) && matches!(s.life, Life::Asleep(_)) {
                dropped.push(i);
            }
            if !s.free() || s.life != Life::Awake && !matches!(s.life, Life::Dart(_)) {
                continue;
            }
            if matches!(s.life, Life::Dart(_)) {
                continue;
            }
            let sees = s.watch(&ctx, dt, &mut events);
            if (r.tick as usize + i).is_multiple_of(6) {
                s.scan(&ctx);
            }
            s.think(&ctx, rng, dt, sees, &mut events);
        }
    }
    for i in dropped {
        sleeper_dropped(r, i, sounds);
    }
    for i in woke {
        // he wakes, radios it in, and starts looking
        if matches!(r.carry, Some(Carry::Soldier(j)) if j == i) {
            r.carry = None;
        }
        let s = &mut r.soldiers[i];
        s.carried = false;
        if let Some(k) = s.locker.take() {
            let l = r.world.lockers[k];
            s.pos = l.at.add(l.front.scale(int(1)));
        }
        s.pos.y = s.pos.y.max(0);
        let at = s.pos;
        s.stand_down(Some(at));
        caution(r);
        sounds.push("radio");
        toasts.push("@radio_woke".into());
    }
    for e in events {
        match e {
            Event::Spotted(i) => match r.phase {
                Phase::Combat(_) => {}
                Phase::Evasion(_) => start_combat(r, sounds),
                _ => {
                    if r.reflex.is_none() {
                        r.reflex = Some((i, REFLEX_TIME));
                        r.lkp = r.p.pos;
                        sounds.push("spotted");
                    }
                }
            },
            Event::Noticed(i) => {
                if r.soldiers[i].pos.dist(r.p.pos) < int(45) {
                    sounds.push("huh");
                }
            }
            Event::Sees(at) => {
                r.lkp = at;
                seen = true;
            }
            Event::Shot { from, hit } => {
                let chest = r.p.chest();
                let miss = if hit {
                    V3::ZERO
                } else {
                    v3(
                        ratio(r.rng.range(-80, 80), 100),
                        ratio(r.rng.range(-40, 60), 100),
                        0,
                    )
                };
                r.tracers.push(Tracer {
                    from,
                    to: chest.add(miss),
                    t: TRACER_TIME,
                });
                sounds.push("enemy_shot");
                if hit && r.outcome.is_none() {
                    fail(r, "shot");
                }
            }
            Event::FoundDead(_) => caution(r),
            Event::Woke { by: _, buddy } => {
                let b = &mut r.soldiers[buddy];
                if b.life == Life::Dead {
                    toasts.push("@radio_body".into());
                } else if b.life != Life::Gone && !b.carried && !b.on_mule && b.locker.is_none() {
                    b.life = Life::Awake;
                    let at = b.pos;
                    b.stand_down(Some(at));
                    toasts.push("@radio_found".into());
                }
                caution(r);
                sounds.push("radio");
            }
            Event::ShutPortal(k) => {
                if r.portals[k].is_some() {
                    r.portals[k] = None;
                    sounds.push("portal_shut");
                    toasts.push("@portal_seen".into());
                    caution(r);
                }
            }
        }
    }
    // reflex mode: the spotter, slowed, until you stop him or it runs out
    if let Some((i, t)) = r.reflex {
        let s = &r.soldiers[i];
        if !s.free() || s.life != Life::Awake {
            r.reflex = None;
            sounds.push("reflex_saved");
        } else if t - dt <= 0 {
            start_combat(r, sounds);
        } else {
            r.reflex = Some((i, t - dt));
        }
    }
    r.phase = match r.phase {
        Phase::Combat(lost) => {
            let lost = if seen { 0 } else { lost + dt };
            if lost > COMBAT_LOST {
                let at = r.lkp;
                for s in r.soldiers.iter_mut() {
                    if matches!(s.mind, Mind::Combat) {
                        s.stand_down(Some(at));
                    }
                }
                sounds.push("evasion");
                Phase::Evasion(EVASION_TIME)
            } else {
                Phase::Combat(lost)
            }
        }
        Phase::Evasion(t) if t - dt <= 0 => Phase::Caution(CAUTION_TIME),
        Phase::Evasion(t) => Phase::Evasion(t - dt),
        Phase::Caution(t) if t - dt <= 0 => Phase::Normal,
        Phase::Caution(t) => Phase::Caution(t - dt),
        Phase::Normal => Phase::Normal,
    };
}

// --- the mule ----------------------------------------------------------------------------------

fn mule_tick(
    r: &mut Run,
    nav: &Nav,
    m: &Mission,
    dt: i32,
    sounds: &mut Vec<&'static str>,
    toasts: &mut Vec<String>,
) {
    // his nose: soldiers near him are marked
    r.mule.sniff -= dt;
    if r.mule.sniff <= 0 {
        r.mule.sniff = MULE_SNIFF_EVERY;
        let at = r.mule.pos;
        for s in r.soldiers.iter_mut() {
            if s.awake() && !s.marked && s.pos.dist(at) < r.sniff_range {
                s.marked = true;
            }
        }
    }
    if let Some(i) = r.mule.load {
        if r.mule.talk > 0 {
            r.mule.talk -= dt;
            if r.mule.talk <= 0 {
                interrogate(r, m, i, true, toasts, sounds);
            }
        }
        r.soldiers[i].pos = r.mule.pos.add(v3(0, ratio(17, 10), 0));
    }
    let me = v3(r.p.pos.x, 0, r.p.pos.z);
    let (goal, speed) = match r.mule.mode {
        MuleMode::Charge(i) => {
            let s = &r.soldiers[i];
            if !s.free() {
                r.mule.mode = MuleMode::Follow;
                (None, 0)
            } else if s.pos.dist(r.mule.pos) < ratio(13, 10) {
                knock_out(r, i, Life::Stunned(STUN_RAM));
                r.soldiers[i].path.clear();
                sounds.push("ram");
                r.mule.mode = MuleMode::Follow;
                (None, 0)
            } else {
                (Some(v3(s.pos.x, 0, s.pos.z)), MULE_CHARGE)
            }
        }
        MuleMode::Come => {
            if r.mule.pos.dist(me) < int(3) {
                r.mule.mode = MuleMode::Follow;
                (None, 0)
            } else {
                (Some(me), MULE_RUN)
            }
        }
        MuleMode::Follow => {
            let d = r.mule.pos.dist(me);
            if d > MULE_FOLLOW {
                (Some(me), if d > MULE_FAR { MULE_RUN } else { MULE_WALK })
            } else {
                (None, 0)
            }
        }
    };
    r.mule.moving = false;
    let Some(goal) = goal else {
        r.mule.path.clear();
        return;
    };
    let stale = r.mule.goal.is_none_or(|g| g.dist(goal) > int(3)) || r.mule.path.is_empty();
    if stale && (r.tick.is_multiple_of(10) || r.mule.goal.is_none()) {
        // off the grid (on the ridge), or no route: straight at it, sliding along what is in the way
        r.mule.path = if nav.straight(&r.world, r.mule.pos, goal) || r.mule.pos.y > ONE {
            vec![goal]
        } else {
            nav.path(&r.world, r.mule.pos, goal).unwrap_or_else(|| vec![goal])
        };
        r.mule.goal = Some(goal);
    }
    // stop short of the player
    if matches!(r.mule.mode, MuleMode::Follow | MuleMode::Come)
        && r.mule.path.len() == 1
        && r.mule.pos.dist(me) < MULE_FOLLOW
    {
        r.mule.path.clear();
    }
    let (p, h) = nav::follow(r.mule.pos, &mut r.mule.path, speed, dt);
    let mut p = r.world.collide(p, ratio(5, 10), ratio(16, 10));
    let ground = r.world.ground(p, ratio(4, 10), r.mule.pos.y);
    p.y = if ground < r.mule.pos.y {
        (r.mule.pos.y - fx::mul(int(8), dt)).max(ground)
    } else {
        ground
    };
    r.mule.moving = p != r.mule.pos;
    r.mule.pos = p;
    if let Some(h) = h {
        let d = fx::wrap(h - r.mule.yaw);
        let max = fx::mul(deg(400), dt);
        r.mule.yaw = (r.mule.yaw + d.clamp(-max, max)) & (fx::TURN - 1);
    }
    r.mule.walk = (r.mule.walk + fx::mul(speed, dt) * 3) & (fx::TURN - 1);
}

// --- balloons, tracers ---------------------------------------------------------------------

fn balloon_rise(t: i32) -> i32 {
    // slow at first, then up and away
    fx::mul(fx::mul(t, t), int(5))
}

fn recruit(r: &mut Run, i: usize, recruits: &Recruits, names: &mut Rng) -> Recruit {
    let s = &r.soldiers[i];
    let a = names.range(0, recruits.first.len() as i32 - 1) as usize;
    let b = names.range(0, recruits.second.len() as i32 - 1) as usize;
    let top = recruits.grades.len() as i32 - 1;
    let grade = if s.elite || s.officer {
        names.range(3, top)
    } else {
        names.range(0, 3.min(top))
    };
    Recruit {
        name: format!("{} {}", recruits.first[a], recruits.second[b]),
        unit: names.range(0, 2) as usize,
        grade: grade as usize,
    }
}

fn world_tick(
    r: &mut Run,
    m: &Mission,
    dt: i32,
    sounds: &mut Vec<&'static str>,
    toasts: &mut Vec<String>,
    recruits: &Recruits,
    names: &mut Rng,
) {
    let _ = m;
    for t in r.tracers.iter_mut() {
        t.t -= dt;
    }
    r.tracers.retain(|t| t.t > 0);
    let mut done = Vec::new();
    for (k, b) in r.balloons.iter_mut().enumerate() {
        b.t += dt;
        if b.t >= FULTON_TIME {
            done.push(k);
        }
    }
    for k in done.into_iter().rev() {
        let b = r.balloons.remove(k);
        if let Who::Soldier(i) = b.who {
            let rec = recruit(r, i, recruits, names);
            toasts.push(format!("@recruited:{}", rec.name));
            r.recruited.push(rec);
            sounds.push("recruit");
        } else {
            toasts.push("@prisoner_out".into());
        }
    }
}

// --- drawing ------------------------------------------------------------------------------------

fn rgb_hex(s: &str) -> Rgb {
    crate::content::hex(s).unwrap_or([ONE, 0, ONE])
}

impl Game {
    fn soldier_look(&self, s: &Soldier) -> Look {
        let p = &self.palette;
        let (body, dark) = if s.elite {
            (p.get("elite"), p.get("elite_dark"))
        } else if s.officer {
            (p.get("officer"), p.get("soldier_dark"))
        } else {
            (p.get("soldier"), p.get("soldier_dark"))
        };
        Look {
            body,
            dark,
            skin: p.pick("skin", s.id * 3 + 1),
            boots: p.get("boots"),
            hat: if s.officer {
                Hat::Beret(p.get("beret"))
            } else {
                Hat::Helmet(if s.elite {
                    p.get("elite_dark")
                } else {
                    p.get("helmet")
                })
            },
            goggles: if s.elite { Some(p.get("lens")) } else { None },
            style: 0,
        }
    }

    fn commando_look(&self) -> Look {
        let p = &self.palette;
        Look {
            body: p.get("commando"),
            dark: p.get("gear"),
            skin: p.pick("skin", 3),
            boots: p.get("boots"),
            hat: Hat::Hair(p.get("hair")),
            goggles: None,
            style: 0,
        }
    }

    fn frame(&mut self) {
        let mut b = Batch::new();
        let p = &self.palette;
        let Some(r) = self.run.as_ref() else {
            let a = self.orbit;
            let eye = v3(
                fx::mul(fx::sin(a), int(140)),
                int(60),
                fx::mul(fx::cos(a), int(140)),
            );
            self.cam = Camera {
                eye,
                fwd: v3(0, int(4), 0).sub(eye).norm(),
                fov: FOV,
                far: FAR,
            };
            let (v, runs) = b.finish();
            self.dynamic_vals = v;
            self.dynamic_runs = runs;
            return;
        };
        let m = &self.missions[r.m];
        self.cam = camera(r);
        // the operative
        let u = &self.uniforms[r.uniform.min(self.uniforms.len() - 1)];
        let me = Look {
            body: rgb_hex(&u.base),
            dark: rgb_hex(&u.blotch),
            skin: p.get("operative_skin"),
            boots: p.get("boots"),
            hat: Hat::Hair(p.get("hair")),
            goggles: None,
            style: if r.cloak { STYLE_CLOAK } else { STYLE_CAMO },
        };
        if r.p.locker.is_none() {
            let moving = r.p.gait != Gait::Still;
            let walk = r.p.walk_phase;
            if r.p.boxed {
                draw::cardboard(&mut b, r.p.pos, r.p.yaw, p.get("box"));
            } else {
                let pose = match r.p.stance {
                    Stance::Stand => Pose::Upright {
                        walk,
                        moving,
                        aim: r.aiming,
                    },
                    Stance::Crouch => Pose::Crouch {
                        walk,
                        moving,
                        aim: r.aiming,
                    },
                    Stance::Prone => Pose::Prone { walk },
                };
                draw::person(&mut b, r.p.pos, r.p.yaw, pose, &me);
            }
        }
        // soldiers
        let near = |at: V3| v3(at.x - r.p.pos.x, 0, at.z - r.p.pos.z).len() < int(75);
        for s in &r.soldiers {
            if s.life == Life::Gone || s.locker.is_some() {
                continue;
            }
            let look = self.soldier_look(s);
            if s.carried || s.on_mule {
                draw::draped(&mut b, s.pos, if s.on_mule { r.mule.yaw } else { r.p.yaw }, &look);
                continue;
            }
            let pose = if s.held {
                Pose::Held
            } else if s.life == Life::Bound {
                Pose::Bound
            } else if s.down() {
                Pose::Lying
            } else {
                Pose::Upright {
                    walk: s.walk_phase,
                    moving: s.moving,
                    aim: matches!(s.mind, Mind::Combat),
                }
            };
            draw::person(&mut b, s.pos, s.yaw, pose, &look);
            if s.free() && s.life == Life::Awake && (near(s.pos) || s.marked) {
                let rgb = match s.mind {
                    Mind::Combat => p.get("cone_combat"),
                    Mind::Patrol => p.get("cone_calm"),
                    _ => p.get("cone_alert"),
                };
                let range = Soldier::sight(r.phase);
                let alpha = if matches!(s.mind, Mind::Patrol) {
                    ratio(30, 100)
                } else {
                    ratio(38, 100)
                };
                draw::cone(
                    &mut b,
                    &r.world,
                    s.eyes(),
                    s.pos.y,
                    s.gaze(),
                    soldier_fan(r.phase),
                    range,
                    11,
                    rgb,
                    alpha,
                );
                draw::cone(
                    &mut b,
                    &r.world,
                    s.eyes(),
                    s.pos.y,
                    s.gaze(),
                    crate::balance::FAN_NEAR,
                    crate::balance::NEAR_SIGHT,
                    7,
                    rgb,
                    alpha / 2,
                );
            }
        }
        // the mule
        let ml = MuleLook {
            coat: p.get("mule"),
            light: p.get("mule_light"),
            dark: p.get("mule_dark"),
            bandana: p.get("bandana"),
            saddle: p.get("saddle"),
        };
        draw::mule(
            &mut b,
            r.mule.pos,
            r.mule.yaw,
            r.mule.walk,
            r.mule.moving,
            matches!(r.mule.mode, MuleMode::Charge(_)),
            &ml,
        );
        // portals
        for (k, pt) in r.portals.iter().enumerate() {
            if let Some(pt) = pt {
                let rgb = p.get(if k == 0 { "portal_a" } else { "portal_b" });
                draw::portal(&mut b, pt.at, pt.n, rgb, PORTAL_W, PORTAL_H, ONE);
            }
        }
        // the prisoner and the case
        let cl = self.commando_look();
        match r.prisoner {
            Prisoner::Caged => draw::person(
                &mut b,
                r.prisoner_at,
                deg(90),
                Pose::Crouch {
                    walk: 0,
                    moving: false,
                    aim: false,
                },
                &cl,
            ),
            Prisoner::Free => draw::person(&mut b, r.prisoner_at, deg(90), Pose::Lying, &cl),
            Prisoner::Carried => draw::draped(&mut b, r.prisoner_at, r.p.yaw, &cl),
            Prisoner::Gone => {}
        }
        if !r.case_taken {
            if let Some(c) = m.target_place {
                b.push(
                    LAYER_INKED,
                    Mesh::Cube,
                    &render::standing(c, v3(ratio(7, 10), ratio(35, 100), ratio(45, 100))),
                    p.get("case"),
                    0,
                );
                b.push(
                    LAYER_INKED,
                    Mesh::Cube,
                    &render::standing(
                        c.add(v3(0, ratio(35, 100), 0)),
                        v3(ratio(5, 10), ratio(6, 100), ratio(16, 100)),
                    ),
                    p.get("lens"),
                    STYLE_GLOW,
                );
            }
        }
        // the landing zone: a green glow you can see from far off
        b.tint(
            Mesh::Cyl,
            &render::standing(v3(m.lz.x, 0, m.lz.z), v3(LZ_RADIUS * 2, int(26), LZ_RADIUS * 2)),
            p.get("lz"),
            ratio(14, 100),
        );
        b.push(
            render::LAYER_PLAIN,
            Mesh::Cyl,
            &render::standing(
                v3(m.lz.x, 0, m.lz.z),
                v3(LZ_RADIUS * 2, ratio(4, 100), LZ_RADIUS * 2),
            ),
            p.get("lz"),
            STYLE_GLOW,
        );
        // balloons
        for bl in &r.balloons {
            let rise = balloon_rise(bl.t);
            let body = bl.at.add(v3(0, rise + ratio(5, 10), 0));
            draw::balloon(&mut b, body.add(v3(0, int(3), 0)), body, p.get("balloon"));
            let look = match bl.who {
                Who::Soldier(i) => self.soldier_look(&r.soldiers[i]),
                Who::Commando => cl,
            };
            draw::draped(&mut b, body, 0, &look);
        }
        for t in &r.tracers {
            draw::tracer(&mut b, t.from, t.to, p.get("tracer"));
        }
        let (v, runs) = b.finish();
        self.dynamic_vals = v;
        self.dynamic_runs = runs;
    }

    // --- what the page reads ------------------------------------------------------------------

    pub fn scenery(&self) -> Vec<i32> {
        self.scenery_vals.clone()
    }

    pub fn scenery_version(&self) -> u32 {
        self.scenery_version
    }

    pub fn dynamic(&self) -> Vec<i32> {
        self.dynamic_vals.clone()
    }

    pub fn draws(&self) -> Vec<u32> {
        render::draws(&self.scenery_runs, &self.dynamic_runs)
    }

    pub fn mats(&self) -> Vec<i32> {
        self.cam.mat_values(self.aspect()).to_vec()
    }

    pub fn mesh(&self, id: u32) -> Vec<i32> {
        Mesh::ALL
            .get(id as usize)
            .map(|m| mesh::vertices(*m))
            .unwrap_or_default()
    }

    pub fn uniforms(&self) -> Vec<i32> {
        let p = &self.palette;
        let light = v3(ratio(-4, 10), ratio(8, 10), ratio(45, 100)).norm();
        let mut out = vec![light.x, light.y, light.z];
        for name in ["sky", "fog", "light", "ink"] {
            out.extend_from_slice(&p.get(name));
        }
        out.extend_from_slice(&[int(120), int(380)]);
        // the ink line: least width, growth per metre, most width
        out.extend_from_slice(&[ratio(15, 1000), ratio(15, 10000), ratio(5, 100)]);
        out
    }

    pub fn clear_color(&self) -> Vec<i32> {
        self.palette.get("sky").to_vec()
    }

    fn screen(&self, at: V3) -> Option<(i32, i32)> {
        let (x, y) = self.cam.screen(self.aspect(), at)?;
        if !(0..=int(100)).contains(&x) || !(0..=int(100)).contains(&y) {
            return None;
        }
        Some((x / ONE, y / ONE))
    }

    /// The HUD, as JSON: everything the page shows during play, and what
    /// each screen needs.
    pub fn hud_json(&self) -> String {
        let mut h = json!({
            "mode": self.mode.name(),
            "sounds": self.sounds,
            "toasts": self.toasts,
        });
        let Some(r) = self.run.as_ref() else {
            return h.to_string();
        };
        let m = &self.missions[r.m];
        let (phase, phase_secs) = match r.phase {
            Phase::Normal => ("normal", 0),
            Phase::Caution(t) => ("caution", t / ONE),
            Phase::Evasion(t) => ("evasion", t / ONE),
            Phase::Combat(_) => ("combat", 0),
        };
        let stance = if r.p.locker.is_some() {
            "locker"
        } else if r.p.boxed {
            "box"
        } else {
            match r.p.stance {
                Stance::Stand => "stand",
                Stance::Crouch => "crouch",
                Stance::Prone => "prone",
            }
        };
        let u = &self.uniforms[r.uniform.min(self.uniforms.len() - 1)];
        // markers on screen: marked soldiers, the objective once known, the LZ, revealed places
        let mut markers = Vec::new();
        for s in &r.soldiers {
            if s.life == Life::Gone || s.locker.is_some() || s.carried || s.on_mule || s.held {
                continue;
            }
            let show = s.marked || (s.target && r.target_known);
            if !show {
                continue;
            }
            let kind = if s.target && r.target_known {
                "target"
            } else if s.down() {
                "down"
            } else if s.elite {
                "elite"
            } else {
                "soldier"
            };
            if let Some((x, y)) = self.screen(s.pos.add(v3(0, ratio(22, 10), 0))) {
                markers.push(json!([x, y, kind, ""]));
            }
        }
        let dist = |at: V3| format!("{}", r.p.pos.dist(at) / ONE);
        if let Some((x, y)) = self.screen(v3(m.lz.x, int(4), m.lz.z)) {
            markers.push(json!([x, y, "lz", dist(m.lz)]));
        }
        if r.target_known && m.kind != Kind::Eliminate {
            let at = if m.kind == Kind::Rescue {
                r.prisoner_at
            } else {
                m.target_place.unwrap_or(V3::ZERO)
            };
            let pending = (m.kind == Kind::Rescue && matches!(r.prisoner, Prisoner::Caged | Prisoner::Free))
                || (m.kind == Kind::Recover && !r.case_taken);
            if pending {
                if let Some((x, y)) = self.screen(at.add(v3(0, int(2), 0))) {
                    markers.push(json!([x, y, "objective", dist(at)]));
                }
            }
        }
        for pl in &r.places {
            if let Some((x, y)) = self.screen(pl.add(v3(0, int(2), 0))) {
                markers.push(json!([x, y, "place", dist(*pl)]));
            }
        }
        if let Some((x, y)) = self.screen(r.mule.pos.add(v3(0, ratio(26, 10), 0))) {
            markers.push(json!([x, y, "mule", ""]));
        }
        // the detection tickers: one per soldier growing aware of you, round the screen centre
        let mut detect = Vec::new();
        for s in &r.soldiers {
            if !s.free() || s.awareness <= 0 || matches!(s.mind, Mind::Combat) {
                continue;
            }
            let d = s.pos.sub(r.p.pos);
            let rel = fx::wrap(fx::atan2(-d.x, -d.z) - r.cam_yaw);
            let fill = s.awareness / ONE;
            let level = if fill >= SPOTTED_AT {
                "full"
            } else if fill >= INVESTIGATE_AT {
                "high"
            } else {
                "low"
            };
            detect.push(json!([-rel * 360 / fx::TURN, level]));
        }
        let use_now = match what_use(r, m) {
            Use::Nothing => "",
            Use::LeaveLocker => "use_leave",
            Use::Talk => "use_talk",
            Use::Stash(_) => "use_stash",
            Use::Drop => "use_drop",
            Use::OpenCage => "use_cage",
            Use::CarryPrisoner => "use_prisoner",
            Use::TakeCase => "use_case",
            Use::CarryBody(_) => "use_carry",
            Use::Hide(_) => "use_hide",
        };
        let mut prompts = vec![use_now];
        if let Some((_, act)) = r.hold {
            prompts.push(match act {
                HoldAct::Grip => "hold_options",
                HoldAct::Talk(_) => "hold_talking",
                HoldAct::Choke(_) => "hold_choking",
            });
        } else if r.carry.is_none() && r.p.locker.is_none() {
            if let Some(i) = nearest_soldier(r, r.p.pos, GRAB_REACH, |s| s.free()) {
                let s = &r.soldiers[i];
                let d = r.p.pos.sub(s.pos);
                let off = fx::wrap(fx::atan2(-d.x, -d.z) - s.gaze()).abs();
                prompts.push(if off > GRAB_BEHIND {
                    "cqc_grab"
                } else {
                    "cqc_throw"
                });
            }
        }
        if fulton_target(r).is_some() && r.p.locker.is_none() {
            prompts.push("fulton_ready");
        }
        if v3(r.p.pos.x, 0, r.p.pos.z).dist(v3(m.lz.x, 0, m.lz.z)) < LZ_RADIUS && !objective_done(r, m) {
            prompts.push("lz_not_done");
        }
        let intel: Vec<&str> = m
            .intel
            .iter()
            .enumerate()
            .filter(|(k, _)| r.revealed[*k])
            .map(|(_, i)| i.text.as_str())
            .collect();
        let done = objective_done(r, m);
        let (said_key, said_text) = match &r.said {
            Some((t, _)) if t.starts_with('@') => (t[1..].to_string(), String::new()),
            Some((t, _)) => (String::new(), t.clone()),
            None => (String::new(), String::new()),
        };
        h["play"] = json!({
            "mission": m.name,
            "phase": phase,
            "phase_secs": phase_secs,
            "reflex": r.reflex.is_some(),
            "camo": r.camo,
            "stance": stance,
            "uniform": u.name,
            "changing": r.changing > 0,
            "rounds": if r.lethal { "lethal" } else { "tranq" },
            "ammo": r.ammo[if r.lethal { 1 } else { 0 }],
            "cloak": r.cloak,
            "battery": r.battery * 100 / r.battery_max.max(1),
            "fultons": r.fultons,
            "portals": [r.portals[0].is_some(), r.portals[1].is_some()],
            "aim": r.aiming,
            "prompts": prompts,
            "objective": m.objective,
            "objective_note": if done || r.target_known { "" } else { m.objective_unknown.as_str() },
            "objective_done": done,
            "intel": intel,
            "intel_total": m.intel.len(),
            "markers": markers,
            "detect": detect,
            "time": secs_text(r.elapsed / ONE),
            "said_key": said_key,
            "said_text": said_text,
            "mule_loaded": r.mule.load.is_some(),
            "mule_mode": match r.mule.mode { MuleMode::Follow => "follow", MuleMode::Come => "come", MuleMode::Charge(_) => "charge" },
        });
        if let Some(o) = &r.outcome {
            h["outcome"] = json!({
                "done": o.done,
                "cause": o.cause,
                "rank": o.rank,
                "time": secs_text(o.secs),
                "alerts": o.alerts,
                "kills": o.kills,
                "recruits": r.recruited.iter().map(|x| x.name.clone()).collect::<Vec<_>>(),
                "intel": o.intel,
                "intel_total": m.intel.len(),
                "mission": m.name,
            });
        }
        h.to_string()
    }

    /// The menus: missions with their briefs and records, the uniforms, Graceland's staff.
    pub fn menu_json(&self) -> String {
        let missions: Vec<Value> = self
            .missions
            .iter()
            .map(|m| {
                let rec = self.records.get(&m.id);
                json!({
                    "id": m.id,
                    "name": m.name,
                    "brief": m.brief,
                    "objective": m.objective,
                    "rank": rec.map(|r| r.rank.clone()).unwrap_or_default(),
                    "time": rec.map(|r| secs_text(r.secs)).unwrap_or_default(),
                })
            })
            .collect();
        let uniforms: Vec<Value> = self
            .uniforms
            .iter()
            .map(|u| json!({"name": u.name, "base": u.base, "blotch": u.blotch, "grass": u.matches[0], "dirt": u.matches[1], "concrete": u.matches[2], "floor": u.matches[3], "sand": u.matches[4]}))
            .collect();
        let roster: Vec<Value> = self
            .roster
            .iter()
            .rev()
            .map(|r| json!([r.name, self.recruits.units[r.unit], self.recruits.grades[r.grade]]))
            .collect();
        let levels = self.unit_levels();
        json!({
            "missions": missions,
            "chosen": self.chosen,
            "uniforms": uniforms,
            "uniform": self.uniform,
            "roster": roster,
            "staff": self.roster.len(),
            "units": self.recruits.units,
            "levels": levels,
            "perks": {
                "tranq": TRANQ_ROUNDS + 2 * levels[1],
                "battery": (BATTERY + fx::mul(BATTERY, ratio(8 * levels[2], 100))) / ONE,
                "sniff": (MULE_SNIFF + int(4) * levels[0]) / ONE,
            },
        })
        .to_string()
    }
}

fn soldier_fan(phase: Phase) -> i32 {
    match phase {
        Phase::Combat(_) => crate::balance::FAN_NEAR,
        _ => crate::balance::FAN_FAR,
    }
}

/// Layout numbers the page needs, so it holds none of its own.
pub fn numbers() -> String {
    let i = |name: &str, size: usize, offset: usize| json!([name, size, offset * 4]);
    json!({
        "vertex": { "stride": mesh::VERTEX_VALUES * 4, "attributes": [i("aPos", 3, 0), i("aNormal", 3, 3)] },
        "instance": {
            "stride": render::INSTANCE_VALUES * 4,
            "attributes": [i("iA", 4, 0), i("iB", 4, 4), i("iC", 4, 8), i("iD", 4, 12)]
        },
        "buffers": { "scenery": render::BUF_SCENERY, "dynamic": render::BUF_DYNAMIC },
        "passes": { "solid": render::PASS_SOLID, "ink": render::PASS_INK, "tint": render::PASS_TINT },
        "draw_values": render::DRAW_VALUES,
        "vertex_values": mesh::VERTEX_VALUES,
        "mat_values": render::MAT_VALUES,
        "view_values": 16,
        "mesh_count": render::MESH_COUNT,
        "uniforms": [["uLightDir", 3], ["uSky", 3], ["uFog", 3], ["uLight", 3], ["uInk", 3], ["uFogRange", 2], ["uInkSize", 3]],
        "actions": {
            "forward": FORWARD, "back": BACK, "left": LEFT, "right": RIGHT, "sprint": SPRINT,
            "crouch": CROUCH, "prone": PRONE, "jump": JUMP, "use": USE, "cqc": CQC, "fire": FIRE,
            "aim": AIM, "rounds": ROUNDS, "portal_a": PORTAL_A, "portal_b": PORTAL_B, "close": CLOSE,
            "cloak": CLOAK, "box": BOX, "uniform": UNIFORM, "whistle": WHISTLE, "ram": RAM,
            "tie": TIE, "fulton": FULTON, "focus": FOCUS
        },
    })
    .to_string()
}

/// White noise for the page's sound, from the core's seeded generator.
pub fn noise_samples(n: u32, seed: u32) -> Vec<i32> {
    let mut rng = Rng::new(seed as u64);
    (0..n.min(1 << 20)).map(|_| rng.range(-ONE, ONE)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::yaw_pitch_of;

    pub fn game() -> Game {
        Game::new(
            include_str!("../../../data/palette.json"),
            include_str!("../../../data/missions.json"),
            include_str!("../../../data/uniforms.json"),
            include_str!("../../../data/recruits.json"),
            "",
        )
        .unwrap()
    }

    fn start(k: u32) -> Game {
        let mut g = game();
        g.choose(k);
        g.deploy();
        g
    }

    fn run(g: &mut Game, bits: u32, frames: usize, t: &mut i64) {
        for _ in 0..frames {
            *t += TICK_US;
            g.advance(*t, bits | FOCUS, 0, 0);
        }
    }

    /// One frame with a key pressed (and released the next frame).
    fn press(g: &mut Game, bit: u32, t: &mut i64) {
        run(g, bit, 1, t);
        run(g, 0, 1, t);
    }

    fn r(g: &mut Game) -> &mut Run {
        g.run.as_mut().unwrap()
    }

    /// Points the camera so the crosshair sits on a point.
    fn aim_at(g: &mut Game, at: V3) {
        for _ in 0..4 {
            let (o, _) = aim_ray(r(g));
            let (yaw, pitch) = yaw_pitch_of(at.sub(o).norm());
            r(g).cam_yaw = yaw;
            r(g).cam_pitch = pitch;
        }
    }

    /// Puts every soldier but `keep` to sleep far away, so a test sees one at a time.
    fn only(g: &mut Game, keep: &[usize]) {
        for (i, s) in r(g).soldiers.iter_mut().enumerate() {
            if !keep.contains(&i) {
                s.life = Life::Gone;
            }
        }
    }

    #[test]
    fn the_game_opens_on_the_title_and_a_mission_deploys_outside_the_wall() {
        let mut g = game();
        assert_eq!(g.mode(), Mode::Title);
        g.choose(0);
        assert_eq!(g.mode(), Mode::Brief);
        g.deploy();
        assert_eq!(g.mode(), Mode::Play);
        let p = r(&mut g).p.pos;
        assert!(p.x < -int(80), "starts west of the wall: {p:?}");
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["play"]["phase"], "normal");
        assert_eq!(
            h["play"]["objective_note"],
            "Where he is held: unknown. Interrogate soldiers."
        );
    }

    #[test]
    fn standing_in_front_of_a_soldier_ends_in_reflex_then_combat_then_a_bullet() {
        let mut g = start(0);
        only(&mut g, &[2]);
        let s = r(&mut g).soldiers[2].pos;
        let yaw = r(&mut g).soldiers[2].yaw;
        // 10 m in front of him, standing
        let at = s.add(facing(yaw, 0).scale(int(10)));
        r(&mut g).p.pos = v3(at.x, 0, at.z);
        r(&mut g).p.stance = Stance::Stand;
        r(&mut g).soldiers[2].wait = int(100);
        let mut t = 0;
        let mut saw_reflex = false;
        for _ in 0..1200 {
            run(&mut g, 0, 1, &mut t);
            if g.mode() != Mode::Play {
                break;
            }
            saw_reflex |= r(&mut g).reflex.is_some();
        }
        assert!(saw_reflex, "reflex mode came first");
        assert_eq!(g.mode(), Mode::Failed);
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["outcome"]["cause"], "shot");
        assert_eq!(h["outcome"]["done"], false);
    }

    #[test]
    fn a_tranquilliser_in_the_head_drops_him_silently_for_five_minutes() {
        let mut g = start(0);
        only(&mut g, &[2, 3]);
        let s = r(&mut g).soldiers[2].pos;
        r(&mut g).soldiers[2].wait = int(100);
        let yaw = r(&mut g).soldiers[2].yaw;
        let behind = s.sub(facing(yaw, 0).scale(int(12)));
        r(&mut g).p.pos = v3(behind.x, 0, behind.z);
        r(&mut g).aiming = true;
        aim_at(&mut g, s.add(v3(0, ratio(165, 100), 0)));
        let mut t = 0;
        r(&mut g).p.yaw = r(&mut g).cam_yaw;
        let before = r(&mut g).soldiers[3].mind;
        shoot(r(&mut g), &mut Vec::new());
        assert!(
            matches!(r(&mut g).soldiers[2].life, Life::Asleep(_)),
            "{:?}",
            r(&mut g).soldiers[2].life
        );
        assert_eq!(
            r(&mut g).soldiers[3].mind,
            before,
            "no sound reached the other soldier"
        );
        assert_eq!(r(&mut g).ammo[0], TRANQ_ROUNDS - 1);
        run(&mut g, 0, 60, &mut t);
        assert!(r(&mut g).soldiers[2].down());
    }

    #[test]
    fn a_lethal_round_is_loud_and_leaves_a_body_that_never_wakes() {
        let mut g = start(0);
        only(&mut g, &[2, 3]);
        r(&mut g).soldiers[2].wait = int(100);
        let s = r(&mut g).soldiers[2].pos;
        // the other soldier stands 20 m off
        r(&mut g).soldiers[3].pos = s.add(v3(int(20), 0, 0));
        r(&mut g).soldiers[3].route = vec![(r(&mut g).soldiers[3].pos, 0)];
        let yaw = r(&mut g).soldiers[2].yaw;
        let behind = s.sub(facing(yaw, 0).scale(int(12)));
        r(&mut g).p.pos = v3(behind.x, 0, behind.z);
        r(&mut g).lethal = true;
        aim_at(&mut g, s.add(v3(0, ratio(12, 10), 0)));
        shoot(r(&mut g), &mut Vec::new());
        assert_eq!(r(&mut g).soldiers[2].life, Life::Dead);
        assert!(
            matches!(r(&mut g).soldiers[3].mind, Mind::Investigate { .. }),
            "{:?}",
            r(&mut g).soldiers[3].mind
        );
        assert_eq!(r(&mut g).kills, 1);
        let mut t = 0;
        r(&mut g).soldiers[3].life = Life::Gone;
        run(&mut g, 0, 60 * 320, &mut t);
        assert_eq!(r(&mut g).soldiers[2].life, Life::Dead);
    }

    #[test]
    fn grabbed_from_behind_he_tells_where_the_prisoner_is() {
        let mut g = start(0);
        // soldier 8 knows where the commando is
        only(&mut g, &[8]);
        let mut t = 0;
        r(&mut g).soldiers[8].wait = int(100);
        let s = r(&mut g).soldiers[8].pos;
        let yaw = r(&mut g).soldiers[8].yaw;
        let behind = s.sub(facing(yaw, 0).scale(ratio(12, 10)));
        r(&mut g).p.pos = v3(behind.x, 0, behind.z);
        r(&mut g).p.stance = Stance::Crouch;
        // talking may fail (70%): find a seed under which he talks
        let mut seed = 1;
        loop {
            let mut trial = g.run.clone().unwrap();
            trial.rng = Rng::new(seed);
            if trial.rng.chance(TALK_CHANCE) {
                break;
            }
            seed += 1;
        }
        r(&mut g).rng = Rng::new(seed);
        press(&mut g, CQC, &mut t);
        assert!(
            matches!(r(&mut g).hold, Some((8, HoldAct::Grip))),
            "{:?}",
            r(&mut g).hold
        );
        press(&mut g, USE, &mut t);
        run(&mut g, 0, 140, &mut t);
        assert!(r(&mut g).soldiers[8].talked);
        assert!(r(&mut g).revealed[0], "the commando's whereabouts");
        assert!(r(&mut g).target_known);
        // then choke him out
        press(&mut g, CQC, &mut t);
        run(&mut g, 0, 170, &mut t);
        assert!(matches!(r(&mut g).soldiers[8].life, Life::Asleep(_)));
        assert!(r(&mut g).hold.is_none());
    }

    #[test]
    fn from_the_front_cqc_disarms_and_throws_him() {
        let mut g = start(0);
        only(&mut g, &[2]);
        r(&mut g).soldiers[2].wait = int(100);
        let s = r(&mut g).soldiers[2].pos;
        let yaw = r(&mut g).soldiers[2].yaw;
        let front = s.add(facing(yaw, 0).scale(ratio(14, 10)));
        r(&mut g).p.pos = v3(front.x, 0, front.z);
        let mut t = 0;
        press(&mut g, CQC, &mut t);
        assert!(matches!(r(&mut g).soldiers[2].life, Life::Stunned(_)));
    }

    #[test]
    fn the_commando_freed_carried_and_fultoned_then_escape_completes_the_mission() {
        let mut g = start(0);
        only(&mut g, &[]);
        let mut t = 0;
        let door = r(&mut g).world.place_at("cage_door").unwrap();
        r(&mut g).p.pos = v3(door.x - ratio(8, 10), 0, door.z);
        press(&mut g, USE, &mut t);
        assert_eq!(r(&mut g).prisoner, Prisoner::Free);
        assert!(r(&mut g).world.cage_open());
        // into the cage, and pick him up
        r(&mut g).p.pos = v3(door.x + int(1), 0, door.z);
        press(&mut g, USE, &mut t);
        assert_eq!(r(&mut g).carry, Some(Carry::Commando));
        // the warehouse has a roof: no balloon indoors
        press(&mut g, FULTON, &mut t);
        assert_eq!(r(&mut g).prisoner, Prisoner::Carried);
        // outside, it goes
        r(&mut g).p.pos = v3(int(42), 0, int(10));
        press(&mut g, FULTON, &mut t);
        assert_eq!(r(&mut g).prisoner, Prisoner::Gone);
        let m0 = g.missions[0].clone();
        assert!(objective_done(r(&mut g), &m0));
        let lz = g.missions[0].lz;
        r(&mut g).p.pos = v3(lz.x, 0, lz.z + int(2));
        run(&mut g, 0, 2, &mut t);
        assert_eq!(g.mode(), Mode::Complete);
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["outcome"]["rank"], "S");
        assert_eq!(g.records["caged_bird"].rank, "S");
    }

    #[test]
    fn the_lz_does_nothing_before_the_objective() {
        let mut g = start(0);
        only(&mut g, &[]);
        let lz = g.missions[0].lz;
        r(&mut g).p.pos = v3(lz.x, 0, lz.z);
        let mut t = 0;
        run(&mut g, 0, 5, &mut t);
        assert_eq!(g.mode(), Mode::Play);
        assert!(g.hud_json().contains("lz_not_done"));
    }

    #[test]
    fn the_mule_rams_a_soldier_and_tied_to_the_mule_he_talks() {
        let mut g = start(1);
        // soldier 7 knows who Varga is
        only(&mut g, &[7]);
        let s = r(&mut g).soldiers[7].pos;
        r(&mut g).p.pos = s.add(v3(0, 0, int(8)));
        r(&mut g).mule.pos = s.add(v3(int(3), 0, int(9)));
        r(&mut g).mule.mode = MuleMode::Charge(7);
        let mut t = 0;
        run(&mut g, 0, 120, &mut t);
        assert!(
            matches!(r(&mut g).soldiers[7].life, Life::Stunned(_)),
            "{:?}",
            r(&mut g).soldiers[7].life
        );
        r(&mut g).p.pos = s.add(v3(ONE, 0, ONE));
        r(&mut g).mule.pos = s.add(v3(int(2), 0, ONE));
        press(&mut g, TIE, &mut t);
        assert_eq!(r(&mut g).mule.load, Some(7));
        run(&mut g, 0, 150, &mut t);
        assert!(r(&mut g).revealed[0], "he told who Varga is");
        assert!(r(&mut g).soldiers.iter().any(|s| s.target && s.marked));
    }

    #[test]
    fn soldiers_never_see_the_mule() {
        let mut g = start(0);
        only(&mut g, &[2]);
        r(&mut g).soldiers[2].wait = int(100);
        let s = r(&mut g).soldiers[2].pos;
        let yaw = r(&mut g).soldiers[2].yaw;
        r(&mut g).mule.pos = s.add(facing(yaw, 0).scale(int(5)));
        r(&mut g).mule.mode = MuleMode::Come;
        // the player hides in a locker far away
        r(&mut g).p.pos = v3(-int(60), 0, -int(28));
        let at = r(&mut g).p.pos;
        let k = nearest_locker(r(&mut g), at).unwrap();
        r(&mut g).p.locker = Some(k);
        let mut t = 0;
        run(&mut g, 0, 300, &mut t);
        assert_eq!(r(&mut g).soldiers[2].awareness, 0);
        assert_eq!(r(&mut g).phase, Phase::Normal);
    }

    #[test]
    fn walking_into_a_portal_comes_out_of_the_other() {
        let mut g = start(0);
        only(&mut g, &[]);
        r(&mut g).portals = [
            Some(Portal {
                at: v3(int(30), ratio(12, 10), int(80) + ratio(32, 100)),
                n: v3(0, 0, ONE),
            }),
            Some(Portal {
                at: v3(int(10), ratio(12, 10), -int(46) + ratio(22, 100)),
                n: v3(0, 0, ONE),
            }),
        ];
        // south of the wall, walking north into the blue portal
        r(&mut g).p.pos = v3(int(30), 0, int(83));
        r(&mut g).cam_yaw = 0;
        let mut t = 0;
        run(&mut g, FORWARD, 100, &mut t);
        let p = r(&mut g).p.pos;
        assert!(
            p.z > -int(46) && p.z < -int(40) && (p.x - int(10)).abs() < int(2),
            "came out by HQ: {p:?}"
        );
    }

    #[test]
    fn a_soldier_left_asleep_wakes_after_five_minutes_and_radios() {
        let mut g = start(0);
        only(&mut g, &[2]);
        knock_out(r(&mut g), 2, Life::Asleep(SLEEP_TIME));
        r(&mut g).p.pos = v3(-int(95), 0, int(95));
        let mut t = 0;
        run(&mut g, 0, 60 * 299, &mut t);
        assert!(r(&mut g).soldiers[2].down());
        run(&mut g, 0, 60 * 2, &mut t);
        assert!(r(&mut g).soldiers[2].awake());
        assert!(matches!(r(&mut g).phase, Phase::Caution(_)));
    }

    #[test]
    fn extracted_soldiers_join_graceland_and_are_kept_between_visits() {
        let mut g = start(0);
        only(&mut g, &[2]);
        knock_out(r(&mut g), 2, Life::Asleep(SLEEP_TIME));
        let s = r(&mut g).soldiers[2].pos;
        r(&mut g).p.pos = s.add(v3(ONE, 0, 0));
        let mut t = 0;
        press(&mut g, FULTON, &mut t);
        run(&mut g, 0, 60 * 4, &mut t);
        assert_eq!(r(&mut g).recruited.len(), 1);
        // finish the mission so the roster is kept
        r(&mut g).prisoner = Prisoner::Gone;
        let lz = g.missions[0].lz;
        r(&mut g).p.pos = v3(lz.x, 0, lz.z);
        run(&mut g, 0, 2, &mut t);
        assert_eq!(g.roster.len(), 1);
        let saved = g.saved();
        let g2 = Game::new(
            include_str!("../../../data/palette.json"),
            include_str!("../../../data/missions.json"),
            include_str!("../../../data/uniforms.json"),
            include_str!("../../../data/recruits.json"),
            &saved,
        )
        .unwrap();
        assert_eq!(g2.roster, g.roster);
        assert!(g2.menu_json().contains(&g.roster[0].name));
    }

    #[test]
    fn every_mission_runs_a_minute_untouched_without_trouble() {
        for k in 0..3 {
            let mut g = start(k);
            let mut t = 0;
            run(&mut g, 0, 3600, &mut t);
            // nobody walked through a wall: every soldier is out of every solid
            let run = r(&mut g);
            for s in &run.soldiers {
                if s.perch || !s.awake() {
                    continue;
                }
                assert!(
                    !run.world
                        .blocked(s.pos, ratio(2, 10), ratio(3, 10), ratio(15, 10)),
                    "mission {k}: soldier {} in a wall at {:?}",
                    s.id,
                    s.pos
                );
            }
            serde_json::from_str::<Value>(&g.hud_json()).unwrap();
        }
    }
}
