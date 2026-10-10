import type { SongDef } from './mml';

// The Flyover Country soundtrack, written in MML (see mml.ts for the dialect).
// Chiptune voices (NES pulse/triangle/noise) with an Ace Combat accent: a
// soft string pad, a filtered-saw brass lead, rock drums, minor-key heroics.
//
// LINER NOTES
//
// 1. Flyover Country (Main Theme): menu and final results.
//    D major with a mixolydian bVII (C), 120 BPM. Form: Intro(4) $ A A' B A''
//    (8 bars each, 32-bar loop). The hook (bars 1-4): A4. D5 F#5 A5 | G5. E5
//    C5 | D5. B4 G5 (a Western sixth leap) | E5. Galloping triangle bass
//    (8th + two 16ths) on root/fifth; NES broken-chord arpeggios in A and A'
//    (the string pad joins in A'), a twin lead in sixths/thirds in B and A''.
//    A: D C G A | D C G-A D.  A': D C G A | D C Em A.
//    B: Bm G D A | Bm G Em A.  A'': D C G A | D C G A (the last A turns back to D).
//
// 2. Briefing Room: dawn newspaper, secret planning, morning report.
//    A minor, 92 BPM. Form: A B A' B' (8 bars each, 32-bar loop, no intro).
//    A quiet 3+3+2 arpeggio whose three voices move by half steps
//    (A3 E4 C5 -> A3 F4 C5 -> G#3 E4 B4), a soft pulsing eighth-note
//    triangle, a held pad line (A4 sighing to G#4). No drums for the first 8
//    bars, then ticking offbeat hats and a soft kick. A sparse pulse50 line
//    only in A' and B' (E5 C5 | A4 | D5 C5 | B4 G#4).
//    A: Am Fmaj7 Dm7 Esus4-E | Am Fmaj7 Dm7 E.
//    B: F G C Am | Dm Bb (Neapolitan colour) E7 E.
//
// 3. Night Sortie: nights 1-5.
//    E minor, 144 BPM. Form: Intro(2) $ Riff(4) A A' Chorus Chorus' Bridge
//    (8 bars each, 44-bar loop). Riff: E4 G4 B4 E5 D5 B4 over i-bVI-bVII-V,
//    doubled an octave down by the saw. Verses are call and response: the
//    pulse lead calls (B4 E5 F#5 G5...), the saw brass answers in the gaps.
//    The chorus soars, leaping up from each downbeat (E5-G5 | D5-F#5 | D5-B5)
//    over IV-V-I-vi of the relative major; each chorus enters deceptively,
//    the B major before it resolving to C instead of Em. Galloping
//    octave bass in the verses, straight octaves in the chorus, a half-time
//    bridge that builds back into the riff.
//    Riff: Em C D B.  A: Em C D Bm | Em C Am B.  A': Em C D Bm | C D Am B.
//    Chorus: C D G Em | C D B B.  Chorus': C D G Em | C D Em Em.
//    Bridge: Am Em C B | Am Em C B.
//
// 4. Gloves Off: nights 6-8, the climax.
//    B minor (harmonic minor), F# Phrygian-dominant stabs, 160 BPM.
//    Form: Intro(2) $ A A' Riff(4) Climb Climax Riff'(4) (8-bar sections,
//    40-bar loop). The motif B4 D5 F#5- E5 D5 is sequenced down the Andalusian
//    cadence (Bm A G F#); the riff is saw+pulse power-chord stabs in 3+3+2;
//    the climb walks the bass up G A A# B while the saw lead runs up to B5;
//    the climax puts the motif in the low pulse and the saw on long notes
//    climbing against the falling bass. 16th hats, tom fills throughout.
//    A: Bm A G F# | Bm A G F#.  A': Bm A G F# | Em G F# F#.  Riff: F# G F# F#.
//    Climb: G A F#/A# Bm | G A F# F#.  Climax: Bm A G F# | Bm A G F#.
//
// Mix: lead v10-11, harmony v7-9, bass v10-11, kick/snare v10-12, hats v6-7,
// pad v6; the briefing sits lower (lead v8, arp v6-7, kick v8). Pads are
// single held lines (q8), one note at a time.

