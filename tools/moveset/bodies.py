"""Whole-body skeletons (falls, getting up, kneeling, rolls, jumps, the parry), for Pixellab's
animate_with_skeleton_v3, with the weapon track the game draws the held weapon along.

    python3 -I bodies.py <out_dir> [clip ...]          the skeletons and tracks
    python3 -I bodies.py --preview <out_dir> <aligned_dir> <skeltool> [clip ...]
                                                        each frame's skeleton and weapon drawn
                                                        over <aligned_dir>/<clip>/NNN.png

stances.py poses an upright body with its arms; these clips leave the feet (lying, kneeling,
tumbling), so a pose here is posed on screen, bone by bone, in the reference frame's fractions:

    hip    the middle of the hips (x, y)
    torso  the hips-to-neck line's tilt, degrees from upright, + toward screen right
    head   the head's extra tilt on the torso's
    wide   how much of the shoulders' and hips' breadth shows (turned side-on: less)
    ra la rl ll   right/left arm and leg: (upper, lower) bone directions, degrees,
           0 = straight down, +90 = screen right, 180 = up
    ks     per-bone length scales where a bone points at or away from the camera
    w      the weapon: dict(blade=deg, reach=1, over=True) held in the right fist (blade in the
           same degrees), or dict(drop=(x, y, deg), over=True) lying where it fell
    two    overrides for the two-handed family (the left fist then takes the grip)

Bone lengths are the reference skeleton's (moveset.py `base_<view>.json`), so the figure keeps
its size whatever it does. For every clip, family and view it writes, as stances.py does:
    <out_dir>/<family>_<clip>_<view>.json / .hold.txt
A `shared` clip is written once, as `one_`, for both families (the weapon in one hand or dropped).
"""
import json, math, os, subprocess, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import moveset as mv

FIST = 0.028
OFF_TWO = 0.035
HEAD = ("NOSE", "RIGHT EYE", "LEFT EYE", "RIGHT EAR", "LEFT EAR")


def bones(base):
    d = lambda a, b: math.dist(base[a][:2], base[b][:2])
    hip = mid(base["RIGHT HIP"], base["LEFT HIP"])
    return dict(torso=math.dist(hip, base["NECK"][:2]),
                ru=d("RIGHT SHOULDER", "RIGHT ELBOW"), rf=d("RIGHT ELBOW", "RIGHT ARM"),
                lu=d("LEFT SHOULDER", "LEFT ELBOW"), lf=d("LEFT ELBOW", "LEFT ARM"),
                rt=d("RIGHT HIP", "RIGHT KNEE"), rs=d("RIGHT KNEE", "RIGHT LEG"),
                lt=d("LEFT HIP", "LEFT KNEE"), ls=d("LEFT KNEE", "LEFT LEG"))


def mid(a, b): return ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
def ray(p, deg, n): return (p[0] + n * math.sin(math.radians(deg)), p[1] + n * math.cos(math.radians(deg)))
def rot(v, deg):
    c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    return (v[0] * c - v[1] * s, v[0] * s + v[1] * c)


def ik(sh, target, a, b, bend):
    """The elbow of a two-bone arm from `sh` reaching `target`, bent to the side `bend` (+1/-1)."""
    dx, dy = target[0] - sh[0], target[1] - sh[1]
    d = min(max(math.hypot(dx, dy), 1e-6), a + b - 1e-6)
    cos_a = max(-1.0, min(1.0, (a * a + d * d - b * b) / (2 * a * d)))
    ang = math.atan2(dy, dx) + bend * math.acos(cos_a)
    return (sh[0] + a * math.cos(ang), sh[1] + a * math.sin(ang))


