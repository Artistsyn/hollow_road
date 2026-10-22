//! The world map (M on the road): the places found so far where the atlas lays them, the roads
//! between them, which are locked and by what, the shortcuts (shut, and opened), the shrines rested
//! at, the bosses still standing, and where the hero is. A road out of a found place into one not
//! yet found shows as a stub, so the map says where there is more to walk. Drawn as one picture;
//! the game puts the found places' names over it (`label_at`).

use crate::atlas::{self, Gate, Place, PLACES};
use crate::world::World;
use image::{Rgba, RgbaImage};

/// The picture's size, pixels.
pub const W: u32 = 1000;
pub const H: u32 = 600;

/// Where a place's map spot (0..100 across, 0..60 down) falls on the picture.
pub fn at(map: (f32, f32)) -> (f32, f32) { (40.0 + map.0 * 9.2, 30.0 + map.1 * 9.0) }

/// Each region's colour, by its index in `atlas::REGIONS`.
const REGION: [[u8; 3]; 8] = [
    [150, 120, 96], [96, 140, 112], [92, 120, 76], [150, 146, 136], [70, 140, 150], [150, 100, 170], [120, 110, 170], [230, 220, 170],
];
const ROAD: [u8; 3] = [176, 164, 140];
const LOCKED: [u8; 3] = [196, 70, 56];
const SHORTCUT: [u8; 3] = [120, 200, 168];
const GOLD: [u8; 3] = [236, 200, 110];

fn blend(img: &mut RgbaImage, x: i32, y: i32, c: [u8; 3], a: f32) {
    if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 || a <= 0.0 { return; }
    let p = img.get_pixel_mut(x as u32, y as u32);
    for k in 0..3 { p.0[k] = (p.0[k] as f32 + (c[k] as f32 - p.0[k] as f32) * a.min(1.0)).round() as u8; }
}

/// A filled disc, its edge softened over a pixel.
fn disc(img: &mut RgbaImage, (cx, cy): (f32, f32), r: f32, c: [u8; 3], a: f32) {
    let (x0, x1, y0, y1) = ((cx - r - 1.0) as i32, (cx + r + 1.0) as i32, (cy - r - 1.0) as i32, (cy + r + 1.0) as i32);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            blend(img, x, y, c, a * (r + 0.5 - d).clamp(0.0, 1.0));
        }
    }
}

/// A ring of width `w`.
fn ring(img: &mut RgbaImage, (cx, cy): (f32, f32), r: f32, w: f32, c: [u8; 3]) {
    let o = r + w;
    for y in (cy - o - 1.0) as i32..=(cy + o + 1.0) as i32 {
        for x in (cx - o - 1.0) as i32..=(cx + o + 1.0) as i32 {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            blend(img, x, y, c, (0.5 * w + 0.5 - (d - r - 0.5 * w).abs()).clamp(0.0, 1.0));
        }
    }
}

/// A line from `a` to `b`, `w` wide; dashed when `dash` (pixels on, then off).
fn line(img: &mut RgbaImage, a: (f32, f32), b: (f32, f32), w: f32, c: [u8; 3], alpha: f32, dash: Option<f32>) {
    let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt().max(1e-3);
    let steps = (len * 2.0) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps.max(1) as f32;
        if dash.is_some_and(|d| (t * len / d) as i32 % 2 == 1) { continue; }
        let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        disc(img, p, 0.5 * w, c, alpha * 0.5);
    }
}

/// A small square, centred.
fn square(img: &mut RgbaImage, (cx, cy): (f32, f32), r: f32, c: [u8; 3]) {
    for y in (cy - r) as i32..=(cy + r) as i32 { for x in (cx - r) as i32..=(cx + r) as i32 { blend(img, x, y, c, 1.0); } }
}

fn spot(id: &str) -> Option<(f32, f32)> { atlas::place_of(id).map(|p| at(p.map)) }