const theme: SongDef = {
  id: 'theme',
  title: 'Flyover Country (Main Theme)',
  bpm: 120,
  channels: {
    lead: {
      inst: 'pulse25',
      mml: `
; Intro (D C G A): fanfare in half notes
v10 q7 o5 d2 o4 a2 | o5 c2 o4 g2 | o4 b2 o5 d2 | v11 o5 e2. r4 |
$
; A (D C G A | D C G-A D): the hook
o4 a4. o5 d8 f+4 a4 | o5 g4. e8 c2 | o5 d4. o4 b8 o5 g2 | o5 e2. r4 |
o4 a4. o5 d8 f+4 a4 | o5 g4. a8 g4 e4 | o5 d4. o4 b8 o5 c+4 e4 | o5 d2. r4 |
; A' (D C G A | D C Em A)
o4 a4. o5 d8 f+4 a4 | o5 g4. e8 c2 | o5 d4. o4 b8 o5 g2 | o5 e2. r4 |
o4 a4. o5 d8 f+4 a4 | o5 g2 e4 c4 | o4 b4. o5 e8 g4 b4 | o5 a2 e4 c+4 |
; B (Bm G D A | Bm G Em A)
o4 b4. o5 c+8 d4 f+4 | o5 g2. f+8 e8 | o5 f+4. e8 d4 o4 a4 | o5 e2. d8 c+8 |
o4 b4. o5 c+8 d4 f+4 | o5 b2. a8 g8 | o5 g4. f+8 e4 o4 b4 | o5 c+2. r4 |
; A'' (D C G A | D C G A)
o4 a4. o5 d8 f+4 a4 | o5 g4. e8 c2 | o5 d4. o4 b8 o5 g2 | o5 e2. r4 |
o4 a4. o5 d8 f+4 a4 | o5 g4. a8 g4 e4 | o5 d4. o4 b8 o5 d4 g4 | o5 e2. r4 |
`,
    },
    harmony: {
      inst: 'pulse12',
      mml: `
; Intro: fanfare a third/fourth under the lead
v8 q7 o4 a2 f+2 | o4 g2 e2 | o4 g2 b2 | o5 c+2. r4 |
$
; A: NES broken-chord eighths
v7 q5 o4 [d8 a8 f+8 a8]2 | o4 [c8 g8 e8 g8]2 | o4 [d8 b8 g8 b8]2 | o4 [c+8 a8 e8 a8]2 |
o4 [d8 a8 f+8 a8]2 | o4 [c8 g8 e8 g8]2 | o4 d8 b8 g8 b8 c+8 a8 e8 a8 | o4 [d8 a8 f+8 a8]2 |
; A': same figures, the pad joins
o4 [d8 a8 f+8 a8]2 | o4 [c8 g8 e8 g8]2 | o4 [d8 b8 g8 b8]2 | o4 [c+8 a8 e8 a8]2 |
o4 [d8 a8 f+8 a8]2 | o4 [c8 g8 e8 g8]2 | o4 [e8 b8 g8 b8]2 | o4 [c+8 a8 e8 a8]2 |
; B: twin lead in sixths and thirds under the melody
v8 q7 o4 d4. e8 f+4 a4 | o4 b2. a8 g8 | o4 a4. g8 f+2 | o4 a2 e2 |
o4 d4. e8 f+4 a4 | o5 d2. f+8 e8 | o5 e4. d8 o4 b4 g4 | o4 a2. r4 |
; A'': the hook harmonised
o4 f+4. a8 o5 d4 f+4 | o5 e4. c8 o4 g2 | o4 b4. g8 b2 | o5 c+2. r4 |
o4 f+4. a8 o5 d4 f+4 | o5 e4. f8 e4 c4 | o4 b4. g8 b4 o5 d4 | o5 c+2. r4 |
`,
    },
    bass: {
      inst: 'tri',
      mml: `
; Intro: roots and fifths, walking up into the gallop
v11 q7 o2 d2 o1 a2 | o2 c2 o1 g2 | o1 g2 b2 | o1 a8 a8 a8 a8 a8 a8 o2 c+8 e8 |
$
; A: Western gallop (8th + two 16ths), root and fifth
q6
o2 d8 d16 d16 o1 a8 a16 a16 o2 d8 d16 d16 o1 a8 a16 a16 |
o2 c8 c16 c16 o1 g8 g16 g16 o2 c8 c16 c16 o1 g8 g16 g16 |
o1 g8 g16 g16 o2 d8 d16 d16 o1 g8 g16 g16 o2 d8 d16 d16 |
o1 a8 a16 a16 o2 e8 e16 e16 o1 a8 a16 a16 o2 e8 e16 e16 |
o2 d8 d16 d16 o1 a8 a16 a16 o2 d8 d16 d16 o1 a8 a16 a16 |
o2 c8 c16 c16 o1 g8 g16 g16 o2 c8 c16 c16 o1 g8 g16 g16 |
o1 g8 g16 g16 o2 d8 d16 d16 o1 a8 a16 a16 o2 e8 e16 e16 |
o2 d8 d16 d16 o1 a8 a16 a16 o2 d8 o1 a8 b8 o2 c+8 |
; A'
o2 d8 d16 d16 o1 a8 a16 a16 o2 d8 d16 d16 o1 a8 a16 a16 |
o2 c8 c16 c16 o1 g8 g16 g16 o2 c8 c16 c16 o1 g8 g16 g16 |
o1 g8 g16 g16 o2 d8 d16 d16 o1 g8 g16 g16 o2 d8 d16 d16 |
o1 a8 a16 a16 o2 e8 e16 e16 o1 a8 a16 a16 o2 e8 e16 e16 |
o2 d8 d16 d16 o1 a8 a16 a16 o2 d8 d16 d16 o1 a8 a16 a16 |
o2 c8 c16 c16 o1 g8 g16 g16 o2 c8 c16 c16 o1 g8 g16 g16 |
o2 e8 e16 e16 o1 b8 b16 b16 o2 e8 e16 e16 o1 b8 b16 b16 |
o1 a8 a16 a16 o2 e8 e16 e16 o2 e8 c+8 o1 b8 a+8 |
; B: driving octave eighths
q7
[o1 b8 o2 b8]4 | [o1 g8 o2 g8]4 | [o2 d8 a8]4 | [o1 a8 o2 a8]4 |
[o1 b8 o2 b8]4 | [o1 g8 o2 g8]4 | [o1 e8 o2 e8]4 | [o1 a8 o2 a8]3 o2 e8 c+8 |
; A''
q6
o2 d8 d16 d16 o1 a8 a16 a16 o2 d8 d16 d16 o1 a8 a16 a16 |
o2 c8 c16 c16 o1 g8 g16 g16 o2 c8 c16 c16 o1 g8 g16 g16 |
o1 g8 g16 g16 o2 d8 d16 d16 o1 g8 g16 g16 o2 d8 d16 d16 |
o1 a8 a16 a16 o2 e8 e16 e16 o1 a8 a16 a16 o2 e8 e16 e16 |
o2 d8 d16 d16 o1 a8 a16 a16 o2 d8 d16 d16 o1 a8 a16 a16 |
o2 c8 c16 c16 o1 g8 g16 g16 o2 c8 c16 c16 o1 g8 g16 g16 |
o1 g8 g16 g16 o2 d8 d16 d16 o1 g8 g16 g16 o2 d8 d16 d16 |
o1 a8 a16 a16 o2 e8 e16 e16 o2 e8 o1 a8 b8 o2 c+8 |
`,
    },
    drums: {
      inst: 'noise',
      mml: `
; Intro: crash and toms, then a snare roll that swells into the loop
v10 c2 v9 t4 t4 | v9 k4 t4 t8 t8 t4 | v10 c2 v9 m4 t4 |
v7 s8 s8 v8 s8 s8 v9 s16 s16 s16 s16 v10 s16 s16 s16 s16 |
$
; A: straight eighth hats
v10 c8 v7 h8 v10 s8 v7 h8 v11 k8 v7 h8 v10 s8 v7 h8 |
[v11 k8 v7 h8 v10 s8 v7 h8 v11 k8 v7 h8 v10 s8 v7 h8 |]6
v11 k8 v7 h8 v10 s8 v7 h8 v9 s16 s16 v10 s16 s16 m16 m16 t16 t16 |
; A': galloping hats
v10 c8 v7 h16 h16 v10 s8 v7 h16 h16 v11 k8 v7 h16 h16 v10 s8 v7 h16 h16 |
[v11 k8 v7 h16 h16 v10 s8 v7 h16 h16 v11 k8 v7 h16 h16 v10 s8 v7 h16 h16 |]6
v11 k8 v7 h16 h16 v10 s8 v7 h16 h16 v9 s16 s16 v10 s16 s16 m16 m16 t16 t16 |
; B: driving rock beat
v10 c8 v7 h8 v10 s8 v7 h8 v11 k8 k8 v10 s8 v7 o8 |
[v11 k8 v7 h8 v10 s8 v7 h8 v11 k8 k8 v10 s8 v7 o8 |]6
v11 k8 v7 h8 v10 s8 v7 h8 v10 s16 s16 s16 s16 v10 m8 t8 |
; A'': galloping hats
v10 c8 v7 h16 h16 v10 s8 v7 h16 h16 v11 k8 v7 h16 h16 v10 s8 v7 h16 h16 |
[v11 k8 v7 h16 h16 v10 s8 v7 h16 h16 v11 k8 v7 h16 h16 v10 s8 v7 h16 h16 |]6
v11 k8 v7 h16 h16 v10 s8 v7 h16 h16 v9 s16 s16 v10 s16 s16 m16 m16 t16 t16 |
`,
    },
    pad: {
      inst: 'pad',
      mml: `
; Intro: a slow descending string line F#-E-D-C#
v6 q8 o4 f+1 | o4 e1 | o4 d1 | o4 c+1 |
$
; A: tacet, chip only
r1 | r1 | r1 | r1 | r1 | r1 | r1 | r1 |
; A': the same line, then rising D-E-G-A into the bridge
o4 f+1 | o4 e1 | o4 d1 | o4 c+1 | o4 d1 | o4 e1 | o4 g1 | o4 a1 |
; B: keeps climbing under the melody
o4 f+1 | o4 g1 | o4 a1 | ^1 | o4 b1 | ^1 | ^1 | o5 c+1 |
; A''
o5 d1 | o5 c1 | o4 b1 | o5 c+1 | o5 d1 | o5 c1 | o4 b1 | o4 a1 |
`,
    },
  },
};

