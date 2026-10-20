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

    /// The first `count` frames of a sprite sheet cut in `cell`-pixel squares, row by row, marked
    /// as pixel art: how a Pixellab effect exports (`<name>.png` beside `<name>.json`).
    pub fn from_grid(path: &Path, cell: u32, count: usize) -> Option<Clip> {
        let sheet = image::open(path).ok()?.to_rgba8();
        let cols = (sheet.width() / cell).max(1);
        let frames: Vec<Arc<RgbaImage>> = (0..count as u32)
            .filter(|i| (i / cols + 1) * cell <= sheet.height())
            .map(|i| Arc::new(image::imageops::crop_imm(&sheet, (i % cols) * cell, (i / cols) * cell, cell, cell).to_image()))
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
        Some(Sheet { clips, ground: bottom as f32 + 1.0, figure: (bottom + 1 - top) as f32, canvas })
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
        let clip = Clip::from_grid(&path, 4, 3).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(clip.frames.len(), 3);
        assert_eq!((clip.frames[1].get_pixel(0, 0)[1], clip.frames[2].get_pixel(3, 3)[2]), (255, 255), "second is green, third blue");
        assert!(quartz::is_pixel_art(&clip.frames[0]) && quartz::is_pixel_art(&clip.mirrored[2]));
    }

    /// Every clip of every character, a row each, numbered by frame:
    /// `cargo test --release sheets -- --ignored` writes target/shots/sheet_<character>.png.
    #[test]
    #[ignore]
    fn sheets() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for name in ["wayfarer", "wayfarer_ne", "husk"] {
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
