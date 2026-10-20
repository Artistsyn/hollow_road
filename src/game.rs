//! The game on the canvas: the road drawn live by PathForge, the warnings painted on it, the hero
//! and the enemies placed on it each tick from the fight's road coordinates, the keys, the bars.
//!
//! Controls (PC and Mac, as in Dark Souls III and Elden Ring): W/S walk on and back, Shift held
//! runs, A/D step a lane across, Space dodges (rolls the way A or D points, or the way a step
//! just began; alone it hops back), left click strikes and Shift + left click strikes heavy,
//! right click held guards and Shift + right click parries, R drinks. Arrow keys walk and step too.

use crate::anim::{Clip, Sheet};
use crate::art::{self, HuskPose, Pose};
use crate::fx::{self, Fx};
use crate::fight::{self, Ai, Archetype, Enemy, Event, Fight, Input, Kind, Move, Phase, HERO_D, LANE_W, SHOULDER};
use image::RgbaImage;
use quartz::plugin::QuartzPlugin;
use quartz::*;
use quartz_path_forge::marks::{Fill, GroundMark};
use quartz_path_forge::PathForgePlugin;
use std::sync::Mutex;

/// The canvas is 3840 x 2160; the road is drawn a little larger so a camera shake never shows
/// its edge.
const BLEED: f32 = 48.0;
/// The road's own pixels: rendered small and scaled up (the scene's look is pixel art).
const ROAD_PX: (u32, u32) = (480, 270);

const ROAD: &str = "road";
const MARKS: &str = "marks";
const HERO: &str = "hero";
const HUD: &str = "hud";
/// Bars: (id, x, y, width, height, colour) along the top left; the enemy's along the top middle.
const BARS: [(&str, f32, f32, f32, f32, [u8; 4]); 6] = [
    ("bar_hp_back", 60.0, 140.0, 900.0, 34.0, [30, 20, 22, 200]),
    ("bar_hp", 60.0, 140.0, 900.0, 34.0, [176, 36, 40, 255]),
    ("bar_st_back", 60.0, 186.0, 700.0, 22.0, [20, 26, 22, 200]),
    ("bar_st", 60.0, 186.0, 700.0, 22.0, [70, 150, 80, 255]),
    ("bar_foe", 1320.0, 140.0, 1200.0, 26.0, [150, 40, 44, 255]),
    ("bar_foe_stance", 1320.0, 172.0, 1200.0, 12.0, [214, 176, 70, 255]),
];

fn enemy_id(i: usize) -> String { format!("enemy_{i}") }

/// The first stretch of road: husks to cut down, the second and third off to the sides.
fn first_enemies(from: f64) -> Vec<Enemy> {
    vec![
        Enemy::new(Archetype::Husk, from + 12.0, 0),
        Enemy::new(Archetype::Husk, from + 24.0, 1),
        Enemy::new(Archetype::Husk, from + 36.0, -1),
    ]
}

/// A full-object image of one colour (stretched to the object's size).
fn swatch(c: [u8; 4]) -> Arc<RgbaImage> { Arc::new(RgbaImage::from_pixel(1, 1, image::Rgba(c))) }