const briefing: SongDef = {
  id: 'briefing',
  title: 'Briefing Room',
  bpm: 92,
  channels: {
    arp: {
      inst: 'pulse12',
      mml: `
; A (Am Fmaj7 Dm7 Esus4-E | Am Fmaj7 Dm7 E): 3+3+2 arpeggio, voices move by half steps
v7 q4
[o3 a8 o4 e8 o5 c8]2 o3 a8 o4 e8 | [o3 a8 o4 e8 o5 c8]2 o3 a8 o4 e8 |
[o3 a8 o4 f8 o5 c8]2 o3 a8 o4 f8 | [o3 a8 o4 e8 b8]2 o3 g+8 o4 e8 |
[o3 a8 o4 e8 o5 c8]2 o3 a8 o4 e8 | [o3 a8 o4 e8 o5 c8]2 o3 a8 o4 e8 |
[o3 a8 o4 f8 o5 c8]2 o3 a8 o4 f8 | [o3 g+8 o4 e8 b8]2 o3 g+8 o4 e8 |
; B (F G C Am | Dm Bb E7 E)
[o3 a8 o4 f8 o5 c8]2 o3 a8 o4 f8 | [o3 b8 o4 g8 o5 d8]2 o3 b8 o4 g8 |
[o4 c8 g8 o5 e8]2 o4 c8 g8 | [o4 c8 a8 o5 e8]2 o4 c8 a8 |
[o4 d8 a8 o5 f8]2 o4 d8 a8 | [o4 d8 b-8 o5 f8]2 o4 d8 b-8 |
[o4 d8 g+8 o5 e8]2 o4 d8 g+8 | [o3 g+8 o4 e8 b8]2 o3 g+8 o4 e8 |
; A' (the lead appears; the arpeggio steps back)
v6
[o3 a8 o4 e8 o5 c8]2 o3 a8 o4 e8 | [o3 a8 o4 e8 o5 c8]2 o3 a8 o4 e8 |
[o3 a8 o4 f8 o5 c8]2 o3 a8 o4 f8 | [o3 a8 o4 e8 b8]2 o3 g+8 o4 e8 |
[o3 a8 o4 e8 o5 c8]2 o3 a8 o4 e8 | [o3 a8 o4 e8 o5 c8]2 o3 a8 o4 e8 |
[o3 a8 o4 f8 o5 c8]2 o3 a8 o4 f8 | [o3 g+8 o4 e8 b8]2 o3 g+8 o4 e8 |
; B'
[o3 a8 o4 f8 o5 c8]2 o3 a8 o4 f8 | [o3 b8 o4 g8 o5 d8]2 o3 b8 o4 g8 |
[o4 c8 g8 o5 e8]2 o4 c8 g8 | [o4 c8 a8 o5 e8]2 o4 c8 a8 |
[o4 d8 a8 o5 f8]2 o4 d8 a8 | [o4 d8 b-8 o5 f8]2 o4 d8 b-8 |
[o4 d8 g+8 o5 e8]2 o4 d8 g+8 | [o3 g+8 o4 e8 b8]2 o3 g+8 o4 e8 |
`,
    },
    lead: {
      inst: 'pulse50',
      mml: `
; A and B: tacet, the room is reading
[r1 |]16
; A': a few quiet words
v8 q7
r2 o5 e4 c4 | o4 a2. r4 | r2 o5 d4 c4 | o4 b2 g+2 |
r2 o5 e4 a4 | o5 a4. g8 e2 | o5 f2 d2 | o4 b1 |
; B': a slow rising line, then home to the dominant
o5 c2. r4 | o5 d2. r4 | o5 e2 g2 | o5 e1 |
o5 f2. e8 d8 | o5 d2 f2 | o5 e2 d2 | o4 b1 |
`,
    },
    bass: {
      inst: 'tri',
      mml: `
; soft pulsing eighths on the root, the same all the way through
v10 q5
; A
o2 [a8]8 | o2 [f8]8 | o2 [d8]8 | o2 [e8]8 | o2 [a8]8 | o2 [f8]8 | o2 [d8]8 | o2 [e8]8 |
; B
o2 [f8]8 | o2 [g8]8 | o3 [c8]8 | o2 [a8]8 | o2 [d8]8 | o1 [b-8]8 | o2 [e8]8 | o2 [e8]8 |
; A'
o2 [a8]8 | o2 [f8]8 | o2 [d8]8 | o2 [e8]8 | o2 [a8]8 | o2 [f8]8 | o2 [d8]8 | o2 [e8]8 |
; B'
o2 [f8]8 | o2 [g8]8 | o3 [c8]8 | o2 [a8]8 | o2 [d8]8 | o1 [b-8]8 | o2 [e8]8 | o2 [e8]8 |
`,
    },
    pad: {
      inst: 'pad',
      mml: `
; A: A4 held over Am-Fmaj7-Dm7, sighing to G#4 as the sus4 resolves; then C5 to B4
v6 q8
o4 a1 | ^1 | ^1 | ^2 g+2 | o5 c1 | ^1 | ^1 | o4 b1 |
; B: rises A4 B4 C5, then a D5 pedal through Dm-Bb-E7
o4 a1 | o4 b1 | o5 c1 | ^1 | o5 d1 | ^1 | ^1 | o4 b1 |
; A'
o4 a1 | ^1 | ^1 | ^2 g+2 | o5 c1 | ^1 | ^1 | o4 b1 |
; B'
o4 a1 | o4 b1 | o5 c1 | ^1 | o5 d1 | ^1 | ^1 | o4 b1 |
`,
    },
    drums: {
      inst: 'noise',
      mml: `
; A: silence
[r1 |]8
; B: soft offbeat hats, a clock ticking
[r8 v6 h8 r8 h8 r8 h8 r8 h8 |]8
; A': a soft kick on the downbeat
[v8 k8 v6 h8 r8 h8 r8 h8 r8 h8 |]8
; B': kick on 1 and 3, an open hat breathes out before the loop
[v8 k8 v6 h8 r8 h8 v8 k8 v6 h8 r8 h8 |]7
v8 k8 v6 h8 r8 h8 v8 k8 v6 h8 o4 |
`,
    },
  },
};

