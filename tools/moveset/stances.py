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
}

# A clip: frames per view, keyframes [(t, pose)], whether the walk's gait runs, whether it is held
# one-handed in both families. A pose: arm (a key of ARMS), left (a key of LEFT, ignored on two
# hands), and any of lean, step, drop, twist, lift.
def k(t, arm, left="hang", **body):
    return (t, dict(arm=arm, left=left, **body))


CLIPS = {
    "walk": dict(frames={"north-east": 8, "east": 8}, gait=True, keys=[k(0, "carry", lean=3), k(1, "carry", lean=3)]),
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

# The walk on screen, per view: a foot's reach forward from the stance's middle, how high the
# passing foot lifts, the free hand's swing back, the body's rise at each passing.
GAIT = {"north-east": dict(step=(0.03, -0.04), lift=0.025, swing=(-0.02, 0.025), bob=0.008),
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
        out.append(f)
    return out


def place_left(j, view, target, left_sh_3d, sh2d):
    """The free arm with its fist at `target` (3D, from the left shoulder), elbow down and out."""
    e, w, _ = mv.solve_arm(target, (0.0, -1.0, -0.6))
    lsh = tuple(j["LEFT SHOULDER"][:2])
    j["LEFT ELBOW"][:2] = list(mv.add(lsh, mv.proj(view, e)))
    j["LEFT ARM"][:2] = list(mv.add(lsh, mv.proj(view, w)))


def pose(base, view, f, family, shared):
    p = dict(upper=f["arm"][0], fore=f["arm"][1], blade=f["arm"][2], lean=f["lean"], step=f["step"], drop=f["drop"], twist=f["twist"])
    off = None if shared else OFF[family]
    joints, track = mv.pose(base, view, p, off)
    j = {d["label"]: [d["x"], d["y"], d["z_index"]] for d in joints}
    if off is None:
        place_left(j, view, f["left"], None, None)
    if f["gait"] is not None:
        g = GAIT[view]
        ph = 2 * math.pi * f["gait"]
        rise = g["bob"] * math.cos(2 * ph)
        for side, a in (("RIGHT", ph), ("LEFT", ph + math.pi)):
            fwd, lift = math.sin(a), max(0.0, math.cos(a))
            hip, knee, ankle = j[f"{side} HIP"], j[f"{side} KNEE"], j[f"{side} LEG"]
            ax, ay = ankle[0] + g["step"][0] * fwd, ankle[1] + g["step"][1] * fwd - g["lift"] * lift
            knee[0] = (hip[0] + ax) / 2 + 0.012 * lift * (1 if g["step"][0] >= 0 else -1)
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
            for family in (["one"] if clip.get("shared") else ["one", "two"]):
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
