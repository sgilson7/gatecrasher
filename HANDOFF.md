# Handoff

12 October 2026. Where Gatecrasher stands.

## What exists

| What | Where |
| --- | --- |
| Repository | https://github.com/sgilson7/gatecrasher (public, MIT) |
| The game | https://sgilson7.github.io/gatecrasher/ (Pages, deployed by `.github/workflows/deploy.yml` after the check passes) |
| Website page | https://sgilson7.github.io/projects/gatecrasher/, from `~/Documents/website` commit `43c9551` (`content/projects/gatecrasher.md`, an entry in `content/projects.yaml`, `/gatecrasher/` in `OFF_BUILD_ROUTES`) |

## Done, against the brief

- Third person, over the right shoulder, with a camera that pulls in front of walls; aiming zooms in.
- One zone, a quarter of an island outpost (`world.rs`), used by all three missions.
- Four missions with different objectives, targets, soldiers, starts and landing zones (`data/missions.json`): rescue Corporal Grace (held in the warehouse cage, not the cell block), eliminate Colonel Varga (one of three red berets), recover the Gate Lens case (on the comms deck, reachable only by portal). Escape to the landing zone ends each. The fourth, Deterrence: climb the unpiloted Deterrence Unit in the yard, set four charges, set them off from 70 m, hold out 75 s for the helicopter.
- Nothing about where or who is known at the start; interrogation (CQC hold, or tied to the mule) reveals it. Soldiers who know nothing mark their neighbours.
- Climbing: vault onto, vault over on the move, leap and hang from ledges up to 2.6 m, shimmy, climb up, let go. The sprint dive leaves the ground and knocks a soldier down for 4 s.
- MGS V detection: drawn sight cones, an awareness meter with suspicious, investigate and spotted stages, hearing by gait and ground, reflex mode, then alert, evasion and caution. One hit kills.
- MGS3 camo index from four uniforms against five grounds, plus stance, movement, bushes, grass and shade; active camo on a battery.
- Bushes, tall grass, lockers, a cardboard box; bodies carried and hidden in bushes and lockers.
- Silenced pistol with tranquilliser rounds (silent; five-minute sleep, then a radio call; nearby soldiers who see him fall go to wake him) and lethal rounds (loud; a body). No respawning.
- CQC: grab, interrogate, choke, kill, throw; a disarming throw from the front. Crouch-walking up behind a soldier is silent and unfelt (after Sam's first play, N16).
- MGS V hold-ups: aim at a soldier close up and he drops his rifle and raises his hands; E interrogates; a pinned soldier on the ground talks too.
- Portals on large walls and floors with momentum through them; with both open, each shows the view out of the other (stencil windows, one level deep) and shots pass through; only elites see them, and they shut what they see.
- The Memphis Mule: follows, whistle, ram, tie and interrogate (E), carries Grace, can be ridden (fast and loud), sniff-marking, never detected. Soldiers kick sleeping buddies awake.
- Fulton recruits to Graceland, kept in local storage, with unit levels that improve the next mission's kit.
- TikZ art in the Sam_Comm_final look: title, failed, complete, Graceland, three mission cards, the mule (`tools/art.py`).

## Not done, or done differently

See `SECOND-ORDER.md` for each: no reload (N6), Graceland is a screen (N4), desktop only (PLAN 10), soldiers do not see through portals (N21), a held-up soldier cannot be ordered to lie down (N18).

## Commands run, and results

| Command | Result |
| --- | --- |
| `./scripts/check.sh` (local) | BUILDER CHECK PASSED: fmt, clippy, 76 core tests, 11 boundary tests, release wasm (about 470 KB), import audit, browser check in Chromium, Firefox and WebKit |
| GitHub Actions `Check, then deploy to Pages` on the first push | check passed in 2 min 12 s (Chromium); deployed |
| `BUILDER_ORIGIN=https://sgilson7.github.io/gatecrasher tests/browser/check.py` | BROWSER CHECK PASSED in Chromium, Firefox and WebKit against the live site |
| website `make check`, `make packet`, `make gate` | 64 pages, no findings; 44 routes clean; gate passed in three engines |
| website `make deploy`, then `make gate-live` | build 73a948cb pushed; `/projects/gatecrasher/` live; gate passed |

## Second round (Sam's first play)

| Command | Result |
| --- | --- |
| `./scripts/check.sh` (local, after sneaking, hold-ups, kicks, mule interrogation, Grace on the mule, riding) | BUILDER CHECK PASSED: 84 core tests, 11 boundary tests, browser check in three engines; pushed as `dacdf6e` |
| `./scripts/check.sh` (local, after portal views and shots through portals) | BUILDER CHECK PASSED: 88 core tests, 11 boundary tests, browser check in three engines |
| GitHub Actions on `39d0f4f`, then `tests/browser/check.py` against the live site | check passed and deployed (1 min 57 s); BROWSER CHECK PASSED in three engines live |
| `./scripts/check.sh` (local, after the mule-tie fix, N23) | BUILDER CHECK PASSED: 92 core tests, 11 boundary tests, browser check in three engines |
| `./scripts/check.sh` (local, after human-paced walk cycles N24, Corporal Grace N25, Big Chum and the thank-you on landing N26) | BUILDER CHECK PASSED: 93 core tests, 11 boundary tests, browser check in three engines |
| website: commit `6d3e9f2` (only `content/projects/gatecrasher.md`), then `make check`, `make packet`, the gate, `publish.sh` and the live gate, all from a clean git worktree | build a9a6d21b live; gate passed live. Built from a worktree because the website folder held another session's uncommitted Long Time Comin' draft, which a deploy from the folder would have published; that draft was left untouched and unpublished |
| Headless Chromium screenshots | portal window shows the far side (a palm tree in front of the blue portal, in the orange window); riding shows camo −60 % |

## Third round

| Command | Result |
| --- | --- |
| `./scripts/check.sh` (local, after varied interrogation lines N28, 8 tranquilliser rounds N29, the score N30, the dive N31) | BUILDER CHECK PASSED: 97 core tests, 11 boundary tests, browser check in three engines |
| GitHub Actions on `15fbcac`, then `tests/browser/check.py` against the live site | check passed and deployed; BROWSER CHECK PASSED in three engines live |
| Headless Chromium screenshot mid-dive | the operative flat in the air, arms ahead, stance still "Standing"; HUD shows "Score 0" and "Tranq 8" |

## Fourth round

| Command | Result |
| --- | --- |
| `./scripts/check.sh` (local, after the Deterrence mission N32-N33, per-soldier last known position N34, shorter reflex and the grace N35, the dive tackle N36, climbing N37) | BUILDER CHECK PASSED: 106 core tests, 11 boundary tests, browser check in three engines |
| 14 planted breaks (`breaks4.py` in the session scratchpad) | each failed its test, each restored |
| Headless Chromium: Deterrence | mission card and list show four missions; the mech stands in the yard with the four charge sites marked; on the deck the reactor, fins, railgun and deck crew; charges set at each site, then set off from outside the west wall: fire and smoke over the wall, Alert, "Hold out: the helicopter lands in 70 s" |

## Next

1. Play Deterrence (T4), and the portal windows on a real GPU (T3).
2. Play it (SECOND-ORDER T1): every feel number is a guess, in `crates/core/src/balance.rs`.
3. The name (T2).
