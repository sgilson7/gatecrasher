//! Every number that tunes the feel, in one place (BRIEF: "every number that
//! tunes the feel" stays with the person). Lengths are metres, times seconds
//! and speeds metres per second, all Q16.16; percentages are plain integers.

use crate::fx::{deg, int, ratio};

// --- the clock ---------------------------------------------------------------
/// One simulation tick, in microseconds and in Q16 seconds.
pub const TICK_US: i64 = 1_000_000 / 60;
pub const DT: i32 = ratio(1, 60);
pub const MAX_TICKS_PER_FRAME: u32 = 6;
/// Reflex mode: the world runs at this share of real time.
pub const REFLEX_SLOW: i32 = 5;
/// Reflex mode lasts this long in game time (five seconds of real time).
pub const REFLEX_TIME: i32 = int(1);

// --- the operative ---------------------------------------------------------------
pub const RADIUS: i32 = ratio(35, 100);
pub const HEIGHT_STAND: i32 = ratio(18, 10);
pub const HEIGHT_CROUCH: i32 = ratio(11, 10);
pub const HEIGHT_PRONE: i32 = ratio(5, 10);
/// The walk cycle: one full stride (a step with each foot) covers
/// `STRIDE + STRIDE_GROW × speed` metres, so legs lengthen their stride as
/// they speed up: about 1.5 m walking, 3 m running, 4.6 m sprinting.
pub const STRIDE: i32 = ratio(6, 10);
pub const STRIDE_GROW: i32 = ratio(6, 10);
/// The mule's four legs reach further.
pub const MULE_STRIDE: i32 = int(1);
pub const MULE_STRIDE_GROW: i32 = ratio(5, 10);
pub const SPEED_SPRINT: i32 = ratio(68, 10);
pub const SPEED_RUN: i32 = ratio(40, 10);
pub const SPEED_CROUCH: i32 = ratio(20, 10);
pub const SPEED_PRONE: i32 = ratio(8, 10);
pub const SPEED_CARRY: i32 = ratio(24, 10);
pub const SPEED_HOLD: i32 = ratio(12, 10);
pub const SPEED_BOX: i32 = ratio(22, 10);
/// How fast the body turns toward where it is going, per second.
pub const TURN_RATE: i32 = deg(720);
pub const GRAVITY: i32 = int(20);
/// A ledge this much higher than the feet is walked onto.
pub const STEP: i32 = ratio(45, 100);
/// A ledge up to this high is vaulted (Space).
pub const VAULT_MAX: i32 = ratio(17, 10);
pub const VAULT_TIME: i32 = ratio(45, 100);
/// The dive from a sprint: forward speed, the upward kick that takes her
/// off the ground (about half a metre up, half a second in the air), and the
/// longest a dive lasts before she is prone even if she has not landed.
pub const DIVE_SPEED: i32 = int(7);
pub const DIVE_LIFT: i32 = ratio(45, 10);
pub const DIVE_TIME: i32 = ratio(8, 10);
/// A fall from higher than this kills (MGS5 lets you drop a storey).
pub const FALL_DEADLY: i32 = int(14);
/// Mouse: angle units per CSS pixel, times 100.
pub const LOOK: i32 = 11;
pub const PITCH_MIN: i32 = -deg(70);
pub const PITCH_MAX: i32 = deg(60);

// --- the camera --------------------------------------------------------------------
pub const CAM_BACK: i32 = ratio(36, 10);
pub const CAM_UP: i32 = ratio(9, 10);
pub const CAM_SIDE: i32 = ratio(7, 10);
pub const CAM_BACK_AIM: i32 = ratio(17, 10);
pub const CAM_SIDE_AIM: i32 = ratio(6, 10);
pub const FOV: i32 = deg(62);
pub const FOV_AIM: i32 = deg(42);
pub const FAR: i32 = int(420);

// --- noise: how far each movement is heard (MGS5's footstep rings) -------------------
pub const NOISE_SPRINT: i32 = int(16);
pub const NOISE_RUN: i32 = int(7);
/// A standing walk; crouch-walking makes the sneak ring, too small to reach
/// a soldier's ears from grabbing distance (MGS5: creeping is silent).
pub const NOISE_WALK: i32 = ratio(15, 10);
pub const NOISE_SNEAK: i32 = ratio(6, 10);
pub const NOISE_PRONE: i32 = 0;
pub const NOISE_LETHAL: i32 = int(34);
pub const NOISE_RICOCHET: i32 = int(4);
pub const NOISE_THROW: i32 = int(6);
pub const NOISE_VAULT: i32 = int(5);
pub const NOISE_LAND: i32 = int(9);
/// Behind a wall a sound carries this share (percent) as far.
pub const NOISE_WALLED: i32 = 55;
/// Footstep rings are sent this often while moving.
pub const STEP_EVERY: i32 = ratio(35, 100);

