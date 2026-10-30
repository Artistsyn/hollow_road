//! What shows while the game gets ready: the hero's and every foe's art read on a thread of its
//! own (seconds on a phone, which would otherwise be seconds of black before the first frame), and
//! over the black meanwhile the wayfarer walking toward a far lantern with embers rising past.

use crate::anim::Sheet;
use crate::fight::Archetype;
use image::RgbaImage;
use quartz::{Align, Canvas, Color, Font, GameObject, Image, ShapeType, Span, Text};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Every figure's art the game holds from the start (`Pilgrimage::new`).
#[derive(Default)]
pub struct Sheets {
    pub hero: Option<Sheet>,
    pub enemies: Vec<(Archetype, Sheet)>,
    pub bell_warden: Option<Sheet>,
    pub keeper: Option<Sheet>,
    pub smith: Option<Sheet>,
    pub gruk: Option<Sheet>,
}

impl Sheets {
    /// Read them all now: the hero, and the foes in a few shares, side by side on threads of
    /// their own (the hero's armed sheet is the longest, every weapon drawn into its clips).
    pub fn read() -> Sheets {
        let kinds = Archetype::ALL;
        let share = kinds.len().div_ceil(3).max(1);
        let read_kinds = |part: &[Archetype]| -> Vec<(Archetype, Sheet)> { part.iter().filter_map(|&k| Some((k, crate::game::load_sheet(crate::game::kind_art(k).0)?))).collect() };
        let (hero, enemies, others) = std::thread::scope(|scope| {
            let hero = scope.spawn(|| crate::game::armed_sheet("wayfarer_ne").or_else(|| crate::game::load_sheet("wayfarer")));
            let foes: Vec<_> = kinds.chunks(share).map(|part| scope.spawn(move || read_kinds(part))).collect();
            let others = ["bell_warden", "keeper", "smith", "gruk"].map(crate::game::load_sheet);
            let enemies: Vec<(Archetype, Sheet)> = foes.into_iter().flat_map(|part| part.join().unwrap_or_default()).collect();
            (hero.join().ok().flatten(), enemies, others)
        });
        let [bell_warden, keeper, smith, gruk] = others;
        Sheets { hero, enemies, bell_warden, keeper, smith, gruk }
    }

    /// Read them on a thread of their own; read here and now if no thread can be had.
    pub fn reading() -> Result<JoinHandle<Sheets>, Sheets> {
        std::thread::Builder::new().name("art".into()).spawn(Sheets::read).map_err(|_| Sheets::read())
    }
}