/// Set the scene up on `canvas`: the road, the hero, the enemies, the numbers, and the game.
pub fn setup(canvas: &mut Canvas) {
    canvas.set_camera(Camera::new((3840.0, 2160.0), (3840.0, 2160.0)));
    let road_rect = |id: &str, layer: i32| {
        let mut o = GameObject::build(id)
            .size(3840.0 + 2.0 * BLEED, 2160.0 + 2.0 * BLEED * 9.0 / 16.0)
            .position(-BLEED, -BLEED * 9.0 / 16.0)
            .layer(layer)
            .finish();
        // The road carries its own light.
        o.unlit = true;
        o
    };
    canvas.add_game_object(ROAD.to_owned(), road_rect(ROAD, -100));
    canvas.add_game_object(MARKS.to_owned(), road_rect(MARKS, -99));
    let scene = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/scenes/road_to_hollowmere.json");
    match PathForgePlugin::live(scene, ROAD, ROAD_PX) {
        // The road is pixel art scaled up eight times: hard pixels, like the sprites over it.
        Ok(p) => canvas.add_plugin(p.with_speed(0.0).with_pixel_art(true)),
        Err(e) => eprintln!("hollow_road: the road would not open: {e}"),
    }

    let sprite = |id: &str, layer: i32| { let mut o = GameObject::build(id).size(1.0, 1.0).position(0.0, 0.0).layer(layer).finish(); o.unlit = true; o };
    canvas.add_game_object(HERO.to_owned(), sprite(HERO, 60));
    let enemies = first_enemies(0.0);
    for i in 0..enemies.len() { canvas.add_game_object(enemy_id(i), sprite(&enemy_id(i), 10)); }
    // The slash over the hero; the hit bursts over everything on the road. A burst's object is
    // a clear picture that only carries its effect.
    canvas.add_game_object(fx::SLASH.to_owned(), sprite(fx::SLASH, 62));
    for i in 0..fx::BURSTS {
        let mut o = sprite(&fx::burst_id(i), 65);
        o.visible = false;
        o.set_image(Image { shape: ShapeType::Rectangle(0.0, (1.0, 1.0), 0.0), image: swatch([0, 0, 0, 0]), color: None });
        canvas.add_game_object(fx::burst_id(i), o);
    }
    let hud = GameObject::build(HUD).size(3600.0, 120.0).position(60.0, 40.0).layer(200).screen_space().finish();
    canvas.add_game_object(HUD.to_owned(), hud);
    for (id, x, y, w, h, c) in BARS {
        let mut o = GameObject::build(id).size(w, h).position(x, y).layer(201).screen_space().finish();
        o.unlit = true;
        o.set_image(Image { shape: ShapeType::Rectangle(0.0, (w, h), 0.0), image: swatch(c), color: None });
        canvas.add_game_object(id.to_owned(), o);
    }

    let presses = Arc::new(Mutex::new(Vec::new()));
    let q = presses.clone();
    canvas.on_key_press(move |_c, k| q.lock().unwrap().push(Press::Key(k.clone())));
    let q = presses.clone();
    canvas.on_mouse_press(move |c, b, _| { let shift = shift_held(c); q.lock().unwrap().push(Press::Mouse(b, shift)); });
    canvas.add_plugin(Pilgrimage::new(Fight::new(enemies), presses));
}

/// A key or a click, as it came (a click with whether Shift was down).
#[derive(Clone, Debug)]
enum Press { Key(Key), Mouse(MouseButton, bool) }

fn shift_held(c: &Canvas) -> bool { c.is_key_held(&Key::Named(NamedKey::Shift)) }

/// The pictures, made once.
struct Art {
    hero: [Arc<RgbaImage>; 4],
    husk: [Arc<RgbaImage>; 6],
    husk_hurt: [Arc<RgbaImage>; 6],
}

impl Art {
    fn new() -> Art {
        let husks = [HuskPose::Idle, HuskPose::Raised, HuskPose::Chop, HuskPose::Wound, HuskPose::Sweep, HuskPose::Reeling];
        Art {
            hero: [Pose::Ready, Pose::Raised, Pose::Struck, Pose::Guard].map(|p| Arc::new(art::hero(p, false))),
            husk: husks.map(|p| Arc::new(art::husk(p, false))),
            husk_hurt: husks.map(|p| Arc::new(art::husk(p, true))),
        }
    }
}

/// The game's own plugin: reads the keys, runs the fight, drives the road and places the sprites.
pub struct Pilgrimage {
    pub(crate) fight: Fight,
    /// Every event, with the fight tick it came on (for the headless capture).
    #[cfg(test)]
    pub(crate) log: Vec<(u64, Event)>,
    presses: Arc<Mutex<Vec<Press>>>,
    /// Presses not yet given to a fight tick (game time can stand still while keys arrive).
    pending: Input,
    /// Fight ticks owed: game time over the fight's 60 Hz tick.
    owed: f32,
    /// Real ticks left of a slow motion, and its scale.
    slow: (u32, f32),
    /// Real ticks since the hero died (to start again).
    dead_for: u32,
    art: Art,
    /// The Pixellab animations, when they are there (the stand-in art otherwise).
    hero_sheet: Option<Sheet>,
    husk_sheet: Option<Sheet>,
    font: Option<Arc<Font>>,
    ticks: u64,
    /// Road frames shown in the last second, and the frame last seen (to count new ones).
    frames: (u32, u32),
    last_frame: Option<Arc<RgbaImage>>,
    /// The road's speed last tick (the hero strides while it moves).
    last_speed: f32,
    /// The warnings painted last, to repaint only on change.
    last_marks: Vec<GroundMark>,
    last_marks_frame: Option<Arc<RgbaImage>>,
    fx: Fx,
}

