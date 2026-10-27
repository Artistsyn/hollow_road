//! The look of the capstones worn (`capstone::Capstone`), laid on the hero's frame where each
//! piece is drawn and nowhere else: a faint rim of the capstone's colour round the weapon's or
//! the offhand's own pixels (`anim::Glint`), breathing slowly, and now and then a mote off it,
//! moving as its kind does (`capstone::Motes`). Armour has no pixels of its own, so its capstone
//! rims the figure's edge across the band of the body the piece covers. Kept faint on purpose:
//! seen when looked for, never louder than the hero.

use crate::anim::Glint;
use crate::capstone::{Capstone, Motes};
use image::RgbaImage;
use std::sync::Arc;

/// Most motes alive at once, all pieces together.
const MOST_MOTES: usize = 18;
/// How strong the aura is round a piece, its first ring and the softer one outside it, and round
/// the figure for armour (of full colour, at the top of a breath).
const RIM: f32 = 0.5;
const OUTER: f32 = 0.2;
const ARMOUR_RIM: f32 = 0.24;
/// How much a piece's own edge is tinted toward its colour.
const EDGE_TINT: f32 = 0.2;

/// A mote off a piece, in the frame's pixels: where it is, how old, how long it lives, and how
/// it moves and looks.
#[derive(Clone, Copy, Debug)]
struct Mote { x: f32, y: f32, age: u32, life: u32, kind: Motes, colour: [u8; 3] }

/// The capstones' look, kept from frame to frame for the motes.
#[derive(Default)]
pub struct Sheen {
    motes: Vec<Mote>,
    last: u64,
    made: Option<(usize, u64, Arc<RgbaImage>)>,
}

/// Where the pieces are in the frame: a mask of the weapon's and offhand's pixels, and the
/// figure's top and bottom rows.
struct Pieces { mask: Vec<bool>, width: u32, top: u32, bottom: u32 }

/// A capstone's colour, and how strong it is laid now.
#[derive(Clone, Copy)]
struct Paint { colour: [u8; 3], strength: f32 }

impl Sheen {
    /// `img` with the look of `worn` (by `gear::SLOTS` place) laid on the pieces `glint` finds in
    /// it, at `tick`; `img` itself if nothing worn has a capstone.
    pub fn dress(&mut self, img: Arc<RgbaImage>, glint: Option<&Glint>, worn: &[Option<Capstone>; 6], tick: u64) -> Arc<RgbaImage> {
        if worn.iter().all(Option::is_none) {
            self.motes.clear();
            return img;
        }
        let key = Arc::as_ptr(&img) as usize;
        if let Some((k, t, made)) = &self.made { if *k == key && *t == tick { return made.clone(); } }
        let empty = Glint::default();
        let glint = glint.unwrap_or(&empty);
        let pieces = Pieces::find(&img, glint);
        self.age(tick);
        self.spawn(glint, worn, tick, &img, &pieces);
        let mut out = (*img).clone();
        for (place, cap) in worn.iter().enumerate() {
            let Some(cap) = cap else { continue };
            let breath = Self::breath(*cap, tick, place);
            match place {
                0 => Self::rim(&mut out, &img, &glint.weapon, &pieces, Paint { colour: cap.colour(), strength: breath }),
                1 => Self::rim(&mut out, &img, &glint.off, &pieces, Paint { colour: cap.colour(), strength: breath }),
                _ => Self::armour_rim(&mut out, &img, &pieces, place, Paint { colour: cap.colour(), strength: ARMOUR_RIM * breath }),
            }
        }
        for mote in &self.motes { Self::mote(&mut out, mote); }
        let made = Arc::new(out);
        quartz::set_pixel_art(&made, true);
        self.made = Some((key, tick, made.clone()));
        made
    }

    /// How strong the look is at `tick`: a slow breath, a glow alone deeper and slower; pieces
    /// out of step with each other.
    fn breath(cap: Capstone, tick: u64, place: usize) -> f32 {
        let (period, depth) = if cap.motes() == Motes::Glow { (200.0, 0.35) } else { (130.0, 0.25) };
        let t = tick as f32 / period + place as f32 * 0.37;
        1.0 - depth + depth * (std::f32::consts::TAU * t).sin()
    }

