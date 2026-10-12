// The rules on one card, and a small card per faction: who leads it and its
// one perk. Nothing else to learn.

import type { FactionId } from '../types';
import { FACTIONS } from '../types';
import { faction, getContent } from '../text';
import { FACTION_HEX } from '../theme';
import { leaderSvg, emblemSvg } from '../art/dogs';
import { crateSvg, factionPlaneSvg, prizeSvg, siteSvg } from './glyph';
import { h, svg, modal } from './dom';

export function factionCard(f: FactionId): HTMLElement {
  const fd = faction(f);
  return h(
    'div.fcard',
    { style: { '--fc': fd.color } as unknown as CSSStyleDeclaration },
    svg(leaderSvg(f, 'neutral', { size: 92, bg: true }), 'fcard-dog'),
    h(
      'div.fcard-body',
      h('div.fcard-name', svg(emblemSvg(f, { size: 22 }), 'fcard-emblem'), fd.short),
      h('div.fcard-leader', fd.leader),
      h('div.fcard-perk', h('b', fd.perk), h('span', fd.perk_text)),
    ),
  );
}

export function rulesModal() {
  modal(h('div.rules', howToPlay()));
}

export function howToPlay(): HTMLElement {
  const c = getContent();
  const r = c.rules;
  const ink = '#8b7e70';
  const block = (icon: string, title: string, ...body: (string | HTMLElement)[]) => h('div.rule', svg(icon, 'rule-icon'), h('div', h('h3', title), ...body.map((b) => (typeof b === 'string' ? h('p', b) : b))));
  return h(
    'div.how',
    h('h2', 'How to play'),
    h('p.how-lead', `Most points after Night ${r.nights} wins. Each night everyone plans in secret, then it all plays out at once on the TV.`),
    h(
      'div.rule-grid',
      block(factionPlaneSvg('Texas', 'Bomber', 36), 'Bombers', 'Drop them on an enemy site to bomb it. Each bomber plane knocks off a pip about half the time.'),
      block(factionPlaneSvg('Texas', 'Fighter', 36), 'Fighters', 'Drop them on your own land to patrol. A patrol jumps raids that fly through its circle on the way to your sites or a prize. Or send them along with a raid to fight off patrols.'),
      block(siteSvg('Factory', ink, 36), 'Sites', 'Each side has an HQ, a Factory, an Airfield and a Crown Jewel. Pips show their health. Your perk works while your Crown Jewel stands.'),
      block(prizeSvg(null, 36), 'Prizes', `First raid to reach a prize holds it. Hold it at dawn for ${r.score.prize} point and ${r.income.prize} Supply.`),
      block(
        siteSvg('HQ', ink, 36),
        'Points',
        `${r.score.pip} per pip you knock off (${r.score.big_pip} for an HQ or Crown Jewel), ${r.score.plane} per plane you shoot down, ${r.score.prize} per prize held at dawn.`,
      ),
      block(crateSvg(36), 'Supply', `New fighters ${r.costs.fighter}, bombers ${r.costs.bomber}, a repair ${r.costs.repair}. You get ${r.income.hq} for your HQ plus 1 per Factory pip each morning. A squadron that rests gets a plane back.`),
      block(siteSvg('Jewel', ink, 36), 'What opens when', `Nights 1–${r.open_factories - 1}: Airfields and prizes. From Night ${r.open_factories}: Factories. From Night ${r.open_hqs}: HQs and Crown Jewels.`),
      block(
        '<svg viewBox="0 0 40 40" width="36" height="36"><rect x="5" y="11" width="30" height="20" rx="3" fill="#8fbf6a" stroke="#231e1a" stroke-width="2"/><circle cx="26" cy="21" r="4" fill="#d7e63e" stroke="#231e1a" stroke-width="1.5"/></svg>',
        'Project Bark Park',
        `Way behind? Once a game (Nights ${r.bark_park.first_night}–${r.bark_park.last_night}) hide at the secret dog park: nothing can hit you that night. You come back repaired with the Big Fetch, one unstoppable hit for ${r.bark_park.fetch_pips} pips.`,
      ),
    ),
    h('h3.perks-h', 'Perks'),
    h(
      'div.perk-list',
      ...FACTIONS.map((f) => {
        const fd = faction(f);
        return h('div.perk-row', { style: { '--fc': FACTION_HEX[f] } as unknown as CSSStyleDeclaration }, svg(emblemSvg(f, { size: 22 })), h('b', fd.short), h('span', `${fd.perk}: ${fd.perk_text}`));
      }),
    ),
  );
}
