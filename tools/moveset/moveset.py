"""Attack skeletons per weapon type, for Pixellab's animate_with_skeleton_v3, and the weapon track
the game draws the weapon along.

    python3 -I moveset.py <out_dir> [type ...]

For every type, move (light, heavy) and view (east: the side-on yard; north-east: the road, seen
from behind and to the right) it writes:
    <out_dir>/<type>_<move>_<view>.json         the skeletons, one per frame (Pixellab `keypoints`)
    <out_dir>/<type>_<move>_<view>.weapon.json  per frame: grip (fractions), angle (degrees, 0 =
                                                right, 90 = down), elbow (the skeleton's, which
                                                the game fits the grip to the drawn fist from),
                                                over (drawn over the body),
                                                reach (how much of the blade shows), and the
                                                wrist's bend (degrees between forearm and blade)
    <out_dir>/<type>_<move>_<view>.hold.txt     the same as the game reads it: the clip folder's
                                                hold.txt (`<type>_<move>` on the road,
                                                `e_<type>_<move>` side-on)
and <out_dir>/<view>_first.json, the skeleton of the bare reference frame (`first_frame_keypoints`).

A pose is authored once, in the body's own 3D frame (F forward, U up, R the body's right): which
way the weapon arm's upper arm and forearm point (`upper`, `fore`), which way the blade points
(`blade`), how far the chest turns into the blow (`twist`, degrees, the right shoulder forward),
leans (`lean`), steps (`step`) and sinks (`drop`). The weapon arm is posed bone by bone, so the
elbow is where the key puts it; the fist closes on the grip a fist's length past the wrist; the
other arm, on a two-handed grip, reaches for it with its elbow mirroring the first; and all of it
is projected into each view, so a thrust that is a straight
arm side-on is the same straight arm, foreshortened, from behind. A two-handed weapon puts the
other fist on the grip too.

The wrist is checked, not trusted: a fist holds a hilt across the palm, so the blade sits 70-80
degrees off the forearm at rest; a wrist snapped fully forward (a chop landing, a thrust) brings
it to 10-25 degrees, cocked back to about 110. Every frame outside that is printed.

Frame counts and where the strike lands match the sword clips the game already plays (game.rs
`phased` keys), so a type's clip drops in for the sword's.
"""
import json, math, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))

# Frames in each phase: windup, live, recovery, as the sword clips have them.
PHASES = {
    ("light", "east"): (3, 2, 4), ("heavy", "east"): (5, 2, 4),
    ("light", "north-east"): (5, 2, 2), ("heavy", "north-east"): (7, 2, 4),
    # The off hand's: a bow drawn and loosed, a focus thrust out.
    ("loose", "east"): (6, 2, 3), ("loose", "north-east"): (6, 2, 3),
    ("cast", "east"): (5, 2, 3), ("cast", "north-east"): (5, 2, 3),
}
# The road's straight-behind view times its blows as the north-east one did.
for (_m, _v), _p in list(PHASES.items()):
    if _v == "north-east": PHASES[(_m, "north")] = _p

# Arm bones and the fist, in image heights (from the reference frames: the arms hanging, the
# longer of the two views' drawings); the chest's width between the shoulders.
UPPER, FORE, FIST, CHEST = 0.147, 0.122, 0.028, 0.32
# The wrist's range, degrees between forearm and blade (see above); a rapier's grip, and a
# spear's underhand, run along the forearm, so their thrusts may line up exactly.
WRIST = {"thrust": (0.0, 115.0), "spear": (0.0, 115.0)}
WRIST_ANY = (8.0, 115.0)