// --- sight -----------------------------------------------------------------------------
pub const SIGHT_NORMAL: i32 = int(34);
pub const SIGHT_CAUTION: i32 = int(44);
pub const SIGHT_COMBAT: i32 = int(64);
/// Inside this, a soldier sees in a wide fan; beyond, in a narrow one.
pub const NEAR_SIGHT: i32 = int(9);
pub const FAN_NEAR: i32 = deg(65);
pub const FAN_FAR: i32 = deg(40);
/// Close enough to sense you running up behind his back (creeping is not felt).
pub const PRESENCE: i32 = ratio(22, 10);
/// Awareness per second, percent, from a man felt behind him.
pub const PRESENCE_RATE: i32 = 120;
/// Awareness per second, percent: base plus the close bonus.
pub const NOTICE_BASE: i32 = 10;
pub const NOTICE_GAIN: i32 = 260;
pub const NOTICE_DECAY: i32 = 14;
/// Thresholds on the 0..100 awareness meter.
pub const SUSPICIOUS_AT: i32 = 30;
pub const INVESTIGATE_AT: i32 = 60;
pub const SPOTTED_AT: i32 = 100;
pub const BODY_SIGHT: i32 = int(26);
pub const PORTAL_SIGHT: i32 = int(32);
pub const BALLOON_SIGHT: i32 = int(45);
/// Soldiers this near a tranquillised one when he drops start searching.
pub const SLEEPER_NOTICE: i32 = int(14);

// --- the camo index (MGS3), in percent --------------------------------------------------
pub const CAMO_MIN: i32 = -60;
pub const CAMO_MAX: i32 = 95;
pub const CAMO_STAND: i32 = 0;
pub const CAMO_CROUCH: i32 = 15;
pub const CAMO_PRONE: i32 = 30;
pub const CAMO_STILL: i32 = 15;
pub const CAMO_WALK: i32 = 0;
pub const CAMO_RUN: i32 = -25;
pub const CAMO_SPRINT: i32 = -45;
pub const CAMO_BUSH_LOW: i32 = 35;
pub const CAMO_BUSH_STAND: i32 = 12;
pub const CAMO_GRASS_PRONE: i32 = 30;
pub const CAMO_GRASS_CROUCH: i32 = 10;
pub const CAMO_INDOORS: i32 = 10;
pub const CAMO_BOX_STILL: i32 = 88;
pub const CAMO_BOX_MOVING: i32 = -10;
pub const CAMO_ACTIVE: i32 = 95;
/// Changing uniform takes this long, standing still.
pub const UNIFORM_TIME: i32 = ratio(15, 10);

// --- active camo (MGS5's stealth camo) ---------------------------------------------------
pub const BATTERY: i32 = int(40);
pub const BATTERY_RECHARGE: i32 = ratio(1, 2);

// --- the alert phases ---------------------------------------------------------------------
pub const COMBAT_LOST: i32 = int(16);
pub const EVASION_TIME: i32 = int(45);
pub const CAUTION_TIME: i32 = int(120);
pub const AIM_TIME: i32 = ratio(9, 10);
pub const AIM_TIME_ELITE: i32 = ratio(6, 10);
pub const FIRE_EVERY: i32 = ratio(11, 10);
pub const GUN_RANGE: i32 = int(60);
/// Chance to hit, percent: at point blank, lost per 10 m, and the floor.
pub const HIT_CLOSE: i32 = 88;
pub const HIT_PER_10M: i32 = 11;
pub const HIT_FLOOR: i32 = 12;
pub const HIT_CROUCH: i32 = 10;
pub const HIT_PRONE: i32 = 25;
pub const HIT_MOVING: i32 = 15;
pub const HIT_SPRINT: i32 = 30;

// --- soldiers --------------------------------------------------------------------------------
pub const WALK: i32 = ratio(15, 10);
pub const JOG: i32 = ratio(30, 10);
pub const RUN: i32 = ratio(48, 10);
pub const SOLDIER_TURN: i32 = deg(240);
pub const LOOK_AROUND: i32 = int(7);
pub const SEARCH_TIME: i32 = int(24);
/// Tranquillised or choked out: five minutes (the brief).
pub const SLEEP_TIME: i32 = int(300);
/// A tranquilliser dart in the body takes this long to drop him.
pub const DART_DELAY: i32 = ratio(25, 10);
pub const STUN_RAM: i32 = int(30);
pub const STUN_THROW: i32 = int(90);
pub const WAKE_BUDDY: i32 = int(3);
pub const UNTIE_TIME: i32 = int(4);