# The standing pose of each view, in these terms (the reference frame as drawn, the weapon carried
# low at the side, the blade forward and a little up).
STAND = {
    "north-east": dict(hip=(0.49, 0.645), torso=0, head=0, wide=1.0, ra=(17, 16), la=(-16, -5), rl=(0, 0), ll=(-9, -5),
                       w=dict(blade=118, reach=0.9, over=False)),
    "east": dict(hip=(0.54, 0.63), torso=0, head=0, wide=1.0, ra=(-16, 35), la=(13, 7), rl=(-18, 5), ll=(-4, 0),
                 w=dict(blade=107, reach=0.97)),
}


def k(t, **p): return (t, p)


# On the back, head to the left, knees a little up, the weapon arm out along the ground with the
# blade lying down-left (seen from behind, the near right side is the lower one: shoulders flipped).
LIE_BACK = dict(hip=(0.62, 0.81), torso=-84, head=-10, wide=-0.5, ra=(-65, -85), la=(-130, -100), rl=(80, 95), ll=(70, 100), ks=dict(torso=0.85),
                w=dict(blade=-80, reach=0.95, over=True))


# Each clip: frames per view, keys per view [(t, pose)] (a pose names only what it changes from the
# one before; the first starts from STAND), and whether it is held one-handed in both families.
CLIPS = {
    # Struck down for good: the knees go, onto them, forward onto the hands, face down; the weapon
    # slips from the hand as it goes down and lies by it. The game plays it over 60 ticks and
    # holds the last frame; waking plays it backwards.
    "death": dict(shared=True, frames={"north-east": 9}, keys={"north-east": [
        k(0),
        k(0.25, hip=(0.49, 0.67), torso=5, ra=(10, 10), la=(-10, -5), rl=(10, -5), ll=(0, -15), w=dict(blade=100, reach=0.9, over=False)),
        k(0.5, hip=(0.47, 0.74), torso=19, head=-5, ra=(-5, -30), la=(10, 15), rl=(17, -95), ll=(10, -100),
          w=dict(blade=60, reach=0.9, over=True)),
        k(0.75, hip=(0.38, 0.76), torso=55, head=-20, ra=(20, 55), la=(15, 30), rl=(25, -95), ll=(15, -100),
          w=dict(drop=(0.80, 0.90, 100), over=True)),
        k(1, hip=(0.38, 0.82), torso=84, head=-7, wide=0.6, ra=(85, 100), la=(70, 80), rl=(-88, -90), ll=(-86, -88),
          ks=dict(ru=0.8), w=dict(drop=(0.80, 0.92, 95), over=True)),
    ]}),
    # Thrown off the feet: the blow snaps the chest back and flings the arms, two steps back, down
    # onto the seat and over onto the back, head to the left, the weapon still in the fist. The game
    # plays 1-8 as the fall and holds the last; getup starts from it.
    "knockdown": dict(shared=True, frames={"north-east": 9}, keys={"north-east": [
        k(0),
        k(0.125, hip=(0.50, 0.65), torso=-12, head=-10, ra=(60, 100), la=(-60, -90), rl=(5, 0), ll=(-15, -5), w=dict(blade=150, reach=0.9, over=True)),
        k(0.25, hip=(0.52, 0.66), torso=-25, ra=(80, 130), la=(-45, -70), rl=(20, 5), ll=(-25, -10), w=dict(blade=160, reach=0.9, over=True)),
        k(0.5, hip=(0.54, 0.74), torso=-45, head=-10, wide=0.6, ra=(90, 140), la=(-50, -80), rl=(45, 30), ll=(30, 10), w=dict(blade=170, reach=0.9, over=True)),
        k(0.625, hip=(0.54, 0.82), torso=-55, wide=0.2, ra=(-10, -40), la=(-120, -150), rl=(75, 95), ll=(65, 100), w=dict(blade=-40, reach=0.95, over=True)),
        k(0.8, **LIE_BACK),
        k(1, **LIE_BACK),
    ]}),
    # Up off the back the way the knockdown left it: onto an elbow, sitting up and round onto one
    # knee, the weapon out in front, up through a crouch to the carry.
    "getup": dict(shared=True, frames={"north-east": 9}, keys={"north-east": [
        k(0, **LIE_BACK),
        k(0.25, hip=(0.58, 0.81), torso=-60, head=-5, wide=-0.4, ra=(-50, -90), la=(-130, -150), rl=(70, 110), ll=(40, 120), w=dict(blade=-70, reach=0.95, over=True)),
        k(0.5, hip=(0.5, 0.76), torso=-10, head=0, wide=0.6, ra=(20, 40), la=(-20, -10), rl=(10, -95), ll=(70, 5), w=dict(blade=100, reach=0.9, over=True)),
        k(0.75, hip=(0.49, 0.70), torso=10, wide=0.9, ra=(15, 30), la=(-15, -5), rl=(25, -30), ll=(40, -20), w=dict(blade=110, reach=0.9, over=True)),
        k(1, **STAND["north-east"]),
    ]}),
    # Resting at a shrine: the weapon turned point down at the side, then down onto the left knee,
    # the right foot planted, the fist on the pommel and the point in the ground, the head bowed.
    # The game kneels over it, holds the last frame, and plays it backwards to rise.
    "rest": dict(shared=True, frames={"north-east": 11}, keys={"north-east": [
        k(0),
        k(0.2, ra=(10, 10), w=dict(blade=10, reach=0.95, over=True)),
        k(0.4, hip=(0.5, 0.66), torso=5, ra=(25, 10), rl=(5, 0), ll=(-12, -5), w=dict(blade=0, reach=0.95, over=True)),
        k(0.6, hip=(0.48, 0.76), torso=12, head=5, ra=(30, 180), la=(10, 60), rl=(70, 0), ll=(5, -95), w=dict(blade=0, reach=0.95, over=True)),
        k(1, hip=(0.48, 0.77), torso=14, head=12, ra=(32, 180), la=(12, 62), rl=(70, 0), ll=(5, -95), w=dict(blade=0, reach=0.95, over=True)),
    ]}),
    # The parry: one-handed, the free hand flicks out to turn the blow (the buckler's way), the
    # weapon carried; two-handed, the weapon itself swept up across the body to meet it. The
    # game's live frames are 3-5; a fumble shows 0.62 and the flourish runs 0.45-0.95.
    "parry": dict(frames={"north-east": 7}, keys={"north-east": [
        k(0, two=dict(ra=(5, -60), w=dict(blade=150, reach=0.8, over=False))),
        k(0.33, torso=-5, la=(40, 70), two=dict(ra=(10, -120), w=dict(blade=160, reach=0.85, over=False))),
        k(0.5, torso=8, wide=0.6, la=(100, 100), rl=(5, 0), ll=(-12, -8), two=dict(ra=(30, -185), w=dict(blade=175, reach=0.85, over=True))),
        k(0.67, torso=8, wide=0.6, la=(105, 100), two=dict(ra=(32, -182), w=dict(blade=178, reach=0.85, over=True))),
        k(0.83, torso=6, wide=0.7, la=(100, 105), two=dict(ra=(30, -195), w=dict(blade=165, reach=0.85, over=True))),
        k(1, torso=4, wide=0.8, la=(95, 110), two=dict(ra=(25, -200), w=dict(blade=145, reach=0.85, over=True))),
    ]}),
    # Side-on (the yard faces right). Struck down: slump, onto the knees, the hands, face down with
    # the head to the right; the weapon slips from the hand at the last. The yard lies at 0.9.
    "e_death": dict(shared=True, frames={"east": 9}, keys={"east": [
        k(0),
        k(0.25, hip=(0.55, 0.66), torso=12, ra=(10, 60), rl=(-5, 5), ll=(-10, -10), w=dict(blade=100, reach=0.97)),
        k(0.5, hip=(0.55, 0.72), torso=35, ra=(30, 70), la=(40, 60), rl=(40, -30), ll=(10, -40), w=dict(blade=100, reach=0.97)),
        k(0.625, hip=(0.45, 0.73), torso=55, head=-10, ra=(15, 60), la=(25, 50), rl=(25, -95), ll=(5, -95), w=dict(blade=95, reach=0.97)),
        k(0.75, hip=(0.40, 0.74), torso=70, head=-15, ra=(10, 50), la=(15, 45), rl=(10, -95), ll=(-5, -95), w=dict(blade=92, reach=0.97)),
        k(1, hip=(0.36, 0.80), torso=84, head=-5, ra=(70, 100), la=(80, 95), rl=(-88, -90), ll=(-86, -90), w=dict(drop=(0.74, 0.92, 95))),
    ]}),
    # A jump in place (the yard lifts the sprite): crouch, spring up with the arms, knees tucked at
    # the top, legs down to land, settle. The yard shows 0.45 rising, 0.6 at the top, 0.75 falling.
    "e_jump": dict(shared=True, frames={"east": 9}, keys={"east": [
        k(0),
        k(0.125, hip=(0.55, 0.70), torso=15, ra=(30, 90), rl=(40, -30), ll=(25, -35), w=dict(blade=105, reach=0.97)),
        k(0.25, hip=(0.55, 0.73), torso=22, ra=(25, 85), la=(30, 20), rl=(55, -35), ll=(40, -40)),
        k(0.375, hip=(0.55, 0.60), torso=5, ra=(60, 120), la=(60, 140), rl=(-5, -10), ll=(-15, -20), w=dict(blade=115, reach=0.97)),
        k(0.5, hip=(0.55, 0.58), torso=0, ra=(70, 120), la=(50, 130), rl=(40, -20), ll=(10, -30)),
        k(0.625, hip=(0.55, 0.58), torso=10, ra=(60, 110), la=(40, 100), rl=(80, 0), ll=(60, -10)),
        k(0.75, hip=(0.55, 0.60), torso=5, ra=(50, 100), la=(30, 60), rl=(30, -5), ll=(0, -10), w=dict(blade=108, reach=0.97)),
        k(0.875, hip=(0.55, 0.72), torso=18, ra=(30, 90), la=(20, 30), rl=(50, -30), ll=(30, -35)),
        k(1, **STAND["east"]),
    ]}),
    # The parry side-on: the free hand punched out ahead (the buckler's way) or the weapon swept up.
    "e_parry": dict(frames={"east": 7}, keys={"east": [
        k(0, two=dict(ra=(-10, 60), w=dict(blade=120, reach=0.95))),
        k(0.33, torso=-3, la=(60, 150), two=dict(ra=(20, 60), w=dict(blade=140, reach=0.95))),
        k(0.5, torso=6, la=(95, 95), rl=(-10, 5), ll=(8, -5), two=dict(ra=(40, 150), w=dict(blade=172, reach=0.9))),
        k(0.67, torso=6, la=(98, 92), two=dict(ra=(42, 152), w=dict(blade=175, reach=0.9))),
        k(0.83, torso=4, la=(90, 100), two=dict(ra=(38, 140), w=dict(blade=165, reach=0.9))),
        k(1, torso=2, la=(80, 110), two=dict(ra=(30, 120), w=dict(blade=145, reach=0.95))),
    ]}),
    # The yard's roll, low as Pixellab draws one: a dip, a low dive onto the knee, along on the knees
    # and hands, up into a crouch. The road's roll takes the dip (2-4) and the crouch (11-12).
    "e_roll": dict(shared=True, frames={"east": 13}, keys={"east": [
        k(0),
        k(0.167, hip=(0.53, 0.66), torso=20, head=-5, ra=(60, 80), la=(70, 90), rl=(10, -10), ll=(-20, -5), w=dict(blade=95, reach=0.97)),
        k(0.333, hip=(0.50, 0.62), torso=45, head=-15, ra=(80, 95), la=(85, 100), rl=(80, -10), ll=(-40, -60)),
        k(0.5, hip=(0.45, 0.70), torso=62, head=-20, ra=(30, 70), la=(40, 80), rl=(25, -95), ll=(-10, -95)),
        k(0.667, hip=(0.42, 0.70), torso=70, head=-25, ra=(20, 60), la=(30, 70), rl=(30, -95), ll=(5, -95), w=dict(blade=92, reach=0.97)),
        k(0.833, hip=(0.42, 0.72), torso=72, head=-25, ra=(15, 60), la=(25, 65), rl=(30, -95), ll=(5, -95)),
        k(1, hip=(0.48, 0.74), torso=35, head=-5, ra=(20, 80), la=(20, 40), rl=(70, 0), ll=(5, -95), w=dict(blade=95, reach=0.97)),
    ]}),
    # The pieces of the road's roll over the shoulder: a crouch, the dive (1), the tuck (2, turned
    # in the engine), again, coming out of it (4), standing.
    "e_flip": dict(shared=True, frames={"east": 6}, keys={"east": [
        k(0, hip=(0.5, 0.72), torso=30, ra=(60, 100), la=(60, 100), rl=(60, -10), ll=(40, -30), w=dict(blade=100, reach=0.97)),
        k(0.2, hip=(0.42, 0.55), torso=80, head=-10, ra=(95, 95), la=(95, 95), rl=(-80, -85), ll=(-95, -90), w=dict(blade=95, reach=0.97)),
        k(0.4, hip=(0.5, 0.62), torso=60, head=20, ra=(40, 120), la=(30, 110), rl=(140, 10), ll=(150, 0), w=dict(blade=140, reach=0.9)),
        k(0.6, hip=(0.5, 0.62), torso=70, head=20, ra=(45, 125), la=(35, 115), rl=(145, 10), ll=(155, 0), w=dict(blade=145, reach=0.9)),
        k(0.8, hip=(0.5, 0.72), torso=10, ra=(60, 90), la=(50, 80), rl=(70, 0), ll=(20, -60), w=dict(blade=100, reach=0.97)),
        k(1, **STAND["east"]),
    ]}),
}