    fn age(&mut self, tick: u64) {
        let dt = tick.saturating_sub(self.last).min(12) as u32;
        self.last = tick;
        for mote in &mut self.motes {
            mote.age += dt;
            let through = mote.age as f32 / mote.life as f32;
            match mote.kind {
                Motes::Rise => { mote.y -= 0.35 * dt as f32; mote.x += 0.12 * dt as f32 * (mote.age as f32 * 0.21).sin(); }
                Motes::Drip => mote.y += dt as f32 * 0.9 * through * through,
                Motes::Twinkle | Motes::Glow => {}
            }
        }
        self.motes.retain(|m| m.age < m.life);
    }

    fn spawn(&mut self, glint: &Glint, worn: &[Option<Capstone>; 6], tick: u64, img: &RgbaImage, pieces: &Pieces) {
        for (place, cap) in worn.iter().enumerate() {
            let Some(cap) = cap else { continue };
            let (every, life) = match cap.motes() { Motes::Rise => (8, 44), Motes::Twinkle => (7, 26), Motes::Drip => (20, 48), Motes::Glow => continue };
            if (tick + 5 * place as u64) % every != 0 || self.motes.len() >= MOST_MOTES { continue; }
            let from = match place { 0 => glint.weapon.clone(), 1 => glint.off.clone(), _ => pieces.band_edge(img, place) };
            if from.is_empty() { continue; }
            let roll = Self::hash(tick as u32, place as u32);
            let (x, y) = from[roll as usize % from.len()];
            self.motes.push(Mote { x: x as f32 + 0.5, y: y as f32 + 0.5, age: 0, life, kind: cap.motes(), colour: cap.colour() });
        }
    }

