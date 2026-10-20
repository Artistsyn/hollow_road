//! Slash and hit effects.
//!
//! A hit is a GPU burst (`Effect::Impact`, the one ball_swing_game's fights use) carried by a
//! transparent object that is placed on the road every tick, so it stays on what it hit while the
//! road and the camera move. Its shape says who was hit (`Dealt` punches outward, `Taken` cracks
//! inward), its colour how the blow was met. Bursts age on fight ticks: they hold at full
//! brightness through a hitstop and stretch out in slow motion, like the rest of the fight.
//!
//! The slash is Pixellab's crescent (`assets/pixellab/fx/slash.png`, made in its VFX workbench),
//! drawn as pixel art in front of the hero from the moment a swing goes live: flattened and
//! mirrored for the light's swipe from right to left, full height for the heavy coming down.

use crate::anim::Clip;
use crate::fight::{Event, Fight, Move, HERO_D, LANE_W, SHOULDER};
use quartz::*;
use quartz_path_forge::PathForgePlugin;
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

pub struct Fx {
    slash: Option<Clip>,
    bursts: Vec<Burst>,
}

impl Fx {
    pub fn load() -> Fx {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/pixellab/fx");
        // The export's layout names every frame; the sheet holds them in 96-pixel cells.
        let count = std::fs::read_to_string(dir.join("slash.json")).map(|j| j.matches("\"filename\"").count()).unwrap_or(0);
        Fx { slash: Clip::from_grid(&dir.join("slash.png"), 96, count), bursts: Vec::new() }
    }

    /// Start the bursts for what the fight just did.
    pub fn on_events(&mut self, events: &[Event], fight: &Fight) {
        let h = &fight.hero;
        // The hero as drawn, and a little in front of them (where a buckler meets a blade).
        let hero_x = h.x + SHOULDER;
        let on_enemy = |i: usize| fight.enemies.get(i).map(|e| (e.lane as f32 * LANE_W, At::Road(e.s - 0.25)));
        for e in events {
            let burst = |x: f32, at: At, y: f32, metres: f32, rgb: [u8; 3], side: ImpactSide, ticks: u32| {
                Burst { x, at, y, metres, rgb, side, left: ticks, total: ticks }
            };
            let b = match *e {
                Event::Hit { enemy, mv } => on_enemy(enemy).map(|(x, at)| match mv {
                    Move::Heavy => burst(x, at, 1.15, 1.7, [255, 196, 128], ImpactSide::Dealt, 24),
                    Move::Riposte => burst(x, at, 1.15, 1.9, [255, 150, 90], ImpactSide::Dealt, 32),
                    _ => burst(x, at, 1.15, 1.2, [255, 228, 184], ImpactSide::Dealt, 20),
                }),
                Event::StanceBroken { enemy } => on_enemy(enemy).map(|(x, at)| burst(x, at, 1.3, 2.2, [255, 246, 222], ImpactSide::Dealt, 30)),
                Event::Parried { .. } => Some(burst(hero_x + 0.2, At::Near(HERO_D + 0.6), 1.25, 2.0, [255, 214, 110], ImpactSide::Dealt, 26)),
                Event::Blocked { .. } => Some(burst(hero_x + 0.15, At::Near(HERO_D + 0.45), 1.1, 1.0, [170, 205, 255], ImpactSide::Dealt, 16)),
                Event::GuardBroken => Some(burst(hero_x + 0.15, At::Near(HERO_D + 0.4), 1.1, 1.7, [255, 140, 50], ImpactSide::Taken, 26)),
                Event::Struck { .. } => Some(burst(hero_x, At::Near(HERO_D - 0.05), 1.15, 1.5, [230, 40, 30], ImpactSide::Taken, 22)),
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

    /// The slash's frame now and where it goes: a swing from the moment its live frames begin.
    fn slash_now(&self, fight: &Fight) -> Option<(Arc<image::RgbaImage>, f32, f32, f32, (f32, f32))> {
        let clip = self.slash.as_ref()?;
        let a = fight.hero.act?;
        let windup = a.mv.spec().frames.0;
        let since = a.t.checked_sub(windup)?;
        let k = (since / SLASH_STEP) as usize;
        if k >= clip.frames.len() { return None; }
        let x = fight.hero.x + SHOULDER + 0.1;
        // (frame, across, ahead, height of its middle, width and height in metres)
        match a.mv {
            Move::Light => Some((clip.mirrored[k].clone(), x, HERO_D + 1.0, 1.0, (2.0, 1.0))),
            Move::Heavy => Some((clip.frames[k].clone(), x, HERO_D + 1.1, 1.2, (1.9, 2.1))),
            Move::Riposte => Some((clip.frames[k].clone(), x, HERO_D + 1.1, 1.25, (2.4, 2.6))),
            _ => None,
        }
    }

    /// Put the slash and the bursts where the road drawn now says they are.
    pub fn place(&self, c: &mut Canvas, walked: f64, fight: &Fight) {
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // Centred on a point `y` metres up: its top-left and size, in canvas units.
        let around = |x: f32, y: f32, d: f32, (w, h): (f32, f32)| -> Option<Spot> {
            if d < 0.5 { return None; }
            let (cx, cy) = pf.project_on(c, x, y, d)?;
            let ppm = pf.scale_on(c, d)?;
            Some(((cx - 0.5 * w * ppm, cy - 0.5 * h * ppm), (w * ppm, h * ppm)))
        };
        let slash = self.slash_now(fight).and_then(|(img, x, d, y, size)| Some((around(x, y, d, size)?, img)));
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
                    o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None });
                }
                None => o.visible = false,
            }
        }
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
        let mut fx = Fx { slash: None, bursts: Vec::new() };
        fx.on_events(&[Event::Hit { enemy: 0, mv: Move::Light }, Event::Whiffed(Move::Light)], &fight);
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
        let mut fx = Fx { slash: None, bursts: Vec::new() };
        fx.on_events(&[Event::Struck { damage: 10.0 }], &fight);
        assert_eq!((fx.bursts[0].side, fx.bursts[0].at), (ImpactSide::Taken, At::Near(HERO_D - 0.05)));
        // Aged below a fresh block's life (16), so it is the first to give way.
        fx.tick(10);
        for _ in 0..BURSTS { fx.on_events(&[Event::Blocked { chip: 1.0 }], &fight); }
        assert_eq!(fx.playing(), BURSTS);
        assert!(fx.bursts.iter().all(|b| b.side == ImpactSide::Dealt), "the aged strike was replaced");
    }
}