# Per type: the second fist's place on the grip, along the blade from the first (None: one hand),
# and the moves as guard -> chamber (-> the smear, `mid`, where a swing passes over the top) ->
# strike (-> follow-through). Each key points the weapon
# arm's two bones (`upper`: shoulder to elbow, `fore`: elbow to wrist) and the blade, all (F, U, R).
# The arm is posed, not reached for, so the elbow is where the key puts it. From fencing and
# axe/longsword reference: an upright blade has a level forearm under it (a hilt crosses the
# palm), a thrust is one straight line from shoulder to point, a chop lands with the arm reaching
# forward and down and the haft nearly in line with the forearm, and a raised weapon lifts the
# elbow out to the side, never straight back.
TYPES = {
    # A sword in one hand: the light cuts flat across from behind the right shoulder to the left,
    # the heavy comes down from overhead, elbow out, the body stepping in behind it.
    "sword": {"off": None, "moves": {
        "light": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                  dict(upper=(-0.20, 0.10, 0.97), fore=(-0.30, 0.80, 0.50), blade=(-0.70, 0.40, 0.60), lean=-4, twist=-25),
                  dict(mid=True, upper=(0.40, 0.00, 0.90), fore=(0.90, 0.20, 0.40), blade=(0.70, 0.05, 0.70), lean=2, step=0.03, drop=0.01, twist=0),
                  dict(upper=(0.85, -0.10, -0.30), fore=(0.90, -0.05, -0.45), blade=(0.45, -0.05, -0.90), lean=8, step=0.05, drop=0.02, twist=25),
                  dict(upper=(0.40, -0.80, -0.45), fore=(0.30, -0.50, -0.80), blade=(0.30, -0.40, -0.85), lean=6, step=0.05, drop=0.02, twist=20)],
        "heavy": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                  dict(upper=(0.10, 0.80, 0.55), fore=(-0.30, 0.90, 0.10), blade=(-0.90, 0.30, 0.10), lean=-10, drop=0.02, twist=-15),
                  dict(mid=True, upper=(0.55, 0.65, 0.40), fore=(0.65, 0.75, 0.00), blade=(0.25, 0.97, 0.00), lean=0, step=0.05, drop=0.03, twist=2),
                  dict(upper=(0.90, -0.20, -0.10), fore=(0.85, -0.45, -0.10), blade=(0.50, -0.85, -0.15), lean=14, step=0.09, drop=0.05, twist=22),
                  dict(upper=(0.50, -0.85, -0.10), fore=(0.40, -0.90, -0.10), blade=(0.55, -0.80, -0.20), lean=12, step=0.09, drop=0.05, twist=18)],
    }},
    # A rapier: point always on the line; the arm straightens behind it and the body lunges.
    "thrust": {"off": None, "moves": {
        "light": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                  dict(upper=(-0.30, -0.90, 0.20), fore=(0.95, 0.15, -0.10), blade=(0.95, 0.15, -0.20), lean=-4, twist=-15),
                  dict(upper=(0.97, -0.05, -0.10), fore=(0.98, -0.02, -0.15), blade=(0.97, -0.02, -0.18), lean=10, step=0.08, drop=0.015, twist=25)],
        "heavy": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                  dict(upper=(-0.50, -0.60, 0.60), fore=(0.90, 0.30, -0.20), blade=(0.95, 0.05, -0.25), lean=-8, drop=0.02, twist=-25),
                  dict(upper=(0.97, -0.08, -0.10), fore=(0.98, -0.05, -0.15), blade=(0.97, -0.06, -0.18), lean=14, step=0.10, drop=0.05, twist=20)],
    }},
    # A spear: the right fist at the butt by the hip pushes, the left runs the shaft; the heavy
    # comes overhand from above the shoulder.
    "spear": {"off": 0.12, "moves": {
        "light": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                  dict(upper=(-0.45, -0.85, 0.25), fore=(0.95, 0.20, -0.20), blade=(0.95, 0.20, -0.25), lean=-4, twist=-15),
                  dict(upper=(0.70, -0.65, -0.10), fore=(0.95, -0.10, -0.20), blade=(0.97, 0.10, -0.20), lean=8, step=0.06, drop=0.01, twist=20)],
        "heavy": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                  dict(upper=(-0.30, 0.45, 0.85), fore=(0.30, 0.95, 0.00), blade=(0.95, -0.15, -0.25), lean=-8, twist=-25),
                  dict(upper=(0.85, 0.20, -0.10), fore=(0.90, -0.30, -0.20), blade=(0.95, -0.20, -0.20), lean=12, step=0.10, drop=0.04, twist=22)],
    }},
    # A cleaver: up over the shoulder, elbow out, then down across the body, the wrist snapping
    # straight as it lands.
    "axe": {"off": None, "moves": {
        "light": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                  dict(upper=(-0.20, 0.35, 0.90), fore=(-0.10, 0.95, 0.10), blade=(-0.75, 0.55, 0.30), lean=-6, twist=-25),
                  dict(mid=True, upper=(0.45, 0.45, 0.60), fore=(0.55, 0.80, -0.20), blade=(-0.15, 0.95, 0.20), lean=2, step=0.03, drop=0.01, twist=0),
                  dict(upper=(0.85, -0.35, -0.30), fore=(0.85, -0.35, -0.35), blade=(0.70, -0.55, -0.40), lean=10, step=0.05, drop=0.02, twist=25),
                  dict(upper=(0.45, -0.80, -0.40), fore=(0.35, -0.60, -0.70), blade=(0.75, -0.30, -0.60), lean=8, step=0.05, drop=0.02, twist=20)],
        "heavy": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                  dict(upper=(-0.10, 0.60, 0.75), fore=(-0.20, 0.95, 0.00), blade=(-0.80, 0.50, 0.00), lean=-12, twist=-20),
                  dict(mid=True, upper=(0.55, 0.65, 0.40), fore=(0.65, 0.75, 0.00), blade=(0.20, 0.98, 0.00), lean=2, step=0.05, drop=0.03, twist=5),
                  dict(upper=(0.90, -0.25, -0.10), fore=(0.85, -0.45, -0.10), blade=(0.60, -0.78, -0.10), lean=16, step=0.09, drop=0.06, twist=25),
                  dict(upper=(0.50, -0.85, -0.10), fore=(0.35, -0.93, -0.10), blade=(0.75, -0.60, -0.20), lean=14, step=0.09, drop=0.06, twist=20)],
    }},
    # A greatsword: the right fist under the guard, the left on the pommel. Middle guard; high
    # right (hands by the shoulder, forearms level, blade upright) for the diagonal; overhead with
    # the elbows out and the blade behind the head for the heavy.
    "greatsword": {"off": -0.045, "moves": {
        "light": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                  dict(upper=(0.10, -0.35, 0.93), fore=(0.45, 0.55, -0.70), blade=(-0.35, 0.92, 0.15), lean=-6, twist=-25),
                  dict(mid=True, upper=(0.60, -0.30, 0.60), fore=(0.75, 0.30, -0.60), blade=(0.50, 0.75, -0.40), lean=2, step=0.03, drop=0.01, twist=0),
                  dict(upper=(0.80, -0.45, -0.35), fore=(0.75, -0.35, -0.55), blade=(0.85, -0.30, -0.40), lean=10, step=0.06, drop=0.02, twist=25),
                  dict(upper=(0.35, -0.90, -0.25), fore=(0.40, -0.50, -0.75), blade=(0.65, -0.40, -0.65), lean=8, step=0.06, drop=0.02, twist=20)],
        "heavy": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                  dict(upper=(0.20, 0.60, 0.75), fore=(-0.10, 0.80, -0.60), blade=(-0.85, 0.15, 0.00), lean=-12, drop=0.03, twist=-15),
                  dict(mid=True, upper=(0.60, 0.55, 0.40), fore=(0.50, 0.75, -0.40), blade=(0.10, 0.99, 0.00), lean=0, step=0.05, drop=0.04, twist=2),
                  dict(upper=(0.90, -0.20, -0.30), fore=(0.85, -0.25, -0.45), blade=(0.95, -0.12, -0.15), lean=12, step=0.11, drop=0.05, twist=20),
                  dict(upper=(0.60, -0.75, -0.20), fore=(0.55, -0.60, -0.55), blade=(0.65, -0.70, -0.15), lean=10, step=0.11, drop=0.05, twist=15)],
    }},
    # An ultra greatsword, too big to swing from the wrists: the whole body turns it. The light
    # is a flat sweep at the hip from far back on the right round to the left; the heavy hauls
    # the blade up until it hangs down the back, then drives it into the ground ahead, the body
    # dropping behind it. The grip is longer, the fists further apart.
    "ultra": {"off": -0.06, "moves": {
        "light": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                  dict(upper=(-0.30, -0.55, 0.78), fore=(-0.45, 0.20, 0.87), blade=(-0.80, 0.15, 0.58), lean=-8, drop=0.03, twist=-35),
                  dict(mid=True, upper=(0.20, -0.45, 0.87), fore=(0.45, 0.10, 0.89), blade=(0.15, 0.05, 0.99), lean=0, step=0.04, drop=0.04, twist=-10),
                  dict(upper=(0.85, -0.35, -0.40), fore=(0.88, -0.15, -0.45), blade=(0.70, -0.05, -0.72), lean=10, step=0.08, drop=0.05, twist=30),
                  dict(upper=(0.45, -0.70, -0.55), fore=(0.25, -0.35, -0.90), blade=(-0.10, -0.25, -0.96), lean=8, step=0.08, drop=0.05, twist=35)],
        "heavy": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                  dict(upper=(0.05, 0.70, 0.71), fore=(-0.25, 0.85, -0.46), blade=(-0.90, -0.35, 0.10), lean=-16, drop=0.03, twist=-15),
                  dict(mid=True, upper=(0.55, 0.70, 0.40), fore=(0.45, 0.85, -0.30), blade=(0.05, 0.99, 0.00), lean=0, step=0.06, drop=0.04, twist=2),
                  dict(upper=(0.85, -0.45, -0.25), fore=(0.70, -0.65, -0.30), blade=(0.70, -0.70, -0.10), lean=20, step=0.13, drop=0.09, twist=18),
                  dict(upper=(0.65, -0.72, -0.20), fore=(0.50, -0.80, -0.30), blade=(0.60, -0.78, -0.15), lean=18, step=0.13, drop=0.09, twist=15)],
    }},
    # The off hand's moves, its arm posed bone by bone too (`lupper`, `lfore`, from the left
    # shoulder). A bow: the left arm out straight at the mark, the stave across it; the right hand
    # nocks at the grip (`pre`), draws to the cheek with the elbow out behind, and flies back past
    # the ear as it looses. The right hand holds the string, not the sword (`string`: its fist is
    # written to draw.txt, not hold.txt). The chest turns side-on to the mark, left shoulder in front.
    "bow": {"off": None, "string": True, "moves": {
        "loose": [dict(upper=(0.10, -0.99, 0.10), fore=(0.30, -0.95, 0.00), blade=(0.95, 0.25, -0.10), lupper=(0.25, -0.96, 0.00), lfore=(0.55, -0.83, 0.05)),
                  dict(pre=True, upper=(0.60, -0.55, -0.58), fore=(0.80, 0.30, -0.50), blade=(0.30, 0.95, 0.00), lupper=(0.85, -0.20, 0.45), lfore=(0.90, 0.20, 0.40), twist=-20),
                  dict(upper=(-0.15, 0.12, 0.98), fore=(0.85, 0.10, -0.52), blade=(0.30, 0.95, 0.00), lupper=(0.96, 0.10, 0.25), lfore=(0.97, 0.12, 0.20), twist=-40, lean=-2),
                  dict(upper=(-0.55, 0.20, 0.81), fore=(-0.20, 0.25, 0.95), blade=(0.30, 0.95, 0.00), lupper=(0.96, 0.10, 0.25), lfore=(0.97, 0.12, 0.20), twist=-40, lean=-2),
                  dict(upper=(-0.20, -0.70, 0.68), fore=(0.20, -0.80, 0.55), blade=(0.30, 0.95, 0.00), lupper=(0.85, -0.30, 0.30), lfore=(0.85, -0.20, 0.30), twist=-25)],
    }},
    # A focus in the left fist: pulled in to the chest, then thrust out at the mark, the sword
    # kept up in the right at guard the whole time.
    "focus": {"off": None, "moves": {
        "cast": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20), lupper=(0.20, -0.97, -0.10), lfore=(0.75, -0.60, 0.25)),
                 dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20), lupper=(-0.40, -0.85, -0.20), lfore=(0.60, 0.50, 0.60), twist=20, lean=-4),
                 dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20), lupper=(0.95, 0.05, 0.30), lfore=(0.96, 0.08, 0.26), twist=-15, lean=6, step=0.02),
                 dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20), lupper=(0.85, -0.30, 0.30), lfore=(0.90, -0.10, 0.30), twist=-10, lean=4, step=0.02)],
    }},
}

