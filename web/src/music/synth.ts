// Plays the soundtrack (songs.ts, parsed by mml.ts) with WebAudio voices:
// NES-style pulse waves and a triangle bass, a filtered-saw "brass" lead, a
// soft string pad, and a synthesized drum kit. Notes are scheduled a little
// ahead on the audio clock, so playback stays steady even when the page is busy.

import { parseSong, type Drum, type Inst, type ParsedNote, type ParsedSong } from './mml';
import { SONGS } from './songs';

export type TrackId = 'theme' | 'briefing' | 'sortie' | 'glovesoff';

const LOOKAHEAD = 0.6; // seconds of music scheduled ahead
const TICK_MS = 120;

/** Loudness of each voice at v15, before the music bus. */
const LEVEL: Record<Inst, number> = { pulse12: 0.105, pulse25: 0.08, pulse50: 0.07, tri: 0.2, saw: 0.075, pad: 0.06, noise: 1 };

interface Playing {
  song: ParsedSong;
  gain: GainNode;
  start: number; // audio time of beat 0
  cursor: number; // absolute beats scheduled so far
  timer: number;
}

export class MusicPlayer {
  private readonly ctx: BaseAudioContext;
  private readonly out: AudioNode;
  private readonly songs = new Map<string, ParsedSong>();
  private waves = new Map<string, PeriodicWave>();
  private noise: AudioBuffer;
  private current: Playing | null = null;
  private currentId: string | null = null;

  constructor(ctx: BaseAudioContext, out: AudioNode) {
    this.ctx = ctx;
    this.out = out;
    for (const s of SONGS) this.songs.set(s.id, parseSong(s));
    for (const [inst, duty] of [['pulse12', 0.125], ['pulse25', 0.25], ['pulse50', 0.5]] as const) this.waves.set(inst, pulseWave(ctx, duty));
    const len = Math.floor(ctx.sampleRate * 1.5);
    this.noise = ctx.createBuffer(1, len, ctx.sampleRate);
    const d = this.noise.getChannelData(0);
    // A fixed seed: the drums sound the same every time.
    let x = 0x2545f491;
    for (let i = 0; i < len; i++) {
      x ^= x << 13;
      x ^= x >>> 17;
      x ^= x << 5;
      d[i] = ((x >>> 0) / 0xffffffff) * 2 - 1;
    }
  }

  get playing(): string | null {
    return this.currentId;
  }

  /** Start a track (crossfading from whatever is playing), or stop with null. */
  play(id: TrackId | null, when = this.ctx.currentTime + 0.05) {
    if (id === this.currentId) return;
    this.fadeOut(when);
    this.currentId = id;
    if (!id) return;
    const song = this.songs.get(id);
    if (!song) return;
    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(0.0001, when);
    gain.gain.exponentialRampToValueAtTime(1, when + 0.9);
    gain.connect(this.out);
    const p: Playing = { song, gain, start: when + 0.1, cursor: 0, timer: 0 };
    this.current = p;
    this.schedule(p);
    if (typeof window !== 'undefined') p.timer = window.setInterval(() => this.schedule(p), TICK_MS);
  }

  stop() {
    this.play(null);
  }

  private fadeOut(when: number) {
    const p = this.current;
    if (!p) return;
    clearInterval(p.timer);
    p.gain.gain.cancelScheduledValues(when);
    p.gain.gain.setValueAtTime(p.gain.gain.value || 1, when);
    p.gain.gain.exponentialRampToValueAtTime(0.0001, when + 1.2);
    const g = p.gain;
    setTimeout(() => g.disconnect(), 2000);
    this.current = null;
  }

  /** Render a whole song (intro plus `loops` passes) offline: for previews and level checks. */
  scheduleRange(id: TrackId, start: number, seconds: number) {
    const song = this.songs.get(id)!;
    const gain = this.ctx.createGain();
    gain.connect(this.out);
    const p: Playing = { song, gain, start, cursor: 0, timer: 0 };
    this.scheduleUntil(p, (seconds * song.bpm) / 60);
  }

