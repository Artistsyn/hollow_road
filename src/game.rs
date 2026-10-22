//! The game on the canvas: the road drawn live by PathForge, the warnings painted on it, the hero
//! and the enemies placed on it each tick from the fight's road coordinates, the keys, the bars.
//!
//! Controls (PC and Mac, as in Dark Souls III and Elden Ring): W/S walk on and back, Shift held
//! runs, A/D roll a lane across (every lane change is the dodge; one that leaves a lane just as
//! a blow falls on it is perfect, and the attacker stumbles), Space steps back, left click strikes and Shift + left click strikes heavy,
//! right click held guards and Shift + right click parries, R drinks. Arrow keys walk and step too.
//! E rests at a shrine: the hero kneels, and the screen goes to the sanctum (`shrine`), where A/D
//! walk between the keeper, the shrine and the smith and E speaks; only the shrine lets them rise.
//!
//! The game opens on a title and the choice of a calling; a death falls, says so, goes dark and
//! wakes the hero getting up at the last shrine.
//!
//! The road is a journey of places (`world`): each holds its enemies, goes on when they are down,
//! and a fork goes the way of the lane the hero holds as it comes into view.

use crate::anim::{Clip, Sheet};
use crate::atlas;
use crate::art::{self, HuskPose, Pose};
use crate::fx::{self, Fx};
use crate::fight::{Ai, Archetype, Enemy, Event, Fight, Input, Kind, Look, Move, Phase, Reel, DOWN_FALL, DOWN_LIE, DOWN_RISE, HERO_D, LANE_W, SHOULDER, STUMBLE, TRAVEL};
use crate::profile::{self, Slot};
use crate::shrine::{self, Begin, Calling, Front, Meeting, Outcome, Panel, Sanctum};
use crate::side::{self, SideFight, SideInput};
use crate::world::{self, Go, Note, World};
use image::RgbaImage;
use quartz::plugin::QuartzPlugin;
use quartz::*;
use quartz_path_forge::light::Relighter;
use quartz_path_forge::marks::{Cast, Fill, GroundMark, Shadow, Sigil};
use quartz::plugin::terrain_collision::TerrainCollisionPlugin;
use quartz_path_forge::PathForgePlugin;
use std::cell::RefCell;
use std::sync::Mutex;

/// The canvas is 3840 x 2160; the road is drawn a little larger so a camera shake never shows
/// its edge.
const BLEED: f32 = 48.0;
/// The road's own pixels: rendered small and scaled up (the scene's look is pixel art).
const ROAD_PX: (u32, u32) = (480, 270);

const ROAD: &str = "road";
const MARKS: &str = "marks";
/// The side-on fight's boss, and its name, height, souls and the art it is drawn from.
const BOSS: &str = "boss";
const BOSS_METRES: f32 = 3.4;
/// Where the boss yard's middle lies, metres ahead of the walker, and the camera that sees it:
/// swung round to the path's side and stood back far enough to take in the whole yard.
/// (5 m out, low, the horizon a little above the middle and zoomed to about 35 px a metre on
/// the 480-wide road, so the yard fills the frame with its near edge on the bottom row.)
const ARENA_D: f32 = 0.0;
const SIDE_VIEW: &str = r#"{"view":"side_left","side":-5.0,"eye_height":2.0,"horizon":0.49,"zoom":1.26,"seconds":2.2}"#;
/// Ticks the yard stands still while the camera swings round, and after the boss falls.
const ARENA_IN: u32 = 140;
const ARENA_OUT: u32 = 150;
/// Ticks the yard stays after an ambush's last foe falls.
const AMBUSH_OUT: u32 = 80;
/// Degrees the camera turns to look down a side path, and ticks it looks before the walk goes.
const PEEK_YAW: f32 = 38.0;
const PEEK_T: u32 = 60;
const BOSS_HELP: &str = "A/D move   A A / D D (held) run   Shift+A/D roll   W jump   LMB strike   Shift+LMB heavy   RMB guard   Shift+RMB parry   R drink   F ranged attack   C cast      gold parry  ·  orange roll or block  ·  purple roll  ·  a blow held high: wait for it  ·  a dodged leap leaves it open";
const AMBUSH_HELP: &str = "A/D move   A A / D D (held) run   Shift+A/D roll   W jump   LMB strike   Shift+LMB heavy (cuts through all in reach)   RMB guard   Shift+RMB parry   R drink   F ranged attack   C cast      jump a low sweep  ·  step off a rune  ·  roll through a lunge or pounce";
/// A spot on the canvas: top-left and size.
type Rect = ((f32, f32), (f32, f32));

/// Where the hero is `k` of the way (0..1, on past 1 while the screen fades) into the mouth of the
/// side path on `side`, from where they stood on the road: out to its centreline and on along it.
fn peek_walk((x, d): (f32, f32), side: i8, k: f32) -> (f32, f32) {
    let e = k.min(1.0) * k.min(1.0) * (3.0 - 2.0 * k.min(1.0)) + (k - 1.0).max(0.0);
    (x + (side as f32 * 2.4 - x) * e, d + 1.8 * e)
}

/// +1 when the road runs left to right across the frame shown (metres ahead go right), -1 the
/// other way: which way the side-on fight's left and right go.
fn screen_dir(pf: &PathForgePlugin) -> f32 {
    match (pf.project(0.0, 0.0, ARENA_D), pf.project(0.0, 0.0, ARENA_D + 1.0)) {
        (Some(a), Some(b)) if (b.0 - a.0).abs() > 0.5 => (b.0 - a.0).signum(),
        _ => 1.0,
    }
}
/// Characters' and props' shadows on the road, under the marks.
const SHADOWS: &str = "shadows";
/// How much of the road's light a sprite takes (1: lit exactly as the road's own pictures are).
/// A little short of all of it, so a figure in deep shade still reads against the ground.
const SPRITE_LIGHT: f32 = 0.85;
const HERO: &str = "hero";
const HUD: &str = "hud";
/// A line across the middle of the screen: where the hero has come to, what to do.
const BANNER: &str = "banner";
/// The word over the hero's head for how a blow was met (a parry, a guard broken, a stagger).
const CALLOUT: &str = "callout";
/// Real ticks a callout shows, the last third fading.
const CALLOUT_T: u32 = 54;
/// Fight ticks a clean parry's flick is held out after it lands.
const FLOURISH_T: u32 = 18;
/// The controls, along the bottom.
const HELP: &str = "help";
/// The kit's shrine and signpost, stood on the road where the world says they are.
const SHRINE: &str = "shrine";
const SIGNPOST: &str = "signpost";
/// The souls left where the hero fell.
const STAIN: &str = "stain";
const CHEST: &str = "chest";
/// The keeper and the smith, in the sanctum (and the smith by the road until met).
const KEEPER: &str = "keeper";
const SMITH: &str = "smith";
/// The black over everything for a cut, and the box words and choices are drawn in.
const FADE: &str = "fade";
const PANEL_BG: &str = "panel_bg";
const PANEL: &str = "panel";
/// The world map (M), where it is drawn on the screen and how much larger than its picture, and its
/// place names.
const MAP: &str = "map";
const MAP_AT: (f32, f32) = (1800.0, 300.0);
const MAP_SCALE: f32 = 1.9;
fn map_label(i: usize) -> String { format!("map_label{i}") }
/// Where the sanctum's stations stand: across, and metres ahead of the camera.
const STATION_X: f32 = 3.2;
const STATION_D: f32 = HERO_D + 2.6;
/// Where the hero stands before a station, across.
const BEFORE_X: f32 = 0.7;
/// Bars: (id, x, y, width, height, colour) along the top left; the enemy's along the top middle.
/// Under the stamina, a bulwark's ward: dim gold while it fills, bright once it can burst.
const BARS: [(&str, f32, f32, f32, f32, [u8; 4]); 9] = [
    ("bar_hp_back", 60.0, 140.0, 900.0, 34.0, [30, 20, 22, 200]),
    ("bar_hp", 60.0, 140.0, 900.0, 34.0, [176, 36, 40, 255]),
    ("bar_st_back", 60.0, 186.0, 700.0, 22.0, [20, 26, 22, 200]),
    ("bar_st", 60.0, 186.0, 700.0, 22.0, [70, 150, 80, 255]),
    ("bar_ward_back", 60.0, 218.0, 500.0, 16.0, [24, 22, 16, 200]),
    ("bar_ward", 60.0, 218.0, 500.0, 16.0, [150, 132, 84, 255]),
    ("bar_ward_ready", 60.0, 218.0, 500.0, 16.0, [255, 236, 170, 255]),
    ("bar_foe", 1320.0, 140.0, 1200.0, 26.0, [150, 40, 44, 255]),
    ("bar_foe_stance", 1320.0, 172.0, 1200.0, 12.0, [214, 176, 70, 255]),
];

fn enemy_id(i: usize) -> String { format!("enemy_{i}") }
/// A shot in flight: a side-on foe's (one at a time each), or an arrow or hex down the road.
fn shot_id(i: usize) -> String { format!("shot_{i}") }
/// Shots that can be drawn at once: an archer's second arrow can be in the air with its first.
const SHOTS: usize = 2 * world::MOST_FOES;

/// An arrow in flight, pointing right: a dark shaft with pale fletching and a head hot in its
/// warning's colour, inside a glow of that colour that trails behind it.
fn arrow_picture(rgb: [u8; 3]) -> Arc<RgbaImage> {
    let (w, h) = (48u32, 11u32);
    Arc::new(RgbaImage::from_fn(w, h, |x, y| {
        let (u, v) = (x as f32 / (w - 1) as f32, (y as f32 - (h - 1) as f32 * 0.5) / ((h - 1) as f32 * 0.5));
        // A comet's tail: full across the middle, thinning and fading toward the fletching.
        let reach = 0.35 + 0.65 * u;
        let glow = (1.0 - (v.abs() / reach).powi(2)).max(0.0) * (0.2 + 0.8 * u) * 0.95;
        let mut px = [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32, glow * 255.0];
        let mut put = |c: [u8; 3]| px = [c[0] as f32, c[1] as f32, c[2] as f32, 255.0];
        if u > 0.8 && v.abs() <= 0.9 * (1.0 - u) / 0.2 + 0.05 {
            let k = 0.75;
            put([(rgb[0] as f32 + (255.0 - rgb[0] as f32) * k) as u8, (rgb[1] as f32 + (255.0 - rgb[1] as f32) * k) as u8, (rgb[2] as f32 + (255.0 - rgb[2] as f32) * k) as u8]);
        } else if (0.04..=0.84).contains(&u) && v.abs() < 0.1 {
            put([92, 64, 42]);
        } else if u < 0.22 && v.abs() > 0.1 && v.abs() < 0.2 + 2.2 * (0.22 - u) {
            put([214, 204, 182]);
        }
        image::Rgba([px[0] as u8, px[1] as u8, px[2] as u8, px[3] as u8])
    }))
}

/// A hex in flight, `age` ticks out: a white-violet core in a violet body, a halo in its
/// warning's colour that flickers, and three sparks circling it.
fn hex_picture(rgb: [u8; 3], age: u32) -> Arc<RgbaImage> { orb_picture([150, 62, 232], rgb, age) }

/// The hero's frost bolt in flight: an ice-blue orb with a pale halo.
fn frost_picture(age: u32) -> Arc<RgbaImage> { orb_picture([70, 160, 250], [200, 240, 255], age) }

/// An orb in flight, `age` ticks out: a white core in a `body` of colour, a flickering halo of
/// `rgb`, and three sparks circling it.
fn orb_picture(body: [u8; 3], rgb: [u8; 3], age: u32) -> Arc<RgbaImage> {
    let n = 33u32;
    let c = (n - 1) as f32 * 0.5;
    let t = age as f32;
    let sparks: Vec<(f32, f32)> = (0..3).map(|i| {
        let a = 0.21 * t + i as f32 * std::f32::consts::TAU / 3.0;
        (c + 0.74 * c * a.cos(), c + 0.74 * c * a.sin())
    }).collect();
    Arc::new(RgbaImage::from_fn(n, n, |x, y| {
        let (dx, dy) = (x as f32 - c, y as f32 - c);
        let q = dx.hypot(dy) / c;
        if sparks.iter().any(|&(sx, sy)| (x as f32 - sx).abs() <= 1.0 && (y as f32 - sy).abs() <= 1.0) { return image::Rgba([255, 226, 255, 255]); }
        let flicker = 0.8 + 0.2 * (0.4 * t + 3.0 * dy.atan2(dx)).sin();
        let mix = |a: [f32; 3], b: [f32; 3], k: f32| [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k];
        let (core, body, halo) = ([248.0, 236.0, 255.0], [body[0] as f32, body[1] as f32, body[2] as f32], [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32]);
        let (col, a) = if q < 0.28 { (core, 1.0) }
            else if q < 0.55 { (mix(core, body, (q - 0.28) / 0.27), 1.0) }
            else if q < 1.0 { (mix(body, halo, ((q - 0.55) / 0.2).min(1.0)), (1.0 - (q - 0.55) / 0.45).powf(1.4) * flicker) }
            else { (halo, 0.0) };
        image::Rgba([col[0] as u8, col[1] as u8, col[2] as u8, (a * 255.0) as u8])
    }))
}

/// A rune's circle on the ground: violet, drawn in as its wind-up runs out, washed when it goes
/// off, and dim while its caster recovers. `ticks` turns it slowly.
fn rune_sigil(x: (f32, f32), d: (f32, f32), phase: Phase, progress: f32, ticks: u32) -> Sigil {
    let (open, color, fill) = match phase {
        Phase::Windup => (progress, [196, 128, 255, (120.0 + 120.0 * progress) as u8], None),
        Phase::Active => (1.0, [236, 200, 255, 255], Some([170, 90, 255, 130])),
        Phase::Recover => (1.0, [170, 110, 230, 90], None),
    };
    Sigil { centre: (0.5 * (x.0 + x.1), 0.5 * (d.0 + d.1)), radius: (0.5 * (x.1 - x.0), 0.5 * (d.1 - d.0)), color, turn: 0.012 * ticks as f32, open, fill }
}

/// A shot's picture: a streak glowing in its warning's colour, white at its head, pointing right.
fn shot_picture(rgb: [u8; 3]) -> Arc<RgbaImage> {
    let (w, h) = (40u32, 14u32);
    Arc::new(RgbaImage::from_fn(w, h, |x, y| {
        let (u, v) = (x as f32 / (w - 1) as f32, (y as f32 - (h - 1) as f32 * 0.5) / ((h - 1) as f32 * 0.5));
        // Thick at the head (the right), thinning to a tail.
        let width = 0.25 + 0.75 * u.powf(1.5);
        let core = (1.0 - (v / width).abs()).clamp(0.0, 1.0);
        let a = (core * (0.25 + 0.75 * u)).powf(0.8);
        let white = (core * u).powf(3.0);
        let mix = |c: u8| (c as f32 + (255.0 - c as f32) * white) as u8;
        image::Rgba([mix(rgb[0]), mix(rgb[1]), mix(rgb[2]), (a * 255.0) as u8])
    }))
}

