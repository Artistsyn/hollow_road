"""Non-attack skeletons for the weapon families, for Pixellab's animate_with_skeleton_v3, with the
weapon track the game draws the held weapon along.

    python3 -I stances.py <out_dir> [clip ...]

The hero's walk, guard, reels, backstep and drink are drawn bare-handed once per way of holding a
weapon, and the game puts whichever weapon is worn in the hand (anim.rs `Sheet::arm`):

    one  a weapon in the right hand, the left free: the sword, the rapier, the axe
    two  both hands on the grip: the spear, the greatsword

For every clip, family and view it writes, as moveset.py does:
    <out_dir>/<family>_<clip>_<view>.json / .weapon.json / .hold.txt
(the game's folders are `<family>_<clip>` on the road and `e_<family>_<clip>` side-on). A clip
marked `shared` holds the weapon one-handed in both families (a drink takes the left hand off the
grip), so it is drawn once, as `one_`, and copied to `two_`.

The weapon arm is posed bone by bone from the keys below, in the body's 3D frame (F, U, R), the way
moveset.py poses a strike; the body leans, steps, sinks and turns with the same parameters, plus:
    gait   the walk's phase (0..1): the feet step in turn and the free hand swings against them
    lift   the whole figure raised (image heights): a hop's flight
    left   where the free fist goes (3D, from the left shoulder), when it is not on the grip
"""
import json, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import moveset as mv

# The weapon arm's keys per family: (upper, fore, blade), each (F, U, R).
ARMS = {
    "one": {
        # Low at the side, the blade forward and a little up, the wrist relaxed.
        "carry": ((0.10, -0.99, 0.10), (0.45, -0.88, 0.05), (0.92, 0.30, 0.20)),
        # Forearm level and forward, the blade up and forward over it.
        "guard": ((0.35, -0.90, 0.20), (0.95, 0.25, -0.10), (0.75, 0.62, -0.20)),
        # The guard knocked back toward the chest.
        "jolt": ((0.05, -0.95, 0.30), (0.85, 0.45, -0.10), (0.55, 0.80, -0.20)),
        # Smashed open: the arm thrown up and out to the side.
        "flung": ((-0.35, -0.40, 0.85), (-0.25, 0.60, 0.75), (-0.30, 0.90, 0.30)),
        # A flinch: the arm out to the side, the blade trailing.
        "flinch": ((-0.15, -0.90, 0.40), (0.40, -0.30, 0.85), (0.80, 0.20, 0.55)),
    },
    "two": {
        # Fists at the right hip, the blade across the front and up.
        "carry": ((0.15, -0.97, 0.10), (0.75, -0.30, -0.55), (0.80, 0.45, -0.40)),
        "guard": ((0.30, -0.90, -0.15), (0.85, 0.05, -0.50), (0.70, 0.65, -0.20)),
        "jolt": ((0.15, -0.95, -0.10), (0.80, 0.30, -0.50), (0.45, 0.85, -0.25)),
        "flung": ((-0.20, 0.10, 0.97), (0.10, 0.85, -0.50), (-0.50, 0.85, 0.10)),
        "flinch": ((-0.10, -0.95, 0.20), (0.60, -0.20, -0.75), (0.70, 0.60, -0.35)),
    },
}
# The second fist's place along the grip from the first (one hand: none).
OFF = {"one": None, "two": 0.035}

# The free (left) fist, from the left shoulder: hanging, raised in a guard, at the mouth.
LEFT = {
    "hang": (0.03, -0.26, -0.02),
    "guard": (0.16, -0.12, 0.10),
    "mouth": (0.10, 0.08, 0.10),
    "out": (-0.05, -0.12, -0.22),
    # The buckler's parry: pulled in to the chest, then punched straight out ahead and up past the
    # left of the head to meet the blow face on, held there, and coming back.
    "tuck": (0.10, -0.10, 0.08),
    "punch": (0.27, 0.03, -0.06),
    "punched": (0.26, 0.05, -0.07),
    "back": (0.17, -0.07, 0.0),
}