  private schedule(p: Playing) {
    if (this.current !== p) return;
    const beats = ((this.ctx.currentTime + LOOKAHEAD - p.start) * p.song.bpm) / 60;
    this.scheduleUntil(p, beats);
  }

  /** Schedule every event up to `target` absolute beats, looping from the song's `$`. */
  private scheduleUntil(p: Playing, target: number) {
    const s = p.song;
    const L = s.length;
    const loop = Math.max(1e-6, L - s.loopAt);
    const spb = 60 / s.bpm;
    while (p.cursor < target - 1e-9) {
      let segStart: number;
      let offset: number;
      let segEnd: number;
      if (p.cursor < L) {
        segStart = 0;
        offset = 0;
        segEnd = L;
      } else {
        const k = Math.floor((p.cursor - L) / loop + 1e-9);
        segStart = L + k * loop;
        offset = s.loopAt;
        segEnd = segStart + loop;
      }
      const end = Math.min(target, segEnd);
      const from = offset + (p.cursor - segStart);
      const to = offset + (end - segStart);
      const at = (beat: number) => p.start + (segStart + beat - offset) * spb;
      for (const ch of s.channels) {
        for (const n of ch.notes) if (n.t >= from - 1e-9 && n.t < to - 1e-9) this.note(ch.inst, n, at(n.t), spb, p.gain);
        for (const d of ch.drums) if (d.t >= from - 1e-9 && d.t < to - 1e-9) this.drum(d.drum, d.vol, at(d.t), p.gain);
      }
      p.cursor = end;
    }
  }

  // --- Voices ---------------------------------------------------------------------------------

  private note(inst: Inst, n: ParsedNote, t: number, spb: number, dest: AudioNode) {
    if (t < this.ctx.currentTime - 0.01) return;
    const ctx = this.ctx;
    const f = 440 * Math.pow(2, (n.midi - 69) / 12);
    const len = Math.max(0.03, n.gate * spb);
    const peak = LEVEL[inst] * (n.vol / 15);
    if (peak <= 0) return;
    const g = ctx.createGain();
    g.connect(dest);
    const oscs: OscillatorNode[] = [];
    let release = 0.05;
    if (inst === 'pad') {
      release = 0.9;
      const lp = ctx.createBiquadFilter();
      lp.type = 'lowpass';
      lp.frequency.value = 1300;
      lp.connect(g);
      for (const cents of [-8, 7]) {
        const o = ctx.createOscillator();
        o.type = 'sawtooth';
        o.frequency.value = f;
        o.detune.value = cents;
        o.connect(lp);
        oscs.push(o);
      }
      g.gain.setValueAtTime(0.0001, t);
      g.gain.linearRampToValueAtTime(peak, t + Math.min(0.45, len * 0.5));
      g.gain.setValueAtTime(peak, t + len);
    } else if (inst === 'saw') {
      const lp = ctx.createBiquadFilter();
      lp.type = 'lowpass';
      lp.Q.value = 1.4;
      lp.frequency.setValueAtTime(3200, t);
      lp.frequency.exponentialRampToValueAtTime(1500, t + 0.18);
      lp.connect(g);
      const o = ctx.createOscillator();
      o.type = 'sawtooth';
      o.frequency.value = f;
      o.connect(lp);
      oscs.push(o);
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(peak, t + 0.012);
      g.gain.exponentialRampToValueAtTime(peak * 0.75, t + Math.min(0.15, len * 0.6));
      g.gain.setValueAtTime(peak * 0.75, t + len);
      release = 0.08;
    } else {
      const o = ctx.createOscillator();
      if (inst === 'tri') o.type = 'triangle';
      else o.setPeriodicWave(this.waves.get(inst)!);
      o.frequency.value = f;
      o.connect(g);
      oscs.push(o);
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(peak, t + 0.004);
      if (inst !== 'tri') g.gain.exponentialRampToValueAtTime(peak * 0.72, t + Math.min(0.09, len * 0.6));
      g.gain.setValueAtTime(inst === 'tri' ? peak : peak * 0.72, t + len);
      release = inst === 'tri' ? 0.025 : 0.05;
    }
    // Vibrato on held lead notes, after a moment, like a real player.
    if (len > 0.35 && inst !== 'tri' && inst !== 'pad') {
      const lfo = ctx.createOscillator();
      const depth = ctx.createGain();
      lfo.frequency.value = 5.6;
      depth.gain.setValueAtTime(0, t);
      depth.gain.linearRampToValueAtTime(f * 0.006, t + Math.min(0.5, len));
      lfo.connect(depth);
      for (const o of oscs) depth.connect(o.frequency);
      lfo.start(t);
      lfo.stop(t + len + release + 0.05);
    }
    g.gain.exponentialRampToValueAtTime(0.0001, t + len + release);
    for (const o of oscs) {
      o.start(t);
      o.stop(t + len + release + 0.05);
    }
  }