/// The map as the hero knows it now.
pub fn draw(w: &World) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(W, H, Rgba([20, 18, 17, 255]));
    let seen = |id: &str| w.seen.contains(&id);
    // Each found place's ground, washed in its region's colour.
    for p in PLACES.iter().filter(|p| seen(p.id)) { disc(&mut img, at(p.map), 46.0, REGION[p.region], 0.10); }
    // The roads: whole between two found places, a stub toward one not yet found.
    for e in atlas::EDGES {
        let (Some(a), Some(b)) = (spot(e.a), spot(e.b)) else { continue };
        let (sa, sb) = (seen(e.a), seen(e.b));
        if !sa && !sb { continue; }
        let (from, to) = if sa { (a, b) } else { (b, a) };
        let to = if sa && sb { to } else { (from.0 + (to.0 - from.0) * 0.4, from.1 + (to.1 - from.1) * 0.4) };
        match e.gate {
            Gate::Open => line(&mut img, from, to, 3.0, ROAD, if sa && sb { 1.0 } else { 0.5 }, None),
            Gate::Keys(need) => {
                let open = need.iter().all(|k| w.keys.contains(k));
                line(&mut img, from, to, 3.0, if open { ROAD } else { LOCKED }, 1.0, None);
                if !open { let m = ((from.0 + to.0) * 0.5, (from.1 + to.1) * 0.5); square(&mut img, m, 5.0, LOCKED); square(&mut img, m, 2.0, [20, 18, 17]); }
            }
            Gate::Shortcut => {
                let opened = w.opened.contains(&e.a);
                // Known only once one end is found; shut, it is a dashed line with a bar at the end
                // it will not open from.
                line(&mut img, from, to, 3.0, SHORTCUT, if opened { 1.0 } else { 0.6 }, (!opened).then_some(7.0));
                if !opened && sb {
                    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
                    let l = (dx * dx + dy * dy).sqrt().max(1e-3);
                    let m = (b.0 + dx / l * 18.0, b.1 + dy / l * 18.0);
                    line(&mut img, (m.0 - dy / l * 7.0, m.1 + dx / l * 7.0), (m.0 + dy / l * 7.0, m.1 - dx / l * 7.0), 3.0, SHORTCUT, 1.0, None);
                }
            }
        }
    }
    // The places: a dot in the region's colour; a gold ring for a shrine rested at (a dim one for
    // a shrine not yet), a red mark for a boss still standing.
    let rested: Vec<&str> = w.rested.iter().map(|s| atlas::node(s)).collect();
    for p in PLACES.iter().filter(|p| seen(p.id)) {
        let c = at(p.map);
        disc(&mut img, c, 7.0, REGION[p.region], 1.0);
        if p.shrine.is_some() {
            if rested.contains(&p.id) { ring(&mut img, c, 10.0, 2.5, GOLD); } else { ring(&mut img, c, 10.0, 1.5, [110, 96, 70]); }
        }
        if let Some((_, b)) = p.boss {
            let down = w.beaten.contains(&p.id);
            square(&mut img, (c.0 + 11.0, c.1 - 11.0), if b.mini { 3.0 } else { 4.5 }, if down { [80, 74, 70] } else { LOCKED });
        }
    }
    // Where the hero is (a side path is marked at its own spot).
    if let Some(p) = atlas::place_of(atlas::node(&w.at)) {
        ring(&mut img, at(p.map), 15.0, 2.0, [250, 246, 236]);
    }
    img
}

/// The found places, and where each one's name goes on the picture (centred under its dot).
pub fn label_at(w: &World) -> Vec<(&'static Place, (f32, f32))> {
    PLACES.iter().filter(|p| w.seen.contains(&p.id)).map(|p| { let (x, y) = at(p.map); (p, (x, y + 22.0)) }).collect()
}

/// The map's legend, and what the hero holds.
pub fn legend(w: &World) -> String {
    let p = w.here();
    let keys = if w.keys.is_empty() { "none".to_string() } else { w.keys.iter().map(|k| k.name()).collect::<Vec<_>>().join(", ") };
    format!(
        "{}  ·  {}\nKey items: {keys}\nShortcuts opened: {}   Places found: {} of {}\n\nGold ring: a shrine rested at.   Red square: a boss still standing.\nRed road: locked.   Green dashes: a shortcut, shut from the barred side.",
        p.title, atlas::REGIONS[p.region].name, w.opened.len(), w.seen.len(), PLACES.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_place_lies_on_the_picture() {
        for p in PLACES { let (x, y) = at(p.map); assert!(x > 20.0 && x < W as f32 - 20.0 && y > 20.0 && y < H as f32 - 20.0, "{}", p.id); }
    }

    #[test]
    fn the_map_shows_only_what_was_found() {
        let mut w = World::new(0.0);
        let blank = draw(&w);
        assert_eq!(label_at(&w).len(), 1);
        w.seen.extend(["vale_edge", "vale", "warrens_lane", "warrens"]);
        w.rested.push(crate::world::START.into());
        assert_eq!(label_at(&w).len(), 5);
        assert_ne!(draw(&w), blank);
        // An unfound place's spot stays dark.
        let (x, y) = at(atlas::place_of("hollowmere").unwrap().map);
        assert_eq!(draw(&w).get_pixel(x as u32, y as u32).0, [20, 18, 17, 255]);
    }

    /// `cargo test --release map_picture -- --ignored` writes target/shots/map.png: the whole world
    /// found, some of it won.
    #[test]
    #[ignore]
    fn map_picture() {
        let mut w = World::new(0.0);
        w.seen = PLACES.iter().map(|p| p.id).collect();
        w.keys = vec![atlas::Key::Hook, atlas::Key::Lantern];
        w.opened = vec!["drowned_mill", "sewers"];
        w.beaten = vec!["gate", "guild_hall", "chief_hold"];
        w.rested = vec![crate::world::START.into(), "ridge_camp@pine_road".into(), "stone_market@lower_stairs".into()];
        w.at = "stone_upper@cliff_path".into();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/shots");
        std::fs::create_dir_all(&dir).unwrap();
        draw(&w).save(dir.join("map.png")).unwrap();
    }
}
