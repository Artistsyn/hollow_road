//! Slash and hit effects.
//!
//! A hit is a GPU burst (`Effect::Impact`, the one ball_swing_game's fights use) carried by a
//! transparent object that is placed on the road every tick, so it stays on what it hit while the
//! road and the camera move. Its shape says who was hit (`Dealt` punches outward, `Taken` cracks
//! inward), its colour how the blow was met. Bursts age on fight ticks: they hold at full
//! brightness through a hitstop and stretch out in slow motion, like the rest of the fight.
//!
//! The strikes are Pixellab effects made in its VFX workbench (`assets/pixellab/fx/`), one light
//! and one heavy per kind of weapon. A sword's light is a swipe from right to left along a flat
//! ellipse, drawn in perspective so it lies across the road in front of the hero, and its heavy
//! an overhead chop; an axe hooks down hard and its heavy throws up dust where it lands; a
//! greatsword's light is a wide diagonal and its heavy slams a shockwave across the lanes it
//! sweeps. A rapier's and a spear's are streaks from the hand to the point, laid along the road:
//! each is turned to the line between those two points as drawn, so it runs toward the vanishing
//! point from whichever lane the hero is in. Each is drawn as pixel art at its own aspect ratio
//! from the moment a swing goes live, behind the hero and in front of every enemy. (Pictures are drawn cover-fitted: one squeezed into a box of another shape loses its
//! top and bottom, which is how the first, squashed slash came out clipped.)

use crate::anim::Clip;
use crate::fight::{Event, Fight, Move, HERO_D, LANE_W, SHOULDER};
use crate::gear::Class;
use quartz::*;
use quartz_path_forge::PathForgePlugin;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub const SLASH: &str = "fx_slash";
/// Bursts that can play at once; a new one past that replaces the oldest.
pub const BURSTS: usize = 6;
pub fn burst_id(i: usize) -> String { format!("fx_hit_{i}") }
/// Fight ticks the slash shows each of its frames for.
const SLASH_STEP: u32 = 3;

/// Where a burst stands along the road: beside the hero (metres from the camera, so it stays
/// with them), or where an enemy was hit (metres walked, so it stays on the enemy).
#[derive(Clone, Copy, Debug, PartialEq)]
enum At { Near(f32), Road(f64) }

/// A hit burst playing: across the road, along it, its height and size in metres, colour, shape,
/// and fight ticks left of its whole life.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Burst { x: f32, at: At, y: f32, metres: f32, rgb: [u8; 3], side: ImpactSide, left: u32, total: u32 }

/// Where to draw one effect this tick: top-left and size in canvas units.
type Spot = ((f32, f32), (f32, f32));

/// How a strike's effect lies on the road.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Lie {
    /// Upright across the road where the blow lands: in the hero's lane, `ahead` metres past them
    /// and `high` up (on the ground when `ground`), its width in metres (at least the lanes the
    /// blow sweeps). The picture's point `land` (fractions of it) goes there; else its middle.
    /// `mirror`: drawn swinging the other way from the hero's blows.
    Across { width: f32, ahead: f32, high: f32, land: Option<(f32, f32)>, ground: bool, mirror: bool },
    /// A streak straight up the hero's lane, from the hand (`from` metres past the hero) to
    /// `past` metres beyond the strike's reach, rising or falling between these heights. Drawn
    /// pointing up, its base `BASE` and its tip `TIP` of the way down the picture, in frames
    /// padded square so it can be turned.
    Along { past: f32, from: f32, high: (f32, f32) },
}

const TIP: f32 = 0.08;
const BASE: f32 = 0.95;

/// One kind of weapon's light or heavy strike: its frames and how they lie.
struct Strike { clip: Clip, lie: Lie }