# A clip: frames per view, keyframes [(t, pose)], whether the walk's gait runs, whether it is held
# one-handed in both families. A pose: arm (a key of ARMS), left (a key of LEFT, ignored on two
# hands), and any of lean, step, drop, twist, lift.
def k(t, arm, left="hang", **body):
    return (t, dict(arm=arm, left=left, **body))


CLIPS = {
    "walk": dict(frames={"north-east": 8, "east": 8}, gait=True, keys=[k(0, "carry", lean=3), k(1, "carry", lean=3)]),
    # Running away up the road (straight behind only): leaning into it, a flight between steps.
    "run": dict(frames={"north": 8}, gait=True, run=True, keys=[k(0, "carry", lean=14), k(1, "carry", lean=14)]),
    # The one-handed parry straight behind (bodies.py poses the others): the free fist punched out
    # ahead with the buckler, the chest turned into it, the weapon kept low for the riposte. The
    # game's live frames are 3-5; a fumble shows 0.62 and the flourish runs 0.45-0.95.
    "parry": dict(frames={"north": 7}, families=["one"], keys=[
        k(0, "carry"), k(0.17, "carry", "tuck", lean=2, twist=-6),
        k(0.33, "carry", "punch", lean=5, step=0.02, twist=-16), k(0.5, "carry", "punched", lean=6, step=0.03, drop=0.01, twist=-18),
        k(0.67, "carry", "punched", lean=6, step=0.03, drop=0.01, twist=-18), k(0.83, "carry", "punched", lean=5, step=0.03, drop=0.008, twist=-16),
        k(1, "carry", "back", lean=3, step=0.02, twist=-8)]),
    # Up from the carry into the guard, held on the last frame.
    "guard": dict(frames={"north-east": 5, "east": 5}, keys=[
        k(0, "carry"), k(1, "guard", "guard", lean=4, step=0.03, drop=0.01, twist=-10)]),
    # Standing ready side-on, breathing.
    "idle": dict(frames={"east": 4}, loop=True, keys=[
        k(0, "guard", "guard", lean=3, step=0.03, drop=0.008, twist=-8), k(0.5, "guard", "guard", lean=3, step=0.03, drop=0.016, twist=-8),
        k(1, "guard", "guard", lean=3, step=0.03, drop=0.008, twist=-8)]),
    # A blow met on the guard: knocked back (the game plays 3-5), then set again.
    "block_hit": dict(frames={"north-east": 7}, keys=[
        k(0, "guard", "guard", lean=4, step=0.03, drop=0.01, twist=-10), k(0.3, "guard", "guard", lean=4, step=0.03, drop=0.01, twist=-10),
        k(0.6, "jolt", "guard", lean=-14, step=-0.02, drop=0.02, twist=5), k(1, "guard", "guard", lean=2, step=0.02, drop=0.012, twist=-8)]),
    # Struck: the head and chest snap back and turn, the arms go out, then come back.
    "hurt": dict(frames={"north-east": 7, "east": 7}, keys=[
        k(0, "carry"), k(0.3, "flinch", "out", lean=-20, step=-0.04, drop=0.015, twist=25),
        k(0.55, "flinch", "out", lean=-15, step=-0.04, drop=0.02, twist=16), k(1, "carry", lean=0)]),
    # Staggered: rocked back hard, swaying (the game loops 1-5), finding the feet again.
    "stagger": dict(frames={"north-east": 9}, keys=[
        k(0, "guard", "guard", lean=4, step=0.03, drop=0.01), k(0.2, "flinch", "out", lean=-26, step=-0.07, drop=0.025, twist=24),
        k(0.4, "flinch", "out", lean=-20, step=-0.07, drop=0.04, twist=12), k(0.6, "flinch", "out", lean=-28, step=-0.07, drop=0.03, twist=26),
        k(1, "carry", lean=-2, drop=0.01)]),
    # The guard smashed open: the weapon thrown up and out (the game plays 3-8), the body rocked.
    "guard_break": dict(frames={"north-east": 9}, keys=[
        k(0, "guard", "guard", lean=4, step=0.03, drop=0.01, twist=-10), k(0.25, "jolt", "guard", lean=-6, step=0.0, drop=0.02),
        k(0.5, "flung", "out", lean=-24, step=-0.06, drop=0.025, twist=28), k(0.8, "flung", "out", lean=-20, step=-0.06, drop=0.035, twist=20),
        k(1, "flinch", "out", lean=-8, step=-0.04, drop=0.03, twist=10)]),
    # A hop back out of reach: crouch, off the ground, land, settle into the guard.
    "backstep": dict(frames={"north-east": 9}, keys=[
        k(0, "guard", "guard", lean=4, step=0.03, drop=0.01), k(0.2, "guard", "guard", lean=6, step=0.02, drop=0.03),
        k(0.45, "guard", "guard", lean=-6, step=-0.04, drop=0.0, lift=0.05), k(0.7, "guard", "guard", lean=2, step=-0.02, drop=0.035),
        k(1, "guard", "guard", lean=4, step=0.03, drop=0.01)]),
    # The flask to the mouth with the free hand, the head tipping back, and down again.
    "drink": dict(frames={"north-east": 9}, shared=True, keys=[
        k(0, "carry"), k(0.3, "carry", "mouth", lean=-2), k(0.6, "carry", "mouth", lean=-8, twist=5), k(1, "carry", lean=0)]),
}