def blend(p, q, t):
    out = {}
    for key in set(p) | set(q):
        a, b = p.get(key), q.get(key)
        if a is None or b is None: out[key] = b if b is not None else a
        elif key == "w": out[key] = blend_w(a, b, t)
        elif key == "ks": out[key] = {n: mv.lerp(a.get(n, 1.0), b.get(n, 1.0), t) for n in set(a) | set(b)}
        elif isinstance(a, tuple): out[key] = tuple(mv.lerp(x, y, t) for x, y in zip(a, b))
        else: out[key] = mv.lerp(a, b, t)
    return out


def blend_w(a, b, t):
    if "drop" in a and "drop" in b:
        return dict(drop=tuple(mv.lerp(x, y, t) for x, y in zip(a["drop"], b["drop"])), over=b.get("over", True))
    if "drop" not in a and "drop" not in b:
        return dict(blade=mv.lerp(a["blade"], b["blade"], t), reach=mv.lerp(a.get("reach", 1), b.get("reach", 1), t), over=b.get("over", a.get("over", True)))
    return dict(a, _to=b, _t=t)  # leaving the hand (or taken up): resolved once the fist is known


def full(view, keys, family):
    """Each key as a whole pose: the one before it with its changes (and the family's)."""
    out, cur = [], dict(STAND[view])
    for t, p in keys:
        p = dict(p)
        two = p.pop("two", None)
        # Foreshortening belongs to the key that sets it: a key without `ks` has full-length bones.
        cur = dict(cur, **p)
        if "ks" not in p: cur["ks"] = {}
        out.append((t, dict(cur, **(two or {})) if family == "two" else dict(cur)))
    return out