# The other arm's elbow, reaching for the grip, goes where the weapon arm's does, mirrored.

# Each view's camera. east: from the body's right, a little above. north-east: Pixellab's
# north-east facing seen by its camera, which looks north and down: the body faces up-right on
# screen and its right side is the nearer. `twist0` is how far the reference drawing's chest is
# already turned. FACING is where "forward" is on screen for the legs and the lean. north: the
# road's camera, straight behind and above: forward is up the screen, foreshortened, the body's
# right is screen right, and a lean tips the chest away from the camera rather than across it.
VIEWS = {
    "east": dict(s=0.12, twist0=-10.0),
    "north-east": dict(s=0.24, twist0=5.0),
    # Tilted as the road's camera looks down at the hero (eye 3.2 m up, 3.5 m back: about 40
    # degrees), so a blade driven forward into the ground foreshortens to a stub ahead of the
    # hands instead of hanging down the screen past the feet.
    "north": dict(s=0.6, twist0=0.0),
}
FACING = {"east": (1.0, 0.0), "north-east": (0.92, -0.22), "north": (0.0, -0.6)}
K = math.sqrt(0.5)


def proj(view, v):
    """A 3D offset (F, U, R) on screen (x right, y down)."""
    f, u, r = v
    s = VIEWS[view]["s"]; c = math.sqrt(1 - s * s)
    if view == "east":
        return (f, -u * c + r * s)
    if view == "north":
        return (r, -u * c - f * s)
    return (K * (f + r), -u * c + K * s * (r - f))


