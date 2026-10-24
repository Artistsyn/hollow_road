//! Sprite animations made with Pixellab, loaded from `assets/pixellab/<character>/<animation>/`
//! (frames `000.png`, `001.png`, ...). Pixellab centres every frame on the character's pivot:
//! custom animations on a larger canvas (room to move), template ones on the character's own.
//! Frames are padded, centred, to the largest canvas, so every animation of a character shares
//! one ground row and one height: both read from the first frame of a standing clip, which keeps
//! the feet planted through a hop or a roll instead of snapping to each frame's lowest pixel.

use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// One animation's frames, marked as pixel art.
#[derive(Clone)]
pub struct Clip {
    pub frames: Vec<Arc<RgbaImage>>,
    /// The same frames flipped left to right (a dodge to the left from one drawn to the right).
    pub mirrored: Vec<Arc<RgbaImage>>,
}

impl Clip {
    /// The frame `k` of the way through (0..1), mirrored or not.
    pub fn at(&self, k: f32, mirror: bool) -> &Arc<RgbaImage> {
        let n = self.frames.len();
        let i = ((k.clamp(0.0, 1.0) * n as f32) as usize).min(n - 1);
        if mirror { &self.mirrored[i] } else { &self.frames[i] }
    }
    /// Frames made in code, with their mirror images, marked as pixel art.
    pub fn marked(frames: Vec<Arc<RgbaImage>>) -> Clip {
        let mirrored: Vec<_> = frames.iter().map(|f| Arc::new(image::imageops::flip_horizontal(f.as_ref()))).collect();
        for f in frames.iter().chain(&mirrored) { quartz::set_pixel_art(f, true); }
        Clip { frames, mirrored }
    }
    /// The frame for `t` of a loop `period` long.
    pub fn looped(&self, t: f32, period: f32) -> &Arc<RgbaImage> { self.at((t / period).rem_euclid(1.0), false) }

    /// The first `count` frames of a sprite sheet cut in `cell` (width, height) pixel cells, row
    /// by row, marked as pixel art: how a Pixellab effect exports (`<name>.png` beside
    /// `<name>.json`).
    pub fn from_grid(path: &Path, (cw, ch): (u32, u32), count: usize) -> Option<Clip> {
        let sheet = image::open(path).ok()?.to_rgba8();
        let cols = (sheet.width() / cw.max(1)).max(1);
        let frames: Vec<Arc<RgbaImage>> = (0..count as u32)
            .filter(|i| (i / cols + 1) * ch <= sheet.height())
            .map(|i| Arc::new(image::imageops::crop_imm(&sheet, (i % cols) * cw, (i / cols) * ch, cw, ch).to_image()))
            .collect();
        if frames.is_empty() { return None; }
        let mirrored: Vec<_> = frames.iter().map(|f| Arc::new(image::imageops::flip_horizontal(f.as_ref()))).collect();
        for f in frames.iter().chain(&mirrored) { quartz::set_pixel_art(f, true); }
        Some(Clip { frames, mirrored })
    }
}

/// A character's animations and how its pictures stand.
#[derive(Clone)]
pub struct Sheet {
    pub clips: HashMap<String, Clip>,
    /// The row the feet stand on and the figure's height, in pixels of the canvas.
    pub ground: f32,
    pub figure: f32,
    pub canvas: (u32, u32),
    /// Where the spin starts and ends in `roll_spun` (frames), so the turn fills a roll's travel.
    pub roll_keys: (usize, usize),
    /// The clips drawn bare-handed for a weapon to be put in (`Sheet::arm`): where the weapon is
    /// held in each frame, from the clip's `hold.txt`.
    pub holds: HashMap<String, Vec<Hold>>,
    /// Each weapon put in hand (`Sheet::arm`), by art name: its length grip to tip, pixels.
    pub blades: HashMap<String, f32>,
    /// The bare-handed clips' free hands, from each clip's `off.txt` (`Sheet::dress`).
    pub offs: HashMap<String, Vec<OffHold>>,
    /// The weapons put in hand and the offhands there are, by art name, for `Sheet::dress`.
    pub weapons: HashMap<String, Arc<Weapon>>,
    pub shields: HashMap<String, Arc<Shield>>,
    /// The bows and foci there are, gripped in the free fist (`Held`), by art name.
    pub helds: HashMap<String, Arc<Held>>,
    /// A bow clip's string hand, from its `draw.txt` (the free fist holds the bow): where it is in
    /// each frame, and as its `over` whether it is on the string.
    pub draws: HashMap<String, Vec<Hold>>,
    /// Which weapon and offhand pairs `dress` has drawn.
    pub dressed: std::collections::HashSet<(String, String)>,
}

/// Where a frame holds its weapon: the grip in canvas pixels, the blade's angle on screen
/// (degrees, 0 = right, 90 = down), whether it is drawn over the body or behind it, and how much
/// of its length shows (1 = all; less when it points toward or away from the viewer).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hold { pub grip: (f32, f32), pub angle: f32, pub over: bool, pub reach: f32 }

/// Where a frame's free hand is, for an offhand to be strapped to: the fist in canvas pixels, the
/// forearm's direction elbow to fist on screen (degrees, 0 = right, 90 = down), whether the arm is
/// drawn over the body or behind it, whether the frame is side-on (`e_` clips) or from behind,
/// how far the whole figure is turned (a roll's tumble), and whether the figure is on the ground
/// (getting up, knocked down, dying: the arm is flung out, not held before an upright body).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OffHold { pub fist: (f32, f32), pub angle: f32, pub over: bool, pub side: bool, pub spin: f32, pub lying: bool }

/// A point track a built clip carries along with its frames (`side_roll`): weapon holds and
/// offhand holds alike.
trait Track: Copy {
    fn moved(self, dy: f32) -> Self;
    fn turned(self, deg: f32, pivot: (f32, f32)) -> Self;
}

fn turn_about(p: (f32, f32), deg: f32, pivot: (f32, f32)) -> (f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    let (dx, dy) = (p.0 - pivot.0, p.1 - pivot.1);
    (pivot.0 + dx * c - dy * s, pivot.1 + dx * s + dy * c)
}

impl Track for Hold {
    fn moved(self, dy: f32) -> Self { Hold { grip: (self.grip.0, self.grip.1 + dy), ..self } }
    fn turned(self, deg: f32, pivot: (f32, f32)) -> Self { Hold { grip: turn_about(self.grip, deg, pivot), angle: self.angle + deg, ..self } }
}

impl Track for OffHold {
    fn moved(self, dy: f32) -> Self { OffHold { fist: (self.fist.0, self.fist.1 + dy), ..self } }
    fn turned(self, deg: f32, pivot: (f32, f32)) -> Self { OffHold { fist: turn_about(self.fist, deg, pivot), angle: self.angle + deg, spin: self.spin + deg, ..self } }
}

/// An offhand's picture: its face and its inside, both drawn straight on, the pixel that sits on
/// the fist, and how far back along the forearm from the fist that pixel is strapped (a buckler is
/// gripped in the fist, a kite shield rides the forearm).
pub struct Shield { pub front: RgbaImage, pub back: RgbaImage, pub centre: (f32, f32), pub strap: f32 }

/// How far ahead of the fist's middle a buckler's face sits, along the forearm (frame pixels):
/// past the knuckles.
const BUCKLER_AHEAD: f32 = 2.5;