def frames(clip, view, family):
    keys = full(view, clip["keys"][view], family)
    n = clip["frames"][view]
    out = []
    for i in range(n):
        t = i / max(1, n - 1)
        j = max(x for x in range(len(keys)) if keys[x][0] <= t + 1e-9)
        if j == len(keys) - 1: out.append(blend(keys[j][1], keys[j][1], 0.0))
        else:
            (t0, a), (t1, b) = keys[j], keys[j + 1]
            out.append(blend(a, b, mv.ease((t - t0) / (t1 - t0))))
    return out


def pose(base, L, f, family, shared):
    j = {key: list(v) for key, v in base.items()}
    bhip = mid(base["RIGHT HIP"], base["LEFT HIP"])
    ks = f.get("ks", {})
    hip, tilt, wide = f["hip"], f["torso"], f.get("wide", 1.0)
    neck = ray(hip, 180 - tilt, L["torso"] * ks.get("torso", 1.0))
    j["NECK"][:2] = neck
    for n in HEAD:
        v = rot((base[n][0] - base["NECK"][0], base[n][1] - base["NECK"][1]), tilt + f.get("head", 0))
        j[n][:2] = (neck[0] + v[0], neck[1] + v[1])
    for side in ("RIGHT", "LEFT"):
        v = rot(((base[f"{side} SHOULDER"][0] - base["NECK"][0]) * wide, base[f"{side} SHOULDER"][1] - base["NECK"][1]), tilt)
        j[f"{side} SHOULDER"][:2] = (neck[0] + v[0], neck[1] + v[1])
        v = rot(((base[f"{side} HIP"][0] - bhip[0]) * wide, base[f"{side} HIP"][1] - bhip[1]), tilt)
        j[f"{side} HIP"][:2] = (hip[0] + v[0], hip[1] + v[1])
    for name, root, a, b, (u, l) in (("ra", "RIGHT SHOULDER", "RIGHT ELBOW", "RIGHT ARM", ("ru", "rf")),
                                     ("la", "LEFT SHOULDER", "LEFT ELBOW", "LEFT ARM", ("lu", "lf")),
                                     ("rl", "RIGHT HIP", "RIGHT KNEE", "RIGHT LEG", ("rt", "rs")),
                                     ("ll", "LEFT HIP", "LEFT KNEE", "LEFT LEG", ("lt", "ls"))):
        d1, d2 = f[name]
        e = ray(j[root][:2], d1, L[u] * ks.get(u, 1.0))
        j[a][:2] = e
        j[b][:2] = ray(e, d2, L[l] * ks.get(l, 1.0))
    for key, z in f.get("z", {}).items(): j[key][2] = z
    # The weapon: the grip a half fist past the wrist along the forearm.
    el, wr = j["RIGHT ELBOW"][:2], j["RIGHT ARM"][:2]
    fl = max(math.dist(el, wr), 1e-6)
    fist = (wr[0] + (wr[0] - el[0]) / fl * FIST * 0.5, wr[1] + (wr[1] - el[1]) / fl * FIST * 0.5)
    w = f["w"]
    def held(w):
        return (fist, w["blade"], w.get("reach", 1.0), w.get("over", True), True)
    def lying(w):
        x, y, deg = w["drop"]
        return ((x, y), deg, 1.0, w.get("over", True), False)
    if "_to" in w:
        a = held(w) if "drop" not in w else lying(w)
        b = held(w["_to"]) if "drop" not in w["_to"] else lying(w["_to"])
        t = w["_t"]
        g = tuple(mv.lerp(p, q, t) for p, q in zip(a[0], b[0]))
        grip = (g, mv.lerp(a[1], b[1], t), mv.lerp(a[2], b[2], t), b[3], a[4] and b[4] and False)
    else:
        grip = held(w) if "drop" not in w else lying(w)
    (gx, gy), deg, reach, over, in_hand = grip
    if family == "two" and not shared and in_hand:
        # The left fist on the grip, a hand's width along the blade, the elbow bent down and out.
        tgt = ray((gx, gy), deg, -OFF_TWO)
        e = ik(j["LEFT SHOULDER"][:2], tgt, L["lu"], L["lf"], 1)
        j["LEFT ELBOW"][:2], j["LEFT ARM"][:2] = e, tgt
    joints = [{"label": key, "x": round(v[0], 4), "y": round(v[1], 4), "z_index": v[2]} for key, v in j.items()]
    # Hold: degrees on screen as the game reads them, 0 = right, 90 = down.
    track = dict(grip=[round(gx, 4), round(gy, 4)], angle=round(90 - deg, 1), over=over, reach=round(reach, 3),
                 elbow=[round(el[0], 4), round(el[1], 4)], fit=in_hand and f.get("fit", False))
    return joints, track