/// Every strike's picture and lie: (kind of weapon, heavy) -> (file, lie).
fn strike_lie(class: Class, heavy: bool) -> (&'static str, Lie) {
    let across = |width, ahead, high| Lie::Across { width, ahead, high, land: None, ground: false, mirror: false };
    // Every chop comes from overhead down in front of the hero, landing in their lane where a
    // foe stands at the blow (at its waist, or on the ground a little farther out).
    let chop = |width, at, ground, mirror| Lie::Across { width, ahead: if ground { 2.0 } else { 1.6 }, high: 0.9, land: Some(at), ground, mirror };
    match (class, heavy) {
        // The sword swings in clips with the blade drawn in, so it has no hold track to follow.
        (Class::Sword, false) => ("light_slash", across(2.6, 2.0, 1.3)),
        (Class::Sword, true) => ("heavy_slash", across(1.4, 1.6, 1.15)),
        // The rapier's flick runs on well past its point; a spear's shaft is long already.
        (Class::Thrust, false) => ("thrust_light", Lie::Along { past: 1.0, from: 0.3, high: (1.25, 1.3) }),
        (Class::Thrust, true) => ("thrust_heavy", Lie::Along { past: 1.4, from: 0.3, high: (1.2, 1.25) }),
        // The light pushes up from the hip; the heavy drives down from over the shoulder.
        (Class::Spear, false) => ("spear_light", Lie::Along { past: 0.8, from: 0.2, high: (1.05, 1.3) }),
        (Class::Spear, true) => ("spear_heavy", Lie::Along { past: 1.2, from: 0.2, high: (1.6, 1.1) }),
        // A light chop's arc is centred on the lane, crossing it; the axe's comes down from over
        // the right shoulder as its swing does, the greatsword's (mirrored) from over the left.
        (Class::Axe, false) => ("axe_light", Lie::Across { width: 1.6, ahead: 1.6, high: 1.0, land: None, ground: false, mirror: false }),
        // The dust (0.89 down) on the ground under the bit.
        (Class::Axe, true) => ("axe_heavy", chop(1.6, (0.55, 0.89), true, false)),
        (Class::Greatsword, false) => ("greatsword_light", Lie::Across { width: 2.6, ahead: 1.6, high: 1.5, land: None, ground: false, mirror: true }),
        // Its shockwave (0.82 down) on the ground, about as wide as the three lanes it sweeps.
        (Class::Greatsword, true) => ("greatsword_heavy", chop(3.8, (0.53, 0.82), true, false)),
        // The ultra's light is a flat sweep at the hip across all three lanes; its heavy lands the
        // greatsword's shockwave, wider.
        (Class::Ultra, false) => ("light_slash", across(4.2, 2.1, 0.6)),
        (Class::Ultra, true) => ("greatsword_heavy", chop(4.8, (0.53, 0.82), true, false)),
    }
}
const CLASSES: [Class; 6] = [Class::Sword, Class::Thrust, Class::Spear, Class::Axe, Class::Greatsword, Class::Ultra];

pub struct Fx {
    /// Per kind of weapon (`CLASSES` order), its light and its heavy.
    strikes: Vec<[Option<Strike>; 2]>,
    /// Streak frames turned to the angle they were last drawn at, kept so a picture is not made
    /// again every tick: (kind, heavy, frame, whole degrees).
    turned: RefCell<HashMap<(usize, bool, usize, i32), Arc<image::RgbaImage>>>,
    bursts: Vec<Burst>,
}

/// A Pixellab effect export: `<name>.png`, a grid of frames, and `<name>.json`, which names every
/// frame and gives the cell's size.
fn effect(dir: &Path, name: &str) -> Option<Clip> {
    let json = std::fs::read_to_string(dir.join(format!("{name}.json"))).ok()?;
    let count = json.matches("\"filename\"").count();
    let frame = &json[json.find("\"frame\"")?..];
    let number = |key: &str| -> Option<u32> {
        let rest = &frame[frame.find(key)? + key.len()..];
        rest.trim_start_matches(|c: char| c == ':' || c.is_whitespace()).split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
    };
    Clip::from_grid(&dir.join(format!("{name}.png")), (number("\"w\"")?, number("\"h\"")?), count)
}

