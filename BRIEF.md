# Brief

Stage 1, written by the person. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The request, word for word (2026-10-11)

> Please use the Builder repo as a base in the directory above, and use the work and repos of Sam gilson at sgilson7.github.io, which can all be found one directory up. please build out the MVP and deploy it to my website. you have approval for creating a public github repo, github pages, and approval to deploy to my website when its ready. any time you need to make 2D art and any kind of asset for the game, use the tikz latex prompt you can find in other repos, and use sam comm final as an inspiration image, alongside the menu screens of all of the sgilson7 games which use a similar build pipeline for images. now build the mvp for the game and deploy it, dont stop till its deployed, good luck. here is the prompt from my friend to follow to create the game containing all design decisions: the game should be 3d.
>
> the game is a 3rd person game single player game that is based on split gate mixed with metal gear solid, where you use portals to sneak around the environment and try to assassinate your target. however if you get shot even once you die like in real life. the game that should serve as the strongest inspiration for game play is metal gear solid 5 ground zeros, where you are always in the same environment but you have a bunch of different missions to complete in that environment. the enemies should have drawn out sight lines, and are able to hear your movement. use the same sound detection system and visual detection system as mgs5 the phantom pain, with the ability to extract soldiers for your own version of mother base, and the primary method of movement should be sneaking around, hiding in bushes, hiding in boxes, hiding in closets, assassinating soldiers, hiding their bodies, interrogating soldiers for unknown mission details (which you know none of at first except the basic high level thing you should do, like save this captured commando, but you have no idea where the commando is. you should have a companion dog like diamond dog instead hes a mule named the memphis mule that rams into people and then you can tie them to the mule to interrogate them further, in which they almost always give up their data if they get tied to the mule; you shoyld have a whsitel for him like d dog, and he cannot be detected like you can. there should be an active camo system like mgs5, with a little bit of mgs3 involved, where your actual camoflauge on your gear and how well it blends in with your current environment effects how well enemies can see you. for the mvp, build 1/4 of the game world zone that will be re-used, and add 3 different sneaking missions with completely different and unique objectives and targets. you must escape at the end of every mission. really use MGS5 as an inspiration for movement mechanics and how the game plays overall. your weapon choices should only be a silenced pistol to start, but you can switch between tranq rounds and deadly rounds, where tranq rounds make no noise but people around the slept target will visibly start to search for who did it and leaves the ability to wake them up and interrogate them, but the tranqd target will wake up after like 5 minutes and inform the others on the radio what happened,  where shooting them with the lethal rounds will just kill them like normal with normal consequences like a loud bang and a body, but they never wake up again and there is no enemy respawning in this game, except at the start of every level, and between different missions the enemy layouts can be different. there should also be some form of cqc combat and being able to do krav maga and stuff to disarm targets and put them in a headlock to interrogate them, sleep them, or kill them like grabbing people in mgs5. enemies are unable to see your portals unless they are wearing special portal seeing glasses, which only elite enemies have on. ok good luck

## The tool

**Gatecrasher** (name chosen by the agent; SECOND-ORDER N1): a single-player, third-person stealth game in a desktop browser (keyboard and mouse). A portal gun, a silenced pistol and a mule, in one quarter of an outpost, across three missions.

## Done means

- The page loads from `https://sgilson7.github.io/gatecrasher/` with no off-origin request.
- A third-person operative walks, sprints, crouches, crawls, vaults and dives through one walled outpost: the zone every mission reuses.
- Two portals, placed on walls and floors with a line of sight, carry the player (and the player's momentum) between them. Ordinary soldiers cannot see portals; elite soldiers wear lenses that can.
- Soldiers patrol with their sight lines drawn on the ground. They see by distance, angle, stance, motion, light and camouflage, and hear footsteps and gunshots, through MGS5's stages: noticing, suspicion, investigation, reflex, combat, evasion, caution.
- One hit kills the player.
- The pistol fires tranquilliser rounds (silent; the sleeper wakes after five minutes and radios in) or lethal rounds (loud; a body).
- CQC: grab from behind, then interrogate, choke out or kill; a disarming throw from the front.
- Bodies can be carried and hidden in bushes and lockers; the player can hide in bushes, lockers and a cardboard box.
- Camouflage: the uniform against the ground underfoot, stance, movement and cover make a camo index, as in MGS3; an active camo device, as in MGS5.
- The Memphis Mule follows, comes to a whistle, rams a soldier down, carries a tied soldier who then tells everything, marks soldiers near him, and is never seen.
- Fulton extraction recruits soldiers to the player's own base, kept between visits.
- Three missions in the same zone with different layouts and objectives: rescue a commando, eliminate an officer, recover a prototype. The details are found by interrogation. Each ends by escaping to a landing zone.
- No respawning within a mission.
- 2D art is TikZ in the Sam_Comm_final look.
- The game is listed on Sam's website.

## Decisions that stay with the person

- The name.
- Every number that tunes the feel.

## Autonomy

Sam approved in advance (2026-10-11): the plan, a public GitHub repository, GitHub Pages deploys and the website deploy, and asked the agent not to stop until the game is deployed. Where the brief is silent, the agent decides, records the decision in `SECOND-ORDER.md`, and continues. The agent may not loosen a Builder requirement or a boundary test to make work pass. It may not add a server, an account, analytics or any off-origin request.

## Allowed core dependencies

`serde`, `serde_json` (reference default). Nothing else.
