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

A move may carry its own keys for one view as `<move>@<view>` (`"heavy@north"`), for a blow that
needs another pose to read from that camera. From the road's, a blow meets the foe dead ahead: the
weapon points straight forward at the hit (its sideways part near 0), the fist in before the chest
(within ~0.07 of the neck one-handed, ~0.03 two-handed; the shoulder is at 0.14), and the body may
hide most of it. The drawing after carries on at most ~0.1 aside; a cut whose blade then points into
the screen drops low ahead instead, since a short stub shows only its lean and reads as sideways.
A blow carried out to either side reads as striking something beside the hero. Side-on, a chest
turned past ~25 degrees faces the camera.

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
    # A finisher (`combo::Route::Finisher`) is timed as the heavy it stands in for, as are the
    # breaker and the follow-through; a string's third strike as a light.
    ("finisher", "east"): (5, 2, 4), ("finisher", "north-east"): (7, 2, 4),
    ("breaker", "east"): (5, 2, 4), ("breaker", "north-east"): (7, 2, 4),
    ("follow", "east"): (5, 2, 4), ("follow", "north-east"): (7, 2, 4),
    ("third", "east"): (3, 2, 4), ("third", "north-east"): (5, 2, 2),
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
# The head's middle above the shoulders (3D), and how near it a grip lies on screen to be over it.
HEAD, HEAD_SEEN, HILT = 0.2, 0.085, 0.1
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
        # The finisher, a wheeling cut: wound far back over the right shoulder, the body coiled
        # low, then the blade carried flat round, the wrist trailing; it meets the foe pointing
        # at it (a flat cut lands with the blade ahead, not across) and carries on round to the
        # back-left, the feet driving in behind. The chest turns no more than 50 degrees: further,
        # and from behind its shoulders close up until the drawing reads as facing the camera.
        "finisher": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                     dict(upper=(-0.45, 0.30, 0.84), fore=(-0.50, 0.50, 0.71), blade=(-0.85, 0.20, 0.49), lean=-8, drop=0.05, twist=-50),
                     dict(mid=True, upper=(0.30, 0.05, 0.95), fore=(0.65, 0.10, 0.75), blade=(0.40, 0.05, 0.92), lean=4, step=0.05, drop=0.06, twist=-10),
                     dict(upper=(0.90, -0.05, -0.43), fore=(0.93, -0.02, -0.37), blade=(0.80, -0.03, -0.60), lean=10, step=0.11, drop=0.07, twist=30),
                     dict(upper=(0.35, -0.30, -0.89), fore=(0.05, -0.10, -0.99), blade=(-0.55, -0.10, -0.83), lean=8, step=0.11, drop=0.06, twist=50)],
        # The third strike of a string, a rising cut: from low behind the right hip up across the front to high over the left shoulder.
        "third": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                 dict(upper=(-0.2, -0.85, 0.5), fore=(-0.4, -0.5, 0.75), blade=(-0.5, -0.6, 0.6), lean=-2, drop=0.03, twist=-25),
                 dict(upper=(0.6, -0.6, 0.5), fore=(0.85, -0.25, 0.45), blade=(0.5, 0.15, 0.85), mid=True, lean=2, step=0.03, drop=0.02, twist=0),
                 dict(upper=(0.6, 0.6, -0.5), fore=(0.4, 0.85, -0.35), blade=(0.2, 0.9, -0.4), lean=6, step=0.05, drop=0.01, twist=25),
                 dict(upper=(0.3, 0.85, -0.4), fore=(-0.1, 0.95, -0.3), blade=(-0.5, 0.7, -0.5), lean=4, step=0.05, twist=30)],
        # A breaker, a lunging thrust: the arm drawn back with the point on the line, then driven straight through the guard.
        "breaker": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                   dict(upper=(-0.5, -0.6, 0.6), fore=(0.9, 0.3, -0.2), blade=(0.95, 0.1, -0.25), lean=-6, drop=0.02, twist=-25),
                   dict(upper=(0.97, -0.05, -0.1), fore=(0.98, -0.02, -0.15), blade=(0.9, 0.25, -0.35), lean=14, step=0.12, drop=0.06, twist=25),
                   dict(upper=(0.9, -0.3, -0.15), fore=(0.92, -0.2, -0.2), blade=(0.9, -0.05, -0.3), lean=12, step=0.12, drop=0.06, twist=22)],
        # A follow-through, a backhand cut: the blade wrapped back over the left shoulder, then carried flat across to the right.
        "follow": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                  dict(upper=(0.5, 0.3, -0.8), fore=(-0.3, 0.4, -0.85), blade=(-0.7, 0.3, -0.6), lean=-4, drop=0.02, twist=35),
                  dict(upper=(0.5, -0.1, 0.85), fore=(0.7, 0.05, 0.7), blade=(0.45, 0.05, 0.9), lean=8, step=0.06, drop=0.03, twist=-25),
                  dict(upper=(0.0, -0.3, 0.95), fore=(-0.4, -0.1, 0.9), blade=(-0.6, -0.1, 0.8), lean=6, step=0.06, drop=0.03, twist=-40)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        # Its breaker's point is driven down into the guard behind a deep lunge, so it shortens as it goes.
        "breaker@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                          dict(upper=(-0.10, -0.65, 0.75), fore=(0.25, 0.90, 0.35), blade=(0.95, 0.25, 0.15), lean=-6, drop=0.02, twist=-25),
                          dict(upper=(0.91, -0.15, -0.18), fore=(0.94, 0.00, -0.24), blade=(0.97, -0.12, 0.00), lean=20, step=0.16, drop=0.1, twist=15),
                          dict(upper=(0.89, -0.30, -0.03), fore=(0.95, 0.05, -0.10), blade=(0.96, -0.16, 0.02), lean=18, step=0.16, drop=0.1, twist=12)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "heavy@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                       dict(upper=(0.10, 0.80, 0.55), fore=(-0.30, 0.90, 0.10), blade=(-0.90, 0.30, 0.10), lean=-10, drop=0.02, twist=-15),
                       dict(mid=True, upper=(0.55, 0.65, 0.40), fore=(0.65, 0.75, 0.00), blade=(0.25, 0.97, 0.00), lean=0, step=0.05, drop=0.03, twist=2),
                       dict(upper=(0.89, -0.15, -0.12), fore=(0.91, -0.30, -0.28), blade=(0.95, -0.05, 0.00), lean=14, step=0.09, drop=0.05, twist=22),
                       dict(upper=(0.60, -0.75, -0.17), fore=(0.53, -0.80, -0.29), blade=(0.41, -0.89, -0.05), lean=12, step=0.09, drop=0.05, twist=18)],
        # Side-on the third rises forward across the screen; the chest stays side-on.
        "third@east": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                       dict(upper=(-0.2, -0.85, 0.5), fore=(-0.4, -0.5, 0.75), blade=(-0.5, -0.6, 0.6), lean=-2, drop=0.03, twist=-20),
                       dict(upper=(0.70, -0.55, 0.45), fore=(0.90, -0.30, 0.30), blade=(0.70, -0.30, 0.40), mid=True, lean=2, step=0.03, drop=0.02, twist=0),
                       dict(upper=(0.75, 0.35, -0.35), fore=(0.70, 0.55, -0.25), blade=(0.62, 0.75, -0.20), lean=6, step=0.05, drop=0.01, twist=15),
                       dict(upper=(0.45, 0.75, -0.40), fore=(0.25, 0.92, -0.30), blade=(0.00, 0.95, -0.30), lean=4, step=0.05, twist=20)],
        # Side-on the follow-through cuts down across the screen with the chest kept side-on.
        "follow@east": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                        dict(upper=(0.5, 0.3, -0.8), fore=(-0.3, 0.4, -0.85), blade=(-0.75, 0.45, -0.45), lean=-4, drop=0.02, twist=20),
                        dict(upper=(0.80, -0.15, 0.55), fore=(0.85, -0.10, 0.50), blade=(0.85, -0.25, 0.45), lean=8, step=0.06, drop=0.03, twist=-15),
                        dict(upper=(0.45, -0.45, 0.75), fore=(0.40, -0.40, 0.82), blade=(0.55, -0.45, 0.70), lean=6, step=0.06, drop=0.03, twist=-20)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "follow@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                        dict(upper=(0.50, 0.30, -0.80), fore=(-0.30, 0.40, -0.85), blade=(-0.70, 0.30, -0.60), lean=-4, drop=0.02, twist=35),
                        dict(upper=(0.53, -0.25, -0.73), fore=(0.68, 0.00, -0.59), blade=(0.97, 0.05, 0.00), lean=8, step=0.06, drop=0.03, twist=-15),
                        dict(upper=(0.58, -0.30, -0.55), fore=(0.86, -0.05, -0.34), blade=(0.97, -0.10, 0.10), lean=6, step=0.06, drop=0.03, twist=-22)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "light@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                       dict(upper=(-0.20, 0.10, 0.97), fore=(-0.30, 0.80, 0.50), blade=(-0.70, 0.40, 0.60), lean=-4, twist=-25),
                       dict(mid=True, upper=(0.40, 0.00, 0.90), fore=(0.90, 0.20, 0.40), blade=(0.70, 0.05, 0.70), lean=2, step=0.03, drop=0.01, twist=0),
                       dict(upper=(0.90, -0.10, -0.17), fore=(0.96, -0.05, -0.28), blade=(0.75, -0.70, 0.00), lean=8, step=0.05, drop=0.02, twist=25),
                       dict(upper=(0.59, -0.80, 0.04), fore=(0.82, -0.50, -0.25), blade=(0.43, -0.88, -0.10), lean=6, step=0.05, drop=0.02, twist=20)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "finisher@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                          dict(upper=(-0.45, 0.30, 0.84), fore=(-0.50, 0.50, 0.71), blade=(-0.85, 0.20, 0.49), lean=-8, drop=0.05, twist=-50),
                          dict(mid=True, upper=(0.30, 0.05, 0.95), fore=(0.65, 0.10, 0.75), blade=(0.40, 0.05, 0.92), lean=4, step=0.05, drop=0.06, twist=-10),
                          dict(upper=(0.98, -0.05, -0.20), fore=(0.99, -0.02, -0.14), blade=(0.99, -0.17, 0.00), lean=10, step=0.11, drop=0.07, twist=30),
                          dict(upper=(0.92, -0.30, 0.27), fore=(0.99, -0.10, -0.03), blade=(0.95, -0.31, -0.10), lean=8, step=0.11, drop=0.06, twist=50)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "third@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.75, 0.62, -0.20)),
                       dict(upper=(-0.20, -0.85, 0.50), fore=(-0.40, -0.50, 0.75), blade=(-0.58, -0.60, 0.52), lean=-2, drop=0.03, twist=-25),
                       dict(upper=(0.60, -0.60, 0.50), fore=(0.85, -0.25, 0.45), blade=(0.50, 0.15, 0.85), mid=True, lean=2, step=0.03, drop=0.02, twist=0),
                       dict(upper=(0.75, 0.60, -0.21), fore=(0.50, 0.85, -0.19), blade=(0.45, 0.90, 0.00), lean=6, step=0.05, drop=0.01, twist=25),
                       dict(upper=(0.51, 0.85, 0.09), fore=(0.20, 0.95, -0.23), blade=(0.67, 0.70, -0.10), lean=4, step=0.05, twist=30)],
    }},
    # A rapier: point always on the line; the arm straightens behind it and the body lunges.
    "thrust": {"off": None, "moves": {
        "light": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                  dict(upper=(-0.30, -0.90, 0.20), fore=(0.95, 0.15, -0.10), blade=(0.95, 0.15, -0.20), lean=-4, twist=-15),
                  dict(upper=(0.97, -0.05, -0.10), fore=(0.98, -0.02, -0.15), blade=(0.97, -0.02, -0.18), lean=10, step=0.08, drop=0.015, twist=25)],
        "heavy": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                  dict(upper=(-0.50, -0.60, 0.60), fore=(0.90, 0.30, -0.20), blade=(0.95, 0.05, -0.25), lean=-8, drop=0.02, twist=-25),
                  dict(upper=(0.97, -0.08, -0.10), fore=(0.98, -0.05, -0.15), blade=(0.97, -0.06, -0.18), lean=14, step=0.10, drop=0.05, twist=20)],
        # The third strike of a string, a flicked cut: the hand up by the right ear, then the edge snapped down across to the low left.
        "third": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                 dict(upper=(0.1, 0.3, 0.95), fore=(-0.2, 0.95, 0.2), blade=(-0.6, 0.6, 0.5), lean=-4, twist=-20),
                 dict(upper=(0.8, -0.3, -0.5), fore=(0.85, -0.35, -0.4), blade=(0.6, -0.5, -0.6), lean=8, step=0.06, drop=0.02, twist=25),
                 dict(upper=(0.6, -0.6, -0.5), fore=(0.5, -0.6, -0.6), blade=(0.4, -0.6, -0.7), lean=6, step=0.06, drop=0.02, twist=22)],
        # A breaker, a lunge under the guard: dropped almost to the knee and thrust up from below.
        "breaker": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                   dict(upper=(-0.3, -0.9, 0.3), fore=(0.9, 0.2, -0.3), blade=(0.95, 0.15, -0.25), lean=-6, drop=0.03, twist=-20),
                   dict(upper=(0.9, 0.2, -0.3), fore=(0.95, 0.2, -0.2), blade=(0.9, 0.35, -0.25), lean=25, step=0.2, drop=0.16, twist=20),
                   dict(upper=(0.85, 0.1, -0.4), fore=(0.93, 0.15, -0.3), blade=(0.9, 0.25, -0.3), lean=22, step=0.2, drop=0.15, twist=18)],
        # A follow-through, a backhand rising cut: from the left hip up across to high on the right.
        "follow": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                  dict(upper=(0.6, -0.6, -0.5), fore=(0.2, -0.6, -0.75), blade=(-0.3, -0.6, -0.7), lean=-2, drop=0.03, twist=35),
                  dict(upper=(0.4, 0.5, 0.75), fore=(0.5, 0.75, 0.4), blade=(0.3, 0.85, 0.4), lean=4, step=0.05, twist=-25),
                  dict(upper=(0.1, 0.7, 0.7), fore=(0.1, 0.9, 0.4), blade=(-0.2, 0.85, 0.45), lean=2, step=0.05, twist=-30)],
        # The finisher, an overhand plunging thrust: the hand raised above the head, the point tipped down at the foe, then driven down and through with the whole body.
        "finisher": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                    dict(upper=(0.2, 0.85, 0.45), fore=(0.6, 0.75, 0.0), blade=(0.85, -0.45, -0.2), lean=-8, drop=-0.02, twist=-20),
                    dict(upper=(0.9, 0.3, -0.15), fore=(0.95, 0.1, -0.2), blade=(0.9, -0.35, -0.2), lean=18, step=0.16, drop=0.08, twist=25),
                    dict(upper=(0.85, 0.15, -0.25), fore=(0.92, 0.0, -0.25), blade=(0.88, -0.4, -0.25), lean=16, step=0.16, drop=0.08, twist=22)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "heavy@north": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                       dict(upper=(-0.10, -0.65, 0.75), fore=(0.30, 0.88, 0.37), blade=(0.85, 0.48, 0.20), lean=-8, drop=0.02, twist=-25),
                       dict(upper=(0.91, -0.14, -0.18), fore=(0.94, -0.02, -0.24), blade=(0.97, 0.00, 0.00), lean=14, step=0.1, drop=0.05, twist=15)],
        "follow@east": [dict(upper=(0.5, -0.85, 0.1), fore=(0.95, 0.2, -0.1), blade=(0.9, 0.3, -0.2)),
                        dict(upper=(0.6, -0.6, -0.5), fore=(0.2, -0.6, -0.75), blade=(-0.45, -0.6, -0.65), lean=-2, drop=0.03, twist=20),
                        dict(upper=(0.70, 0.40, 0.30), fore=(0.70, 0.62, 0.30), blade=(0.70, 0.65, 0.25), lean=4, step=0.05, twist=-12),
                        dict(upper=(0.40, 0.78, 0.40), fore=(0.30, 0.90, 0.30), blade=(0.25, 0.93, 0.25), lean=2, step=0.05, twist=-15)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "follow@north": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                        dict(upper=(0.60, -0.60, -0.50), fore=(0.20, -0.60, -0.75), blade=(-0.30, -0.60, -0.70), lean=-2, drop=0.03, twist=35),
                        dict(upper=(0.72, -0.05, -0.23), fore=(0.79, 0.35, -0.31), blade=(0.83, 0.45, 0.00), lean=4, step=0.05, twist=-15),
                        dict(upper=(0.60, 0.40, -0.03), fore=(0.49, 0.80, -0.14), blade=(0.42, 0.90, 0.10), lean=2, step=0.05, twist=-20)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "third@north": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                       dict(upper=(0.10, 0.30, 0.95), fore=(-0.20, 0.95, 0.20), blade=(-0.60, 0.60, 0.50), lean=-4, twist=-20),
                       dict(upper=(0.91, -0.30, -0.24), fore=(0.93, -0.35, -0.13), blade=(0.85, -0.50, 0.00), lean=8, step=0.06, drop=0.02, twist=25),
                       dict(upper=(0.78, -0.60, -0.00), fore=(0.77, -0.60, -0.14), blade=(0.80, -0.60, -0.01), lean=6, step=0.06, drop=0.02, twist=22)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        # Its breaker's point is driven down into the guard behind a deep lunge, so it shortens as it goes.
        "breaker@north": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                          dict(upper=(-0.30, -0.90, 0.30), fore=(0.90, 0.20, -0.30), blade=(0.95, 0.15, -0.25), lean=-6, drop=0.03, twist=-20),
                          dict(upper=(0.91, 0.20, -0.30), fore=(0.95, 0.20, -0.19), blade=(0.97, -0.10, 0.00), lean=25, step=0.2, drop=0.16, twist=20),
                          dict(upper=(0.89, 0.10, -0.30), fore=(0.96, 0.15, -0.19), blade=(0.96, -0.14, -0.05), lean=22, step=0.2, drop=0.15, twist=18)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "finisher@north": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                          dict(upper=(0.20, 0.85, 0.45), fore=(0.60, 0.75, 0.00), blade=(0.85, -0.45, -0.20), lean=-8, drop=-0.02, twist=-20),
                          dict(upper=(0.89, 0.30, -0.20), fore=(0.94, 0.10, -0.24), blade=(0.92, -0.35, 0.00), lean=18, step=0.16, drop=0.08, twist=25),
                          dict(upper=(0.86, 0.15, -0.20), fore=(0.93, 0.00, -0.20), blade=(0.91, -0.40, -0.10), lean=16, step=0.16, drop=0.08, twist=22)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "light@north": [dict(upper=(0.50, -0.85, 0.10), fore=(0.95, 0.20, -0.10), blade=(0.90, 0.30, -0.20)),
                       dict(upper=(-0.30, -0.90, 0.20), fore=(0.95, 0.15, -0.10), blade=(0.95, 0.15, -0.20), lean=-4, twist=-15),
                       dict(upper=(0.95, -0.05, -0.20), fore=(0.96, -0.02, -0.25), blade=(0.99, -0.02, 0.00), lean=10, step=0.08, drop=0.015, twist=25)],
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
        # The third strike of a string, a rising thrust: the point dropped to the knee, then driven up and forward.
        "third": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                 dict(upper=(-0.4, -0.9, 0.2), fore=(0.85, -0.3, -0.4), blade=(0.85, -0.35, -0.4), lean=-4, drop=0.05, twist=-15),
                 dict(upper=(0.75, 0.3, -0.2), fore=(0.9, 0.35, -0.2), blade=(0.85, 0.45, -0.25), lean=6, step=0.08, twist=20),
                 dict(upper=(0.7, 0.2, -0.3), fore=(0.88, 0.25, -0.3), blade=(0.85, 0.35, -0.3), lean=5, step=0.08, twist=18)],
        # A breaker, a shaft bash: the shaft pulled level across the chest, then both arms drive it into the guard.
        "breaker": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                   dict(upper=(-0.3, -0.8, 0.5), fore=(0.6, 0.3, -0.75), blade=(0.15, 0.1, -0.98), lean=-6, drop=0.02, twist=-10),
                   dict(upper=(0.9, -0.2, 0.0), fore=(0.95, 0.05, -0.3), blade=(0.25, 0.1, -0.96), lean=16, step=0.12, drop=0.05, twist=10),
                   dict(upper=(0.8, -0.4, 0.0), fore=(0.92, -0.05, -0.35), blade=(0.25, 0.05, -0.96), lean=14, step=0.12, drop=0.05, twist=8)],
        # A follow-through, a chop with the head: the spear raised over the right shoulder, then brought down across to the low left.
        "follow": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                  dict(upper=(0.1, 0.8, 0.6), fore=(-0.1, 0.95, 0.3), blade=(-0.5, 0.7, 0.5), lean=-10, twist=-20),
                  dict(upper=(0.85, -0.2, -0.4), fore=(0.8, -0.45, -0.4), blade=(0.6, -0.6, -0.5), lean=14, step=0.09, drop=0.05, twist=22),
                  dict(upper=(0.6, -0.6, -0.5), fore=(0.5, -0.65, -0.55), blade=(0.4, -0.7, -0.6), lean=12, step=0.09, drop=0.05, twist=25)],
        # The finisher, a whirling sweep: the spear trailed far back on the right, then swung flat round at the hip, head leading, across to the left.
        "finisher": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                    dict(upper=(-0.4, -0.7, 0.6), fore=(-0.5, -0.2, 0.85), blade=(-0.7, -0.1, 0.7), lean=-4, drop=0.06, twist=-30),
                    dict(upper=(0.3, -0.7, 0.6), fore=(0.6, -0.2, 0.75), blade=(0.4, 0.05, 0.9), mid=True, lean=2, step=0.05, drop=0.06, twist=-10),
                    dict(upper=(0.8, -0.5, -0.2), fore=(0.85, -0.1, -0.5), blade=(0.7, 0.05, -0.7), lean=8, step=0.1, drop=0.06, twist=20),
                    dict(upper=(0.4, -0.7, -0.6), fore=(0.3, -0.2, -0.93), blade=(0.0, 0.05, -1.0), lean=6, step=0.1, drop=0.05, twist=28)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "light@north": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                       dict(upper=(-0.45, -0.85, 0.25), fore=(0.95, 0.20, -0.20), blade=(0.95, 0.20, -0.25), lean=-4, twist=-15),
                       dict(upper=(0.66, -0.65, -0.27), fore=(0.89, -0.10, -0.40), blade=(0.99, 0.10, 0.00), lean=8, step=0.06, drop=0.01, twist=20)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "heavy@north": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                       dict(upper=(-0.30, 0.45, 0.85), fore=(0.30, 0.95, 0.00), blade=(0.95, -0.15, -0.25), lean=-8, twist=-25),
                       dict(upper=(0.81, 0.20, -0.24), fore=(0.84, -0.30, -0.40), blade=(0.97, -0.20, 0.00), lean=12, step=0.1, drop=0.04, twist=22)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "third@north": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                       dict(upper=(-0.40, -0.90, 0.20), fore=(0.85, -0.30, -0.40), blade=(0.85, -0.35, -0.40), lean=-4, drop=0.05, twist=-15),
                       dict(upper=(0.73, 0.30, -0.30), fore=(0.86, 0.35, -0.34), blade=(0.89, 0.45, 0.00), lean=6, step=0.08, twist=20),
                       dict(upper=(0.73, 0.20, -0.21), fore=(0.91, 0.25, -0.19), blade=(0.90, 0.35, -0.10), lean=5, step=0.08, twist=18)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        # Its breaker stays a shaft bash from the road: the shaft driven ahead crosswise, both ends showing past the body.
        "breaker@north": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                          dict(upper=(-0.30, -0.80, 0.50), fore=(0.60, 0.30, -0.75), blade=(0.15, 0.10, -0.98), lean=-6, drop=0.02, twist=-10),
                          dict(upper=(0.90, -0.20, 0.10), fore=(0.95, 0.05, -0.20), blade=(0.25, 0.10, -0.96), lean=18, step=0.14, drop=0.07, twist=10),
                          dict(upper=(0.82, -0.40, 0.10), fore=(0.93, -0.05, -0.25), blade=(0.25, 0.05, -0.96), lean=16, step=0.14, drop=0.07, twist=8)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "follow@north": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                        dict(upper=(0.10, 0.80, 0.60), fore=(-0.10, 0.95, 0.30), blade=(-0.50, 0.70, 0.50), lean=-10, twist=-20),
                        dict(upper=(0.88, -0.20, -0.36), fore=(0.81, -0.45, -0.37), blade=(0.78, -0.60, 0.00), lean=14, step=0.09, drop=0.05, twist=22),
                        dict(upper=(0.78, -0.60, -0.04), fore=(0.73, -0.65, -0.14), blade=(0.72, -0.70, -0.05), lean=12, step=0.09, drop=0.05, twist=25)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "finisher@north": [dict(upper=(-0.10, -0.95, 0.25), fore=(0.90, 0.05, -0.30), blade=(0.92, 0.15, -0.30)),
                          dict(upper=(-0.40, -0.70, 0.60), fore=(-0.50, -0.20, 0.85), blade=(-0.70, -0.10, 0.70), lean=-4, drop=0.06, twist=-30),
                          dict(upper=(0.30, -0.70, 0.60), fore=(0.60, -0.20, 0.75), blade=(0.40, 0.05, 0.90), mid=True, lean=2, step=0.05, drop=0.06, twist=-10),
                          dict(upper=(0.80, -0.50, -0.18), fore=(0.86, -0.10, -0.50), blade=(0.99, 0.05, 0.00), lean=8, step=0.1, drop=0.06, twist=20),
                          dict(upper=(0.71, -0.70, -0.01), fore=(0.94, -0.20, -0.24), blade=(1.00, 0.05, -0.10), lean=6, step=0.1, drop=0.05, twist=28)],
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
        # The finisher, a cleave: the body arched right back under the raised axe, elbow high and
        # out, then the whole weight dropped into it. It meets the foe at chest height.
        "finisher": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                     dict(upper=(-0.05, 0.80, 0.60), fore=(-0.30, 0.95, 0.05), blade=(-0.90, 0.35, 0.05), lean=-18, twist=-25),
                     dict(mid=True, upper=(0.55, 0.70, 0.45), fore=(0.70, 0.70, 0.00), blade=(0.15, 0.99, 0.00), lean=4, step=0.06, drop=0.03, twist=5),
                     dict(upper=(0.80, -0.45, 0.30), fore=(0.75, -0.62, 0.15), blade=(0.85, -0.48, 0.15), lean=22, step=0.12, drop=0.11, twist=25),
                     dict(upper=(0.55, -0.80, 0.20), fore=(0.40, -0.90, 0.15), blade=(0.60, -0.78, 0.10), lean=20, step=0.12, drop=0.11, twist=22)],
        # The third strike of a string, a backhand chop: the axe swung back over the left shoulder, then flat across to the right.
        "third": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                 dict(upper=(0.5, 0.35, -0.8), fore=(-0.3, 0.5, -0.8), blade=(-0.5, 0.75, -0.4), lean=-4, twist=35),
                 dict(upper=(0.6, -0.1, 0.8), fore=(0.75, 0.0, 0.65), blade=(0.3, 0.1, 0.95), lean=8, step=0.05, drop=0.02, twist=-25),
                 dict(upper=(0.1, -0.3, 0.95), fore=(-0.3, -0.2, 0.93), blade=(-0.6, -0.1, 0.8), lean=6, step=0.05, drop=0.02, twist=-35)],
        # A breaker, a hook-and-wrench: the axe cocked short at the right shoulder (quick to come), the
        # hero lunges and slams it down onto the guard, then wrenches it down to the hip, dragging the guard.
        "breaker": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                   dict(upper=(-0.10, 0.40, 0.90), fore=(-0.10, 0.98, 0.15), blade=(-0.75, 0.60, 0.20), lean=-10, drop=0.03, twist=-30),
                   dict(mid=True, upper=(0.45, 0.70, 0.55), fore=(0.70, 0.70, 0.10), blade=(0.30, 0.95, 0.00), lean=6, step=0.10, drop=0.04, twist=0),
                   dict(upper=(0.90, -0.20, 0.30), fore=(0.88, -0.45, 0.15), blade=(0.80, -0.60, 0.05), lean=26, step=0.16, drop=0.12, twist=30),
                   dict(upper=(0.20, -0.95, 0.25), fore=(0.65, -0.70, 0.20), blade=(0.55, -0.83, 0.05), lean=22, step=0.16, drop=0.14, twist=15)],
        # A follow-through, a second chop from over the left shoulder down to the right, mirroring the heavy.
        "follow": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                  dict(upper=(0.4, 0.6, -0.65), fore=(-0.2, 0.9, -0.3), blade=(-0.75, 0.55, -0.3), lean=-10, twist=25),
                  dict(upper=(0.55, 0.65, 0.2), fore=(0.65, 0.75, -0.1), blade=(0.2, 0.98, 0.0), mid=True, lean=2, step=0.05, drop=0.03, twist=5),
                  dict(upper=(0.9, -0.25, 0.2), fore=(0.85, -0.45, 0.25), blade=(0.6, -0.75, 0.25), lean=16, step=0.09, drop=0.06, twist=-10),
                  dict(upper=(0.5, -0.85, 0.15), fore=(0.35, -0.93, 0.2), blade=(0.7, -0.65, 0.25), lean=14, step=0.09, drop=0.06, twist=-15)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "light@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                       dict(upper=(-0.20, 0.35, 0.90), fore=(-0.10, 0.95, 0.10), blade=(-0.75, 0.55, 0.30), lean=-6, twist=-25),
                       dict(mid=True, upper=(0.45, 0.45, 0.60), fore=(0.55, 0.80, -0.20), blade=(-0.15, 0.95, 0.20), lean=2, step=0.03, drop=0.01, twist=0),
                       dict(upper=(0.86, -0.50, -0.05), fore=(0.88, -0.40, -0.26), blade=(0.97, -0.25, 0.00), lean=10, step=0.05, drop=0.02, twist=35),
                       dict(upper=(0.58, -0.80, -0.16), fore=(0.63, -0.65, -0.42), blade=(0.40, -0.91, -0.10), lean=8, step=0.05, drop=0.02, twist=40)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "heavy@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                       dict(upper=(-0.10, 0.60, 0.75), fore=(-0.20, 0.95, 0.00), blade=(-0.80, 0.50, 0.00), lean=-12, twist=-20),
                       dict(mid=True, upper=(0.55, 0.65, 0.40), fore=(0.65, 0.75, 0.00), blade=(0.20, 0.98, 0.00), lean=2, step=0.05, drop=0.03, twist=5),
                       dict(upper=(0.89, -0.15, -0.12), fore=(0.91, -0.30, -0.28), blade=(0.95, -0.05, 0.00), lean=16, step=0.09, drop=0.06, twist=25),
                       dict(upper=(0.60, -0.75, -0.17), fore=(0.53, -0.80, -0.29), blade=(0.41, -0.89, -0.05), lean=14, step=0.09, drop=0.06, twist=20)],
        "third@east": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                       dict(upper=(0.5, 0.35, -0.8), fore=(-0.3, 0.5, -0.8), blade=(-0.5, 0.75, -0.4), lean=-4, twist=20),
                       dict(upper=(0.80, 0.00, 0.55), fore=(0.88, 0.05, 0.45), blade=(0.80, -0.15, 0.55), lean=8, step=0.05, drop=0.02, twist=-12),
                       dict(upper=(0.45, -0.25, 0.85), fore=(0.35, -0.25, 0.90), blade=(0.55, -0.35, 0.75), lean=6, step=0.05, drop=0.02, twist=-18)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "follow@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                        dict(upper=(0.40, 0.60, -0.65), fore=(-0.20, 0.90, -0.30), blade=(-0.75, 0.55, -0.30), lean=-10, twist=25),
                        dict(upper=(0.55, 0.65, 0.20), fore=(0.65, 0.75, -0.10), blade=(0.20, 0.98, 0.00), mid=True, lean=2, step=0.05, drop=0.03, twist=5),
                        dict(upper=(0.81, -0.15, -0.34), fore=(0.85, -0.35, -0.14), blade=(0.89, -0.15, 0.00), lean=16, step=0.09, drop=0.06, twist=-5),
                        dict(upper=(0.54, -0.80, -0.12), fore=(0.51, -0.80, 0.10), blade=(0.13, -0.96, 0.05), lean=14, step=0.09, drop=0.06, twist=-8)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "finisher@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                          dict(upper=(-0.05, 0.80, 0.60), fore=(-0.30, 0.95, 0.05), blade=(-0.90, 0.35, 0.05), lean=-18, twist=-25),
                          dict(mid=True, upper=(0.55, 0.70, 0.45), fore=(0.70, 0.70, 0.00), blade=(0.15, 0.99, 0.00), lean=4, step=0.06, drop=0.03, twist=5),
                          dict(upper=(0.86, -0.25, -0.13), fore=(0.84, -0.45, -0.30), blade=(0.92, -0.20, 0.00), lean=22, step=0.12, drop=0.11, twist=25),
                          dict(upper=(0.58, -0.78, -0.16), fore=(0.48, -0.82, -0.31), blade=(0.40, -0.89, -0.10), lean=20, step=0.12, drop=0.11, twist=22)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "third@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                       dict(upper=(0.50, 0.35, -0.80), fore=(-0.30, 0.50, -0.80), blade=(-0.50, 0.75, -0.40), lean=-4, twist=35),
                       dict(upper=(0.53, -0.25, -0.73), fore=(0.68, 0.00, -0.59), blade=(0.97, 0.05, 0.00), lean=8, step=0.05, drop=0.02, twist=-15),
                       dict(upper=(0.58, -0.30, -0.55), fore=(0.86, -0.05, -0.34), blade=(0.97, -0.10, 0.10), lean=6, step=0.05, drop=0.02, twist=-20)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        # The breaker's hook-and-wrench lands dead ahead too: down onto the guard, then wrenched low.
        "breaker@north": [dict(upper=(0.35, -0.90, 0.20), fore=(0.95, 0.25, -0.10), blade=(0.35, 0.93, -0.05)),
                         dict(upper=(-0.10, 0.40, 0.90), fore=(-0.10, 0.98, 0.15), blade=(-0.75, 0.60, 0.20), lean=-10, drop=0.03, twist=-30),
                         dict(mid=True, upper=(0.55, 0.65, 0.40), fore=(0.65, 0.75, 0.00), blade=(0.20, 0.98, 0.00), lean=4, step=0.10, drop=0.04, twist=5),
                         dict(upper=(0.89, -0.15, -0.12), fore=(0.91, -0.30, -0.28), blade=(0.85, -0.53, 0.00), lean=26, step=0.16, drop=0.12, twist=25),
                         dict(upper=(0.40, -0.90, -0.10), fore=(0.60, -0.78, -0.10), blade=(0.45, -0.89, 0.00), lean=22, step=0.16, drop=0.14, twist=15)],
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
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "light@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                       dict(upper=(0.10, -0.35, 0.93), fore=(0.45, 0.55, -0.70), blade=(-0.35, 0.92, 0.15), lean=-6, twist=-25),
                       dict(mid=True, upper=(0.60, -0.30, 0.60), fore=(0.75, 0.30, -0.60), blade=(0.50, 0.75, -0.40), lean=2, step=0.03, drop=0.01, twist=0),
                       dict(upper=(0.82, -0.55, -0.14), fore=(0.77, -0.45, -0.46), blade=(0.97, -0.25, 0.00), lean=10, step=0.06, drop=0.03, twist=35),
                       dict(upper=(0.58, -0.80, -0.15), fore=(0.64, -0.65, -0.41), blade=(0.42, -0.90, -0.10), lean=8, step=0.06, drop=0.03, twist=40)],
        "heavy": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                  dict(upper=(0.20, 0.60, 0.75), fore=(-0.10, 0.80, -0.60), blade=(-0.85, 0.15, 0.00), lean=-12, drop=0.03, twist=-15),
                  dict(mid=True, upper=(0.60, 0.55, 0.40), fore=(0.50, 0.75, -0.40), blade=(0.10, 0.99, 0.00), lean=0, step=0.05, drop=0.04, twist=2),
                  dict(upper=(0.90, -0.20, -0.30), fore=(0.85, -0.25, -0.45), blade=(0.95, -0.12, -0.15), lean=12, step=0.11, drop=0.05, twist=20),
                  dict(upper=(0.60, -0.75, -0.20), fore=(0.55, -0.60, -0.55), blade=(0.65, -0.70, -0.15), lean=10, step=0.11, drop=0.05, twist=15)],
        # The finisher, a rising sweep: the blade trailed low behind the right hip, the body
        # sunk and coiled, then swept up through the front and on to high over the left shoulder,
        # the body rising with it.
        "finisher": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                     dict(upper=(-0.35, -0.80, 0.48), fore=(-0.40, -0.45, 0.80), blade=(-0.75, -0.45, 0.48), lean=-4, drop=0.08, twist=-45),
                     dict(mid=True, upper=(0.50, -0.80, 0.33), fore=(0.85, -0.10, 0.50), blade=(0.70, -0.25, 0.67), lean=2, step=0.06, drop=0.06, twist=-10),
                     dict(upper=(0.85, -0.30, -0.43), fore=(0.85, 0.25, -0.46), blade=(0.80, 0.45, -0.40), lean=6, step=0.12, drop=0.04, twist=25),
                     dict(upper=(0.45, 0.55, -0.70), fore=(0.10, 0.85, -0.52), blade=(-0.20, 0.90, -0.40), lean=-4, step=0.12, twist=45)],
        # The third strike of a string, a flat sweep at the waist from the right round to the left.
        "third": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                 dict(upper=(-0.3, -0.8, 0.5), fore=(-0.3, -0.2, 0.93), blade=(-0.7, 0.0, 0.7), lean=-4, drop=0.04, twist=-40),
                 dict(upper=(0.4, -0.7, 0.6), fore=(0.7, -0.1, 0.7), blade=(0.4, 0.05, 0.9), mid=True, lean=2, step=0.04, drop=0.04, twist=-10),
                 dict(upper=(0.85, -0.4, -0.3), fore=(0.8, -0.1, -0.6), blade=(0.6, 0.05, -0.8), lean=8, step=0.08, drop=0.04, twist=30),
                 dict(upper=(0.4, -0.6, -0.7), fore=(0.2, -0.2, -0.95), blade=(-0.2, 0.05, -0.97), lean=6, step=0.08, drop=0.03, twist=45)],
        # A breaker, a two-handed thrust: the hilt drawn back to the right hip, then the point driven straight ahead with the body behind it.
        "breaker": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                   dict(upper=(-0.4, -0.85, 0.3), fore=(0.85, 0.1, -0.5), blade=(0.95, 0.15, -0.2), lean=-6, drop=0.03, twist=-25),
                   dict(upper=(0.95, -0.15, -0.25), fore=(0.95, 0.0, -0.3), blade=(0.97, 0.12, -0.2), lean=16, step=0.14, drop=0.07, twist=20),
                   dict(upper=(0.85, -0.4, -0.25), fore=(0.92, -0.15, -0.35), blade=(0.95, 0.02, -0.3), lean=14, step=0.14, drop=0.07, twist=18)],
        # A follow-through, a reverse diagonal: the blade over the left shoulder, then down across to the low right.
        "follow": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                  dict(upper=(0.3, 0.6, -0.75), fore=(-0.2, 0.85, -0.5), blade=(-0.7, 0.6, -0.4), lean=-8, twist=30),
                  dict(upper=(0.6, 0.55, 0.0), fore=(0.5, 0.8, 0.1), blade=(0.1, 0.99, 0.0), mid=True, lean=0, step=0.05, drop=0.04, twist=5),
                  dict(upper=(0.85, -0.3, 0.3), fore=(0.8, -0.4, 0.45), blade=(0.65, -0.25, 0.72), lean=12, step=0.1, drop=0.05, twist=-15),
                  dict(upper=(0.4, -0.8, 0.4), fore=(0.3, -0.75, 0.6), blade=(0.4, -0.6, 0.7), lean=10, step=0.1, drop=0.05, twist=-25)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "heavy@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                       dict(upper=(0.20, 0.60, 0.75), fore=(-0.10, 0.80, -0.60), blade=(-0.85, 0.15, 0.00), lean=-12, drop=0.03, twist=-15),
                       dict(mid=True, upper=(0.60, 0.55, 0.40), fore=(0.50, 0.75, -0.40), blade=(0.10, 0.99, 0.00), lean=0, step=0.05, drop=0.04, twist=2),
                       dict(upper=(0.86, -0.15, -0.22), fore=(0.83, -0.30, -0.47), blade=(0.97, -0.10, 0.00), lean=12, step=0.11, drop=0.05, twist=20),
                       dict(upper=(0.59, -0.75, -0.12), fore=(0.56, -0.70, -0.44), blade=(0.41, -0.89, -0.05), lean=10, step=0.11, drop=0.05, twist=15)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "follow@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                        dict(upper=(0.30, 0.60, -0.75), fore=(-0.20, 0.85, -0.50), blade=(-0.70, 0.60, -0.40), lean=-8, twist=30),
                        dict(upper=(0.60, 0.55, 0.00), fore=(0.50, 0.80, 0.10), blade=(0.10, 0.99, 0.00), mid=True, lean=0, step=0.05, drop=0.04, twist=5),
                        dict(upper=(0.78, -0.20, -0.41), fore=(0.79, -0.35, -0.25), blade=(0.95, -0.20, 0.00), lean=12, step=0.1, drop=0.05, twist=-10),
                        dict(upper=(0.50, -0.75, -0.24), fore=(0.54, -0.75, 0.03), blade=(0.36, -0.90, 0.10), lean=10, step=0.1, drop=0.05, twist=-20)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "finisher@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                          dict(upper=(-0.35, -0.80, 0.48), fore=(-0.40, -0.45, 0.80), blade=(-0.75, -0.45, 0.48), lean=-4, drop=0.08, twist=-45),
                          dict(mid=True, upper=(0.50, -0.80, 0.33), fore=(0.85, -0.10, 0.50), blade=(0.70, -0.25, 0.67), lean=2, step=0.06, drop=0.06, twist=-10),
                          dict(upper=(0.89, -0.30, -0.35), fore=(0.89, 0.25, -0.38), blade=(0.98, -0.24, 0.00), lean=6, step=0.12, drop=0.04, twist=25),
                          dict(upper=(0.81, 0.55, 0.17), fore=(0.52, 0.85, -0.11), blade=(0.73, 0.69, -0.10), lean=-4, step=0.12, twist=45)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "third@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                       dict(upper=(-0.30, -0.80, 0.50), fore=(-0.30, -0.20, 0.93), blade=(-0.70, 0.00, 0.70), lean=-4, drop=0.04, twist=-40),
                       dict(upper=(0.40, -0.70, 0.60), fore=(0.70, -0.10, 0.70), blade=(0.40, 0.05, 0.90), mid=True, lean=2, step=0.04, drop=0.04, twist=-10),
                       dict(upper=(0.88, -0.40, -0.20), fore=(0.88, -0.10, -0.47), blade=(1.01, 0.05, 0.00), lean=8, step=0.08, drop=0.04, twist=30),
                       dict(upper=(0.81, -0.60, 0.04), fore=(0.94, -0.20, -0.25), blade=(0.99, 0.05, -0.10), lean=6, step=0.08, drop=0.03, twist=45)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        # Its breaker's point is driven down into the guard behind a deep lunge, so it shortens as it goes.
        "breaker@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                          dict(upper=(-0.40, -0.85, 0.30), fore=(0.85, 0.10, -0.50), blade=(0.95, 0.15, -0.20), lean=-6, drop=0.03, twist=-25),
                          dict(upper=(0.92, -0.15, -0.35), fore=(0.91, 0.00, -0.40), blade=(0.99, -0.12, 0.00), lean=20, step=0.16, drop=0.1, twist=20),
                          dict(upper=(0.87, -0.40, -0.16), fore=(0.95, -0.15, -0.25), blade=(0.98, -0.16, -0.05), lean=18, step=0.16, drop=0.1, twist=18)],
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
        # The finisher, a ground slam: hauled higher than the heavy, up on the toes with the blade
        # hanging down the back, then the whole body thrown down behind it into a deep crouch. It
        # lands just ahead of the feet.
        # The raise passes overhead (`pre`): guard and the hang down the back are near opposite, and
        # a blade turned straight between them swings out sideways.
        "finisher": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                     dict(pre=True, upper=(0.35, 0.80, 0.48), fore=(0.20, 0.95, -0.24), blade=(-0.35, 0.93, 0.10), lean=-10, twist=-5),
                     dict(upper=(0.00, 0.85, 0.53), fore=(-0.30, 0.90, -0.31), blade=(-0.75, -0.65, 0.10), lean=-22, drop=-0.02, twist=-10),
                     dict(mid=True, upper=(0.55, 0.75, 0.37), fore=(0.45, 0.87, -0.20), blade=(0.05, 0.99, 0.00), lean=2, step=0.07, drop=0.04, twist=2),
                     dict(upper=(0.80, -0.55, 0.10), fore=(0.65, -0.72, 0.10), blade=(0.60, -0.75, 0.28), lean=28, step=0.15, drop=0.14, twist=15),
                     dict(upper=(0.65, -0.73, 0.05), fore=(0.50, -0.83, 0.10), blade=(0.55, -0.78, 0.30), lean=26, step=0.15, drop=0.14, twist=12)],
        # The third strike of a string, a rising upswing: the blade low on the left, then heaved up across to high on the right, the whole body turning.
        "third": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                 dict(upper=(0.4, -0.8, -0.45), fore=(0.2, -0.5, -0.85), blade=(-0.3, -0.4, -0.87), lean=-4, drop=0.05, twist=40),
                 dict(upper=(0.6, -0.5, 0.3), fore=(0.8, 0.0, 0.6), blade=(0.6, 0.3, 0.75), mid=True, lean=2, step=0.05, drop=0.04, twist=5),
                 dict(upper=(0.4, 0.6, 0.7), fore=(0.3, 0.85, 0.45), blade=(0.1, 0.85, 0.5), lean=-4, step=0.06, twist=-30),
                 dict(upper=(0.1, 0.8, 0.6), fore=(-0.2, 0.95, 0.25), blade=(-0.5, 0.8, 0.3), lean=-4, step=0.06, twist=-35)],
        # A breaker, a lunging thrust: the huge blade's point driven ahead from the hip, the body thrown in behind it.
        "breaker": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                   dict(upper=(-0.4, -0.85, 0.3), fore=(0.85, 0.1, -0.5), blade=(0.95, 0.15, -0.2), lean=-8, drop=0.04, twist=-25),
                   dict(upper=(0.95, -0.15, -0.25), fore=(0.95, 0.0, -0.3), blade=(0.97, 0.12, -0.2), lean=22, step=0.16, drop=0.1, twist=20),
                   dict(upper=(0.85, -0.4, -0.25), fore=(0.92, -0.15, -0.35), blade=(0.95, 0.02, -0.3), lean=20, step=0.16, drop=0.1, twist=18)],
        # A follow-through, a reverse sweep at the hip: from far back on the left round to the right.
        "follow": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                  dict(upper=(0.4, -0.7, -0.6), fore=(0.1, -0.2, -0.97), blade=(-0.6, 0.05, -0.8), lean=-6, drop=0.05, twist=45),
                  dict(upper=(0.6, -0.75, 0.1), fore=(0.95, -0.05, 0.25), blade=(0.9, 0.05, 0.4), mid=True, lean=2, step=0.05, drop=0.05, twist=0),
                  dict(upper=(0.3, -0.6, 0.74), fore=(0.4, 0.0, 0.9), blade=(0.15, 0.1, 0.98), lean=10, step=0.08, drop=0.05, twist=-30),
                  dict(upper=(-0.1, -0.5, 0.85), fore=(-0.4, 0.0, 0.9), blade=(-0.6, 0.05, 0.8), lean=8, step=0.08, drop=0.05, twist=-45)],
        # From the road the slab meets the foe dead ahead, then its weight carries it on down past
        # the hit to the ground by the right foot, where it shows past the cloak: dropped straight
        # down the middle, the body hides it and the blow never seems to land.
        "heavy@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                       dict(upper=(0.05, 0.70, 0.71), fore=(-0.25, 0.85, -0.46), blade=(-0.90, -0.35, 0.10), lean=-16, drop=0.03, twist=-15),
                       dict(mid=True, upper=(0.55, 0.70, 0.40), fore=(0.45, 0.85, -0.30), blade=(0.05, 0.99, 0.00), lean=0, step=0.06, drop=0.04, twist=2),
                       dict(upper=(0.85, -0.20, -0.25), fore=(0.79, -0.38, -0.49), blade=(0.92, -0.25, 0.00), lean=20, step=0.13, drop=0.09, twist=18),
                       dict(upper=(0.50, -0.80, 0.30), fore=(0.45, -0.80, 0.40), blade=(0.20, -0.90, 0.40), lean=22, step=0.13, drop=0.11, twist=22)],
        # The slam the same way, thrown down harder into a deeper crouch.
        "finisher@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                          dict(pre=True, upper=(0.35, 0.80, 0.48), fore=(0.20, 0.95, -0.24), blade=(-0.35, 0.93, 0.10), lean=-10, twist=-5),
                          dict(upper=(0.00, 0.85, 0.53), fore=(-0.30, 0.90, -0.31), blade=(-0.75, -0.65, 0.10), lean=-22, drop=-0.02, twist=-10),
                          dict(mid=True, upper=(0.55, 0.75, 0.37), fore=(0.45, 0.87, -0.20), blade=(0.05, 0.99, 0.00), lean=2, step=0.07, drop=0.04, twist=2),
                          dict(upper=(0.82, -0.35, -0.25), fore=(0.70, -0.50, -0.49), blade=(0.89, -0.35, 0.00), lean=28, step=0.15, drop=0.14, twist=15),
                          dict(upper=(0.50, -0.82, 0.28), fore=(0.42, -0.83, 0.38), blade=(0.18, -0.92, 0.36), lean=30, step=0.15, drop=0.16, twist=20)],
        "third@east": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                       dict(upper=(0.4, -0.8, -0.45), fore=(0.2, -0.5, -0.85), blade=(-0.6, -0.5, -0.6), lean=-4, drop=0.05, twist=20),
                       dict(upper=(0.70, -0.55, 0.35), fore=(0.90, -0.10, 0.40), blade=(0.75, -0.20, 0.60), mid=True, lean=2, step=0.05, drop=0.04, twist=0),
                       dict(upper=(0.65, 0.45, 0.35), fore=(0.62, 0.62, 0.35), blade=(0.55, 0.80, 0.25), lean=-2, step=0.06, twist=-12),
                       dict(upper=(0.35, 0.80, 0.40), fore=(0.20, 0.92, 0.30), blade=(0.05, 0.97, 0.25), lean=-4, step=0.06, twist=-15)],
        "follow@east": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                        dict(upper=(0.4, -0.7, -0.6), fore=(0.1, -0.2, -0.97), blade=(-0.8, 0.05, -0.6), lean=-6, drop=0.05, twist=20),
                        dict(upper=(0.6, -0.75, 0.1), fore=(0.95, -0.05, 0.25), blade=(0.9, 0.05, 0.4), mid=True, lean=2, step=0.05, drop=0.05, twist=0),
                        dict(upper=(0.75, -0.55, 0.35), fore=(0.90, -0.05, 0.43), blade=(0.85, 0.08, 0.52), lean=10, step=0.08, drop=0.05, twist=-12),
                        dict(upper=(0.35, -0.60, 0.72), fore=(0.45, 0.00, 0.89), blade=(0.30, 0.08, 0.95), lean=8, step=0.08, drop=0.05, twist=-20)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "follow@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                        dict(upper=(0.40, -0.70, -0.60), fore=(0.10, -0.20, -0.97), blade=(-0.60, 0.05, -0.80), lean=-6, drop=0.05, twist=45),
                        dict(upper=(0.60, -0.75, 0.10), fore=(0.95, -0.05, 0.25), blade=(0.90, 0.05, 0.40), mid=True, lean=2, step=0.05, drop=0.05, twist=0),
                        dict(upper=(0.40, -0.80, -0.41), fore=(0.79, -0.05, -0.52), blade=(0.99, 0.00, 0.00), lean=10, step=0.08, drop=0.05, twist=-15),
                        dict(upper=(0.42, -0.78, -0.35), fore=(0.92, 0.00, -0.28), blade=(1.00, 0.05, 0.10), lean=8, step=0.08, drop=0.05, twist=-28)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "third@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                       dict(upper=(0.40, -0.80, -0.45), fore=(0.20, -0.50, -0.85), blade=(-0.30, -0.40, -0.87), lean=-4, drop=0.05, twist=40),
                       dict(upper=(0.60, -0.50, 0.30), fore=(0.80, 0.00, 0.60), blade=(0.60, 0.30, 0.75), mid=True, lean=2, step=0.05, drop=0.04, twist=5),
                       dict(upper=(0.57, -0.60, -0.20), fore=(0.74, 0.20, -0.52), blade=(0.92, 0.15, 0.00), lean=2, step=0.06, twist=-10),
                       dict(upper=(0.63, 0.45, -0.10), fore=(0.40, 0.85, -0.21), blade=(0.45, 0.85, 0.10), lean=-4, step=0.06, twist=-25)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        "light@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                       dict(upper=(-0.30, -0.55, 0.78), fore=(-0.45, 0.20, 0.87), blade=(-0.80, 0.15, 0.58), lean=-8, drop=0.03, twist=-35),
                       dict(mid=True, upper=(0.20, -0.45, 0.87), fore=(0.45, 0.10, 0.89), blade=(0.15, 0.05, 0.99), lean=0, step=0.04, drop=0.04, twist=-10),
                       dict(upper=(0.89, -0.35, -0.32), fore=(0.93, -0.15, -0.33), blade=(1.00, -0.05, 0.00), lean=10, step=0.08, drop=0.05, twist=30),
                       dict(upper=(0.71, -0.70, 0.04), fore=(0.87, -0.35, -0.35), blade=(0.96, -0.25, -0.10), lean=8, step=0.08, drop=0.05, twist=35)],
        # From the road the blow meets the foe dead ahead: the fist comes in before the chest and the
        # blade crosses the middle, the body hiding much of it; a cut then drops low ahead, not aside.
        # Its breaker's point is driven down into the guard behind a deep lunge, so it shortens as it goes.
        "breaker@north": [dict(upper=(0.35, -0.90, -0.15), fore=(0.85, 0.05, -0.50), blade=(0.70, 0.65, -0.20)),
                          dict(upper=(-0.40, -0.85, 0.30), fore=(0.85, 0.10, -0.50), blade=(0.95, 0.15, -0.20), lean=-8, drop=0.04, twist=-25),
                          dict(upper=(0.92, -0.15, -0.35), fore=(0.91, 0.00, -0.40), blade=(0.99, -0.10, 0.00), lean=24, step=0.18, drop=0.12, twist=20),
                          dict(upper=(0.87, -0.40, -0.16), fore=(0.95, -0.15, -0.25), blade=(0.98, -0.14, -0.05), lean=22, step=0.18, drop=0.12, twist=18)],
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
    else:
        # The free arm hangs from its shoulder: where the lean and the turn carry the shoulder,
        # the elbow and fist go too, or the upper arm stretches and shrinks as the chest turns.
        moved = sub(tuple(j["LEFT SHOULDER"][:2]), tuple(base["LEFT SHOULDER"][:2]))
        for k in ("LEFT ELBOW", "LEFT ARM"):
            j[k][0] += moved[0]; j[k][1] += moved[1]
    gx, gy = at(grip)
    bx, by = proj(view, blade)
    over = near(add(grip, mul(blade, 0.1))) >= -0.02
    if view == "north":
        # Hands raised before the face, the hilt (HILT of the frame back from the grip) on the head on
        # screen, are farther from the eye than the head (HEAD above the shoulders): it hides the hilt.
        a = math.radians(lean)
        head = (HEAD * math.sin(a), HEAD * math.cos(a), 0.0)
        reach = math.hypot(bx, by) or 1.0
        hilt = [(gx - bx / reach * HILT * k / 4, gy - by / reach * HILT * k / 4) for k in range(5)]
        on_head = any(math.hypot(x - j["NECK"][0], y - j["NOSE"][1]) < HEAD_SEEN for x, y in hilt)
        if on_head and near(add(grip, mul(blade, 0.1))) < depth(view, head) - 0.02: over = False
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