/// `img` centred in a transparent square as tall as it is (a streak, so it can be turned).
fn squared(img: &image::RgbaImage) -> image::RgbaImage {
    let side = img.width().max(img.height());
    let mut out = image::RgbaImage::new(side, side);
    image::imageops::overlay(&mut out, img, ((side - img.width()) / 2) as i64, ((side - img.height()) / 2) as i64);
    out
}

impl Fx {
    pub fn load() -> Fx {
        let dir = crate::assets::path("pixellab/fx");
        let strikes = CLASSES.iter().map(|&class| [false, true].map(|heavy| {
            let (name, lie) = strike_lie(class, heavy);
            let mut clip = effect(&dir, name)?;
            if let Lie::Along { .. } = lie {
                clip.frames = clip.frames.iter().map(|f| Arc::new(squared(f))).collect();
                for f in &clip.frames { quartz::set_pixel_art(f, true); }
            }
            Some(Strike { clip, lie })
        })).collect();
        Fx { strikes, turned: RefCell::new(HashMap::new()), bursts: Vec::new() }
    }

    #[cfg(test)]
    fn bare() -> Fx { Fx { strikes: Vec::new(), turned: RefCell::new(HashMap::new()), bursts: Vec::new() } }

    /// Start the bursts for what the fight just did.
    pub fn on_events(&mut self, events: &[Event], fight: &Fight, walked: f64) {
        // Nothing on one got past: it is behind the camera's view of the hero, not drawn.
        let on_enemy = |i: usize| fight.enemies.get(i).filter(|e| e.gap(walked) > -0.6).map(|e| (e.lane as f32 * LANE_W, e.s - 0.25));
        self.on_events_at(events, (fight.hero.x + SHOULDER, HERO_D), 1.0, on_enemy);
    }