def depth(view, v):
    """How much nearer the camera a 3D offset is."""
    f, u, r = v
    s = VIEWS[view]["s"]; c = math.sqrt(1 - s * s)
    if view == "east":
        return r * c + u * s
    if view == "north":
        return -f * c + u * s
    return K * c * (r - f) + u * s


def dn(v):
    """Straight behind: Pixellab's `depth` (0..255, 128 the body's middle, higher nearer the
    camera) of a joint at 3D offset `v` from the body's middle, by how far in front of the body
    (hidden from behind) or behind it it is, not its height. A draw order alone does not keep a
    fist held before the chest off the drawn back; this does."""
    d = depth("north", (v[0], 0.0, v[2]))
    return int(max(30, min(225, 128 + 420 * d)))


def zn(v):
    """Straight behind: the draw order of a joint at 3D offset `v` from its shoulder, by how far
    in front of the body (hidden) or behind it (over) it is; its height does not count, since the
    torso goes down with it (the torso is drawn at 5 and 6)."""
    d = depth("north", (v[0], 0.0, v[2]))
    return 8 if d > 0.03 else 3 if d < -0.03 else 5


# How much of an authored lean the body takes (a hinge at the hips over-tips it).
LEAN = 0.6


def load_base(view):
    return {k: list(v) for k, v in json.load(open(os.path.join(HERE, f"base_{view}.json"))).items()}


