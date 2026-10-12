# Flyover Country

An air war over a post-collapse American West, for 1 to 7 players on one
laptop (plus a TV, if you have one), led by dogs.

Everyone plans in secret, then the whole couch watches the night play out on
the TV. A game is eight nights and fits in an evening. Nobody dies on screen:
planes splash and every pilot gets a little parachute. The dogs in charge are
never in any danger.

## Start playing

All you need is [Node.js](https://nodejs.org) 20.19 or newer. The LTS
(long-term support) download is the right one. The game engine comes
prebuilt, so you don't need Rust.

**Windows:** get the code (Code → Download ZIP and unzip it, or `git clone`),
then double-click **`play.cmd`**. The first run installs the game's tools,
which takes a minute or two and needs the internet. Your browser then opens
the game at <http://localhost:5173>. Keep the black window open while you
play, and close it when you're done.

- If Windows says it "protected your PC", choose **More info → Run anyway**.
  `play.cmd` is a short text file, so you can open it in Notepad first.
- You can also type the two commands yourself in a Command Prompt in the
  game's folder: `npm install`, then `npm start`. If PowerShell says
  running scripts is disabled, type `npm.cmd` instead of `npm`.

**WSL (Windows Subsystem for Linux), macOS or Linux:** from the game's folder,

```sh
npm install
npm start
```

(`./play.sh` does the same two steps.) Tips for WSL:

- Install Node inside WSL with [nvm](https://github.com/nvm-sh/nvm):
  `nvm install --lts`. The Node from Ubuntu's `apt` is often too old.
- Keep the game in your Linux home folder (`~/`), not under `/mnt/c`.
  Installing is much faster there.
- The game opens in your Windows browser. If no window appears, go to
  <http://localhost:5173> yourself.

Saved games live in your browser at that address, so stick with the same
browser. To play on a tablet on the same Wi-Fi, run `npm run lan` and open
the "Network" address it prints. Windows may ask you to let Node.js through
the firewall.

## The dogs in charge

| Side | Leader | Perk |
| --- | --- | --- |
| California | **Princess Donut**, Founder & Chief Princess Officer | *Move fast:* any new squadron costs 3 Supply |
| Cascadia | Ranger Huckleberry | *Overcast:* the first patrol a raid meets gets only one round at it |
| Colorado | **Griffin & Pan**, Co-Commanders (Brothers) | *The Mountain:* bigger patrol circles |
| Texas | Coach Tank | *Everything's bigger:* 4-plane bomber squadrons |
| Nevada | Lucky | *High roller:* prizes pay double Supply |
| Grain Belt | Agent Sorghum | *Crop dusters:* bombs hit more often |
| Utah | Biscuit | *Neighborhood watch:* patrols jump every raid that flies through |

A perk works while that side's Crown Jewel stands. Princess Donut, Griffin and
Pan are drawn from the real dogs.

## Playing

Pick 2 to 7 sides and make each one a person or a bot. Then, every night:

1. **Plan, in secret.** Pass the laptop. Drag a **Bombers** card onto an
   enemy site to bomb it. Drag a **Fighters** card onto your own land to
   patrol: the circle is the sky they guard, and they jump raids that fly
   through it on the way to your sites or to a prize. Drop fighters on a
   target instead to fly along and protect the raid. Spend Supply on new
   squadrons and repairs, then Commit. *Suggest Plan* fills in a sensible
   plan you can change.
2. **Watch.** Every plan plays out at once on the TV. Raids fly in straight
   lines. Whenever something happens, the night pauses, the camera moves in
   and the result is stamped on the map: *2 DOWN*, *HIT −1*, *MISSED*,
   *TAKEN*.
3. **Morning.** One card: the headline, what happened, who scored, one dog
   talking, and anything new for tonight.

**Sites and points.** Each side has an HQ, a Factory, an Airfield and a
Crown Jewel, each with a few pips of health. Knock off a pip: 2 points (3
for an HQ or Crown Jewel). Shoot down a plane: 1. Airfields and the three
prizes (Old Faithful, the Boneyard and White Sands) are open from Night 1,
Factories from Night 3, HQs and Crown Jewels from Night 6. The first raid to
reach a prize holds it, worth 1 point and 1 Supply each morning.

**Everything is public except tonight's orders.** The map shows where
everyone flew and patrolled last night as faded marks, so you can read
habits.

**Nobody gets knocked out: Project Bark Park.** Last place and 10 or more
points off the lead, or down to one squadron? Once a game, between Nights 3
and 7, you can spend a night at a secret dog park. Nothing can hit you that
night: bombs bounce off the fence. You come back repaired and topped up with
the **Big Fetch**, a stadium-sized tennis ball that knocks 2 pips off any
open target and can't be stopped.

**Laptop + TV:** choose it under Screens at setup. A TV window opens: drag it
onto the TV and double-click for full screen. The laptop becomes a remote
while the TV plays. **One screen** works too.

**New players:** *Learn by playing* is a normal game against three easy bots,
with four short tips from Princess Donut on the first night. *Rules* is one
card.

**Music:** four chiptune songs play by screen. The menu has Sound and Music
switches.

Games save themselves every morning. From *Saved games* you can resume,
watch any past night again, or export and import a save file.

## Working on the code

The Rust engine is compiled to WebAssembly and checked in under
`web/src/wasm`, so playing needs only Node. If you change the Rust code,
rebuild the engine and commit the result. CI (continuous integration)
checks that the checked-in engine plays exactly like the Rust source.

You need Rust (stable, with the `wasm32-unknown-unknown` target) and
`wasm-bindgen-cli` 0.2.129:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm install
npm run wasm      # Rust engine -> web/src/wasm
npm run dev       # http://localhost:5173, without opening a browser
npm run build     # static site in web/dist
```

`web/dist` is plain static files: host it anywhere. The repository includes a
GitHub Pages workflow. To turn it on, go to Settings → Pages and set the source
to **GitHub Actions**. After that, every push to `main` deploys the game.

## How it's built

| Piece | Job |
| --- | --- |
| `crates/flyover-core` | Pure deterministic simulation: map, orders, patrols, raids, fights, flak, bombs, prizes, the economy and the event log. Integer math only, seeded ChaCha8 per night, no I/O. |
| `crates/flyover-ai` | Suggest Plan, the Easy and Normal bots, the planning preview (raid paths and which patrols they cross) and the advisor's tips. |
| `crates/flyover-wasm` | wasm-bindgen bindings for the browser. |
| `crates/flyover-cli` | `flyover play`, `flyover balance`, `flyover hashes`, `flyover replay <save>`. |
| `content/*.ron` | The map, factions, rules and every line the dogs say. Loaded at startup, so balance changes need no recompile. |
| `web/` | TypeScript + PixiJS frontend: planning screen, TV window, the night show, synthesized audio, saves, offline service worker. |
| `web/src/music/` | The soundtrack: four songs written in MML (Music Macro Language, the classic chiptune notation) in `songs.ts`, a parser, and a WebAudio synth with NES-style voices. `node web/scripts/check-music.ts` checks every song. |
| `tools/mapgen.py` | Builds `content/map.ron` from real coordinates, nudged apart so nothing overlaps on a TV. |

The whole night resolves in one call, `resolve_night(state, orders) ->
(state, events)`. Raids fly straight from their Airfield to their target;
patrols are circles that jump raids passing through. The show is a playback
of the event log. That
means a save is just the seed plus everyone's orders. Replays are exact, and
the native and WebAssembly builds produce byte-identical states (CI checks
this).

### Tests

```sh
cargo test --release                                   # determinism, rules, 2 to 7 sides
cargo run --release -p flyover-cli -- balance --games 10000
cargo run --release -p flyover-cli -- balance --players 6 # any count from 2 to 7
```

## Balance

The harness plays Normal bots against each other across every lineup,
rotating seat order. Four sides is the tuned count: over 10,000 games, every
faction lands inside the 20–30% win band.

| Faction | Win rate | Avg. points |
| --- | --- | --- |
| California | 25.1% | 52.6 |
| Cascadia | 28.1% | 54.0 |
| Colorado | 22.3% | 52.0 |
| Texas | 24.1% | 53.6 |
| Nevada | 24.8% | 53.1 |
| Grain Belt | 23.2% | 52.5 |
| Utah | 27.3% | 52.9 |

Project Bark Park gets about 2.4 visits a game. Visitors win 21% of the time
and climb out of last place 70% of the time. The average winning margin is
11.5 points, so most games stay close to the end.

Other sizes play fine but aren't tuned as tightly. Utah's neighborhood watch
gets stronger the more raids are in the air, and Cascadia's clouds matter
most one on one:

| Sides | Fair share | Lowest | Highest |
| --- | --- | --- | --- |
| 2 | 50% | Utah 37% | Cascadia 60% |
| 3 | 33% | Utah 29% | Cascadia 39% |
| 5 | 20% | Grain Belt 15% | Utah 24% |
| 6 | 17% | Grain Belt 12% | Utah 22% |
| 7 | 14% | Colorado 9% | Utah 21% |

## Version 2

Version 2 came out of a playtest: the game was fun, but hidden. What changed:

- **One clear loop.** Four sites per side instead of a web of nodes, three
  prizes, and two kinds of squadron. Patrols are circles you can see, and a
  raid's path shows which circles it crosses.
- **A show you can follow.** The night stops on every moment that matters
  and stamps the result on the map, with one caption.
- **Far less text.** One morning card instead of a briefing and a report;
  one dog line a morning, never repeated within a game; no jokes about
  rules you haven't met yet; a one-card rulebook.
- **Postcard West.** Paper and ink by day, the same map in blue ink at
  night, one slab typeface and one plain one, and each side's own ink color.
- **No planning timer by default** (3 or 5 minutes if you want one), and
  last night's orders carry over.
- **2 to 7 sides.**

The full design is in [`docs/design-v2.md`](docs/design-v2.md).

