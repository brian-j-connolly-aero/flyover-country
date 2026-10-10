// Night Zero: a short, skippable guided night with Princess Donut.

import type { Coach, PlanningScreen } from './planning';
import { h } from './dom';
import { leaderSvg } from '../art/dogs';

interface Step {
  text: string;
  done?: (s: PlanningScreen) => boolean;
  anchor?: string;
}

const STEPS: Step[] = [
  {
    text: "This is the West, and those glowing pink borders are ours. Our squadrons wait in the tray at the bottom. Drag the Unicorn Swarm or a Bombers card onto an enemy airbase with a red ring. Tonight is Saber Rattling, so only military targets are open.",
    done: (s) => s.orders.missions.some((m) => m.mission.t === 'Attack'),
    anchor: '.plan-tray',
  },
  {
    text: "See the route? Its color is the risk: green is fine, yellow is risky, red means a lot of learnings. Click that card in the tray to read why in one sentence.",
    done: (s) => s.selected !== null && s.orders.missions.some((m) => m.sq === s.selected && m.mission.t === 'Attack'),
    anchor: '.plan-tray',
  },
  {
    text: "Bombers hate flying alone. Drag a Fighters card onto your bomber's card (or use the ⋯ button) and choose Escort. Fighters can also Guard a node: drop one on a node with a green ring.",
    done: (s) => s.orders.missions.some((m) => m.mission.t === 'Escort' || m.mission.t === 'Guard'),
    anchor: '.plan-tray',
  },
  {
    text: "Want the neutral objectives? Guard one in the Big Empty and hold the sky at dawn to capture it. Or press Suggest Plan and I'll draft everything. My notes on the right flag problems, worst first.",
    anchor: '.plan-btns',
  },
  {
    text: "When you're happy, press Commit orders. Then watch the show: Space pauses, 1, 2 and 4 change speed. Ship it!",
    anchor: '.commit',
  },
];

export function tutorialCoach(): Coach {
  let i = 0;
  let box: HTMLElement | null = null;
  const render = (screen: PlanningScreen) => {
    box?.remove();
    if (i >= STEPS.length) return;
    const st = STEPS[i];
    box = h(
      'div.coach',
      h('div.coach-dog', { html: leaderSvg('Pacific', i === STEPS.length - 1 ? 'happy' : 'neutral', { size: 72, bg: true }) }),
      h(
        'div.coach-body',
        h('div.coach-step', `Night Zero · ${i + 1} of ${STEPS.length}`),
        h('div.coach-text', st.text),
        h(
          'div.coach-btns',
          h('button.pbtn', { onclick: () => { i++; render(screen); } }, i === STEPS.length - 1 ? 'Got it' : 'Next'),
          h('button.pbtn', { onclick: () => { i = STEPS.length; box?.remove(); } }, 'Skip tutorial'),
        ),
      ),
    );
    document.body.appendChild(box);
    const target = st.anchor ? document.querySelector(st.anchor) : null;
    target?.classList.add('coach-glow');
    setTimeout(() => target?.classList.remove('coach-glow'), 2500);
  };
  let watching: MutationObserver | null = null;
  return {
    step(screen: PlanningScreen) {
      if (i < STEPS.length && STEPS[i].done?.(screen)) i++;
      if (!screen.el.isConnected) {
        box?.remove();
        return;
      }
      render(screen);
      // Tidy up the coach once the planning screen goes away.
      watching?.disconnect();
      watching = new MutationObserver(() => {
        if (!screen.el.isConnected) {
          box?.remove();
          watching?.disconnect();
        }
      });
      watching.observe(document.body, { childList: true, subtree: true });
    },
  };
}