/// The folder of each kind's Pixellab animations, and how tall it stands, metres.
fn kind_art(k: Archetype) -> (&'static str, f32) {
    match k {
        Archetype::Husk => ("husk", art::HUSK_METRES),
        Archetype::Lancer => ("lancer", 1.9),
        Archetype::Shieldbearer => ("shieldbearer", 2.0),
        Archetype::Wretch => ("wretch", 1.55),
        Archetype::Archer => ("archer", 1.85),
        Archetype::Hexer => ("hexer", 1.9),
    }
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
    canvas.add_game_object(SHADOWS.to_owned(), road_rect(SHADOWS, -99));
    canvas.add_game_object(MARKS.to_owned(), road_rect(MARKS, -98));
    let journey = crate::assets::path(crate::assets::JOURNEY);
    match PathForgePlugin::live(journey, ROAD, ROAD_PX) {
        // The road is pixel art scaled up eight times: hard pixels, like the sprites over it.
        // It measures its light along the road, so the figures on it are lit as it is.
        Ok(p) => canvas.add_plugin(p.with_speed(0.0).with_pixel_art(true).with_sprite_light()),
        Err(e) => eprintln!("hollow_road: the road would not open: {e}"),
    }

    let sprite = |id: &str, layer: i32| { let mut o = GameObject::build(id).size(1.0, 1.0).position(0.0, 0.0).layer(layer).finish(); o.unlit = true; o };
    canvas.add_game_object(HERO.to_owned(), sprite(HERO, 60));
    canvas.add_game_object(BOSS.to_owned(), sprite(BOSS, 40));
    let mut world = World::new(0.0);
    for i in 0..world::MOST_FOES { canvas.add_game_object(enemy_id(i), sprite(&enemy_id(i), 10)); }
    for i in 0..SHOTS { canvas.add_game_object(shot_id(i), sprite(&shot_id(i), 45)); }
    for id in [SHRINE, SIGNPOST, STAIN, KEEPER, SMITH, CHEST] { canvas.add_game_object(id.to_owned(), sprite(id, 5)); }
    // The slash in front of the hero, so behind them (60) and over every enemy (50 and under);
    // the hit bursts over everything on the road. A burst's object is a clear picture that only
    // carries its effect.
    canvas.add_game_object(fx::SLASH.to_owned(), sprite(fx::SLASH, 55));
    for i in 0..fx::BURSTS {
        let mut o = sprite(&fx::burst_id(i), 65);
        o.visible = false;
        o.set_image(Image { shape: ShapeType::Rectangle(0.0, (1.0, 1.0), 0.0), image: swatch([0, 0, 0, 0]), color: None });
        canvas.add_game_object(fx::burst_id(i), o);
    }
    let hud = GameObject::build(HUD).size(3600.0, 120.0).position(60.0, 40.0).layer(200).screen_space().finish();
    canvas.add_game_object(HUD.to_owned(), hud);
    let banner = GameObject::build(BANNER).size(3840.0, 300.0).position(0.0, 560.0).layer(200).screen_space().finish();
    canvas.add_game_object(BANNER.to_owned(), banner);
    let mut callout = GameObject::build(CALLOUT).size(1800.0, 140.0).position(1020.0, 900.0).layer(202).screen_space().finish();
    callout.visible = false;
    canvas.add_game_object(CALLOUT.to_owned(), callout);
    let help = GameObject::build(HELP).size(3720.0, 80.0).position(60.0, 2060.0).layer(200).screen_space().finish();
    canvas.add_game_object(HELP.to_owned(), help);
    for (id, x, y, w, h, c) in BARS {
        let mut o = GameObject::build(id).size(w, h).position(x, y).layer(201).screen_space().finish();
        o.unlit = true;
        o.set_image(Image { shape: ShapeType::Rectangle(0.0, (w, h), 0.0), image: swatch(c), color: None });
        canvas.add_game_object(id.to_owned(), o);
    }

    // The box for words and choices over the road, and the black for cuts over everything.
    for (id, layer) in [(PANEL_BG, 210), (FADE, 300)] {
        let mut o = GameObject::build(id).size(3840.0, 2160.0).position(0.0, 0.0).layer(layer).screen_space().finish();
        o.unlit = true;
        o.visible = id == FADE;
        o.set_image(Image { shape: ShapeType::Rectangle(0.0, (3840.0, 2160.0), 0.0), image: swatch([0, 0, 0, if id == FADE { 255 } else { 0 }]), color: None });
        canvas.add_game_object(id.to_owned(), o);
    }
    let panel = GameObject::build(PANEL).size(2240.0, 640.0).position(800.0, 1320.0).layer(211).screen_space().finish();
    canvas.add_game_object(PANEL.to_owned(), panel);
    let size = (crate::map::W as f32 * MAP_SCALE, crate::map::H as f32 * MAP_SCALE);
    let mut o = GameObject::build(MAP).size(size.0, size.1).position(MAP_AT.0, MAP_AT.1).layer(212).screen_space().finish();
    (o.unlit, o.visible) = (true, false);
    canvas.add_game_object(MAP.to_owned(), o);
    for i in 0..atlas::PLACES.len() {
        let mut o = GameObject::build(&map_label(i)).size(420.0, 60.0).layer(213).screen_space().finish();
        o.visible = false;
        canvas.add_game_object(map_label(i), o);
    }

    let presses = Arc::new(Mutex::new(Vec::new()));
    let q = presses.clone();
    canvas.on_key_press(move |c, k| { let shift = shift_held(c); q.lock().unwrap().push(Press::Key(k.clone(), shift)); });
    let q = presses.clone();
    canvas.on_mouse_press(move |c, b, _| { let shift = shift_held(c); q.lock().unwrap().push(Press::Mouse(b, shift)); });
    canvas.add_plugin(Pilgrimage::new(Fight::new(world.foes()), world, presses));
}

/// A key or a click, as it came, with whether Shift was down.
#[derive(Clone, Debug)]
enum Press { Key(Key, bool), Mouse(MouseButton, bool) }

/// A second press of the same way within this long (seconds of play), held, is a run.
const DOUBLE_TAP: f64 = 0.3;

/// The way a key points (W, A, S, D or the arrows), as `w`, `a`, `s` or `d`.
fn way(k: &Key) -> Option<char> {
    [('w', "w", NamedKey::ArrowUp), ('a', "a", NamedKey::ArrowLeft), ('s', "s", NamedKey::ArrowDown), ('d', "d", NamedKey::ArrowRight)]
        .into_iter().find(|(_, ch, n)| is(k, &[ch], Some(*n))).map(|w| w.0)
}
fn way_held(c: &Canvas, w: char) -> bool {
    let (ch, n) = match w { 'w' => ("w", NamedKey::ArrowUp), 'a' => ("a", NamedKey::ArrowLeft), 's' => ("s", NamedKey::ArrowDown), _ => ("d", NamedKey::ArrowRight) };
    c.is_key_held(&Key::Character(ch.into())) || c.is_key_held(&Key::Named(n))
}

/// A key as the title, the sanctum and their menus read it.
fn menu_key(p: &Press) -> Option<shrine::Key> {
    let Press::Key(k, _) = p else { return None };
    use shrine::Key as M;
    [(M::Left, &["a"][..], NamedKey::ArrowLeft), (M::Right, &["d"], NamedKey::ArrowRight), (M::Up, &["w"], NamedKey::ArrowUp),
     (M::Down, &["s"], NamedKey::ArrowDown), (M::Use, &["e", " "], NamedKey::Enter), (M::Back, &["q"], NamedKey::Escape)]
        .iter().find(|(_, ch, n)| is(k, ch, Some(*n))).map(|m| m.0)
        .or_else(|| (is(k, &[], Some(NamedKey::Space))).then_some(M::Use))
        .or_else(|| (is(k, &[], Some(NamedKey::Backspace))).then_some(M::Back))
}

/// What the game is doing: the fight on the road is only one of them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Mode {
    /// The title and the choice of a calling, over the road standing still.
    Front(Front),
    /// On the road: walking and fighting.
    Play,
    /// Kneeling at a shrine, `t` ticks in, before the dark.
    Kneel(u32),
    /// Going dark; once black, what comes next.
    Out(Next),
    /// Black, waiting for the road to reach where it was sent; then what to show.
    Cut(After),
    /// At rest in the sanctum.
    Sanctum(Sanctum),
    /// Up from rest by the shrine on the road, `t` ticks in (the kneel backwards).
    Rise(u32),
    /// Fallen, `t` ticks in: the fall, YOU DIED, then the dark.
    Dying(u32),
    /// Getting up off the ground at the shrine after a death (or at the start), `t` ticks in.
    Wake(u32),
    /// Words with someone met on the road.
    Meet(Meeting),
    /// A side-on fight: the boss, or an ambush.
    Side(Box<Arena>),
    /// Turning to look down a side path (-1 left, 1 right), `t` ticks in, before walking down it.
    Peek(u32, i8),
    /// The world map open, the road standing still.
    Map,
}

/// The boss fight under way: the fight, ticks since it began (the camera swings round first),
/// and ticks since the boss fell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Arena { pub side: SideFight, pub t: u32, pub done: u32, pub why: Why }

/// What a side-on fight is: the boss at the gate, or one of the place's ambushes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Why { Boss, Ambush(usize) }

/// What a fade to black leads to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Next { Start, Resume, Sanctum, Road, Respawn, Detour(i8), Back, Turn }
/// What a cut shows once the road is there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum After { Sanctum, Rise, Wake, Walk }

/// How a panel is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style { Title, Menu, Died }

const KNEEL_T: u32 = 50;
const RISE_T: u32 = 40;
const WAKE_T: u32 = 80;
const DIE_T: u32 = 150;
/// How much a fade moves a tick (half a second end to end).
const FADE_STEP: f32 = 1.0 / 30.0;

fn shift_held(c: &Canvas) -> bool { c.is_key_held(&Key::Named(NamedKey::Shift)) }

/// The pictures, made once.
struct Art {
    hero: [Arc<RgbaImage>; 4],
    husk: [Arc<RgbaImage>; 6],
    husk_hurt: [Arc<RgbaImage>; 6],
    /// The glow of souls left on the road.
    stain: Arc<RgbaImage>,
}

impl Art {
    fn new() -> Art {
        let husks = [HuskPose::Idle, HuskPose::Raised, HuskPose::Chop, HuskPose::Wound, HuskPose::Sweep, HuskPose::Reeling];
        Art {
            hero: [Pose::Ready, Pose::Raised, Pose::Struck, Pose::Guard].map(|p| Arc::new(art::hero(p, false))),
            husk: husks.map(|p| Arc::new(art::husk(p, false))),
            husk_hurt: husks.map(|p| Arc::new(art::husk(p, true))),
            stain: swatch([150, 255, 190, 210]),
        }
    }
}

/// The game's own plugin: reads the keys, runs the fight, drives the road and places the sprites.
pub struct Pilgrimage {
    pub(crate) fight: Fight,
    /// Every event, with the fight tick it came on (for the headless capture).
    #[cfg(test)]
    pub(crate) log: Vec<(u64, Event)>,
    /// Events a capture puts in, shown on the next tick as if the fight had made them.
    #[cfg(test)]
    pub(crate) staged: Vec<Event>,
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
    /// Each kind's sheet; a kind without one yet borrows the husk's.
    enemy_sheets: Vec<(Archetype, Sheet)>,
    font: Option<Arc<Font>>,
    ticks: u64,
    /// Road frames shown in the last second, and the frame last seen (to count new ones).
    frames: (u32, u32),
    last_frame: Option<Arc<RgbaImage>>,
    /// The road's speed last tick (the hero strides while it moves).
    last_speed: f32,
    /// The warnings painted last, to repaint only on change.
    last_marks: Vec<GroundMark>,
    last_sigils: Vec<Sigil>,
    last_marks_frame: Option<Arc<RgbaImage>>,
    /// Sprite frames lit by the road's light, and the shadows last painted with the frame they
    /// were painted for (`place` works from `&self`).
    relit: RefCell<Relighter>,
    /// Foes' frames frosted over while a frost bolt's chill holds them, by the frame (kept beside
    /// it, so its address is not reused while it is a key).
    frosted: RefCell<std::collections::HashMap<usize, (Arc<RgbaImage>, Arc<RgbaImage>)>>,
    last_shadows: RefCell<(Vec<Shadow>, Vec<Cast>, Option<Arc<RgbaImage>>)>,
    /// Traces each sprite frame's outline from its pixels once, for the shadow it casts.
    tracer: RefCell<TerrainCollisionPlugin>,
    /// The Bell Warden's Pixellab art (also what a boss without its own is drawn as), and presses
    /// waiting for the side-on fight.
    boss_sheet: Option<Sheet>,
    /// The art of the boss of the place the hero is in, by its folder, loaded on arriving there
    /// (one at a time: each is ~20 MB of frames).
    boss_art: Option<(&'static str, Option<Sheet>)>,
    /// The world map's picture while it is open.
    map_img: Option<Arc<RgbaImage>>,
    side_pending: SideInput,
    /// The last way pressed without Shift and when (seconds played), and the way being run: a
    /// double tap, run while its second press is held.
    tap: Option<(char, f64)>,
    running: Option<char>,
    fx: Fx,
    pub(crate) world: World,
    pub(crate) mode: Mode,
    /// The calling chosen at the start.
    calling: Option<Calling>,
    /// The black over everything (0 clear, 1 black) and where it is headed.
    fade: (f32, f32),
    /// The keeper's and the smith's Pixellab sheets.
    keeper_sheet: Option<Sheet>,
    smith_sheet: Option<Sheet>,
    /// The banner's line and the real ticks it has left.
    banner: (String, u32),
    /// The callout's word, its colour and the real ticks it has left.
    callout: (String, [u8; 3], u32),
    /// Fight ticks left of a clean parry's follow-through (the buckler's flick held out), drawn
    /// while nothing else is under way: the parry itself ends at once so a riposte can follow.
    flourish: u32,
    /// The kit's shrine and signpost pictures.
    props: Option<(Arc<RgbaImage>, Arc<RgbaImage>)>,
    /// A cache's chest, shut and opened.
    chest: Option<(Arc<RgbaImage>, Arc<RgbaImage>)>,
    /// The save slot this pilgrimage is kept in (`profile`), the folder the slots are in, and the
    /// slots as the title lists them.
    pub(crate) slot: Option<usize>,
    pub(crate) saves: std::path::PathBuf,
    slots: Vec<Slot>,
    /// Seconds played, the save last written and the tick it was (`keep`), and whether the last
    /// try failed (said once, not at every look).
    played: f64,
    kept: Option<(serde_json::Value, u64)>,
    keep_failed: bool,
}

/// How often the pilgrimage is looked at to be kept, ticks; a save that has not changed is
/// written again only for the time played, once a minute.
const KEEP_EVERY: u64 = 300;
const KEEP_IDLE: u64 = 3600;

/// Where the slots are kept: the player's own (`profile::dir`), or under the build's target for
/// a test, so a capture never touches a real pilgrimage.
fn save_dir() -> std::path::PathBuf {
    if cfg!(test) { std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test_saves") } else { profile::dir() }
}

/// An attack's warning colour, by how it can be met: gold for a parry, red for a blow that only
/// blocks, red-orange for one that breaks guards, purple for one only a roll escapes.
pub(crate) fn warning_rgb(kind: Kind, parry: bool) -> [u8; 3] {
    match kind {
        Kind::Physical if parry => [240, 190, 56],
        Kind::Physical => [214, 48, 36],
        Kind::GuardBreak => [238, 92, 28],
        Kind::Unblockable => [156, 72, 230],
    }
}

fn kit_picture(name: &str) -> Option<Arc<RgbaImage>> {
    let path = crate::assets::path(format!("kits/hollowmere/{name}/{name}_1.png"));
    let img = Arc::new(image::open(path).ok()?.to_rgba8());
    set_pixel_art(&img, true);
    Some(img)
}

/// A still Pixellab picture (`assets/pixellab/<path>`), as pixel art.
fn pixellab_picture(path: &str) -> Option<Arc<RgbaImage>> {
    let img = Arc::new(image::open(crate::assets::path("pixellab").join(path)).ok()?.to_rgba8());
    set_pixel_art(&img, true);
    Some(img)
}

fn load_sheet(name: &str) -> Option<Sheet> {
    Sheet::load(&crate::assets::path("pixellab").join(name))
}

/// A sheet whose bare-handed clips have been given every weapon there is art for.
pub fn armed_sheet(name: &str) -> Option<Sheet> {
    let mut s = load_sheet(name)?;
    let dir = crate::assets::path("pixellab/weapons");
    for a in crate::gear::ITEMS.iter().filter_map(|i| i.arms).chain([crate::gear::SWORD]) {
        if let Some(w) = crate::anim::Weapon::load(&dir, a.art) {
            s.arm(a.art, a.class.slug(), &w);
            s.arm(a.art, a.class.family(), &w);
        }
    }
    // The offhands, drawn on when worn (`Sheet::dress`).
    let dir = crate::assets::path("pixellab/offhand");
    for o in crate::gear::ITEMS.iter().filter_map(|i| i.off) {
        if let Some(sh) = crate::anim::Shield::load(&dir, o.art) { s.shields.insert(o.art.to_owned(), std::sync::Arc::new(sh)); }
    }
    // Bows, crossbows and catalysts, gripped in the fist.
    let helds = crate::gear::ITEMS.iter().filter_map(|i| i.ranged.map(crate::gear::Ranged::art).or(i.catalyst.map(|c| c.art)));
    for art in helds {
        if let Some(g) = crate::anim::Held::load(&dir, art) { s.helds.insert(art.to_owned(), std::sync::Arc::new(g)); }
    }
    Some(s)
}

/// What `kit` has in the other hand to be drawn: the shield, else the bow or focus.
fn off_art(kit: &crate::gear::Kit) -> Option<&'static str> { kit.off.map(|o| o.art).or(kit.ranged.map(crate::gear::Ranged::art)).or(kit.catalyst.map(|c| c.art)) }

/// The bare-handed clip `bare` as `kit` holds it: weapon and offhand (`<bare>@<art>+<off>`, see
/// `Sheet::dress`), else the weapon alone (`<bare>@<art>`).
fn kitted<'a>(s: &'a Sheet, kit: &crate::gear::Kit, bare: &str) -> Option<&'a Clip> {
    off_art(kit).and_then(|o| s.clip(&format!("{bare}@{}+{o}", kit.arms.art))).or_else(|| s.clip(&format!("{bare}@{}", kit.arms.art)))
}

