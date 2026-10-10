import '@fontsource/oswald/500.css';
import '@fontsource/oswald/700.css';
import '@fontsource/jetbrains-mono/400.css';
import './style.css';
import { Engine } from './engine';
import { setContent } from './text';
import { App } from './ui/app';
import { audio } from './audio';

async function boot() {
  audio.unlockOnGesture();
  const root = document.getElementById('app')!;
  try {
    const engine = await Engine.load();
    setContent(engine.content);
    root.innerHTML = '';
    const app = new App(root, engine);
    (window as unknown as { flyover: App; flyoverAudio: typeof audio }).flyover = app;
    (window as unknown as { flyoverAudio: typeof audio }).flyoverAudio = audio;
    await app.menu();
  } catch (e) {
    console.error(e);
    root.innerHTML = `<div class="boot">Something went wrong loading the game.<br><span>${String((e as Error).message ?? e)}</span></div>`;
  }
  if ('serviceWorker' in navigator && import.meta.env.PROD) {
    navigator.serviceWorker.register('./sw.js').catch(() => undefined);
  }
}

void boot();