/// The art of the boss of the place the hero is in, by its folder (one at a time: each is about
/// 20 MB of frames), read on a thread of its own on arriving there, so the walk does not stall
/// for it.
#[derive(Default)]
pub struct BossArt {
    held: Option<(&'static str, Option<Sheet>)>,
    due: Option<(&'static str, JoinHandle<Option<Sheet>>)>,
}

impl BossArt {
    /// Ask for `folder`'s art: read on a thread unless it is held or being read (here and now in
    /// a test, or if no thread can be had).
    pub fn want(&mut self, folder: &'static str) {
        let asked = |f: &str| f == folder;
        if self.held.as_ref().is_some_and(|(f, _)| asked(f)) || self.due.as_ref().is_some_and(|(f, _)| asked(f)) { return; }
        let spawned = (!cfg!(test)).then(|| std::thread::Builder::new().name("boss art".into()).spawn(move || crate::game::load_sheet(folder)).ok()).flatten();
        match spawned {
            Some(reading) => self.due = Some((folder, reading)),
            None => self.take((folder, crate::game::load_sheet(folder))),
        }
    }

    /// Take the art in once it is read; with `now` (its fight needs it this tick) wait for it.
    pub fn settle(&mut self, now: bool) {
        if !now && !self.due.as_ref().is_some_and(|(_, reading)| reading.is_finished()) { return; }
        if let Some((folder, reading)) = self.due.take() { self.take((folder, reading.join().ok().flatten())); }
    }

    /// The art read for `folder`, if it is the one held.
    pub fn sheet(&self, folder: &str) -> Option<&Sheet> { self.held.as_ref().filter(|(f, _)| *f == folder).and_then(|(_, sheet)| sheet.as_ref()) }

    /// Hold `read`, letting the art held before go on a thread of its own (freeing 20 MB of
    /// frames is not nothing either; here, if no thread can be had).
    fn take(&mut self, read: (&'static str, Option<Sheet>)) {
        if let Some(old) = self.held.replace(read) {
            let _ = std::thread::Builder::new().name("boss art gone".into()).spawn(move || drop(old));
        }
    }
}

/// The objects of the loading screen, over the fade's black (300).
const GLOW: &str = "loading_glow";
const LAMP: &str = "loading_lamp";
const HERO: &str = "loading_hero";
const WORDS: &str = "loading_words";
const MOTES: usize = 26;
/// The road's rows, one a pixel of the art from the horizon to the foot of the screen, and how
/// long a band of its paving is (in the hero's steps).
const ROWS: usize = 168;
const BAND_LONG: f32 = 0.7;
/// Steps the road comes on by a tick, under the hero's walk.
const PACE: f32 = 0.012;
/// One pixel of the art on the canvas, as the road draws it.
const PIXEL: f32 = 8.0;
/// The hero's frames drawn this many canvas units a pixel of the art.
const HERO_SCALE: f32 = 3.0;
/// Ticks a walk frame is shown.
const STRIDE: u32 = 7;
/// How fast the screen comes and goes, a tick.
const EASE: f32 = 1.0 / 18.0;
/// Where the far lantern hangs and where the hero's feet are (canvas units).
const LANTERN: (f32, f32) = (1920.0, 820.0);
const FEET: (f32, f32) = (1920.0, 1720.0);

/// The loading screen: how much of it shows, its clock, and its pictures (made once each, tinted
/// as it moves, so it uploads nothing after its first frame).
pub struct Loading {
    shown: f32,
    ticks: u32,
    walk: Vec<Arc<RgbaImage>>,
    glow: Arc<RgbaImage>,
}

impl Loading {
    /// The screen, hidden, with its objects added to `canvas`.
    pub fn new(canvas: &mut Canvas) -> Loading {
        let walk = Loading::walk_frames();
        // The falloff in the colour as well as the alpha, so it is all but black where the
        // renderer stops drawing faint alpha: a hard ring there otherwise, plain over the dark.
        let glow = Arc::new(RgbaImage::from_fn(128, 128, |x, y| {
            let (u, v) = ((x as f32 + 0.5) / 64.0 - 1.0, (y as f32 + 0.5) / 64.0 - 1.0);
            let r2 = (u * u + v * v).min(1.0);
            let fall = (255.0 * (-4.0 * r2).exp() * (1.0 - r2)) as u8;
            image::Rgba([fall, fall, fall, fall])
        }));
        let mut ids: Vec<(String, i32)> = vec![(GLOW.into(), 301), (LAMP.into(), 303), (HERO.into(), 305), (WORDS.into(), 306)];
        ids.extend((0..ROWS).map(|i| (Loading::row_id(i), 302)));
        ids.extend((0..MOTES).map(|i| (Loading::mote_id(i), if i % 3 == 0 { 306 } else { 304 })));
        for (id, layer) in ids {
            let mut sprite = GameObject::build(&id).size(1.0, 1.0).layer(layer).screen_space().finish();
            (sprite.unlit, sprite.visible) = (true, false);
            canvas.add_game_object(id, sprite);
        }
        Loading { shown: 0.0, ticks: 0, walk, glow }
    }

    /// One tick: come in while `wanted` (or go), and draw. `title` is written above while it
    /// shows, in `face`.
    pub fn draw(&mut self, canvas: &mut Canvas, wanted: bool, title: Option<&str>, face: Option<&Arc<Font>>) {
        let was = self.shown;
        self.shown = if wanted { (self.shown + EASE).min(1.0) } else { (self.shown - EASE).max(0.0) };
        if was == 0.0 && self.shown == 0.0 { return; }
        self.ticks += 1;
        let opacity = self.shown * self.shown * (3.0 - 2.0 * self.shown);
        let flicker = self.flicker();
        self.place_light(canvas, opacity, flicker);
        self.place_road(canvas, opacity, flicker);
        self.place_hero(canvas, opacity, flicker);
        self.place_motes(canvas, opacity);
        if let Some(sprite) = canvas.get_game_object_mut(WORDS) {
            let shown = title.zip(face).filter(|_| opacity > 0.01);
            sprite.visible = shown.is_some();
            if let Some((words, font)) = shown {
                let scale = canvas.base_scale();
                let t = Text::new(vec![Span::new(words.to_owned(), 110.0 * scale, Some(140.0 * scale), font.clone(), Color(214, 196, 160, (200.0 * opacity) as u8), 12.0)], Some(3000.0 * scale), Align::Center, None);
                if let Some(sprite) = canvas.get_game_object_mut(WORDS) {
                    sprite.set_drawable(Box::new(t));
                    (sprite.position, sprite.size) = ((420.0, 300.0), (3000.0, 160.0));
                }
            }
        }
    }

    /// The lantern's light from tick to tick: a slow breath and a quicker waver, 0.8..1.
    fn flicker(&self) -> f32 {
        let t = self.ticks as f32 / 60.0;
        0.9 + 0.06 * (t * 1.7).sin() + 0.04 * (t * 7.3 + (t * 2.1).sin()).sin()
    }

    /// The far lantern and its glow on the dark.
    fn place_light(&self, canvas: &mut Canvas, opacity: f32, flicker: f32) {
        let warm = |alpha: f32| Color(255, 178, 96, (255.0 * alpha).clamp(0.0, 255.0) as u8);
        let glow = 1100.0 * (0.96 + 0.04 * flicker);
        let parts = [
            (GLOW, (LANTERN.0, LANTERN.1), (glow, glow), self.glow.clone(), warm(0.42 * opacity * flicker)),
            (LAMP, LANTERN, (2.0 * PIXEL, 2.0 * PIXEL), quartz::white_pixel(), Color(255, 236, 196, (255.0 * opacity) as u8)),
        ];
        for (id, at, size, image, color) in parts {
            if let Some(sprite) = canvas.get_game_object_mut(id) {
                sprite.visible = opacity > 0.0;
                (sprite.position, sprite.size) = (Loading::snapped((at.0 - 0.5 * size.0, at.1 - 0.5 * size.1)), size);
                sprite.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image, color: Some(color), recolor: None });
            }
        }
    }

    /// The road to the lantern a row of the art's pixels at a time, wide at the hero's feet and
    /// narrowing to the light: every other band of paving a shade lighter, the bands coming on as
    /// the hero walks, and all of it lit by the lantern, so brighter toward it.
    fn place_road(&self, canvas: &mut Canvas, opacity: f32, flicker: f32) {
        // Steps ahead for a screen row: the horizon just under the lantern, the foot of the screen
        // one step ahead.
        let horizon = LANTERN.1 + 2.0 * PIXEL;
        let walked = self.ticks as f32 * PACE;
        for i in 0..ROWS {
            let top = horizon + i as f32 * PIXEL;
            let ahead = (2200.0 - horizon) / (top + 0.5 * PIXEL - horizon);
            let half = Loading::on_grid(700.0 / ahead);
            let Some(sprite) = canvas.get_game_object_mut(&Loading::row_id(i)) else { continue };
            sprite.visible = opacity > 0.0 && half > 0.0 && top < 2200.0;
            if !sprite.visible { continue; }
            let band = ((ahead + walked) / BAND_LONG).floor() as i64;
            let lit = (0.05 + 0.5 / (1.0 + 0.04 * (ahead - 14.0).powi(2))) * flicker * if band % 2 == 0 { 1.0 } else { 0.8 };
            let tone = |k: f32| (255.0 * (lit * k).min(1.0)) as u8;
            let size = (2.0 * half, PIXEL);
            (sprite.position, sprite.size) = ((LANTERN.0 - half, top), size);
            sprite.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: quartz::white_pixel(), color: Some(Color(tone(0.62), tone(0.46), tone(0.32), (255.0 * opacity) as u8)), recolor: None });
        }
    }

    /// The wayfarer from behind, walking on toward the light, lit faintly from ahead.
    fn place_hero(&self, canvas: &mut Canvas, opacity: f32, flicker: f32) {
        let Some(sprite) = canvas.get_game_object_mut(HERO) else { return };
        let Some(frame) = self.walk.get((self.ticks / STRIDE) as usize % self.walk.len().max(1)) else { sprite.visible = false; return };
        let size = (frame.width() as f32 * HERO_SCALE, frame.height() as f32 * HERO_SCALE);
        // The feet stand near the frame's foot (PixelLab's canvas leaves a margin below them).
        let feet = 0.83 * size.1;
        sprite.visible = opacity > 0.0;
        (sprite.position, sprite.size) = (Loading::snapped((FEET.0 - 0.5 * size.0, FEET.1 - feet)), size);
        let lit = 0.55 + 0.1 * flicker;
        let tone = |k: f32| (255.0 * lit * k) as u8;
        sprite.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: frame.clone(), color: Some(Color(tone(1.0), tone(0.88), tone(0.8), (255.0 * opacity) as u8)), recolor: None });
    }

    /// Embers rising past from the lantern's side of the road, each on its own sway, warm as
    /// they go up and cooling to nothing.
    fn place_motes(&self, canvas: &mut Canvas, opacity: f32) {
        for i in 0..MOTES {
            let seed = Loading::hash(i as u32);
            let unit = |k: u32| (Loading::hash(seed ^ k) & 0xFFFF) as f32 / 65535.0;
            let period = 360.0 + 300.0 * unit(1);
            let life = ((self.ticks as f32 + period * unit(2)) / period).fract();
            let x = 600.0 + 2640.0 * unit(3) + 90.0 * (life * 6.0 + 6.28 * unit(4)).sin();
            let y = 2080.0 - 1500.0 * life * (0.7 + 0.3 * unit(5));
            let fade = (life * 8.0).min(1.0) * (1.0 - life).powf(1.5);
            let size = if i % 3 == 0 { 2.0 * PIXEL } else { PIXEL };
            let Some(sprite) = canvas.get_game_object_mut(&Loading::mote_id(i)) else { continue };
            sprite.visible = opacity * fade > 0.01;
            (sprite.position, sprite.size) = (Loading::snapped((x, y)), (size, size));
            let (g, b) = (150.0 + 70.0 * (1.0 - life), 60.0 * (1.0 - life));
            sprite.set_image(Image { shape: ShapeType::Rectangle(0.0, (size, size), 0.0), image: quartz::white_pixel(), color: Some(Color(255, g as u8, b as u8, (230.0 * opacity * fade) as u8)), recolor: None });
        }
    }

    /// The hero walking straight up the road from behind (its own clip: the road's are drawn
    /// turned a little toward the north-east), read from its frames; none, and the screen is
    /// drawn without the figure.
    fn walk_frames() -> Vec<Arc<RgbaImage>> {
        let dir = crate::assets::path("pixellab/loading/walk");
        let mut files: Vec<_> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "png")).collect()).unwrap_or_default();
        files.sort();
        let frames: Vec<Arc<RgbaImage>> = files.iter().filter_map(|p| image::open(p).ok()).map(|i| Arc::new(i.to_rgba8())).collect();
        for f in &frames { quartz::set_pixel_art(f, true); }
        frames
    }

    fn mote_id(i: usize) -> String { format!("loading_mote_{i}") }

    fn row_id(i: usize) -> String { format!("loading_row_{i}") }

    /// On the art's pixel grid (a road row or width).
    fn on_grid(units: f32) -> f32 { (units / PIXEL).round() * PIXEL }

    /// On the art's pixel grid, so nothing on the screen sits between pixels.
    fn snapped(at: (f32, f32)) -> (f32, f32) { ((at.0 / 2.0).round() * 2.0, (at.1 / 2.0).round() * 2.0) }

    fn hash(seed: u32) -> u32 {
        let mut mixed = seed.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
        mixed ^= mixed >> 15;
        mixed = mixed.wrapping_mul(0x2C1B_3C6D);
        mixed ^ (mixed >> 12)
    }
}