def cross(a, b): return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def carried(v, frm, to):
    """`v` turned by the least rotation that takes unit `frm` to unit `to` (Rodrigues): the way
    the head's edge turns with the haft in the hand."""
    axis, cos = cross(frm, to), dot(frm, to)
    sin = norm(axis)
    if sin < 1e-6: return v
    k = mul(axis, 1 / sin)
    return add(add(mul(v, cos), mul(cross(k, v), sin)), mul(k, dot(k, v) * (1 - cos)))


def faces(view, frames, phases):
    """Per frame, how much of the weapon's breadth shows on screen (anim.rs `Hold::face`, 1 its
    whole face, 0 edge-on, where anim.rs keeps the head's own thickness). The breadth runs the way
    the edge (or a hammer's face) points: the way the blade moves at the hit, square to it, carried
    with the haft to every other drawing, so a guard held up from behind shows its edge as the hit
    does. Of that only what crosses the blade's screen direction (the hold's angle) shows: a chop
    seen from behind swings down the line its haft points up, edge-on. A thrust, which moves along
    itself at the hit, and a blade turned right round from the hit use each drawing's own sweep,
    and a still one its neighbours' value (1 if none)."""
    blades = [unit(p["blade"]) for p in frames]
    n = len(blades)
    sweeps = []
    for i, bl in enumerate(blades):
        moved = sub(blades[min(i + 1, n - 1)], blades[max(i - 1, 0)])
        across = sub(moved, mul(bl, dot(moved, bl)))
        sweeps.append(unit(across) if norm(across) >= 0.05 else None)
    w, a, _ = phases
    hit = next((k for k in (w + a - 1, w + a) if k < n and sweeps[k] is not None), None)
    out = []
    for i, bl in enumerate(blades):
        edge = sweeps[i]
        if hit is not None and dot(blades[hit], bl) > -0.9: edge = carried(sweeps[hit], blades[hit], bl)
        if edge is None: out.append(None); continue
        (ax, ay), (bx, by) = proj(view, edge), proj(view, bl)
        shown = math.hypot(bx, by)
        wide = math.hypot(ax, ay) if shown < 1e-3 else abs(ax * by - ay * bx) / shown
        out.append(min(1.0, wide))
    known = [i for i, f in enumerate(out) if f is not None]
    return [round(f if f is not None else out[min(known, key=lambda k: abs(k - i))] if known else 1.0, 2) for i, f in enumerate(out)]


