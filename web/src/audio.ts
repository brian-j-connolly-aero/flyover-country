// All sound is synthesized with WebAudio: no audio files to ship. A squelch
// before every radio line, launch roars, explosion thumps, a "MISSION UPDATE"
// sting, and a small synthwave loop that speeds up at Gloves Off.

class Audio {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private musicGain: GainNode | null = null;
  private enabled = true;
  private musicTimer: number | null = null;
  private stage = 0;
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
        this.musicGain.gain.value = 0.18;
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
      void this.ctx?.resume();
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
    }
  }

  /** A small synthwave loop. Gloves Off plays it faster and in a darker key. */
  music(stage: number) {
    this.stage = stage;
    if (this.musicTimer !== null) return;
    const ctx = this.ensure();
    if (!ctx) return;
    const bassLines = [
      [55, 55, 65.4, 49],
      [55, 65.4, 73.4, 49],
      [49, 46.2, 55, 41.2],
    ];
    let step = 0;
    let next = ctx.currentTime + 0.1;
    const schedule = () => {
      if (!this.enabled || !this.ctx) return;
      const bpm = this.stage === 2 ? 132 : this.stage === 1 ? 116 : 104;
      const beat = 60 / bpm / 2;
      while (next < this.ctx.currentTime + 0.4) {
        const bar = Math.floor(step / 8) % 4;
        const root = bassLines[Math.min(this.stage, 2)][bar];
        const note = step % 8 === 3 || step % 8 === 7 ? root * 2 : root;
        this.tone(note, 'sawtooth', next, beat * 0.9, 0.18, this.musicGain!);
        if (step % 8 === 0) {
          [root * 4, root * 5, root * 6].forEach((f) => this.tone(f, 'triangle', next, beat * 7, 0.035, this.musicGain!));
        }
        if (step % 2 === 0) this.tone(step % 4 === 0 ? 60 : 3000, step % 4 === 0 ? 'sine' : 'square', next, 0.05, step % 4 === 0 ? 0.25 : 0.01, this.musicGain!, step % 4 === 0 ? 40 : undefined);
        next += beat;
        step++;
      }
    };
    this.musicTimer = window.setInterval(schedule, 120);
  }

  stopMusic() {
    if (this.musicTimer !== null) clearInterval(this.musicTimer);
    this.musicTimer = null;
  }
}

export const audio = new Audio();
