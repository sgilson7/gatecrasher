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
- Three missions with different objectives, targets, soldiers, starts and landing zones (`data/missions.json`): rescue Corporal Vance (held in the warehouse cage, not the cell block), eliminate Colonel Varga (one of three red berets), recover the Gate Lens case (on the comms deck, reachable only by portal). Escape to the landing zone ends each.
- Nothing about where or who is known at the start; interrogation (CQC hold, or tied to the mule) reveals it. Soldiers who know nothing mark their neighbours.
- MGS V detection: drawn sight cones, an awareness meter with suspicious, investigate and spotted stages, hearing by gait and ground, reflex mode, then alert, evasion and caution. One hit kills.
- MGS3 camo index from four uniforms against five grounds, plus stance, movement, bushes, grass and shade; active camo on a battery.
- Bushes, tall grass, lockers, a cardboard box; bodies carried and hidden in bushes and lockers.
- Silenced pistol with tranquilliser rounds (silent; five-minute sleep, then a radio call; nearby soldiers who see him fall go to wake him) and lethal rounds (loud; a body). No respawning.
- CQC: grab, interrogate, choke, kill, throw; a disarming throw from the front.
- Portals on large walls and floors with momentum through them; only elites see them, and they shut what they see.
- The Memphis Mule: follows, whistle, ram, tie and interrogate, sniff-marking, never detected.
- Fulton recruits to Graceland, kept in local storage, with unit levels that improve the next mission's kit.
- TikZ art in the Sam_Comm_final look: title, failed, complete, Graceland, three mission cards, the mule (`tools/art.py`).

## Not done, or done differently

See `SECOND-ORDER.md` for each: no view through the portals (N5), no reload (N6), Graceland is a screen (N4), desktop only (PLAN 10).

## Commands run, and results

| Command | Result |
| --- | --- |
| `./scripts/check.sh` (local) | BUILDER CHECK PASSED: fmt, clippy, 76 core tests, 11 boundary tests, release wasm (about 470 KB), import audit, browser check in Chromium, Firefox and WebKit |
| GitHub Actions `Check, then deploy to Pages` on the first push | check passed in 2 min 12 s (Chromium); deployed |
| `BUILDER_ORIGIN=https://sgilson7.github.io/gatecrasher tests/browser/check.py` | BROWSER CHECK PASSED in Chromium, Firefox and WebKit against the live site |
| website `make check`, `make packet`, `make gate` | 64 pages, no findings; 44 routes clean; gate passed in three engines |
| website `make deploy`, then `make gate-live` | build 73a948cb pushed; `/projects/gatecrasher/` live; gate passed |

## Next

1. Play it (SECOND-ORDER T1): every feel number is a guess, in `crates/core/src/balance.rs`.
2. The name (T2).