impl Shield {
    /// `<dir>/<name>_front.png` and `_back.png`, placed by `<dir>/offhand.txt` (`name cx cy strap`
    /// a line).
    pub fn load(dir: &Path, name: &str) -> Option<Shield> {
        let open = |side: &str| image::open(dir.join(format!("{name}_{side}.png"))).ok().map(|i| i.to_rgba8());
        let (front, back) = (open("front")?, open("back")?);
        let list = std::fs::read_to_string(dir.join("offhand.txt")).ok()?;
        let n: Vec<f32> = list.lines().find_map(|l| l.strip_prefix(name)?.strip_prefix(' ').map(|r| r.split_whitespace().filter_map(|v| v.parse().ok()).collect()))?;
        (n.len() == 3).then(|| Shield { front, back, centre: (n[0], n[1]), strap: n[2] })
    }

    /// How it shows on a forearm at `hold`: the picture (face or inside), how wide it looks (it is
    /// strapped flat to the outside of the forearm, so it turns with it), and its lean.
    ///
    /// Side-on the free arm is the far one: hanging, the shield faces away and its inside shows
    /// whole; raised before the body it faces the way the hero does and is seen edge-on, face
    /// forward. From behind the inside always shows, narrowed while the arm hangs (hung outside the
    /// cloak at the hero's left side, so drawn over it and out past the cloak) and whole when it is raised before them (and
    /// behind them on screen, held across the left of the chest so its edge shows past the body,
    /// or punched straight out ahead, where it shows whole on the fist over the shoulder).
    /// The arm hangs while the forearm is within 50 degrees of straight down. Returns the picture,
    /// its width, its lean, whether it is drawn over the body, and how far it sits left of the
    /// strap (pixels).
    fn as_seen(&self, hold: &OffHold) -> (&RgbaImage, f32, f32, bool, f32) {
        let raised = hold.angle.to_radians().cos().abs();
        let hang = ((hold.angle - 90.0 + 540.0) % 360.0) - 180.0;
        let lean = (hang * 0.35).clamp(-20.0, 20.0) + hold.spin;
        if hold.side {
            let wide = 1.0 - 0.55 * raised;
            // Strapped along the forearm (a kite shield, a bulwark), the shield lies in the
            // forearm's plane: raised before the body its inside faces the eye whole, behind the
            // arm, which runs across it to the grip. Only a buckler, gripped in the fist, turns
            // its face along the arm and is seen edge-on (`worn_on` puts it ahead of the fist).
            if self.strap > 0.0 { return (&self.back, 1.0 - 0.15 * raised, lean, false, 0.0); }
            (if wide < 0.6 { &self.front } else { &self.back }, wide, lean, hold.over, 0.0)
        } else {
            // From behind the body is between the eye and most of the shield: hanging, it is seen
            // nearly edge-on against the thigh, a sliver out past the cloak; raised before the
            // chest, only its rim shows past the left side. Either way the body hides the rest.
            let hanging = hang.abs() < 50.0;
            // A forearm driven straight up the screen is punched out ahead (a parry): the shield
            // faces the blow, so its inside faces the eye whole, on the fist that grips it, over
            // the shoulder rather than past the body's side.
            let k = ((-hold.angle.to_radians().sin() - 0.75) / 0.2).clamp(0.0, 1.0);
            let ahead = if hanging { 0.0 } else { k * k * (3.0 - 2.0 * k) };
            let wide = if hanging { 0.42 } else { (0.55 + 0.35 * raised) * (1.0 - ahead) + ahead };
            // On the ground an arm not hanging is flung out, not held across the chest: on the fist.
            let left = if hold.lying && !hanging { 0.0 } else { self.back.width() as f32 * wide * if hanging { 0.3 } else { 0.8 * (1.0 - ahead) } };
            (&self.back, wide, lean, hold.over && !hanging, left)
        }
    }

    /// `frame` with the shield on the forearm `hold` says, over the body or behind it.
    pub fn worn_on(&self, frame: &RgbaImage, hold: &OffHold) -> RgbaImage {
        let (img, wide, lean, over, left) = self.as_seen(hold);
        let (fs, fc) = hold.angle.to_radians().sin_cos();
        // A buckler seen edge-on is gripped in the fist, its face just ahead of the knuckles:
        // drawn there and over the fist, the hand never shows past it.
        let edge = hold.side && self.strap == 0.0 && wide < 0.6;
        let ahead = if edge { BUCKLER_AHEAD } else { -self.strap };
        let over = over || edge;
        let at = (hold.fist.0 + fc * ahead - left, hold.fist.1 + fs * ahead);
        let (s, c) = lean.to_radians().sin_cos();
        let (iw, ih) = (img.width() as f32, img.height() as f32);
        let r = [(0.0, 0.0), (iw, 0.0), (0.0, ih), (iw, ih)].iter().map(|&(x, y): &(f32, f32)| (x - self.centre.0).hypot(y - self.centre.1)).fold(0.0, f32::max).ceil();
        let (w, h) = frame.dimensions();
        let (x0, x1) = ((at.0 - r).max(0.0) as u32, ((at.0 + r) as u32).min(w));
        let (y0, y1) = ((at.1 - r).max(0.0) as u32, ((at.1 + r) as u32).min(h));
        let mut out = frame.clone();
        for y in y0..y1 {
            for x in x0..x1 {
                let (dx, dy) = (x as f32 + 0.5 - at.0, y as f32 + 0.5 - at.1);
                // Turned back by the lean, then widened back to the picture's own width.
                let (lx, ly) = (dx * c + dy * s, -dx * s + dy * c);
                let (sx, sy) = (self.centre.0 + lx / wide, self.centre.1 + ly);
                if sx < 0.0 || sy < 0.0 || sx >= iw || sy >= ih { continue; }
                let p = *img.get_pixel(sx as u32, sy as u32);
                if p[3] < 128 { continue; }
                if over || frame.get_pixel(x, y)[3] < 128 { out.put_pixel(x, y, p); }
            }
        }
        out
    }
}

/// A thing gripped in the free fist rather than strapped to the arm: a bow or a focus. Its picture
/// stands upright; with it, the pixel the fist closes on, how big it is drawn (frame pixels to one
/// of its own), how far it turns with the forearm (1: along it, as a bow is carried; 0: upright
/// whatever the arm does), and for a bow where its string is tied.
pub struct Held { pub image: RgbaImage, pub grip: (f32, f32), pub scale: f32, pub follow: f32, pub bow: Option<Bow> }

/// A bow's two string ends (pixels of its picture) and the colour of the arrow it nocks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bow { pub tips: [(f32, f32); 2], pub shaft: [u8; 3] }

/// How far a bow's tips come back toward the string hand at full draw, in bow lengths.
const FLEX: f32 = 0.15;
const STRING: Rgba<u8> = Rgba([176, 166, 138, 255]);
const FLETCH: Rgba<u8> = Rgba([226, 222, 210, 255]);
const POINT: Rgba<u8> = Rgba([172, 174, 182, 255]);

impl Held {
    /// `<dir>/<name>.png`, placed by `<dir>/held.txt` (`name gx gy scale follow` a line, and for a
    /// bow `tx ty tx ty r g b` after: its tips and its arrow's colour).
    pub fn load(dir: &Path, name: &str) -> Option<Held> {
        let image = image::open(dir.join(format!("{name}.png"))).ok()?.to_rgba8();
        let list = std::fs::read_to_string(dir.join("held.txt")).ok()?;
        let n: Vec<f32> = list.lines().find_map(|l| l.strip_prefix(name)?.strip_prefix(' ').map(|r| r.split_whitespace().filter_map(|v| v.parse().ok()).collect()))?;
        let bow = (n.len() == 11).then(|| Bow { tips: [(n[4], n[5]), (n[6], n[7])], shaft: [n[8] as u8, n[9] as u8, n[10] as u8] });
        (n.len() == 4 || bow.is_some()).then(|| Held { image, grip: (n[0], n[1]), scale: n[2], follow: n[3], bow })
    }

