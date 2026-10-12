# Flyover Country, version 2

Playtest notes that drove this version:

1. The core loop was hard to read. There were too many places to defend, it
   was hard to see what happened, and 90 seconds wasn't enough to plan, so
   everyone pressed Suggest Plan. Guarding a node (CAP, combat air patrol)
   didn't feel like it did anything.
2. Far too much text. The dogs were funny, but they talked constantly,
   repeated themselves and joked about rules nobody had learned yet. Every
   screen had a title, a subtitle and an explanation.
3. The look felt generic: dark navy, neon rainbow, glows.

Version 2 keeps what worked (secret planning on the laptop, watching the night
together on the TV, the dogs, the newspaper, Project Bark Park) and rebuilds
the rules underneath so that everything that matters is visible on the map.

## The rules on one card

- Each faction has **4 sites**: HQ, Factory, Airfield and Crown Jewel. Each
  has a few health pips.
- Each faction starts with **4 squadrons** of 3 planes: an Ace fighter
  squadron, a fighter squadron and two bomber squadrons.
- Every night, give each squadron one order by dragging it on the map:
  - **Bombers → enemy site: Strike.** They fly straight there from your
    Airfield. Each bomber that arrives has a good chance to knock off a
    health pip.
  - **Fighters → your land: Patrol.** They fly a circle you can see. An enemy
    raid that flies through the circle on its way to your sites or to a prize
    gets jumped. (Utah's patrols jump every raid that flies through.)
  - **Fighters → enemy site: Escort.** They fly with your bombers to that
    target and fight off patrols on the way. With no bombers, they sweep
    alone.
  - **Anything → a prize: Claim.** First to arrive holds it; later arrivals
    fight for it.
  - **Rest.** Resting squadrons get a plane back at dawn.
- **Bragging Rights:** 2 per pip you knock off (3 for an HQ or Crown Jewel),
  1 per enemy plane you shoot down, and 1 per prize you hold at dawn.
- **Supply** pays for new squadrons (Fighters 4, Bombers 5) and repairs (2 per
  pip). It comes from your HQ, your Factory and your prizes each dawn.
- Targets open in stages, so the first nights are simpler:
  - Nights 1–2: Airfields and prizes.
  - Nights 3–5: Factories as well.
  - Nights 6–8: HQs and Crown Jewels as well.
- After Night 8, the most Bragging Rights wins.
- Any 2 to 7 factions can play; four is the count the numbers are tuned for.
  The homes of factions nobody picked join the Big Empty.

Everything is public except tonight's orders. Last night's patrols and raids
stay on the map as faded marks, so you can guess what everyone will do.

## Why these rules

| Playtest problem | Version 2 answer |
| --- | --- |
| Too many spots to defend | 4 sites per faction, down from about 18. No roads, pipelines, fuel, radar or air-defense nodes. |
| Couldn't tell what defense did | A patrol is a circle on the map, drawn while you plan. Raids that cross it get jumped, and the show stops on every interception. |
| Too hard to see what happened | One raid per target per faction, drawn as a chunky arrow with a plane count. Each fight and strike ends with a big result marker (HIT −1, MISSED, 2 DOWN). |
| 90 seconds was too short | Planning is a handful of drags. The timer defaults to off, with 3- and 5-minute options. |
| Suggest Plan did everything | The plan is short enough to read at a glance. Suggest Plan fills it in so you can edit it. |

## Combat, exactly

All rolls come from the night's seeded generator, so replays are exact.

- **Interception.** When a raid enters an enemy patrol circle, the patrol
  fights it for two rounds:
  - Every fighter plane on both sides fires once a round.
  - Patrol fighters aim at escorts first, then at bombers. Escorts aim at the
    patrol.
  - A shot hits half the time; an Ace hits 65% of the time.
  - Bombers don't shoot.
  - Survivors carry on. A patrol keeps fighting every raid that enters its
    circle for as long as it has planes.
- **At the target.**
  - Each site's flak fires once at the raid (an HQ twice), downing a plane 40%
    of the time.
  - Each surviving bomber then hits 55% of the time. A hit knocks off one
    pip.
- **At a prize.**
  - The first raid to arrive holds it.
  - A later raid fights the holders for two rounds. Whichever side has more
    planes left keeps it, and the holders win ties.
  - At dawn, whoever holds a prize owns it. Owners keep a prize until someone
    takes it.
- **Damage that matters.**
  - A destroyed Airfield stops new squadrons and dawn recovery.
  - A destroyed Factory or HQ stops its Supply.
  - A destroyed Crown Jewel switches off its faction's perk until repaired.

## The seven dogs and their one perk each

A perk works while that faction's Crown Jewel stands.

| Faction | Dog | Perk |
| --- | --- | --- |
| California | Princess Donut | **Move fast:** any new squadron costs 3 Supply. |
| Colorado | Griffin & Pan | **The Mountain:** bigger patrol circles. |
| Cascadia | Ranger Huckleberry | **Overcast:** the first patrol a Cascadian raid meets gets only one round at it. |
| Texas | Coach Tank | **Everything's bigger:** bomber squadrons fly with 4 planes. |
| Nevada | Lucky | **High roller:** prizes pay double Supply. |
| Grain Belt | Agent Sorghum | **Crop dusters:** bombs hit 70% of the time instead of 55%. |
| Utah | Biscuit | **Neighborhood watch:** patrols jump every raid that flies through, not just ones headed for Utah. |

The first drafts (cheaper squadrons, bombs missing Cascadia, Nevada slipping
past patrols, extra Supply, a free first volley) came out of balance testing
between 1% and 75% win rates. The perks above all land between 22% and 28%
over 10,000 four-sided bot games.

## Project Bark Park

This stays, but the rules are simpler:
- **Who can go:** once a game, from Night 3 to 7, if you're in last place and
  10 or more Bragging Rights behind, or down to one squadron.
- **The night away:** your squadrons stay home, and nothing can damage your
  sites. Bombs bounce off the fence.
- **Coming back:**
  - Every squadron refills, and you're topped up to 4 squadrons.
  - You get 6 extra Supply.
  - You get one Big Fetch: a stadium-sized tennis ball that knocks 2 pips off
    any open target and can't be intercepted.

## Text budget

- **Morning:** one card replaces the old dawn briefing and morning report: the
  headline, up to three plain highlights, points gained, one dog line, and a
  news strip only when something new opens (factories, HQs, Bark Park).
- **Planning:** the advisor shows at most two short, plain tips, with no jokes.
  The dog's face shows the mood.
- **The show:** one caption per moment (interceptions, strikes, prizes, the
  park, the Big Fetch). A pilot's radio line rides along at most every 28
  ticks.
- **Lines:** no "Title: subtitle" text, and no line repeats within a game. Dogs
  don't joke about a rule until the game has introduced it.

## Look: Postcard West

The game's world is the American West as roadside America: postcards, travel
posters, a small-town newspaper, a big paper map on the coffee table.

- **Daytime screens** (menu, setup, planning, dawn and report) are printed
  paper: warm cream stock, dark ink and flat poster colors.
- **The show** is the same map after dark, as if printed in blue ink (a
  multiply wash, so land, sea and faction colors stay apart). Planes get a
  paper outline so they read like stickers on top.
- **Faction colors:** seven inks, one per hue family, chosen to read on paper
  and at night:
  - poppy (California)
  - brick (Texas)
  - honey (Utah)
  - wheat green (Grain Belt)
  - pine (Cascadia)
  - mountain blue (Colorado)
  - desert violet (Nevada)
- **Type:** a heavy slab serif for headlines, names and numbers, and Atkinson
  Hyperlegible for everything you read, because it stays clear on a TV across
  the room.
- **One way of drawing things:**
  - 2px ink outlines.
  - Hard offset shadows instead of glows.
  - Small corner radii.
  - Pictogram icons for sites and planes.