def build(name, view):
    clip = CLIPS[name]
    base = mv.load_base(view)
    L = bones(base)
    for family in (["one"] if clip.get("shared") else ["one", "two"]):
        yield family, [pose(base, L, f, family, clip.get("shared")) for f in frames(clip, view, family)]


def write(out, names):
    os.makedirs(out, exist_ok=True)
    for name in names:
        for view in CLIPS[name]["frames"]:
            for family, fr in build(name, view):
                stem = f"{family}_{name}_{view}"
                json.dump([s for s, _ in fr], open(os.path.join(out, stem + ".json"), "w"))
                with open(os.path.join(out, stem + ".hold.txt"), "w") as fh:
                    for _, t in fr:
                        elbow = f" {t['elbow'][0]} {t['elbow'][1]}" if t["fit"] else ""
                        fh.write(f"{t['grip'][0]} {t['grip'][1]} {t['angle']} {int(t['over'])} {t['reach']}{elbow}\n")
                xs = [d["x"] for s, _ in fr for d in s]; ys = [d["y"] for s, _ in fr for d in s]
                edge = min(xs) < 0.04 or max(xs) > 0.96 or min(ys) < 0.02 or max(ys) > 0.98
                print(f"{stem:30} {len(fr):2} frames" + (f"  EDGE x {min(xs):.2f}..{max(xs):.2f} y {min(ys):.2f}..{max(ys):.2f}" if edge else ""))