fn load_sheet(name: &str) -> Option<Sheet> {
    Sheet::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/pixellab").join(name))
}

/// The frame of `clip` for tick `t` of a move with `frames` (wind-up, live, recovery): the
/// wind-up plays frames up to `keys.0`, the live ticks up to `keys.1`, the recovery the rest, so
/// the blow lands on the drawing of it whatever the clip's own timing.
fn phased(clip: &Clip, frames: (u32, u32, u32), t: u32, keys: (usize, usize), mirror: bool) -> Arc<RgbaImage> {
    let n = clip.frames.len();
    let (a, b) = (keys.0.min(n), keys.1.clamp(keys.0.min(n), n));
    let (w, act, rec) = frames;
    let span = |from: usize, to: usize, k: f32| from as f32 + (to - from) as f32 * k.clamp(0.0, 0.999);
    let f = if t < w { span(0, a, t as f32 / w.max(1) as f32) }
        else if t < w + act { span(a, b, (t - w) as f32 / act.max(1) as f32) }
        else { span(b, n, (t - w - act) as f32 / rec.max(1) as f32) };
    clip.at((f + 0.5) / n as f32, mirror).clone()
}

fn is(k: &Key, chars: &[&str], named: Option<NamedKey>) -> bool {
    match k { Key::Character(c) => chars.iter().any(|ch| c.eq_ignore_ascii_case(ch)), Key::Named(n) => Some(*n) == named }
}

impl Pilgrimage {
    fn new(fight: Fight, presses: Arc<Mutex<Vec<Press>>>) -> Pilgrimage {
        let font = Font::from_bytes(include_bytes!("../assets/font.ttf")).ok().map(Arc::new);
        Pilgrimage {
            #[cfg(test)]
            log: Vec::new(),
            fight, presses, pending: Input::default(), owed: 0.0, slow: (0, 1.0), dead_for: 0,
            art: Art::new(), hero_sheet: load_sheet("wayfarer_ne").or_else(|| load_sheet("wayfarer")), husk_sheet: load_sheet("husk"), font, ticks: 0, frames: (0, 0), last_frame: None, last_speed: 0.0, last_marks: Vec::new(), last_marks_frame: None, fx: Fx::load(),
        }
    }

    /// This tick's presses joined to any still waiting; held keys and buttons as they are now.
    fn input(&mut self, c: &Canvas, presses: &[Press]) -> Input {
        let held = |ch: &str, n: NamedKey| c.is_key_held(&Key::Character(ch.into())) || c.is_key_held(&Key::Named(n));
        let (left, right) = (held("a", NamedKey::ArrowLeft), held("d", NamedKey::ArrowRight));
        let p = &mut self.pending;
        for press in presses {
            match press {
                Press::Key(k) => {
                    if is(k, &["a"], Some(NamedKey::ArrowLeft)) { p.step = -1; }
                    if is(k, &["d"], Some(NamedKey::ArrowRight)) { p.step = 1; }
                    // The roll goes the way a direction is held; with none, a hop back.
                    if is(k, &[" "], Some(NamedKey::Space)) { p.dodge = Some(if left && !right { -1 } else if right && !left { 1 } else { 0 }); }
                    if is(k, &["r"], None) { p.flask = true; }
                }
                Press::Mouse(MouseButton::Left, false) => p.light = true,
                Press::Mouse(MouseButton::Left, true) => p.heavy = true,
                Press::Mouse(MouseButton::Right, true) => p.parry = true,
                Press::Mouse(..) => {}
            }
        }
        // A direction pressed together with the dodge is the roll's direction, not a step first.
        if p.dodge.is_some() && p.step != 0 { p.dodge = Some(p.step); p.step = 0; }
        Input {
            forward: held("w", NamedKey::ArrowUp),
            back: held("s", NamedKey::ArrowDown),
            run: shift_held(c),
            guard: c.is_mouse_held(MouseButton::Right) && !shift_held(c),
            ..*p
        }
    }

