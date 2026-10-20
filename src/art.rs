//! Stand-in art until the Pixellab sprites: the hero seen from behind in three poses and a
//! training dummy, painted as pixel art. Each picture stands on its bottom row's middle, and
//! `METRES` says how tall it stands in the world.

use image::{Rgba, RgbaImage};

/// How tall each picture stands, metres (its bottom row is the ground).
pub const HERO_METRES: f32 = 1.85;
pub const DUMMY_METRES: f32 = 1.75;
pub const HUSK_METRES: f32 = 1.8;

const OUTLINE: [u8; 4] = [14, 10, 12, 255];

struct Canvas { img: RgbaImage }

impl Canvas {
    fn new(w: u32, h: u32) -> Canvas { Canvas { img: RgbaImage::new(w, h) } }

    fn put(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if x >= 0 && y >= 0 && (x as u32) < self.img.width() && (y as u32) < self.img.height() {
            self.img.put_pixel(x as u32, y as u32, Rgba(c));
        }
    }

    fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: [u8; 4]) {
        for y in y0..=y1 { for x in x0..=x1 { self.put(x, y, c); } }
    }

    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: [u8; 4]) {
        for y in (cy - ry).floor() as i32..=(cy + ry).ceil() as i32 {
            for x in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                if dx * dx + dy * dy <= 1.0 { self.put(x, y, c); }
            }
        }
    }

    /// A trapezoid from a top edge to a bottom edge, both centred on `cx`.
    fn taper(&mut self, cx: f32, y0: i32, y1: i32, w0: f32, w1: f32, c: [u8; 4]) {
        for y in y0..=y1 {
            let t = (y - y0) as f32 / (y1 - y0).max(1) as f32;
            let hw = 0.5 * (w0 + (w1 - w0) * t);
            for x in (cx - hw).round() as i32..=(cx + hw).round() as i32 { self.put(x, y, c); }
        }
    }

    /// A line `width` pixels thick.
    fn line(&mut self, (x0, y0): (f32, f32), (x1, y1): (f32, f32), width: f32, c: [u8; 4]) {
        let n = ((x1 - x0).abs().max((y1 - y0).abs()) * 2.0).ceil() as i32 + 1;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.ellipse(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, width * 0.5, width * 0.5, c);
        }
    }

    /// A dark outline round everything painted, so the figure reads against any road.
    fn finish(mut self) -> RgbaImage {
        let src = self.img.clone();
        let (w, h) = (src.width() as i32, src.height() as i32);
        for y in 0..h { for x in 0..w {
            if src.get_pixel(x as u32, y as u32)[3] != 0 { continue; }
            let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                nx >= 0 && ny >= 0 && nx < w && ny < h && src.get_pixel(nx as u32, ny as u32)[3] != 0
            });
            if near { self.put(x, y, OUTLINE); }
        }}
        self.img
    }
}

fn shade(c: [u8; 4], k: f32) -> [u8; 4] { [(c[0] as f32 * k).min(255.0) as u8, (c[1] as f32 * k).min(255.0) as u8, (c[2] as f32 * k).min(255.0) as u8, c[3]] }

/// How the hero holds the sword.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pose { Ready, Raised, Struck, Guard }

/// The hero from behind: a hooded cloak over a pilgrim's tunic, the sword in the right hand.
/// 64 x 96 pixels.
pub fn hero(pose: Pose, flash: bool) -> RgbaImage {
    let mut c = Canvas::new(64, 96);
    let (cloak, cloak_dark, boots, skin) = ([92, 46, 40, 255], [58, 28, 28, 255], [40, 32, 30, 255], [176, 132, 104, 255]);
    let (blade, hilt) = ([196, 202, 210, 255], [120, 86, 44, 255]);
    let cx = 30.0;
    // Legs and boots.
    c.rect(24, 74, 28, 92, boots);
    c.rect(33, 74, 37, 92, boots);
    c.rect(23, 92, 29, 95, shade(boots, 0.7));
    c.rect(32, 92, 38, 95, shade(boots, 0.7));
    // The sword behind the body when raised over the shoulder.
    if pose == Pose::Raised {
        c.line((44.0, 30.0), (58.0, 4.0), 3.0, blade);
        c.line((42.0, 34.0), (45.0, 29.0), 3.0, hilt);
    }
    // The cloak: shoulders to just below the knee, darker at its folds.
    c.taper(cx, 22, 80, 26.0, 34.0, cloak);
    for k in [-8.0f32, 0.0, 8.0] { c.line((cx + k * 0.6, 30.0), (cx + k, 80.0), 1.5, cloak_dark); }
    // The hood.
    c.ellipse(cx, 16.0, 9.0, 10.0, cloak);
    c.ellipse(cx, 18.0, 6.0, 6.0, cloak_dark);
    // The sword arm.
    match pose {
        Pose::Ready => {
            c.line((43.0, 28.0), (47.0, 52.0), 4.0, cloak);
            c.ellipse(47.5, 54.0, 2.5, 2.5, skin);
            c.line((48.0, 56.0), (52.0, 84.0), 3.0, blade);
            c.line((47.0, 53.0), (49.0, 58.0), 3.0, hilt);
        }
        Pose::Raised => {
            c.line((43.0, 28.0), (45.0, 34.0), 4.0, cloak);
            c.ellipse(44.0, 32.0, 2.5, 2.5, skin);
        }
        Pose::Guard => {
            // The buckler raised before them, the sword held back.
            c.line((43.0, 28.0), (46.0, 50.0), 4.0, cloak);
            c.line((47.0, 52.0), (50.0, 74.0), 3.0, blade);
        }
        Pose::Struck => {
            // Swept across to the left, the blade out in front of them.
            c.line((40.0, 28.0), (24.0, 40.0), 4.0, cloak);
            c.ellipse(22.0, 41.0, 2.5, 2.5, skin);
            c.line((20.0, 42.0), (2.0, 50.0), 3.0, blade);
            c.line((23.0, 40.0), (19.0, 43.0), 3.0, hilt);
        }
    }
    // The off arm, and its buckler raised in a guard.
    if pose == Pose::Guard {
        c.line((17.0, 28.0), (24.0, 36.0), 4.0, cloak);
        c.ellipse(26.0, 36.0, 9.0, 9.0, [96, 98, 104, 255]);
        c.ellipse(26.0, 36.0, 3.0, 3.0, [150, 152, 160, 255]);
    } else {
        c.line((17.0, 28.0), (14.0, 52.0), 4.0, cloak);
    }
    let mut img = c.finish();
    if flash { brighten(&mut img, 1.8); }
    img
}