def ease(t):
    return t * t * (3 - 2 * t)


def lerp(a, b, t):
    return a + (b - a) * t


def add(a, b): return tuple(x + y for x, y in zip(a, b))
def sub(a, b): return tuple(x - y for x, y in zip(a, b))
def mul(a, k): return tuple(x * k for x in a)
def dot(a, b): return sum(x * y for x, y in zip(a, b))
def norm(a): return math.sqrt(dot(a, a))
def unit(a): n = norm(a); return mul(a, 1 / n) if n > 1e-9 else a


def slerp(a, b, t):
    a, b = unit(a), unit(b)
    w = math.acos(max(-1.0, min(1.0, dot(a, b))))
    if w < 1e-4:
        return a
    return unit(add(mul(a, math.sin((1 - t) * w) / math.sin(w)), mul(b, math.sin(t * w) / math.sin(w))))


def blend(p, q, t):
    """Pose between p and q: each bone and the blade turn (arcs, as a swing does)."""
    out = {k: lerp(p.get(k, 0.0), q.get(k, 0.0), t) for k in ("lean", "step", "drop", "twist")}
    for k in ("upper", "fore", "blade"):
        out[k] = slerp(p[k], q[k], t)
    for k in ("lupper", "lfore"):
        if k in p and k in q: out[k] = slerp(p[k], q[k], t)
    return out