    /// Hitstop, slow motion, shake and flashes for what the fight did.
    fn show(&mut self, c: &mut Canvas, events: &[Event]) {
        for e in events {
            match *e {
                Event::Hit { mv: Move::Light, .. } => { c.hitstop(4); c.run(Action::CameraShake { intensity: 14.0, duration: 0.12 }); }
                Event::Hit { mv: Move::Heavy, .. } => {
                    c.hitstop(7);
                    c.run(Action::CameraShake { intensity: 36.0, duration: 0.22 });
                    c.run(Action::CameraZoomPunch { amount: 0.03, duration: 0.18 });
                }
                Event::Hit { mv: Move::Riposte, .. } => {
                    c.hitstop(12);
                    c.run(Action::CameraShake { intensity: 48.0, duration: 0.3 });
                    c.run(Action::CameraZoomPunch { amount: 0.06, duration: 0.35 });
                }
                Event::Parried { .. } => {
                    c.hitstop(10);
                    self.slow = (24, 0.5);
                    c.run(Action::CameraFlash { color: Color(255, 236, 190, 90), duration: 0.18 });
                }
                Event::JustDodge => self.slow = (30, 0.35),
                Event::StanceBroken { .. } => c.run(Action::CameraZoomPunch { amount: 0.04, duration: 0.25 }),
                Event::Felled { .. } => c.run(Action::CameraZoomPunch { amount: 0.05, duration: 0.3 }),
                Event::Struck { .. } => {
                    c.hitstop(6);
                    c.run(Action::CameraShake { intensity: 40.0, duration: 0.25 });
                    c.run(Action::CameraFlash { color: Color(160, 0, 0, 110), duration: 0.25 });
                }
                Event::Blocked { .. } => { c.hitstop(4); c.run(Action::CameraShake { intensity: 18.0, duration: 0.12 }); }
                Event::GuardBroken => { c.hitstop(8); c.run(Action::CameraShake { intensity: 44.0, duration: 0.3 }); }
                Event::Died => c.run(Action::CameraFlash { color: Color(60, 0, 0, 200), duration: 1.5 }),
                _ => {}
            }
        }
    }

    /// The warnings on the road for every attack under way: where it will land, how it is met.
    fn marks(&self, walked: f64) -> Vec<GroundMark> {
        self.fight.threats(walked).iter().map(|t| {
            let live = t.phase != Phase::Windup;
            // Brighter as the wind-up runs out; the fill tells the kind apart without colour.
            let a = if live { 200 } else { (70.0 + 90.0 * t.progress) as u8 };
            let (color, fill) = match t.kind {
                Kind::Physical => ([214, 48, 36, a], Fill::Solid),
                Kind::GuardBreak => ([236, 140, 30, a], Fill::Stripes),
                Kind::Unblockable => ([150, 70, 220, a], Fill::Dots),
            };
            // Short of the lane's edges, so a gap shows which lane is safe.
            let half = 0.4 * LANE_W;
            GroundMark {
                x: (t.lanes.0 as f32 * LANE_W - half, t.lanes.1 as f32 * LANE_W + half),
                d: t.d,
                color,
                fill,
                edge: Some([color[0].saturating_add(40), color[1].saturating_add(40), color[2].saturating_add(40), if live { 255 } else { a.saturating_add(40) }]),
            }
        }).collect()
    }

