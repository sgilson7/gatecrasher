# Gatecrasher

A third-person stealth game that runs in the browser. You are Big Chum, a legendary soldier every soldier on the island dreams of serving with. You infiltrate one quarter of a walled island outpost, the same zone in every mission, with a silenced pistol, a portal gun and a mule. Soldiers see and hear you the way they do in Metal Gear Solid V, and one bullet kills you. Each mission tells you only what to do; where and who you find out by interrogating the soldiers. Every mission ends when you escape to the landing zone.

**Play:** https://sgilson7.github.io/gatecrasher/

## The missions

- **Caged Bird.** Rescue Corporal Grace. Nobody says where he is held until you make someone talk.
- **Red Beret.** Eliminate Colonel Varga, one of three officers in red berets. Find out which one he is.
- **Gate Lens.** Recover a black case from somewhere on the site. More of the elite soldiers are out, and their lenses can see your portals.

Each mission has its own soldiers, routes, start, landing zone and intel. Soldiers do not come back within a mission.

## What you can do

- **Move as in MGS V.** Stand, crouch (C), crawl (Z), sprint (Shift), vault onto a ledge (Space), dive out of a sprint (Space: a leap through the air that lands you prone). Footsteps are heard as far as the gait carries them: sprinting about 16 m, crouch-walking about 1.5 m, crawling not at all. Floors indoors carry further, grass and sand less.
- **Hide.** In bushes and tall grass, in lockers (E), and in a cardboard box (B) that soldiers ignore while it keeps still.
- **Camouflage.** A camo index in percent, as in MGS3: the uniform against the ground underfoot (four uniforms, each strong on some grounds and weak on others; change in the field with U), plus stance, movement, cover and shade. The index scales how far a soldier can see you. Active camo (V) runs on a battery.
- **Portals.** Q and R open a blue and an orange portal on a large wall, roof or floor in sight (crates, lockers and railings are too small). Walk into one and you come out of the other, turned and with your speed carried through. With both open, each one shows the view out of the other, and a shot into one comes out of the other. Ordinary soldiers cannot see them. Elite soldiers wear lenses that can, and walk over to shut any portal they spot.
- **The pistol.** You start with 8 tranquilliser rounds (more with R&D staff), so spend them carefully. Tranquilliser rounds are silent; a soldier hit in the body stays on his feet for a moment, a hit in the head drops him at once. He sleeps five minutes, then wakes and radios it in. Soldiers near him who see him fall go and wake him. Lethal rounds (T) are loud and leave a body that does not get up.
- **CQC.** Crouch-walking up behind a soldier is silent. F behind him grabs him; then E interrogates (each soldier answers in his own words, from `data/talk.json`), F chokes him out, X kills, C throws him down. F from the front takes his rifle and throws him; aim at him on the ground and E interrogates him.
- **Hold-ups.** Aim at a soldier close up who has noticed you, or whose back is to you, and he drops his rifle and puts his hands up. E interrogates him and F grabs him. Take the gun off him for long and he picks the rifle up and radios in.
- **Bodies.** Carry them (E) and leave them in a bush or a locker. A soldier who finds a body raises the caution phase; one who finds a sleeper wakes him.
- **The Memphis Mule.** He follows you and soldiers never notice him. H whistles him over. M sends him to ram the soldier you point at. Y ties a soldier to his back (one lying near you, on your shoulder or in your hold), or puts Corporal Grace on; if the mule is far, Y calls him over; E interrogates a soldier tied there, who almost always talks and tells more. J rides him: fast, and seen and heard from far off. Soldiers waking a tranquillised buddy kick him awake. The mule also marks soldiers he smells nearby.
- **Fulton.** G sends a downed soldier up on a balloon, outdoors, to your base, Graceland. The soldiers on this outpost hate serving it; each one you extract radios his thanks on landing and signs on with Big Chum. Recruits are kept between visits, and each unit's level improves your kit: R&D adds tranquilliser rounds, Support adds active camo time, Intel lengthens the mule's nose.
- **Score.** Intel, recruits, finishing and finishing fast earn points; each alert costs 1,000 and each soldier killed costs 2,500 (Colonel Varga, the mission 2 target, excepted). The score shows on the HUD and the results screen.
- **Detection.** A soldier's sight is drawn on the ground. White ticks round the screen centre show who is noticing you. At full awareness the game drops into reflex mode, five seconds of slow motion to put him down; miss it and the base goes to alert, then evasion, then caution.

Press I for the intel you have, Esc to pause, N for sound.

## How it is built

Gatecrasher is a [Builder](https://github.com/sgilson7/builder-setting) project, built from Pursuer's engine (Q16.16 fixed-point maths with no floats in the core, instanced WebGL 2 draws, an inverted-hull ink outline, banded light) and the TikZ art pipeline after `Sam_Comm_final.png` used across Sam's games.

- **`crates/core`** holds every rule: the zone (`world.rs`), the soldiers' navigation grid and A* (`nav.rs`), sight, hearing and behaviour (`soldier.rs`), the operative and the camo index (`player.rs`), portals (`portal.rs`), the missions (`mission.rs`), and everything else in `game.rs`. No input or output, no floats, no clock. Every feel number is in `balance.rs`.
- **`crates/shim`** moves values across the WebAssembly boundary and decides nothing.
- **`web/`** is a static page with a WebGL 2 canvas. It draws what the core returns.
- **`data/`** holds the missions, uniforms, recruit names, colours, words, sound recipes and shaders.
- **`tools/art.py`** draws every menu picture as TikZ; the sources are in `tools/art/`.

```bash
./scripts/setup.sh   # once: Playwright and three browser engines
./scripts/check.sh   # fmt, clippy, the core tests, the wasm build and audit, the browser check
python3 scripts/serve.py 8000
```

A push to `main` runs the same check in Chromium and deploys `dist/` to GitHub Pages if it passes.

`BRIEF.md` holds the request word for word, `PLAN.md` the decisions, `SECOND-ORDER.md` what was found while building, and `HANDOFF.md` where the work stands.