def preview(out, aligned, tool, names):
    os.makedirs(out, exist_ok=True)
    for name in names:
        for view in CLIPS[name]["frames"]:
            pre = "e_" if view == "east" and not name.startswith("e_") else ""
            for family, fr in build(name, view):
                imgs = []
                for i, (s, t) in enumerate(fr):
                    a = math.radians(t["angle"]); g = t["grip"]; n = 0.36 * t["reach"]
                    s = s + [{"label": "WEAPON", "x": g[0] - 0.03 * math.cos(a), "y": g[1] - 0.03 * math.sin(a), "tx": g[0] + n * math.cos(a), "ty": g[1] + n * math.sin(a)}]
                    p = os.path.join(out, "_kp.json"); json.dump(s, open(p, "w"))
                    ref = os.path.join(aligned, pre + name, f"{i:03}.png")
                    if not os.path.exists(ref): ref = os.path.join(aligned, "blank.png")
                    o = os.path.join(out, f"_{i}.png")
                    subprocess.run([tool, "skel", ref, p, "3", o], check=True); imgs.append(o)
                rows = []
                for r in range(0, len(imgs), 5):
                    rows.append(os.path.join(out, f"_row{r}.png"))
                    subprocess.run([tool, "sheet", rows[-1], "1"] + imgs[r:r + 5], check=True)
                subprocess.run([tool, "stack", os.path.join(out, f"{family}_{name}_{view}.png")] + rows, check=True)
                for o in imgs + rows: os.remove(o)


if __name__ == "__main__":
    if sys.argv[1] == "--preview":
        preview(sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5:] or list(CLIPS))
    else:
        write(sys.argv[1], sys.argv[2:] or list(CLIPS))
