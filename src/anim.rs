//! Sprite animations made with Pixellab, loaded from `assets/pixellab/<character>/<animation>/`
//! (frames `000.png`, `001.png`, ...). Pixellab centres every frame on the character's pivot:
//! custom animations on a larger canvas (room to move), template ones on the character's own.
//! Frames are padded, centred, to the largest canvas, so every animation of a character shares
//! one ground row and one height: both read from the first frame of a standing clip, which keeps
//! the feet planted through a hop or a roll instead of snapping to each frame's lowest pixel.

use image::RgbaImage;
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
}

impl Sheet {
    /// Every animation folder under `dir`; None when there is none (the stand-in art is used).
    pub fn load(dir: &Path) -> Option<Sheet> {
        let mut clips: HashMap<String, Clip> = HashMap::new();
        let mut names: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().filter(|e| e.path().is_dir()).collect();
        names.sort_by_key(|e| e.file_name());
        for e in names {
            let mut files: Vec<_> = std::fs::read_dir(e.path()).ok()?.flatten().map(|f| f.path()).filter(|p| p.extension().is_some_and(|x| x == "png")).collect();
            files.sort();
            let frames: Vec<Arc<RgbaImage>> = files.iter().filter_map(|p| image::open(p).ok()).map(|i| Arc::new(i.to_rgba8())).collect();
            if frames.is_empty() { continue; }
            clips.insert(e.file_name().to_string_lossy().into_owned(), Clip { frames, mirrored: Vec::new() });
        }
        // One canvas for all: pad each frame, centred, to the largest.
        let canvas = clips.values().flat_map(|c| c.frames.iter()).fold((0, 0), |a, f| (a.0.max(f.width()), a.1.max(f.height())));
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
        let reference = ["idle", "walk", "shamble", "light"].iter().find_map(|n| clips.get(*n)).or_else(|| clips.values().next())?.frames[0].clone();
        let (top, bottom) = opaque_rows(&reference)?;
        let roll_keys = side_roll(&mut clips, &reference, bottom).or_else(|| spin_roll(&mut clips)).unwrap_or((0, 0));
        Some(Sheet { clips, ground: bottom as f32 + 1.0, figure: (bottom + 1 - top) as f32, canvas, roll_keys })
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
fn side_roll(clips: &mut HashMap<String, Clip>, stand: &Arc<RgbaImage>, ground: u32) -> Option<(usize, usize)> {
    let (side, dip, flip) = (clips.get("e_idle")?.frames[0].clone(), clips.get("e_roll")?, clips.get("e_flip")?);
    if dip.frames.len() < 13 || flip.frames.len() < 6 { return None; }
    let grounded = |f: &RgbaImage| -> Arc<RgbaImage> {
        let drop = opaque_rows(f).map_or(0, |(_, b)| ground as i64 - b as i64);
        let mut out = RgbaImage::new(f.width(), f.height());
        image::imageops::overlay(&mut out, f, 0, drop);
        Arc::new(out)
    };
    let tuck = flip.frames[2].as_ref();
    let (x0, y0, x1, y1) = opaque_box(tuck)?;
    let pivot = ((x0 + x1) as f32 * 0.5, (y0 + y1) as f32 * 0.5);
    let mut frames = vec![stand.clone(), side.clone()];
    frames.extend(dip.frames[2..5].iter().map(|f| grounded(f)));
    let start = frames.len();
    frames.push(grounded(&flip.frames[1]));
    frames.extend((0..6).map(|k| grounded(&rotated(tuck, 60.0 * k as f32, pivot))));
    frames.push(grounded(&flip.frames[4]));
    let end = frames.len();
    frames.extend(dip.frames[11..13].iter().map(|f| grounded(f)));
    frames.extend([side, stand.clone()]);
    let mirrored: Vec<_> = frames.iter().map(|f| Arc::new(image::imageops::flip_horizontal(f.as_ref()))).collect();
    for f in frames.iter().chain(&mirrored) { quartz::set_pixel_art(f, true); }
    clips.insert("roll_side".into(), Clip { frames, mirrored });
    Some((start, end))
}

/// `img` turned `degrees` clockwise on screen about `pivot`, nearest pixel (it stays pixel art).
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
