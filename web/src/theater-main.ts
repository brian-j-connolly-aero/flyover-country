import '@fontsource/alfa-slab-one/400.css';
import '@fontsource/atkinson-hyperlegible/400.css';
import '@fontsource/atkinson-hyperlegible/700.css';
import '@fontsource/atkinson-hyperlegible/400-italic.css';
import './style.css';
import { Engine } from './engine';
import { setContent } from './text';
import { setFactionColors } from './theme';
import { Theater } from './ui/theater';
import { Bus, type FromTheater, type ToTheater } from './bus';
import { audio } from './audio';

// The Theater window is a dumb renderer: public state and event logs in,
// "continue" and playback status out.
async function boot() {
  audio.unlockOnGesture();
  const root = document.getElementById('theater')!;
  const engine = await Engine.load();
  setContent(engine.content);
  setFactionColors(engine.content);
  root.innerHTML = '';
  let theater: Theater | null = null;
  const bus = new Bus<ToTheater, FromTheater>((m) => void theater?.handle(m));
  theater = new Theater(root, engine.content, (m) => bus.send(m));
  bus.send({ type: 'ready' });
  root.addEventListener('dblclick', () => {
    if (!document.fullscreenElement) void document.documentElement.requestFullscreen?.();
  });
}

void boot();