    /// `frame` with this in the fist `hold` says, over the body or behind it: turned with the
    /// forearm by `follow`, a little narrowed from behind (a bow's curve is seen more edge-on
    /// there), a bow strung tip to tip. `draw` is a bow clip's string hand (`draw.txt`): the bow
    /// stands upright in the fist, and while the hand is on the string (its `over`) the string is
    /// pulled back to it and an arrow lies nocked from it, string and arrow over the body. Side-on
    /// the arrow runs from the hand out through the grip; from behind it points away up the road,
    /// foreshortened (the two fists are too nearly in line there to give it a direction).
    pub fn gripped(&self, frame: &RgbaImage, hold: &OffHold, draw: Option<&Hold>) -> RgbaImage {
        let hang = ((hold.angle - 90.0 + 540.0) % 360.0) - 180.0;
        let lean = if draw.is_some() { 0.0 } else { hang * self.follow } + hold.spin;
        // A bow is drawn shorter than its picture but as thick: scaled across too, its stave
        // thins to a pixel and breaks up.
        let across = if self.bow.is_some() { 1.0 } else { self.scale };
        let (kx, ky) = (across * if hold.side { 1.0 } else { 0.8 }, self.scale);
        let (s, c) = lean.to_radians().sin_cos();
        let place = |p: (f32, f32)| {
            let (dx, dy) = ((p.0 - self.grip.0) * kx, (p.1 - self.grip.1) * ky);
            (hold.fist.0 + dx * c - dy * s, hold.fist.1 + dx * s + dy * c)
        };
        let over = hold.over;
        // Drawn, the limbs flex back toward the string hand, the more the farther it is drawn:
        // each point of the stave moves across the bow by the square of how far out along its
        // limb it is, so the grip stays put and the tips come back most. Seen from behind the
        // hand is nearly in line with the bow, and so is the bend.
        let half = self.bow.map_or(1.0, |b| 0.5 * (b.tips[1].1 - b.tips[0].1).abs() * ky).max(1.0);
        let (flex, toward) = match (self.bow, draw.filter(|d| d.over)) {
            (Some(_), Some(d)) => {
                let (hx, hy) = (d.grip.0 - hold.fist.0, d.grip.1 - hold.fist.1);
                let dist = hx.hypot(hy).max(1.0);
                (FLEX * 2.0 * half * (dist / (1.1 * half)).min(1.0), (hx * c + hy * s) / dist)
            }
            _ => (0.0, 0.0),
        };
        let bend = |ly: f32| toward * flex * (ly / half).powi(2).min(1.0);
        let (iw, ih) = (self.image.width() as f32, self.image.height() as f32);
        let r = [(0.0, 0.0), (iw, 0.0), (0.0, ih), (iw, ih)].iter().map(|&(x, y): &(f32, f32)| (x - self.grip.0).hypot(y - self.grip.1)).fold(0.0, f32::max) * self.scale;
        let (w, h) = frame.dimensions();
        let (x0, x1) = ((hold.fist.0 - r).max(0.0) as u32, ((hold.fist.0 + r).ceil() as u32).min(w));
        let (y0, y1) = ((hold.fist.1 - r).max(0.0) as u32, ((hold.fist.1 + r).ceil() as u32).min(h));
        let mut out = frame.clone();
        for y in y0..y1 {
            for x in x0..x1 {
                let (dx, dy) = (x as f32 + 0.5 - hold.fist.0, y as f32 + 0.5 - hold.fist.1);
                let (lx, ly) = (dx * c + dy * s, -dx * s + dy * c);
                let lx = lx - bend(ly);
                let (sx, sy) = (self.grip.0 + lx / kx, self.grip.1 + ly / ky);
                if sx < 0.0 || sy < 0.0 || sx >= iw || sy >= ih { continue; }
                let p = *self.image.get_pixel(sx as u32, sy as u32);
                if p[3] < 128 { continue; }
                if over || frame.get_pixel(x, y)[3] < 128 { out.put_pixel(x, y, p); }
            }
        }
        let Some(bow) = self.bow else { return out };
        let tip = |t: (f32, f32)| {
            let (x, y) = place(t);
            let off = bend((t.1 - self.grip.1) * ky);
            (x + off * c, y + off * s)
        };
        let (a, b) = (tip(bow.tips[0]), tip(bow.tips[1]));
        match draw.filter(|d| d.over) {
            Some(d) => {
                let hand = d.grip;
                stroke(&mut out, frame, a, hand, STRING, true);
                stroke(&mut out, frame, hand, b, STRING, true);
                // The arrow, nock at the string hand: side-on out past the bow hand, as long as
                // six tenths of the bow, its point beyond the grip however far the string is back;
                // from behind, away up the road and a third as long.
                let bow_len = (b.0 - a.0).hypot(b.1 - a.1);
                let (vx, vy) = (hold.fist.0 - hand.0, hold.fist.1 - hand.1);
                let reach = vx.hypot(vy).max(1.0);
                let ((ux, uy), len) = if hold.side { ((vx / reach, vy / reach), (0.6 * bow_len).max(reach + 4.0)) }
                                      else { ((0.35, -0.94), 0.35 * bow_len) };
                let at = |t: f32| (hand.0 + ux * t, hand.1 + uy * t);
                let [r, g, bl] = bow.shaft;
                stroke(&mut out, frame, at(0.0), at(len - 3.0), Rgba([r, g, bl, 255]), true);
                stroke(&mut out, frame, at(1.0), at(4.0), FLETCH, true);
                stroke(&mut out, frame, at(len - 3.0), at(len), POINT, true);
            }
            None => stroke(&mut out, frame, a, b, STRING, over),
        }
        out
    }
}

/// A one-pixel line from `a` to `b` on `out`, over the body (`over`) or only where `frame` is
/// clear.
fn stroke(out: &mut RgbaImage, frame: &RgbaImage, a: (f32, f32), b: (f32, f32), colour: Rgba<u8>, over: bool) {
    let n = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().max(1.0) as u32;
    let (w, h) = out.dimensions();
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let (x, y) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        if x < 0.0 || y < 0.0 || x >= w as f32 || y >= h as f32 { continue; }
        let (x, y) = (x as u32, y as u32);
        if over || frame.get_pixel(x, y)[3] < 128 { out.put_pixel(x, y, colour); }
    }
}

/// Room left round a bare-handed clip's frames for the weapon to reach into, pixels.
const HOLD_MARGIN: u32 = 64;

/// A weapon's picture, drawn lying along its blade, with the pixel the hand closes on and its tip.
#[derive(Clone)]
pub struct Weapon { pub image: RgbaImage, pub grip: (f32, f32), pub tip: (f32, f32) }

impl Weapon {
    /// `<dir>/<name>.png`, its grip and tip from `<dir>/weapons.txt` (`name gx gy tx ty` a line).
    pub fn load(dir: &Path, name: &str) -> Option<Weapon> {
        let image = image::open(dir.join(format!("{name}.png"))).ok()?.to_rgba8();
        let list = std::fs::read_to_string(dir.join("weapons.txt")).ok()?;
        let n: Vec<f32> = list.lines().find_map(|l| l.strip_prefix(name)?.strip_prefix(' ').map(|r| r.split_whitespace().filter_map(|v| v.parse().ok()).collect()))?;
        (n.len() == 4).then(|| Weapon { image, grip: (n[0], n[1]), tip: (n[2], n[3]) })
    }