  private drum(d: Drum, vol: number, t: number, dest: AudioNode) {
    if (t < this.ctx.currentTime - 0.01) return;
    const v = vol / 15;
    if (v <= 0) return;
    if (d === 'k' || d === 'c') this.kick(t, v * 0.45, dest);
    if (d === 's') {
      this.noiseHit(t, v * 0.32, 'bandpass', 1900, 0.7, 0.17, dest);
      this.tone(t, 185, 140, v * 0.16, 0.09, 'triangle', dest);
    }
    if (d === 'h') this.noiseHit(t, v * 0.11, 'highpass', 7200, 0.7, 0.045, dest);
    if (d === 'o') this.noiseHit(t, v * 0.1, 'highpass', 6500, 0.7, 0.28, dest);
    if (d === 'c') this.noiseHit(t, v * 0.16, 'highpass', 4200, 0.5, 1.3, dest);
    if (d === 't') this.tone(t, 120, 72, v * 0.42, 0.26, 'sine', dest);
    if (d === 'm') this.tone(t, 175, 105, v * 0.38, 0.22, 'sine', dest);
  }

  private kick(t: number, v: number, dest: AudioNode) {
    this.tone(t, 150, 44, v, 0.22, 'sine', dest);
    this.noiseHit(t, v * 0.25, 'highpass', 3000, 0.7, 0.008, dest);
  }

  private tone(t: number, f0: number, f1: number, v: number, dur: number, type: OscillatorType, dest: AudioNode) {
    const o = this.ctx.createOscillator();
    const g = this.ctx.createGain();
    o.type = type;
    o.frequency.setValueAtTime(f0, t);
    o.frequency.exponentialRampToValueAtTime(f1, t + dur * 0.7);
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(v, t + 0.003);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    o.connect(g).connect(dest);
    o.start(t);
    o.stop(t + dur + 0.02);
  }

  private noiseHit(t: number, v: number, type: BiquadFilterType, freq: number, q: number, dur: number, dest: AudioNode) {
    const src = this.ctx.createBufferSource();
    src.buffer = this.noise;
    const f = this.ctx.createBiquadFilter();
    f.type = type;
    f.frequency.value = freq;
    f.Q.value = q;
    const g = this.ctx.createGain();
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(v, t + 0.002);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    src.connect(f).connect(g).connect(dest);
    src.start(t, (t * 7.31) % 0.5);
    src.stop(t + dur + 0.02);
  }
}

/** An NES-style pulse wave with the given duty cycle. */
function pulseWave(ctx: BaseAudioContext, duty: number): PeriodicWave {
  const n = 40;
  const real = new Float32Array(n);
  const imag = new Float32Array(n);
  for (let k = 1; k < n; k++) {
    real[k] = Math.sin(2 * Math.PI * k * duty) / (k * Math.PI);
    imag[k] = (1 - Math.cos(2 * Math.PI * k * duty)) / (k * Math.PI);
  }
  return ctx.createPeriodicWave(real, imag);
}