const sortie: SongDef = {
  id: 'sortie',
  title: 'Night Sortie',
  bpm: 144,
  channels: {
    lead: {
      inst: 'pulse25',
      mml: `
; Intro (Em): drums and bass only
r1 | r1 |
$
; Riff (Em C D B)
v10 q6 o4 e8 g8 b8 o5 e4 d8 o4 b4 | o4 c8 e8 g8 o5 c4 o4 b8 g4 |
o4 d8 f+8 a8 o5 d4 c8 o4 a4 | o4 f+8 b8 o5 d+8 f+4 e8 d+4 |
; A (Em C D Bm | Em C Am B): the lead calls, the saw answers in the gaps
v11 q7 o4 b8 o5 e8 f+8 g8 ^2 | o5 e2 r2 | o4 a8 o5 d8 e8 f+8 ^2 | o5 d2 r2 |
o4 b8 o5 e8 f+8 g8 ^4 a8 g8 | o5 e2 r2 | o4 a8 o5 c8 e8 a8 ^4 g8 f+8 | o5 f+2. d+4 |
; A' (Em C D Bm | C D Am B): the second half climbs toward the chorus
o4 b8 o5 e8 f+8 g8 ^2 | o5 e2 r2 | o4 a8 o5 d8 e8 f+8 ^2 | o5 d2 r2 |
o5 e4. d8 e4 g4 | o5 f+4. e8 f+4 a4 | o5 a4. g8 a4 b4 | o5 b2. r4 |
; Chorus (C D G Em | C D B B): soaring sixths
o5 e4 g2. | o5 d4 f+2. | o5 d4 b2. | o5 g4. f+8 e2 |
o5 e4 g2. | o5 d4 a2. | o5 f+4 b2. | o5 a4 f+4 d+4 o4 b4 |
; Chorus' (C D G Em | C D Em Em): lands home
o5 e4 g2. | o5 d4 f+2. | o5 d4 b2. | o5 g4. f+8 e2 |
o5 e4 g2. | o5 d4 a2. | o5 g4 b2. | o5 e1 |
; Bridge (Am Em C B | Am Em C B): the saw sings first, the lead answers
r1 | r1 | r1 | r1 |
v10 o5 a2. g8 e8 | o5 g2. e4 | o5 e2. d8 c8 | o4 b2. d+4 |
`,
    },
    harmony: {
      inst: 'saw',
      mml: `
; Intro
r1 | r1 |
$
; Riff: doubled an octave below
v8 q6 o3 e8 g8 b8 o4 e4 d8 o3 b4 | o3 c8 e8 g8 o4 c4 o3 b8 g4 |
o3 d8 f+8 a8 o4 d4 c8 o3 a4 | o3 f+8 b8 o4 d+8 f+4 e8 d+4 |
; A: brass answers, echoing the lead's leap-and-climb
q7 r1 | r2 o4 g8 o5 c8 d8 e8 | r1 | r2 o4 f+8 b8 o5 c+8 d8 |
r1 | r2 o4 g8 o5 c8 d8 e8 | o4 e1 | o4 d+1 |
; A': answers, then thirds and sixths under the climb
r1 | r2 o4 g8 o5 c8 d8 e8 | r1 | r2 o4 f+8 b8 o5 c+8 d8 |
o5 c4. o4 b8 o5 c4 e4 | o5 d4. c8 d4 f+4 | o5 c4. o4 b8 o5 c4 d4 | o5 d+2. r4 |
; Chorus: brass counter-line in half notes
o5 c2 e2 | o4 a2 o5 d2 | o4 b2 o5 g2 | o5 e2 o4 b2 |
o5 c2 e2 | o4 a2 o5 f+2 | o4 b2 o5 d+2 | o5 f+2 o4 b2 |
; Chorus': twin melody a third below
o5 c4 e2. | o4 a4 o5 d2. | o4 b4 o5 g2. | o5 e4. d8 o4 b2 |
o5 c4 e2. | o4 a4 o5 f+2. | o5 e4 g2. | o4 g2 b2 |
; Bridge: saw solo, then held notes under the lead's answer
v9 o5 c2. o4 b8 a8 | o4 b1 | o4 g2. a8 b8 | o4 f+2 d+2 |
v8 o5 c1 | o4 b1 | o4 g1 | o4 f+1 |
`,
    },
    bass: {
      inst: 'tri',
      mml: `
; Intro (Em): octave eighths
v11 q6 [o1 e8 o2 e8]4 | [o1 e8 o2 e8]4 |
$
; Riff (Em C D B)
[o1 e8 o2 e8]4 | [o2 c8 o3 c8]4 | [o2 d8 a8]4 | [o1 b8 o2 b8]4 |
; A: gallop (low 8th, two high 16ths)
[o1 e8 o2 e16 e16]4 | [o2 c8 o3 c16 c16]4 | [o2 d8 a16 a16]4 | [o1 b8 o2 b16 b16]4 |
[o1 e8 o2 e16 e16]4 | [o2 c8 o3 c16 c16]4 | [o1 a8 o2 a16 a16]4 | [o1 b8 o2 b16 b16]4 |
; A'
[o1 e8 o2 e16 e16]4 | [o2 c8 o3 c16 c16]4 | [o2 d8 a16 a16]4 | [o1 b8 o2 b16 b16]4 |
[o2 c8 o3 c16 c16]4 | [o2 d8 a16 a16]4 | [o1 a8 o2 a16 a16]4 | [o1 b8 o2 b16 b16]3 o2 b8 o1 b8 |
; Chorus: straight octaves
[o2 c8 o3 c8]4 | [o2 d8 a8]4 | [o1 g8 o2 g8]4 | [o1 e8 o2 e8]4 |
[o2 c8 o3 c8]4 | [o2 d8 a8]4 | [o1 b8 o2 b8]4 | [o1 b8 o2 b8]4 |
; Chorus'
[o2 c8 o3 c8]4 | [o2 d8 a8]4 | [o1 g8 o2 g8]4 | [o1 e8 o2 e8]4 |
[o2 c8 o3 c8]4 | [o2 d8 a8]4 | [o1 e8 o2 e8]4 | o1 e8 o2 e8 o1 e8 o2 e8 o1 e8 f+8 g8 g+8 |
; Bridge: half notes, then octaves building back into the riff
q7 o1 a2 o2 e2 | o2 e2 o1 b2 | o2 c2 o1 g2 | o1 b2 f+2 |
q6 [o1 a8 o2 a8]4 | [o1 e8 o2 e8]4 | [o2 c8 o3 c8]4 | [o1 b8 o2 b8]2 o2 b8 f+8 d+8 o1 b8 |
`,
    },
    drums: {
      inst: 'noise',
      mml: `
; Intro: kick and toms, then a snare-and-tom build
v11 k4 v10 t8 t8 v11 k4 v10 m8 m8 |
v9 s16 s16 s16 s16 v10 s16 s16 s16 s16 m16 m16 m16 m16 v11 t16 t16 t16 t16 |
$
; Riff: rock beat
v12 c8 v7 h8 v11 s8 v7 h8 v11 k8 k8 s8 v7 h8 |
[v11 k8 v7 h8 v11 s8 v7 h8 v11 k8 k8 s8 v7 h8 |]2
v11 k8 v7 h8 v11 s8 v7 h8 v11 k8 v10 s16 s16 v11 s8 v7 o8 |
; A
v12 c8 v7 h8 v11 s8 v7 h8 v11 k8 k8 s8 v7 h8 |
[v11 k8 v7 h8 v11 s8 v7 h8 v11 k8 k8 s8 v7 h8 |]6
v11 k8 v7 h8 v11 s8 v7 h8 v10 s16 s16 s16 s16 m8 t8 |
; A'
v12 c8 v7 h8 v11 s8 v7 h8 v11 k8 k8 s8 v7 h8 |
[v11 k8 v7 h8 v11 s8 v7 h8 v11 k8 k8 s8 v7 h8 |]6
v11 k8 v10 s16 s16 v11 s8 v10 s16 s16 m16 m16 m16 m16 v11 t16 t16 t16 t16 |
; Chorus: open hat on the last eighth
v12 c8 v7 h8 v11 s8 v7 h8 v11 k8 v7 h8 v11 s8 v7 o8 |
[v11 k8 v7 h8 v11 s8 v7 h8 v11 k8 v7 h8 v11 s8 v7 o8 |]6
v11 k8 v7 h8 v11 s8 v7 h8 v10 s16 s16 s16 s16 m8 t8 |
; Chorus': ends on a crash and a breath
v12 c8 v7 h8 v11 s8 v7 h8 v11 k8 v7 h8 v11 s8 v7 o8 |
[v11 k8 v7 h8 v11 s8 v7 h8 v11 k8 v7 h8 v11 s8 v7 o8 |]6
v11 k8 v7 h8 v11 s8 v7 h8 v12 c4 r4 |
; Bridge: half time (snare on 3), then the backbeat returns and builds
v11 c8 v7 h8 h8 h8 v11 s8 v7 h8 h8 h8 |
[v11 k8 v7 h8 h8 h8 v11 s8 v7 h8 h8 h8 |]3
[v11 k8 v7 h8 v11 s8 v7 h8 v11 k8 k8 s8 v7 h8 |]3
v10 s16 s16 s16 s16 v11 s16 s16 s16 s16 v10 m16 m16 m16 m16 v11 t16 t16 t16 t16 |
`,
    },
    pad: {
      inst: 'pad',
      mml: `
; Intro, Riff and A: tacet
r1 | r1 |
$
[r1 |]12
; A': strings creep in and rise toward the chorus
v6 q8 o4 g1 | ^1 | o4 f+1 | ^1 | o4 g1 | o4 a1 | ^1 | o4 b1 |
; Chorus
o4 e1 | o4 f+1 | o4 g1 | ^1 | o4 e1 | o4 f+1 | ^1 | o4 d+1 |
; Chorus'
o4 e1 | o4 f+1 | o4 g1 | ^1 | o4 e1 | o4 f+1 | o4 g1 | ^1 |
; Bridge: an E pedal sighing to D#
o4 e1 | ^1 | ^1 | o4 d+1 | o4 e1 | ^1 | ^1 | o4 d+1 |
`,
    },
  },
};