    /// `frame` with the weapon put in its hand as `hold` says.
    pub fn held_in(&self, frame: &RgbaImage, hold: &Hold) -> RgbaImage {
        let own = (self.tip.1 - self.grip.1).atan2(self.tip.0 - self.grip.0);
        let (s, c) = (hold.angle.to_radians() - own).sin_cos();
        let (os, oc) = own.sin_cos();
        let stretch = 1.0 / hold.reach.clamp(0.2, 1.0);
        let (w, h) = frame.dimensions();
        // Only the pixels the turned weapon can cover: a circle round the grip as far as its
        // farthest corner.
        let (iw, ih) = (self.image.width() as f32, self.image.height() as f32);
        let r = [(0.0, 0.0), (iw, 0.0), (0.0, ih), (iw, ih)].iter().map(|&(x, y): &(f32, f32)| (x - self.grip.0).hypot(y - self.grip.1)).fold(0.0, f32::max).ceil();
        let (x0, x1) = ((hold.grip.0 - r).max(0.0) as u32, ((hold.grip.0 + r) as u32).min(w));
        let (y0, y1) = ((hold.grip.1 - r).max(0.0) as u32, ((hold.grip.1 + r) as u32).min(h));
        let mut out = frame.clone();
        for y in y0..y1 {
            for x in x0..x1 {
                let (dx, dy) = (x as f32 + 0.5 - hold.grip.0, y as f32 + 0.5 - hold.grip.1);
                // The weapon's own pixel: this one turned back about the grip.
                let (ox, oy) = (dx * c + dy * s, -dx * s + dy * c);
                // Foreshortened: what shows of the blade is drawn from farther along it.
                let (along, across) = ((ox * oc + oy * os) * stretch, -ox * os + oy * oc);
                let (sx, sy) = (self.grip.0 + along * oc - across * os, self.grip.1 + along * os + across * oc);
                if sx < 0.0 || sy < 0.0 || sx >= iw || sy >= ih { continue; }
                let p = *self.image.get_pixel(sx as u32, sy as u32);
                if p[3] < 128 { continue; }
                if hold.over || frame.get_pixel(x, y)[3] < 128 { out.put_pixel(x, y, p); }
            }
        }
        out
    }
}

/// A clip's weapon track: `x y angle over [reach [ex ey]]` a line, x and y fractions of the
/// frame; `ex ey`, the elbow of the skeleton the frame was drawn to, lets [`fit_hold`] find the
/// fist as drawn.
fn read_holds(path: &Path) -> Option<Vec<(f32, f32, f32, bool, f32, Option<(f32, f32)>)>> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().filter(|l| !l.trim().is_empty()).map(|l| {
        let v: Vec<&str> = l.split_whitespace().collect();
        let reach = v.get(4).map_or(Some(1.0), |r| r.parse().ok())?;
        let elbow = match (v.get(5), v.get(6)) { (Some(x), Some(y)) => Some((x.parse().ok()?, y.parse().ok()?)), _ => None };
        Some((v.first()?.parse().ok()?, v.get(1)?.parse().ok()?, v.get(2)?.parse().ok()?, *v.get(3)? == "1", reach, elbow))
    }).collect()
}

/// The grip and blade angle of one frame's hold, moved to the fist as `frame` draws it. The
/// drawing follows its skeleton only roughly (an outstretched arm comes out higher and flatter),
/// so a grip put at the skeleton's wrist can hang below the fist. The fist is the far end of the
/// arm: the opaque pixel farthest along the skeleton's forearm (elbow to grip), within a corridor
/// round it, less half a fist. The blade turns by as much as the drawn forearm does, keeping the
/// wrist's bend as it was authored. Fractions of the frame in and out; unchanged when nothing of
/// the arm is found there.
pub fn fit_hold(frame: &RgbaImage, grip: (f32, f32), angle: f32, elbow: (f32, f32)) -> ((f32, f32), f32) {
    let (w, h) = frame.dimensions();
    let px = |p: (f32, f32)| (p.0 * w as f32, p.1 * h as f32);
    let (g, e) = (px(grip), px(elbow));
    let (dx, dy) = (g.0 - e.0, g.1 - e.1);
    let len = dx.hypot(dy);
    if len < 1.0 { return (grip, angle); }
    let d = (dx / len, dy / len);
    // How far the fist may be from where the skeleton put it, and its half-length: image heights.
    let (reach, side, half) = (0.15 * h as f32, 0.10 * h as f32, 0.022 * h as f32);
    let mut tip: Option<(f32, (f32, f32))> = None;
    for (x, y, p) in frame.enumerate_pixels() {
        if p[3] < 128 { continue; }
        let (qx, qy) = (x as f32 + 0.5 - e.0, y as f32 + 0.5 - e.1);
        let (along, across) = (qx * d.0 + qy * d.1, (qy * d.0 - qx * d.1).abs());
        if along < len * 0.5 || across > side || ((x as f32 + 0.5 - g.0).hypot(y as f32 + 0.5 - g.1)) > reach { continue; }
        if tip.map_or(true, |(a, _)| along > a) { tip = Some((along, (x as f32 + 0.5, y as f32 + 0.5))); }
    }
    let Some((_, t)) = tip else { return (grip, angle) };
    // The drawn forearm runs from the elbow to the tip; the fist's middle is half a fist back.
    let (fx, fy) = (t.0 - e.0, t.1 - e.1);
    let fl = fx.hypot(fy).max(1.0);
    let fist = (t.0 - fx / fl * half, t.1 - fy / fl * half);
    let turn = (fy.atan2(fx) - d.1.atan2(d.0)).to_degrees();
    let turn = ((turn + 540.0) % 360.0 - 180.0).clamp(-35.0, 35.0);
    ((fist.0 / w as f32, fist.1 / h as f32), angle + turn)
}

