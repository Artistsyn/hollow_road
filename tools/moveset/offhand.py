"""Offhand tracks: where a bare-handed clip's free (left) hand is, frame by frame, so a shield can
be strapped to it (`anim::Sheet::dress`).

    python3 -I offhand.py <keypoints.json> <off.txt>

Reads the skeleton a clip was drawn to (the keypoint frames sent to animate_with_skeleton_v3) and
writes `off.txt` in the hold.txt format: `x y angle over 1 [ex ey]` a line. x y is the left fist
(fractions of the frame), angle the forearm's direction elbow to fist on screen (degrees, 0 =
right, 90 = down), over 1 when the left arm is drawn in front of the body. The elbow is written
only when the forearm is clear of the body, so `anim::fit_hold` moves the fist onto the drawing
without grabbing the torso.
"""
import json
import math
import sys


def joints(frame):
    return {k["label"]: (k["x"], k["y"], k.get("z_index", 0)) for k in frame}


def line(frame):
    j = joints(frame)
    wx, wy, wz = j["LEFT ARM"]
    ex, ey, _ = j["LEFT ELBOW"]
    angle = math.degrees(math.atan2(wy - ey, wx - ex))
    over = 1 if wz > j["NECK"][2] else 0
    # The body's width at the fist's height: between the shoulders and the hips.
    xs = [j[n][0] for n in ("LEFT SHOULDER", "RIGHT SHOULDER", "LEFT HIP", "RIGHT HIP")]
    clear = wx < min(xs) - 0.02 or wx > max(xs) + 0.02 or wy < j["NECK"][1]
    tail = f" {ex:.4f} {ey:.4f}" if clear and not over else ""
    return f"{wx:.4f} {wy:.4f} {angle:.1f} {over} 1{tail}"


def main():
    frames = json.load(open(sys.argv[1]))
    with open(sys.argv[2], "w") as out:
        out.write("\n".join(line(f) for f in frames) + "\n")


if __name__ == "__main__":
    main()