/// The `<prefix>` ranged attack (or, `cast`, the spell) as `kit` makes it, and the frames where it
/// lets go: a bow's draw with the bow in the fist (`<prefix>loose+<bow>`, see `Sheet::dress`), the
/// release on 6 and 7; a spell through the catalyst, and a hand crossbow aimed, the other arm
/// thrust out with the weapon held at guard (`<prefix><family>_cast`), letting go on 5 and 6.
fn loosing<'a>(s: &'a Sheet, kit: &crate::gear::Kit, prefix: &str, cast: bool) -> Option<(&'a Clip, (usize, usize))> {
    match kit.shot(cast)? {
        crate::gear::Ranged::Frost | crate::gear::Ranged::BloodCrossbow => kitted(s, kit, &format!("{prefix}{}_cast", kit.arms.class.family())).map(|c| (c, (5, 7))),
        r => s.clip(&format!("{prefix}loose+{}", r.art())).map(|c| (c, (6, 8))),
    }
}

/// The clip `name` (`e_` side-on) with what is worn in hand: the bare-handed clip drawn for the
/// way the weapon is held (`<family>_<name>`, see `Class::family`) with weapon and offhand put in
/// (`kitted`) where there is one, else the clip as drawn (the Wayfarer's sword in it).
fn worn<'a>(s: &'a Sheet, kit: &crate::gear::Kit, name: &str) -> Option<&'a Clip> {
    let (pre, rest) = name.strip_prefix("e_").map_or(("", name), |r| ("e_", r));
    kitted(s, kit, &format!("{pre}{}_{rest}", kit.arms.class.family())).or_else(|| s.clip(name))
}

/// The `<prefix><move>` clip as the weapon held swings it: its type's own clip with what is worn
/// in hand (`<prefix><type>_<move>`, `kitted`) where there is one, else the sword's.
fn held<'a>(s: &'a Sheet, kit: &crate::gear::Kit, prefix: &str, mv: &str) -> Option<&'a Clip> {
    kitted(s, kit, &format!("{prefix}{}_{mv}", kit.arms.class.slug())).or_else(|| s.clip(&format!("{prefix}{mv}")))
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
        else if t < w + act + rec { span(b, n, (t - w - act) as f32 / rec.max(1) as f32) }
        // Stuck past its recovery after a miss: held where the blow came down.
        else { span(a, b, 0.999) };
    clip.at((f + 0.5) / n as f32, mirror).clone()
}

fn is(k: &Key, chars: &[&str], named: Option<NamedKey>) -> bool {
    match k { Key::Character(c) => chars.iter().any(|ch| c.eq_ignore_ascii_case(ch)), Key::Named(n) => Some(*n) == named }
}

impl Pilgrimage {
    fn new(mut fight: Fight, world: World, presses: Arc<Mutex<Vec<Press>>>) -> Pilgrimage {
        fight.hero.stats.kit = world.gear.kit();
        let font = Font::from_bytes(include_bytes!("../assets/font.ttf")).ok().map(Arc::new);
        Pilgrimage {
            #[cfg(test)]
            log: Vec::new(),
            #[cfg(test)]
            staged: Vec::new(),
            fight, presses, pending: Input::default(), owed: 0.0, slow: (0, 1.0), dead_for: 0,
            art: Art::new(), hero_sheet: armed_sheet("wayfarer_ne").or_else(|| load_sheet("wayfarer")), enemy_sheets: Archetype::ALL.iter().filter_map(|&k| Some((k, load_sheet(kind_art(k).0)?))).collect(), font, ticks: 0, frames: (0, 0), last_frame: None, last_speed: 0.0, last_marks: Vec::new(), last_sigils: Vec::new(), last_marks_frame: None, relit: RefCell::new(Relighter::new()), frosted: RefCell::new(std::collections::HashMap::new()), last_shadows: RefCell::new((Vec::new(), Vec::new(), None)), tracer: RefCell::new(TerrainCollisionPlugin::new()), boss_sheet: load_sheet("bell_warden"), boss_art: None, map_img: None, side_pending: SideInput::default(), tap: None, running: None, fx: Fx::load(),
            world, mode: Mode::Front(Front::Title), calling: None, fade: (1.0, 0.0), keeper_sheet: load_sheet("keeper"), smith_sheet: load_sheet("smith"), banner: (String::new(), 0), callout: (String::new(), [0; 3], 0), flourish: 0, props: kit_picture("shrine").zip(kit_picture("signpost")), chest: pixellab_picture("chest/shut.png").zip(pixellab_picture("chest/open.png")),
            slot: None, saves: save_dir(), slots: Vec::new(), played: 0.0, kept: None, keep_failed: false,
        }
        .listed()
    }

    /// With the slots read for the title.
    fn listed(mut self) -> Pilgrimage { self.slots = profile::summaries(&self.saves); self }

    /// Keep the pilgrimage in its slot, if it has one and something has changed (or a minute of
    /// play has gone by since it was last written); not before the world it was set out into is
    /// made, nor on the title.
    pub(crate) fn keep(&mut self) {
        if matches!(self.mode, Mode::Front(_) | Mode::Out(Next::Start | Next::Resume)) { return; }
        self.write_kept(false);
    }

    /// Keep the pilgrimage as it is this moment, changed or not, on leaving it from the pause
    /// menu (for the list, or out of the game): the autosave looks only every few seconds, and
    /// what was done since, and the time played, would be lost. Whether it was written.
    fn keep_leaving(&mut self) -> bool { self.write_kept(true) }

    fn write_kept(&mut self, now: bool) -> bool {
        let (Some(slot), Some(calling)) = (self.slot, self.calling) else { return false };
        let record = profile::record(calling, &self.fight.hero.stats, self.played as u64, &self.world);
        if let Some((last, at)) = self.kept.as_ref().filter(|_| !now) {
            if last["world"] == record["world"] && last["hero"] == record["hero"] && self.ticks < at + KEEP_IDLE { return true; }
        }
        match profile::write(&self.saves, slot, &record) {
            Ok(()) => { (self.kept, self.keep_failed) = (Some((record, self.ticks)), false); true }
            // Tried again at the next look.
            Err(e) => {
                if !self.keep_failed || now { eprintln!("[hollow_road] could not keep pilgrimage {}: {e}", slot + 1); }
                self.keep_failed = true;
                false
            }
        }
    }

    /// Double taps: a way pressed twice in quick succession (without Shift) is run while the
    /// second press stays held.
    fn taps(&mut self, c: &Canvas, presses: &[Press]) {
        for press in presses {
            let Press::Key(k, false) = press else { continue };
            let Some(w) = way(k) else { continue };
            if self.tap.is_some_and(|(t, at)| t == w && self.played - at <= DOUBLE_TAP) { self.running = Some(w); }
            self.tap = Some((w, self.played));
        }
        if self.running.is_some_and(|w| !way_held(c, w)) { self.running = None; }
    }

    /// A chilled foe's frame: paled toward ice, its shading kept, its edge rimed. Made once a frame.
    fn frosted(&self, img: Arc<RgbaImage>) -> Arc<RgbaImage> {
        let mut made = self.frosted.borrow_mut();
        if made.len() > 256 { made.clear(); }
        made.entry(Arc::as_ptr(&img) as usize).or_insert_with(|| {
            let mut out = (*img).clone();
            let (w, h) = out.dimensions();
            let solid = |x: i64, y: i64| x >= 0 && y >= 0 && x < w as i64 && y < h as i64 && img.get_pixel(x as u32, y as u32).0[3] > 40;
            for (x, y, p) in out.enumerate_pixels_mut() {
                let [r, g, b, a] = p.0;
                if a == 0 { continue; }
                let l = 0.3 * r as f32 + 0.59 * g as f32 + 0.11 * b as f32;
                let (x, y) = (x as i64, y as i64);
                // Rime where the figure meets the air.
                let edge = !(solid(x - 1, y) && solid(x + 1, y) && solid(x, y - 1) && solid(x, y + 1));
                let ice = if edge { [225.0, 245.0, 255.0] } else { [0.55 * l + 60.0, 0.75 * l + 70.0, 0.85 * l + 110.0] };
                let k = if edge { 0.8 } else { 0.55 };
                let mix = |c: u8, i: f32| (c as f32 * (1.0 - k) + i.min(255.0) * k) as u8;
                p.0 = [mix(r, ice[0]), mix(g, ice[1]), mix(b, ice[2]), a];
            }
            (img.clone(), Arc::new(out))
        }).1.clone()
    }

