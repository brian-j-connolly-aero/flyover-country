// All sound is synthesized with WebAudio: no audio files to ship. A squelch
// before every radio line, launch roars, explosion thumps, a "MISSION UPDATE"
// sting, and the four-song soundtrack (music/songs.ts, played by music/synth.ts).

/** The music bus, relative to the master volume. */
const MUSIC_LEVEL = 0.8;

function readMusicSetting(): boolean {
  try {
    return localStorage.getItem('flyover-music') !== '0';
  } catch {
    return true;
  }
}

import { MusicPlayer, type TrackId } from './music/synth';

class Audio {
  private ctx: AudioContext | null = null;
  private songs: MusicPlayer | null = null;
  private wantTrack: TrackId | null = null;
  private musicOn = readMusicSetting();
  private master: GainNode | null = null;
  private musicGain: GainNode | null = null;
  private enabled = true;
  private lastPlayed = new Map<string, number>();

  private ensure(): AudioContext | null {
    if (!this.enabled) return null;
    if (!this.ctx) {
      try {
        this.ctx = new AudioContext();
        this.master = this.ctx.createGain();
        this.master.gain.value = 0.5;
        this.master.connect(this.ctx.destination);
        this.musicGain = this.ctx.createGain();
        this.musicGain.gain.value = MUSIC_LEVEL;
        this.musicGain.connect(this.master);
      } catch {
        return null;
      }
    }
    if (this.ctx.state === 'suspended') void this.ctx.resume();
    return this.ctx;
  }

  setEnabled(on: boolean) {
    this.enabled = on;
    if (!on) {
      this.stopMusic();
      void this.ctx?.suspend();
    } else {
      void this.ctx?.resume().then(() => this.applyMusic());
    }
  }

  get on() {
    return this.enabled;
  }

  private noise(dur: number): AudioBufferSourceNode | null {
    const ctx = this.ctx!;
    const len = Math.max(1, Math.floor(ctx.sampleRate * dur));
    const buf = ctx.createBuffer(1, len, ctx.sampleRate);
    const d = buf.getChannelData(0);
    for (let i = 0; i < len; i++) d[i] = Math.random() * 2 - 1;
    const src = ctx.createBufferSource();
    src.buffer = buf;
    return src;
  }

  private env(g: GainNode, t: number, a: number, peak: number, dec: number) {
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(peak, t + a);
    g.gain.exponentialRampToValueAtTime(0.0001, t + a + dec);
  }

  private tone(freq: number, type: OscillatorType, t: number, dur: number, peak: number, dest?: AudioNode, slideTo?: number) {
    const ctx = this.ctx!;
    const o = ctx.createOscillator();
    const g = ctx.createGain();
    o.type = type;
    o.frequency.setValueAtTime(freq, t);
    if (slideTo) o.frequency.exponentialRampToValueAtTime(slideTo, t + dur);
    this.env(g, t, 0.01, peak, dur);
    o.connect(g).connect(dest ?? this.master!);
    o.start(t);
    o.stop(t + dur + 0.05);
  }

  private burst(t: number, dur: number, freq: number, q: number, peak: number, type: BiquadFilterType = 'bandpass') {
    const ctx = this.ctx!;
    const n = this.noise(dur)!;
    const f = ctx.createBiquadFilter();
    f.type = type;
    f.frequency.value = freq;
    f.Q.value = q;
    const g = ctx.createGain();
    this.env(g, t, 0.005, peak, dur);
    n.connect(f).connect(g).connect(this.master!);
    n.start(t);
    n.stop(t + dur + 0.05);
  }