/// How a husk stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HuskPose { Idle, Raised, Chop, Wound, Sweep, Reeling }

/// A husk facing the hero: grey, hunched, a cleaver in its right hand (screen left). 56 x 96.
pub fn husk(pose: HuskPose, flash: bool) -> RgbaImage {
    let mut c = Canvas::new(56, 96);
    let (skin, skin_dark, rags, iron, glow) = ([132, 128, 124, 255], [92, 88, 90, 255], [70, 58, 52, 255], [118, 104, 96, 255], [168, 96, 220, 255]);
    let cx = 28.0;
    let bow = if pose == HuskPose::Reeling { 8.0 } else { 0.0 };
    // Legs.
    c.rect(20, 70, 24, 95, skin_dark);
    c.rect(32, 70, 36, 95, skin_dark);
    // Rags from the shoulders to the knee.
    c.taper(cx, 30 + bow as i32, 76, 22.0, 28.0, rags);
    c.line((cx - 6.0, 40.0 + bow), (cx - 8.0, 76.0), 1.5, shade(rags, 0.7));
    // The head, hung forward, with the Taint glowing through its cracks.
    c.ellipse(cx + 1.0, 22.0 + bow, 8.0, 9.0, skin);
    c.ellipse(cx - 2.0, 22.0 + bow, 1.5, 1.5, glow);
    c.ellipse(cx + 4.0, 22.0 + bow, 1.5, 1.5, glow);
    c.line((cx + 1.0, 14.0 + bow), (cx + 3.0, 20.0 + bow), 1.0, glow);
    // The cleaver arm, by pose.
    let (hand, tip) = match pose {
        HuskPose::Idle | HuskPose::Reeling => ((12.0, 60.0), (10.0, 76.0)),
        HuskPose::Raised => ((14.0, 12.0), (20.0, 0.0)),
        HuskPose::Chop => ((16.0, 58.0), (22.0, 76.0)),
        HuskPose::Wound => ((6.0, 34.0), (0.0, 30.0)),
        HuskPose::Sweep => ((50.0, 50.0), (55.0, 46.0)),
    };
    c.line((18.0, 34.0 + bow), hand, 4.0, skin);
    c.line(hand, tip, 6.0, iron);
    c.ellipse(hand.0, hand.1, 2.5, 2.5, skin_dark);
    // The other arm hangs.
    c.line((38.0, 34.0 + bow), (43.0, 62.0), 4.0, skin);
    let mut img = c.finish();
    if flash { brighten(&mut img, 1.9); }
    img
}

/// A training dummy: a post, a crossbar for arms, a sack body and head. 48 x 96 pixels.
pub fn dummy(hurt: bool) -> RgbaImage {
    let mut c = Canvas::new(48, 96);
    let (wood, sack, sack_dark, rope) = ([104, 72, 44, 255], [176, 150, 98, 255], [132, 108, 66, 255], [80, 60, 36, 255]);
    c.rect(22, 30, 25, 95, wood);
    c.rect(19, 92, 28, 95, shade(wood, 0.7));
    c.rect(4, 36, 43, 39, wood);
    c.ellipse(24.0, 54.0, 12.0, 17.0, sack);
    c.ellipse(27.0, 58.0, 6.0, 10.0, sack_dark);
    c.rect(13, 44, 35, 45, rope);
    c.rect(13, 64, 35, 65, rope);
    c.ellipse(24.0, 20.0, 8.0, 9.0, sack);
    c.ellipse(26.0, 22.0, 4.0, 4.0, sack_dark);
    c.rect(18, 28, 30, 29, rope);
    let mut img = c.finish();
    if hurt { brighten(&mut img, 1.9); }
    img
}

fn brighten(img: &mut RgbaImage, k: f32) {
    for p in img.pixels_mut() { if p[3] > 0 { let c = shade(p.0, k); *p = Rgba([c[0].max(60), c[1].max(60), c[2].max(60), c[3]]); } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_picture_stands_on_its_bottom_row() {
        let husks = [HuskPose::Idle, HuskPose::Raised, HuskPose::Chop, HuskPose::Wound, HuskPose::Sweep, HuskPose::Reeling].map(|p| husk(p, false));
        for img in [hero(Pose::Ready, false), hero(Pose::Raised, false), hero(Pose::Struck, true), hero(Pose::Guard, false), dummy(false), dummy(true)].into_iter().chain(husks) {
            let (w, h) = img.dimensions();
            assert!((0..w).any(|x| img.get_pixel(x, h - 1)[3] > 0), "it should touch the ground");
        }
        assert_ne!(hero(Pose::Ready, false), hero(Pose::Struck, false), "the poses differ");
    }
}