impl Sheet {
    /// Every animation folder under `dir`; None when there is none (the stand-in art is used).
    pub fn load(dir: &Path) -> Option<Sheet> {
        let mut clips: HashMap<String, Clip> = HashMap::new();
        // Weapon tracks, in fractions of each clip's own frames until the canvas is known.
        let mut tracks: HashMap<String, (Vec<(f32, f32, f32, bool, f32)>, (u32, u32))> = HashMap::new();
        let mut off_tracks: HashMap<String, (Vec<(f32, f32, f32, bool, f32)>, (u32, u32))> = HashMap::new();
        let mut draw_tracks: HashMap<String, (Vec<(f32, f32, f32, bool, f32)>, (u32, u32))> = HashMap::new();
        let mut names: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().filter(|e| e.path().is_dir()).collect();
        names.sort_by_key(|e| e.file_name());
        for e in names {
            let mut files: Vec<_> = std::fs::read_dir(e.path()).ok()?.flatten().map(|f| f.path()).filter(|p| p.extension().is_some_and(|x| x == "png")).collect();
            files.sort();
            let frames: Vec<Arc<RgbaImage>> = files.iter().filter_map(|p| image::open(p).ok()).map(|i| Arc::new(i.to_rgba8())).collect();
            if frames.is_empty() { continue; }
            let name = e.file_name().to_string_lossy().into_owned();
            if let Some(t) = read_holds(&e.path().join("hold.txt")).filter(|t| t.len() == frames.len()) {
                let t = t.into_iter().zip(&frames).map(|((x, y, angle, over, reach, elbow), f)| {
                    let ((x, y), angle) = elbow.map_or(((x, y), angle), |el| fit_hold(f, (x, y), angle, el));
                    (x, y, angle, over, reach)
                }).collect();
                tracks.insert(name.clone(), (t, frames[0].dimensions()));
            }
            if let Some(t) = read_holds(&e.path().join("off.txt")).filter(|t| t.len() == frames.len()) {
                let t = t.into_iter().zip(&frames).map(|((x, y, angle, over, reach, elbow), f)| {
                    let ((x, y), angle) = elbow.map_or(((x, y), angle), |el| fit_hold(f, (x, y), angle, el));
                    (x, y, angle, over, reach)
                }).collect();
                off_tracks.insert(name.clone(), (t, frames[0].dimensions()));
            }
            if let Some(t) = read_holds(&e.path().join("draw.txt")).filter(|t| t.len() == frames.len()) {
                let t = t.into_iter().zip(&frames).map(|((x, y, angle, on, reach, elbow), f)| {
                    let ((x, y), angle) = elbow.map_or(((x, y), angle), |el| fit_hold(f, (x, y), angle, el));
                    (x, y, angle, on, reach)
                }).collect();
                draw_tracks.insert(name.clone(), (t, frames[0].dimensions()));
            }
            clips.insert(name, Clip { frames, mirrored: Vec::new() });
        }
        // One canvas for all: pad each frame, centred, to the largest (and a bare-handed clip's
        // by room for the weapon it will hold).
        let mut canvas = clips.values().flat_map(|c| c.frames.iter()).fold((0, 0), |a, f| (a.0.max(f.width()), a.1.max(f.height())));
        for (_, size) in tracks.values() { canvas = (canvas.0.max(size.0 + 2 * HOLD_MARGIN), canvas.1.max(size.1 + 2 * HOLD_MARGIN)); }
        let on_canvas = |tracks: HashMap<String, (Vec<(f32, f32, f32, bool, f32)>, (u32, u32))>| -> HashMap<String, Vec<Hold>> {
            tracks.into_iter().map(|(name, (t, (w, h)))| {
                let (ox, oy) = (((canvas.0 - w) / 2) as f32, ((canvas.1 - h) / 2) as f32);
                (name, t.into_iter().map(|(x, y, angle, over, reach)| Hold { grip: (ox + x * w as f32, oy + y * h as f32), angle, over, reach }).collect())
            }).collect()
        };
        let mut holds = on_canvas(tracks);
        let draws = on_canvas(draw_tracks);
        let mut offs: HashMap<String, Vec<OffHold>> = off_tracks.into_iter().map(|(name, (t, (w, h)))| {
            let (ox, oy) = (((canvas.0 - w) / 2) as f32, ((canvas.1 - h) / 2) as f32);
            let side = name.starts_with("e_");
            let lying = ["getup", "knockdown", "death"].iter().any(|n| name.ends_with(n));
            (name, t.into_iter().map(|(x, y, angle, over, _)| OffHold { fist: (ox + x * w as f32, oy + y * h as f32), angle, over, side, spin: 0.0, lying }).collect())
        }).collect();
        for clip in clips.values_mut() {
            for f in clip.frames.iter_mut() {
                if f.dimensions() != canvas {
                    let mut big = RgbaImage::new(canvas.0, canvas.1);
                    image::imageops::overlay(&mut big, f.as_ref(), ((canvas.0 - f.width()) / 2) as i64, ((canvas.1 - f.height()) / 2) as i64);
                    *f = Arc::new(big);
                }
            }
            clip.mirrored = clip.frames.iter().map(|f| Arc::new(image::imageops::flip_horizontal(f.as_ref()))).collect();
            for f in clip.frames.iter().chain(&clip.mirrored) { quartz::set_pixel_art(f, true); }
        }
        // Stand and height from the first frame of the steadiest clip there is.
        let reference = ["idle", "one_walk", "walk", "shamble", "light"].iter().find_map(|n| clips.get(*n)).or_else(|| clips.values().next())?.frames[0].clone();
        let (top, bottom) = opaque_rows(&reference)?;
        let roll_keys = side_roll(&mut clips, &mut holds, &mut offs, (&reference, None, None), bottom, ["e_idle", "e_roll", "e_flip"], "roll_side")
            .or_else(|| spin_roll(&mut clips)).unwrap_or((0, 0));
        // The same roll for each way of holding a weapon, from its bare-handed clips.
        for fam in ["one", "two"] {
            let Some(walk) = clips.get(&format!("{fam}_walk")) else { continue };
            let stand = (walk.frames[0].clone(), holds.get(&format!("{fam}_walk")).and_then(|t| t.first().copied()),
                         offs.get(&format!("{fam}_walk")).and_then(|t| t.first().copied()));
            side_roll(&mut clips, &mut holds, &mut offs, (&stand.0, stand.1, stand.2), bottom,
                      [&format!("e_{fam}_idle"), &format!("e_{fam}_roll"), &format!("e_{fam}_flip")], &format!("{fam}_roll_side"));
        }
        Some(Sheet { clips, ground: bottom as f32 + 1.0, figure: (bottom + 1 - top) as f32, canvas, roll_keys, holds, blades: HashMap::new(),
                     offs, weapons: HashMap::new(), shields: HashMap::new(), helds: HashMap::new(), draws, dressed: Default::default() })
    }

    /// Put `weapon` in the hands of the bare-handed clips drawn for its kind (`<kind>_...` and,
    /// side-on, `e_<kind>_...`; a kind is a weapon type's strikes or a way of holding one, see
    /// `Class::family`): each gains `<clip>@<art>`.
    pub fn arm(&mut self, art: &str, kind: &str, weapon: &Weapon) {
        self.blades.insert(art.to_owned(), (weapon.tip.0 - weapon.grip.0).hypot(weapon.tip.1 - weapon.grip.1));
        self.weapons.entry(art.to_owned()).or_insert_with(|| Arc::new(weapon.clone()));
        for (name, holds) in &self.holds {
            if !name.strip_prefix("e_").unwrap_or(name).strip_prefix(kind).is_some_and(|r| r.starts_with('_')) { continue; }
            let Some(bare) = self.clips.get(name) else { continue };
            let frames: Vec<Arc<RgbaImage>> = bare.frames.iter().zip(holds).map(|(f, h)| Arc::new(weapon.held_in(f, h))).collect();
            let mirrored: Vec<_> = frames.iter().map(|f| Arc::new(image::imageops::flip_horizontal(f.as_ref()))).collect();
            for f in frames.iter().chain(&mirrored) { quartz::set_pixel_art(f, true); }
            self.clips.insert(format!("{name}@{art}"), Clip { frames, mirrored });
        }
    }