    /// The same for a hero standing `hero` (across, metres ahead of the camera) facing `fw` along
    /// the road (1 away from the camera, -1 toward it), and enemies where `on_enemy` says (across,
    /// road metres): the side-on fight, whose hero faces along the road either way.
    pub fn on_events_at(&mut self, events: &[Event], hero: (f32, f32), fw: f32, on_enemy: impl Fn(usize) -> Option<(f32, f64)>) {
        // The hero as drawn, and a little in front of them (where a buckler meets a blade).
        let (hero_x, hd) = hero;
        let on_enemy = |i: usize| on_enemy(i).map(|(x, s)| (x, At::Road(s)));
        for e in events {
            let burst = |x: f32, at: At, y: f32, metres: f32, rgb: [u8; 3], side: ImpactSide, ticks: u32| {
                Burst { x, at, y, metres, rgb, side, left: ticks, total: ticks }
            };
            let b = match *e {
                Event::Hit { enemy, mv } => on_enemy(enemy).map(|(x, at)| match mv {
                    Move::Heavy => burst(x, at, 1.15, 1.7, [255, 196, 128], ImpactSide::Dealt, 24),
                    Move::Art => burst(x, at, 1.15, 1.8, [210, 200, 255], ImpactSide::Dealt, 26),
                    Move::Riposte => burst(x, at, 1.15, 1.9, [255, 150, 90], ImpactSide::Dealt, 32),
                    _ => burst(x, at, 1.15, 1.2, [255, 228, 184], ImpactSide::Dealt, 20),
                }),
                Event::Deflected { enemy } => on_enemy(enemy).map(|(x, at)| burst(x - 0.2, at, 1.1, 1.0, [200, 222, 255], ImpactSide::Dealt, 14)),
                Event::StanceBroken { enemy } => on_enemy(enemy).map(|(x, at)| burst(x, at, 1.3, 2.2, [255, 246, 222], ImpactSide::Dealt, 30)),
                // A perfect roll: a cold flash at its feet as it stumbles through empty air.
                Event::JustDodge { enemy } => on_enemy(enemy).map(|(x, at)| burst(x, at, 0.55, 1.8, [150, 225, 255], ImpactSide::Dealt, 28)),
                // The glint on the attacker the moment its attack commits, in its warning colour.
                Event::Commits { enemy, kind, parry } => on_enemy(enemy).map(|(x, at)| burst(x + 0.15, at, 1.55, 0.7, crate::game::warning_rgb(kind, parry), ImpactSide::Dealt, 12)),
                // The string's snap or the hex leaving the hand, small and bright; and the rune
                // going off, a wide violet blast low on its patch.
                Event::Loosed { enemy } => on_enemy(enemy).map(|(x, at)| burst(x + 0.1, at, 1.4, 0.55, [255, 236, 200], ImpactSide::Dealt, 10)),
                Event::Burst { enemy, at } => on_enemy(enemy).map(|(x, _)| burst(x, At::Road(at), 0.5, 2.6, [190, 110, 255], ImpactSide::Dealt, 34)),
                // The hero's own: a flick off the string or the hand, and a shot caught on a shield.
                Event::Shot(r) => Some(burst(hero_x + 0.25, At::Near(hd + fw * 0.6), 1.35, 0.6, if r == crate::gear::Ranged::Frost { [150, 210, 255] } else { [240, 230, 200] }, ImpactSide::Dealt, 10)),
                Event::Glanced { enemy } => on_enemy(enemy).map(|(x, at)| burst(x - 0.2, at, 1.1, 0.8, [200, 222, 255], ImpactSide::Dealt, 12)),
                // Blood Imbue: a red welling round the hero's hands.
                Event::Imbued => Some(burst(hero_x + 0.2, At::Near(hd + fw * 0.4), 1.2, 1.1, [210, 24, 36], ImpactSide::Dealt, 22)),
                // A weapon art going off: a wide blast in its colour round the hero (a scream or a
                // slam fills the road about them); Sentence's blow, a gold strike on the marked.
                Event::Art { art } => {
                    use crate::arts::WeaponArt;
                    let (r, g, b) = crate::game::art_rgb(art);
                    let (y, metres) = match art { WeaponArt::Toll | WeaponArt::StoneSkin => (0.4, 3.4), WeaponArt::Wail | WeaponArt::Reap => (1.1, 3.8), WeaponArt::Wingbeat => (1.2, 3.0), _ => (1.2, 1.6) };
                    Some(burst(hero_x, At::Near(hd + fw * 0.7), y, metres, [r, g, b], ImpactSide::Dealt, 34))
                }
                // The bell rung on the stones: a wide bronze blast low where the ring sets out; a
                // boss's blow meeting the ground: dust and grit thrown up.
                Event::Rung { at, .. } => Some(burst(0.0, At::Road(at), 0.35, 3.4, [255, 206, 150], ImpactSide::Dealt, 42)),
                // A bloater bursting: a wide sickly spray; the dead clawing up out of the ground.
                Event::Popped { enemy, at } => on_enemy(enemy).map(|(x, _)| burst(x, At::Road(at), 0.6, 3.2, [168, 196, 74], ImpactSide::Dealt, 40)),
                Event::Raised { enemy } => on_enemy(enemy).map(|(x, at)| burst(x, at, 0.3, 1.6, [126, 150, 112], ImpactSide::Dealt, 30)),
                Event::Slam { at, heavy, .. } => Some(burst(0.0, At::Road(at), 0.2, if heavy { 2.4 } else { 1.6 }, [214, 196, 168], ImpactSide::Dealt, if heavy { 30 } else { 22 })),
                Event::Sentenced { enemy } => on_enemy(enemy).map(|(x, at)| burst(x, at, 1.6, 2.8, [255, 214, 110], ImpactSide::Dealt, 40)),
                Event::Parried { .. } => Some(burst(hero_x + 0.2, At::Near(hd + fw * 0.8), 1.25, 1.4, [255, 214, 110], ImpactSide::Dealt, 26)),
                // A bulwark's last-moment block rings white on the shield; its ward bursting is a
                // wide white-gold blast on the attacker, as big as the ward was full.
                Event::PerfectBlock => Some(burst(hero_x + 0.15, At::Near(hd + fw * 0.45), 1.15, 1.3, [246, 240, 222], ImpactSide::Dealt, 20)),
                // A ward let go, in its shield's way: a bulwark's wide gold blast, a kite's tight
                // white bash, a buckler's blow turned back in steel blue, a parma's riposte in red.
                Event::WardBurst { enemy, power, ward } => on_enemy(enemy).map(|(x, at)| {
                    use crate::gear::Ward;
                    let (size, rgb, n) = match ward {
                        Ward::Bulwark => (1.8 + 1.8 * power, [255, 232, 160], 36),
                        Ward::Kite => (0.9 + 0.9 * power, [240, 240, 236], 24),
                        Ward::Buckler => (1.0 + 1.2 * power, [170, 200, 255], 28),
                        Ward::Parma => (0.8 + 1.0 * power, [230, 60, 60], 26),
                    };
                    burst(x, at, 1.0, size, rgb, ImpactSide::Dealt, n)
                }),
                // Met on the buckler: pale sparks for a guard that held, hot ones for one that let
                // some through, a big orange crack for one smashed open, grey for a fumbled parry.
                Event::Blocked { chip } if chip <= 0.0 => Some(burst(hero_x + 0.15, At::Near(hd + fw * 0.45), 1.1, 0.9, [190, 215, 255], ImpactSide::Dealt, 14)),
                Event::Blocked { .. } => Some(burst(hero_x + 0.15, At::Near(hd + fw * 0.45), 1.1, 1.25, [255, 176, 96], ImpactSide::Dealt, 20)),
                Event::GuardBroken { .. } => Some(burst(hero_x + 0.15, At::Near(hd + fw * 0.4), 1.1, 1.7, [255, 140, 50], ImpactSide::Taken, 26)),
                Event::ParryMissed { .. } => Some(burst(hero_x + 0.2, At::Near(hd + fw * 0.5), 1.15, 1.15, [190, 130, 130], ImpactSide::Taken, 20)),
                // Taken: a small red crack for a flinch, a big one for a heavy blow, amber for a
                // stagger, and low and wide at the feet for a knockdown.
                Event::Struck { heavy: false, .. } => Some(burst(hero_x, At::Near(hd - fw * 0.05), 1.15, 1.0, [230, 40, 30], ImpactSide::Taken, 16)),
                Event::Struck { heavy: true, .. } => Some(burst(hero_x, At::Near(hd - fw * 0.05), 1.15, 1.65, [230, 40, 30], ImpactSide::Taken, 24)),
                Event::Staggered { .. } => Some(burst(hero_x, At::Near(hd - fw * 0.05), 1.1, 1.9, [255, 150, 50], ImpactSide::Taken, 30)),
                Event::KnockedDown { .. } => Some(burst(hero_x, At::Near(hd - fw * 0.1), 0.45, 2.3, [200, 30, 24], ImpactSide::Taken, 34)),
                _ => None,
            };
            let Some(b) = b else { continue };
            if self.bursts.len() >= BURSTS {
                if let Some(oldest) = (0..self.bursts.len()).min_by_key(|&i| self.bursts[i].left) { self.bursts.remove(oldest); }
            }
            self.bursts.push(b);
        }
    }