def lead(view, frames, phases):
    """Which way the head sweeps round the grip on screen through the strike, from the chamber to
    the blow (anim.rs `Hold::lead`): 1 clockwise, -1 anticlockwise, 0 neither (a thrust). Read from
    the blade's swept area, so a frame where it points at the eye and its angle flips counts for
    little; the wind-up and the return to guard may turn either way and are left out."""
    w, a, _ = phases
    shown = [proj(view, unit(p["blade"])) for p in frames[w - 1:w + a]]
    swept = sum(b0[0] * b1[1] - b0[1] * b1[0] for b0, b1 in zip(shown, shown[1:]))
    return 0 if abs(swept) < 0.05 else 1 if swept > 0 else -1


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
                if "@" in move: continue
                keys = spec["moves"].get(f"{move}@{view}", keys)
                frames = timeline(keys, PHASES[(move, view)])
                skel, track = zip(*(pose(base, view, p, spec["off"]) for p in frames))
                for t, f in zip(track, faces(view, frames, PHASES[(move, view)])): t["face"] = f
                for t in track: t["lead"] = lead(view, frames, PHASES[(move, view)])
                name = f"{kind}_{move}_{view}"
                json.dump(list(skel), open(os.path.join(out, name + ".json"), "w"))
                json.dump(list(track), open(os.path.join(out, name + ".weapon.json"), "w"), indent=1)
                # The game's copy (anim.rs `read_holds`): x y angle over reach ex ey lock face lead, a
                # frame a line; the game moves the grip to the fist as drawn (`fit_hold`), from the
                # elbow. Lock (keep the angle as written) is set by hand where a thrust needs it.
                # A bow's right hand holds the string: its track is the draw hand's, not a grip.
                with open(os.path.join(out, name + (".draw.txt" if spec.get("string") else ".hold.txt")), "w") as f:
                    for t in track: f.write(f"{t['grip'][0]} {t['grip'][1]} {t['angle']} {int(t['over'])} {t['reach']} {t['elbow'][0]} {t['elbow'][1]} 0 {t['face']} {t['lead']}\n")
                lo, hi = WRIST.get(kind, WRIST_ANY) if not spec.get("string") else (0.0, 180.0)
                bad = [f"{i}:{t['wrist']}" for i, t in enumerate(track) if not lo <= t["wrist"] <= hi]
                print(f"{name:28} {len(skel):2} frames  wrist {' '.join(str(t['wrist']) for t in track)}" + (f"  OUT OF RANGE {' '.join(bad)}" if bad else ""))


if __name__ == "__main__":
    main()