def timeline(keys, phases):
    """One pose per frame: windup guard->chamber, live chamber->strike (over the smear when
    there is one), recovery back to guard (through the follow-through when there is one)."""
    w, a, r = phases
    m = next((k for k in keys if k.get("mid")), None)
    pre = next((k for k in keys if k.get("pre")), None)
    keys = [k for k in keys if not k.get("mid") and not k.get("pre")]
    g, c, s = keys[0], keys[1], keys[2]
    f = keys[3] if len(keys) > 3 else None
    # A windup through `pre` (the bow raised and nocked) spends half its frames on each leg.
    wind = lambda t: blend(g, c, ease(t)) if pre is None else blend(g, pre, ease(2 * t)) if t <= 0.5 else blend(pre, c, ease(2 * t - 1))
    out = [wind(i / (w - 1)) for i in range(w)]
    for i in range(a):
        t = (i + 1) / a
        out.append(blend(c, s, t) if m is None else blend(c, m, 2 * t) if t <= 0.5 else blend(m, s, 2 * t - 1))
    for i in range(r):
        t = (i + 1) / r
        if f is None:
            out.append(blend(s, g, ease(t)))
        elif t <= 0.4:
            out.append(blend(s, f, t / 0.4))
        else:
            out.append(blend(f, g, ease((t - 0.4) / 0.6)))
    return out


def solve_arm(target, pole):
    """Elbow and wrist (from the shoulder at the origin) for a fist closing on `target`: the
    wrist a fist's length short of it along the forearm, the elbow bent toward `pole`. Out of
    reach, the arm straightens toward it."""
    wrist = sub(target, mul(unit(target), FIST))
    for _ in range(6):
        d = norm(wrist)
        axis = unit(wrist)
        d = max(min(d, UPPER + FORE - 1e-4), abs(UPPER - FORE) + 1e-4)
        x = (UPPER * UPPER - FORE * FORE + d * d) / (2 * d)
        h = math.sqrt(max(0.0, UPPER * UPPER - x * x))
        side = unit(sub(pole, mul(axis, dot(pole, axis))))
        elbow = add(mul(axis, x), mul(side, h))
        fore = unit(sub(wrist, elbow))
        wrist = sub(target, mul(fore, FIST))
    wrist = add(elbow, mul(fore, FORE))
    return elbow, wrist, fore


def rot2(p, c, deg):
    s, co = math.sin(math.radians(deg)), math.cos(math.radians(deg))
    x, y = p[0] - c[0], p[1] - c[1]
    return (c[0] + co * x - s * y, c[1] + s * x + co * y)


def chest(twist):
    """The right shoulder and the left, from the spine, with the chest turned `twist` degrees."""
    a = math.radians(twist)
    return (CHEST / 2 * math.sin(a), 0.0, CHEST / 2 * math.cos(a)), (-CHEST / 2 * math.sin(a), 0.0, -CHEST / 2 * math.cos(a))