const glovesoff: SongDef = {
  id: 'glovesoff',
  title: 'Gloves Off',
  bpm: 160,
  channels: {
    lead: {
      inst: 'saw',
      mml: `
; Intro (F#): drums, then a stab pickup
r1 | v11 q4 o5 c+4. c+4. o4 a+4 |
$
; A (Bm A G F# | Bm A G F#): the motif, sequenced down the Andalusian cadence
q7 o4 b4 o5 d8 f+8 ^4 e8 d8 | o4 a4 o5 c+8 e8 ^4 d8 c+8 |
o4 g4 b8 o5 d8 ^4 c+8 o4 b8 | o4 a+4 o5 c+8 e8 ^4 d8 c+8 |
o5 d4 f+8 b8 ^4 a8 f+8 | o5 c+4 e8 a8 ^4 g8 e8 |
o4 b4 o5 d8 g8 ^4 f+8 e8 | o5 f+8 g8 f+8 e8 c+8 d8 c+8 o4 a+8 |
; A' (Bm A G F# | Em G F# F#)
o4 b4 o5 d8 f+8 ^4 e8 d8 | o4 a4 o5 c+8 e8 ^4 d8 c+8 |
o4 g4 b8 o5 d8 ^4 c+8 o4 b8 | o4 a+4 o5 c+8 e8 ^4 d8 c+8 |
o5 e4 g8 b8 ^4 a8 g8 | o5 d4 g8 b8 ^4 a8 g8 |
o5 c+4 f+8 a+8 ^4 g8 f+8 | o5 f+2 e4 c+4 |
; Riff (F# G F# F#): power-chord stabs in 3+3+2, the fifth on top
q4 o5 c+4. c+4. c+4 | o5 d4. d4. d4 | o5 c+4. c+4. c+4 | o5 c+4. c+4. c+4 |
; Climb (G A F#/A# Bm | G A F# F#)
q7 o5 d2 ^8 e8 f+8 g8 | o5 e2 ^8 f+8 g8 a8 | o5 f+2 ^8 g8 f+8 a+8 | o5 b2. r4 |
o5 b4. a8 g4 d4 | o5 c+4. d8 e4 a4 | o5 a+4. b8 a+4 f+4 | o5 c+4 e4. f+8 g8 a+8 |
; Climax (Bm A G F# | Bm A G F#): long notes climbing against the falling bass
o5 b2 f+2 | o5 e2 a2 | o5 g2 b2 | o5 a+2 f+2 |
o5 b1 | o5 a2 e2 | o5 g2 b2 | o5 a+2. r4 |
; Riff' (F# G F# F#)
q4 o5 c+4. c+4. c+4 | o5 d4. d4. d4 | o5 c+4. c+4. c+4 | o5 c+4. c+4. o4 a+4 |
`,
    },
    harmony: {
      inst: 'pulse25',
      mml: `
; Intro: stab pickup, the root under the lead's fifth
r1 | v9 q4 o4 f+4. f+4. f+4 |
$
; A: eighth-note broken chords
v7 q5 o4 [d8 f+8 b8 f+8]2 | o4 [c+8 e8 a8 e8]2 | o4 [d8 g8 b8 g8]2 | o4 [c+8 f+8 a+8 f+8]2 |
o4 [d8 f+8 b8 f+8]2 | o4 [c+8 e8 a8 e8]2 | o4 [d8 g8 b8 g8]2 | o4 [c+8 f+8 a+8 f+8]2 |
; A': twin line under the lead
v8 q7 o4 f+4 b8 o5 d8 ^4 c+8 o4 b8 | o4 e4 a8 o5 c+8 ^4 o4 b8 a8 |
o4 d4 g8 b8 ^4 a8 g8 | o4 f+4 a+8 o5 c+8 ^4 o4 b8 a+8 |
o4 b4 o5 e8 g8 ^4 f+8 e8 | o4 b4 o5 d8 g8 ^4 f+8 d8 |
o4 a+4 o5 c+8 f+8 ^4 e8 c+8 | o5 c+2 o4 a+4 f+4 |
; Riff: stabs, the root under the lead's fifth
v9 q4 o4 f+4. f+4. f+4 | o4 g4. g4. g4 | o4 f+4. f+4. f+4 | o4 f+4. f+4. f+4 |
; Climb: held notes, then sixths and thirds under the lead
v8 q7 o4 b1 | o5 c+1 | o5 c+1 | o5 d1 |
o5 d4. c+8 o4 b4 g4 | o4 a4. b8 o5 c+4 e4 | o5 f+4. g8 f+4 c+4 | o4 a+4 o5 c+2 e8 f+8 |
; Climax: the motif moves down to the low pulse
o3 b4 o4 d8 f+8 ^4 e8 d8 | o3 a4 o4 c+8 e8 ^4 d8 c+8 |
o3 g4 b8 o4 d8 ^4 c+8 o3 b8 | o3 a+4 o4 c+8 e8 ^4 d8 c+8 |
o3 b4 o4 d8 f+8 ^4 e8 d8 | o3 a4 o4 c+8 e8 ^4 d8 c+8 |
o3 g4 b8 o4 d8 ^4 c+8 o3 b8 | o3 a+4 o4 c+8 e8 ^4 d8 c+8 |
; Riff'
v9 q4 o4 f+4. f+4. f+4 | o4 g4. g4. g4 | o4 f+4. f+4. f+4 | o4 f+4. f+4. f+4 |
`,
    },
    bass: {
      inst: 'tri',
      mml: `
; Intro (F#): octave eighths, leading-tone pickup
v11 q6 [o1 f+8 o2 f+8]4 | [o1 f+8 o2 f+8]3 o2 c+8 o1 a+8 |
$
; A (Bm A G F# x2): octave eighths down the Andalusian cadence
[o1 b8 o2 b8]4 | [o1 a8 o2 a8]4 | [o1 g8 o2 g8]4 | [o1 f+8 o2 f+8]4 |
[o1 b8 o2 b8]4 | [o1 a8 o2 a8]4 | [o1 g8 o2 g8]4 | [o1 f+8 o2 f+8]3 o2 c+8 o1 a+8 |
; A' (Bm A G F# | Em G F# F#)
[o1 b8 o2 b8]4 | [o1 a8 o2 a8]4 | [o1 g8 o2 g8]4 | [o1 f+8 o2 f+8]4 |
[o1 e8 o2 e8]4 | [o1 g8 o2 g8]4 | [o1 f+8 o2 f+8]4 | [o1 f+8 o2 f+8]4 |
; Riff: locks to the 3+3+2 stabs
[o1 f+8 o2 f+8 f+8]2 o1 f+8 o2 f+8 | [o1 g8 o2 g8 g8]2 o1 g8 o2 g8 |
[o1 f+8 o2 f+8 f+8]2 o1 f+8 o2 f+8 | [o1 f+8 o2 f+8 f+8]2 o1 f+8 o2 f+8 |
; Climb: the bass walks up G A A# B
[o1 g8 o2 g8]4 | [o1 a8 o2 a8]4 | [o1 a+8 o2 a+8]4 | [o1 b8 o2 b8]4 |
[o1 g8 o2 g8]4 | [o1 a8 o2 a8]4 | [o1 f+8 o2 f+8]4 | [o1 f+8 o2 f+8]2 o2 e8 c+8 o1 a+8 o2 c+8 |
; Climax (Bm A G F# x2)
[o1 b8 o2 b8]4 | [o1 a8 o2 a8]4 | [o1 g8 o2 g8]4 | [o1 f+8 o2 f+8]4 |
[o1 b8 o2 b8]4 | [o1 a8 o2 a8]4 | [o1 g8 o2 g8]4 | [o1 f+8 o2 f+8]4 |
; Riff': leading tone back to Bm
[o1 f+8 o2 f+8 f+8]2 o1 f+8 o2 f+8 | [o1 g8 o2 g8 g8]2 o1 g8 o2 g8 |
[o1 f+8 o2 f+8 f+8]2 o1 f+8 o2 f+8 | [o1 f+8 o2 f+8 f+8]2 o2 c+8 o1 a+8 |
`,
    },
    drums: {
      inst: 'noise',
      mml: `
; Intro: kick and toms, then crash stabs in 3+3+2
v11 k8 v10 t8 t8 v11 k8 v10 t8 t8 v11 k8 v10 m8 |
v12 c4. c4. v10 s16 s16 s16 s16 |
$
; A: double-time hats, kick on 1 and 3 (and the "and" of 3), snare on 2 and 4
v12 c16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 |
[v11 k16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 |]6
v11 k16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v10 m16 m16 m16 m16 v11 t16 t16 t16 t16 |
; A'
v12 c16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 |
[v11 k16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 |]6
v11 k16 v6 h16 h16 h16 v11 s16 v10 m16 m16 t16 v11 s16 s16 s16 s16 t16 t16 t16 t16 |
; Riff: hits on the 3+3+2 stabs
v12 c8 v6 h8 h8 v11 s8 v6 h8 h8 v11 s8 v6 h8 |
[v11 k8 v6 h8 h8 v11 s8 v6 h8 h8 v11 s8 v6 h8 |]2
v11 k8 v6 h8 h8 v11 s8 v10 m16 m16 m16 m16 v11 t16 t16 t16 t16 |
; Climb: galloping kicks
v12 c16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 |
[v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 |]6
v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 v10 m16 m16 m16 m16 v11 t16 t16 t16 t16 |
; Climax: crash at the top of each half
v12 c16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 |
[v11 k16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 |]2
v11 k16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v10 m16 m16 m16 m16 v11 t16 t16 t16 t16 |
v12 c16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 |
[v11 k16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v11 k16 v6 h16 v11 k16 v6 h16 v11 s16 v6 h16 h16 h16 |]2
v11 k16 v6 h16 h16 h16 v11 s16 v6 h16 h16 h16 v10 m16 m16 m16 m16 v11 t16 t16 t16 t16 |
; Riff'
v12 c8 v6 h8 h8 v11 s8 v6 h8 h8 v11 s8 v6 h8 |
[v11 k8 v6 h8 h8 v11 s8 v6 h8 h8 v11 s8 v6 h8 |]2
v11 k8 v6 h8 h8 v11 s8 v10 s16 s16 m16 m16 v11 t16 t16 t16 t16 |
`,
    },
    pad: {
      inst: 'pad',
      mml: `
; Intro: the dominant's third, swelling
v6 q8 o4 c+1 | ^1 |
$
; A: inner voice falling with the cadence, F# E D C#
o4 f+1 | o4 e1 | o4 d1 | o4 c+1 | o4 f+1 | o4 e1 | o4 d1 | o4 c+1 |
; A'
o4 f+1 | o4 e1 | o4 d1 | o4 c+1 | o4 e1 | o4 d1 | o4 c+1 | o3 a+1 |
; Riff: tacet
[r1 |]4
; Climb: rising with the bass
o4 d1 | o4 e1 | o4 f+1 | ^1 | o4 g1 | o4 a1 | o4 a+1 | ^1 |
; Climax: an upper line in tenths with the falling bass, above the motif
o5 d1 | o5 c+1 | o4 b1 | o4 a+1 | o5 d1 | o5 c+1 | o4 b1 | o4 a+1 |
; Riff': tacet
[r1 |]4
`,
    },
  },
};

export const SONGS: SongDef[] = [theme, briefing, sortie, glovesoff];
