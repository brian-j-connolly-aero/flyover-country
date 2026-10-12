// Learn by playing: four short tips from Princess Donut during the first
// planning screen. Each one moves on by itself once you've done it.

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
    text: 'Drag a Bombers card onto an enemy Airfield. The ones you can hit tonight get a red ring.',
    done: (s) => s.orders.missions.some((m) => m.mission.t === 'Fly' && s.squad(m.sq)?.kind === 'Bomber'),
    anchor: '.plan-tray',
  },
  {
    text: 'Now drag a Fighters card onto our own land. The circle is the sky they guard tonight.',
    done: (s) => s.orders.missions.some((m) => m.mission.t === 'Patrol'),
    anchor: '.plan-tray',
  },
  {
    text: 'Faded marks are where everyone flew last night. A dashed ring on your raid’s path is a patrol that might be there again.',
  },
  {
    text: 'Happy? Press Commit, then watch the night play out.',
    anchor: '.commit',
  },
];

export function tutorialCoach(): Coach {
  let i = 0;
  let box: HTMLElement | null = null;
  let shown = -1;
  const render = (screen: PlanningScreen) => {
    if (shown === i && box?.isConnected) return;
    shown = i;
    box?.remove();
    if (i >= STEPS.length) return;
    const st = STEPS[i];
    box = h(
      'div.coach',
      h('div.coach-dog', { html: leaderSvg('Pacific', i === STEPS.length - 1 ? 'happy' : 'neutral', { size: 64, bg: true }) }),
      h(
        'div.coach-body',
        h('div.coach-text', st.text),
        h(
          'div.coach-btns',
          st.done ? null : h('button.btn', { onclick: () => { i++; render(screen); } }, i === STEPS.length - 1 ? 'Got it' : 'Next'),
          h('button.link', { onclick: () => { i = STEPS.length; box?.remove(); } }, 'Skip tips'),
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
      while (i < STEPS.length && STEPS[i].done?.(screen)) i++;
      if (!screen.el.isConnected) {
        box?.remove();
        return;
      }
      render(screen);
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
