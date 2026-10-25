#!/usr/bin/env python3
"""The game's synthesized sounds: the ones the CC0 packs in assets/sfx have no take on.

    python3 tools/sfx_synth.py            # writes assets/sfx/<name>.wav

Mono, 44.1 kHz, 16-bit. Seeded, so a run gives the same files. Standard library only.
"""
import math
import random
import struct
import wave
from pathlib import Path

RATE = 44100
OUT = Path(__file__).resolve().parent.parent / "assets" / "sfx"


def write(name, samples, gain=0.9):
    peak = max(1e-9, max(abs(s) for s in samples))
    k = gain / peak
    with wave.open(str(OUT / f"{name}.wav"), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(b"".join(struct.pack("<h", int(max(-1.0, min(1.0, s * k)) * 32767)) for s in samples))


class BandPass:
    """A resonant band-pass (RBJ biquad), its centre settable every sample."""

    def __init__(self, q):
        self.q = q
        self.x1 = self.x2 = self.y1 = self.y2 = 0.0

    def run(self, x, f):
        w = 2 * math.pi * f / RATE
        a = math.sin(w) / (2 * self.q)
        b0, b2 = a, -a
        a0, a1, a2 = 1 + a, -2 * math.cos(w), 1 - a
        y = (b0 * x + b2 * self.x2 - a1 * self.y1 - a2 * self.y2) / a0
        self.x2, self.x1, self.y2, self.y1 = self.x1, x, self.y1, y
        return y


def env(t, attack, decay):
    return (t / attack if t < attack else math.exp(-(t - attack) / decay))


def swing(seconds, f_from, f_to, q, rng, body=0.0):
    """A blade through the air: noise through a band sweeping down, swelling and dying away."""
    n = int(seconds * RATE)
    bp, low = BandPass(q), BandPass(0.7)
    out = []
    for i in range(n):
        t = i / RATE
        u = t / seconds
        f = f_from * (f_to / f_from) ** u
        x = rng.uniform(-1, 1)
        s = bp.run(x, f) * math.sin(math.pi * min(1.0, u * 1.25)) ** 1.5
        if body:
            s += body * low.run(x, f * 0.35) * math.sin(math.pi * u)
        out.append(s)
    return out


def bow(freq, rng):
    """An arrow loosed: the string snapping home, low and quickly damped (Karplus-Strong with a
    soft pluck and a dull loop), the limbs' thump, and the arrow's short whoosh away. No long
    ring and no bright click: it is heard every shot."""
    n, period = int(0.34 * RATE), int(RATE / freq)
    # A soft pluck: smoothed noise, so the string starts mellow rather than buzzing.
    raw = [rng.uniform(-1, 1) for _ in range(period)]
    buf = [(raw[i - 1] + 2 * raw[i] + raw[(i + 1) % period]) / 4 for i in range(period)]
    bp, lp = BandPass(1.1), 0.0
    out = []
    for i in range(n):
        t = i / RATE
        s = buf[i % period]
        buf[i % period] = 0.982 * (0.42 * s + 0.58 * buf[(i + 1) % period])
        string = s * math.exp(-t / 0.07)
        thump = math.sin(2 * math.pi * (78 * t - 40 * t * t)) * math.exp(-t / 0.03)
        u = t / 0.2
        whoosh = bp.run(rng.uniform(-1, 1), 1500 * (0.4 ** min(u, 1.0))) * (math.sin(math.pi * min(u, 1.0)) ** 2 if u < 1 else 0.0)
        x = 0.9 * string + 0.8 * thump + 0.35 * whoosh
        lp += 0.35 * (x - lp)  # take the edge off
        out.append(lp)
    return out


def glug(rng):
    """A drink: three bubbles rising in pitch, a little wet noise between."""
    out = [0.0] * int(0.55 * RATE)
    for k, (at, f0, f1) in enumerate([(0.02, 170, 300), (0.19, 190, 330), (0.35, 160, 280)]):
        start, length = int(at * RATE), int(0.09 * RATE)
        phase = 0.0
        for i in range(length):
            u = i / length
            f = f0 + (f1 - f0) * u
            phase += 2 * math.pi * f / RATE
            out[start + i] += math.sin(phase) * math.sin(math.pi * u) ** 2 * (1.0 - 0.15 * k)
    bp = BandPass(2.0)
    for i in range(len(out)):
        out[i] += 0.12 * bp.run(rng.uniform(-1, 1), 900) * math.exp(-i / (RATE * 0.25))
    return out


def frost(rng):
    """A frost bolt loosed: a cold chime cluster over a breath of hiss."""
    n = int(0.7 * RATE)
    partials = [(1760, 1.0, 0.18), (2349, 0.7, 0.14), (3136, 0.5, 0.1), (4186, 0.35, 0.07)]
    hp = BandPass(0.9)
    out = []
    for i in range(n):
        t = i / RATE
        s = sum(a * math.sin(2 * math.pi * f * t * (1 + 0.002 * math.sin(2 * math.pi * 6 * t))) * env(t, 0.004, d) for f, a, d in partials)
        s += 0.5 * hp.run(rng.uniform(-1, 1), 5200) * env(t, 0.01, 0.12)
        out.append(s)
    return out


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    rng = random.Random(7)
    write("swing_light", swing(0.17, 2600, 650, 1.6, rng), 0.8)
    write("swing_heavy", swing(0.3, 1500, 260, 1.2, rng, body=0.6), 0.9)
    bow_rng = random.Random(11)
    for k, f in enumerate([132, 124, 141]):
        write(f"bow_{k}", bow(f, bow_rng), 0.8)
    # The draws the old bowstring took here, so the sounds after it keep their noise.
    for _ in range(int(RATE / 98) + int(0.5 * RATE)):
        rng.uniform(-1, 1)
    write("glug", glug(rng), 0.7)
    write("cast_frost", frost(rng), 0.7)
    print("wrote", ", ".join(sorted(p.name for p in OUT.glob("*.wav"))))


if __name__ == "__main__":
    main()