    /// An aura round the piece at `px`, on the clear pixels about it and never on the body: a
    /// ring beside it and a softer one outside that, each pixel laid once; and a little tint on
    /// the piece's own edge.
    fn rim(out: &mut RgbaImage, img: &RgbaImage, px: &[(u16, u16)], pieces: &Pieces, paint: Paint) {
        let (w, h) = img.dimensions();
        let mut ring = vec![0u8; (w * h) as usize];
        let free = |x: i64, y: i64, ring: &[u8]| {
            x >= 0 && y >= 0 && x < w as i64 && y < h as i64 && !pieces.has(x as u32, y as u32)
                && ring[(y as u32 * w + x as u32) as usize] == 0 && img.get_pixel(x as u32, y as u32).0[3] < 128
        };
        let mut first = Vec::new();
        for &(x, y) in px {
            let (x, y) = (x as i64, y as i64);
            let mut edge = false;
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                let outside = nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 && !pieces.has(nx as u32, ny as u32);
                edge |= outside;
                if free(nx, ny, &ring) { ring[(ny as u32 * w + nx as u32) as usize] = 1; first.push((nx, ny)); }
            }
            if edge { Self::tint(out, (x as u32, y as u32), paint.colour, EDGE_TINT * paint.strength); }
        }
        for &(x, y) in &first {
            Self::lay(out, (x as u32, y as u32), paint.colour, RIM * paint.strength);
            for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if !free(nx, ny, &ring) { continue; }
                ring[(ny as u32 * w + nx as u32) as usize] = 2;
                Self::lay(out, (nx as u32, ny as u32), paint.colour, OUTER * paint.strength);
            }
        }
    }

    fn armour_rim(out: &mut RgbaImage, img: &RgbaImage, pieces: &Pieces, place: usize, paint: Paint) {
        let (w, h) = img.dimensions();
        for (x, y) in pieces.band_edge(img, place) {
            let (x, y) = (x as i64, y as i64);
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 { continue; }
                if img.get_pixel(nx as u32, ny as u32).0[3] < 128 { Self::lay(out, (nx as u32, ny as u32), paint.colour, paint.strength); }
            }
        }
    }

    /// A foe burning under an ember capstone (`fight::Burn`), at `tick`: an ember glow licking up
    /// off the tops of its outline, flickering, and a spark or two rising. Made fresh each tick
    /// while it burns (three seconds), so it is not kept.
    pub fn smouldering(img: &RgbaImage, tick: u64) -> Arc<RgbaImage> {
        const EMBER: [u8; 3] = [255, 120, 40];
        let mut out = img.clone();
        let (w, h) = img.dimensions();
        let solid = |x: u32, y: u32| img.get_pixel(x, y).0[3] >= 128;
        let mut tops = Vec::new();
        for y in 1..h {
            for x in 0..w {
                if !solid(x, y) || solid(x, y - 1) { continue; }
                tops.push((x, y));
                let flicker = 0.5 + 0.5 * ((x as f32 * 0.9 + tick as f32 * 0.35).sin() * (x as f32 * 0.37 - tick as f32 * 0.21).cos());
                Self::lay(&mut out, (x, y - 1), EMBER, 0.38 * flicker);
                Self::tint(&mut out, (x, y), EMBER, 0.22 * flicker);
            }
        }
        for n in 0..3u32 {
            if tops.is_empty() { break; }
            let life = 30u64;
            let born = tick.saturating_sub((tick + 11 * n as u64) % life);
            let (x, y) = tops[Self::hash(born as u32, n) as usize % tops.len()];
            let rise = (tick - born) as f32 * 0.4;
            let alpha = 0.8 * (1.0 - (tick - born) as f32 / life as f32);
            if (y as f32) > rise + 1.0 { Self::lay(&mut out, (x, (y as f32 - 1.0 - rise) as u32), EMBER, alpha); }
        }
        let made = Arc::new(out);
        quartz::set_pixel_art(&made, true);
        made
    }

    /// A twinkle at its brightest is a small cross; any other mote a pixel.
    fn mote(out: &mut RgbaImage, mote: &Mote) {
        let through = mote.age as f32 / mote.life as f32;
        let alpha = match mote.kind { Motes::Twinkle => (std::f32::consts::PI * through).sin(), _ => (through / 0.15).min(1.0) * (1.0 - through) };
        let (w, h) = out.dimensions();
        let at = |dx: f32, dy: f32| { let (x, y) = (mote.x + dx, mote.y + dy); (x >= 0.0 && y >= 0.0 && x < w as f32 && y < h as f32).then(|| (x as u32, y as u32)) };
        if let Some(px) = at(0.0, 0.0) { Self::lay(out, px, mote.colour, 0.85 * alpha); }
        if mote.kind == Motes::Twinkle && alpha > 0.8 {
            for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] { if let Some(px) = at(dx, dy) { Self::lay(out, px, mote.colour, 0.35 * alpha); } }
        }
    }

    /// `colour` over what is there at `alpha`, or alone on a clear pixel.
    fn lay(out: &mut RgbaImage, at: (u32, u32), colour: [u8; 3], alpha: f32) {
        let pixel = out.get_pixel_mut(at.0, at.1);
        let [r, g, b, below] = pixel.0;
        let alpha = alpha.clamp(0.0, 1.0);
        let under = below as f32 / 255.0;
        let total = alpha + under * (1.0 - alpha);
        if total <= 0.0 { return; }
        let mix = |c: u8, o: u8| ((c as f32 * alpha + o as f32 * under * (1.0 - alpha)) / total) as u8;
        pixel.0 = [mix(colour[0], r), mix(colour[1], g), mix(colour[2], b), (255.0 * total) as u8];
    }

    fn tint(out: &mut RgbaImage, at: (u32, u32), colour: [u8; 3], share: f32) {
        let pixel = out.get_pixel_mut(at.0, at.1);
        for c in 0..3 { pixel.0[c] = (pixel.0[c] as f32 * (1.0 - share) + colour[c] as f32 * share) as u8; }
    }

    fn hash(first: u32, second: u32) -> u32 {
        let mixed = first.wrapping_mul(0x9E37_79B9) ^ second.wrapping_mul(0x85EB_CA6B);
        (mixed ^ (mixed >> 15)).wrapping_mul(0x2C1B_3C6D) ^ (mixed >> 13)
    }
}