    /// Draw the offhand `off` (one of `shields`, or of `helds`) on the free arm of every clip that
    /// holds the weapon `art` and has a free hand (`off.txt`): each gains `<clip>@<art>+<off>`, the
    /// offhand put on first and the weapon put in hand over it. A bow also gets its own clips, the
    /// ones with a string hand (`draw.txt`) and nothing in the weapon hand: `<clip>+<off>`. Once per
    /// pair; what is worn is drawn when it is put on, not every pair at load.
    pub fn dress(&mut self, art: &str, off: &str) {
        if !self.dressed.insert((art.to_owned(), off.to_owned())) { return; }
        let (shield, held) = (self.shields.get(off).cloned(), self.helds.get(off).cloned());
        let Some(weapon) = self.weapons.get(art).cloned().filter(|_| shield.is_some() || held.is_some()) else { return };
        let put = |f: &RgbaImage, o: &OffHold| match (&shield, &held) {
            (Some(s), _) => s.worn_on(f, o),
            (None, Some(g)) => g.gripped(f, o, None),
            (None, None) => f.clone(),
        };
        let suffix = format!("@{art}");
        let names: Vec<String> = self.clips.keys().filter_map(|n| n.strip_suffix(&suffix)).filter(|n| self.offs.contains_key(*n)).map(str::to_owned).collect();
        for name in names {
            let (Some(bare), Some(holds), Some(offs)) = (self.clips.get(&name), self.holds.get(&name), self.offs.get(&name)) else { continue };
            let frames: Vec<Arc<RgbaImage>> = bare.frames.iter().zip(holds).zip(offs)
                .map(|((f, h), o)| Arc::new(weapon.held_in(&put(f, o), h))).collect();
            self.clips.insert(format!("{name}@{art}+{off}"), Clip::marked(frames));
        }
        let Some(bow) = held.filter(|g| g.bow.is_some()) else { return };
        let names: Vec<String> = self.draws.keys().filter(|n| !self.clips.contains_key(&format!("{n}+{off}"))).cloned().collect();
        for name in names {
            let (Some(bare), Some(draws), Some(offs)) = (self.clips.get(&name), self.draws.get(&name), self.offs.get(&name)) else { continue };
            let frames: Vec<Arc<RgbaImage>> = bare.frames.iter().zip(draws).zip(offs).map(|((f, d), o)| Arc::new(bow.gripped(f, o, Some(d)))).collect();
            self.clips.insert(format!("{name}+{off}"), Clip::marked(frames));
        }
    }

    pub fn clip(&self, name: &str) -> Option<&Clip> { self.clips.get(name) }

    /// Where to draw a frame so the figure stands `metres` tall with its feet at `feet` (canvas
    /// units), `ppm` canvas units to the metre: the object's position and size.
    pub fn place(&self, feet: (f32, f32), metres: f32, ppm: f32) -> ((f32, f32), (f32, f32)) {
        let k = metres * ppm / self.figure;
        let (w, h) = (self.canvas.0 as f32 * k, self.canvas.1 as f32 * k);
        ((feet.0 - 0.5 * w, feet.1 - self.ground * k), (w, h))
    }
}

/// Steps in the full turn of a roll's tuck.
const SPIN_STEPS: usize = 8;

/// A `roll_spun` clip from a `roll` clip: the crouch into the tuck, the tuck turned a full circle
/// clockwise (the top going right, as a roll to the right turns) and kept on the ground, then the
/// rise. Pixellab draws a roll as a tuck in place; the turn is what makes it read as a roll.
/// Returns where the spin starts and ends.
fn spin_roll(clips: &mut HashMap<String, Clip>) -> Option<(usize, usize)> {
    let roll = clips.get("roll")?;
    let n = roll.frames.len();
    if n < 4 { return None; }
    // The tuck: the lowest drawing in the clip's middle.
    let height = |f: &RgbaImage| opaque_rows(f).map_or(u32::MAX, |(t, b)| b - t);
    let tuck = (n / 4..n * 3 / 4 + 1).min_by_key(|&i| height(&roll.frames[i]))?;
    let src = roll.frames[tuck].as_ref();
    let (x0, y0, x1, y1) = opaque_box(src)?;
    let pivot = ((x0 + x1) as f32 * 0.5, (y0 + y1) as f32 * 0.5);
    let spun = (0..SPIN_STEPS).map(|k| {
        let r = rotated(src, 360.0 * k as f32 / SPIN_STEPS as f32, pivot);
        // Back down onto the tuck's lowest row: a turning body rolls along the ground.
        let drop = opaque_rows(&r).map_or(0, |(_, b)| y1 as i64 - b as i64);
        let mut out = RgbaImage::new(src.width(), src.height());
        image::imageops::overlay(&mut out, &r, 0, drop);
        Arc::new(out)
    });
    let mut frames: Vec<Arc<RgbaImage>> = roll.frames[tuck / 2..tuck].to_vec();
    let start = frames.len();
    frames.extend(spun);
    let end = frames.len();
    frames.extend(roll.frames[tuck + 1..].iter().cloned());
    let mirrored: Vec<_> = frames.iter().map(|f| Arc::new(image::imageops::flip_horizontal(f.as_ref()))).collect();
    for f in frames.iter().chain(&mirrored) { quartz::set_pixel_art(f, true); }
    clips.insert("roll_spun".into(), Clip { frames, mirrored });
    Some((start, end))
}

/// A `roll_side` clip, the roll as a Souls game shows it from behind: the figure turns side-on to
/// the way it rolls, dips, dives over the leading shoulder, turns a full circle along the ground,
/// lands in a crouch and turns back. Built from the side-on clips (`e_idle`, `e_roll` for the dip
/// and the landing crouch, `e_flip` for the dive, the tuck and the coming out) with the tuck turned
/// in between, every frame set down on the ground row `ground`. Returns where the turn over the
/// shoulder starts and ends.
///
/// `src` names the side-on idle, dip and flip clips and `out` the clip built from them. When every
/// source carries a weapon track (the bare-handed family clips, `e_one_roll` and so on, with the
/// family's walk as `stand`), the built clip gets one too, moved and turned with its frames, so
/// `Sheet::arm` puts the weapon worn in the roll's hand; the same for a free hand's track
/// (`Sheet::dress`).
fn side_roll(clips: &mut HashMap<String, Clip>, holds: &mut HashMap<String, Vec<Hold>>, offs: &mut HashMap<String, Vec<OffHold>>,
             stand: (&Arc<RgbaImage>, Option<Hold>, Option<OffHold>), ground: u32, src: [&str; 3], out: &str) -> Option<(usize, usize)> {
    let (side, dip, flip) = (clips.get(src[0])?.frames[0].clone(), clips.get(src[1])?, clips.get(src[2])?);
    if dip.frames.len() < 13 || flip.frames.len() < 6 { return None; }
    let drop_of = |f: &RgbaImage| opaque_rows(f).map_or(0, |(_, b)| ground as i64 - b as i64);
    let grounded = |f: &RgbaImage| -> Arc<RgbaImage> {
        let mut out = RgbaImage::new(f.width(), f.height());
        image::imageops::overlay(&mut out, f, 0, drop_of(f));
        Arc::new(out)
    };
    let tuck = flip.frames[2].as_ref();
    let (x0, y0, x1, y1) = opaque_box(tuck)?;
    let pivot = ((x0 + x1) as f32 * 0.5, (y0 + y1) as f32 * 0.5);
    // Each built frame: where its track point comes from (None: the stand), turned clockwise on
    // screen about the tuck's middle, then set down by as much as its picture was.
    let mut frames = vec![stand.0.clone(), side.clone()];
    let mut plan: Vec<(Option<(usize, usize)>, f32, f32)> = vec![(None, 0.0, 0.0), (Some((0, 0)), 0.0, 0.0)];
    for i in 2..5 { frames.push(grounded(&dip.frames[i])); plan.push((Some((1, i)), 0.0, drop_of(&dip.frames[i]) as f32)); }
    // Straight from the crouch into the tuck: the flip's own dive (flat on the front) read as a fall.
    let start = frames.len();
    for k in 0..6 {
        let spun = rotated(tuck, 60.0 * k as f32, pivot);
        plan.push((Some((2, 2)), 60.0 * k as f32, drop_of(&spun) as f32));
        frames.push(grounded(&spun));
    }
    frames.push(grounded(&flip.frames[4]));
    plan.push((Some((2, 4)), 0.0, drop_of(&flip.frames[4]) as f32));
    let end = frames.len();
    for i in 11..13 { frames.push(grounded(&dip.frames[i])); plan.push((Some((1, i)), 0.0, drop_of(&dip.frames[i]) as f32)); }
    frames.extend([side, stand.0.clone()]);
    plan.extend([(Some((0, 0)), 0.0, 0.0), (None, 0.0, 0.0)]);
    fn follow<T: Track>(map: &mut HashMap<String, Vec<T>>, src: [&str; 3], stand: Option<T>, plan: &[(Option<(usize, usize)>, f32, f32)], pivot: (f32, f32), out: &str) {
        let track: Option<Vec<T>> = plan.iter().map(|&(from, deg, dy)| match from {
            None => stand,
            Some((c, i)) => map.get(src[c]).and_then(|t| t.get(i).copied()).map(|p| if deg != 0.0 { p.turned(deg, pivot) } else { p }.moved(dy)),
        }).collect();
        if let Some(t) = track { map.insert(out.into(), t); }
    }
    follow(holds, src, stand.1, &plan, pivot, out);
    follow(offs, src, stand.2, &plan, pivot, out);
    let mirrored: Vec<_> = frames.iter().map(|f| Arc::new(image::imageops::flip_horizontal(f.as_ref()))).collect();
    for f in frames.iter().chain(&mirrored) { quartz::set_pixel_art(f, true); }
    clips.insert(out.into(), Clip { frames, mirrored });
    Some((start, end))
}