# The road's straight-behind view has every clip the north-east one has, frame for frame.
for _c in CLIPS.values():
    if "north-east" in _c["frames"]: _c["frames"]["north"] = _c["frames"]["north-east"]

# Straight behind, the walk and the run are posed in 3D (`gait3d`) rather than nudged on screen:
# per leg a stance (the foot planted, sliding back under the body as it goes on, the heel rising
# off it at the end) and a swing (the foot lifted, the shin folding back, carried forward to land
# on the heel); the hips ride over the planted foot, highest over it walking and lowest running
# (the flight comes between steps), and sway toward it; the shoulders turn against the hips and
# the arms swing against the legs. Lengths are the reference body's, in image heights: thigh,
# shin, the hips' half breadth, the hip's height over the ankle standing.
LEG3 = dict(thigh=0.165, shin=0.155, hw=0.08, stand=0.309)
GAIT3 = {
    # duty: the part of a stride a foot is planted. front/back: how far ahead of the hips it lands
    # and how far behind them it leaves. track: how far out from the middle the feet fall.
    # clear: the swing ankle's highest. heel: the ankle's rise as the heel peels off. hip: the
    # hips' mean height. bob: their rise over the planted foot (negative: a dip). sway: out over
    # it. twist: the shoulders' turn (degrees). arm/free: the weapon arm's and the free arm's swing.
    "walk": dict(duty=0.62, front=0.12, back=0.13, track=0.045, clear=0.04, heel=0.022, hip=0.300,
                 bob=0.011, sway=0.016, twist=6.0, arm=7.0, free=22.0),
    "run": dict(duty=0.38, front=0.08, back=0.18, track=0.03, clear=0.13, heel=0.03, hip=0.284,
                bob=-0.018, sway=0.008, twist=12.0, arm=14.0, free=34.0),
}
# Running the free arm is bent at the elbow, the fist carried by the ribs.
LEFT_RUN = (0.10, -0.15, -0.03)


def rot_r(v, deg):
    """A 3D (F, U, R) offset turned about the body's right axis: + swings a hanging arm forward."""
    a = math.radians(deg); f, u, r = v
    return (f * math.cos(a) - u * math.sin(a), u * math.cos(a) + f * math.sin(a), r)