    /// Age every burst by `ticks` fight ticks (none in a hitstop).
    pub fn tick(&mut self, ticks: u32) {
        for b in &mut self.bursts { b.left = b.left.saturating_sub(ticks); }
        self.bursts.retain(|b| b.left > 0);
    }

    /// The strike playing now, which frame of it, and its kind and weight: a swing from the
    /// moment its live frames begin, in the weapon's own effect (a sword's where a kind has none).
    fn strike_now(&self, fight: &Fight) -> Option<(&Strike, usize, usize, bool, f32)> {
        let a = fight.hero.act?;
        // A riposte is the heavy, a little larger.
        let (heavy, grow) = match a.mv { Move::Light => (false, 1.0), Move::Heavy => (true, 1.0), Move::Riposte => (true, 1.2), _ => return None };
        let class = fight.hero.stats.kit.swung_as();
        let kind = CLASSES.iter().position(|&c| c == class).filter(|&k| self.strikes.get(k).is_some_and(|s| s[heavy as usize].is_some())).unwrap_or(0);
        let strike = self.strikes.get(kind)?[heavy as usize].as_ref()?;
        let since = a.t.checked_sub(a.spec.frames.0)?;
        let frame = (since / SLASH_STEP) as usize;
        strike.clip.frames.get(frame)?;
        Some((strike, kind, frame, heavy, grow))
    }