def pose(base, view, p, off):
    """The 18 joints and the weapon's grip, angle, layer and reach for pose p."""
    fx, fy = FACING[view]
    j = {k: list(v) for k, v in base.items()}
    hips = ((j["RIGHT HIP"][0] + j["LEFT HIP"][0]) / 2, (j["RIGHT HIP"][1] + j["LEFT HIP"][1]) / 2)
    length = lambda a, b: math.hypot(base[a][0] - base[b][0], base[a][1] - base[b][1])
    leg = {s: (length(f"{s} HIP", f"{s} KNEE"), length(f"{s} KNEE", f"{s} LEG")) for s in ("RIGHT", "LEFT")}
    step, drop, lean = p["step"], p["drop"], p["lean"] * LEAN
    # Legs: the weapon-side foot leads, the other pushes back; the hips follow between and sink.
    shift = (fx * step * 0.45, fy * step * 0.45 + drop)
    for side, k in (("RIGHT", 1.0), ("LEFT", -0.35)):
        ank = j[f"{side} LEG"]
        ank[0] += fx * step * k; ank[1] += fy * step * k
        hip = j[f"{side} HIP"]
        hip[0] += shift[0]; hip[1] += shift[1]
        # Seen from behind, a bending knee goes out to its own side.
        bend = (-1 if side == "RIGHT" else 1) if view == "north" else -1 if fx >= 0 else 1
        kn, an = leg2d(hip, ank, *leg[side], bend=bend)
        j[f"{side} KNEE"][:2], j[f"{side} LEG"][:2] = list(kn), list(an)
    # The upper body rides on the hips and leans about them (toward the facing).
    upper = ["NOSE", "RIGHT EYE", "LEFT EYE", "RIGHT EAR", "LEFT EAR", "NECK", "RIGHT SHOULDER", "LEFT SHOULDER"]
    for k in upper:
        q = (j[k][0] + shift[0], j[k][1] + shift[1])
        if view == "north":
            # Tipped away from the camera: the height above the hips shortens and climbs the
            # screen by the camera's tilt; tipped back toward it, it drops.
            s = VIEWS[view]["s"]; h = hips[1] + shift[1] - q[1]; a = math.radians(lean)
            j[k][0], j[k][1] = q[0], hips[1] + shift[1] - h * math.cos(a) - s * h * math.sin(a)
        else:
            j[k][0], j[k][1] = rot2(q, (hips[0] + shift[0], hips[1] + shift[1]), lean * (1 if fx >= 0 else -1))
    # The chest turns into the blow: each shoulder moves by the turn from the drawing's own.
    t0 = VIEWS[view]["twist0"]
    (r0, l0), (r1, l1) = chest(t0), chest(t0 + p["twist"])
    for side, a, b in (("RIGHT", r0, r1), ("LEFT", l0, l1)):
        dx, dy = proj(view, sub(b, a))
        j[f"{side} SHOULDER"][0] += dx; j[f"{side} SHOULDER"][1] += dy
    # Everything from here is from the right shoulder, in 3D.
    sh = tuple(j["RIGHT SHOULDER"][:2])
    left_sh = sub(l1, r1)
    at = lambda v: add(sh, proj(view, v))
    blade = unit(p["blade"])
    e = mul(unit(p["upper"]), UPPER)
    fore = unit(p["fore"])
    w = add(e, mul(fore, FORE))
    grip = add(w, mul(fore, FIST))
    bend = math.degrees(math.acos(max(-1.0, min(1.0, dot(fore, blade)))))
    j["RIGHT ELBOW"][:2], j["RIGHT ARM"][:2] = list(at(e)), list(at(w))
    torso = mul(add(r1, l1), 0.5)
    torso = sub(torso, r1)
    near = lambda v: depth(view, v) - depth(view, torso)
    j["RIGHT ELBOW"][2] = 8 if near(e) > -0.03 else 4
    j["RIGHT ARM"][2] = 8 if near(w) > -0.03 else 4
    if "lupper" in p:
        # The off arm posed for itself, from the left shoulder.
        e2 = mul(unit(p["lupper"]), UPPER)
        w2 = add(e2, mul(unit(p["lfore"]), FORE))
        lsh = tuple(j["LEFT SHOULDER"][:2])
        j["LEFT ELBOW"][:2] = list(add(lsh, proj(view, e2)))
        j["LEFT ARM"][:2] = list(add(lsh, proj(view, w2)))
        j["LEFT ELBOW"][2] = 8 if near(add(left_sh, e2)) > -0.03 else 4
        j["LEFT ARM"][2] = 8 if near(add(left_sh, w2)) > -0.03 else 4
    elif off is not None:
        g2 = add(grip, mul(blade, off))
        e2, w2, _ = solve_arm(sub(g2, left_sh), (e[0], e[1], -e[2]))
        # The drawing's shoulders are not quite where the camera puts them; the fist goes to the
        # grip all the same, the elbow takes half the difference.
        lsh = tuple(j["LEFT SHOULDER"][:2])
        miss = sub(sub(at(left_sh), lsh), (0.0, 0.0))
        j["LEFT ELBOW"][:2] = list(add(add(lsh, proj(view, e2)), mul(miss, 0.5)))
        j["LEFT ARM"][:2] = list(add(add(lsh, proj(view, w2)), miss))
        if view == "north":
            j["LEFT ELBOW"][2], j["LEFT ARM"][2] = zn(e2), zn(w2)
    gx, gy = at(grip)
    bx, by = proj(view, blade)
    over = near(add(grip, mul(blade, 0.1))) >= -0.02
    joints = [{"label": k, "x": round(v[0], 4), "y": round(v[1], 4), "z_index": v[2]} for k, v in j.items()]
    if view == "north":
        # Depth for the arms from the pose (from the body's middle), the rest at the body's.
        mid = mul(add(r1, l1), 0.5)
        dep = {"RIGHT ELBOW": dn(sub(add(r1, e), mid)), "RIGHT ARM": dn(sub(add(r1, w), mid))}
        if "lupper" in p:
            dep["LEFT ELBOW"], dep["LEFT ARM"] = dn(sub(add(l1, e2), mid)), dn(sub(add(l1, w2), mid))
        elif off is not None:
            dep["LEFT ELBOW"], dep["LEFT ARM"] = dn(sub(add(l1, e2), mid)), dn(sub(add(l1, w2), mid))
        for d in joints: d["depth"] = dep.get(d["label"], 128)
    ex, ey = at(e)
    return joints, {"grip": [round(gx, 4), round(gy, 4)], "angle": round(math.degrees(math.atan2(by, bx)), 1), "elbow": [round(ex, 4), round(ey, 4)],
                    "over": over, "reach": round(max(0.35, math.hypot(bx, by)), 2), "wrist": round(bend)}