def foot3d(g, q):
    """A foot at its own phase q (0 = landing on the heel): (forward, up) of the ankle, from under
    its hip at the ground."""
    d = g["duty"]
    if q < d:
        k = q / d
        heel = 0.0 if k < 0.7 else g["heel"] * ((k - 0.7) / 0.3) ** 2
        return g["front"] - (g["front"] + g["back"]) * k, heel
    k = (q - d) / (1 - d)
    return -g["back"] + (g["front"] + g["back"]) * (1 - math.cos(math.pi * k)) / 2, g["clear"] * math.sin(math.pi * k ** 0.7)


def knee3d(hip, ankle, pole):
    """The knee between hip and ankle (3D), bent toward pole."""
    L1, L2 = LEG3["thigh"], LEG3["shin"]
    v = mv.sub(ankle, hip); d = max(min(mv.norm(v), L1 + L2 - 1e-4), abs(L1 - L2) + 1e-4)
    axis = mv.unit(v)
    x = (L1 * L1 - L2 * L2 + d * d) / (2 * d); h = math.sqrt(max(0.0, L1 * L1 - x * x))
    side = mv.unit(mv.sub(pole, mv.mul(axis, mv.dot(pole, axis))))
    return mv.add(hip, mv.add(mv.mul(axis, x), mv.mul(side, h)))


def gait3d(j, dep, track, g, t):
    """Straight behind: the legs and the hips' ride at phase t of the stride (the right heel lands
    at 0), the upper body moved with the hips. `j` is the posed figure, changed in place."""
    s = mv.VIEWS["north"]["s"]; c = math.sqrt(1 - s * s)
    # The ground under the hips where the reference drawing stands (base_north.json's ankles).
    scr = lambda v: (0.5 + v[2], 0.8775 - v[1] * c - v[0] * s)
    d = g["duty"]
    ride = lambda ph: math.cos(2 * math.pi * (2 * ph - d))   # 1 with a foot under the hips
    hip_u = g["hip"] + g["bob"] * ride(t)
    sway = g["sway"] * math.cos(2 * math.pi * (t - d / 2))
    stood = LEG3["stand"]
    for side, sign, ph in (("RIGHT", 1.0, t), ("LEFT", -1.0, (t + 0.5) % 1.0)):
        planted = ph < d
        # The swing side's hip drops a little while the other leg holds the weight.
        hip = (0.0, hip_u - (0.0 if planted else 0.006), sign * LEG3["hw"] + sway)
        fwd, up = foot3d(g, ph)
        ankle = (fwd, up, sign * g["track"] + sway * 0.25)
        knee = knee3d(hip, ankle, (1.0, 0.0, sign * 0.25))
        for part, v in (("HIP", hip), ("KNEE", knee), ("LEG", ankle)):
            j[f"{side} {part}"][0], j[f"{side} {part}"][1] = scr(v)
        # A leg back behind the body is nearer the camera than one carried forward.
        dep[f"{side} KNEE"] = mv.dn((knee[0], 0.0, 0.0)); dep[f"{side} LEG"] = mv.dn((ankle[0], 0.0, 0.0))
    # Everything above the hips rides with them: up and down, and out over the planted foot.
    dx, dy = sway * 0.8, -(hip_u - stood) * c
    for key in j:
        if not key.endswith(("HIP", "KNEE", "LEG")): j[key][0] += dx; j[key][1] += dy
    for key in ("grip", "elbow"):
        track[key][0] = round(track[key][0] + dx, 4); track[key][1] = round(track[key][1] + dy, 4)

# The walk on screen, per view: a foot's reach forward from the stance's middle, how high the
# passing foot lifts, the free hand's swing back, the body's rise at each passing.
GAIT = {"north-east": dict(step=(0.03, -0.04), lift=0.025, swing=(-0.02, 0.025), bob=0.008),
        # Walking away: the leading foot climbs the screen as it goes further off, the passing
        # heel lifts toward the camera, the free hand swings forward (up) and back (down).
        "north": dict(step=(0.0, -0.035), lift=0.03, swing=(0.0, -0.03), bob=0.008),
        "east": dict(step=(0.055, 0.0), lift=0.025, swing=(-0.03, 0.0), bob=0.008)}


