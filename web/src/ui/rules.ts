// Rules on one screen: each faction's five special rules on a single card,
// plus the whole game in a few plain sentences.

import type { FactionId } from '../types';
import { FACTIONS } from '../types';
import { faction, leader, turns } from '../text';
import { leaderSvg, emblemSvg } from '../art/dogs';
import { h, svg, modal } from './dom';

export function factionCard(f: FactionId, opts: { compact?: boolean } = {}): HTMLElement {
  const fd = faction(f);
  const L = leader(f);
  const rows: [string, [string, string]][] = [
    ['Signature', fd.card.signature],
    ['Quirk', fd.card.quirk],
    ['Unique unit', fd.card.unique],
    ['Crown jewel', fd.card.jewel],
    ['Ace', fd.card.ace],
  ];
  return h(
    `div.fcard${opts.compact ? '.compact' : ''}`,
    { style: { '--fc': fd.color } as unknown as CSSStyleDeclaration },
    h(
      'div.fcard-head',
      svg(leaderSvg(f, 'neutral', { size: opts.compact ? 84 : 120, bg: true }), 'fcard-dog'),
      h('div', h('div.fcard-name', svg(emblemSvg(f, { size: 26 }), 'fcard-emblem'), fd.name), h('div.fcard-home', `${fd.home} · led by ${L.name}, ${L.title}`), h('div.fcard-plays', fd.plays_like)),
    ),
    opts.compact ? null : h('div.fcard-bio', ...turns(L.bio).map((t) => h('span', t.text + ' '))),
    h('dl.fcard-rules', ...rows.flatMap(([k, [t, d]]) => [h('dt', k), h('dd', h('b', t + '. '), d)])),
  );
}

export function rulesModal(focus?: FactionId) {
  const tabs = h('div.rules-tabs');
  const body = h('div.rules-body');
  const show = (which: FactionId | 'how') => {
    body.innerHTML = '';
    tabs.querySelectorAll('button').forEach((b) => b.classList.toggle('on', b.dataset.k === which));
    body.appendChild(which === 'how' ? howToPlay() : factionCard(which));
  };
  tabs.appendChild(h('button.pbtn', { 'data-k': 'how', onclick: () => show('how') }, 'How to play'));
  for (const f of FACTIONS) tabs.appendChild(h('button.pbtn', { 'data-k': f, onclick: () => show(f) }, faction(f).short));
  modal(h('div.rules', tabs, body));
  show(focus ?? 'how');
}

export function howToPlay(): HTMLElement {
  return h(
    'div.how',
    h('h2', 'Flyover Country in two minutes'),
    h('p', 'Four breakaway states fight over the ruins of the West for eight nights. Whoever has the most Bragging Rights after Night 8 wins. Nobody is ever eliminated, and every pilot ejects with a little parachute.'),
    h('h3', 'Every night'),
    h('ol',
      h('li', h('b', 'Dawn briefing. '), 'The newspaper, the scores, tonight\'s escalation stage and any special offers.'),
      h('li', h('b', 'Planning, in secret. '), 'Pass the laptop. Drag squadrons onto targets, queue builds and repairs, press Commit. Last night\'s orders carry over, so a quiet turn takes seconds.'),
      h('li', h('b', 'The show. '), 'Everything resolves at once and plays out on the TV. Yell at it.'),
      h('li', h('b', 'Morning report. '), 'Damage, losses, Bragging Rights and the Play of the Night.'),
    ),
    h('h3', 'Orders'),
    h('ul',
      h('li', h('b', 'Attack: '), 'bombers and strike units knock a node down a step (full, half, out).'),
      h('li', h('b', 'Guard: '), 'fighters defend a node all night and meet raiders headed for it (or flying right over it). Guard a neutral node and hold the sky at dawn to capture it.'),
      h('li', h('b', 'Escort: '), 'fighters fly with a strike and fight off interceptors.'),
      h('li', h('b', 'Sweep: '), 'fighters hunt any enemy aircraft passing over a state.'),
      h('li', h('b', 'Scout: '), 'reveal a state until next night, including damage and parked squadrons.'),
      h('li', h('b', 'Jam: '), 'blind air defense and radar along a strike\'s route or over a state.'),
      h('li', h('b', 'Rebase / Rest: '), 'move to another airbase, or stay home and repair faster.'),
    ),
    h('p', 'Fighters beat bombers and jammers. Air defense beats bombers and fighters. Jammers beat air defense. Scouts beat ignorance. Bigger groups win, with diminishing returns; a squadron that survives two fights earns a veteran star. A side with the first shot (a nearby Watchtower, SWITCHBACK, or Utah\'s guards) gets a quick snap volley before the dogfight.'),
    h('h3', 'Risk colors'),
    h('p', 'Every route is green (under 15% chance of losing the squadron), yellow (15 to 40%) or red (over 40%). Your dog advisor explains why in one sentence.'),
    h('h3', 'Escalation'),
    h('ul',
      h('li', h('b', 'Nights 1–2, Saber Rattling: '), 'military targets only (airbases, radar, air defense) and neutral ground. Race for the Big Empty.'),
      h('li', h('b', 'Nights 3–5, Open Season: '), 'cities, factories, fuel and junctions.'),
      h('li', h('b', 'Nights 6–8, Gloves Off: '), 'crown jewels and the Mass Driver. Crown jewels are hardened: it takes a heavy raid to crack one. Strike scores double on Night 8.'),
    ),
    h('h3', 'Bragging Rights'),
    h('ul',
      h('li', 'Hit a crown jewel: 8 (the Ball of Twine: 12).'),
      h('li', 'Shoot down an ace: 5. A squadron: 2. Aces score double for their kills.'),
      h('li', 'Knock a city or factory down a step: 2. Anything else: 1.'),
      h('li', 'Hold a neutral objective at dawn: 1 a night (the Boneyard and the Mass Driver: 2).'),
      h('li', 'Whoever is last at dawn scores 50% more for strikes that night.'),
    ),
    h('h3', 'Nobody gets knocked out: Project Bark Park'),
    h('p', 'A faction in the doghouse (last at dawn and 10 or more Bragging Rights behind the leader, or down to its last squadron) may, once a game between Nights 3 and 7, spend the night at Project Bark Park, a secret dog park at Four Corners. That night its squadrons stay home and its land can\'t be touched: bombs, the Mass Driver and everything else bounce off the fence. At dawn it comes back with everything repaired, at least five squadrons, 6 Supply, and one shot of the Big Fetch.'),
    h('p', h('b', 'The Big Fetch: '), 'a stadium-sized tennis ball launched from the park late in the night. Pick any target you could attack tonight in the Faction tab. It can\'t be intercepted and knocks the target down two steps (a crown jewel counts as a full hit). It doesn\'t get the last-place or Night 8 bonus.'),
    h('h3', 'Economy'),
    h('p', 'Supply builds and repairs; Fuel flies missions. Both only count once they can flow along your roads, rails and pipelines to your capital, so a well-chosen junction strike can hurt more than flattening a city. The Weak Spot button shows where you\'re thin.'),
    h('h3', 'The map'),
    h('p', 'East of the Mississippi is a wall of radio static. Pilots who fly into it don\'t come back, or come back and refuse to talk about it. The Big Empty (Idaho, Montana, Wyoming, Arizona, New Mexico) is up for grabs, including the Boneyard near Tucson (a free salvaged squadron every other night) and the White Sands Mass Driver (one unstoppable shot per game, from Night 6). Home regions of factions nobody picked become Fallen zones you can capture.'),
  );
}