def leg2d(hip, target, upper, fore, bend):
    """Knee and ankle reaching for target from hip, in the drawing's plane; bend picks the side."""
    dx, dy = target[0] - hip[0], target[1] - hip[1]
    d = math.hypot(dx, dy)
    d = max(min(d, upper + fore - 1e-4), abs(upper - fore) + 1e-4)
    a = math.atan2(dy, dx)
    k = math.acos(max(-1.0, min(1.0, (upper * upper + d * d - fore * fore) / (2 * upper * d))))
    return ((hip[0] + upper * math.cos(a + bend * k), hip[1] + upper * math.sin(a + bend * k)),
            (hip[0] + d * math.cos(a), hip[1] + d * math.sin(a)))


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    kinds = sys.argv[2:] or list(TYPES)
    for view in VIEWS:
        base = load_base(view)
        first = [{"label": k, "x": v[0], "y": v[1], "z_index": v[2]} for k, v in base.items()]
        json.dump(first, open(os.path.join(out, f"{view}_first.json"), "w"))
        for kind in kinds:
            spec = TYPES[kind]
            for move, keys in spec["moves"].items():
                frames = timeline(keys, PHASES[(move, view)])
                skel, track = zip(*(pose(base, view, p, spec["off"]) for p in frames))
                name = f"{kind}_{move}_{view}"
                json.dump(list(skel), open(os.path.join(out, name + ".json"), "w"))
                json.dump(list(track), open(os.path.join(out, name + ".weapon.json"), "w"), indent=1)
                # The game's copy (anim.rs `read_holds`): x y angle over reach ex ey, a frame a
                # line; the game moves the grip to the fist as drawn (`fit_hold`), from the elbow.
                # A bow's right hand holds the string: its track is the draw hand's, not a grip.
                with open(os.path.join(out, name + (".draw.txt" if spec.get("string") else ".hold.txt")), "w") as f:
                    for t in track: f.write(f"{t['grip'][0]} {t['grip'][1]} {t['angle']} {int(t['over'])} {t['reach']} {t['elbow'][0]} {t['elbow'][1]}\n")
                lo, hi = WRIST.get(kind, WRIST_ANY) if not spec.get("string") else (0.0, 180.0)
                bad = [f"{i}:{t['wrist']}" for i, t in enumerate(track) if not lo <= t["wrist"] <= hi]
                print(f"{name:28} {len(skel):2} frames  wrist {' '.join(str(t['wrist']) for t in track)}" + (f"  OUT OF RANGE {' '.join(bad)}" if bad else ""))


if __name__ == "__main__":
    main()