    /// The strike's picture now and where it goes (top-left and size, canvas units): where the
    /// blow goes, in the hero's lane ahead of them, whichever side the weapon is drawn on.
    fn strike_spot(&self, c: &Canvas, pf: &PathForgePlugin, fight: &Fight) -> Option<(Spot, Arc<image::RgbaImage>)> {
        let (strike, kind, i, heavy, grow) = self.strike_now(fight)?;
        let frame = &strike.clip.frames[i];
        let spec = fight.hero.act?.spec;
        let x = fight.hero.x;
        match strike.lie {
            Lie::Across { width, ahead, high, land, ground, mirror } => {
                let frame = if mirror { &strike.clip.mirrored[i] } else { frame };
                let d = HERO_D + ahead;
                let (px, py) = pf.project_on(c, x, if ground { 0.0 } else { high }, d)?;
                let ppm = pf.scale_on(c, d)?;
                // At the picture's own aspect, so none of it is cropped; as wide as the lanes it sweeps.
                let wide = width.max(LANE_W * (2 * spec.wide + 1) as f32);
                let (w, h) = (wide * grow * ppm, wide * grow * ppm * frame.height() as f32 / frame.width() as f32);
                let at = land.unwrap_or((0.5, 0.5));
                Some((((px - at.0 * w, py - at.1 * h), (w, h)), frame.clone()))
            }
            Lie::Along { past, from, high } => {
                // From the weapon's shoulder, as the point goes out.
                let x = x + SHOULDER;
                let near = pf.project_on(c, x, high.0, HERO_D + from)?;
                let far = pf.project_on(c, x, high.1, HERO_D + spec.reach + past * grow)?;
                let (vx, vy) = (far.0 - near.0, far.1 - near.1);
                let side = frame.height() as f32;
                // Canvas units a picture pixel: the streak, BASE to TIP, spans hand to point.
                let k = vx.hypot(vy) / ((BASE - TIP) * side);
                if !(k > 0.0) { return None; }
                // Turned clockwise from pointing up to pointing along the line, about the streak's
                // middle, which goes halfway between the hand and the point.
                let degrees = (vy.atan2(vx).to_degrees() + 90.0).round() as i32;
                let pivot = (0.5 * side, 0.5 * (TIP + BASE) * side);
                let img = self.turned.borrow_mut().entry((kind, heavy, i, degrees))
                    .or_insert_with(|| {
                        let img = Arc::new(crate::anim::rotated(frame, degrees as f32, pivot));
                        quartz::set_pixel_art(&img, true);
                        img
                    }).clone();
                let mid = (0.5 * (near.0 + far.0), 0.5 * (near.1 + far.1));
                Some((((mid.0 - pivot.0 * k, mid.1 - pivot.1 * k), (side * k, side * k)), img))
            }
        }
    }

