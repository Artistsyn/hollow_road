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
/// Fight ticks a burst drawn from a clip lives.
const CLIP_TICKS: u32 = 48;

/// Where a burst stands along the road: beside the hero (metres from the camera, so it stays
/// with them), or where an enemy was hit (metres walked, so it stays on the enemy).
#[derive(Clone, Copy, Debug, PartialEq)]
enum At { Near(f32), Road(f64) }

/// A hit burst playing: across the road, along it, its height and size in metres, colour, shape,
/// and fight ticks left of its whole life; drawn from `clip` (`quartz_path_forge::Vfx`, what was
/// struck: steel, flesh, a heavy landing) when that is loaded, else as the GPU impact. `from`:
/// where the blow came from (across, metres ahead of the camera); a clip thrown one way is turned
/// to fly on away from it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Burst { x: f32, at: At, y: f32, metres: f32, rgb: [u8; 3], side: ImpactSide, left: u32, total: u32, clip: Option<&'static str>, from: Option<(f32, f32)> }

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

/// Strike art drawn bulging toward the camera, turned upside down as it loads. A flat cut round
/// the hero passes farthest from the camera straight ahead of them, so seen from behind its arc
/// bows away, toward the foe: its middle high, its ends low.
const UPTURNED: [&str; 1] = ["light_slash"];

impl Strike {
    /// The effect `name` laid as `lie`; `flip` turns it upside down, so a falling swing's art draws
    /// a rising one (art in `UPTURNED` is turned once more).
    fn load(dir: &Path, name: &str, lie: Lie, flip: bool) -> Option<Strike> {
        let mut clip = effect(dir, name)?;
        if flip != UPTURNED.contains(&name) {
            let upturned = |frames: &[Arc<image::RgbaImage>]| frames.iter().map(|f| Arc::new(image::imageops::flip_vertical(f.as_ref()))).collect::<Vec<_>>();
            (clip.frames, clip.mirrored) = (upturned(&clip.frames), upturned(&clip.mirrored));
        }
        if let Lie::Along { .. } = lie {
            clip.frames = clip.frames.iter().map(|f| Arc::new(squared(f))).collect();
        }
        for f in clip.frames.iter().chain(&clip.mirrored) { quartz::set_pixel_art(f, true); }
        Some(Strike { clip, lie })
    }