pub fn rotated(img: &RgbaImage, degrees: f32, pivot: (f32, f32)) -> RgbaImage {
    let (s, c) = degrees.to_radians().sin_cos();
    let (w, h) = img.dimensions();
    RgbaImage::from_fn(w, h, |x, y| {
        let (dx, dy) = (x as f32 + 0.5 - pivot.0, y as f32 + 0.5 - pivot.1);
        // The source pixel: the destination turned back.
        let (sx, sy) = (pivot.0 + dx * c + dy * s, pivot.1 - dx * s + dy * c);
        if sx < 0.0 || sy < 0.0 || sx >= w as f32 || sy >= h as f32 { return image::Rgba([0, 0, 0, 0]); }
        *img.get_pixel(sx as u32, sy as u32)
    })
}

/// The smallest box (x0, y0, x1, y1), inclusive, holding every visible pixel.
fn opaque_box(img: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (top, bottom) = opaque_rows(img)?;
    let col = |x: u32| (top..=bottom).any(|y| img.get_pixel(x, y)[3] > 32);
    let left = (0..img.width()).find(|&x| col(x))?;
    let right = (0..img.width()).rev().find(|&x| col(x))?;
    Some((left, top, right, bottom))
}

/// The first and last rows with a visible pixel.
fn opaque_rows(img: &RgbaImage) -> Option<(u32, u32)> {
    let row = |y: u32| (0..img.width()).any(|x| img.get_pixel(x, y)[3] > 32);
    let top = (0..img.height()).find(|&y| row(y))?;
    let bottom = (0..img.height()).rev().find(|&y| row(y))?;
    Some((top, bottom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_stands_on_its_reference_frame_and_places_feet_where_asked() {
        let dir = std::env::temp_dir().join(format!("hollow_road_sheet_{}", std::process::id()));
        let idle = dir.join("idle");
        std::fs::create_dir_all(&idle).unwrap();
        // A 20 x 40 canvas with a figure on rows 10..=29.
        let mut img = RgbaImage::new(20, 40);
        for y in 10..30 { for x in 8..12 { img.put_pixel(x, y, image::Rgba([200, 0, 0, 255])); } }
        img.save(idle.join("000.png")).unwrap();
        // A clip on a smaller canvas (10 x 20) is padded, centred, to 20 x 40.
        let small = dir.join("drink");
        std::fs::create_dir_all(&small).unwrap();
        let mut tiny = RgbaImage::new(10, 20);
        tiny.put_pixel(0, 0, image::Rgba([0, 255, 0, 255]));
        tiny.save(small.join("000.png")).unwrap();
        let s = Sheet::load(&dir).unwrap();
        let drink = &s.clip("drink").unwrap().frames[0];
        assert_eq!((drink.dimensions(), drink.get_pixel(5, 10)[1]), ((20, 40), 255), "padded and centred");
        assert_eq!((s.ground, s.figure, s.canvas), (30.0, 20.0, (20, 40)));
        // 2 m at 100 units a metre: 10 units a pixel, so the canvas is 200 x 400 and the feet
        // (row 30) land on y 1000.
        let ((x, y), (w, h)) = s.place((500.0, 1000.0), 2.0, 100.0);
        assert_eq!((x, y, w, h), (400.0, 700.0, 200.0, 400.0));
        let clip = s.clip("idle").unwrap();
        assert!(quartz::is_pixel_art(&clip.frames[0]) && quartz::is_pixel_art(&clip.mirrored[0]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_grid_sheet_is_cut_row_by_row_and_stops_at_the_count() {
        // A 2 x 2 grid of 4-pixel cells, each its own colour; only three are frames.
        let mut sheet = RgbaImage::new(8, 8);
        for (i, c) in [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [9, 9, 9, 255]].iter().enumerate() {
            let (cx, cy) = ((i as u32 % 2) * 4, (i as u32 / 2) * 4);
            for y in cy..cy + 4 { for x in cx..cx + 4 { sheet.put_pixel(x, y, image::Rgba(*c)); } }
        }
        let path = std::env::temp_dir().join(format!("hollow_road_grid_{}.png", std::process::id()));
        sheet.save(&path).unwrap();
        let clip = Clip::from_grid(&path, (4, 4), 3).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(clip.frames.len(), 3);
        assert_eq!((clip.frames[1].get_pixel(0, 0)[1], clip.frames[2].get_pixel(3, 3)[2]), (255, 255), "second is green, third blue");
        assert!(quartz::is_pixel_art(&clip.frames[0]) && quartz::is_pixel_art(&clip.mirrored[2]));
    }

    #[test]
    fn a_weapon_is_put_at_the_grip_along_the_holds_angle_and_behind_or_over_the_body() {
        // A 5-pixel blade, grip at its left end; a body pixel where it will cross.
        let mut image = RgbaImage::new(5, 1);
        for x in 0..5 { image.put_pixel(x, 0, image::Rgba([200, 200, 255, 255])); }
        let w = Weapon { image, grip: (0.5, 0.5), tip: (4.5, 0.5) };
        let mut frame = RgbaImage::new(16, 16);
        frame.put_pixel(8, 10, image::Rgba([90, 50, 20, 255]));
        // Straight down from (8, 7): the tip at (8, 11), crossing the body at (8, 10).
        let over = w.held_in(&frame, &Hold { grip: (8.5, 7.5), angle: 90.0, over: true, reach: 1.0 });
        assert_eq!(over.get_pixel(8, 11)[2], 255);
        assert_eq!(over.get_pixel(8, 10)[2], 255, "over the body");
        assert_eq!(over.get_pixel(9, 9)[3], 0, "nothing beside the blade");
        let under = w.held_in(&frame, &Hold { grip: (8.5, 7.5), angle: 90.0, over: false, reach: 1.0 });
        assert_eq!(under.get_pixel(8, 10)[2], 20, "behind the body");
        assert_eq!(under.get_pixel(8, 11)[2], 255);
        // Pointing partly toward the viewer, three-fifths of it shows.
        let short = w.held_in(&RgbaImage::new(16, 16), &Hold { grip: (8.5, 7.5), angle: 90.0, over: true, reach: 0.6 });
        assert_eq!((short.get_pixel(8, 9)[2], short.get_pixel(8, 10)[3]), (255, 0), "the tip comes nearer the hand");
    }

    #[test]
    fn from_behind_a_buckler_punched_out_ahead_shows_its_inside_whole_on_the_fist() {
        let side = |c| RgbaImage::from_pixel(10, 10, image::Rgba(c));
        let shield = Shield { front: side([255, 0, 0, 255]), back: side([0, 0, 255, 255]), centre: (5.0, 5.0), strap: 0.0 };
        // Punched straight up the screen (a parry): the inside, full width, centred on the fist.
        let punched = OffHold { fist: (20.0, 10.0), angle: -90.0, over: false, side: false, spin: 0.0, lying: false };
        let (img, wide, _, _, left) = shield.as_seen(&punched);
        assert_eq!(img.get_pixel(0, 0)[2], 255, "the inside faces the eye");
        assert!(wide > 0.99 && left.abs() < 1e-3, "whole, on the fist: {wide} {left}");
        // Held across the chest (a guard): its edge out past the body's left, as before.
        let (_, wide, _, _, left) = shield.as_seen(&OffHold { angle: -30.0, ..punched });
        assert!(wide < 0.95 && left > 3.0, "edge past the body: {wide} {left}");
    }

    #[test]
    fn a_bow_hangs_along_the_arm_strung_and_its_string_comes_back_to_the_drawing_hand() {
        // A stave down the right of a 4 by 9 picture, gripped at its middle; the tips at the left.
        let stave = image::Rgba([120, 80, 40, 255]);
        let mut image = RgbaImage::new(4, 9);
        for y in 2..7 { image.put_pixel(3, y, stave); }
        let bow = Held { image, grip: (3.5, 4.5), scale: 1.0, follow: 1.0, bow: Some(Bow { tips: [(0.5, 0.5), (0.5, 8.5)], shaft: [90, 60, 30] }) };
        let frame = RgbaImage::new(32, 32);
        let hang = OffHold { fist: (16.5, 16.5), angle: 90.0, over: true, side: true, spin: 0.0, lying: false };
        let carried = bow.gripped(&frame, &hang, None);
        assert_eq!(*carried.get_pixel(16, 16), stave, "the grip in the fist");
        assert_eq!(*carried.get_pixel(13, 16), STRING, "strung tip to tip");
        // On the string: pulled back to the hand at (8, 16), an arrow from it out past the bow.
        let drawn = bow.gripped(&frame, &OffHold { angle: 0.0, ..hang }, Some(&Hold { grip: (8.5, 16.5), angle: 0.0, over: true, reach: 1.0 }));
        assert_eq!(*drawn.get_pixel(16, 14), stave, "upright in the reaching fist");
        assert_eq!(*drawn.get_pixel(10, 14), STRING, "the string comes back to the hand");
        assert_eq!(*drawn.get_pixel(12, 12), STRING, "the tips flex back toward it");
        assert_eq!(*drawn.get_pixel(10, 16), FLETCH);
        assert_eq!(*drawn.get_pixel(13, 16), Rgba([90, 60, 30, 255]), "the shaft through the bow");
        assert_eq!(*drawn.get_pixel(19, 16), POINT, "its point past the grip");
        // Let go: straight again. Carried on a forearm reaching forward: along it.
        let loosed = bow.gripped(&frame, &OffHold { angle: 0.0, ..hang }, Some(&Hold { grip: (8.5, 16.5), angle: 0.0, over: false, reach: 1.0 }));
        assert_eq!(*loosed.get_pixel(13, 16), STRING);
        let reaching = bow.gripped(&frame, &OffHold { angle: 0.0, ..hang }, None);
        assert_eq!((*reaching.get_pixel(14, 16), reaching.get_pixel(16, 13)[3]), (stave, 0), "turned with the forearm");
    }

    #[test]
    fn a_hold_moves_to_the_fist_as_drawn_and_turns_with_the_forearm() {
        // A forearm drawn from (20, 50) rising to a fist ending at x 80, above and beyond where
        // the skeleton put the grip (72, 46) along its own forearm from the elbow (20, 50).
        let mut frame = RgbaImage::new(100, 100);
        for x in 20..80 { for y in 48..52 { frame.put_pixel(x, y - (x - 20) / 6, image::Rgba([120, 80, 40, 255])); } }
        let (grip, angle) = fit_hold(&frame, (0.72, 0.46), 40.0, (0.2, 0.5));
        assert!((grip.0 - 0.775).abs() < 0.02 && (grip.1 - 0.40).abs() < 0.02, "{grip:?}");
        // The skeleton's forearm rose 4 degrees, the drawing's about 10: the blade turns up with it.
        assert!(angle < 38.0 && angle > 31.0, "{angle}");
        // Nothing drawn near: unchanged.
        assert_eq!(fit_hold(&RgbaImage::new(100, 100), (0.6, 0.6), 40.0, (0.2, 0.5)), ((0.6, 0.6), 40.0));
    }

    #[test]
    fn a_quarter_turn_takes_the_top_to_the_right() {
        let mut img = RgbaImage::new(9, 9);
        img.put_pixel(4, 1, image::Rgba([255, 0, 0, 255]));
        let r = rotated(&img, 90.0, (4.5, 4.5));
        assert_eq!(r.get_pixel(7, 4)[0], 255, "top went right");
        assert_eq!(opaque_box(&r), Some((7, 4, 7, 4)));
    }

    /// Every clip of every character, a row each, numbered by frame:
    /// `cargo test --release sheets -- --ignored` writes target/shots/sheet_<character>.png.
    #[test]
    #[ignore]
    fn sheets() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for name in ["wayfarer", "wayfarer_ne", "keeper", "smith", "husk", "lancer", "wretch", "shieldbearer"] {
            let Some(s) = Sheet::load(&root.join("assets/pixellab").join(name)) else { continue };
            let mut clips: Vec<_> = s.clips.iter().collect();
            clips.sort_by_key(|c| c.0.clone());
            let cols = clips.iter().map(|c| c.1.frames.len()).max().unwrap_or(1) as u32;
            let (w, h) = s.canvas;
            let mut out = RgbaImage::from_pixel(cols * w, clips.len() as u32 * h, image::Rgba([60, 64, 58, 255]));
            for (r, (_, c)) in clips.iter().enumerate() {
                for (k, f) in c.frames.iter().enumerate() { image::imageops::overlay(&mut out, f.as_ref(), (k as u32 * w) as i64, (r as u32 * h) as i64); }
            }
            let dir = root.join("target/shots");
            let _ = std::fs::create_dir_all(&dir);
            out.save(dir.join(format!("sheet_{name}.png"))).unwrap();
            eprintln!("[sheet] {name}: {}", clips.iter().map(|(n, c)| format!("{n} {}", c.frames.len())).collect::<Vec<_>>().join(", "));
        }
    }
}