    fn paint_marks(&mut self, c: &mut Canvas, walked: f64) {
        let marks = self.marks(walked);
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // The camera moves with every new road frame, so repaint on those too.
        let frame = pf.current_image().cloned();
        let same_frame = match (&frame, &self.last_marks_frame) { (Some(a), Some(b)) => Arc::ptr_eq(a, b), (None, None) => true, _ => false };
        if marks == self.last_marks && (same_frame || marks.is_empty()) { return; }
        let img = if marks.is_empty() { None } else { pf.paint_marks(&marks) };
        self.last_marks = marks;
        self.last_marks_frame = frame;
        let Some(o) = c.get_game_object_mut(MARKS) else { return };
        match img {
            Some(img) => {
                let img = Arc::new(img);
                set_pixel_art(&img, true);
                o.visible = true;
                let size = o.size;
                o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None });
            }
            None => o.visible = false,
        }
    }

    /// The hero's frame now, from their Pixellab animations (seen from behind and to the right,
    /// as the camera sees them); standing still where a clip is missing.
    fn hero_frame(&self, s: &Sheet, walked: f64) -> Option<Arc<RgbaImage>> {
        let stand = || ["walk", "light", "guard"].iter().find_map(|n| s.clip(n)).map(|c| c.frames[0].clone());
        self.hero_clip_frame(s, walked).or_else(stand)
    }

    fn hero_clip_frame(&self, s: &Sheet, walked: f64) -> Option<Arc<RgbaImage>> {
        let h = &self.fight.hero;
        let clip = |n: &str| s.clip(n);
        let k = |t: u32, total: u32| t as f32 / total.max(1) as f32;
        // Falling takes a second, then they lie there until the road starts again.
        if h.dead { return clip("death").map(|c| c.at(self.dead_for as f32 / 60.0, false).clone()); }
        if let Some(a) = h.act {
            let spec = a.mv.spec();
            let (w, b, r) = spec.frames;
            return match a.mv {
                // The blow's frames, read off the north-east contact sheet: the light's blade is out
                // on 5 and 6, the heavy comes down on 7 and 8, the buckler flashes on 3 to 5.
                Move::Light => clip("light").map(|c| phased(c, spec.frames, a.t, (5, 7), false)),
                Move::Heavy | Move::Riposte => clip("heavy").map(|c| phased(c, spec.frames, a.t, (7, 9), false)),
                Move::Parry => clip("parry").map(|c| phased(c, spec.frames, a.t, (3, 6), false)),
                Move::Dodge(0) => clip("hop").map(|c| c.at(k(a.t, w + b + r), false).clone()),
                // The roll is drawn going right; to the left it is mirrored. A step is its start.
                Move::Dodge(d) => clip("dodge").map(|c| c.at(k(a.t, w + b + r), d < 0).clone()),
                Move::Step(d) => clip("dodge").map(|c| c.at(0.25 * k(a.t, w + b + r), d < 0).clone()),
                Move::Flask => clip("drink").map(|c| c.at(k(a.t, w + b + r), false).clone()),
            };
        }
        if h.reeling > 0 { return clip("hurt").map(|c| c.at(1.0 - h.reeling as f32 / 24.0, false).clone()); }
        if h.guarding { return clip("guard").map(|c| c.at(1.0, false).clone()); }
        // Walking strides with the road (a stride every 1.6 m); standing is the first frame.
        if self.fight.speed(&self.pending, walked) != 0.0 || self.last_speed != 0.0 {
            if let Some(c) = clip("walk") { return Some(c.looped(walked as f32, 1.6).clone()); }
        }
        None
    }

    /// An enemy's frame now, from the husk's Pixellab animations.
    fn husk_frame(&self, s: &Sheet, e: &Enemy) -> Option<Arc<RgbaImage>> {
        let clip = |n: &str| s.clip(n);
        // (The template shamble dropped the cleaver: an approach plays the idle loop.)
        let first = || ["idle", "chop"].iter().find_map(|n| clip(n)).map(|c| c.frames[0].clone());
        match e.ai {
            Ai::Attack { i, t, .. } => {
                let a = e.kind.attacks()[i];
                // The blow's frames in each clip: the chop strikes on 7 (its flash), the sweep
                // crosses on 7 to 9.
                let keys = if a.wide != 0 { (7, 10) } else { (7, 9) };
                clip(a.name).map(|c| phased(c, a.frames, t, keys, false)).or_else(first)
            }
            Ai::Staggered(n) => clip("stagger").map(|c| c.looped((150 - n) as f32, 60.0).clone()).or_else(first),
            _ if e.hurt > 0 => clip("hurt").map(|c| c.at(1.0 - e.hurt as f32 / 12.0, false).clone()).or_else(first),
            Ai::Dead => clip("death").map(|c| c.frames[c.frames.len() - 1].clone()),
            _ => clip("idle").map(|c| c.looped(self.ticks as f32, 90.0).clone()).or_else(first),
        }
    }

    /// Put the hero and the enemies where the road drawn now says they stand.
    fn place(&self, c: &mut Canvas, walked: f64) {
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // Where something stands and how big it is: its feet on screen and its size, in canvas units.
        let at = |x: f32, d: f32, metres: f32, img: &RgbaImage| -> Option<((f32, f32), (f32, f32))> {
            let (fx, fy) = pf.project_on(c, x, 0.0, d)?;
            let ppm = pf.scale_on(c, d)?;
            let h = metres * ppm;
            let w = h * img.width() as f32 / img.height() as f32;
            Some(((fx - 0.5 * w, fy - h), (w, h)))
        };
        let h = &self.fight.hero;
        let pose = match h.act.map(|a| (a.mv, a.phase())) {
            Some((Move::Light | Move::Heavy | Move::Riposte, Phase::Windup)) => 1,
            Some((Move::Light | Move::Heavy | Move::Riposte, _)) => 2,
            Some((Move::Parry, Phase::Windup | Phase::Active)) => 3,
            _ if h.guarding => 3,
            _ => 0,
        };
        // Pixellab's drawing when there is one, sized by the sheet's own figure; the stand-in else.
        let sheet_at = |s: &Sheet, x: f32, d: f32, metres: f32| -> Option<((f32, f32), (f32, f32))> {
            Some(s.place(pf.project_on(c, x, 0.0, d)?, metres, pf.scale_on(c, d)?))
        };
        let hero = match self.hero_sheet.as_ref().and_then(|s| Some((s, self.hero_frame(s, walked)?))) {
            Some((s, img)) => (!h.dead || s.clip("death").is_some()).then(|| sheet_at(s, h.x + SHOULDER, HERO_D, art::HERO_METRES)).flatten().map(|r| (r, img, 60)),
            None => {
                let img = &self.art.hero[pose];
                (!h.dead).then(|| at(h.x + SHOULDER, HERO_D, art::HERO_METRES, img)).flatten().map(|r| (r, img.clone(), 60))
            }
        };
        let mut placed: Vec<(String, Option<(((f32, f32), (f32, f32)), Arc<RgbaImage>, i32)>)> = vec![(HERO.to_owned(), hero)];
        for (i, e) in self.fight.enemies.iter().enumerate() {
            let d = e.ahead(walked);
            let pose = match (e.ai, e.attack()) {
                (Ai::Staggered(_), _) => 5,
                (_, Some((a, Phase::Windup, _))) => if a.wide != 0 { 3 } else { 1 },
                (_, Some((a, _, _))) => if a.wide != 0 { 4 } else { 2 },
                _ => 0,
            };
            let img = if e.hurt > 0 { &self.art.husk_hurt[pose] } else { &self.art.husk[pose] };
            // A hit sets it rocking.
            let sway = if e.hurt > 0 { 0.06 * (e.hurt as f32 * 1.3).sin() } else { 0.0 };
            let x = e.lane as f32 * LANE_W + sway;
            let drawn = self.husk_sheet.as_ref().and_then(|s| Some((s, self.husk_frame(s, e)?)));
            let (spot, img) = match drawn {
                Some((s, frame)) => ((e.alive() && d > 0.6).then(|| sheet_at(s, x, d, art::HUSK_METRES)).flatten(), frame),
                None => ((e.alive() && d > 0.6).then(|| at(x, d, art::HUSK_METRES, img)).flatten(), img.clone()),
            };
            let img = &img;
            // Nearer draws over further; all of them under the hero, who stands nearest.
            let layer = (50.0 - d * 2.0).round().clamp(1.0, 50.0) as i32;
            placed.push((enemy_id(i), spot.map(|r| (r, img.clone(), layer))));
        }
        for (id, spot) in placed {
            let Some(o) = c.get_game_object_mut(&id) else { continue };
            match spot {
                Some(((pos, size), img, layer)) => {
                    o.visible = true;
                    o.position = pos;
                    o.size = size;
                    o.layer = layer;
                    o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None });
                }
                None => o.visible = false,
            }
        }
    }

    /// The bars: the hero's health and stamina, and the nearest living enemy's health and stance.
    fn bars(&self, c: &mut Canvas, walked: f64) {
        let h = &self.fight.hero;
        let foe = self.fight.enemies.iter()
            .filter(|e| e.alive() && e.gap(walked) < 6.0 && e.gap(walked) > -0.5)
            .min_by(|a, b| a.gap(walked).total_cmp(&b.gap(walked)));
        let full = |id: &str| BARS.iter().find(|b| b.0 == id).map(|b| (b.3, b.4)).unwrap();
        let mut set = |id: &str, k: Option<f32>| {
            let (w, ht) = full(id);
            let Some(o) = c.get_game_object_mut(id) else { return };
            match k {
                Some(k) => { o.visible = true; o.size = ((w * k.clamp(0.0, 1.0)).max(1.0), ht); }
                None => o.visible = false,
            }
        };
        set("bar_hp", Some(h.hp / fight::HP));
        set("bar_st", Some(h.stamina / fight::STAMINA));
        set("bar_foe", foe.map(|e| e.hp / e.kind.hp()));
        set("bar_foe_stance", foe.map(|e| if matches!(e.ai, Ai::Staggered(_)) { 0.0 } else { e.stance / e.kind.stance() }));
    }

    fn hud(&mut self, c: &mut Canvas, walked: f64) {
        let Some(font) = self.font.clone() else { return };
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        let h = &self.fight.hero;
        let line = if h.dead {
            "YOU DIED".to_owned()
        } else {
            format!(
                "{} {:.1} ms  |  road {} fps  |  {:.1} m  |  flasks {}  |  WASD move  Shift run  Space dodge  LMB strike  Shift+LMB heavy  RMB guard  Shift+RMB parry  R drink",
                pf.engine().to_uppercase(), pf.render_ms(), self.frames.1, walked, h.flasks,
            )
        };
        // Text is laid out in window pixels: sized by the window's fit, like the box it sits in.
        let s = c.base_scale();
        let text = Text::new(vec![Span::new(line, 52.0 * s, Some(64.0 * s), font, Color(236, 228, 210, 255), 0.0)], Some(3600.0 * s), Align::Left, None);
        if let Some(o) = c.get_game_object_mut(HUD) { o.set_drawable(Box::new(text)); }
    }

    /// Back on the road with full health and the husks in front again.
    fn start_again(&mut self, walked: f64) {
        self.fight = Fight::new(first_enemies(walked));
        self.dead_for = 0;
    }
}