def blend(p, q, t):
    out = {key: mv.lerp(p.get(key, 0.0), q.get(key, 0.0), t) for key in ("lean", "step", "drop", "twist", "lift")}
    out["arm"] = tuple(mv.slerp(a, b, t) for a, b in zip(p["armv"], q["armv"]))
    out["left"] = tuple(mv.lerp(a, b, t) for a, b in zip(p["leftv"], q["leftv"]))
    return out


def frames(clip, n, family):
    keys = []
    for t, p in clip["keys"]:
        p = dict(p)
        p["armv"] = ARMS["one" if clip.get("shared") else family][p["arm"]]
        p["leftv"] = LEFT[p["left"]]
        keys.append((t, p))
    out = []
    for i in range(n):
        # A loop's last frame leads back into its first; a one-shot ends on its last key.
        t = i / n if clip.get("loop") or clip.get("gait") else i / max(1, n - 1)
        j = max(x for x in range(len(keys)) if keys[x][0] <= t + 1e-9)
        if j == len(keys) - 1:
            f = blend(keys[j][1], keys[j][1], 0.0)
        else:
            (t0, a), (t1, b) = keys[j], keys[j + 1]
            f = blend(a, b, mv.ease((t - t0) / (t1 - t0)))
        f["gait"] = t if clip.get("gait") else None
        f["run"] = bool(clip.get("run"))
        out.append(f)
    return out


def place_left(j, view, target, left_sh_3d, sh2d):
    """The free arm with its fist at `target` (3D, from the left shoulder), elbow down and out."""
    e, w, _ = mv.solve_arm(target, (0.0, -1.0, -0.6))
    lsh = tuple(j["LEFT SHOULDER"][:2])
    j["LEFT ELBOW"][:2] = list(mv.add(lsh, mv.proj(view, e)))
    j["LEFT ARM"][:2] = list(mv.add(lsh, mv.proj(view, w)))
    if view == "north":
        # Straight behind, a fist out in front is hidden by the back; one drawn over it reads as a
        # hand on the cloak.
        for part, v in (("LEFT ELBOW", e), ("LEFT ARM", w)): j[part][2] = mv.zn(v)
        return {"LEFT ELBOW": mv.dn(mv.add(e, (0.0, 0.0, -0.5 * mv.CHEST))), "LEFT ARM": mv.dn(mv.add(w, (0.0, 0.0, -0.5 * mv.CHEST)))}
    return {}