// --- the pistol --------------------------------------------------------------------------------
pub const FIRE_COOLDOWN: i32 = ratio(35, 100);
pub const PISTOL_RANGE: i32 = int(90);
pub const TRANQ_ROUNDS: i32 = 8;
pub const LETHAL_ROUNDS: i32 = 21;
pub const HEAD_RADIUS: i32 = ratio(18, 100);
pub const BODY_RADIUS: i32 = ratio(32, 100);
pub const TRACER_TIME: i32 = ratio(12, 100);
pub const MARK_TIME: i32 = ratio(3, 10);
pub const MARK_RANGE: i32 = int(110);

// --- CQC ---------------------------------------------------------------------------------------
pub const GRAB_REACH: i32 = ratio(17, 10);
pub const THROW_REACH: i32 = ratio(22, 10);
/// Behind him: the grab works when you are outside this angle of his face.
pub const GRAB_BEHIND: i32 = deg(70);
pub const INTERROGATE_TIME: i32 = int(2);
pub const CHOKE_TIME: i32 = ratio(25, 10);
pub const TALK_CHANCE: i32 = 70;
pub const MULE_TALK_CHANCE: i32 = 96;
pub const TALK_MARK_RANGE: i32 = int(40);
/// A hold-up: aim at a soldier this close who has noticed you, or whose
/// back is to you, and he drops his rifle and puts his hands up.
pub const HOLDUP_RANGE: i32 = int(6);
/// He stays held up while the gun is on him from this near...
pub const HOLDUP_KEEP: i32 = int(12);
/// ...and this long after it leaves him, then picks up his rifle and radios.
pub const HOLDUP_LAPSE: i32 = int(4);
/// Aiming at a man on the ground this close, E interrogates him (MGS5's pin).
pub const PIN_REACH: i32 = int(3);
/// A soldier waking a sleeping buddy kicks him this often.
pub const KICK_EVERY: i32 = ratio(9, 10);

// --- bodies, hiding, fulton -----------------------------------------------------------------------
pub const REACH: i32 = ratio(18, 10);
pub const FULTON_TIME: i32 = ratio(35, 10);
pub const FULTONS: i32 = 12;

// --- portals ----------------------------------------------------------------------------------------
pub const PORTAL_RANGE: i32 = int(70);
pub const PORTAL_W: i32 = ratio(15, 10);
pub const PORTAL_H: i32 = ratio(24, 10);
pub const PORTAL_FLOOR: i32 = ratio(18, 10);
pub const PORTAL_COOLDOWN: i32 = ratio(5, 10);
pub const PORTAL_ENTER: i32 = ratio(6, 10);
pub const PORTAL_EXIT: i32 = ratio(7, 10);

// --- the Memphis Mule ---------------------------------------------------------------------------
pub const MULE_WALK: i32 = ratio(32, 10);
pub const MULE_RUN: i32 = ratio(80, 10);
pub const MULE_CHARGE: i32 = ratio(105, 10);
pub const MULE_FOLLOW: i32 = int(4);
pub const MULE_FAR: i32 = int(12);
pub const MULE_SNIFF: i32 = int(24);
pub const MULE_SNIFF_EVERY: i32 = int(1);
pub const MULE_RAM_RANGE: i32 = int(45);
pub const MULE_TIE_REACH: i32 = int(5);
/// Tying a soldier on: the mule must be this near you, or Y calls him over.
pub const MULE_TIE_FAR: i32 = int(10);
/// Riding him: close enough to climb on, the trot, where you sit, the
/// hoofbeats' rings, and the camo index a rider has (a man on a mule is
/// seen from far off: the brief says very noticeable).
pub const MULE_MOUNT_REACH: i32 = ratio(25, 10);
pub const MULE_TROT: i32 = ratio(55, 10);
pub const RIDE_SEAT: i32 = ratio(60, 100);
pub const NOISE_TROT: i32 = int(12);
pub const NOISE_GALLOP: i32 = int(24);
pub const RIDE_CAMO: i32 = -60;

// --- score -------------------------------------------------------------------------------------
/// Points for a finished mission, each piece of intel, each recruit, and each
/// second under par; taken away for each alert and, heavily, each soldier
/// killed (the mission 2 target excepted). Whole points, not Q16.16.
pub const SCORE_DONE: i32 = 10_000;
pub const SCORE_INTEL: i32 = 500;
pub const SCORE_RECRUIT: i32 = 1_000;
pub const SCORE_PAR_SECS: i32 = 900;
pub const SCORE_PER_SEC: i32 = 5;
pub const SCORE_ALERT: i32 = 1_000;
pub const SCORE_KILL: i32 = 2_500;
