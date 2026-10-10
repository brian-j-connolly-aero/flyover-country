# Flyover Country

A 4-player hotseat air war over a post-collapse American West, led by dogs.

Everyone plans their sorties in secret on a laptop, then the whole couch
watches the night's air war play out on the TV. Seven breakaway states, eight
nights, one evening, and someone walks away with the Bragging Rights. Nobody
dies on screen: planes splash and every pilot gets a good chute. The dogs in
charge are never in any danger.

## The dogs in charge

| Faction | Home | Leader | Plays like |
| --- | --- | --- | --- |
| Pacific Venture Republic | California | **Princess Donut**, Founder & Chief Princess Officer | Swarm cheaply, gamble on updates |
| Front Range Free State | Colorado | **Griffin & Pan**, Co-Commanders (Brothers) | See everything, sell secrets |
| Cascadian Commonwealth | Washington, Oregon | Ranger Huckleberry | Hide in the clouds, strike from lakes |
| Republic of Texas (Again) | Texas | Coach Tank | Huge bombers, longer grudges |
| Dreamland Syndicate | Nevada | Lucky | Bet on everyone else's war |
| Grain Belt Union | North Dakota to Oklahoma | Agent Sorghum | Feed the West, auction dinner |
| Beehive Cooperative | Utah | Biscuit | Help the neighbors, never start it |

Princess Donut, Griffin and Pan are drawn from the real dogs. Griffin and Pan
bicker in every advisor line, but they love each other.

## Playing

Open the game in a browser (desktop Chrome, Firefox, Safari or Edge, or a
tablet). Pick four factions, and make any seat a human or a bot. Then:

1. **Dawn briefing** on the TV: the newspaper, the scores and tonight's stage.
2. **Planning**, in secret: pass the laptop. Hold the button to reveal your
   map. Drag squadron cards onto targets, zones or other squadrons. Routes
   draw themselves in green, yellow or red risk. Queue builds and repairs,
   use your faction's special action, then Commit.
3. **The show** on the TV: the night plays out with radio chatter, flak,
   dogfights, strikes and parachutes. Space pauses; 1, 2 and 4 set the speed.
4. **Morning report**: the scorecards, a headline and the Play of the Night.

**Laptop + TV:** choose "Laptop + TV window" at setup. A Theater window opens.
Drag it onto the TV and double-click it for full screen. It only ever gets
public information. **One screen** works too, on the honor system.

**New players:** use *Night Zero* (the 5-minute tutorial with Princess Donut)
and the *Suggest Plan* button. The dog advisor flags problems as you plan.

Games autosave at every dawn. From *Saved wars and replays* you can resume,
replay any past night, or export and import a save file.

## Running it locally

You need Rust (stable, with the `wasm32-unknown-unknown` target),
`wasm-bindgen-cli` 0.2.129 and Node 22.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
cd web
npm install
npm run wasm      # Rust engine -> web/src/wasm
npm run dev       # http://localhost:5173
npm run build     # static site in web/dist
```

`web/dist` is plain static files: host it anywhere. The repository includes a
GitHub Pages workflow. To turn it on, go to Settings → Pages and set the source
to **GitHub Actions**. After that, every push to `main` deploys the game.

## How it's built

| Piece | Job |
| --- | --- |
| `crates/flyover-core` | Pure deterministic simulation: map, orders, combat, economy, night resolution, event log. Integer math only, seeded ChaCha8 per night, no I/O. |
| `crates/flyover-ai` | Advisor warnings, risk colors, Suggest Plan, Easy/Normal bots, Weak Spot (minimum-cut style analysis), Best Targets, seat and public views. |
| `crates/flyover-wasm` | wasm-bindgen bindings for the browser. |
| `crates/flyover-cli` | `flyover play`, `flyover balance`, `flyover hashes`, `flyover replay <save>`. |
| `content/*.ron` | The map, units, factions, rules, Patch Notes and every line of flavor text. Loaded at startup, so balance changes need no recompile. |
| `web/` | TypeScript + PixiJS frontend: planning window, Theater window, show playback, synthesized audio, saves, offline service worker. |
| `tools/mapgen.py` | Builds `content/map.ron` from real coordinates, nudged apart so nothing overlaps on a TV. |

The whole night resolves in one call, `resolve_night(state, orders) ->
(state, events)`. A night is 360 one-minute ticks of flying, radar, air
defense, dogfights and strikes. The show is a playback of the event log. That
means a save is just the seed plus everyone's orders. Replays are exact, and
the native and WebAssembly builds produce byte-identical states (CI checks
this).

### Tests

```sh
cargo test --release                                   # determinism, replays, invariants, rules
cargo run --release -p flyover-cli -- balance --games 10000
```

## Balance

The harness plays Normal bots against each other across all 35 four-faction
lineups, rotating seat order. Over 10,000 games, every faction lands inside
the 20–30% win band:

| Faction | Win rate | Avg. Bragging Rights |
| --- | --- | --- |
| Pacific Venture Republic | 21.5% | 46.5 |
| Cascadian Commonwealth | 29.0% | 52.0 |
| Front Range Free State | 23.6% | 47.9 |
| Republic of Texas (Again) | 28.3% | 50.4 |
| Dreamland Syndicate | 26.0% | 48.6 |
| Grain Belt Union | 25.3% | 51.6 |
| Beehive Cooperative | 21.3% | 46.4 |

The average winning margin is 12 Bragging Rights, so most games stay close
to the end.

The tuning was done with prices and numbers, without removing any faction's
rule-breaker. The main moves were:

- Guards defend their post, and Sweep hunts traffic in transit.
- Crown jewels are hardened. A single squadron rarely cracks one, so Gloves
  Off raids are team efforts.
- Utah's first shot applies when defending a home (its own or a neighbor's),
  not when grabbing neutral land.
- The Longhorn and the Seagulls were toned down; California's drones were
  toned up.

## Design decisions beyond the requirements doc

- **Dogs lead every faction**, and each dog is that faction's advisor.
- **Utah is a seventh faction**, the Beehive Cooperative, so any four of seven
  play. The Big Empty is now Idaho, Montana, Wyoming, Arizona and New Mexico.
- **The Mass Driver moved to White Sands, New Mexico** (Utah's salt flats are
  Utah's now). It fires **once per game**, from Night 6. Firing every night
  was worth about 10 Bragging Rights to whoever held it, which swung games too
  hard.
- **Arizona and New Mexico** stay in the Big Empty, with the Boneyard, the
  Mass Driver and the Route 66 truck stops.
- **Laptop + TV** is the default recommendation; single-screen is supported.
- **Bets and bids:** the Harvest winner and winning bid are announced on the
  title card, and winning bets flash during the show. Losing bids and bets
  stay secret.
- **Suggest Plan is always available.** It's a learning tool, and the bots use
  the same planner.
- **8 nights** as written. The stages are Saber Rattling (1–2), Open Season
  (3–5) and Gloves Off (6–8).
- **Colorado's radar sale** needs both sides. Colorado lists a price, buyers
  name a region and a top price, and the deal delivers at dawn.
- **Tone:** every line stays affectionate and about places and food. The
  Beehive's humor is fry sauce, green Jell-O, "oh my heck", casseroles and the
  Greatest Snow on Earth. There is nothing about religion.