    /// The trail (file, lie, upside down) of a swing that ends a string (`combo::Route::clip_for`:
    /// `ENDERS`), drawn along its own path rather than its light's or heavy's: flat cuts lie
    /// across the lanes (mirrored for one going left to right), thrusts streak up the lane,
    /// `greatsword_light` falls from high left (mirrored: from high right), and `greatsword_rising`
    /// (bowing toward the camera as it falls) turned upside down climbs from low right to high
    /// left, mirrored from low left; every arc bows toward the foe. The axe's cleave and the
    /// ultra's slam come down as their heavies do.
    fn ender(class: Class, name: &str) -> Option<(&'static str, Lie, bool)> {
        let across = |width, ahead, high, mirror| Lie::Across { width, ahead, high, land: None, ground: false, mirror };
        let along = |past, from, high| Lie::Along { past, from, high };
        let chop = |width, mirror| Lie::Across { width, ahead: 2.0, high: 0.9, land: Some((0.55, 0.89)), ground: true, mirror };
        let (file, lie, flip) = match (class, name) {
            (Class::Sword, "third") => ("greatsword_rising", across(2.6, 1.8, 1.2, false), true),
            (Class::Sword, "breaker") => ("thrust_heavy", along(1.2, 0.3, (1.2, 1.25)), false),
            (Class::Sword, "follow") => ("light_slash", across(2.8, 1.9, 1.25, true), false),
            (Class::Sword, "finisher") => ("light_slash", across(3.6, 1.8, 1.15, false), false),
            (Class::Thrust, "third") => ("greatsword_light", across(2.0, 1.8, 1.1, true), false),
            (Class::Thrust, "breaker") => ("thrust_heavy", along(1.6, 0.3, (0.8, 1.25)), false),
            (Class::Thrust, "follow") => ("greatsword_rising", across(2.2, 1.8, 1.2, true), true),
            (Class::Thrust, "finisher") => ("thrust_heavy", along(2.4, 0.3, (1.6, 1.0)), false),
            (Class::Spear, "third") => ("spear_light", along(1.0, 0.2, (0.7, 1.3)), false),
            (Class::Spear, "breaker") => ("spear_light", along(0.5, 0.2, (1.2, 1.2)), false),
            (Class::Spear, "follow") => ("greatsword_light", across(2.2, 1.8, 1.2, true), false),
            (Class::Spear, "finisher") => ("light_slash", across(3.8, 1.8, 0.9, false), false),
            (Class::Axe, "third") => ("axe_light", across(1.8, 1.6, 1.0, true), false),
            (Class::Axe, "breaker") => ("greatsword_rising", across(1.6, 1.6, 1.1, false), true),
            (Class::Axe, "follow") => ("axe_heavy", chop(1.6, true), false),
            (Class::Greatsword, "third") => ("light_slash", across(3.4, 1.8, 0.8, false), false),
            (Class::Greatsword, "breaker") => ("thrust_heavy", along(1.6, 0.4, (1.2, 1.2)), false),
            (Class::Greatsword, "follow") => ("greatsword_light", across(2.6, 1.6, 1.5, false), false),
            (Class::Greatsword, "finisher") => ("greatsword_rising", across(3.8, 1.6, 1.3, false), true),
            (Class::Ultra, "third") => ("greatsword_rising", across(3.8, 1.8, 1.2, true), true),
            (Class::Ultra, "breaker") => ("thrust_heavy", along(2.0, 0.4, (1.0, 1.0)), false),
            (Class::Ultra, "follow") => ("light_slash", across(4.2, 2.1, 0.6, true), false),
            _ => return None,
        };
        Some((file, lie, flip))
    }
}

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
/// The swings that end a string with a trail of their own (`Strike::ender`), as `Route::clip_for` names them.
const ENDERS: [&str; 4] = ["third", "breaker", "follow", "finisher"];

