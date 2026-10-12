# Plan

Stage 2, written by the agent. `B2 Require step-by-step justification`.

Sam approved the plan in advance and asked that the work not stop until the game is deployed (BRIEF.md, Autonomy), so this plan was written and followed in one session without a stop for review (SECOND-ORDER N2).

## Where it comes from

| Source | Taken |
| --- | --- |
| Builder (`builder-setting`) | the three layers, the boundary tests, `check.sh`, the browser check |
| Pursuer (`LargeFlatEscape`) | the engine: `fx.rs`, `geom.rs`, `mesh.rs`, the instanced renderer and inverted-hull ink, the shim's shape, the build and Pages workflow |
| Pursuer's `tools/art.py`, `Sam_Comm_final.png` | the TikZ art pipeline and palette |
| MGS V (the request) | stances, footstep noise, the awareness meter and its stages, reflex mode, alert/evasion/caution, CQC, interrogation, D-Dog (the mule), Fulton and Mother Base (Graceland) |
| MGS3 (the request) | the camo index: uniform against ground |
| Splitgate (the request) | two portals carrying position, heading and speed |

## The zone

One quarter of an island outpost, 200 m square, walled at ±80 m (`world.rs`). Inside: HQ with an office and lockers, barracks with lockers, a warehouse with a back room and a cage, a cell block, a comms tower with a deck and no stairs, two guard towers, shipping containers, fuel tanks, trucks, crates, sandbags, bushes and tall grass. Outside: grass and bushes to the south and west, a beach to the east, a rock ridge to the north that looks down into the yard. A storm drain under the west wall can be crawled through. Ground types: grass, dirt, concrete, indoor floor, sand.

## Decisions

1. **All rules in the core, fixed point.** The reference default (no floats) stays; Pursuer's `fx` module carries trigonometry and square roots.
2. **Everything solid is an axis-aligned box.** Collision (cylinder against box), sight and bullets (ray against box), and portal faces all use the same list.
3. **Soldiers walk a 2 m navigation grid** with A*, pulled tight where a straight walk is clear. Doors are centred on even metres and at least 2.4 m wide so a node or an edge passes through each; thin bars close the edge between nodes.
4. **Detection follows MGS V.** Awareness rises faster the closer and plainer you are; sight range is the phase's range times (100 − camo index)%. Thresholds at 30 (suspicious), 60 (investigate), 100 (spotted → reflex mode → alert).
5. **One hit kills.** A soldier in combat aims for 0.9 s (elites 0.6 s), then fires; the chance to hit falls with distance, crouching, crawling and moving.
6. **Interrogation reveals mission intel.** Each mission lists its intel and which soldiers know it. A soldier who knows nothing marks the soldiers near him instead. Tied to the mule, a soldier talks 96% of the time and tells something extra if he knows nothing new.
7. **Missions are data** (`data/missions.json`): kind, start, landing zone, objective place, intel, and each soldier's route or perch, flags and knowledge.
8. **A portal needs a face big enough** (1.5 × 2.4 m on a wall, 1.8 m square on a floor) and room in front to stand. The comms deck is reachable only through a portal on its hut, which is in sight from the ridge.
9. **Graceland is kept in local storage** with mission records and the chosen uniform; the game works without storage.
10. **Desktop only.** Keyboard and mouse; no touch controls in this build.