    /// Put the slash and the bursts where the road drawn now says they are.
    /// `yard`: the camera turned side-on, where the whole arena is in view, either side of its
    /// middle (on the road, nothing behind the camera is drawn).
    pub fn place(&self, c: &mut Canvas, walked: f64, fight: &Fight, yard: bool) {
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // Centred on a point `y` metres up: its top-left and size, in canvas units.
        let around = |x: f32, y: f32, d: f32, (w, h): (f32, f32)| -> Option<Spot> {
            if d < 0.5 && !yard { return None; }
            let (cx, cy) = pf.project_on(c, x, y, d)?;
            let ppm = pf.scale_on(c, d)?;
            Some(((cx - 0.5 * w * ppm, cy - 0.5 * h * ppm), (w * ppm, h * ppm)))
        };
        let slash = self.strike_spot(c, pf, fight);
        let bursts: Vec<Option<(Spot, Burst)>> = (0..BURSTS).map(|i| {
            let b = *self.bursts.get(i)?;
            let d = match b.at { At::Near(d) => d, At::Road(s) => (s - walked) as f32 };
            Some((around(b.x, b.y, d, (b.metres, b.metres))?, b))
        }).collect();

        if let Some(o) = c.get_game_object_mut(SLASH) {
            match slash {
                Some(((pos, size), img)) => {
                    o.visible = true;
                    o.position = pos;
                    o.size = size;
                    o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None, recolor: None });
                }
                None => o.visible = false,
            }
        }
        // Turned streaks are kept for one swing's worth of angles, not for ever.
        if self.turned.borrow().len() > 192 { self.turned.borrow_mut().clear(); }
        for (i, spot) in bursts.into_iter().enumerate() {
            let id = burst_id(i);
            match spot {
                Some(((pos, size), b)) => {
                    if let Some(o) = c.get_game_object_mut(&id) {
                        o.visible = true;
                        o.position = pos;
                        o.size = size;
                    }
                    let age = b.left as f32 / b.total as f32;
                    c.attach_effect(&id, Effect::Impact { age, side: b.side }, EffectColor::srgb8(b.rgb[0], b.rgb[1], b.rgb[2]), size);
                }
                // Cleared on every tick it is not playing, so nothing is left on a parked object.
                None => {
                    if let Some(o) = c.get_game_object_mut(&id) { o.visible = false; }
                    c.clear_effect(&id);
                }
            }
        }
    }

    #[cfg(test)]
    pub fn playing(&self) -> usize { self.bursts.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fight::{Archetype, Enemy};

    #[test]
    fn a_hit_bursts_on_its_enemy_and_ages_on_fight_ticks_only() {
        let fight = Fight::new(vec![Enemy::new(Archetype::Husk, 10.0, 1)]);
        let mut fx = Fx::bare();
        fx.on_events(&[Event::Hit { enemy: 0, mv: Move::Light }, Event::Whiffed(Move::Light)], &fight, 0.0);
        assert_eq!(fx.playing(), 1, "a whiff makes no burst");
        let b = fx.bursts[0];
        assert_eq!((b.x, b.at, b.side), (LANE_W, At::Road(9.75), ImpactSide::Dealt));
        // A hitstop runs no fight ticks: the burst holds.
        fx.tick(0);
        assert_eq!(fx.bursts[0].left, b.total);
        fx.tick(b.total);
        assert_eq!(fx.playing(), 0);
    }

    #[test]
    fn being_struck_cracks_inward_at_the_hero_and_the_oldest_burst_gives_way() {
        let fight = Fight::new(Vec::new());
        let mut fx = Fx::bare();
        fx.on_events(&[Event::Struck { damage: 10.0, heavy: false }], &fight, 0.0);
        assert_eq!((fx.bursts[0].side, fx.bursts[0].at), (ImpactSide::Taken, At::Near(HERO_D - 0.05)));
        // Aged below a fresh block's life (16), so it is the first to give way.
        fx.tick(10);
        for _ in 0..BURSTS { fx.on_events(&[Event::Blocked { chip: 1.0 }], &fight, 0.0); }
        assert_eq!(fx.playing(), BURSTS);
        assert!(fx.bursts.iter().all(|b| b.side == ImpactSide::Dealt), "the aged strike was replaced");
    }
}
