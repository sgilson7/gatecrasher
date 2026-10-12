# Second-order notebook

`B3 Build in self-questioning`. One row per assumption, surprise or divergence, written when it happened. *Divergence*: assumed, found, changed. *Finding*: something learned. *Worklist*: Sam's to do.

| # | Kind | Row |
|---|---|---|
| N1 | divergence | The request gives no name. The game, repository and site are **Gatecrasher** (`sgilson7/gatecrasher`): portals, and getting in where you are not wanted. The project lives in `~/Documents/FPS`, the folder the session started in. |
| N2 | divergence | Builder's procedure stops after the plan for the person to read it. Sam approved in advance and asked the agent not to stop until the game is deployed, so the plan (`PLAN.md`) was written and followed in one session. |
| N3 | divergence | "Diamond Dog ... instead he's a mule": the mule does D-Dog's jobs that fit a mule. He rams (stuns 30 s), marks soldiers he smells (24 m), comes to a whistle, carries a tied soldier, and is invisible to soldiers. He does not attack with a knife or fetch. |
| N4 | divergence | "Your own version of mother base" is **Graceland** (Memphis, the mule's name). It is a screen with the roster and three unit levels whose perks change the next mission. There is no base to walk around. |
| N5 | divergence | The portals do not show the view through them; each is an opaque swirl. Rendering through a portal needs a second camera pass into a texture and was left out of the MVP. |
| N6 | divergence | The pistol has no magazine or reload: each round type has its own count (tranquilliser 18 plus R&D's bonus, lethal 21). |
| N7 | divergence | "Krav maga ... disarm": CQC from the front takes the rifle and throws the soldier down (stunned 90 s). There are no chained throws or weapon pickups. |
| N8 | divergence | The comms tower first stood in the south yard, where no spot could see its deck. It was moved beside HQ and given a hut whose north wall is in sight from the ridge, so a portal there drops you onto the deck. Test `from_the_ridge_a_portal_opens_on_the_comms_hut_just_over_the_deck`. |
| N9 | finding | The navigation grid joined two open nodes on either side of the cage's bars, so a route could pass through them. Each edge between nodes is now checked for a solid at its middle, and the path is pulled straight only where a ray at knee and chest height is clear. Test `a_path_goes_round_a_wall_through_a_door_not_through_it`. |
| N10 | finding | Walking into a wall portal never fired: the wall's collision stopped the step, so the measured velocity into the wall was zero. Portal entry now uses the direction asked for. Test `walking_into_a_portal_comes_out_of_the_other`. |
| N11 | finding | The boundary scan forbids the identifier `time` in the core (it reads as a clock); the mission clock is `elapsed`. |
| N12 | finding | The test radius `dm(35)` was 3.5 m, not 0.35 m (`dm` is decimetres), and made the drain test fail for the wrong reason. The tests use `balance::RADIUS`. |
| N13 | divergence | The mission 2 art (drawn first) shows a man with a moustache and dark glasses; the brief had called Varga "she". The brief now says Colonel Ivo Varga, he, with dark glasses. |
| N14 | finding | The mule spawned behind the operative, between her and the camera, filling the screen. He starts on her left now. |
| N15 | divergence | The request asks for the tranquillised soldier to "inform the others on the radio". On waking he radios, which raises the caution phase (wider sight) and sends him to search. There is no spoken radio. |
| T1 | worklist | Every feel number is the agent's guess until Sam plays it (`balance.rs`): sight ranges, how fast awareness rises, enemy aim time and accuracy, noise radii, speeds. |
| T2 | worklist | Sam: whether the name Gatecrasher stays. |