def pose(base, view, f, family, shared):
    arm, twist, left = f["arm"], f["twist"], f["left"]
    g3 = GAIT3["run" if f.get("run") else "walk"] if view == "north" and f["gait"] is not None else None
    if g3:
        # The arms swing against the legs (the right arm back as the right heel lands at 0), the
        # shoulders turning with them.
        w = math.cos(2 * math.pi * f["gait"])
        sw = g3["arm"] * (0.4 if family == "two" else 1.0)
        arm = tuple(rot_r(v, -sw * w) for v in arm)
        twist = twist - g3["twist"] * w * (0.5 if family == "two" else 1.0)
        left = rot_r(LEFT_RUN if f.get("run") else left, g3["free"] * w)
    p = dict(upper=arm[0], fore=arm[1], blade=arm[2], lean=f["lean"], step=f["step"], drop=f["drop"], twist=twist)
    off = None if shared else OFF[family]
    joints, track = mv.pose(base, view, p, off)
    dep = {d["label"]: d["depth"] for d in joints if "depth" in d}
    j = {d["label"]: [d["x"], d["y"], d["z_index"]] for d in joints}
    if off is None:
        dep.update(place_left(j, view, left, None, None))
    if g3:
        gait3d(j, dep, track, g3, f["gait"])
    elif f["gait"] is not None:
        g = GAIT[view]
        ph = 2 * math.pi * f["gait"]
        rise = g["bob"] * math.cos(2 * ph)
        for side, a in (("RIGHT", ph), ("LEFT", ph + math.pi)):
            fwd, lift = math.sin(a), max(0.0, math.cos(a))
            hip, knee, ankle = j[f"{side} HIP"], j[f"{side} KNEE"], j[f"{side} LEG"]
            ax, ay = ankle[0] + g["step"][0] * fwd, ankle[1] + g["step"][1] * fwd - g["lift"] * lift
            out = (1 if side == "RIGHT" else -1) if g["step"][0] == 0 else 1 if g["step"][0] > 0 else -1
            knee[0] = (hip[0] + ax) / 2 + 0.012 * lift * out
            knee[1] = (hip[1] + ay) / 2 - 0.008 * lift
            ankle[0], ankle[1] = ax, ay
            # The free hand swings against its own side's foot.
            if side == "LEFT" and off is None:
                for part, kk in (("LEFT ARM", 1.0), ("LEFT ELBOW", 0.5)):
                    j[part][0] += g["swing"][0] * fwd * kk; j[part][1] += g["swing"][1] * fwd * kk
        for key in j:
            if not key.endswith(("KNEE", "LEG")): j[key][1] -= rise
        track["grip"][1] = round(track["grip"][1] - rise, 4)
        track["elbow"][1] = round(track["elbow"][1] - rise, 4)
    if f["lift"]:
        for key in j: j[key][1] -= f["lift"]
        track["grip"][1] = round(track["grip"][1] - f["lift"], 4)
        track["elbow"][1] = round(track["elbow"][1] - f["lift"], 4)
    joints = [{"label": key, "x": round(v[0], 4), "y": round(v[1], 4), "z_index": v[2]} for key, v in j.items()]
    if dep:
        for d in joints: d["depth"] = dep.get(d["label"], 128)
    return joints, track


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    names = sys.argv[2:] or list(CLIPS)
    for view in mv.VIEWS:
        base = mv.load_base(view)
        first = [{"label": key, "x": v[0], "y": v[1], "z_index": v[2]} for key, v in base.items()]
        json.dump(first, open(os.path.join(out, f"{view}_first.json"), "w"))
        for name in names:
            clip = CLIPS[name]
            n = clip["frames"].get(view)
            if n is None: continue
            for family in (["one"] if clip.get("shared") else clip.get("families", ["one", "two"])):
                skel, track = zip(*(pose(base, view, f, family, clip.get("shared")) for f in frames(clip, n, family)))
                stem = f"{family}_{name}_{view}"
                json.dump(list(skel), open(os.path.join(out, stem + ".json"), "w"))
                json.dump(list(track), open(os.path.join(out, stem + ".weapon.json"), "w"), indent=1)
                # The elbow (which has the game fit the grip to the fist as drawn, anim.rs
                # `fit_hold`) only for a forearm raised clear of the body: by the hip, the farthest
                # drawn pixel along the forearm is the cloak or a leg, not the fist.
                raised = [f["arm"][1][1] > -0.3 for f in frames(clip, n, family)]
                with open(os.path.join(out, stem + ".hold.txt"), "w") as fh:
                    for t, up in zip(track, raised):
                        elbow = f" {t['elbow'][0]} {t['elbow'][1]}" if up else ""
                        fh.write(f"{t['grip'][0]} {t['grip'][1]} {t['angle']} {int(t['over'])} {t['reach']}{elbow}\n")
                lo, hi = mv.WRIST_ANY
                bad = [f"{i}:{t['wrist']}" for i, t in enumerate(track) if not lo <= t["wrist"] <= hi]
                xs = [d["x"] for s in skel for d in s]
                print(f"{stem:28} {len(skel):2} frames  wrist {' '.join(str(t['wrist']) for t in track)}"
                      + (f"  OUT OF RANGE {' '.join(bad)}" if bad else "") + (f"  EDGE x {min(xs):.2f}..{max(xs):.2f}" if min(xs) < 0.04 or max(xs) > 0.96 else ""))


if __name__ == "__main__":
    main()