  play(kind: string) {
    const ctx = this.ensure();
    if (!ctx) return;
    const now = ctx.currentTime;
    if (kind.startsWith('motif:')) {
      this.motif(kind.slice(6), now);
      return;
    }
    // Don't machine-gun the same sound.
    const last = this.lastPlayed.get(kind) ?? 0;
    const gap = kind === 'guns' || kind === 'flak' ? 0.18 : 0.08;
    if (now - last < gap) return;
    this.lastPlayed.set(kind, now);
    switch (kind) {
      case 'squelch':
        this.burst(now, 0.09, 2400, 1.5, 0.25);
        this.tone(1800, 'square', now + 0.02, 0.05, 0.03);
        break;
      case 'launch':
        this.burst(now, 0.7, 400, 0.6, 0.12, 'lowpass');
        break;
      case 'guns':
        for (let i = 0; i < 5; i++) this.burst(now + i * 0.05, 0.03, 1500, 2, 0.18);
        break;
      case 'duel':
        for (let i = 0; i < 8; i++) this.burst(now + i * 0.04, 0.03, 1800, 2, 0.2);
        this.tone(330, 'sawtooth', now, 0.5, 0.05, undefined, 660);
        break;
      case 'flak':
        for (let i = 0; i < 3; i++) this.burst(now + i * 0.11, 0.12, 300, 1, 0.3, 'lowpass');
        break;
      case 'splash':
        this.tone(900, 'triangle', now, 0.6, 0.08, undefined, 180);
        break;
      case 'boom':
      case 'thud':
        this.burst(now, kind === 'boom' ? 0.6 : 0.3, 120, 0.7, kind === 'boom' ? 0.6 : 0.3, 'lowpass');
        this.tone(70, 'sine', now, 0.5, 0.4, undefined, 35);
        break;
      case 'bigboom':
        this.burst(now, 1.4, 90, 0.5, 0.9, 'lowpass');
        this.tone(55, 'sine', now, 1.2, 0.6, undefined, 25);
        break;
      case 'sting':
        [392, 494, 587, 784].forEach((f, i) => this.tone(f, 'sawtooth', now + i * 0.02, 0.9, 0.06));
        this.tone(98, 'square', now, 0.9, 0.08);
        break;
      case 'coin':
        [988, 1319, 1568, 1976].forEach((f, i) => this.tone(f, 'square', now + i * 0.07, 0.15, 0.06));
        break;
      case 'static':
        this.burst(now, 1.2, 3000, 0.3, 0.2, 'highpass');
        break;
      case 'report':
        [523, 659, 784].forEach((f, i) => this.tone(f, 'triangle', now + i * 0.12, 0.4, 0.08));
        break;
      case 'fanfare':
        [523, 659, 784, 1047, 784, 1047].forEach((f, i) => this.tone(f, 'sawtooth', now + i * 0.16, 0.35, 0.07));
        break;
      case 'click':
        this.tone(1200, 'square', now, 0.03, 0.03);
        break;
      case 'commit':
        [660, 880].forEach((f, i) => this.tone(f, 'triangle', now + i * 0.08, 0.2, 0.07));
        break;
      case 'boing': {
        // Off the Bark Park fence: a springy wobble.
        const o = ctx.createOscillator();
        const g = ctx.createGain();
        const lfo = ctx.createOscillator();
        const depth = ctx.createGain();
        o.type = 'sine';
        o.frequency.setValueAtTime(170, now);
        o.frequency.linearRampToValueAtTime(520, now + 0.08);
        o.frequency.exponentialRampToValueAtTime(260, now + 0.45);
        lfo.frequency.value = 17;
        depth.gain.setValueAtTime(60, now);
        depth.gain.exponentialRampToValueAtTime(1, now + 0.45);
        lfo.connect(depth).connect(o.frequency);
        this.env(g, now, 0.01, 0.22, 0.45);
        o.connect(g).connect(this.master!);
        o.start(now);
        lfo.start(now);
        o.stop(now + 0.55);
        lfo.stop(now + 0.55);
        break;
      }
      case 'squeak':
        // A dog toy, stadium-sized.
        this.tone(1350, 'triangle', now, 0.09, 0.12, undefined, 1950);
        this.tone(1750, 'triangle', now + 0.1, 0.14, 0.12, undefined, 1250);
        break;
      case 'fetch':
        // The launcher: a thump, a whoosh and a rising whistle.
        this.tone(95, 'sine', now, 0.35, 0.5, undefined, 40);
        this.burst(now, 0.6, 700, 0.5, 0.18, 'lowpass');
        this.tone(600, 'sine', now + 0.1, 0.9, 0.05, undefined, 1500);
        break;
      case 'park':
        // A happy little jingle, and a woof.
        [523, 659, 784, 1047].forEach((f, i) => this.tone(f, 'triangle', now + i * 0.1, 0.22, 0.08));
        this.tone(170, 'sawtooth', now + 0.5, 0.12, 0.08, undefined, 110);
        this.tone(170, 'sawtooth', now + 0.68, 0.12, 0.08, undefined, 110);
        break;
    }
  }

  /** Each faction's four-note motif, for its big moments. */
  private motif(faction: string, now: number) {
    const MOTIFS: Record<string, [number[], OscillatorType]> = {
      Pacific: [[659, 784, 988, 1319], 'square'], // bright and upbeat, like a launch keynote
      Cascadia: [[392, 440, 523, 440], 'triangle'], // soft, rainy
      FrontRange: [[523, 659, 784, 1047], 'triangle'], // wide-open mountain fifths
      Texas: [[196, 247, 294, 392], 'sawtooth'], // big brass
      Dreamland: [[587, 698, 880, 1175], 'square'], // slot-machine jingle
      GrainBelt: [[330, 392, 440, 392], 'triangle'], // county-fair fiddle
      Beehive: [[523, 587, 659, 784], 'sine'], // polite and cheerful
    };
    const m = MOTIFS[faction];
    if (!m) return;
    m[0].forEach((f, i) => this.tone(f, m[1], now + 0.25 + i * 0.13, 0.22, 0.07));
  }

  // --- Music ----------------------------------------------------------------------------------

  /** Ask for a track; it starts as soon as the browser lets audio play (after the first click). */
  music(track: TrackId | null) {
    this.wantTrack = track;
    this.applyMusic();
  }

  setMusic(on: boolean) {
    this.musicOn = on;
    try {
      localStorage.setItem('flyover-music', on ? '1' : '0');
    } catch {
      // Private windows may refuse storage; the setting just won't stick.
    }
    this.applyMusic();
  }

  get musicEnabled() {
    return this.musicOn;
  }

  private applyMusic() {
    if (!this.enabled) {
      this.songs?.play(null);
      return;
    }
    const ctx = this.ensure();
    if (!ctx || ctx.state !== 'running') return;
    this.songs ??= new MusicPlayer(ctx, this.musicGain!);
    this.songs.play(this.musicOn ? this.wantTrack : null);
  }

  /** Browsers start audio suspended until the first click or key press. */
  unlockOnGesture() {
    const go = () => {
      const ctx = this.ensure();
      if (ctx) void ctx.resume().then(() => this.applyMusic());
    };
    window.addEventListener('pointerdown', go, { capture: true });
    window.addEventListener('keydown', go, { capture: true });
  }

  stopMusic() {
    this.songs?.play(null);
  }
}

export const audio = new Audio();