impl QuartzPlugin for Pilgrimage {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn name(&self) -> &str { "pilgrimage" }

    fn on_update(&mut self, c: &mut Canvas, dt: f32) {
        self.ticks += 1;
        // Slow motion counts down in real time.
        if self.slow.0 > 0 {
            self.slow.0 -= 1;
            c.set_time_scale(if self.slow.0 > 0 { self.slow.1 } else { 1.0 });
        }
        let presses = std::mem::take(&mut *self.presses.lock().unwrap());
        let input = self.input(c, &presses);
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // Where the frame shown stands (sprites and warnings go by it) and where the walk is headed
        // (the rules go by that, or a slow frame lets the hero walk on into an enemy).
        let walked = pf.state().walked;
        let ahead = pf.walked_ahead();
        // Count the road's new frames (its rate is its own, not the game's).
        if let Some(img) = pf.current_image() {
            if !self.last_frame.as_ref().is_some_and(|l| Arc::ptr_eq(l, img)) { self.frames.0 += 1; self.last_frame = Some(img.clone()); }
        }
        if self.ticks % 60 == 0 { self.frames = (0, self.frames.0); }
        // The fight runs on game time: slower in slow motion, not at all in a hitstop.
        self.owed += dt / Canvas::TICK;
        let mut events = Vec::new();
        let mut first = true;
        let mut ran = 0;
        while self.owed >= 1.0 {
            self.owed -= 1.0;
            ran += 1;
            let i = if first { input } else { Input { forward: input.forward, back: input.back, run: input.run, guard: input.guard, ..Input::default() } };
            if first { self.pending = Input::default(); }
            first = false;
            events.extend(self.fight.tick(&i, ahead));
        }
        let speed = self.fight.speed(&input, ahead);
        if let Some(pf) = c.get_plugin_mut::<PathForgePlugin>() { pf.set_speed(speed); }
        self.last_speed = speed;
        #[cfg(test)]
        self.log.extend(events.iter().map(|e| (self.ticks, *e)));
        self.show(c, &events);
        self.fx.tick(ran);
        self.fx.on_events(&events, &self.fight);
        if self.fight.hero.dead {
            self.dead_for += 1;
            if self.dead_for > 180 { self.start_again(ahead); }
        } else if self.fight.enemies.iter().all(|e| !e.alive()) {
            // Every husk down: three more further on.
            self.fight.enemies = first_enemies(ahead + 4.0);
        }
        self.paint_marks(c, walked);
        self.place(c, walked);
        self.fx.place(c, walked, &self.fight);
        self.bars(c, walked);
        if self.ticks % 15 == 1 || events.contains(&Event::Died) { self.hud(c, walked); }
    }
}
