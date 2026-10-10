import '@fontsource/oswald/500.css';
import '@fontsource/oswald/700.css';
import '@fontsource/jetbrains-mono/400.css';
import './style.css';
import { Engine } from './engine';
import { setContent } from './text';
import { Theater } from './ui/theater';
import { Bus, type FromTheater, type ToTheater } from './bus';

// The Theater window is a dumb renderer: public state and event logs in,
// "continue" and playback status out.
async function boot() {
  const root = document.getElementById('theater')!;
  const engine = await Engine.load();
  setContent(engine.content);
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