impl Pieces {
    fn find(img: &RgbaImage, glint: &Glint) -> Pieces {
        let (w, h) = img.dimensions();
        let mut mask = vec![false; (w * h) as usize];
        for &(x, y) in glint.weapon.iter().chain(&glint.off) {
            if (x as u32) < w && (y as u32) < h { mask[(y as u32 * w + x as u32) as usize] = true; }
        }
        let body = |y: u32| (0..w).any(|x| img.get_pixel(x, y).0[3] > 128 && !mask[(y * w + x) as usize]);
        let top = (0..h).find(|&y| body(y)).unwrap_or(0);
        let bottom = (0..h).rev().find(|&y| body(y)).unwrap_or(h.saturating_sub(1));
        Pieces { mask, width: w, top, bottom }
    }

    fn has(&self, x: u32, y: u32) -> bool { self.mask[(y * self.width + x) as usize] }

    /// The figure's edge pixels (solid, beside a clear one, not a weapon's or offhand's) across
    /// the band armour worn at `place` covers: the head's top fifth, the body's chest and back,
    /// the hands' forearms, the legs' lower two fifths.
    fn band_edge(&self, img: &RgbaImage, place: usize) -> Vec<(u16, u16)> {
        let (from, to) = match place { 2 => (0.0, 0.2), 3 => (0.2, 0.55), 4 => (0.4, 0.62), _ => (0.6, 1.0) };
        let span = (self.bottom - self.top) as f32;
        let (y0, y1) = (self.top + (span * from) as u32, self.top + (span * to) as u32);
        let (w, h) = img.dimensions();
        let clear = |x: i64, y: i64| x < 0 || y < 0 || x >= w as i64 || y >= h as i64 || img.get_pixel(x as u32, y as u32).0[3] < 128;
        let mut edge = Vec::new();
        for y in y0..=y1.min(h - 1) {
            for x in 0..w {
                if img.get_pixel(x, y).0[3] < 128 || self.has(x, y) { continue; }
                let (xi, yi) = (x as i64, y as i64);
                if clear(xi - 1, yi) || clear(xi + 1, yi) || clear(xi, yi - 1) || clear(xi, yi + 1) { edge.push((x as u16, y as u16)); }
            }
        }
        edge
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame with a body (a block) and a weapon (a line off it), and its glint.
    fn frame() -> (Arc<RgbaImage>, Glint) {
        let mut img = RgbaImage::new(32, 32);
        for y in 8..28 { for x in 10..18 { img.put_pixel(x, y, image::Rgba([90, 80, 70, 255])); } }
        let weapon: Vec<(u16, u16)> = (2..8).map(|y| (20, y)).collect();
        for &(x, y) in &weapon { img.put_pixel(x as u32, y as u32, image::Rgba([200, 200, 210, 255])); }
        (Arc::new(img), Glint { weapon, off: Vec::new() })
    }

    #[test]
    fn the_look_lies_on_its_piece_faintly_and_leaves_the_body_alone() {
        let (img, glint) = frame();
        let mut sheen = Sheen::default();
        let none = sheen.dress(img.clone(), Some(&glint), &[None; 6], 1);
        assert!(Arc::ptr_eq(&none, &img), "nothing worn, nothing laid");
        let mut worn = [None; 6];
        worn[0] = Some(Capstone::Ember);
        let out = sheen.dress(img.clone(), Some(&glint), &worn, 2);
        // Beside the blade, a faint glow of ember; the body's pixels are as they were.
        let beside = out.get_pixel(21, 4).0;
        assert!(beside[3] > 20 && beside[3] < 140 && beside[0] > beside[2], "{beside:?}");
        assert!((8..28).all(|y| (10..18).all(|x| out.get_pixel(x, y) == img.get_pixel(x, y))), "the body is left alone");
        // Far from the blade, nothing.
        assert_eq!(out.get_pixel(2, 30).0[3], 0);
        // Motes come off it in time, and no more than the most at once.
        for t in 3..400 { sheen.dress(img.clone(), Some(&glint), &worn, t); }
        assert!(!sheen.motes.is_empty() && sheen.motes.len() <= MOST_MOTES);
        // Armour's look rims the figure's edge in its band, not the blade.
        let mut worn = [None; 6];
        worn[5] = Some(Capstone::Fleet);
        let out = sheen.dress(img.clone(), Some(&glint), &worn, 500);
        assert!(out.get_pixel(9, 25).0[3] > 0 && out.get_pixel(9, 10).0[3] == 0 && out.get_pixel(21, 4).0[3] == 0);
    }
}