    /// This tick's presses joined to any still waiting; held keys and buttons as they are now.
    /// A and D step a lane across; with Shift, W, A, S and D roll that way; W twice, held, runs.
    fn input(&mut self, c: &Canvas, presses: &[Press]) -> Input {
        self.taps(c, presses);
        let run = self.running == Some('w');
        let p = &mut self.pending;
        for press in presses {
            match press {
                Press::Key(k, shift) => {
                    match (way(k), shift) {
                        (Some('a'), false) => p.lane = -1,
                        (Some('d'), false) => p.lane = 1,
                        (Some('a'), true) => p.step = -1,
                        (Some('d'), true) => p.step = 1,
                        (Some('w'), true) => p.tumble = true,
                        (Some('s'), true) => p.backstep = true,
                        _ => {}
                    }
                    if is(k, &[" "], Some(NamedKey::Space)) { p.backstep = true; }
                    if is(k, &["r"], None) { p.flask = true; }
                    if is(k, &["f"], None) { p.loose = true; }
                    if is(k, &["c"], None) { p.cast = true; }
                }
                Press::Mouse(MouseButton::Left, false) => p.light = true,
                Press::Mouse(MouseButton::Left, true) => p.heavy = true,
                Press::Mouse(MouseButton::Right, true) => p.parry = true,
                Press::Mouse(..) => {}
            }
        }
        Input {
            forward: way_held(c, 'w'),
            back: way_held(c, 's'),
            run,
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
                    self.call("PARRIED", [255, 214, 110]);
                    self.flourish = FLOURISH_T;
                }
                // A last-moment block on a bulwark: a beat held, and if the ward goes, a white-gold
                // flash and a heavy shake as it bursts into the attacker.
                Event::PerfectBlock => {
                    c.hitstop(6);
                    self.slow = (18, 0.5);
                    self.call("PERFECT BLOCK", [236, 226, 196]);
                }
                Event::WardBurst { power, .. } => {
                    c.hitstop(12);
                    self.slow = (30, 0.4);
                    c.run(Action::CameraFlash { color: Color(255, 240, 200, (24.0 + 40.0 * power) as u8), duration: 0.16 });
                    c.run(Action::CameraShake { intensity: 30.0 + 30.0 * power, duration: 0.3 });
                    c.run(Action::CameraZoomPunch { amount: 0.05, duration: 0.3 });
                    self.call("WARD BURST", [255, 236, 170]);
                    self.flourish = FLOURISH_T;
                }
                Event::JustDodge { .. } => {
                    c.hitstop(5);
                    self.slow = (36, 0.35);
                    c.run(Action::CameraZoomPunch { amount: 0.03, duration: 0.3 });
                    self.call("PERFECT", [160, 226, 255]);
                }
                Event::StanceBroken { .. } => c.run(Action::CameraZoomPunch { amount: 0.04, duration: 0.25 }),
                Event::Felled { .. } => c.run(Action::CameraZoomPunch { amount: 0.05, duration: 0.3 }),
                // Taken: the harder it is, the longer the freeze, the harder the kick, the redder.
                Event::Struck { heavy: false, .. } => {
                    c.hitstop(3);
                    c.run(Action::CameraShake { intensity: 18.0, duration: 0.12 });
                    c.run(Action::CameraFlash { color: Color(160, 0, 0, 55), duration: 0.12 });
                }
                Event::Struck { heavy: true, .. } => {
                    c.hitstop(7);
                    c.run(Action::CameraShake { intensity: 42.0, duration: 0.26 });
                    c.run(Action::CameraFlash { color: Color(170, 0, 0, 120), duration: 0.26 });
                }
                Event::Staggered { .. } => {
                    c.hitstop(9);
                    c.run(Action::CameraShake { intensity: 50.0, duration: 0.34 });
                    c.run(Action::CameraFlash { color: Color(190, 70, 0, 120), duration: 0.3 });
                    self.call("STAGGERED", [255, 170, 70]);
                }
                Event::KnockedDown { .. } => {
                    c.hitstop(11);
                    c.run(Action::CameraShake { intensity: 72.0, duration: 0.45 });
                    c.run(Action::CameraFlash { color: Color(150, 0, 0, 150), duration: 0.4 });
                    c.run(Action::CameraZoomPunch { amount: 0.05, duration: 0.4 });
                    self.call("KNOCKED DOWN", [235, 70, 60]);
                }
                // Met: a full guard barely moves the camera; one that let some through, more.
                Event::Blocked { chip } if chip <= 0.0 => {
                    c.hitstop(3);
                    c.run(Action::CameraShake { intensity: 12.0, duration: 0.1 });
                    self.call("BLOCKED", [190, 215, 255]);
                }
                Event::Blocked { chip } => {
                    c.hitstop(5);
                    c.run(Action::CameraShake { intensity: 26.0, duration: 0.16 });
                    c.run(Action::CameraFlash { color: Color(255, 140, 40, 45), duration: 0.12 });
                    self.call(&format!("BLOCKED  -{}", chip.round().max(1.0)), [255, 176, 96]);
                }
                Event::Deflected { .. } => { c.hitstop(5); c.run(Action::CameraShake { intensity: 22.0, duration: 0.15 }); }
                Event::GuardBroken { .. } => {
                    c.hitstop(8);
                    c.run(Action::CameraShake { intensity: 44.0, duration: 0.3 });
                    c.run(Action::CameraFlash { color: Color(255, 120, 30, 90), duration: 0.22 });
                    self.call("GUARD BROKEN", [255, 140, 50]);
                }
                Event::ParryMissed { .. } => {
                    c.hitstop(5);
                    c.run(Action::CameraShake { intensity: 30.0, duration: 0.2 });
                    c.run(Action::CameraFlash { color: Color(110, 100, 100, 80), duration: 0.16 });
                    self.call("MISSED PARRY", [200, 150, 150]);
                }
                Event::Died => c.run(Action::CameraFlash { color: Color(60, 0, 0, 200), duration: 1.5 }),
                _ => {}
            }
        }
    }

    /// Show `word` over the hero's head for a moment, in `rgb`.
    fn call(&mut self, word: &str, rgb: [u8; 3]) { self.callout = (word.to_owned(), rgb, CALLOUT_T); }

    /// The hero's weapon as drawn at the height of the blow now being struck (the last live frame
    /// of its weapon's own clip), on the canvas: what a thrust's streak runs along. None outside a
    /// strike, or for a weapon swung in the sword's clip (no hold track to read).
    fn hero_blade(&self, c: &Canvas) -> Option<crate::fx::Blade> {
        let h = &self.fight.hero;
        let a = h.act?;
        let (mv, last) = match a.mv { Move::Light => ("light", 6), Move::Heavy | Move::Riposte => ("heavy", 8), _ => return None };
        let s = self.hero_sheet.as_ref()?;
        let arms = &h.stats.kit.arms;
        let hold = *s.holds.get(&format!("{}_{mv}", arms.class.slug()))?.get(last)?;
        let length = s.blades.get(arms.art)? * hold.reach;
        let pf = c.get_plugin::<PathForgePlugin>()?;
        let (hx, hd) = (h.x + SHOULDER, HERO_D - self.knocked_back());
        let (feet, ppm) = (pf.project_on(c, hx, 0.0, hd)?, pf.scale_on(c, hd)?);
        let ((x, y), (w, _)) = s.place(feet, art::HERO_METRES, ppm);
        let k = w / s.canvas.0 as f32;
        Some(crate::fx::Blade { grip: (x + hold.grip.0 * k, y + hold.grip.1 * k), angle: hold.angle, length: length * k, feet, ppm })
    }

    /// How far the last blow has pushed the hero back toward the camera, metres: out fast and
    /// back over the rest of the reel (down, they stay pushed back until they are up).
    fn knocked_back(&self) -> f32 {
        let h = &self.fight.hero;
        if h.reeling == 0 { return 0.0; }
        let total = h.reel.ticks().max(1) as f32;
        let k = 1.0 - h.reeling as f32 / total;
        let far = match h.reel {
            Reel::None | Reel::Flinch => 0.05, Reel::BlockJolt => 0.08, Reel::Bounce => 0.14, Reel::ChipJolt | Reel::Fumble => 0.22,
            Reel::Recoil => 0.3, Reel::GuardBreak => 0.28, Reel::Stagger => 0.34, Reel::Down => 0.32,
        };
        let shape = if h.reel == Reel::Down {
            let rise = (DOWN_FALL + DOWN_LIE) as f32 / total;
            if k < 0.12 { k / 0.12 } else if k < rise { 1.0 } else { 1.0 - (k - rise) / (1.0 - rise) }
        } else if k < 0.2 { k / 0.2 } else { 1.0 - (k - 0.2) / 0.8 };
        let s = shape.clamp(0.0, 1.0);
        far * s * s * (3.0 - 2.0 * s)
    }

    /// The warnings on the road for every attack under way: where it will land, how it is met;
    /// and at a fork, a trail up the side the hero leans toward.
    fn marks(&self, walked: f64) -> Vec<GroundMark> {
        if let Mode::Side(a) = &self.mode { return Self::side_marks(a); }
        let lean = self.world.signpost.filter(|_| !self.world.chose).zip(self.world.lean).map(|(sign, b)| {
            let side = match b { quartz_path_forge::Branch::Left => -1.0, quartz_path_forge::Branch::Right => 1.0 };
            let to = ((sign - walked) as f32 - world::SIGNPOST_PAST).max(HERO_D + 1.0);
            GroundMark { x: (side * LANE_W - 0.35 * LANE_W, side * LANE_W + 0.35 * LANE_W), d: (HERO_D + 0.8, to), color: [240, 214, 130, 70], fill: Fill::Dots, edge: None }
        });
        self.fight.threats(walked).iter().map(|t| {
            let live = t.phase != Phase::Windup;
            // Brighter as the wind-up runs out. The colour says how to meet it (gold: parry it;
            // red-orange: it breaks guards and cannot be parried, so roll or pay to block; purple:
            // only a roll), and the fill says the same without colour.
            let a = if live { 200 } else { (70.0 + 90.0 * t.progress) as u8 };
            let [r, g, b] = warning_rgb(t.kind, t.parry);
            let color = [r, g, b, a];
            let fill = match t.kind { Kind::Physical => Fill::Solid, Kind::GuardBreak => Fill::Stripes, Kind::Unblockable => Fill::Dots };
            // Short of the lane's edges, so a gap shows which lane is safe.
            let half = 0.4 * LANE_W;
            GroundMark {
                x: (t.lanes.0 as f32 * LANE_W - half, t.lanes.1 as f32 * LANE_W + half),
                d: t.d,
                color,
                fill,
                edge: Some([color[0].saturating_add(40), color[1].saturating_add(40), color[2].saturating_add(40), if live { 255 } else { a.saturating_add(40) }]),
            }
        }).chain(lean).collect()
    }

    /// The runes being cast, as circles on the ground inside their warnings.
    fn sigils(&self, walked: f64) -> Vec<Sigil> {
        if let Mode::Side(a) = &self.mode {
            return a.side.foes.iter().filter_map(|f| {
                let (b, phase, progress) = f.blow()?;
                if b.motion != side::Motion::Rune { return None; }
                let (from, to, ..) = f.warning().unwrap_or((f.s, f.s, b.kind, false, 1.0));
                (from < to).then(|| rune_sigil((-1.1, 1.1), (ARENA_D + from, ARENA_D + to), phase, progress, self.ticks as u32))
            }).collect();
        }
        self.fight.threats(walked).iter().filter(|t| t.area).map(|t| {
            let half = 0.4 * LANE_W;
            rune_sigil((t.lanes.0 as f32 * LANE_W - half, t.lanes.1 as f32 * LANE_W + half), t.d, t.phase, t.progress, self.ticks as u32)
        }).collect()
    }

    fn paint_marks(&mut self, c: &mut Canvas, walked: f64) {
        let marks = self.marks(walked);
        let sigils = self.sigils(walked);
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // The camera moves with every new road frame, so repaint on those too.
        let frame = pf.current_image().cloned();
        let same_frame = match (&frame, &self.last_marks_frame) { (Some(a), Some(b)) => Arc::ptr_eq(a, b), (None, None) => true, _ => false };
        if marks == self.last_marks && sigils == self.last_sigils && (same_frame || (marks.is_empty() && sigils.is_empty())) { return; }
        let img = if marks.is_empty() && sigils.is_empty() { None } else { pf.paint_ground(&marks, &sigils) };
        self.last_marks = marks;
        self.last_sigils = sigils;
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
        // Standing is the first frame of the walk, with what is worn in hand.
        let kit = &self.fight.hero.stats.kit;
        let stand = || ["walk", "light", "guard"].iter().find_map(|n| worn(s, kit, n)).map(|c| c.frames[0].clone());
        self.hero_clip_frame(s, walked).or_else(stand)
    }

    /// Draw the offhand worn onto the hero's clips if it is not yet (`Sheet::dress`, once a pair):
    /// what the loadout holds, and what the hero fighting holds (drawn from), should they differ.
    fn dress_hero(&mut self) {
        let Some(s) = self.hero_sheet.as_mut() else { return };
        for kit in [self.world.gear.kit(), self.fight.hero.stats.kit] {
            if let Some(o) = off_art(&kit) { s.dress(kit.arms.art, o); }
        }
    }

    fn hero_clip_frame(&self, s: &Sheet, walked: f64) -> Option<Arc<RgbaImage>> {
        let h = &self.fight.hero;
        let clip = |n: &str| worn(s, &h.stats.kit, n);
        let k = |t: u32, total: u32| t as f32 / total.max(1) as f32;
        let kneel = |k: f32| clip("rest").map(|c| c.at(k, false).clone());
        let fall = |k: f32| clip("death").map(|c| c.at(k, false).clone());
        match self.mode {
            Mode::Kneel(t) => return kneel(k(t, KNEEL_T)),
            Mode::Rise(t) => return kneel(1.0 - k(t, RISE_T)),
            Mode::Out(Next::Sanctum) | Mode::Cut(After::Sanctum | After::Rise) => return kneel(1.0),
            // Getting up off the ground: the getup where there is one, else the fall played backwards.
            Mode::Wake(t) => return clip("getup").map(|c| c.at(k(t, WAKE_T), false).clone()).or_else(|| fall(1.0 - k(t, WAKE_T))),
            Mode::Out(Next::Respawn | Next::Start) | Mode::Cut(After::Wake) => return fall(1.0),
            // Off into the side path: the north-east walk, mirrored when it leaves on the left.
            Mode::Peek(t, side) => return clip("walk").map(|c| c.at((t as f32 / 36.0).rem_euclid(1.0), side < 0).clone()),
            Mode::Out(Next::Detour(side)) => return clip("walk").map(|c| c.at(((PEEK_T as f32 + 24.0 * self.fade.0) / 36.0).rem_euclid(1.0), side < 0).clone()),
            Mode::Sanctum(_) => {
                let target = self.fight.hero.lane as f32 * BEFORE_X;
                let k = if (target - h.x).abs() > 0.05 { (self.ticks as f32 / 36.0).rem_euclid(1.0) } else { 0.0 };
                return clip("walk").map(|c| c.at(k, self.fight.hero.lane < 0).clone());
            }
            _ => {}
        }
        // Falling takes a second, then they lie there until the road starts again.
        if h.dead { return clip("death").map(|c| c.at(self.dead_for as f32 / 60.0, false).clone()); }
        if let Some(a) = h.act {
            let spec = a.spec;
            let (w, b, r) = spec.frames;
            return match a.mv {
                // The blow's frames, read off the north-east contact sheet: the light's blade is out
                // on 5 and 6, the heavy comes down on 7 and 8, the buckler flashes on 3 to 5.
                Move::Light => held(s, &h.stats.kit, "", "light").map(|c| phased(c, spec.frames, a.t, (5, 7), false)),
                Move::Heavy | Move::Riposte => held(s, &h.stats.kit, "", "heavy").map(|c| phased(c, spec.frames, a.t, (7, 9), false)),
                Move::Parry => clip("parry").map(|c| phased(c, spec.frames, a.t, (3, 6), false)),
                Move::Backstep => clip("backstep").or_else(|| clip("hop")).map(|c| c.at(k(a.t, w + b + r), false).clone()),
                // The roll (side-on, a dip, over the shoulder a full turn, a crouch, back) turns the
                // way it travels: drawn going right, mirrored going left. The turn fills the travel.
                Move::Step(d) => clip("roll_side").or_else(|| clip("roll_spun")).or_else(|| clip("roll"))
                    .map(|c| phased(c, (3, TRAVEL - 3, w + b + r - TRAVEL), a.t, s.roll_keys, d < 0)),
                // A plain step across: a quick walk that way (mirrored going left).
                Move::Lane(d) => clip("walk").map(|c| c.at(k(a.t, w + b + r), d < 0).clone()),
                Move::Tumble => clip("roll").map(|c| phased(c, (3, TRAVEL - 3, w + b + r - TRAVEL), a.t, (2, 8), false)),
                Move::Flask => clip("drink").map(|c| c.at(k(a.t, w + b + r), false).clone()),
                // The draw and the release, or the cast: its own clip, held at full draw until it
                // looses; without one, the other hand raised as for a parry.
                Move::Loose | Move::Cast => loosing(s, &h.stats.kit, "", a.mv == Move::Cast).map(|(c, keys)| phased(c, spec.frames, a.t, keys, false))
                    .or_else(|| clip("parry").map(|c| phased(c, spec.frames, a.t, (3, 6), false))),
            };
        }
        if h.reeling > 0 {
            // Each way of reeling has its own body: flinch, recoil, stagger, floored, a guard
            // jolted, a guard smashed open, a parry fumbled, a blade turned.
            let total = h.reel.ticks().max(1);
            let k = 1.0 - h.reeling as f32 / total as f32;
            let at = |c: &Clip, k: f32| c.at(k.clamp(0.0, 0.999), false).clone();
            // The frames of a clip that tell this, in order, over `k` (Pixellab's clips start from
            // the standing pose and some run on past the part we want).
            let seq = |c: &Clip, frames: &[usize], k: f32| {
                let i = ((k.clamp(0.0, 0.999) * frames.len() as f32) as usize).min(frames.len() - 1);
                c.frames.get(frames[i]).or(c.frames.last()).cloned()
            };
            let hurt = |a: f32, b: f32, k: f32| clip("hurt").map(|c| at(c, a + (b - a) * k));
            return match h.reel {
                Reel::Flinch => hurt(0.0, 0.55, k),
                Reel::Recoil | Reel::None => hurt(0.0, 1.0, k),
                // Arms flung wide, rocking for balance, then caught (before the clip throws its
                // sword and buckler away).
                Reel::Stagger => clip("stagger").and_then(|c| seq(c, &[3, 4, 5, 4, 5, 4, 5, 4, 3, 2, 1], k)).or_else(|| hurt(0.2, 1.0, k)),
                Reel::Down => {
                    let t = total - h.reeling;
                    match (clip("knockdown"), clip("getup")) {
                        // Thrown back and down onto the back; lying there; up off it the way it
                        // fell (getup starts from the knockdown's last pose, see bodies.py).
                        (Some(kd), up) => {
                            if t < DOWN_FALL { seq(kd, &[1, 2, 3, 4, 5, 6, 7, 8], t as f32 / DOWN_FALL as f32) }
                            else if t < DOWN_FALL + DOWN_LIE { kd.frames.last().cloned() }
                            else {
                                let u = (t - DOWN_FALL - DOWN_LIE) as f32 / DOWN_RISE as f32;
                                match up {
                                    Some(g) => seq(g, &[0, 1, 2, 3, 4, 5, 6, 7, 8], u),
                                    None => seq(kd, &[8, 7, 6, 5, 4, 3, 2, 1, 0], u),
                                }
                            }
                        }
                        (None, _) => clip("death").map(|c| at(c, if t < DOWN_FALL + DOWN_LIE { t as f32 / DOWN_FALL as f32 } else { 1.0 - (t - DOWN_FALL - DOWN_LIE) as f32 / DOWN_RISE as f32 })),
                    }
                }
                // The buckler up and jolted back, settling (a chip block the same, pushed further).
                Reel::BlockJolt | Reel::ChipJolt => clip("block_hit").and_then(|c| seq(c, &[4, 5, 5, 4, 3], k)).or_else(|| clip("guard").map(|c| at(c, 1.0))),
                Reel::GuardBreak => clip("guard_break").and_then(|c| seq(c, &[3, 4, 5, 6, 7, 8, 8, 7], k)).or_else(|| hurt(0.0, 1.0, k)),
                // The flick goes out into nothing, then the half-caught blow tells.
                Reel::Fumble => if k < 0.35 { clip("parry").map(|c| at(c, 0.62)) } else { hurt(0.25, 1.0, (k - 0.35) / 0.65) },
                // The blade comes back off the shield: the light's swing, run backwards.
                Reel::Bounce => held(s, &h.stats.kit, "", "light").map(|c| at(c, 0.62 * (1.0 - k))),
            };
        }
        if self.flourish > 0 {
            // The flick out and held, then back in.
            let k = 1.0 - self.flourish as f32 / FLOURISH_T as f32;
            return clip("parry").map(|c| c.at((0.45 + 0.5 * k).min(0.999), false).clone());
        }
        if h.guarding { return clip("guard").map(|c| c.at(1.0, false).clone()); }
        // Walking strides with the road (a stride every 1.6 m); standing is the first frame.
        if self.fight.speed(&self.pending, walked) != 0.0 || self.last_speed != 0.0 {
            if let Some(c) = clip("walk") { return Some(c.looped(walked as f32, 1.6).clone()); }
        }
        None
    }

    /// An enemy's frame now, from the husk's Pixellab animations.
    /// An enemy's frame now, from its kind's Pixellab animations.
    fn enemy_frame(&self, s: &Sheet, e: &Enemy) -> Option<Arc<RgbaImage>> {
        let clip = |n: &str| s.clip(n);
        // (The husk's template shamble dropped the cleaver: an approach plays the idle loop.)
        let first = || ["idle", "chop", "thrust", "bash", "rake"].iter().find_map(|n| clip(n)).map(|c| c.frames[0].clone());
        match e.ai {
            Ai::Attack { i, t, .. } => {
                let a = e.kind.attacks()[i];
                // The wretch's frenzy is one clip of three blows: each blow of the combo plays its third.
                let third = match (e.kind, a.name, clip(a.name)) {
                    (Archetype::Wretch, "frenzy", Some(c)) if c.frames.len() >= 6 => {
                        let k = c.frames.len() / 3;
                        let (from, to) = ((i - 1).min(2) * k, if i >= 3 { c.frames.len() } else { i * k });
                        Some(Clip { frames: c.frames[from..to].to_vec(), mirrored: c.mirrored[from..to].to_vec() })
                    }
                    _ => None,
                };
                third.as_ref().or_else(|| clip(a.art)).map(|c| {
                    // The blow's frames: read off the husk's contact sheet (the chop strikes on 7,
                    // the sweep crosses on 7 to 9); others strike past the middle of their clip.
                    let n = c.frames.len();
                    let keys = match (e.kind, a.art) {
                        (Archetype::Husk, "sweep") => (7, 10),
                        (Archetype::Husk, _) => (7, 9),
                        // Read off their contact sheets: the spear's point comes at the viewer on
                        // 5 to 7, the claw rakes down on 5 to 7.
                        (Archetype::Lancer, "thrust") => (5, 8),
                        (Archetype::Wretch, "rake") => (5, 8),
                        // The axe lands on 7 (its spark).
                        (Archetype::Shieldbearer, "smash") => (6, 8),
                        // At full draw on 6 to 8; the kick's boot is out on 5 and 6.
                        (Archetype::Archer, "loose") => (6, 8),
                        (Archetype::Archer, "kick") => (5, 7),
                        // The hex leaves the palm on 8; the staff comes down on 7 and 8; the
                        // ring spreads on 4 to 7.
                        (Archetype::Hexer, "hex") => (7, 9),
                        (Archetype::Hexer, "rune") => (6, 9),
                        (Archetype::Hexer, "repel") => (4, 7),
                        _ => (n * 5 / 10, (n * 7 / 10).max(n * 5 / 10 + 1)),
                    };
                    phased(c, a.frames, t, keys, false)
                }).or_else(first)
            }
            Ai::Staggered(n) => clip("stagger").map(|c| c.looped((150 - n) as f32, 60.0).clone()).or_else(first),
            // A stumble plays the stagger once, quicker, and holds its last drawing.
            Ai::Stumbled(n) => clip("stagger").map(|c| c.at(((STUMBLE - n) as f32 / 40.0).min(1.0), false).clone()).or_else(first),
            _ if e.hurt > 0 => clip("hurt").map(|c| c.at(1.0 - e.hurt as f32 / 12.0, false).clone()).or_else(first),
            Ai::Dead => clip("death").map(|c| c.frames[c.frames.len() - 1].clone()),
            _ => clip("idle").map(|c| c.looped(self.ticks as f32, 90.0).clone()).or_else(first),
        }
    }

    /// Put the hero and the enemies where the road drawn now says they stand.
    fn place(&self, c: &mut Canvas, walked: f64) {
        if let Mode::Side(a) = &self.mode { return self.place_side(c, a); }
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // A frame lit where it stands, by the road's light there.
        // Each frame drawn, as drawn (where, in what rect, from what picture), for its shadow.
        let seen: RefCell<Vec<(f32, f32, Rect, Arc<RgbaImage>)>> = RefCell::new(Vec::new());
        let lit = |r: Rect, img: Arc<RgbaImage>, x: f32, d: f32| {
            seen.borrow_mut().push((x, d, r, img.clone()));
            match pf.light_at(x, d) {
                Some((p, look)) => self.relit.borrow_mut().light(&img, &p, &look, SPRITE_LIGHT),
                None => img,
            }
        };
        // What stands on the road, for its shadows: (x, d, height, width) in metres.
        let mut stands: Vec<(f32, f32, f32, f32)> = Vec::new();
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
        let (hx, hd) = (h.x + SHOULDER, HERO_D - self.knocked_back());
        // Turning down a side path the hero walks into its mouth while the view turns after them.
        let (hx, hd) = match self.mode {
            Mode::Peek(t, side) => peek_walk((hx, hd), side, t as f32 / PEEK_T as f32),
            Mode::Out(Next::Detour(side)) => peek_walk((hx, hd), side, 1.0 + 0.4 * self.fade.0),
            _ => (hx, hd),
        };
        let hero = match self.hero_sheet.as_ref().and_then(|s| Some((s, self.hero_frame(s, walked)?))) {
            Some((s, img)) => (!h.dead || s.clip("death").is_some()).then(|| sheet_at(s, hx, hd, art::HERO_METRES)).flatten().map(|r| (r, lit(r, img, hx, hd), 60)),
            None => {
                let img = &self.art.hero[pose];
                (!h.dead).then(|| at(hx, HERO_D, art::HERO_METRES, img)).flatten().map(|r| (r, lit(r, img.clone(), hx, HERO_D), 60))
            }
        };
        // Lying down the hero's shadow is as long as they are and low.
        if hero.is_some() { stands.push((hx, hd, if h.floored() { 0.5 } else { art::HERO_METRES }, 0.55)); }
        // The callout rides over the hero's head.
        let head = hero.as_ref().map(|(((x, y), (w, _)), _, _)| (x + 0.5 * w - 900.0, (y - 150.0).max(240.0)));
        let mut placed: Vec<(String, Option<(Rect, Arc<RgbaImage>, i32)>)> = vec![(HERO.to_owned(), hero), (BOSS.to_owned(), None)];
        let (resting, hidden) = (self.at_rest(), !self.road_shown());
        for (i, e) in self.fight.enemies.iter().enumerate().filter(|_| !hidden) {
            let d = e.ahead(walked);
            let pose = match (e.ai, e.attack()) {
                (Ai::Staggered(_) | Ai::Stumbled(_), _) => 5,
                (_, Some((a, Phase::Windup, _))) => if a.wide != 0 { 3 } else { 1 },
                (_, Some((a, _, _))) => if a.wide != 0 { 4 } else { 2 },
                _ => 0,
            };
            let img = if e.hurt > 0 { &self.art.husk_hurt[pose] } else { &self.art.husk[pose] };
            // A hit sets it rocking.
            let sway = if e.hurt > 0 { 0.06 * (e.hurt as f32 * 1.3).sin() } else { 0.0 };
            let x = e.lane as f32 * LANE_W + sway;
            let metres = kind_art(e.kind).1;
            let sheet = |k: Archetype| self.enemy_sheets.iter().find(|(s, _)| *s == k).map(|(_, s)| s);
            let drawn = sheet(e.kind).or_else(|| sheet(Archetype::Husk)).and_then(|s| Some((s, self.enemy_frame(s, e)?)));
            let (spot, img) = match drawn {
                Some((s, frame)) => ((e.alive() && d > 0.6).then(|| sheet_at(s, x, d, metres)).flatten(), frame),
                None => ((e.alive() && d > 0.6).then(|| at(x, d, metres, img)).flatten(), img.clone()),
            };
            let img = if e.chill > 0 && e.alive() { self.frosted(img) } else { img };
            // Nearer draws over further; all of them under the hero, who stands nearest.
            let layer = (50.0 - d * 2.0).round().clamp(1.0, 50.0) as i32;
            if spot.is_some() { stands.push((x, d, metres, 0.3 * metres)); }
            placed.push((enemy_id(i), spot.map(|r| (r, lit(r, img, x, d), layer))));
        }
        // Enemy objects the place leaves empty.
        let drawn = if hidden { 0 } else { self.fight.enemies.len() };
        for i in drawn..world::MOST_FOES { placed.push((enemy_id(i), None)); }
        // Arrows and hexes down the road, at chest height: an arrow foreshortened from its tail to
        // its head as it comes at the hero, a hex round and turning.
        for i in 0..SHOTS {
            let drawn = self.fight.missiles.get(i).filter(|_| !hidden).and_then(|m| {
                let flight = m.attack.shot?;
                let d = (m.s - walked) as f32;
                if d < 0.5 { return None; }
                let rgb = warning_rgb(m.attack.kind, m.attack.parry);
                let layer = (50.0 - d * 2.0).round().clamp(1.0, 50.0) as i32;
                match flight.look {
                    Look::Arrow => {
                        // Seen nearly head-on an arrow is a few pixels: drawn with a long trail
                        // and a wide glow, so it reads as coming down its lane.
                        let (head, tail) = (pf.project_on(c, m.x, 1.35, d)?, pf.project_on(c, m.x, 1.5, d + 2.4)?);
                        let w = 0.42 * pf.scale_on(c, d)?;
                        let len = (head.1 - tail.1).abs().max(0.6 * w);
                        let img = Arc::new(image::imageops::rotate90(&*arrow_picture(rgb)));
                        Some((((0.5 * (head.0 + tail.0) - 0.5 * w, head.1 - len), (w, len)), img, layer))
                    }
                    Look::Hex => {
                        let (px, py) = pf.project_on(c, m.x, 1.25, d)?;
                        let size = 0.75 * pf.scale_on(c, d)?;
                        Some((((px - 0.5 * size, py - 0.5 * size), (size, size)), hex_picture(rgb, m.age), layer))
                    }
                }
            });
            // The hero's own, after the foes': flying away from them up the lane.
            let drawn = drawn.or_else(|| {
                let b = self.fight.bolts.get(i.checked_sub(self.fight.missiles.len())?).filter(|_| !hidden)?;
                let d = (b.s - walked) as f32;
                let layer = (50.0 - d * 2.0).round().clamp(1.0, 59.0) as i32;
                match b.ranged {
                    crate::gear::Ranged::Frost => {
                        let (px, py) = pf.project_on(c, b.x, 1.2, d)?;
                        let size = 0.6 * pf.scale_on(c, d)?;
                        Some((((px - 0.5 * size, py - 0.5 * size), (size, size)), frost_picture(b.age), layer))
                    }
                    r => {
                        let rgb = if r.blood() > 0.0 { [220, 30, 40] } else { [236, 226, 196] };
                        let (head, tail) = (pf.project_on(c, b.x + SHOULDER, 1.35, d)?, pf.project_on(c, b.x + SHOULDER, 1.3, (d - 2.0).max(HERO_D))?);
                        let w = 0.36 * pf.scale_on(c, d)?;
                        let len = (tail.1 - head.1).abs().max(0.6 * w);
                        let img = Arc::new(image::imageops::rotate270(&*arrow_picture(rgb)));
                        Some((((0.5 * (head.0 + tail.0) - 0.5 * w, head.1), (w, len)), img, layer))
                    }
                }
            });
            placed.push((shot_id(i), drawn));
        }
        // The shrine on the right verge, the fork's signpost on the left, the souls on the road.
        let prop = |img: &Arc<RgbaImage>, s: Option<f64>, x: f32, metres: f32, stands: &mut Vec<(f32, f32, f32, f32)>| {
            let d = (s? - walked) as f32;
            let layer = (50.0 - d * 2.0).round().clamp(1.0, 50.0) as i32;
            let r = (d > 0.8).then(|| at(x, d, metres, img)).flatten()?;
            stands.push((x, d, metres, 0.35 * metres));
            Some((r, lit(r, img.clone(), x, d), layer))
        };
        let (shrine, signpost) = match &self.props { Some((a, b)) => (Some(a), Some(b)), None => (None, None) };
        // At rest the shrine stands in the middle of the clearing, the keeper to its left and the
        // smith (once met) to its right; on the road the shrine is on the right verge and the
        // smith, until met, waits on the left.
        let station = |d: f32| (walked + d as f64, 50 - (d * 2.0) as i32);
        let npc = |sheet: &Option<Sheet>, x: f32, d: f32, period: f32, stands: &mut Vec<(f32, f32, f32, f32)>| {
            let s = sheet.as_ref()?;
            let img = s.clip("idle")?.looped(self.ticks as f32, period).clone();
            let layer = (50.0 - d * 2.0).round().clamp(1.0, 50.0) as i32;
            let r = (d > 0.8).then(|| sheet_at(s, x, d, 1.85)).flatten()?;
            stands.push((x, d, 1.85, 0.6));
            Some((r, lit(r, img, x, d), layer))
        };
        if resting {
            let (at, _) = station(STATION_D + 0.4);
            placed.push((SHRINE.to_owned(), shrine.and_then(|img| prop(img, Some(at), 0.0, 2.4, &mut stands))));
            placed.push((KEEPER.to_owned(), npc(&self.keeper_sheet, -STATION_X, STATION_D, 80.0, &mut stands)));
            placed.push((SMITH.to_owned(), self.world.met_smith.then(|| npc(&self.smith_sheet, STATION_X, STATION_D, 70.0, &mut stands)).flatten()));
        } else {
            placed.push((SHRINE.to_owned(), shrine.and_then(|img| prop(img, self.world.shrine(), 3.4 * self.world.side(), 1.9, &mut stands))));
            placed.push((KEEPER.to_owned(), None));
            let smith = self.world.smith_at().and_then(|s| npc(&self.smith_sheet, -3.0 * self.world.side(), (s - walked) as f32, 70.0, &mut stands));
            placed.push((SMITH.to_owned(), smith));
        }
        // On the island where the branches part.
        placed.push((SIGNPOST.to_owned(), signpost.filter(|_| !resting).and_then(|img| prop(img, self.world.signpost, self.world.signpost_x(walked), 2.4, &mut stands))));
        let stain = self.world.stain_at().filter(|_| !resting).and_then(|s| {
            let d = (s - walked) as f32;
            let (fx, fy) = pf.project_on(c, 0.0, 0.0, d)?;
            let w = 0.6 * pf.scale_on(c, d)?;
            (d > 0.8).then(|| (((fx - 0.5 * w, fy - 0.2 * w), (w, 0.2 * w)), self.art.stain.clone(), 2))
        });
        placed.push((STAIN.to_owned(), stain));
        // The place's cache: a chest at its metre, beside the path, shut until it is taken.
        let side = self.world.side();
        let chest = self.world.cache_at().filter(|_| !resting).zip(self.chest.as_ref()).and_then(|(at, (shut, open))| {
            let img = if self.world.cache_taken() { open } else { shut };
            prop(img, Some(at), 1.1 * side, 0.95, &mut stands)
        });
        placed.push((CHEST.to_owned(), chest));
        self.relit.borrow_mut().trim();
        let shadow_img = self.shadows(c, pf, &stands, &seen.into_inner());
        Self::show_shadows(c, shadow_img);
        if let (Some(p), Some(o)) = (head, c.get_game_object_mut(CALLOUT)) { o.position = p; }
        Self::put(c, placed);
    }

    /// Show each object where it was placed (hidden where it was not).
    fn put(c: &mut Canvas, placed: Vec<(String, Option<(Rect, Arc<RgbaImage>, i32)>)>) {
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

    /// The shadows of what stands on the road, (x, d, height, width) metres: a figure drawn from a
    /// picture (`seen`: where, in what rect, from what picture) casts its outline, traced from the
    /// picture's pixels; the rest a soft column. None when nothing has moved since the last.
    fn shadows(&self, c: &Canvas, pf: &PathForgePlugin, stands: &[(f32, f32, f32, f32)], seen: &[(f32, f32, Rect, Arc<RgbaImage>)]) -> Option<Option<Arc<RgbaImage>>> {
        let (mut shadows, mut casts) = (Vec::new(), Vec::new());
        let mut tracer = self.tracer.borrow_mut();
        for &(x, d, ht, w) in stands {
            let drawn = seen.iter().find(|s| s.0.to_bits() == x.to_bits() && s.1.to_bits() == d.to_bits());
            let outline = drawn.and_then(|(_, _, ((left, top), (rw, rh)), img)| {
                let (iw, ih) = (img.width() as f32, img.height() as f32);
                let name = format!("{:p}", Arc::as_ptr(img));
                if !tracer.ensure_outline_for_image(&name, img, (iw, ih)) { return None; }
                let sil = tracer.dynamic_outline_world_silhouette(&name, (0.0, 0.0), 0.0)?;
                let (fx, fy) = pf.project_on(c, x, 0.0, d)?;
                let ppm = pf.scale_on(c, d)?;
                Some(sil.iter().map(|&(px, py)| ((left + px / iw * rw - fx) / ppm, (fy - (top + py / ih * rh)) / ppm)).collect::<Vec<_>>())
            });
            match outline {
                Some(o) => {
                    shadows.push(pf.contact_shadow(x, d, w));
                    casts.extend(pf.cast_shadow(x, d, o));
                }
                None => shadows.extend(pf.shadows_of(x, d, ht, w)),
            }
        }
        let frame = pf.current_image().cloned();
        let mut last = self.last_shadows.borrow_mut();
        let same_frame = match (&frame, &last.2) { (Some(a), Some(b)) => Arc::ptr_eq(a, b), (None, None) => true, _ => false };
        if shadows == last.0 && casts == last.1 && (same_frame || shadows.is_empty()) { return None; }
        let img = if shadows.is_empty() && casts.is_empty() { None } else { pf.paint_shadows(&shadows, &casts).map(Arc::new) };
        *last = (shadows, casts, frame);
        Some(img)
    }

    fn show_shadows(c: &mut Canvas, img: Option<Option<Arc<RgbaImage>>>) {
        let (Some(img), Some(o)) = (img, c.get_game_object_mut(SHADOWS)) else { return };
        match img {
            Some(img) => {
                set_pixel_art(&img, true);
                o.visible = true;
                let size = o.size;
                o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None });
            }
            None => o.visible = false,
        }
    }

    /// The side-on fight's hero and boss where the turned camera sees them; everything else of
    /// the road's hidden.
    fn place_side(&self, c: &mut Canvas, a: &Arena) {
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        let dir = screen_dir(pf);
        let seen: RefCell<Vec<(f32, f32, Rect, Arc<RgbaImage>)>> = RefCell::new(Vec::new());
        let lit = |r: Rect, img: Arc<RgbaImage>, x: f32, d: f32| {
            seen.borrow_mut().push((x, d, r, img.clone()));
            match pf.light_at(x, d) {
                Some((p, look)) => self.relit.borrow_mut().light(&img, &p, &look, SPRITE_LIGHT),
                None => img,
            }
        };
        let sheet_at = |s: &Sheet, d: f32, y: f32, metres: f32| -> Option<Rect> {
            Some(s.place(pf.project_on(c, 0.0, y, d)?, metres, pf.scale_on(c, d)?))
        };
        let mut stands = Vec::new();
        let h = &a.side.hero;
        // While the camera swings round the hero walks from where they stood into the yard.
        let k = (a.t as f32 / ARENA_IN as f32).min(1.0);
        let k = k * k * (3.0 - 2.0 * k);
        let hd = ARENA_D + if a.t < ARENA_IN { (HERO_D - ARENA_D) + (h.s - (HERO_D - ARENA_D)) * k } else { h.s };
        let hero = self.hero_sheet.as_ref().and_then(|s| {
            let img = if a.t < ARENA_IN && k < 0.95 { worn(s, &h.stats.kit, "e_walk").map(|c| c.at((hd / 1.6).rem_euclid(1.0), dir < 0.0).clone())? } else { self.side_hero_frame(s, h, dir)? };
            let r = sheet_at(s, hd, h.y, art::HERO_METRES)?;
            Some((r, lit(r, img, 0.0, hd), 60))
        });
        if hero.is_some() { stands.push((0.0, hd, art::HERO_METRES, 0.55)); }
        let head = hero.as_ref().map(|(((x, y), (w, _)), _, _)| (x + 0.5 * w - 900.0, (y - 150.0).max(240.0)));
        // The foes step out once the camera has come round (before, they would stand in its way):
        // the Warden as the boss, an ambush's as the road's enemies.
        let foe = |f: &side::Foe, sheet: Option<&Sheet>, metres: f32, layer: i32, stands: &mut Vec<(f32, f32, f32, f32)>| {
            let s = sheet.filter(|_| k > 0.8)?;
            let fd = ARENA_D + f.s;
            let img = self.foe_frame(s, f, h.s, dir)?;
            let img = if f.chill > 0 && f.alive() { self.frosted(img) } else { img };
            let r = sheet_at(s, fd, f.lift(), metres)?;
            stands.push((0.0, fd, metres, 0.45 * metres));
            Some((r, lit(r, img, 0.0, fd), layer))
        };
        // A boss or mini-boss is drawn from its own art; while that is missing, a mini-boss from its
        // kind's, a head taller, and a boss from the Warden's.
        let boss = (a.why == Why::Boss).then(|| a.side.foes.first().and_then(|f| {
            let own = |folder: &str| self.boss_art.as_ref().filter(|(x, _)| *x == folder).and_then(|(_, s)| s.as_ref());
            let (sheet, metres) = match (f.breed.sprite.and_then(|(folder, m)| Some((own(folder)?, m))), f.breed.kind) {
                (Some((s, m)), _) => (Some(s), m),
                (None, Some(kind)) => (self.enemy_sheets.iter().find(|(s, _)| *s == kind).map(|(_, s)| s), kind_art(kind).1 * 1.2),
                (None, None) => (self.boss_sheet.as_ref(), BOSS_METRES),
            };
            foe(f, sheet, metres, 40, &mut stands)
        })).flatten();
        let mut placed: Vec<(String, Option<(Rect, Arc<RgbaImage>, i32)>)> = vec![(HERO.to_owned(), hero), (BOSS.to_owned(), boss)];
        for i in 0..world::MOST_FOES {
            let drawn = match (a.why, a.side.foes.get(i)) {
                (Why::Ambush(_), Some(f)) => {
                    let kind = f.breed.kind.unwrap_or(Archetype::Husk);
                    let sheet = self.enemy_sheets.iter().find(|(s, _)| *s == kind).map(|(_, s)| s);
                    foe(f, sheet, kind_art(kind).1, 40 - i as i32, &mut stands)
                }
                _ => None,
            };
            placed.push((enemy_id(i), drawn));
        }
        for id in [SHRINE, SIGNPOST, STAIN, KEEPER, SMITH, CHEST] { placed.push((id.to_owned(), None)); }
        // Shots in flight, at the middle of the height they fly at, pointing the way they go: an
        // archer's arrow, a hexer's hex, anyone else's streak.
        for i in 0..SHOTS {
            let drawn = a.side.foes.get(i).and_then(|f| {
                let (at, (b, _, _)) = (f.shot()?, f.blow()?);
                let (d, y) = (ARENA_D + at, 0.5 * (b.height.0 + b.height.1));
                let (px, py) = pf.project_on(c, 0.0, y, d)?;
                let m = pf.scale_on(c, d)?;
                let rgb = warning_rgb(b.kind, b.parry);
                let (size, mut img) = match f.breed.kind {
                    Some(Archetype::Archer) => ((0.9 * m, 0.2 * m), arrow_picture(rgb)),
                    Some(Archetype::Hexer) => ((0.5 * m, 0.5 * m), hex_picture(rgb, self.ticks as u32)),
                    _ => ((0.8 * m, 0.28 * m), shot_picture(rgb)),
                };
                if f.facing * dir < 0.0 { img = Arc::new(image::imageops::flip_horizontal(&*img)); }
                Some((((px - 0.5 * size.0, py - 0.5 * size.1), size), img, 45))
            });
            // The hero's own, in the slots past the foes'.
            let drawn = drawn.or_else(|| {
                let b = a.side.bolts.get(i.checked_sub(world::MOST_FOES)?)?;
                let d = ARENA_D + b.s;
                let (px, py) = pf.project_on(c, 0.0, b.y, d)?;
                let m = pf.scale_on(c, d)?;
                let (size, mut img) = match b.ranged {
                    crate::gear::Ranged::Frost => ((0.55 * m, 0.55 * m), frost_picture(b.age)),
                    crate::gear::Ranged::BloodBow => ((0.9 * m, 0.2 * m), arrow_picture([220, 30, 40])),
                    crate::gear::Ranged::BloodCrossbow => ((0.6 * m, 0.22 * m), arrow_picture([200, 20, 30])),
                    crate::gear::Ranged::Bow => ((0.9 * m, 0.2 * m), arrow_picture([236, 226, 196])),
                };
                if b.dir * dir < 0.0 { img = Arc::new(image::imageops::flip_horizontal(&*img)); }
                Some((((px - 0.5 * size.0, py - 0.5 * size.1), size), img, 46))
            });
            placed.push((shot_id(i), drawn));
        }
        self.relit.borrow_mut().trim();
        let shadow_img = self.shadows(c, pf, &stands, &seen.into_inner());
        Self::show_shadows(c, shadow_img);
        if let (Some(p), Some(o)) = (head, c.get_game_object_mut(CALLOUT)) { o.position = p; }
        Self::put(c, placed);
    }

    /// The hero's side-on frame (the clips face right; mirrored when they face left on screen).
    fn side_hero_frame(&self, s: &Sheet, h: &side::SideHero, dir: f32) -> Option<Arc<RgbaImage>> {
        let mirror = h.facing * dir < 0.0;
        let at = |n: &str, k: f32| worn(s, &h.stats.kit, n).map(|c| c.at(k.clamp(0.0, 0.999), mirror).clone());
        let total = |f: (u32, u32, u32)| (f.0 + f.1 + f.2).max(1) as f32;
        use side::Act;
        let class = h.stats.kit.arms.class;
        if h.dead { return at("e_death", 1.0); }
        if h.reel == Reel::Down {
            // Down over the death's first frames, lying on its last, up again the same way back.
            let t = Reel::Down.ticks() - h.reeling;
            let k = if t < DOWN_FALL { 0.9 * t as f32 / DOWN_FALL as f32 } else if t < DOWN_FALL + DOWN_LIE { 0.9 } else { 0.9 * (1.0 - (t - DOWN_FALL - DOWN_LIE) as f32 / DOWN_RISE as f32) };
            return at("e_death", k);
        }
        if h.reel != Reel::None { return at("e_hurt", 1.0 - h.reeling as f32 / h.reel.ticks().max(1) as f32); }
        match h.act {
            Some((Act::Roll(_), t)) => at("e_roll", t as f32 / side::ROLL_T as f32),
            Some((Act::Light, t)) => held(s, &h.stats.kit, "e_", "light").map(|c| phased(c, Act::Light.frames(&h.stats.kit), t, (3, 5), mirror)),
            Some((Act::Heavy, t)) => held(s, &h.stats.kit, "e_", "heavy").map(|c| phased(c, Act::Heavy.frames(&h.stats.kit), t, (5, 7), mirror)),
            Some((Act::Parry, t)) => at("e_parry", t as f32 / total(Act::Parry.frames(&h.stats.kit))),
            Some((Act::Flask, _)) => at("e_guard", 0.4),
            // Its own draw or cast where there is one; else the other hand raised as for a parry.
            Some((a @ (Act::Loose | Act::Cast), t)) => loosing(s, &h.stats.kit, "e_", a == Act::Cast).map(|(c, keys)| phased(c, a.frames(&h.stats.kit), t, keys, mirror))
                .or_else(|| at("e_parry", (t as f32 / total(a.frames(&h.stats.kit))).min(0.45))),
            // Rising (sprung, knees coming up), tucked at the top, legs down for the landing.
            None if h.airborne() => at("e_jump", if h.y > 0.5 { 0.6 } else if h.rising() { 0.45 } else { 0.75 }),
            None if h.guarding => at("e_guard", 1.0),
            None if h.walking => at("e_walk", (h.s / 1.6).rem_euclid(1.0)),
            None => at("e_idle", (self.ticks as f32 / 90.0).fract()),
        }
    }

    /// A side-on foe's frame: its blows phased to their ticks, open, walking, standing, falling.
    /// Its clips face left; an ambusher's side-on ones are its road sheet's `w_` clips (until it
    /// has them, its road drawing stands in).
    fn foe_frame(&self, s: &Sheet, f: &side::Foe, hero_s: f32, dir: f32) -> Option<Arc<RgbaImage>> {
        let mirror = f.facing * dir > 0.0;
        let pre = if f.breed.kind.is_some() { "w_" } else { "" };
        let clip = |n: &str| s.clip(&format!("{pre}{n}")).or_else(|| s.clip(&format!("{pre}walk"))).or_else(|| s.clip("idle"));
        let at = |n: &str, k: f32| clip(n).map(|c| c.at(k.clamp(0.0, 0.999), mirror).clone());
        match f.ai {
            // Without a fall of its own to play, a felled foe is gone.
            side::Ai::Dead => s.clip(&format!("{pre}death")).map(|c| c.at((f.down as f32 / 60.0).min(0.999), mirror).clone()),
            side::Ai::Open(n) => at("stagger", ((f.breed.open - n.min(f.breed.open)) as f32 / 50.0).fract()),
            side::Ai::Blow { i, t, .. } => {
                let blow = f.timed(i);
                clip(blow.art).map(|c| { let n = c.frames.len(); phased(c, blow.frames, t, (n * 5 / 10, n * 7 / 10), mirror) })
            }
            side::Ai::Wait(_) if (hero_s - f.s).abs() > f.breed.keep + 0.05 => at("walk", (f.s / 1.8).rem_euclid(1.0)),
            // Standing: its idle, or the first of its walk.
            side::Ai::Wait(_) => if s.clip(&format!("{pre}idle")).is_some() { at("idle", (self.ticks as f32 / 100.0).fract()) } else { at("walk", 0.0) },
        }
    }

    /// The side-on fight's warnings on the ground, one for each foe winding up or live.
    fn side_marks(a: &Arena) -> Vec<GroundMark> {
        a.side.foes.iter().filter_map(|f| f.warning()).map(|(from, to, kind, parry, progress)| {
            let live = progress >= 1.0;
            let al = if live { 200 } else { (70.0 + 90.0 * progress) as u8 };
            let [r, g, b] = warning_rgb(kind, parry);
            let fill = match kind { Kind::Physical => Fill::Solid, Kind::GuardBreak => Fill::Stripes, Kind::Unblockable => Fill::Dots };
            GroundMark { x: (-1.1, 1.1), d: (ARENA_D + from, ARENA_D + to), color: [r, g, b, al], fill, edge: Some([r.saturating_add(40), g.saturating_add(40), b.saturating_add(40), al.saturating_add(40)]) }
        }).collect()
    }

    /// This tick's presses for the side-on fight, joined to any waiting; held keys as they are.
    /// A and D are left and right on screen, whichever way the road runs across it.
    fn side_input(&mut self, c: &Canvas, presses: &[Press]) -> SideInput {
        self.taps(c, presses);
        let run = matches!(self.running, Some('a' | 'd'));
        let dir = c.get_plugin::<PathForgePlugin>().map_or(1.0, screen_dir);
        let held = |ch: &str, n: NamedKey| c.is_key_held(&Key::Character(ch.into())) || c.is_key_held(&Key::Named(n));
        let (l, r) = (held("a", NamedKey::ArrowLeft), held("d", NamedKey::ArrowRight));
        let (left, right) = if dir > 0.0 { (l, r) } else { (r, l) };
        let p = &mut self.side_pending;
        for press in presses {
            match press {
                Press::Key(k, shift) => {
                    // Shift with A or D rolls that way; Space rolls the way held, or faced.
                    if *shift && matches!(way(k), Some('a' | 'd')) { p.roll = true; }
                    if is(k, &[" "], Some(NamedKey::Space)) { p.roll = true; }
                    if is(k, &["w"], Some(NamedKey::ArrowUp)) { p.jump = true; }
                    if is(k, &["r"], None) { p.flask = true; }
                    if is(k, &["f"], None) { p.loose = true; }
                    if is(k, &["c"], None) { p.cast = true; }
                }
                Press::Mouse(MouseButton::Left, false) => p.light = true,
                Press::Mouse(MouseButton::Left, true) => p.heavy = true,
                Press::Mouse(MouseButton::Right, true) => p.parry = true,
                Press::Mouse(..) => {}
            }
        }
        SideInput { left, right, run, guard: c.is_mouse_held(MouseButton::Right) && !shift_held(c), ..*p }
    }

    fn swing(c: &mut Canvas, spec: &str) {
        if let Some(pf) = c.get_plugin_mut::<PathForgePlugin>() { pf.swing(spec); }
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
        let shown = !matches!(self.mode, Mode::Front(_) | Mode::Out(Next::Start));
        let mut set = |id: &str, k: Option<f32>| set(id, k.filter(|_| shown));
        // The backs are shown or hidden with their bars (full width when shown).
        set("bar_hp_back", Some(1.0));
        set("bar_st_back", Some(1.0));
        set("bar_hp", Some(h.hp / h.stats.max_hp()));
        set("bar_st", Some(h.stamina / h.stats.max_stamina()));
        // The ward, only with a bulwark in hand; the yard's own while in one.
        let ward = match &self.mode { Mode::Side(a) => a.side.hero.bulwark.ward, _ => h.bulwark.ward };
        let bulwark = h.stats.kit.ward > 0.0;
        let ready = ward >= crate::fight::WARD_BURST;
        set("bar_ward_back", bulwark.then_some(1.0));
        set("bar_ward", (bulwark && !ready).then_some(ward / crate::fight::WARD_MAX));
        set("bar_ward_ready", (bulwark && ready).then_some(ward / crate::fight::WARD_MAX));
        if let Mode::Side(a) = &self.mode {
            // The boss; in an ambush, the nearest still standing.
            let b = a.side.nearest().map(|i| &a.side.foes[i]);
            set("bar_foe", b.map(|b| b.hp / b.max_hp()));
            set("bar_foe_stance", b.map(|b| if matches!(b.ai, side::Ai::Open(_)) { 0.0 } else { b.stance / b.max_stance() }));
            return;
        }
        set("bar_foe", foe.map(|e| e.hp / e.top));
        set("bar_foe_stance", foe.map(|e| if matches!(e.ai, Ai::Staggered(_)) { 0.0 } else { e.stance / e.kind.stance() }));
    }

    fn hud(&mut self, c: &mut Canvas, walked: f64) {
        let Some(font) = self.font.clone() else { return };
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        let h = &self.fight.hero;
        let w = &self.world;
        let calling = self.calling.map_or("", |c| c.name());
        let line = match self.mode {
            Mode::Front(_) => String::new(),
            Mode::Sanctum(_) => format!(
                "At rest  |  {calling}  level {}  |  {} {}  |  shards {}  |  weapon +{}  armour +{}",
                h.stats.level(), world::CURRENCY, w.souls, w.shards, h.stats.weapon, h.stats.armour,
            ),
            _ => format!(
                "{}  |  {} {}  shards {}  |  level {}  |  flasks {}{}  |  {} {:.1} ms  road {} fps  {:.0} m{}",
                w.here().title, world::CURRENCY, w.souls, w.shards, h.stats.level(), h.flasks,
                {
                    let hand = match h.stats.kit.ranged {
                        Some(crate::gear::Ranged::Bow) => format!("  arrows {}", h.arrows),
                        Some(r @ (crate::gear::Ranged::BloodBow | crate::gear::Ranged::BloodCrossbow)) => format!("  shots {} health", r.blood()),
                        _ => String::new(),
                    };
                    let spell = match h.stats.kit.spell_cast() {
                        Some(sp) if !h.stats.spell_ready() => format!("  {} needs focus {}", sp.name(), sp.needs()[2]),
                        Some(_) => format!("  spell {}/{}", h.charges.floor(), h.stats.max_charges()),
                        None => String::new(),
                    };
                    hand + &spell
                },
                pf.engine().to_uppercase(), pf.render_ms(), self.frames.1, walked,
                if w.can_rest(&self.fight, walked) { "  |  E rest at the shrine" } else if w.can_meet_smith(&self.fight, walked) { "  |  E speak with the smith" }
                    else if self.mode == Mode::Play && w.side_path_at(&self.fight, walked, &pf.state().side_paths).is_some() { "  |  E turn down the side path" }
                    else if w.can_come_back(&self.fight, walked) { "  |  E go back to the road" } else { "" },
            ),
        };
        // Text is laid out in window pixels: sized by the window's fit, like the box it sits in.
        let s = c.base_scale();
        let text = Text::new(vec![Span::new(line, 52.0 * s, Some(64.0 * s), font.clone(), Color(236, 228, 210, 255), 0.0)], Some(3600.0 * s), Align::Left, None);
        if let Some(o) = c.get_game_object_mut(HUD) { o.set_drawable(Box::new(text)); }
        let boss = matches!(self.mode, Mode::Side(_));
        let help = if let Mode::Side(a) = &self.mode { if a.why == Why::Boss { BOSS_HELP } else { AMBUSH_HELP } } else { "W/S walk   W W (held) run   A/D change lane   Shift+W/A/S/D roll   LMB strike   Shift+LMB heavy   RMB guard   Shift+RMB parry   R drink   F ranged attack   C cast   E rest   T turn back   M map      warnings: gold parry  ·  orange roll or block  ·  purple roll" };
        let help = Text::new(vec![Span::new(help.into(), 40.0 * s, Some(50.0 * s), font.clone(), Color(236, 228, 210, 190), 0.0)], Some(3720.0 * s), Align::Left, None);
        if let Some(o) = c.get_game_object_mut(HELP) { o.visible = self.mode == Mode::Play || boss; o.set_drawable(Box::new(help)); }
        // The banner, fading out over its last second.
        let (line, left) = &self.banner;
        let a = (*left as f32 / 60.0).min(1.0);
        let banner = Text::new(vec![Span::new(line.clone(), 110.0 * s, Some(130.0 * s), font, Color(240, 226, 190, (255.0 * a) as u8), 0.0)], Some(3840.0 * s), Align::Center, None);
        if let Some(o) = c.get_game_object_mut(BANNER) { o.visible = *left > 0; o.set_drawable(Box::new(banner)); }
    }

    /// The callout, while it lasts: popping in a little large, fading over its last third.
    fn draw_callout(&mut self, c: &mut Canvas) {
        let (word, rgb, left) = self.callout.clone();
        if left == 0 {
            if let Some(o) = c.get_game_object_mut(CALLOUT) { o.visible = false; }
            return;
        }
        self.callout.2 -= 1;
        let Some(font) = self.font.clone() else { return };
        let s = c.base_scale();
        let age = CALLOUT_T - left;
        let size = 76.0 * (1.0 + 0.25 * (1.0 - (age as f32 / 6.0).min(1.0)));
        let a = (left as f32 / (CALLOUT_T as f32 / 3.0)).min(1.0);
        let text = Text::new(vec![Span::new(word, size * s, Some(size * 1.15 * s), font, Color(rgb[0], rgb[1], rgb[2], (255.0 * a) as u8), 0.0)], Some(1800.0 * s), Align::Center, None);
        if let Some(o) = c.get_game_object_mut(CALLOUT) { o.visible = true; o.set_drawable(Box::new(text)); }
    }

    fn say(&mut self, notes: &[Note]) {
        for n in notes {
            let line = match n {
                Note::Arrived(t) => t.to_string(),
                Note::ShrineFound => continue,
                Note::Rested => "Rested at the shrine. The road's dead walk again.".into(),
                Note::Rose(l) => format!("Level {l}"),
                Note::Region(name, false) => format!("{name}"),
                Note::Region(name, true) => format!("{name}  ·  a foreboding air: the dead here are far beyond you"),
                Note::Fork(l, r) => format!("The road forks. Step left for {l}, right for {r}."),
                Note::Lean(quartz_path_forge::Branch::Left, to) => format!("Left: {to}"),
                Note::Lean(quartz_path_forge::Branch::Right, to) => format!("Right: {to}"),
                Note::Barred(to, Some(k)) => format!("The way to {to} is barred. It wants the {}.", k.name()),
                Note::Barred(to, None) => format!("The way to {to} will not open from this side."),
                Note::Opened(to) => format!("A shortcut opens back to {to}"),
                Note::Turned => "You turn back.".into(),
                Note::Recovered(n) => format!("{n} {} recovered", world::CURRENCY),
                Note::Forged(what, n) => format!("{what} forged to +{n}"),
                Note::Met(who) => format!("{who} will wait at the shrines"),
                Note::Found(what) => format!("Found: {what}"),
                Note::Wore(what) => format!("Wearing {what}"),
                Note::Crafted(what) => format!("The smith forged the {what}"),
                Note::Vanquished(who, souls) => format!("{who} falls  ·  {souls} {}", world::CURRENCY),
                Note::Ambushed(false) => "Ambush!".into(),
                Note::Ambushed(true) => "The chest was a trap!".into(),
                Note::Pilgrimage(n) => format!("The Archangel falls. The pilgrimage begins again ({n}), and the dead are stronger."),
            };
            self.banner = (line, 240);
        }
    }

    /// Whether the road shown is the sanctum's (no enemies, no road props).
    fn at_rest(&self) -> bool { matches!(self.mode, Mode::Sanctum(_) | Mode::Cut(After::Sanctum) | Mode::Out(Next::Road)) }
    /// Whether the road's own things (enemies, bars) are shown: not at rest, not on the title.
    fn road_shown(&self) -> bool { !self.at_rest() && !matches!(self.mode, Mode::Front(_) | Mode::Out(Next::Start)) }

    /// Whether the hero is on the road with the fight running (the capture waits for it).
    pub(crate) fn playing(&self) -> bool { self.mode == Mode::Play }

    /// The black eases toward where it is headed.
    fn fade_step(&mut self, c: &mut Canvas) {
        let (now, to) = self.fade;
        if now == to { return; }
        let next = if to > now { (now + FADE_STEP).min(to) } else { (now - FADE_STEP).max(to) };
        self.fade.0 = next;
        if let Some(o) = c.get_game_object_mut(FADE) {
            o.visible = next > 0.0;
            o.set_image(Image { shape: ShapeType::Rectangle(0.0, (3840.0, 2160.0), 0.0), image: swatch([0, 0, 0, (255.0 * next) as u8]), color: None });
        }
    }

    /// The words and choices to show now, and how.
    fn panel(&self) -> Option<(Panel, Style, f32)> {
        match &self.mode {
            Mode::Front(f) => Some((f.panel(&self.slots), if matches!(f, Front::Pause(_)) { Style::Menu } else { Style::Title }, 1.0)),
            Mode::Sanctum(s) => Some((s.panel(&self.world, &self.fight), Style::Menu, 1.0)),
            Mode::Meet(m) => Some((m.panel(), Style::Menu, 1.0)),
            Mode::Map => Some((Panel { title: "The Shattered Realms".into(), text: crate::map::legend(&self.world), hint: "M or Esc close".into(), ..Panel::default() }, Style::Menu, 1.0)),
            // The fall first; then the words come up out of the dark and stay until it is black.
            Mode::Dying(t) if *t > 40 => Some((Panel { title: "YOU DIED".into(), ..Panel::default() }, Style::Died, ((*t - 40) as f32 / 50.0).min(1.0))),
            Mode::Out(Next::Respawn) => Some((Panel { title: "YOU DIED".into(), ..Panel::default() }, Style::Died, 1.0 - self.fade.0)),
            _ => None,
        }
    }

    fn draw_panel(&self, c: &mut Canvas) {
        let Some(font) = self.font.clone() else { return };
        let s = c.base_scale();
        let Some((p, style, a)) = self.panel() else {
            for id in [PANEL, PANEL_BG] { if let Some(o) = c.get_game_object_mut(id) { o.visible = false; } }
            return;
        };
        let al = |v: f32| (v * a).round().clamp(0.0, 255.0) as u8;
        let gold = Color(232, 200, 120, al(255.0));
        let cream = Color(236, 228, 210, al(255.0));
        let dim = Color(236, 228, 210, al(150.0));
        let span = |t: String, size: f32, col: Color| Span::new(t, size * s, Some(size * 1.25 * s), font.clone(), col, 0.0);
        // (box position and size, its darkness, text position and size, alignment, title size and colour)
        // A menu stands on the side away from who is spoken to (the keeper is on the left, the
        // smith on the right, by the road and at rest), as tall as its lines.
        let left = matches!(&self.mode, Mode::Sanctum(s) if s.station() == shrine::Station::Smith) || self.mode == Mode::Map;
        let (mw, pad) = (1560.0, 60.0);
        let wrap = |t: &str, size: f32| (t.len() as f32 * size * 0.5 / (mw - 2.0 * pad)).ceil();
        let lines = 66.0 * 1.25 + wrap(&p.text, 52.0) * 52.0 * 1.25 + if p.options.is_empty() { 0.0 } else { 30.0 * 1.25 + p.options.len() as f32 * 54.0 * 1.25 } + 2.0 * 40.0 * 1.25;
        let mh = lines + 2.0 * pad;
        let mx = if left { 120.0 } else { 3840.0 - 120.0 - mw };
        let my = (1960.0 - mh).min(980.0);
        let (bg, bg_a, at, size, align, title) = match style {
            Style::Title => (((0.0, 0.0), (3840.0, 2160.0)), 165.0, (0.0, 520.0), (3840.0, 1400.0), Align::Center, (190.0, gold)),
            Style::Menu => (((mx, my), (mw, mh)), 215.0, (mx + pad, my + pad), (mw - 2.0 * pad, mh - 2.0 * pad), Align::Left, (66.0, gold)),
            Style::Died => (((0.0, 800.0), (3840.0, 480.0)), 215.0, (0.0, 870.0), (3840.0, 380.0), Align::Center, (220.0, Color(158, 14, 10, al(255.0)))),
        };
        let mut spans = vec![span(format!("{}\n", p.title), title.0, title.1)];
        if !p.text.is_empty() { spans.push(span(format!("{}\n", p.text), if style == Style::Title { 64.0 } else { 52.0 }, cream)); }
        if !p.options.is_empty() { spans.push(span("\n".into(), 30.0, cream)); }
        for (i, (label, can)) in p.options.iter().enumerate() {
            let picked = i == p.choice;
            let col = if picked && *can { gold } else if *can { cream } else if picked { Color(196, 176, 140, al(230.0)) } else { Color(170, 164, 156, al(220.0)) };
            spans.push(span(format!("{}{label}\n", if picked { "> " } else { "   " }), 54.0, col));
        }
        if !p.hint.is_empty() { spans.push(span(format!("\n{}", p.hint), 40.0, dim)); }
        let text = Text::new(spans, Some(size.0 * s), align, None);
        if let Some(o) = c.get_game_object_mut(PANEL_BG) {
            o.visible = true;
            o.position = bg.0;
            o.size = bg.1;
            o.set_image(Image { shape: ShapeType::Rectangle(0.0, bg.1, 0.0), image: swatch([8, 8, 12, al(bg_a)]), color: None });
        }
        if let Some(o) = c.get_game_object_mut(PANEL) {
            o.visible = true;
            o.position = at;
            o.size = size;
            o.set_drawable(Box::new(text));
        }
    }

    /// The world map over the road while it is open: the picture, and the found places' names.
    fn draw_map(&mut self, c: &mut Canvas) {
        let open = self.mode == Mode::Map;
        if !open {
            if self.map_img.take().is_some() {
                if let Some(o) = c.get_game_object_mut(MAP) { o.visible = false; }
                for i in 0..atlas::PLACES.len() { if let Some(o) = c.get_game_object_mut(&map_label(i)) { o.visible = false; } }
            }
            return;
        }
        if self.map_img.is_some() { return; }
        let img = Arc::new(crate::map::draw(&self.world));
        self.map_img = Some(img.clone());
        let size = (crate::map::W as f32 * MAP_SCALE, crate::map::H as f32 * MAP_SCALE);
        if let Some(o) = c.get_game_object_mut(MAP) {
            o.visible = true;
            o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None });
        }
        let Some(font) = self.font.clone() else { return };
        let s = c.base_scale();
        let here = atlas::node(&self.world.at);
        let labels = crate::map::label_at(&self.world);
        for i in 0..atlas::PLACES.len() {
            let Some(o) = c.get_game_object_mut(&map_label(i)) else { continue };
            let Some((p, (x, y))) = labels.get(i) else { o.visible = false; continue };
            let col = if p.id == here { Color(250, 246, 236, 255) } else { Color(214, 204, 184, 230) };
            o.visible = true;
            o.position = (MAP_AT.0 + x * MAP_SCALE - 210.0, MAP_AT.1 + y * MAP_SCALE);
            o.set_drawable(Box::new(Text::new(vec![Span::new(p.title.into(), 30.0 * s, Some(36.0 * s), font.clone(), col, 0.0)], Some(420.0 * s), Align::Center, None)));
        }
    }

    /// Tell the road what the world asks.
    fn send(c: &mut Canvas, go: Go) {
        let Some(pf) = c.get_plugin_mut::<PathForgePlugin>() else { return };
        match go {
            Go::Next => pf.next(),
            Go::Choose(b) => { pf.choose(b); }
            Go::Warp(stop) => pf.warp(&stop),
            Go::WarpAt(stop, m) => pf.warp_at(&stop, m),
        }
    }

}