pub struct Fx {
    /// Per kind of weapon (`CLASSES` order), its light and its heavy.
    strikes: Vec<[Option<Strike>; 2]>,
    /// Per kind of weapon, the trail of each swing that ends a string (`ENDERS` order) where it has
    /// its own.
    enders: Vec<[Option<Strike>; 4]>,
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
            Strike::load(&dir, name, lie, false)
        })).collect();
        let enders = CLASSES.iter().map(|&class| ENDERS.map(|ender| {
            let (name, lie, flip) = Strike::ender(class, ender)?;
            Strike::load(&dir, name, lie, flip)
        })).collect();
        Fx { strikes, enders, turned: RefCell::new(HashMap::new()), bursts: Vec::new() }
    }

    #[cfg(test)]
    fn bare() -> Fx { Fx { strikes: Vec::new(), enders: Vec::new(), turned: RefCell::new(HashMap::new()), bursts: Vec::new() } }

    /// Start the bursts for what the fight just did.
    pub fn on_events(&mut self, events: &[Event], fight: &Fight, walked: f64) {
        // Nothing on one got past: it is behind the camera's view of the hero, not drawn.
        let on_enemy = |i: usize| fight.enemies.get(i).filter(|e| e.gap(walked) > -0.6).map(|e| (e.lane as f32 * LANE_W, e.s - 0.25, "hit_flesh"));
        self.on_events_at(events, (fight.hero.x + SHOULDER, HERO_D), 1.0, on_enemy);
    }

    /// The same for a hero standing `hero` (across, metres ahead of the camera) facing `fw` along
    /// the road (1 away from the camera, -1 toward it), and enemies where `foes` says (across, road
    /// metres, the clip of a blow landing on its body): the side-on fight, whose hero faces along
    /// the road either way.
    pub fn on_events_at(&mut self, events: &[Event], hero: (f32, f32), fw: f32, foes: impl Fn(usize) -> Option<(f32, f64, &'static str)>) {
        // The hero as drawn, and a little in front of them (where a buckler meets a blade).
        let (hero_x, hd) = hero;
        let on_enemy = |i: usize| foes(i).map(|(x, s, _)| (x, At::Road(s)));
        for e in events {
            let burst = |x: f32, at: At, y: f32, metres: f32, rgb: [u8; 3], side: ImpactSide, ticks: u32| {
                Burst { x, at, y, metres, rgb, side, left: ticks, total: ticks, clip: None, from: None }
            };
            let b = match *e {
                Event::Hit { enemy, mv } => on_enemy(enemy).map(|(x, at)| match mv {
                    Move::Heavy => burst(x, at, 1.15, 1.7, [255, 196, 128], ImpactSide::Dealt, 24),
                    Move::Art => burst(x, at, 1.15, 1.8, [210, 200, 255], ImpactSide::Dealt, 26),
                    Move::Riposte => burst(x, at, 1.15, 1.9, [255, 150, 90], ImpactSide::Dealt, 32),
                    _ => burst(x, at, 1.15, 1.2, [255, 228, 184], ImpactSide::Dealt, 20),
                }),
                Event::Deflected { enemy } => on_enemy(enemy).map(|(x, at)| burst(x - 0.2, at, 1.1, 1.0, [200, 222, 255], ImpactSide::Dealt, 14)),
                // An ally's blow or arrow: smaller and warmer than the hero's own.
                // Melting into the dark: a violet smoke where it stood.
                Event::Fled { enemy, at } => on_enemy(enemy).map(|(x, _)| burst(x, At::Road(at), 1.0, 2.4, [150, 90, 220], ImpactSide::Dealt, 40)),
                Event::AllyHit { enemy } => on_enemy(enemy).map(|(x, at)| burst(x, at, 1.2, 1.0, [255, 214, 160], ImpactSide::Dealt, 16)),
                Event::StanceBroken { enemy } => on_enemy(enemy).map(|(x, at)| burst(x, at, 1.3, 2.2, [255, 246, 222], ImpactSide::Dealt, 30)),
                // A finisher's slam: the ground cratered ahead, or a sweep's dust thrown low.
                Event::Finished(crate::combo::Finish::Quake) => Some(burst(hero_x, At::Near(hd + fw * 2.2), 0.0, 3.0, [214, 196, 168], ImpactSide::Dealt, 40)),
                Event::Finished(crate::combo::Finish::Sweep) => Some(burst(hero_x, At::Near(hd + fw * 1.6), 0.0, 2.6, [214, 196, 168], ImpactSide::Dealt, 34)),
                // A gathered heavy: the strength held in the blade bursts on the body (a hot crack
                // for one part gathered).
                Event::ChargedBlow { enemy, full } => on_enemy(enemy).map(|(x, at)| burst(x, at, 1.15, if full { 1.8 } else { 2.1 }, [255, 214, 150], ImpactSide::Dealt, 28)),
                // A perfect roll: a cold flash at its feet as it stumbles through empty air.
                Event::JustDodge { enemy } => on_enemy(enemy).map(|(x, at)| burst(x, at, 0.55, 1.8, [150, 225, 255], ImpactSide::Dealt, 28)),
                // The glint on the attacker the moment its attack commits, in its warning colour.
                Event::Commits { enemy, kind, parry } => on_enemy(enemy).map(|(x, at)| burst(x + 0.15, at, 1.55, 0.7, crate::game::warning_rgb(kind, parry), ImpactSide::Dealt, 12)),
                // The string's snap or the hex leaving the hand, small and bright. A rune going off
                // is its own drawn eruption (`Pilgrimage::rune_clip`), so it adds nothing here.
                Event::Loosed { enemy } => on_enemy(enemy).map(|(x, at)| burst(x + 0.1, at, 1.4, 0.55, [255, 236, 200], ImpactSide::Dealt, 10)),
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
            let Some(mut b) = b else { continue };
            if matches!(e, Event::ChargedBlow { .. }) { b.from = Some(hero); }
            b.clip = match *e {
                Event::Hit { enemy, .. } | Event::AllyHit { enemy } | Event::StanceBroken { enemy } => foes(enemy).map(|f| f.2),
                _ => Self::material(e),
            };
            // A landing's dust and cracks stand on the ground, not at the blast's height.
            if b.clip.is_some_and(|k| k.starts_with("leap_land")) { b.y = 0.0; }
            // A clip plays out at its own pace (most under 0.8 s; a one-shot then holds its faded
            // last frame), however short the impact it stands in for.
            if b.clip.is_some() { (b.left, b.total) = (b.total.max(CLIP_TICKS), b.total.max(CLIP_TICKS)); }
            self.push(b);
            // Knocked back by a full one: heavy dust thrown up where its feet skid.
            if let Event::ChargedBlow { enemy, full: true } = *e {
                if let Some((x, at)) = on_enemy(enemy) {
                    self.push(Burst { x, at, y: 0.0, metres: 1.6, rgb: [214, 196, 168], side: ImpactSide::Dealt, left: CLIP_TICKS, total: CLIP_TICKS, clip: Some("leap_land_heavy"), from: None });
                }
            }
        }
    }

    /// Start burst `b`, the one nearest its end making way when all are in use.
    fn push(&mut self, b: Burst) {
        if self.bursts.len() >= BURSTS {
            if let Some(oldest) = (0..self.bursts.len()).min_by_key(|&i| self.bursts[i].left) { self.bursts.remove(oldest); }
        }
        self.bursts.push(b);
    }

    /// The clip of a blow landing on a boss whose art is in `folder`: bone, bronze, stone, a
    /// ghost's wisps, or flesh.
    pub fn body_of(folder: Option<&str>) -> &'static str {
        match folder {
            Some("keening_cage") => "hit_bone",
            Some("bell_warden") => "hit_steel",
            Some("statue_archangel" | "radiant_construct" | "rune_sentinel") => "hit_stone",
            Some("wraith_lord" | "fog_horror" | "void_stalker" | "lurz_echo" | "krix_echo" | "bounty_echo") => "hit_spectral",
            _ => "hit_flesh",
        }
    }

    /// The drawn clip for what the fight just did, by what met what: steel on steel for a block or
    /// a parry, flesh for a blow that lands on the hero or a foe, a landing for a slam (and the
    /// ground a ring sets out from).
    fn material(e: &Event) -> Option<&'static str> {
        match e {
            Event::Blocked { .. } | Event::Parried { .. } | Event::PerfectBlock | Event::Deflected { .. } | Event::Glanced { .. } => Some("hit_steel"),
            Event::Hit { .. } | Event::AllyHit { .. } | Event::Struck { .. } | Event::Staggered { .. } => Some("hit_flesh"),
            Event::Slam { heavy: true, .. } | Event::KnockedDown { .. } | Event::Rung { .. } => Some("leap_land_heavy"),
            // The blow committing: a glint of steel on the attacker, never a coloured flare.
            Event::Commits { .. } => Some("tell_steel_glint"),
            Event::ChargedBlow { full: true, .. } => Some("hit_crush"),
            Event::Finished(crate::combo::Finish::Quake) => Some("brute_slam_crater"),
            Event::Finished(crate::combo::Finish::Sweep) => Some("leap_land_heavy"),
            Event::Slam { .. } => Some("leap_land_light"),
            _ => None,
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
        // A string's ender has its own trail, kept apart (its kind past the plain ones') in the
        // turned cache.
        let ender = matches!(a.mv, Move::Light | Move::Heavy).then(|| a.route.clip_for(a.mv, a.link)).flatten()
            .and_then(|name| ENDERS.iter().position(|e| *e == name))
            .zip(CLASSES.iter().position(|&c| c == class))
            .and_then(|(e, k)| Some((self.enders.get(k)?[e].as_ref()?, k + CLASSES.len() * (1 + e))));
        let (strike, kind) = match ender {
            Some(found) => found,
            None => {
                let kind = CLASSES.iter().position(|&c| c == class).filter(|&k| self.strikes.get(k).is_some_and(|s| s[heavy as usize].is_some())).unwrap_or(0);
                (self.strikes.get(kind)?[heavy as usize].as_ref()?, kind)
            }
        };
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
        let act = fight.hero.act?;
        // Gathered to the full, a finisher's trail reaches as far and as wide as its finishing blow.
        let spec = if act.route == crate::combo::Route::Finisher && act.charge.is_full() {
            crate::combo::Finish::of(fight.hero.stats.kit.swung_as()).widen(act.spec)
        } else {
            act.spec
        };
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
    pub fn place(&self, c: &mut Canvas, walked: f64, fight: &Fight, yard: bool, vfx: &quartz_path_forge::Vfx) {
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // Centred on a point `y` metres up: its top-left and size, in canvas units.
        let around = |x: f32, y: f32, d: f32, (w, h): (f32, f32)| -> Option<Spot> {
            if d < 0.5 && !yard { return None; }
            let (cx, cy) = pf.project_on(c, x, y, d)?;
            let ppm = pf.scale_on(c, d)?;
            Some(((cx - 0.5 * w * ppm, cy - 0.5 * h * ppm), (w * ppm, h * ppm)))
        };
        let slash = self.strike_spot(c, pf, fight);
        // A burst with its clip loaded is that clip's frame, at its own size; the rest the impact.
        let bursts: Vec<Option<(Spot, Burst, Option<Arc<image::RgbaImage>>)>> = (0..BURSTS).map(|i| {
            let b = *self.bursts.get(i)?;
            let d = match b.at { At::Near(d) => d, At::Road(s) => (s - walked) as f32 };
            if d < 0.5 && !yard { return None; }
            let clip = b.clip.filter(|k| vfx.has(k));
            // From the clip's own moment (its hit frame): what the event stands for. Played out,
            // it is gone (not held on its last frame, nor drawn the old way).
            let t = clip.map_or(0.0, |k| (b.total - b.left) as f32 / 60.0 + vfx.cued(k, 1.0, None));
            if clip.is_some_and(|k| t >= vfx.length(k)) { return None; }
            let drawn = clip.and_then(|clip| {
                let left = b.from.and_then(|(x, from)| Some(pf.project_on(c, b.x, b.y, d)?.0 < pf.project_on(c, x, b.y, from)?.0)).unwrap_or(false);
                let play = quartz_path_forge::VfxPlay { clip, x: b.x, y: b.y, along: d, t, left, layer: 65 };
                vfx.place(c, pf, &[play]).into_iter().next().flatten()
            });
            match drawn {
                Some((spot, img, _)) => Some((spot, b, Some(img))),
                None => Some((around(b.x, b.y, d, (b.metres, b.metres))?, b, None)),
            }
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
                Some(((pos, size), _, Some(img))) => {
                    c.clear_effect(&id);
                    if let Some(o) = c.get_game_object_mut(&id) {
                        (o.visible, o.position, o.size) = (true, pos, size);
                        o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None, recolor: None });
                    }
                }
                Some(((pos, size), b, None)) => {
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

    /// A finisher swings its own trail where its kind has one, upside down for the rising sweep,
    /// and the weapon's heavy trail otherwise.
    #[test]
    fn a_finisher_draws_its_own_trail_and_the_rising_sweep_climbs() {
        let fx = Fx::load();
        for (k, &class) in CLASSES.iter().enumerate() {
            for (e, ender) in ENDERS.iter().enumerate() {
                assert_eq!(fx.enders[k][e].is_some(), Strike::ender(class, ender).is_some(), "{class:?}'s {ender} trail did not load");
            }
        }
        let sweep = fx.enders[CLASSES.iter().position(|&c| c == Class::Greatsword).unwrap()][3].as_ref().unwrap();
        let plain = effect(&crate::assets::path("pixellab/fx"), "greatsword_rising").unwrap();
        let (a, b) = (&sweep.clip.frames[1], &plain.frames[1]);
        assert!(a.as_raw() == image::imageops::flip_vertical(b.as_ref()).as_raw(), "turned upside down");
        // The wheeling cut's arc bows toward the foe: its middle higher on screen than its ends.
        let cut = &fx.enders[0][3].as_ref().unwrap().clip.frames[2];
        let height = |x: u32| { let ys: Vec<u32> = (0..cut.height()).filter(|&y| cut.get_pixel(x, y)[3] > 100).collect(); ys.iter().sum::<u32>() as f32 / ys.len().max(1) as f32 };
        let lit: Vec<u32> = (0..cut.width()).filter(|&x| (0..cut.height()).any(|y| cut.get_pixel(x, y)[3] > 100)).collect();
        let (first, last) = (lit[0], lit[lit.len() - 1]);
        assert!(height((first + last) / 2) < height(first).min(height(last)), "the flat slash sags toward the camera");
        let mut fight = Fight::new(vec![Enemy::new(Archetype::Husk, 10.0, 1)]);
        let spec = Move::Heavy.spec_for(Class::Sword, None);
        fight.hero.act = Some(crate::fight::Act { mv: Move::Heavy, spec, t: spec.frames.0, landed: false, from_lane: 0, from_x: 0.0, link: 0, charge: Default::default(), route: crate::combo::Route::Finisher });
        let (_, kind, ..) = fx.strike_now(&fight).expect("a finisher's trail plays");
        assert_eq!(kind, CLASSES.len() * 4, "the sword's finisher trail, not its heavy's");
    }

    /// Every flat or diagonal trail bows toward the foe: seen from behind, a cut round the hero is
    /// farthest from the camera straight ahead, so the middle of its arc stands above the line
    /// between its ends. (The overhead chops, drawn up and down the screen, are not arcs across.)
    #[test]
    fn every_arc_across_the_road_bows_toward_the_foe() {
        let fx = Fx::load();
        // How far the middle of the arc stands above the line between its two farthest points
        // (positive: bowed up the screen, toward the foe).
        let bow = |f: &image::RgbaImage| -> f32 {
            let lit: Vec<(f32, f32)> = f.enumerate_pixels().filter(|p| p.2[3] > 100).map(|p| (p.0 as f32, p.1 as f32)).collect();
            let gap = |p: (f32, f32), q: (f32, f32)| (p.0 - q.0).hypot(p.1 - q.1);
            let (a, b) = lit.iter().flat_map(|&p| lit.iter().map(move |&q| (p, q))).fold(((0.0, 0.0), (0.0, 0.0)), |m, pq| if gap(pq.0, pq.1) > gap(m.0, m.1) { pq } else { m });
            let length = gap(a, b);
            let along = ((b.0 - a.0) / length, (b.1 - a.1) / length);
            let up = if along.0 > 0.0 { (along.1, -along.0) } else { (-along.1, along.0) };
            let middle: Vec<f32> = lit.iter().filter(|p| ((p.0 - a.0) * along.0 + (p.1 - a.1) * along.1 - 0.5 * length).abs() < 0.1 * length)
                .map(|p| (p.0 - a.0) * up.0 + (p.1 - a.1) * up.1).collect();
            middle.iter().sum::<f32>() / middle.len().max(1) as f32
        };
        let at = |class: Class| CLASSES.iter().position(|&c| c == class).unwrap();
        let arcs = [("sword light", &fx.strikes[at(Class::Sword)][0]), ("axe light", &fx.strikes[at(Class::Axe)][0]), ("greatsword light", &fx.strikes[at(Class::Greatsword)][0]),
            ("sword finisher", &fx.enders[at(Class::Sword)][3]), ("greatsword finisher", &fx.enders[at(Class::Greatsword)][3])];
        // And every ender's flat or diagonal trail (a chop's lands on the ground: not an arc).
        let enders = CLASSES.iter().enumerate().flat_map(|(k, &class)| ENDERS.iter().enumerate().map(move |(e, ender)| (k, e, class, *ender)))
            .filter(|&(_, _, class, ender)| matches!(Strike::ender(class, ender), Some((_, Lie::Across { land: None, .. }, _))))
            .map(|(k, e, ..)| ("an ender", &fx.enders[k][e]));
        for (name, strike) in arcs.into_iter().chain(enders) {
            let clip = &strike.as_ref().unwrap().clip;
            let fullest = clip.frames.iter().max_by_key(|f| f.pixels().filter(|p| p[3] > 100).count()).unwrap();
            assert!(bow(fullest) > 1.0, "{name}: its arc bows {} px toward the foe (negative: toward the camera)", bow(fullest));
        }
    }
}