impl QuartzPlugin for Pilgrimage {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn name(&self) -> &str { "pilgrimage" }

    fn on_update(&mut self, c: &mut Canvas, dt: f32) {
        self.ticks += 1;
        self.dress_hero();
        // The boss fighting now, else the boss of the place the hero is in, drawn from its own
        // art: loaded on arrival, so it is ready when the boss steps out.
        let fighting = match &self.mode { Mode::Side(a) if a.why == Why::Boss => a.side.foes.first().and_then(|f| f.breed.sprite), _ => None };
        if let Some((folder, _)) = fighting.or_else(|| self.world.here().boss.and_then(|(_, b)| b.breed.sprite)) {
            if folder != "bell_warden" && self.boss_art.as_ref().map(|(f, _)| *f) != Some(folder) { self.boss_art = Some((folder, load_sheet(folder))); }
        }
        // Slow motion counts down in real time.
        if self.slow.0 > 0 {
            self.slow.0 -= 1;
            c.set_time_scale(if self.slow.0 > 0 { self.slow.1 } else { 1.0 });
        }
        let presses = std::mem::take(&mut *self.presses.lock().unwrap());
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // Where the frame shown stands (sprites and warnings go by it) and where the walk is headed
        // (the rules go by that, or a slow frame lets the hero walk on into an enemy).
        let walked = pf.state().walked;
        let ahead = pf.walked_ahead();
        let stop_now = pf.state().stop.clone();
        let side_path = self.world.side_path_at(&self.fight, ahead, &pf.state().side_paths);
        let ambush = if self.mode == Mode::Play { self.world.ambush_due(&self.fight, ahead) } else { None };
        // Count the road's new frames (its rate is its own, not the game's).
        if let Some(img) = pf.current_image() {
            if !self.last_frame.as_ref().is_some_and(|l| Arc::ptr_eq(l, img)) { self.frames.0 += 1; self.last_frame = Some(img.clone()); }
        }
        if self.ticks % 60 == 0 { self.frames = (0, self.frames.0); }
        self.fade_step(c);
        let keys: Vec<shrine::Key> = presses.iter().filter_map(menu_key).collect();
        let pressed_e = presses.iter().any(|p| matches!(p, Press::Key(k, _) if is(k, &["e"], None)));
        let pressed_t = presses.iter().any(|p| matches!(p, Press::Key(k, _) if is(k, &["t"], None)));
        let pressed_m = presses.iter().any(|p| matches!(p, Press::Key(k, _) if is(k, &["m"], None)));
        let mut notes = Vec::new();

        // What each mode does with this tick, and what it becomes.
        let mode = std::mem::replace(&mut self.mode, Mode::Play);
        self.mode = match mode {
            Mode::Front(mut f) => {
                let slots = self.slots.clone();
                let chosen = keys.iter().fold(None, |got, k| got.or(f.press(*k, &slots)));
                match chosen {
                    Some(Begin::New(slot, calling)) => {
                        (self.slot, self.calling, self.played, self.kept) = (Some(slot), Some(calling), 0.0, None);
                        self.fade.1 = 1.0;
                        Mode::Out(Next::Start)
                    }
                    Some(Begin::Continue(slot)) => { (self.slot, self.kept) = (Some(slot), None); self.fade.1 = 1.0; Mode::Out(Next::Resume) }
                    Some(Begin::Forget(slot)) => {
                        if let Err(e) = profile::forget(&self.saves, slot) { eprintln!("[hollow_road] could not forget pilgrimage {}: {e}", slot + 1); }
                        self.slots = profile::summaries(&self.saves);
                        Mode::Front(f)
                    }
                    Some(Begin::Resume) => Mode::Play,
                    Some(Begin::Profile) => {
                        // Kept first, so the list shows it as it was just left.
                        self.keep_leaving();
                        self.slots = profile::summaries(&self.saves);
                        self.slot = None;
                        self.calling = None;
                        self.played = 0.0;
                        self.kept = None;
                        Mode::Front(Front::profiles(&self.slots))
                    }
                    Some(Begin::Quit) => {
                        // Kept first: quitting must never lose what was done since the last look.
                        self.keep_leaving();
                        eprintln!("[hollow_road] quitting to desktop");
                        #[cfg(not(target_arch = "wasm32"))]
                        std::process::exit(0);
                        #[cfg(target_arch = "wasm32")]
                        { Mode::Front(Front::profiles(&self.slots)) }
                    }
                    None => Mode::Front(f),
                }
            }
            Mode::Play if self.fight.hero.dead => { self.banner.1 = 0; Mode::Dying(0) }
            Mode::Play if pressed_e && self.world.can_rest(&self.fight, ahead) => Mode::Kneel(0),
            Mode::Play if pressed_e && self.world.can_meet_smith(&self.fight, ahead) => Mode::Meet(Meeting::smith()),
            Mode::Play if pressed_e && self.world.can_come_back(&self.fight, ahead) => { self.fade.1 = 1.0; Mode::Out(Next::Back) }
            Mode::Play if pressed_e && side_path.is_some() => {
                // The camera turns to look down it, then the walk goes that way.
                let side = side_path.unwrap_or(1.0);
                Self::swing(c, &format!(r#"{{"yaw":{},"seconds":0.8}}"#, side * PEEK_YAW));
                Mode::Peek(0, side as i8)
            }
            Mode::Peek(t, side) if t >= PEEK_T => { self.fade.1 = 1.0; Mode::Out(Next::Detour(side)) }
            Mode::Peek(t, side) => Mode::Peek(t + 1, side),
            Mode::Play if keys.contains(&shrine::Key::Back) => Mode::Front(Front::Pause(0)),
            Mode::Play if pressed_m => Mode::Map,
            Mode::Map if pressed_m || keys.contains(&shrine::Key::Back) => Mode::Play,
            Mode::Map => Mode::Map,
            Mode::Play if pressed_t && self.world.can_turn(&self.fight, ahead) => { self.fade.1 = 1.0; Mode::Out(Next::Turn) }
            Mode::Play if self.world.boss_due(&self.fight, ahead).is_some() => {
                // The camera swings round to the path's side, and the fight goes on along it.
                Self::swing(c, SIDE_VIEW);
                let h = &self.fight.hero;
                let boss = self.world.boss_due(&self.fight, ahead).unwrap_or(&atlas::BELL_WARDEN);
                let mut side = SideFight::boss(h.stats, h.hp, h.stamina, h.flasks, boss.breed, self.world.scale());
                (side.hero.arrows, side.hero.charges, side.hero.bulwark) = (h.arrows, h.charges, h.bulwark);
                notes.push(Note::Arrived(boss.name()));
                Mode::Side(Box::new(Arena { side, t: 0, done: 0, why: Why::Boss }))
            }
            Mode::Play if ambush.is_some() => {
                // Foes spring out: the camera swings round, and they are fought along the road.
                let k = ambush.unwrap_or(0);
                let chest = self.world.here().ambushes[k].chest;
                let at: Vec<(&'static side::Breed, f32)> = self.world.ambush_foes(k).into_iter().map(|(kind, s)| (side::breed_of(kind), s)).collect();
                Self::swing(c, SIDE_VIEW);
                let h = &self.fight.hero;
                notes.push(Note::Ambushed(chest));
                let mut side = SideFight::ambush(h.stats, h.hp, h.stamina, h.flasks, &at);
                (side.hero.arrows, side.hero.charges, side.hero.bulwark) = (h.arrows, h.charges, h.bulwark);
                let scale = self.world.scale();
                side.foes = side.foes.into_iter().map(|f| f.scaled(scale)).collect();
                Mode::Side(Box::new(Arena { side, t: 0, done: 0, why: Why::Ambush(k) }))
            }
            Mode::Play => Mode::Play,
            Mode::Dying(t) if t >= DIE_T => { self.fade.1 = 1.0; Mode::Out(Next::Respawn) }
            Mode::Dying(t) => Mode::Dying(t + 1),
            Mode::Kneel(t) if t >= KNEEL_T => { self.fade.1 = 1.0; Mode::Out(Next::Sanctum) }
            Mode::Kneel(t) => Mode::Kneel(t + 1),
            Mode::Out(next) if self.fade.0 < 1.0 => Mode::Out(next),
            Mode::Out(Next::Detour(side)) => {
                if let Some(go) = self.world.explore(&self.fight, ahead, side as f32) { Self::send(c, go); }
                Self::swing(c, r#"{"view":"home","seconds":0.05}"#);
                Mode::Cut(After::Walk)
            }
            Mode::Out(Next::Back) => {
                if let Some(go) = self.world.come_back() { Self::send(c, go); }
                Mode::Cut(After::Walk)
            }
            Mode::Out(Next::Turn) => {
                if let Some(go) = self.world.turn(&self.fight, ahead) { Self::send(c, go); }
                notes.push(Note::Turned);
                Mode::Cut(After::Walk)
            }
            Mode::Out(Next::Resume) => {
                // A pilgrimage carried on: its calling, its levels and its world as they were
                // kept, waking at the shrine it last rested at. A slot that cannot be read after
                // all goes back to the list.
                match self.slot.and_then(|slot| profile::read(&self.saves, slot)).and_then(|s| Some((World::restored(walked, &s.world)?, s))) {
                    Some((world, saved)) => {
                        self.world = world;
                        (self.calling, self.played) = (Some(saved.calling), saved.played as f64);
                        self.fight = Fight::new(self.world.foes());
                        let mut stats = saved.stats;
                        stats.kit = self.world.gear.kit();
                        self.fight.hero = crate::fight::Hero::with(stats);
                        notes.push(Note::Arrived(self.world.checkpoint_title()));
                        let go = self.world.resume();
                        Self::send(c, go);
                        Mode::Cut(After::Wake)
                    }
                    None => {
                        self.slots = profile::summaries(&self.saves);
                        (self.slot, self.fade.1) = (None, 0.0);
                        Mode::Front(Front::profiles(&self.slots))
                    }
                }
            }
            Mode::Out(Next::Start) => {
                // A pilgrimage begun: whole, at the first shrine, getting up off the ground, at
                // the levels everyone starts with.
                self.world = World::new(walked);
                self.fight = Fight::new(self.world.foes());
                self.fight.hero.stats = crate::fight::Stats::default();
                // The calling's own: what the other hand holds, and where its levels lean.
                if let Some(c) = self.calling {
                    if let Some(name) = c.offhand() {
                        let i = crate::gear::find(name);
                        self.world.gear.gain(i);
                        self.world.gear.wear(1, Some(i));
                    }
                    let (st, [m, d, f]) = (&mut self.fight.hero.stats, c.leaning());
                    (st.might, st.dexterity, st.focus) = (st.might + m, st.dexterity + d, st.focus + f);
                }
                self.fight.hero.stats.kit = self.world.gear.kit();
                self.fight.hero = crate::fight::Hero::with(self.fight.hero.stats);
                notes.push(Note::Arrived(self.world.here().title));
                self.fade.1 = 0.0;
                Mode::Wake(0)
            }
            Mode::Out(Next::Respawn) => { let go = self.world.died(ahead); Self::send(c, go); Mode::Cut(After::Wake) }
            Mode::Out(Next::Sanctum) => {
                notes.extend(self.world.rest(&mut self.fight));
                Self::send(c, Go::Warp(world::SANCTUM.into()));
                Mode::Cut(After::Sanctum)
            }
            Mode::Out(Next::Road) => { let go = self.world.leave_rest(); Self::send(c, go); Mode::Cut(After::Rise) }
            Mode::Cut(After::Sanctum) if stop_now.as_deref() == Some(world::SANCTUM) => {
                let h = &mut self.fight.hero;
                (h.lane, h.x) = (0, 0.0);
                self.fade.1 = 0.0;
                Mode::Sanctum(Sanctum::new())
            }
            Mode::Cut(After::Sanctum) => Mode::Cut(After::Sanctum),
            // The world arrives when the walk shows the warp (`World::tick` below).
            Mode::Cut(after) if self.world.warping.is_some() => Mode::Cut(after),
            Mode::Cut(after) => {
                self.fade.1 = 0.0;
                let h = &mut self.fight.hero;
                (h.lane, h.x) = (0, 0.0);
                match after { After::Wake => Mode::Wake(0), After::Walk => Mode::Play, _ => Mode::Rise(0) }
            }
            Mode::Sanctum(mut s) => {
                let mut rise = false;
                for k in &keys {
                    match s.press(*k, &mut self.world, &mut self.fight) {
                        Some(Outcome::Rise) => rise = true,
                        Some(Outcome::Note(n)) => notes.push(n),
                        None => {}
                    }
                }
                // Across to the station chosen, at a walk.
                let h = &mut self.fight.hero;
                h.lane = s.lane;
                h.x += (s.lane as f32 * BEFORE_X - h.x) * 0.08;
                if rise { self.fade.1 = 1.0; Mode::Out(Next::Road) } else { Mode::Sanctum(s) }
            }
            Mode::Rise(t) if t >= RISE_T => Mode::Play,
            Mode::Rise(t) => Mode::Rise(t + 1),
            Mode::Wake(t) if t >= WAKE_T => Mode::Play,
            Mode::Wake(t) => Mode::Wake(t + 1),
            Mode::Side(a) if a.side.hero.dead => {
                Self::swing(c, "home");
                let h = &mut self.fight.hero;
                (h.hp, h.dead) = (0.0, true);
                Mode::Dying(0)
            }
            Mode::Side(a) if a.side.won() && a.done >= if a.why == Why::Boss { ARENA_OUT } else { AMBUSH_OUT } => {
                match a.why {
                    Why::Boss => notes.extend(self.world.boss_felled()),
                    Why::Ambush(k) => {
                        let fallen = a.side.foes.iter().filter_map(|f| f.breed.kind).collect();
                        notes.extend(self.world.ambush_cleared(k, fallen));
                    }
                }
                Self::swing(c, "home");
                let h = &mut self.fight.hero;
                (h.hp, h.stamina, h.flasks, h.arrows, h.charges, h.bulwark) = (a.side.hero.hp, a.side.hero.stamina, a.side.hero.flasks, a.side.hero.arrows, a.side.hero.charges, a.side.hero.bulwark);
                Mode::Play
            }
            Mode::Side(a) => Mode::Side(a),
            Mode::Meet(mut m) => {
                if keys.iter().fold(false, |done, k| done || m.press(*k)) {
                    self.world.met_smith = true;
                    notes.push(Note::Met("Brannoc"));
                    Mode::Play
                } else {
                    Mode::Meet(m)
                }
            }
        };

        // The fight runs on game time (slower in slow motion, not at all in a hitstop), and only
        // on the road: playing, or falling.
        let fighting = matches!(self.mode, Mode::Play | Mode::Dying(_));
        let mut events = Vec::new();
        let mut ran = 0;
        let mut input = Input::default();
        if fighting {
            input = self.input(c, if self.mode == Mode::Play { &presses } else { &[] });
            self.owed += dt / Canvas::TICK;
            let mut first = true;
            while self.owed >= 1.0 {
                self.owed -= 1.0;
                ran += 1;
                let i = if first { input } else { Input { forward: input.forward, back: input.back, run: input.run, guard: input.guard, ..Input::default() } };
                if first { self.pending = Input::default(); }
                first = false;
                events.extend(self.fight.tick(&i, ahead));
            }
        } else if matches!(self.mode, Mode::Side(_)) {
            let bi = self.side_input(c, &presses);
            self.owed += dt / Canvas::TICK;
            let mut first = true;
            while self.owed >= 1.0 {
                self.owed -= 1.0;
                ran += 1;
                let i = if first { bi } else { SideInput { left: bi.left, right: bi.right, run: bi.run, guard: bi.guard, ..SideInput::default() } };
                if first { self.side_pending = SideInput::default(); }
                first = false;
                let Mode::Side(a) = &mut self.mode else { break };
                // Nothing moves while the camera swings round.
                if a.t < ARENA_IN { a.t += 1; continue; }
                if a.side.won() { a.done += 1; }
                events.extend(a.side.tick(&i));
            }
            // The bars and the HUD read the road fight's hero.
            if let Mode::Side(a) = &self.mode {
                let (h, s) = (&mut self.fight.hero, &a.side.hero);
                (h.hp, h.stamina, h.flasks, h.arrows, h.charges) = (s.hp.max(0.0), s.stamina, s.flasks, s.arrows, s.charges);
            }
        } else {
            (self.owed, self.pending) = (0.0, Input::default());
        }
        #[cfg(test)]
        events.append(&mut self.staged);
        // At a fork the walk waits at the signpost for a side to be picked.
        let held = self.world.holds_at_fork(c.get_plugin::<PathForgePlugin>().and_then(|p| p.state().choose_within), self.fight.hero.lane);
        let speed = if self.mode != Mode::Play || held { 0.0 } else { self.fight.speed(&input, ahead) };
        if let Some(pf) = c.get_plugin_mut::<PathForgePlugin>() { pf.set_speed(speed); }
        self.last_speed = speed;
        #[cfg(test)]
        self.log.extend(events.iter().map(|e| (self.ticks, *e)));
        self.show(c, &events);
        self.fx.tick(ran);
        // A follow-through runs on fight ticks, and gives way to whatever the hero does next.
        self.flourish = if self.fight.hero.act.is_some() || self.fight.hero.reeling > 0 { 0 } else { self.flourish.saturating_sub(ran) };
        match &self.mode {
            Mode::Side(a) => {
                let (hs, fw) = (a.side.hero.s, a.side.hero.facing);
                // A rune goes off where the yard puts it, which is road metres past the arena's.
                let events: Vec<Event> = events.iter().map(|e| match *e { Event::Burst { enemy, at } => Event::Burst { enemy, at: walked + ARENA_D as f64 + at }, e => e }).collect();
                self.fx.on_events_at(&events, (0.0, ARENA_D + hs), fw, |i| a.side.foes.get(i).map(|f| (0.0, walked + (ARENA_D + f.s) as f64)));
            }
            _ => self.fx.on_events(&events, &self.fight),
        }
        // Played time, and the pilgrimage kept as it goes.
        if !matches!(self.mode, Mode::Front(_)) { self.played += dt as f64; }
        if self.ticks % KEEP_EVERY == 0 { self.keep(); }
        // Souls (and shards) for the fallen (the boss's come when the fight ends).
        let road = !matches!(self.mode, Mode::Side(_));
        let fallen: Vec<Archetype> = events.iter().filter(|_| road).filter_map(|e| match *e { Event::Felled { enemy } => self.fight.enemies.get(enemy).map(|e| e.kind), _ => None }).collect();
        let found = self.world.felled(fallen);
        self.dead_for = if let Mode::Dying(t) = self.mode { t } else { 0 };
        // The world follows the road everywhere but the sanctum (it is not a place).
        if !self.at_rest() && !matches!(self.mode, Mode::Front(_)) {
            let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
            let st = pf.state();
            let (stop, within) = (st.stop.clone(), st.choose_within);
            let (go, n) = self.world.tick(&mut self.fight, ahead, stop.as_deref(), within);
            notes.extend(n);
            if let Some(go) = go { Self::send(c, go); }
        }
        notes.extend(found);
        if !notes.is_empty() { self.say(&notes); }
        self.banner.1 = self.banner.1.saturating_sub(1);
        self.draw_callout(c);
        self.paint_marks(c, walked);
        self.place(c, walked);
        let blade = self.hero_blade(c);
        self.fx.place(c, walked, &self.fight, blade);
        self.bars(c, walked);
        self.draw_panel(c);
        self.draw_map(c);
        if self.ticks % 15 == 1 || self.banner.1 > 0 || self.mode != Mode::Play || events.contains(&Event::Died) { self.hud(c, walked); }
    }
}

#[cfg(test)]
mod armed {
    /// Every weapon in every bare-handed clip it has, a row each:
    /// `cargo test --release armed_sheet -- --ignored` writes target/shots/sheet_armed.png
    /// (`ARMED=<text>`: only the clips whose names hold it; `OFFHAND=<art>`: with that offhand).
    #[test]
    #[ignore]
    fn armed_sheet() {
        let mut s = super::armed_sheet("wayfarer_ne").expect("hero sheet");
        // `OFFHAND=<art>`: every one-handed weapon dressed with that offhand too.
        if let Ok(off) = std::env::var("OFFHAND") {
            for a in crate::gear::ITEMS.iter().filter_map(|i| i.arms).chain([crate::gear::SWORD]).filter(|a| !a.class.two_handed()) { s.dress(a.art, &off); }
        }
        let only = std::env::var("ARMED").unwrap_or_default();
        let mut clips: Vec<_> = s.clips.iter().filter(|c| (c.0.contains('@') || c.0.contains('+')) && c.0.contains(only.as_str())).collect();
        clips.sort_by_key(|c| c.0.clone());
        assert!(!clips.is_empty(), "no armed clips");
        let cols = clips.iter().map(|c| c.1.frames.len()).max().unwrap_or(1) as u32;
        let (w, h) = s.canvas;
        let mut out = image::RgbaImage::from_pixel(cols * w, clips.len() as u32 * h, image::Rgba([60, 64, 58, 255]));
        for (r, (_, c)) in clips.iter().enumerate() {
            for (k, f) in c.frames.iter().enumerate() { image::imageops::overlay(&mut out, f.as_ref(), (k as u32 * w) as i64, (r as u32 * h) as i64); }
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/shots");
        let _ = std::fs::create_dir_all(&dir);
        out.save(dir.join("sheet_armed.png")).unwrap();
        eprintln!("[armed] {}", clips.iter().map(|(n, c)| format!("{n} {}", c.frames.len())).collect::<Vec<_>>().join(", "));
    }
}
