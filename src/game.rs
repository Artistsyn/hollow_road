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
use crate::loading::{BossArt, Loading, Sheets};
use crate::audio::{Audio, Sfx, Tune};
use crate::cutscene::{Cue, Cutscene, Focus};
use crate::atlas;
use crate::art::{self, HuskPose, Pose};
use crate::fx::{self, Fx};
use crate::fight::{Ai, Archetype, Enemy, Event, Fight, Idle, Input, Kind, Look, Move, Phase, Reel, Way, DOWN_FALL, DOWN_LIE, DOWN_RISE, HERO_D, LANE_W, SHOULDER, STUMBLE, TRAVEL};
use crate::profile::{self, Slot};
use crate::prologue::{self, Lesson, Prologue, Step};
use crate::shrine::{self, Begin, Calling, Front, Meeting, Outcome, Panel, Sanctum};
use crate::side::{self, SideFight, SideInput};
use crate::world::{self, Go, Note, World};
use image::RgbaImage;
use quartz::plugin::QuartzPlugin;
use quartz::*;
use quartz_path_forge::light::Relighter;
use quartz_path_forge::marks::{Blow, Cast, Fill, GroundMark, Shadow, Sigil, Telegraph};
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
/// Ticks a banner line shows for, and how many may wait behind it.
const BANNER_T: u32 = 200;
const BANNERS_HELD: usize = 4;
/// Ticks the yard stays after an ambush's last foe falls.
const AMBUSH_OUT: u32 = 80;
/// Degrees the camera turns to look down a side path, and ticks it looks before the walk goes.
const PEEK_YAW: f32 = 38.0;
const PEEK_T: u32 = 60;
/// Ticks between the road's new frames that still count as drawing steadily (it draws about one
/// every two), and the longest the start or a cut waits for that, ticks: the loading screen loops
/// until the road is ready, and only a road that never steadies lets the game on regardless.
const FLOW_GAP: u64 = 8;
const ROAD_WAIT: u32 = 1800;
/// A cut waiting on the road longer than this (ticks) shows the loading screen over its black.
const CUT_SHOWS_LOADING: u32 = 24;
/// Ticks stepping through the veil, and the pace of it (m/s): through it and a little on.
const CROSS_T: u32 = 54;
const CROSS_SPEED: f32 = 1.4;
const BOSS_HELP: &str = "A/D move   A A / D D (held) run   Shift+A/D roll   Space jump   LMB strike   Shift+LMB heavy   RMB guard   Shift+RMB parry   R drink   F ranged attack   C cast   Q weapon art      gold parry  ·  orange roll or block  ·  purple roll  ·  a blow held high: wait for it  ·  a dodged leap leaves it open";
const AMBUSH_HELP: &str = "A/D move   A A / D D (held) run   Shift+A/D roll   Space jump   LMB strike   Shift+LMB heavy (cuts through all in reach)   RMB guard   Shift+RMB parry   R drink   F ranged attack   C cast   Q weapon art      jump a low sweep  ·  step off a rune  ·  roll through a lunge or pounce";
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
/// A dread foe's aura (`Pilgrimage::wreathe`): its colour, and how strongly it shows.
const DREAD_AURA: ([u8; 3], f32) = ([255, 64, 30], 0.85);
/// The rune's hum on the hero at a fork: its green (the well-stones' glow), its strongest glow,
/// how long it lasts and how many ticks one beat of it takes.
/// Ticks after the rune is thrown as bait before it hums back into the hand.
const RUNE_BACK: u32 = 900;
const RUNE_PULSE: ([u8; 3], f32, u64, f32) = ([120, 230, 120], 0.7, 240, 48.0);
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
const FOE_NAME: &str = "foe_name";
/// The kit's shrine and signpost, stood on the road where the world says they are.
const SHRINE: &str = "shrine";
const SIGNPOST: &str = "signpost";
/// What is set in the wall a barred fork branch is shut with (PathForge walls it up in its own
/// wall material, `PathForgePlugin::bar`): a locked gate where a key opens it, a door barred from
/// the far side for a shortcut.
const BARRIER_ART: (&str, &str) = ("pixellab/barrier/locked.png", "pixellab/barrier/shut.png");
/// Where in each picture its two leaves are (x0 y0 x1 y1), to swing them back once opened.
const BARRIER_LEAVES: ([f32; 4], [f32; 4]) = ([0.11, 0.1, 0.89, 0.9], [0.16, 0.14, 0.84, 0.94]);
/// The least brightness the road may fall to (the mean of its ground half, 0..1): enough to see
/// a threat coming down a night street (`PathForgePlugin::set_visibility_floor`).
const VISIBILITY_FLOOR: f32 = 0.15;
/// The souls left where the hero fell.
const STAIN: &str = "stain";
/// The veil before a standing boss (`veil_picture`), and how tall it hangs, metres.
const VEIL: &str = "veil";
const VEIL_H: f32 = 4.2;
/// Ring blows' fronts as they roll (`crest_picture`): two to a ring, two rings at once.
const WAVES: usize = 4;
fn wave_id(i: usize) -> String { format!("wave_{i}") }
/// People standing on the road in a cutscene (`cutscene::Figure`), at most this many at once.
const CAMEOS: usize = 5;
fn cameo_id(i: usize) -> String { format!("cameo_{i}") }
const CHEST: &str = "chest";
/// The keeper and the smith, in the sanctum (and the smith by the road until met).
const KEEPER: &str = "keeper";
const SMITH: &str = "smith";
/// Gruk behind his counter at the guild hall's end, and the counter.
const GRUK: &str = "gruk";
const COUNTER: &str = "counter";
/// Someone met on the road (`visitors::Visitor`).
const VISITOR: &str = "visitor";
/// The guild's bounty board, in the foot of the wall the hall ends at.
const BOARD_ART: &str = "pixellab/guild/board.png";
/// Where a road with no walls ends: fallen old stones heaped across it.
const FALLEN_STONES_ART: &str = "pixellab/barrier/fallen_stones.png";
/// The black over everything for a cut, and the box words and choices are drawn in.
const FADE: &str = "fade";
const PANEL_BG: &str = "panel_bg";
const PANEL: &str = "panel";
/// What E would do here, on a plate low in the middle: the plate, the key on it, the key's letter
/// and the words.
const PROMPT_BG: &str = "prompt_bg";
const PROMPT_KEY: &str = "prompt_key";
const PROMPT_E: &str = "prompt_e";
const PROMPT: &str = "prompt";
const PROMPT_Y: f32 = 1880.0;
/// The touch stick's ring and the thumb's knob on it; each button's objects (`touch_id`).
const TOUCH_STICK: &str = "touch_stick";
const TOUCH_KNOB: &str = "touch_knob";
fn touch_id(part: &str, i: usize) -> String { format!("touch_{part}_{i}") }
/// Where the stick's ring rests with no thumb on it, showing where one goes.
const STICK_REST: (f32, f32) = (700.0, 1640.0);

/// A soft disc `size` pixels across, `fill` inside a rim of `edge`, smoothed at both edges.
fn touch_disc(size: u32, fill: [u8; 4], edge: [u8; 4]) -> Arc<RgbaImage> {
    let r = size as f32 * 0.5;
    let rim = (size as f32 * 0.035).max(2.0);
    Arc::new(RgbaImage::from_fn(size, size, |x, y| {
        let d = ((x as f32 + 0.5 - r).powi(2) + (y as f32 + 0.5 - r).powi(2)).sqrt();
        let outer = (r - 0.5 - d).clamp(0.0, 1.0);
        let in_rim = ((d - (r - 1.5 - rim)) + 0.5).clamp(0.0, 1.0);
        let mix = |k: usize| (fill[k] as f32 + (edge[k] as f32 - fill[k] as f32) * in_rim).round() as u8;
        let a = (fill[3] as f32 + (edge[3] as f32 - fill[3] as f32) * in_rim) * outer;
        image::Rgba([mix(0), mix(1), mix(2), a.round() as u8])
    }))
}
/// A cutscene's letterbox bars, its words under the picture, its title card (the big line and the
/// small), the white it fades through, and how to skip it.
const CINE_TOP: &str = "cine_top";
const CINE_BOTTOM: &str = "cine_bottom";
const CINE_CAPTION: &str = "cine_caption";
const CINE_TITLE: &str = "cine_title";
const CINE_SUB: &str = "cine_sub";
const CINE_WHITE: &str = "cine_white";
const CINE_SKIP: &str = "cine_skip";
/// A soft dark band behind the title card, so it reads over anything.
const CINE_SHADE: &str = "cine_shade";
/// How far past the canvas the cutscene's bars and white reach: a window of another shape shows
/// the road beyond the canvas (`BLEED`).
const CINE_OVER: f32 = 1200.0;
/// The fade's black, as far past the canvas as the cutscene's bars, so no window shows the road
/// round it.
const COVER: (f32, f32) = (3840.0 + 2.0 * CINE_OVER, 2160.0 + 2.0 * CINE_OVER);
/// A watcher looks along the road for the first this many ticks of each of its waits (it faces the
/// road the rest of the time).
const LINGER_GLANCE: u32 = 150;
/// The top of a prologue lesson's box, under the hero's bars.
const LESSON_Y: f32 = 420.0;
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
/// Under the stamina, a bulwark's ward: dim gold while it fills, bright once it can burst; under
/// that (or in its place), a forged weapon's art meter: dusk blue while it fills, pale once the
/// art is paid for.
const BARS: [(&str, f32, f32, f32, f32, [u8; 4]); 15] = [
    ("bar_hp_back", 60.0, 140.0, 900.0, 34.0, [30, 20, 22, 200]),
    ("bar_hp", 60.0, 140.0, 900.0, 34.0, [176, 36, 40, 255]),
    ("bar_st_back", 60.0, 186.0, 700.0, 22.0, [20, 26, 22, 200]),
    ("bar_st", 60.0, 186.0, 700.0, 22.0, [70, 150, 80, 255]),
    ("bar_ward_back", 60.0, 218.0, 500.0, 16.0, [24, 22, 16, 200]),
    ("bar_ward", 60.0, 218.0, 500.0, 16.0, [150, 132, 84, 255]),
    ("bar_ward_ready", 60.0, 218.0, 500.0, 16.0, [255, 236, 170, 255]),
    ("bar_art_back", 60.0, 218.0, 500.0, 16.0, [16, 18, 30, 200]),
    ("bar_art", 60.0, 218.0, 500.0, 16.0, [92, 108, 196, 255]),
    ("bar_art_ready", 60.0, 218.0, 500.0, 16.0, [190, 210, 255, 255]),
// Whoever was called to a boss, under the hero's, while they stand.
("bar_ally_back", 60.0, 272.0, 420.0, 14.0, [26, 22, 18, 200]),
("bar_ally", 60.0, 272.0, 420.0, 14.0, [206, 154, 84, 255]),
    // The foe's, top right, mirroring the hero's, its name over it (`FOE_NAME`).
    ("bar_foe_back", 2580.0, 140.0, 1200.0, 26.0, [30, 20, 22, 200]),
    ("bar_foe", 2580.0, 140.0, 1200.0, 26.0, [150, 40, 44, 255]),
    ("bar_foe_stance", 2580.0, 172.0, 1200.0, 12.0, [214, 176, 70, 255]),
];

fn enemy_id(i: usize) -> String { format!("enemy_{i}") }
/// A shot in flight: a side-on foe's (one at a time each), or an arrow or hex down the road.
fn shot_id(i: usize) -> String { format!("shot_{i}") }

/// The colour a weapon art flashes and calls in.
pub(crate) fn art_rgb(art: crate::arts::WeaponArt) -> (u8, u8, u8) {
    use crate::arts::WeaponArt;
    match art {
        WeaponArt::Toll => (255, 214, 140), WeaponArt::Wail => (120, 220, 190), WeaponArt::Hatchet => (236, 160, 110),
        WeaponArt::Sentence => (255, 214, 110), WeaponArt::Reap => (190, 140, 255), WeaponArt::Lance => (255, 240, 170),
        WeaponArt::StoneSkin => (210, 200, 190), WeaponArt::Wingbeat => (255, 236, 200),
    }
}

/// The thrown Hatchet as drawn: the cleaver's own picture, a quarter turn each few ticks.
fn hatchet_picture(s: &Sheet, age: u32) -> Option<Arc<RgbaImage>> {
    let w = s.weapons.get("chieftains_cleaver")?;
    Some(Arc::new(match (age / 3) % 4 { 0 => w.image.clone(), 1 => image::imageops::rotate90(&w.image), 2 => image::imageops::rotate180(&w.image), _ => image::imageops::rotate270(&w.image) }))
}
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

/// The veil across the road before a standing boss, `ticks` into its loop: a black shroud come
/// out of nothing overhead, its folds stirring as if something breathes beyond it, a crimson glow
/// beating behind it like a heart, flecks of ash drifting up off its frayed hem, the wrong way.
/// Now and then a hand far too large presses into it from the other side, and slowly withdraws.
/// `part` (0 to 1) is a hero stepping through: it burns open from the middle, a ragged tear with
/// an ember edge, and what is left of it thins to nothing. The still loop and the hand are made
/// once, a frame every four ticks.
fn veil_picture(ticks: u32, part: f32) -> Arc<RgbaImage> {
    const W: u32 = 320;
    const H: u32 = 224;
    const LOOP: u32 = 128;
    // The hand comes once every `PRESS_EVERY` ticks, `PRESS_AT` into it, for `PRESS_T` ticks (each
    // a whole number of loops, so the folds it presses into are the loop's own).
    const PRESS_EVERY: u32 = LOOP * 4;
    const PRESS_AT: u32 = LOOP;
    const PRESS_T: u32 = 64;
    fn make(t: u32, part: f32, press: f32) -> RgbaImage {
        use std::f32::consts::{PI, TAU};
        let hash = |i: u32, k: u32| { let mut h = i.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B); h ^= h >> 15; h = h.wrapping_mul(0x2C1B_3C6D); h ^= h >> 13; (h & 0xFFFF) as f32 / 65535.0 };
        let p = (t % LOOP) as f32 / LOOP as f32;
        // Two beats, close together, and a long quiet.
        let beat = (-((p - 0.05) / 0.04).powi(2)).exp() + 0.65 * (-((p - 0.2) / 0.04).powi(2)).exp();
        let thin = (1.0 - part).max(0.0).powf(0.8);
        let aspect = W as f32 / H as f32;
        // The hand: a palm and four fingers spread, pressing in (`press` 0 to 1, in and back out).
        let push = (PI * press).sin().powi(2);
        let hand = |u: f32, v: f32| -> f32 {
            if push <= 0.0 { return 0.0; }
            let (x, y) = ((u - 0.3) * aspect / 0.2, (v - 0.42) / 0.2);
            if !(-1.6..1.4).contains(&x) || !(-1.6..1.2).contains(&y) { return 0.0; }
            // A finger: soft round about the segment from (ax, ay) to (bx, by).
            let finger = |ax: f32, ay: f32, bx: f32, by: f32, r: f32| {
                let (dx, dy) = (bx - ax, by - ay);
                let k = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
                let d = (x - ax - k * dx).hypot(y - ay - k * dy) / r;
                (-d * d).exp()
            };
            let palm = (-((x / 0.44).powi(2) + ((y - 0.28) / 0.4).powi(2)).powi(2)).exp();
            let mut b = palm;
            for (fx, len, lean) in [(-0.3, 0.62f32, -0.22f32), (-0.1, 0.78, -0.07), (0.1, 0.74, 0.07), (0.29, 0.56, 0.2)] {
                b = b.max(finger(fx, -0.02, fx + lean * len, -0.02 - len, 0.1));
            }
            b.max(finger(-0.36, 0.42, -0.78, 0.05, 0.11)) * push
        };
        // Where it burns open: a tear down the middle, widest at the hero's height, ragged.
        // Value noise along a line, for edges that do not repeat.
        let noise = |x: f32, k: u32| { let (i, f) = (x.floor(), x - x.floor()); let f = f * f * (3.0 - 2.0 * f); let n = |i: f32| hash(i as i32 as u32, k); n(i) * (1.0 - f) + n(i + 1.0) * f };
        let tear = |u: f32, v: f32| -> f32 {
            if part <= 0.0 { return -1.0; }
            let side = if u < 0.5 { 7 } else { 11 };
            let ragged = 0.05 * (noise(v * 9.0 + 3.0 * part, side) - 0.5) + 0.03 * (noise(v * 23.0, side + 1) - 0.5) + 0.014 * (noise(v * 61.0 - 9.0 * part, side + 2) - 0.5);
            let open = part.powf(1.1) * 0.85 * (1.0 - ((v - 0.62) / 0.75).powi(2)).max(0.0).sqrt();
            open + ragged * part.min(0.3) / 0.3 - (u - 0.5).abs()
        };
        let mut img = RgbaImage::from_fn(W, H, |x, y| {
            let (u, v) = (x as f32 / (W - 1) as f32, y as f32 / (H - 1) as f32);
            let edge = (u / 0.07).min((1.0 - u) / 0.07).clamp(0.0, 1.0);
            // Out of nothing overhead; frayed at the hem, a shadow on the ground beneath.
            let top = (v / 0.38).clamp(0.0, 1.0).powf(1.6);
            let hem_at = 0.9 + 0.03 * (u * 37.0 + (TAU * p).sin()).sin() + 0.025 * (u * 97.0).sin();
            let hem = if v < hem_at { 1.0 } else { 0.25 * (1.0 - (v - hem_at) / (1.0 - hem_at + 1e-3)).max(0.0) };
            // Folds, swaying most low down, as cloth does.
            let uu = u + 0.018 * v.powf(1.5) * (TAU * p + u * 3.0).sin();
            // Uneven folds, not a row of pleats.
            let warp = 1.6 * (uu * TAU * 1.3 + 0.7).sin() + 0.6 * (uu * TAU * 2.9).sin();
            let mut fold = 0.5 + 0.3 * (uu * TAU * 4.0 + warp + 0.8 * (TAU * p).sin()).sin() + 0.2 * (uu * TAU * 9.0 + 0.5 * warp - TAU * 2.0 * p).sin();
            // Pressed from beyond: the folds pulled smooth over it, lit on the upper left.
            let (b, lit) = { let b = hand(u, v); let d = 0.012; (b, (hand(u - d, v - d) - hand(u + d, v + d)) * 6.0) };
            fold = fold * (1.0 - b) + 0.5 * b;
            let (gx, gy) = ((u - 0.5) / 0.26, (v - 0.72) / 0.32);
            let glow = (-(gx * gx + gy * gy)).exp() * (0.2 + 0.8 * beat);
            // Deep in the troughs, a dull sheen on the ridges.
            let mut a = (0.86 - 0.1 * fold + 0.08 * b) * top * edge * hem * thin;
            let sheen = fold.powi(2);
            let mut rgb = [6.0 + 20.0 * sheen, 4.0 + 17.0 * sheen, 9.0 + 23.0 * sheen];
            for (c, g) in rgb.iter_mut().zip([132.0, 14.0, 30.0]) { *c += g * glow * (1.0 - 0.7 * b); }
            if b > 0.0 {
                // Cloth drawn tight over it: a dull sheen where it faces up and left, shadow below.
                let l = lit.clamp(-1.0, 1.0);
                for (c, h) in rgb.iter_mut().zip([62.0, 56.0, 74.0]) { *c += h * l.max(0.0) * b.sqrt(); *c *= 1.0 - 0.6 * (-l).max(0.0); }
            }
            // The tear, its edge burning.
            let o = tear(u, v);
            if o > 0.0 { a = 0.0; }
            else if o > -0.06 {
                // Charred toward the edge, and at the very edge burning, flickering along it.
                let char = 1.0 + o / 0.06;
                for c in rgb.iter_mut() { *c *= 1.0 - 0.8 * char; }
                a = a.max(0.85 * char * top * hem);
                if o > -0.012 {
                    let k = (1.0 + o / 0.012) * (0.7 + 0.3 * noise(v * 40.0 + t as f32 * 0.4, 5));
                    for (c, e) in rgb.iter_mut().zip([255.0, 120.0, 46.0]) { *c = *c * (1.0 - k) + e * k; }
                    a = a.max(k * top * hem);
                }
            }
            if a <= 0.004 { return image::Rgba([0, 0, 0, 0]); }
            image::Rgba([rgb[0].min(255.0) as u8, rgb[1].min(255.0) as u8, rgb[2].min(255.0) as u8, (a.min(1.0) * 255.0) as u8])
        });
        // Ash off the hem, drifting up, the wrong way; a few embers. Each rises a whole number of
        // times its height in a loop, so the loop has no seam.
        for i in 0..420u32 {
            let ember = hash(i, 3) < 0.07;
            let u0 = hash(i, 1);
            let rise = ((hash(i, 2) + p * (1.0 + (hash(i, 4) * 2.0).floor())) % 1.0).powf(0.9);
            let mut px = u0 * (W - 1) as f32 + 3.0 * (TAU * (p * 2.0 + hash(i, 5))).sin();
            let mut py = (0.95 - 0.8 * rise) * (H - 1) as f32;
            if part > 0.0 {
                let (dx, dy) = (px - 0.5 * W as f32, py - 0.62 * H as f32);
                let len = dx.hypot(dy).max(1.0);
                let push = part.powf(1.5) * (90.0 + 60.0 * hash(i, 6));
                px += dx / len * push;
                py += dy / len * push - 30.0 * part * hash(i, 7);
            }
            let edge = (u0 / 0.08).min((1.0 - u0) / 0.08).clamp(0.0, 1.0);
            let lit = (rise / 0.15).min(1.0) * (1.0 - rise).powf(0.7) * edge * (0.6 + 0.4 * (TAU * (p * 3.0 + hash(i, 8))).sin()) * (1.0 - 0.6 * part);
            if lit <= 0.03 { continue; }
            let rgb = if ember { [255.0, 70.0, 50.0] } else { [176.0, 166.0, 190.0] };
            let size = if hash(i, 9) < 0.12 { 2 } else { 1 };
            for oy in 0..size { for ox in 0..size {
                let (x, y) = (px as i32 + ox, py as i32 + oy);
                if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 { continue; }
                let q = img.get_pixel_mut(x as u32, y as u32);
                let a = (lit * 230.0) as u8;
                if a > q.0[3] || lit > 0.5 { q.0 = [(rgb[0] * lit + q.0[0] as f32 * (1.0 - lit)) as u8, (rgb[1] * lit + q.0[1] as f32 * (1.0 - lit)) as u8, (rgb[2] * lit + q.0[2] as f32 * (1.0 - lit)) as u8, a.max(q.0[3])]; }
            }}
        }
        // Sparks off the burning edge, rising.
        if part > 0.0 {
            for i in 0..90u32 {
                let v = 0.15 + 0.8 * hash(i, 20);
                let (mut lo, mut hi) = (0.0f32, 0.5f32);
                let left = hash(i, 21) < 0.5;
                for _ in 0..12 { let m = 0.5 * (lo + hi); let u = if left { 0.5 - m } else { 0.5 + m }; if tear(u, v) > 0.0 { lo = m } else { hi = m } }
                let u = if left { 0.5 - lo } else { 0.5 + lo };
                let age = (hash(i, 22) + t as f32 * 0.03) % 1.0;
                let (px, py) = (u * (W - 1) as f32 + (if left { -1.0 } else { 1.0 }) * 6.0 * age, v * (H - 1) as f32 - 26.0 * age);
                let k = (1.0 - age) * (1.0 - 0.5 * part);
                let (x, y) = (px as i32, py as i32);
                if k > 0.05 && x >= 0 && y >= 0 && x < W as i32 && y < H as i32 {
                    img.put_pixel(x as u32, y as u32, image::Rgba([255, (150.0 + 80.0 * k) as u8, (60.0 * k) as u8, (255.0 * k) as u8]));
                }
            }
        }
        img
    }
    if part > 0.0 { return Arc::new(make(ticks, part, 0.0)); }
    static STILL: std::sync::OnceLock<Vec<Arc<RgbaImage>>> = std::sync::OnceLock::new();
    static PRESS: std::sync::OnceLock<Vec<Arc<RgbaImage>>> = std::sync::OnceLock::new();
    let cycle = ticks % PRESS_EVERY;
    if (PRESS_AT..PRESS_AT + PRESS_T).contains(&cycle) {
        let frames = PRESS.get_or_init(|| (0..PRESS_T / 4).map(|k| Arc::new(make(k * 4, 0.0, (k * 4 + 2) as f32 / PRESS_T as f32))).collect());
        return frames[((cycle - PRESS_AT) / 4) as usize].clone();
    }
    let frames = STILL.get_or_init(|| (0..LOOP / 4).map(|k| Arc::new(make(k * 4, 0.0, 0.0))).collect());
    frames[((ticks % LOOP) / 4) as usize].clone()
}

/// A ring blow's front rolling along the ground, travelling right (mirrored going left): a crest
/// of light leaning into its way, brightest at its leading face, a smear of it trailing low, the
/// ground under it lit, and sparks thrown up off it. `ticks` runs its flicker (eight frames, made
/// once for each colour and way).
fn crest_picture(ticks: u32, rgb: [u8; 3], mirror: bool) -> Arc<RgbaImage> {
    const W: u32 = 128;
    const H: u32 = 80;
    static MADE: std::sync::Mutex<Option<std::collections::HashMap<(u32, [u8; 3], bool), Arc<RgbaImage>>>> = std::sync::Mutex::new(None);
    let frame = (ticks / 3) % 8;
    let mut made = MADE.lock().unwrap();
    made.get_or_insert_with(Default::default).entry((frame, rgb, mirror)).or_insert_with(|| {
        let hash = |i: u32, k: u32| { let mut h = i.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B); h ^= h >> 15; h = h.wrapping_mul(0x2C1B_3C6D); h ^= h >> 13; (h & 0xFFFF) as f32 / 65535.0 };
        let t = frame as f32;
        let body = [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32];
        const PEAK: f32 = 0.74;
        let mut img = RgbaImage::from_fn(W, H, |x, y| {
            let (u, v) = (x as f32 / (W - 1) as f32, 1.0 - y as f32 / (H - 1) as f32);
            // The wave's height along it: steep at its leading face, a long slope behind.
            let wob = 0.035 * (t * 0.8 * std::f32::consts::TAU / 8.0 * 2.0 + u * 23.0).sin();
            let crest = 0.82 * if u > PEAK { (-((u - PEAK) / 0.09).powi(2)).exp() } else { (-((u - PEAK) / 0.34).powi(2)).exp() } + wob * (u < PEAK) as u8 as f32;
            let tail = (u / 0.12).clamp(0.0, 1.0);
            // Its skin brightest, a glow standing off it, and the ground lit under it.
            let skin = (-(((crest - v) / 0.05).powi(2))).exp();
            let halo = if v > crest { 0.55 * (-((v - crest) / 0.09).powi(2)).exp() } else { 0.0 };
            let fill = if v <= crest { 0.28 + 0.2 * (u * 41.0 - t * 2.4).sin().max(0.0) } else { 0.0 };
            let ground = 0.8 * (-(v / 0.06).powi(2)).exp() * crest.max(0.15);
            let face = (-((u - (PEAK + 0.05)) / 0.05).powi(2)).exp() * (v <= crest + 0.05) as u8 as f32;
            let a = ((skin * 0.95 + halo + fill + ground + face * 0.6) * tail).min(1.0);
            if a < 0.01 { return image::Rgba([0, 0, 0, 0]); }
            let hot = (skin * 0.8 + face).min(1.0);
            let col = |c: f32| (c + (255.0 - c) * hot) as u8;
            image::Rgba([col(body[0]), col(body[1]), col(body[2]), (a * 240.0) as u8])
        });
        // Sparks thrown up and back off the crest.
        for i in 0..40u32 {
            let life = ((t + 8.0 * hash(i, 1)) % 8.0) / 8.0;
            let (u, v) = (0.45 + 0.4 * hash(i, 2) - 0.3 * life, 0.15 + 0.55 * hash(i, 3) + 0.4 * life);
            let (x, y) = ((u * (W - 1) as f32) as i32, ((1.0 - v) * (H - 1) as f32) as i32);
            if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 { continue; }
            let a = ((1.0 - life) * 255.0) as u8;
            let q = img.get_pixel_mut(x as u32, y as u32);
            if a > q.0[3] { q.0 = [255, 244, 220, a]; }
        }
        let img = if mirror { image::imageops::flip_horizontal(&img) } else { img };
        Arc::new(img)
    }).clone()
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
pub(crate) fn kind_art(k: Archetype) -> (&'static str, f32) {
    match k {
        Archetype::Husk => ("husk", art::HUSK_METRES),
        Archetype::Lancer => ("lancer", 1.9),
        Archetype::Shieldbearer => ("shieldbearer", 2.0),
        Archetype::Wretch => ("wretch", 1.55),
        Archetype::Archer => ("archer", 1.85),
        Archetype::Hexer => ("hexer", 1.9),
        Archetype::Brute => ("brute", 2.5),
        Archetype::Hound => ("hound", 1.25),
        Archetype::Bloater => ("bloater", 1.95),
        Archetype::Gravecaller => ("gravecaller", 2.1),
    }
}

/// A full-object image of one colour (stretched to the object's size).
/// How a menu's choices are set (`Pilgrimage::draw_panel`), in canvas pixels: their font, size
/// and line height, the marker's column and the details' hanging indent, and the tab stops their
/// headings' parts line up on (the first, the marker's column).
struct MenuLook { font: Arc<Font>, size: f32, line: f32, mark: f32, detail: f32, tabs: Vec<f32>, spaced: bool }

impl MenuLook {
    /// The look for `options`, choices `size` tall on lines `line` times that: a stop after the
    /// widest part of each column of their first lines (split by tabs).
    fn new(font: Arc<Font>, size: f32, line: f32, options: &[(String, bool)]) -> MenuLook {
        let width = |t: &str| t.chars().map(|ch| font.0.metrics(ch, size).advance_width).sum::<f32>();
        let mark = 1.2 * width("> ");
        let heads: Vec<Vec<&str>> = options.iter().map(|o| o.0.lines().next().unwrap_or("").split('\t').collect()).collect();
        let mut tabs = vec![mark];
        for col in 0..heads.iter().map(Vec::len).max().unwrap_or(1).saturating_sub(1) {
            let widest = heads.iter().filter(|h| h.len() > col + 1).map(|h| width(h[col])).fold(0.0, f32::max);
            tabs.push(tabs[col] + widest + 1.2 * size);
        }
        let spaced = options.iter().any(|o| o.0.contains('\n'));
        MenuLook { mark, detail: mark + 0.9 * size, tabs, spaced, font, size, line }
    }

    /// How wide the widest first line lies, on its columns.
    fn widest(&self, options: &[(String, bool)]) -> f32 {
        let width = |t: &str| t.chars().map(|ch| self.font.0.metrics(ch, self.size).advance_width).sum::<f32>();
        options.iter().map(|o| {
            let parts: Vec<&str> = o.0.lines().next().unwrap_or("").split('\t').collect();
            self.tabs.get(parts.len() - 1).copied().unwrap_or(self.mark) + parts.last().map_or(0.0, |p| width(p))
        }).fold(0.0, f32::max)
    }

    /// One choice's spans: the marker when `picked`, its first line in `head` on the columns
    /// (wrapping under itself), and the lines after it under it on a hanging indent, smaller,
    /// in `detail` (in `warn` those `shrine::WARN` starts); a little room after, if any have details.
    fn choice(&self, label: &str, picked: bool, (head, detail, warn): (Color, Color, Color)) -> Vec<Span> {
        let span = |t: String, size: f32, line: f32, col: Color| Span::new(t, size, Some(size * line), self.font.clone(), col, 0.0);
        let mut lines = label.lines();
        let first = lines.next().unwrap_or("");
        let mut spans = Vec::new();
        // Marked, the first line follows the marker to its column; else it starts there.
        if picked { spans.push(span(">".into(), self.size, self.line, head)); }
        spans.push(span(format!("{}{first}\n", if picked { "\t" } else { "" }), self.size, self.line, head).indented(self.mark));
        for line in lines {
            let (line, col) = match line.strip_prefix(shrine::WARN) { Some(l) => (l, warn), None => (line, detail) };
            spans.push(span(format!("{}\n", line.trim_start()), 0.84 * self.size, 1.3, col).indented(self.detail));
        }
        if self.spaced { spans.push(span("\n".into(), 0.3 * self.size, 1.0, head)); }
        spans
    }

    /// A line saying `n` more choices are `way` (above or below), at the choices' column.
    fn more(&self, n: usize, way: &str, col: Color) -> Span {
        Span::new(format!("{n} more {way}\n"), 0.75 * self.size, Some(0.75 * self.size * 1.4), self.font.clone(), col, 0.0).indented(self.mark)
    }
}

/// Where fixed points on the road fall in its frame (`Pilgrimage::view_of`).
type View = [Option<(f32, f32)>; 3];

fn swatch(c: [u8; 4]) -> Arc<RgbaImage> { Arc::new(RgbaImage::from_pixel(1, 1, image::Rgba(c))) }

/// The canvas camera at `zoom`, centred on `at` (canvas units) as far as the canvas allows: a
/// cutscene closing in on a figure, the road and everything on it scaling together (the HUD is
/// screen-space and stays). 1 and the middle is the whole canvas, as the game plays it.
fn frame_canvas(c: &mut Canvas, zoom: f32, at: (f32, f32)) {
    let Some(cam) = c.camera_mut() else { return };
    let zoom = zoom.max(1.0);
    (cam.zoom, cam.zoom_target, cam.zoom_anchor) = (zoom, zoom, None);
    let (w, h) = (3840.0 / zoom, 2160.0 / zoom);
    cam.position = ((at.0 - 0.5 * w).clamp(0.0, 3840.0 - w), (at.1 - 0.5 * h).clamp(0.0, 2160.0 - h));
}

/// Set the scene up on `canvas`: the road, the hero, the enemies, the numbers, and the game.
pub fn setup(canvas: &mut Canvas) {
    // On a phone: rayon's pool (before anything else uses it) and the road's threads on the fast
    // cores, and this, the game's thread, on one of its own.
    quartz_path_forge::cores::use_fast_cores();
    quartz_path_forge::cores::pin_game();
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
    // Dev only: PathForge's per-stage frame report on a phone, which has no shell to set it in
    // (`adb shell setprop debug.hr.pf_profile 60`).
    if let Some(frames) = Pilgrimage::dev_prop("debug.hr.pf_profile", "PF_PROFILE") { std::env::set_var("PF_PROFILE", frames); }
    // With each GPU pass's own time (`setprop debug.hr.pf_gpu 1`).
    if let Some(on) = Pilgrimage::dev_prop("debug.hr.pf_gpu", "PF_PROFILE_GPU") { std::env::set_var("PF_PROFILE_GPU", on); }
    match PathForgePlugin::live(journey, ROAD, ROAD_PX) {
        // The road is pixel art scaled up eight times: hard pixels, like the sprites over it.
        // It measures its light along the road, so the figures on it are lit as it is.
        // On a phone it runs at an even 30 (each frame shown for two refreshes), with the frame
        // queued behind as headroom and the heat left for the long run; elsewhere up to 60.
        Ok(p) => {
            let p = p.with_speed(0.0).with_pixel_art(true).with_sprite_light();
            canvas.add_plugin(if cfg!(target_os = "android") { p.with_render_fps(30.0) } else { p })
        }
        Err(e) => eprintln!("hollow_road: the road would not open: {e}"),
    }
    // Never too dark to see what is coming (a place meant to hide its threats sets `keep_dark`).
    if let Some(pf) = canvas.get_plugin_mut::<PathForgePlugin>() { pf.set_visibility_floor(Some(VISIBILITY_FLOOR)); }

    let sprite = |id: &str, layer: i32| { let mut o = GameObject::build(id).size(1.0, 1.0).position(0.0, 0.0).layer(layer).finish(); o.unlit = true; o };
    canvas.add_game_object(HERO.to_owned(), sprite(HERO, 60));
    canvas.add_game_object(BOSS.to_owned(), sprite(BOSS, 40));
    let mut world = World::new(0.0);
    for i in 0..world::MOST_FOES { canvas.add_game_object(enemy_id(i), sprite(&enemy_id(i), 10)); }
    for i in 0..SHOTS { canvas.add_game_object(shot_id(i), sprite(&shot_id(i), 45)); }
    for i in 0..WAVES { canvas.add_game_object(wave_id(i), sprite(&wave_id(i), 44)); }
    for id in [SHRINE, SIGNPOST, STAIN, KEEPER, SMITH, GRUK, COUNTER, VISITOR, CHEST, VEIL] { canvas.add_game_object(id.to_owned(), sprite(id, 5)); }
    for i in 0..CAMEOS { canvas.add_game_object(cameo_id(i), sprite(&cameo_id(i), 5)); }
    // The slash in front of the hero, so behind them (60) and over every enemy (50 and under);
    // the hit bursts over everything on the road. A burst's object is a clear picture that only
    // carries its effect.
    // Under the hero: seen from behind them, the weapon and its trail are on their far side.
    canvas.add_game_object(fx::SLASH.to_owned(), sprite(fx::SLASH, 55));
    for i in 0..fx::BURSTS {
        let mut o = sprite(&fx::burst_id(i), 65);
        o.visible = false;
        o.set_image(Image { shape: ShapeType::Rectangle(0.0, (1.0, 1.0), 0.0), image: swatch([0, 0, 0, 0]), color: None, recolor: None });
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
    let mut foe_name = GameObject::build(FOE_NAME).size(1200.0, 80.0).position(2580.0, 62.0).layer(201).screen_space().finish();
    foe_name.visible = false;
    canvas.add_game_object(FOE_NAME.to_owned(), foe_name);
    for (id, x, y, w, h, c) in BARS {
        let mut o = GameObject::build(id).size(w, h).position(x, y).layer(201).screen_space().finish();
        o.unlit = true;
        o.set_image(Image { shape: ShapeType::Rectangle(0.0, (w, h), 0.0), image: swatch(c), color: None, recolor: None });
        canvas.add_game_object(id.to_owned(), o);
    }

    // The box for words and choices over the road, and the black for cuts over everything.
    for (id, layer) in [(PANEL_BG, 210), (FADE, 300)] {
        let mut o = GameObject::build(id).size(COVER.0, COVER.1).position(-CINE_OVER, -CINE_OVER).layer(layer).screen_space().finish();
        o.unlit = true;
        o.visible = id == FADE;
        o.set_image(Image { shape: ShapeType::Rectangle(0.0, COVER, 0.0), image: quartz::white_pixel(), color: Some(Color(0, 0, 0, if id == FADE { 255 } else { 0 })), recolor: None });
        canvas.add_game_object(id.to_owned(), o);
    }
    let panel = GameObject::build(PANEL).size(2240.0, 640.0).position(800.0, 1320.0).layer(211).screen_space().finish();
    canvas.add_game_object(PANEL.to_owned(), panel);
    // The touch controls (`draw_touch`): over the menus, under the fade. A button is a disc as it
    // rests and a brighter one held, swapped by showing one or the other (no picture changes
    // hands while a thumb is on it), and its word.
    let disc_objects = [(TOUCH_STICK, touch_disc(240, [14, 12, 18, 70], [236, 228, 210, 110]), 240), (TOUCH_KNOB, touch_disc(120, [236, 228, 210, 80], [255, 250, 236, 170]), 241)];
    for (id, art, layer) in disc_objects {
        let mut o = GameObject::build(id).size(1.0, 1.0).layer(layer).screen_space().finish();
        (o.unlit, o.visible) = (true, false);
        o.set_image(Image { shape: ShapeType::Rectangle(0.0, (1.0, 1.0), 0.0), image: art, color: None, recolor: None });
        canvas.add_game_object(id.to_owned(), o);
    }
    let (rest, held) = (touch_disc(160, [14, 12, 18, 120], [236, 228, 210, 200]), touch_disc(160, [200, 160, 80, 150], [255, 236, 180, 240]));
    for i in 0..crate::touch::Btn::ALL.len() {
        for (part, art) in [("ring", Some(rest.clone())), ("glow", Some(held.clone())), ("label", None)] {
            let id = touch_id(part, i);
            let mut o = GameObject::build(&id).size(1.0, 1.0).layer(if part == "label" { 243 } else { 242 }).screen_space().finish();
            (o.unlit, o.visible) = (true, false);
            if let Some(art) = art { o.set_image(Image { shape: ShapeType::Rectangle(0.0, (1.0, 1.0), 0.0), image: art, color: None, recolor: None }); }
            canvas.add_game_object(id, o);
        }
    }
    for (id, layer) in [(PROMPT_BG, 204), (PROMPT_KEY, 205), (PROMPT_E, 206), (PROMPT, 206)] {
        let mut o = GameObject::build(id).size(1.0, 1.0).position(0.0, PROMPT_Y).layer(layer).screen_space().finish();
        (o.unlit, o.visible) = (true, false);
        canvas.add_game_object(id.to_owned(), o);
    }
    let size = (crate::map::W as f32 * MAP_SCALE, crate::map::H as f32 * MAP_SCALE);
    let mut o = GameObject::build(MAP).size(size.0, size.1).position(MAP_AT.0, MAP_AT.1).layer(212).screen_space().finish();
    (o.unlit, o.visible) = (true, false);
    canvas.add_game_object(MAP.to_owned(), o);
    for i in 0..atlas::PLACES.len() {
        let mut o = GameObject::build(&map_label(i)).size(420.0, 60.0).layer(213).screen_space().finish();
        o.visible = false;
        canvas.add_game_object(map_label(i), o);
    }

    // A cutscene's bars over the HUD, its words over them, its white under the cut's black.
    for (id, layer) in [(CINE_TOP, 290), (CINE_BOTTOM, 290), (CINE_WHITE, 296), (CINE_SHADE, 291)] {
        let (w, h) = (3840.0 + 2.0 * CINE_OVER, 2160.0 + 2.0 * CINE_OVER);
        let mut o = GameObject::build(id).size(w, h).position(-CINE_OVER, -CINE_OVER).layer(layer).screen_space().finish();
        (o.unlit, o.visible) = (true, false);
        let img = match id {
            CINE_WHITE => swatch([255, 252, 244, 255]),
            // Dark in the middle, clear at the top and bottom.
            CINE_SHADE => Arc::new(RgbaImage::from_fn(1, 64, |_, y| { let u = (y as f32 + 0.5) / 64.0 * 2.0 - 1.0; image::Rgba([0, 0, 0, (200.0 * (1.0 - u * u).powf(1.5)) as u8]) })),
            _ => swatch([0, 0, 0, 255]),
        };
        o.set_image(Image { shape: ShapeType::Rectangle(0.0, (w, h), 0.0), image: img, color: None, recolor: None });
        canvas.add_game_object(id.to_owned(), o);
    }
    for (id, y, h) in [(CINE_CAPTION, 1910.0, 220.0), (CINE_TITLE, 1420.0, 260.0), (CINE_SUB, 1660.0, 110.0), (CINE_SKIP, 50.0, 70.0)] {
        let mut o = GameObject::build(id).size(3840.0, h).position(0.0, y).layer(292).screen_space().finish();
        o.visible = false;
        canvas.add_game_object(id.to_owned(), o);
    }

    let presses = Arc::new(Mutex::new(Vec::new()));
    let q = presses.clone();
    canvas.on_key_press(move |c, k| { let shift = shift_held(c); q.lock().unwrap().push(Press::Key(k.clone(), shift)); });
    let q = presses.clone();
    // A touch arrives as a finger first and then as the mouse; the finger is the control, so its
    // mouse twin is not a click (a strike).
    canvas.on_mouse_press(move |c, b, _| { if !c.touches().is_empty() { return; } let shift = shift_held(c); q.lock().unwrap().push(Press::Mouse(b, shift)); });
    let fingers = Arc::new(Mutex::new(Vec::new()));
    let f = fingers.clone();
    canvas.on_touch(move |_, id, phase, at| f.lock().unwrap().push((id, phase, at)));
    let loading = Loading::new(canvas);
    canvas.add_plugin(Pilgrimage::new(Fight::new(world.foes()), world, presses, fingers, loading));
}

/// A key or a click, as it came, with whether Shift was down.
#[derive(Clone, Debug)]
enum Press { Key(Key, bool), Mouse(MouseButton, bool), Touch(crate::touch::Tap) }

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

/// A menu's hint as a touch screen is worked: "W/S choose   E take   Esc back" is a tap on a
/// choice, "E continue" a tap, and so on.
fn for_touch(hint: &str) -> String {
    if let Some(rest) = hint.strip_prefix("W/S choose   E ") {
        return format!("Tap a choice to {}   BACK to go back", rest.split("   ").next().unwrap_or(rest));
    }
    match hint {
        "E or Enter to begin" => "Tap to begin".into(),
        "E continue" => "Tap to continue".into(),
        "M or Esc close" => "BACK to close".into(),
        "E leave   Esc stay" => "Tap to leave   BACK to stay".into(),
        "A/D move   E approach" => "Swipe to move   Tap to approach".into(),
        h => h.replacen("E go on", "Tap to go on", 1),
    }
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
    /// Stepping through the veil before a standing boss, `t` ticks in (`CROSS_T`), then its fight.
    Cross(u32),
    /// The world map open, the road standing still.
    Map,
    /// A cutscene on its own over the road (`Pilgrimage::scene`), and what follows it.
    Scene(Then),
}

/// The boss fight under way: the fight, ticks since it began (the camera swings round first),
/// and ticks since the boss fell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Arena { pub side: SideFight, pub t: u32, pub done: u32, pub why: Why, pub rune: u32 }

/// What a side-on fight is: the boss at the gate, one of the place's ambushes, or the prologue's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Why { Boss, Ambush(usize), Prologue }

/// What a fade to black leads to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Next { Start, Prologue, Resume, Sanctum, Road, Respawn, Detour(i8), Back, Turn, Opening(bool) }
/// What comes after a cutscene played on its own: a pilgrimage begun (through the prologue or
/// not), or the pilgrimage's end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Then { Begin(bool), Taken, Conclude }
/// What a cut shows once the road is there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum After { Sanctum, Rise, Wake, Walk, Prologue }

/// How a panel is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style { Title, Menu, Died, Lesson }

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
    /// Fingers' events since the last tick (`Canvas::on_touch`), the touch controls they work
    /// (`touch`), what those hold now, and where each row of the menu shown last stood (top,
    /// bottom; virtual units), the choice it is and whether a tap takes it (a line saying more are
    /// above or below only moves to them).
    fingers: Arc<Mutex<Vec<(u64, TouchPhase, (f32, f32))>>>,
    pad: crate::touch::Pad,
    touch_held: crate::touch::Held,
    menu_rows: RefCell<Vec<(f32, f32, usize, bool)>>,
    /// The first choice in view of a menu too long for the screen (`menu_window`).
    menu_top: std::cell::Cell<usize>,
    /// The touch controls shown whatever the device (`HR_TOUCH=1`, or a capture): on Android they
    /// always are, elsewhere from the first touch.
    pub(crate) touch_ui: bool,
    /// Whether the hero can turn round now (the TURN button shows only then).
    turnable: bool,
    /// The controls as last laid out (for what, at what window scale): their words and places
    /// are set again only when that changes.
    touch_laid: std::cell::Cell<Option<(crate::touch::Ui, f32)>>,
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
    pub(crate) ticks: u64,
    /// The tick the rune's pulse on the hero fades out by (`RUNE_PULSE`).
    hum_until: u64,
    /// Road frames shown in the last second, and the frame last seen (to count new ones).
    frames: (u32, u32),
    last_frame: Option<Arc<RgbaImage>>,
    /// The ticks the road's last four new frames came at, and how long a cut has waited for
    /// them to flow once its warp arrived (`road_flowing`).
    frame_at: [u64; 4],
    cut_wait: u32,
    /// The road's speed last tick (the hero strides while it moves).
    last_speed: f32,
    /// How far through a stride the hero's legs are (0..1): carried on by the road's speed, a
    /// walk's stride or a run's, so going from one gait into the other keeps the step.
    stride: f32,
    /// The warnings painted last, to repaint only on change.
    last_marks: Vec<GroundMark>,
    last_sigils: Vec<Sigil>,
    last_marks_view: View,
    /// Whether the marks last painted held a telegraph (so the next tick without one clears it).
    telegraphed: bool,
    /// Sprite frames lit by the road's light, and the shadows last painted with the frame they
    /// were painted for (`place` works from `&self`).
    relit: RefCell<Relighter>,
    /// Frames frosted over while a frost bolt's chill holds a foe, or turned to stone while Stone
    /// Skin holds the hero, by the frame and which (kept beside it, so its address is not reused
    /// while it is a key).
    frosted: RefCell<std::collections::HashMap<(usize, bool), (Arc<RgbaImage>, Arc<RgbaImage>)>>,
    /// The look of the capstones worn, laid on the hero's frame (`sheen::Sheen`).
    sheen: RefCell<crate::sheen::Sheen>,
    last_shadows: RefCell<(Vec<Shadow>, Vec<Cast>, View)>,
    /// Traces each sprite frame's outline from its pixels once, for the shadow it casts.
    tracer: RefCell<TerrainCollisionPlugin>,
    /// The Bell Warden's Pixellab art (also what a boss without its own is drawn as), and presses
    /// waiting for the side-on fight.
    boss_sheet: Option<Sheet>,
    /// The art of the boss of the place the hero is in, read on arriving there.
    boss_art: BossArt,
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
    /// Gruk's sheet, and his counter.
    gruk_sheet: Option<Sheet>,
    /// The sheet of whoever waits on the road here (`World::visitor_at`), and whose it is.
    visitor_sheet: (&'static str, Option<Sheet>),
    /// The side-on art of whoever was called to the boss here (`allies::Style::sprite`).
    ally_sheet: Vec<(&'static str, Sheet)>,
    /// The sheets of the people in the cutscene playing (`Cutscene::sheets`), read as it starts.
    cameos: Vec<(&'static str, Sheet)>,
    counter: Option<Arc<RgbaImage>>,
    /// The wall across the road's end last sent (`PathForgePlugin::set_end`), road metres.
    end_sent: Option<f64>,
    /// The banner's line and the real ticks it has left.
    pub(crate) banner: (String, u32),
    /// Lines waiting to show after the banner.
    banners: std::collections::VecDeque<String>,
    /// The callout's word, its colour and the real ticks it has left.
    callout: (String, [u8; 3], u32),
    /// Fight ticks left of a clean parry's follow-through (the buckler's flick held out), drawn
    /// while nothing else is under way: the parry itself ends at once so a riposte can follow.
    flourish: u32,
    /// The kit's shrine and signpost pictures.
    props: Option<(Arc<RgbaImage>, Arc<RgbaImage>)>,
    /// The stop and the gates last stood on the road's fork (`PathForgePlugin::bar`): each gated
    /// way, whether a key opens it, and whether it is open.
    bar_sent: Option<(String, [Option<(bool, bool)>; 2])>,
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
    /// The prologue under way, from its first lesson to its side-on bout (`prologue`).
    pub(crate) prologue: Option<Prologue>,
    /// The face for titles, banners and callouts (Cinzel); the plain one reads the rest.
    title_font: Option<Arc<Font>>,
    /// The art still being read (`Sheets::reading`), and the screen shown over the black while
    /// it is or while a cut waits on the road.
    art_due: Option<std::thread::JoinHandle<Sheets>>,
    loading: Loading,
    /// Past the start: the art read and the road drawing (or waited on long enough).
    started: bool,
    #[cfg(test)]
    loading_held: bool,
    /// What the prompt plate says, and how far it has faded in.
    prompt: (&'static str, f32),
    /// Whether the list of controls is shown (H), and the developer's numbers (F3); kept in
    /// `settings.json` beside the profiles' folder.
    show_help: bool,
    show_dev: bool,
    audio: Audio,
    /// The cutscene playing, over whatever the mode draws (`run_scene`).
    scene: Option<Cutscene>,
}

/// What the player chose to see, kept between launches.
fn settings_path(saves: &std::path::Path) -> std::path::PathBuf { saves.parent().unwrap_or(saves).join("settings.json") }
fn read_settings(saves: &std::path::Path) -> (bool, bool) {
    let v: serde_json::Value = std::fs::read_to_string(settings_path(saves)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    (v["show_controls"].as_bool().unwrap_or(false), v["show_dev"].as_bool().unwrap_or(false))
}
fn write_settings(saves: &std::path::Path, (help, dev): (bool, bool)) {
    let path = settings_path(saves);
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    if let Err(e) = std::fs::write(&path, serde_json::json!({ "show_controls": help, "show_dev": dev }).to_string()) { eprintln!("[hollow_road] could not keep settings: {e}"); }
}

/// How often the pilgrimage is looked at to be kept, ticks; a save that has not changed is
/// written again only for the time played, once a minute.
const KEEP_EVERY: u64 = 300;
const KEEP_IDLE: u64 = 3600;

/// Where the slots are kept: the player's own (`profile::dir`), or under the build's target for
/// a test, so a capture never touches a real pilgrimage.
fn save_dir() -> std::path::PathBuf {
    if cfg!(test) { return std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test_saves"); }
    // A bench keeps its own pilgrimages, never touching the player's.
    if Pilgrimage::bench().is_some() { profile::dir().join("bench") } else { profile::dir() }
}

/// An attack's warning colour, by how it can be met: gold for a parry, red for a blow that only
/// blocks, red-orange for one that breaks guards, purple for one only a roll escapes.
/// Metres back within which a chaser shows at the bottom of the screen, brightening as it nears.
const BEHIND_SHOWN: f32 = 12.0;

/// Enemies after the hero from behind, warned of where the road meets the bottom of the screen
/// (nothing behind the camera is in view): an ember glow rising from the edge in the chaser's
/// lane (`xs`, pixels), stronger the nearer it is, flaring and pulsing in its blow's colour as it
/// winds up. Dithered like the ground's telegraphs.
fn paint_behind(img: &mut RgbaImage, chasers: &[crate::fight::Chaser], xs: &[f32], time: f32) {
    const BAYER: [[f32; 4]; 4] = [[0.0, 8.0, 2.0, 10.0], [12.0, 4.0, 14.0, 6.0], [3.0, 11.0, 1.0, 9.0], [15.0, 7.0, 13.0, 5.0]];
    let (w, h) = (img.width() as f32, img.height() as f32);
    for (ch, &cx) in chasers.iter().zip(xs).take(3) {
        let near = (1.0 - ch.back / BEHIND_SHOWN).clamp(0.0, 1.0);
        if near <= 0.0 { continue; }
        let (rgb, strength) = match ch.blow {
            Some((kind, parry, progress)) => (warning_rgb(kind, parry), (0.8 + 0.3 * progress) * (0.85 + 0.15 * (time * 14.0).sin())),
            None => ([210, 72, 40], 0.4 + 0.5 * near),
        };
        let half = w * 0.2;
        let top = h * (1.0 - 0.1 - 0.14 * near.max(strength.min(1.0)));
        let (x0, x1) = ((cx - half).max(0.0) as u32, ((cx + half).min(w - 1.0)) as u32);
        for y in top.max(0.0) as u32..img.height() {
            let up = ((y as f32 - top) / (h - top)).clamp(0.0, 1.0).powf(1.6);
            for x in x0..=x1 {
                let across = 1.0 - ((x as f32 - cx) / half).powi(2);
                if across <= 0.0 { continue; }
                // Six steps of strength, dithered, the brightest of the glows kept.
                let a = (strength * up * across).min(1.0);
                let level = ((a * 6.0 + BAYER[(y % 4) as usize][(x % 4) as usize] / 16.0).floor() / 6.0).min(1.0);
                if level <= 0.0 { continue; }
                let p = img.get_pixel_mut(x, y);
                let alpha = (level * 230.0) as u8;
                if alpha > p[3] { *p = image::Rgba([rgb[0], rgb[1], rgb[2], alpha]); }
            }
        }
    }
}

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

/// Where a picture of something standing meets the ground, as a share of its height from the
/// top: its widest row in the lower part of what is drawn (a round plinth seen from above is
/// widest across its middle, where it lies on the ground; a flat base, along its foot). The
/// lowest such row when several tie, and the foot of what is drawn when nothing is.
fn ground_line(img: &RgbaImage) -> f32 {
    let (w, h) = img.dimensions();
    let width = |y: u32| -> u32 {
        let row = (0..w).filter(|&x| img.get_pixel(x, y)[3] > 16);
        match (row.clone().next(), row.last()) { (Some(a), Some(b)) => b - a + 1, _ => 0 }
    };
    let drawn: Vec<u32> = (0..h).filter(|&y| width(y) > 0).collect();
    let (Some(&top), Some(&foot)) = (drawn.first(), drawn.last()) else { return 1.0 };
    let from = foot - (foot - top) * 35 / 100;
    let widest = (from..=foot).max_by_key(|&y| (width(y), y)).unwrap_or(foot);
    (widest + 1) as f32 / h as f32
}

/// A still Pixellab picture (`assets/pixellab/<path>`), as pixel art.
fn pixellab_picture(path: &str) -> Option<Arc<RgbaImage>> {
    let img = Arc::new(image::open(crate::assets::path("pixellab").join(path)).ok()?.to_rgba8());
    set_pixel_art(&img, true);
    Some(img)
}

pub(crate) fn load_sheet(name: &str) -> Option<Sheet> {
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
    // The hand the Hatchet has left (`in_hand`): the axe's clips with nothing in them.
    let empty = crate::anim::Weapon { image: image::RgbaImage::new(1, 1), grip: (0.0, 0.0), tip: (1.0, 0.0) };
    s.arm(EMPTY_HAND, crate::gear::Class::Axe.slug(), &empty);
    s.arm(EMPTY_HAND, crate::gear::Class::Axe.family(), &empty);
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

/// The art name of an empty weapon hand (`armed_sheet`).
const EMPTY_HAND: &str = "empty_hand";

/// `kit` as drawn: with the weapon hand empty while its Hatchet is thrown.
fn in_hand(kit: &crate::gear::Kit, thrown: bool) -> crate::gear::Kit {
    if thrown { crate::gear::Kit { arms: crate::gear::Arms { art: EMPTY_HAND, ..kit.arms }, ..*kit } } else { *kit }
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
        crate::gear::Ranged::Frost | crate::gear::Ranged::BloodCrossbow => kitted(s, kit, &format!("{prefix}{}_cast", kit.swung_as().family())).map(|c| (c, (5, 7))),
        r => s.clip(&format!("{prefix}loose+{}", r.art())).map(|c| (c, (6, 8))),
    }
}

/// The clip `name` (`e_` side-on) with what is worn in hand: the bare-handed clip drawn for the
/// way the weapon is held (`<family>_<name>`, see `Class::family`) with weapon and offhand put in
/// (`kitted`) where there is one, else the clip as drawn (the Wayfarer's sword in it).
/// Road walked over one stride (two steps) of the hero's walk and of the run: the legs' reach
/// (tools/moveset/stances.py GAIT3) at the road's pace, a little long for so short a body.
const WALK_STRIDE: f32 = 1.4;
const RUN_STRIDE: f32 = 2.0;

fn worn<'a>(s: &'a Sheet, kit: &crate::gear::Kit, name: &str) -> Option<&'a Clip> {
    let (pre, rest) = name.strip_prefix("e_").map_or(("", name), |r| ("e_", r));
    kitted(s, kit, &format!("{pre}{}_{rest}", kit.swung_as().family())).or_else(|| s.clip(name))
}

/// The `<prefix><move>` clip as the weapon held swings it: its type's own clip with what is worn
/// in hand (`<prefix><type>_<move>`, `kitted`) where there is one, else the sword's.
fn held<'a>(s: &'a Sheet, kit: &crate::gear::Kit, prefix: &str, mv: &str) -> Option<&'a Clip> {
    kitted(s, kit, &format!("{prefix}{}_{mv}", kit.swung_as().slug())).or_else(|| s.clip(&format!("{prefix}{mv}")))
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
    fn new(mut fight: Fight, world: World, presses: Arc<Mutex<Vec<Press>>>, fingers: Arc<Mutex<Vec<(u64, TouchPhase, (f32, f32))>>>, loading: Loading) -> Pilgrimage {
        fight.hero.stats.kit = world.kit();
        // A test reads the art before its first tick, as it has no screen to wait behind.
        let (sheets, art_due) = if cfg!(test) { (Sheets::read(), None) } else { Sheets::reading().map_or_else(|read| (read, None), |due| (Sheets::default(), Some(due))) };
        let font = Font::from_bytes(include_bytes!("../assets/font.ttf")).ok().map(Arc::new);
        Pilgrimage {
            #[cfg(test)]
            log: Vec::new(),
            #[cfg(test)]
            staged: Vec::new(),
            #[cfg(test)]
            loading_held: false,
            fight, presses, fingers, pad: crate::touch::Pad::default(), touch_held: crate::touch::Held::default(), menu_rows: RefCell::new(Vec::new()), menu_top: std::cell::Cell::new(0),
            touch_ui: cfg!(target_os = "android") || std::env::var_os("HR_TOUCH").is_some(), turnable: false, touch_laid: std::cell::Cell::new(None), pending: Input::default(), owed: 0.0, slow: (0, 1.0), dead_for: 0,
            art: Art::new(), hero_sheet: sheets.hero, enemy_sheets: sheets.enemies, font, ticks: 0, hum_until: 0, frames: (0, 0), last_frame: None, frame_at: [0; 4], cut_wait: 0, last_speed: 0.0, stride: 0.0, last_marks: Vec::new(), last_sigils: Vec::new(), last_marks_view: [None; 3], telegraphed: false, relit: RefCell::new(Relighter::new()), frosted: RefCell::new(std::collections::HashMap::new()), sheen: RefCell::default(), last_shadows: RefCell::new((Vec::new(), Vec::new(), [None; 3])), tracer: RefCell::new(TerrainCollisionPlugin::new()), boss_sheet: sheets.bell_warden, boss_art: BossArt::default(), map_img: None, side_pending: SideInput::default(), tap: None, running: None, fx: Fx::load(),
            world, mode: Mode::Front(Front::Title), calling: None, fade: (1.0, 0.0), keeper_sheet: sheets.keeper, smith_sheet: sheets.smith, gruk_sheet: sheets.gruk, visitor_sheet: ("", None), ally_sheet: Vec::new(), cameos: Vec::new(), counter: pixellab_picture("guild/counter.png"), end_sent: None, art_due, loading, started: cfg!(test), banner: (String::new(), 0), banners: std::collections::VecDeque::new(), callout: (String::new(), [0; 3], 0), flourish: 0, props: kit_picture("shrine").zip(kit_picture("signpost")), bar_sent: None, chest: pixellab_picture("chest/shut.png").zip(pixellab_picture("chest/open.png")),
            slot: None, saves: save_dir(), slots: Vec::new(), played: 0.0, kept: None, keep_failed: false, prologue: None,
            title_font: Font::from_bytes(include_bytes!("../assets/fonts/Cinzel-Bold.ttf")).ok().map(Arc::new), prompt: ("", 0.0), show_help: false, show_dev: false, audio: Audio::new(), scene: None,
        }
        .listed()
    }

    /// A pilgrimage begun: whole, the world as at the start, at the levels everyone starts with
    /// and the calling's own (what the other hand holds, and where its levels lean).
    fn set_out(&mut self, walked: f64) {
        self.world = World::new(walked);
        self.fight = Fight::new(self.world.foes());
        self.fight.hero.stats = crate::fight::Stats::default();
        if let Some(c) = self.calling {
            self.world.gear.warden = c == Calling::Warden;
            if let Some(name) = c.offhand() {
                let i = crate::gear::find(name);
                self.world.gear.gain(i);
                self.world.gear.wear(1, Some(i));
            }
            let (st, [m, d, f]) = (&mut self.fight.hero.stats, c.leaning());
            (st.might, st.dexterity, st.focus) = (st.might + m, st.dexterity + d, st.focus + f);
        }
        self.fight.hero.stats.kit = self.world.kit();
        self.fight.hero = crate::fight::Hero { quiver: self.world.ammo, ..crate::fight::Hero::with(self.fight.hero.stats) };
    }

    /// The prologue's side-on bout: one husk, soft in the arm, the hero whole.
    fn prologue_bout(&self) -> Box<Arena> {
        let h = &self.fight.hero;
        let mut side = SideFight::ambush(h.stats, h.stats.max_hp(), h.stats.max_stamina(), h.flasks.max(1), &[(&side::HUSK, 3.0)]);
        (side.hero.quiver, side.hero.charges, side.hero.bulwark) = (h.quiver, h.charges, h.bulwark);
        side.foes = side.foes.into_iter().map(|f| f.scaled((1.0, 0.35))).collect();
        Box::new(Arena { side, t: 0, done: 0, why: Why::Prologue, rune: 0 })
    }

    /// With the slots read for the title, and the settings kept last time.
    fn listed(mut self) -> Pilgrimage {
        self.slots = profile::summaries(&self.saves);
        (self.show_help, self.show_dev) = read_settings(&self.saves);
        self
    }

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
        self.world.ammo = crate::fight::Quiver { imbued: 0, ..self.fight.hero.quiver };
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
        if self.running.is_some_and(|w| !way_held(c, w) && !self.touch_held.way(w)) { self.running = None; }
    }

    /// The rune's hum on the hero, traced from the hero's unlit frame: beating softly for a few
    /// seconds after it hums at a fork, fading as it goes; cleared otherwise.
    fn pulse_rune(&self, c: &mut Canvas, source: Option<Arc<RgbaImage>>) {
        let Some(o) = c.get_game_object_mut(HERO) else { return };
        let (rgb, peak, lasts, beat) = RUNE_PULSE;
        let left = self.hum_until.saturating_sub(self.ticks);
        match source.filter(|_| left > 0 && o.visible) {
            Some(frame) => {
                let wave = 0.5 - 0.5 * (self.ticks as f32 * std::f32::consts::TAU / beat).cos();
                let fade = (left as f32 / lasts as f32 * 3.0).min(1.0);
                o.set_outline_state_traced(&frame, MarkerMode::Aura, peak * (0.35 + 0.65 * wave) * fade, EffectColor::srgb8(rgb[0], rgb[1], rgb[2]), StateTiming::SETTLED);
            }
            None => o.clear_mega_fx(),
        }
    }

    /// The dread foes' aura, traced from each one's unlit frame (`auras`, by object), the
    /// engine's `MarkerMode::Aura`; cleared from every other enemy object, so a pooled object
    /// never carries one into its next foe.
    fn wreathe(c: &mut Canvas, auras: &[(String, Option<Arc<RgbaImage>>)]) {
        for (id, frame) in auras {
            let Some(o) = c.get_game_object_mut(id) else { continue };
            match frame {
                Some(f) if o.visible => { o.set_outline_state_traced(f, MarkerMode::Aura, DREAD_AURA.1, EffectColor::srgb8(DREAD_AURA.0[0], DREAD_AURA.0[1], DREAD_AURA.0[2]), StateTiming::SETTLED); }
                _ => o.clear_mega_fx(),
            }
        }
    }

    /// How a foe of `kind` that its region has made `variant` is recoloured as it is drawn (the
    /// engine's `Recolor`, on the GPU): a blighted one bruised violet in its shadows and sickly
    /// green in its lights, a dread one near black, its lights turned to ember red; both richer
    /// and harder in contrast than the plain, so they read as worse, not faded. A dread one also
    /// smoulders (`DREAD_AURA`). None for the plain.
    fn variant_look(kind: Archetype, variant: crate::fight::Variant) -> Option<Recolor> {
        use crate::fight::Variant;
        let amount = match kind { Archetype::Husk | Archetype::Wretch => 0.5, _ => 0.44 };
        // Pale skin, bone and grey-lit fur would glow red: graded darker.
        let pale = matches!(kind, Archetype::Brute | Archetype::Hound | Archetype::Bloater | Archetype::Gravecaller);
        let (amount, tone) = if pale { (0.6, (0.7, 1.25, 0.4)) } else { (amount, (0.8, 1.6, 0.8)) };
        // A bloater is plague-yellow already: turned the other way, toward rot green, not pink.
        let turn = if kind == Archetype::Bloater { 0.12 } else { -0.3 };
        match variant {
            Variant::Plain => None,
            // Hues turned (leather and rust to bruise purple, skin and steel toward rot green),
            // lightly graded so the kinds agree.
            Variant::Blighted => Some(Recolor::graded(Color(30, 10, 40, 255), Color(170, 220, 120, 255), 0.22).turned(turn).toned(1.3, 1.2, 0.9)),
            // Near black, its lights ember red, hard in contrast: fierce.
            Variant::Dread => Some(Recolor::graded(Color(6, 2, 4, 255), Color(230, 52, 32, 255), amount + 0.1).toned(tone.0, tone.1, tone.2)),
        }
    }

    /// A chilled foe's frame: paled toward ice, its shading kept, its edge rimed. Made once a frame.
    fn frosted(&self, img: Arc<RgbaImage>) -> Arc<RgbaImage> { self.tinted(img, false) }

    /// The hero's frame under Stone Skin: granite grey, its shading kept, flecked, its edge a dark
    /// seam. Made once a frame.
    fn stoned(&self, img: Arc<RgbaImage>) -> Arc<RgbaImage> { self.tinted(img, true) }

    fn tinted(&self, img: Arc<RgbaImage>, stone: bool) -> Arc<RgbaImage> {
        let mut made = self.frosted.borrow_mut();
        if made.len() > 256 { made.clear(); }
        made.entry((Arc::as_ptr(&img) as usize, stone)).or_insert_with(|| {
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
                // Granite: a fleck from the pixel's place, so it stays put on the figure.
                let fleck = (((x * 73_856_093) ^ (y * 19_349_663)) & 15) as f32 - 7.5;
                let (ice, k) = match (stone, edge) {
                    (false, true) => ([225.0, 245.0, 255.0], 0.8),
                    (false, false) => ([0.55 * l + 60.0, 0.75 * l + 70.0, 0.85 * l + 110.0], 0.55),
                    (true, true) => ([58.0, 55.0, 52.0], 0.75),
                    (true, false) => ([0.8 * l + 34.0 + fleck * 2.0, 0.78 * l + 32.0 + fleck * 2.0, 0.74 * l + 30.0 + fleck * 2.0], 0.72),
                };
                let mix = |c: u8, i: f32| (c as f32 * (1.0 - k) + i.min(255.0) * k) as u8;
                p.0 = [mix(r, ice[0]), mix(g, ice[1]), mix(b, ice[2]), a];
            }
            (img.clone(), Arc::new(out))
        }).1.clone()
    }

    /// This tick's presses joined to any still waiting; held keys and buttons as they are now.
    /// A and D step a lane across, and held keep stepping; with Shift, W, A, S and D roll that
    /// way; W twice, held, runs.
    fn input(&mut self, c: &Canvas, presses: &[Press]) -> Input {
        self.taps(c, presses);
        let th = self.touch_held;
        let run = self.running == Some('w') || (th.run && th.up);
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
                    if is(k, &["q"], None) { p.art = true; }
                }
                Press::Mouse(MouseButton::Left, false) => p.light = true,
                Press::Mouse(MouseButton::Left, true) => p.heavy = true,
                Press::Mouse(MouseButton::Right, true) => p.parry = true,
                Press::Mouse(..) => {}
                // A roll goes the way the stick leans: a lane, on ahead, or back.
                Press::Touch(t) => {
                    use crate::touch::Tap;
                    match t {
                        Tap::Light => p.light = true, Tap::Heavy => p.heavy = true, Tap::Parry => p.parry = true,
                        Tap::Flask => p.flask = true, Tap::Loose => p.loose = true, Tap::Cast => p.cast = true, Tap::Art => p.art = true,
                        Tap::Roll if th.left => p.step = -1,
                        Tap::Roll if th.right => p.step = 1,
                        Tap::Roll if th.up => p.tumble = true,
                        Tap::Roll => p.backstep = true,
                        Tap::Jump => {}
                    }
                }
            }
        }
        // A lane held goes on to the next once its step is done, as far as the road goes, by key
        // or stick alike (the stick presses a way once however long it leans).
        let held = if way_held(c, 'a') || th.left { -1 } else if way_held(c, 'd') || th.right { 1 } else { 0 };
        if held != 0 && p.lane == 0 && p.step == 0 && !shift_held(c) && self.fight.hero.act.is_none() { p.lane = held; }
        Input {
            forward: way_held(c, 'w') || th.up,
            back: way_held(c, 's') || th.down,
            run,
            guard: (c.is_mouse_held(MouseButton::Right) && !shift_held(c)) || th.guard,
            ..*p
        }
    }

    /// Hitstop, slow motion, shake and flashes for what the fight did. A flash is a brief tint,
    /// never a wash: the sparks and the callout carry the moment, and the fight stays readable.
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
                    c.run(Action::CameraFlash { color: Color(255, 236, 190, 30), duration: 0.1 });
                    self.call("PARRIED", [255, 214, 110]);
                    self.flourish = FLOURISH_T;
                }
                // A last-moment block on a bulwark: a beat held, and if the ward goes, a white-gold
                // flash and a heavy shake as it bursts into the attacker.
                Event::Art { art } => {
                    use crate::arts::WeaponArt;
                    let big = matches!(art, WeaponArt::Toll | WeaponArt::StoneSkin | WeaponArt::Reap | WeaponArt::Wingbeat);
                    c.hitstop(if big { 8 } else { 4 });
                    c.run(Action::CameraShake { intensity: if big { 40.0 } else { 18.0 }, duration: 0.25 });
                    c.run(Action::CameraZoomPunch { amount: if big { 0.05 } else { 0.025 }, duration: 0.3 });
                    let (r, g, b) = art_rgb(art);
                    c.run(Action::CameraFlash { color: Color(r, g, b, 24), duration: 0.12 });
                    self.call(&art.name().to_uppercase(), [r, g, b]);
                }
                // The bell rung on the stones: the yard jolts, a faint bronze flash.
                Event::Rung { .. } => {
                    c.run(Action::CameraShake { intensity: 42.0, duration: 0.4 });
                    c.run(Action::CameraZoomPunch { amount: 0.025, duration: 0.3 });
                    c.run(Action::CameraFlash { color: Color(255, 214, 160, 22), duration: 0.15 });
                }
                Event::Slam { heavy, .. } => c.run(Action::CameraShake { intensity: if heavy { 30.0 } else { 16.0 }, duration: 0.2 }),
                Event::Sentenced { .. } => {
                    c.hitstop(14);
                    self.slow = (24, 0.5);
                    c.run(Action::CameraShake { intensity: 54.0, duration: 0.35 });
                    c.run(Action::CameraZoomPunch { amount: 0.06, duration: 0.35 });
                    self.call("SENTENCED", [255, 214, 110]);
                }
                Event::Hit { mv: Move::Art, .. } => { c.hitstop(6); c.run(Action::CameraShake { intensity: 28.0, duration: 0.18 }); }
                Event::PerfectBlock => {
                    c.hitstop(6);
                    self.slow = (18, 0.5);
                    self.call("PERFECT BLOCK", [236, 226, 196]);
                }
                Event::WardBurst { power, .. } => {
                    c.hitstop(12);
                    self.slow = (30, 0.4);
                    c.run(Action::CameraFlash { color: Color(255, 240, 200, (14.0 + 24.0 * power) as u8), duration: 0.12 });
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
                    c.run(Action::CameraFlash { color: Color(160, 0, 0, 32), duration: 0.1 });
                }
                Event::Struck { heavy: true, .. } => {
                    c.hitstop(7);
                    c.run(Action::CameraShake { intensity: 42.0, duration: 0.26 });
                    c.run(Action::CameraFlash { color: Color(170, 0, 0, 64), duration: 0.2 });
                }
                Event::Staggered { .. } => {
                    c.hitstop(9);
                    c.run(Action::CameraShake { intensity: 50.0, duration: 0.34 });
                    c.run(Action::CameraFlash { color: Color(190, 70, 0, 60), duration: 0.22 });
                    self.call("STAGGERED", [255, 170, 70]);
                }
                Event::KnockedDown { .. } => {
                    c.hitstop(11);
                    c.run(Action::CameraShake { intensity: 72.0, duration: 0.45 });
                    c.run(Action::CameraFlash { color: Color(150, 0, 0, 90), duration: 0.3 });
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
                    c.run(Action::CameraFlash { color: Color(255, 140, 40, 24), duration: 0.1 });
                    self.call(&format!("BLOCKED  -{}", chip.round().max(1.0)), [255, 176, 96]);
                }
                Event::Deflected { .. } => { c.hitstop(5); c.run(Action::CameraShake { intensity: 22.0, duration: 0.15 }); }
                Event::GuardBroken { .. } => {
                    c.hitstop(8);
                    c.run(Action::CameraShake { intensity: 44.0, duration: 0.3 });
                    c.run(Action::CameraFlash { color: Color(255, 120, 30, 50), duration: 0.18 });
                    self.call("GUARD BROKEN", [255, 140, 50]);
                }
                Event::ParryMissed { .. } => {
                    c.hitstop(5);
                    c.run(Action::CameraShake { intensity: 30.0, duration: 0.2 });
                    c.run(Action::CameraFlash { color: Color(110, 100, 100, 40), duration: 0.12 });
                    self.call("MISSED PARRY", [200, 150, 150]);
                }
                Event::Died => c.run(Action::CameraFlash { color: Color(60, 0, 0, 200), duration: 1.5 }),
                _ => {}
            }
        }
    }

    /// Show `word` over the hero's head for a moment, in `rgb`.
    fn call(&mut self, word: &str, rgb: [u8; 3]) { self.callout = (word.to_owned(), rgb, CALLOUT_T); }

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

    /// The marks on the road that are not blows: at a fork, a trail up the side the hero leans
    /// toward.
    fn marks(&self, walked: f64) -> Vec<GroundMark> {
        if matches!(self.mode, Mode::Side(_)) { return Vec::new(); }
        self.world.signpost.filter(|_| !self.world.chose).zip(self.world.lean).map(|(sign, b)| {
            let side = match b { quartz_path_forge::Branch::Left => -1.0, quartz_path_forge::Branch::Right => 1.0 };
            let to = ((sign - walked) as f32 - world::SIGNPOST_PAST).max(HERO_D + 1.0);
            GroundMark { x: (side * LANE_W - 0.35 * LANE_W, side * LANE_W + 0.35 * LANE_W), d: (HERO_D + 0.8, to), color: [240, 214, 130, 70], fill: Fill::Dots, edge: None }
        }).into_iter().collect()
    }

    /// The telegraph of every blow under way: where it will land, lit in how it is met (gold:
    /// parry it; red-orange: it breaks guards, so roll or pay to block; purple: only a roll), shaped
    /// by what it is (a blade down the lane, a crescent across lanes, a crackling oval nothing
    /// stops), with a front of light running from the attacker that lands as the blow does.
    fn telegraphs(&self, walked: f64) -> Vec<Telegraph> {
        let time = self.ticks as f32 / 60.0;
        if let Mode::Side(a) = &self.mode {
            return a.side.foes.iter().filter_map(|f| Some((f.facing, f.blow()?.0.motion, f.warning()?))).map(|(facing, motion, (from, to, kind, parry, progress))| Telegraph {
                x: (-1.1, 1.1), d: (ARENA_D + from, ARENA_D + to), rgb: warning_rgb(kind, parry),
                // What comes down on a spot has no way to it; the rest point at the hero (seen
                // side-on, a blade's chevrons point along the road at them).
                blow: match motion {
                    side::Motion::Ring | side::Motion::Leap | side::Motion::Rune => Blow::Area,
                    _ if kind == Kind::Unblockable => Blow::Crush,
                    _ => Blow::Chop,
                },
                toward: if facing > 0.0 { 1.0 } else { -1.0 }, progress, live: progress >= 1.0, fade: 1.0, time,
            }).collect();
        }
        let threats = self.fight.threats(walked);
        // The last of them are arrows and hexes in flight: charged all the way, never a flash.
        let swung = threats.len().saturating_sub(self.fight.missiles.len());
        threats.iter().enumerate().map(|(k, t)| {
            // Short of the lane's edges, so a gap shows which lane is safe.
            let half = 0.42 * LANE_W;
            let blow = match t.kind {
                _ if t.area => Blow::Area,
                Kind::Physical => Blow::Chop,
                // Across, from the attacker's own lane toward the one it reaches into.
                Kind::GuardBreak => Blow::Sweep(if self.fight.enemies.get(t.enemy).is_some_and(|e| e.lane > t.lanes.0) { -1.0 } else { 1.0 }),
                Kind::Unblockable => Blow::Crush,
            };
            let flying = k >= swung;
            Telegraph {
                x: (t.lanes.0 as f32 * LANE_W - half, t.lanes.1 as f32 * LANE_W + half), d: t.d, rgb: warning_rgb(t.kind, t.parry), blow, toward: -1.0,
                progress: if flying { 1.0 } else { t.progress }, live: !flying && t.phase == Phase::Active,
                fade: if t.phase == Phase::Recover { 0.3 } else { 1.0 }, time,
            }
        }).collect()
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
        let telegraphs = self.telegraphs(walked);
        let chasers = if self.mode == Mode::Play { self.fight.chasers(walked) } else { Vec::new() };
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // Repainted when the view moves (not with every frame of a road that animates in place);
        // a telegraph moves every tick.
        let view = Self::view_of(pf);
        let same_frame = view == self.last_marks_view;
        let none = marks.is_empty() && sigils.is_empty() && telegraphs.is_empty() && chasers.is_empty();
        if telegraphs.is_empty() && chasers.is_empty() && marks == self.last_marks && sigils == self.last_sigils && (same_frame || none) && !self.telegraphed { return; }
        self.telegraphed = !telegraphs.is_empty() || !chasers.is_empty();
        // A fork's trail alone is a decal the GPU fills; blows' warnings are still painted.
        let decal = (!marks.is_empty() && sigils.is_empty() && telegraphs.is_empty() && chasers.is_empty()).then(|| pf.ground_decal(&marks)).flatten();
        let mut img = if none || decal.is_some() { None } else { pf.paint_warnings(&marks, &sigils, &telegraphs) };
        if let Some(img) = img.as_mut() {
            let lane_x = |lane: i32| pf.project(lane as f32 * LANE_W, 0.0, 1.0).map_or(img.width() as f32 * (0.5 + 0.3 * lane as f32), |p| p.0);
            let xs: Vec<f32> = chasers.iter().map(|ch| lane_x(ch.lane)).collect();
            paint_behind(img, &chasers, &xs, self.ticks as f32 / 60.0);
        }
        self.last_marks = marks;
        self.last_sigils = sigils;
        self.last_marks_view = view;
        let Some(o) = c.get_game_object_mut(MARKS) else { return };
        if let Some(mut decal) = decal {
            o.visible = true;
            decal.size = o.scaled_size.get();
            o.set_drawable(Box::new(decal));
            return;
        }
        match img {
            Some(img) => {
                let img = Arc::new(img);
                set_pixel_art(&img, true);
                o.visible = true;
                let size = o.size;
                o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None, recolor: None });
            }
            None => o.visible = false,
        }
    }

    /// The hero's frame now, from their Pixellab animations (seen from straight behind,
    /// as the camera sees them); standing still where a clip is missing.
    fn hero_frame(&self, s: &Sheet, walked: f64) -> Option<Arc<RgbaImage>> {
        // Standing is the first frame of the walk, with what is worn in hand.
        let kit = &in_hand(&self.fight.hero.stats.kit, !self.fight.hatchets.is_empty());
        let stand = || ["walk", "light", "guard"].iter().find_map(|n| worn(s, kit, n)).map(|c| c.frames[0].clone());
        self.hero_clip_frame(s, walked).or_else(stand)
    }

    /// Draw the offhand worn onto the hero's clips if it is not yet (`Sheet::dress`, once a pair):
    /// what the loadout holds, and what the hero fighting holds (drawn from), should they differ.
    fn dress_hero(&mut self) {
        let Some(s) = self.hero_sheet.as_mut() else { return };
        for kit in [self.world.kit(), self.fight.hero.stats.kit] {
            // Held in one hand, a weapon made for both is drawn into the one-handed clips.
            if kit.one_hand {
                s.arm_as(kit.arms.art, kit.swung_as().slug());
                s.arm_as(kit.arms.art, kit.swung_as().family());
            }
            if let Some(o) = off_art(&kit) {
                s.dress(kit.arms.art, o);
                if kit.art == Some(crate::arts::WeaponArt::Hatchet) { s.dress(EMPTY_HAND, o); }
            }
        }
    }

    /// Where three points on the road fall in the frame: the same while the road's camera has
    /// not moved, however its picture animates.
    fn view_of(pf: &PathForgePlugin) -> View { [(0.0, 0.0, 2.0), (1.5, 0.0, 6.0), (0.0, 2.0, 12.0)].map(|(x, y, d)| pf.project(x, y, d)) }

    /// The road's pace `dt` seconds on from the last, toward `target`: picking up over about a
    /// fifth of a second to a walk, settling quicker than that.
    fn eased_walk(&self, target: f32, dt: f32) -> f32 {
        const PICK_UP: f32 = 11.0;
        const SETTLE: f32 = 16.0;
        let from = self.last_speed;
        let gaining = target * from >= 0.0 && target.abs() > from.abs();
        let most = if gaining { PICK_UP } else { SETTLE } * dt;
        from + (target - from).clamp(-most, most)
    }

    fn hero_clip_frame(&self, s: &Sheet, walked: f64) -> Option<Arc<RgbaImage>> {
        let h = &self.fight.hero;
        let kit = in_hand(&h.stats.kit, !self.fight.hatchets.is_empty());
        let clip = |n: &str| worn(s, &kit, n);
        let k = |t: u32, total: u32| t as f32 / total.max(1) as f32;
        let kneel = |k: f32| clip("rest").map(|c| c.at(k, false).clone());
        let fall = |k: f32| clip("death").map(|c| c.at(k, false).clone());
        // A parry is the shield's flick out with one on the arm, else the weapon's up and back
        // (whatever else the free hand holds): the clip, and its live frames.
        let parry = || match kit.off {
            Some(_) => clip("parry").map(|c| (c, (3, 6))),
            None => clip("parry_weapon").map(|c| (c, (2, 5))).or_else(|| clip("parry").map(|c| (c, (3, 6)))),
        };
        match self.mode {
            Mode::Kneel(t) => return kneel(k(t, KNEEL_T)),
            Mode::Rise(t) => return kneel(1.0 - k(t, RISE_T)),
            Mode::Out(Next::Sanctum) | Mode::Cut(After::Sanctum | After::Rise) => return kneel(1.0),
            // Getting up off the ground: the getup where there is one, else the fall played backwards.
            Mode::Wake(t) => return clip("getup").map(|c| c.at(k(t, WAKE_T), false).clone()).or_else(|| fall(1.0 - k(t, WAKE_T))),
            Mode::Out(Next::Respawn | Next::Start) | Mode::Cut(After::Wake) => return fall(1.0),
            // Off into the side path: turned side-on and walking that way (the side-on walk,
            // mirrored when it leaves on the left), else the walk away.
            Mode::Peek(t, side) => return clip("e_walk").or_else(|| clip("walk")).map(|c| c.at((t as f32 / 36.0).rem_euclid(1.0), side < 0).clone()),
            Mode::Out(Next::Detour(side)) => return clip("e_walk").or_else(|| clip("walk")).map(|c| c.at(((PEEK_T as f32 + 24.0 * self.fade.0) / 36.0).rem_euclid(1.0), side < 0).clone()),
            Mode::Sanctum(_) => {
                let target = self.fight.hero.lane as f32 * BEFORE_X;
                // Crossing to a place in the sanctum: side-on, walking that way; there, the stand.
                if (target - h.x).abs() > 0.05 {
                    let k = (self.ticks as f32 / 36.0).rem_euclid(1.0);
                    return clip("e_walk").or_else(|| clip("walk")).map(|c| c.at(k, target < h.x).clone());
                }
                return clip("walk").map(|c| c.frames[0].clone());
            }
            _ => {}
        }
        // Falling takes a second, then they lie there until the road starts again.
        if h.dead { return clip("death").map(|c| c.at(self.dead_for as f32 / 60.0, false).clone()); }
        if let Some(a) = h.act {
            let spec = a.spec;
            let (w, b, r) = spec.frames;
            return match a.mv {
                // The blow's frames, read off the contact sheet (tools/moveset PHASES): the light's blade is out
                // on 5 and 6, the heavy comes down on 7 and 8, the buckler flashes on 3 to 5.
                Move::Light => held(s, &kit, "", "light").map(|c| phased(c, spec.frames, a.t, (5, 7), false)),
                Move::Heavy | Move::Riposte => held(s, &kit, "", "heavy").map(|c| phased(c, spec.frames, a.t, (7, 9), false)),
                Move::Parry => parry().map(|(c, keys)| phased(c, spec.frames, a.t, keys, false)),
                // Over backwards toward the camera: down into the knees in the wind-up, up off them
                // arms first and heels over head through the live ticks, landing in a crouch for
                // the recovery (`flip_back`). (Else the old roll back, or the hop.)
                Move::Backstep => clip("flip_back").map(|c| phased(c, spec.frames, a.t, (3, 8), false))
                    .or_else(|| clip("roll_back").map(|c| phased(c, spec.frames, a.t, (3, 9), false)))
                    .or_else(|| clip("backstep").or_else(|| clip("hop")).map(|c| c.at(k(a.t, w + b + r), false).clone())),
                // The roll (side-on, a dip, over the shoulder a full turn, a crouch, back) turns the
                // way it travels: drawn going right, mirrored going left. The turn fills the travel.
                Move::Step(d) => clip("roll_side").or_else(|| clip("roll_spun")).or_else(|| clip("roll"))
                    .map(|c| phased(c, (3, TRAVEL - 3, w + b + r - TRAVEL), a.t, s.roll_keys, d < 0)),
                // A plain step across: a quick walk that way (mirrored going left).
                Move::Lane(d) => clip("walk").map(|c| c.at(k(a.t, w + b + r), d < 0).clone()),
                // A somersault away down the road: the dive in the first ticks, head over heels
                // while it travels, the crouch and rise after. (Else the old low dodge.)
                Move::Tumble => clip("roll_forward").map(|c| phased(c, (3, TRAVEL - 3, w + b + r - TRAVEL), a.t, (3, 9), false))
                    .or_else(|| clip("roll").map(|c| phased(c, (3, TRAVEL - 3, w + b + r - TRAVEL), a.t, (2, 8), false))),
                Move::Flask => clip("drink").map(|c| c.at(k(a.t, w + b + r), false).clone()),
                // The draw and the release, or the cast: its own clip, held at full draw until it
                // looses; without one, the other hand raised as for a parry.
                Move::Loose | Move::Cast => loosing(s, &kit, "", a.mv == Move::Cast).map(|(c, keys)| phased(c, spec.frames, a.t, keys, false))
                    .or_else(|| clip("parry").map(|c| phased(c, spec.frames, a.t, (3, 6), false))),
                // A weapon art: the weapon's own swing (its light for a throw or a scream, else its
                // heavy), stretched over the art's timing so the blow lands as it goes off.
                Move::Art if kit.art.is_some_and(|a| a.light()) => held(s, &kit, "", "light").map(|c| phased(c, spec.frames, a.t, (5, 7), false)),
                Move::Art => held(s, &kit, "", "heavy").map(|c| phased(c, spec.frames, a.t, (7, 9), false)),
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
                Reel::Fumble => if k < 0.35 { parry().map(|(c, _)| at(c, 0.62)) } else { hurt(0.25, 1.0, (k - 0.35) / 0.65) },
                // The blade comes back off the shield: the light's swing, run backwards.
                Reel::Bounce => held(s, &kit, "", "light").map(|c| at(c, 0.62 * (1.0 - k))),
            };
        }
        if self.flourish > 0 {
            // The flick out and held, then back in.
            let k = 1.0 - self.flourish as f32 / FLOURISH_T as f32;
            return parry().map(|(c, _)| c.at((0.45 + 0.5 * k).min(0.999), false).clone());
        }
        if h.guarding { return clip("guard").map(|c| c.at(1.0, false).clone()); }
        // Walking or running strides with the road (`stride`); standing is the walk's first frame.
        if self.fight.speed(&self.pending, walked) != 0.0 || self.last_speed != 0.0 {
            let gait = if self.last_speed > 0.5 * (crate::fight::WALK + crate::fight::RUN) { clip("run").or_else(|| clip("walk")) } else { clip("walk") };
            if let Some(c) = gait { return Some(c.at(self.stride, false).clone()); }
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
                        // The maul lands on 4 (and the ground bursts under the stamp's); the
                        // sweep crosses on 4 to 6.
                        (Archetype::Brute, "slam" | "stamp") => (4, 6),
                        (Archetype::Brute, "sweep") => (4, 7),
                        // The jaws are out on 3 and 4; the pounce rears at the viewer on 5 and 6.
                        (Archetype::Hound, "bite") => (3, 5),
                        (Archetype::Hound, "pounce") => (5, 7),
                        // The bile leaves the mouth on 5 and sprays wide on 6 and 7; the belly
                        // comes down on 3 and 4.
                        (Archetype::Bloater, "belch") => (5, 8),
                        (Archetype::Bloater, "slam") => (3, 5),
                        // The grave-mist rolls out at its feet on 6 to 8; the hand reaches on 3 to
                        // 6; the staff's arc crosses on 3 to 5.
                        (Archetype::Gravecaller, "raise") => (6, 9),
                        (Archetype::Gravecaller, "grasp") => (3, 6),
                        (Archetype::Gravecaller, "swipe") => (3, 6),
                        _ => (n * 5 / 10, (n * 7 / 10).max(n * 5 / 10 + 1)),
                    };
                    phased(c, a.frames, t, keys, false)
                }).or_else(first)
            }
            Ai::Staggered(n) => clip("stagger").map(|c| c.looped((150 - n) as f32, 60.0).clone()).or_else(first),
            // A stumble plays the stagger once, quicker, and holds its last drawing.
            Ai::Stumbled(n) => clip("stagger").map(|c| c.at(((STUMBLE - n) as f32 / 40.0).min(1.0), false).clone()).or_else(first),
            _ if e.hurt > 0 => clip("hurt").map(|c| c.at(1.0 - e.hurt as f32 / 12.0, false).clone()).or_else(first),
            // A felled bloater swells and bursts through its death clip; the rest lie as they fell.
            Ai::Dead if e.grave.swell > 0 => clip("death").map(|c| c.at(e.grave.swell as f32 / crate::fight::BURST_END as f32, false).clone()),
            Ai::Dead => clip("death").map(|c| c.frames[c.frames.len() - 1].clone()),
            _ => {
                // Each its own time through its idle, so a group never breathes as one.
                let own = e.idle.map_or(((e.s * 37.0) as i64).unsigned_abs() as f32, |i| (i.seed % 997) as f32);
                let idle = || clip("idle").map(|c| c.looped(self.ticks as f32 + own, 90.0).clone()).or_else(first);
                // Unaware: walking across the road side-on (its west-facing walk, turned to go
                // right), or standing side-on looking along it; once it notices, it faces the hero.
                let side = |moving: bool, facing: f32| clip("w_walk").map(|c| if moving { c.at(((self.ticks as f32 + own) / 54.0).rem_euclid(1.0), facing > 0.0).clone() } else { c.at(0.0, facing > 0.0).clone() });
                match e.idle {
                    Some(Idle { way: Way::Pace, wait: 0, facing, .. }) => side(true, facing).or_else(idle),
                    Some(Idle { way: Way::Pace, facing, .. }) => side(false, facing).or_else(idle),
                    // A watcher faces the road, glancing along it for a moment every so often.
                    Some(Idle { way: Way::Watch, wait, facing, .. }) if wait > LINGER_GLANCE => side(false, facing).or_else(idle),
                    _ => idle(),
                }
            }
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
        let place_at = |x: f32, d: f32, metres: f32, img: &RgbaImage, ground: f32| -> Option<((f32, f32), (f32, f32))> {
            let (fx, fy) = pf.project_on(c, x, 0.0, d)?;
            let ppm = pf.scale_on(c, d)?;
            let h = metres * ppm;
            let w = h * img.width() as f32 / img.height() as f32;
            Some(((fx - 0.5 * w, fy - ground * h), (w, h)))
        };
        let at = |x: f32, d: f32, metres: f32, img: &RgbaImage| place_at(x, d, metres, img, 1.0);
        // A prop's picture stands with its ground line on the ground there (`ground_line`).
        let on_ground = |x: f32, d: f32, metres: f32, img: &RgbaImage| place_at(x, d, metres, img, ground_line(img));
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
        let shown = !matches!(self.mode, Mode::Scene(_)) || self.scene.as_ref().is_some_and(Cutscene::hero);
        let mut pulse_source = None;
        let hero = match self.hero_sheet.as_ref().and_then(|s| Some((s, self.hero_frame(s, walked)?))) {
            _ if !shown => None,
            Some((s, img)) => {
                pulse_source = Some(img.clone());
                let glint = s.glint(&img);
                let img = if h.stone > 0 && !h.dead { self.stoned(img) } else { img };
                let worn = h.stats.kit.capped.worn;
                (!h.dead || s.clip("death").is_some()).then(|| sheet_at(s, hx, hd, art::HERO_METRES)).flatten()
                    .map(|r| (r, self.sheen.borrow_mut().dress(lit(r, img, hx, hd), glint.as_ref(), &worn, self.ticks), 60))
            }
            None => {
                let img = &self.art.hero[pose];
                (!h.dead).then(|| at(hx, HERO_D, art::HERO_METRES, img)).flatten().map(|r| (r, lit(r, img.clone(), hx, HERO_D), 60))
            }
        };
        // Lying down the hero's shadow is as long as they are and low.
        if hero.is_some() { stands.push((hx, hd, if h.floored() { 0.5 } else { art::HERO_METRES }, 0.55)); }
        TMP_LAP.with(|l| l.borrow_mut().mark("p.hero"));
        // The callout rides over the hero's head.
        let head = hero.as_ref().map(|(((x, y), (w, _)), _, _)| (x + 0.5 * w - 900.0, (y - 150.0).max(240.0)));
        let mut placed: Vec<(String, Option<(Rect, Arc<RgbaImage>, i32)>)> = vec![(HERO.to_owned(), hero), (BOSS.to_owned(), None)];
        let mut auras: Vec<(String, Option<Arc<RgbaImage>>)> = Vec::new();
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
            let x = e.lane as f32 * LANE_W + e.drift + sway;
            let metres = kind_art(e.kind).1;
            let sheet = |k: Archetype| self.enemy_sheets.iter().find(|(s, _)| *s == k).map(|(_, s)| s);
            let drawn = sheet(e.kind).or_else(|| sheet(Archetype::Husk)).and_then(|s| Some((s, self.enemy_frame(s, e)?)));
            let source = drawn.as_ref().map(|(_, f)| f.clone());
            // One got past is drawn while it is still beside the hero; further back it would stand
            // between them and the camera, huge, and is warned of at the screen's edge instead
            // (`paint_behind`).
            // A felled bloater stays while it swells and bursts, then is gone.
            let swelling = (1..crate::fight::BURST_END).contains(&e.grave.swell);
            let shown = (e.alive() || swelling) && d > 0.6 && e.gap(walked) > -0.6;
            let (spot, img) = match drawn {
                Some((s, frame)) => (shown.then(|| sheet_at(s, x, d, metres)).flatten(), frame),
                None => (shown.then(|| at(x, d, metres, img)).flatten(), img.clone()),
            };
            let img = if e.chill > 0 && e.alive() { self.frosted(img) } else { img };
            let img = if e.burn.left > 0 && e.alive() { crate::sheen::Sheen::smouldering(&img, self.ticks) } else { img };
            // Nearer draws over further; all of them under the hero, who stands nearest.
            let layer = (50.0 - d * 2.0).round().clamp(1.0, 50.0) as i32;
            if spot.is_some() { stands.push((x, d, metres, 0.3 * metres)); }
            auras.push((enemy_id(i), source.filter(|_| spot.is_some() && e.variant == crate::fight::Variant::Dread)));
            placed.push((enemy_id(i), spot.map(|r| (r, lit(r, img, x, d), layer))));
        }
        // Enemy objects the place leaves empty.
        TMP_LAP.with(|l| l.borrow_mut().mark("p.enemies"));
        let drawn = if hidden { 0 } else { self.fight.enemies.len() };
        for i in drawn..world::MOST_FOES { placed.push((enemy_id(i), None)); auras.push((enemy_id(i), None)); }
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
                        // Imbued with the hero's blood: red down its length.
                        let rgb = if b.imbued { [220, 30, 40] } else if r.bolts() { [200, 184, 156] } else { [236, 226, 196] };
                        let (head, tail) = (pf.project_on(c, b.x + SHOULDER, 1.35, d)?, pf.project_on(c, b.x + SHOULDER, 1.3, (d - 2.0).max(HERO_D))?);
                        let w = 0.36 * pf.scale_on(c, d)?;
                        let len = (tail.1 - head.1).abs().max(0.6 * w);
                        let img = Arc::new(image::imageops::rotate270(&*arrow_picture(rgb)));
                        Some((((0.5 * (head.0 + tail.0) - 0.5 * w, head.1), (w, len)), img, layer))
                    }
                }
            });
            // The thrown Hatchet, spinning out and back.
            let drawn = drawn.or_else(|| {
                let t = self.fight.hatchets.get(i.checked_sub(self.fight.missiles.len() + self.fight.bolts.len())?).filter(|_| !hidden)?;
                let d = (t.s - walked) as f32;
                let layer = (50.0 - d * 2.0).round().clamp(1.0, 59.0) as i32;
                let (px, py) = pf.project_on(c, t.x + SHOULDER, 1.2, d)?;
                let size = 0.8 * pf.scale_on(c, d)?;
                Some((((px - 0.5 * size, py - 0.5 * size), (size, size)), hatchet_picture(self.hero_sheet.as_ref()?, t.age)?, layer))
            });
            placed.push((shot_id(i), drawn));
        }
        // The shrine on the right verge, the fork's signpost on the left, the souls on the road.
        TMP_LAP.with(|l| l.borrow_mut().mark("p.shots"));
        let prop = |img: &Arc<RgbaImage>, s: Option<f64>, x: f32, metres: f32, stands: &mut Vec<(f32, f32, f32, f32)>| {
            let d = (s? - walked) as f32;
            let layer = (50.0 - d * 2.0).round().clamp(1.0, 50.0) as i32;
            let r = (d > 0.8).then(|| on_ground(x, d, metres, img)).flatten()?;
            stands.push((x, d, metres, 0.35 * metres));
            Some((r, lit(r, img.clone(), x, d), layer))
        };
        // The prologue's road is a memory and a cutscene's roads are elsewhere: none of the
        // world's things stand on them.
        let tutoring = self.prologue.is_some() || matches!(self.mode, Mode::Out(Next::Prologue) | Mode::Cut(After::Prologue) | Mode::Scene(_));
        let (shrine, signpost) = match &self.props { Some((a, b)) => (Some(a), Some(b)), None => (None, None) };
        // At rest the shrine stands in the middle of the clearing, the keeper to its left and the
        // smith (once met) to its right; on the road the shrine is on the right verge and the
        // smith, until met, waits on the left.
        let station = |d: f32| (walked + d as f64, 50 - (d * 2.0) as i32);
        // Someone met on the road, idling: (its idle's period in ticks, its height in metres).
        let figure = |sheet: &Option<Sheet>, x: f32, d: f32, (period, tall): (f32, f32), stands: &mut Vec<(f32, f32, f32, f32)>| {
            let s = sheet.as_ref()?;
            let img = s.clip("idle")?.looped(self.ticks as f32, period).clone();
            let layer = (50.0 - d * 2.0).round().clamp(1.0, 50.0) as i32;
            let r = (d > 0.8).then(|| sheet_at(s, x, d, tall)).flatten()?;
            stands.push((x, d, tall, 0.6));
            Some((r, lit(r, img, x, d), layer))
        };
        let npc = |sheet: &Option<Sheet>, x: f32, d: f32, period: f32, stands: &mut Vec<(f32, f32, f32, f32)>| figure(sheet, x, d, (period, 1.85), stands);
        if resting {
            let (at, _) = station(STATION_D + 0.4);
            placed.push((SHRINE.to_owned(), shrine.and_then(|img| prop(img, Some(at), 0.0, 2.4, &mut stands))));
            placed.push((KEEPER.to_owned(), npc(&self.keeper_sheet, -STATION_X, STATION_D, 80.0, &mut stands)));
            placed.push((SMITH.to_owned(), self.world.met_smith.then(|| npc(&self.smith_sheet, STATION_X, STATION_D, 70.0, &mut stands)).flatten()));
        } else {
            // Beside the path, but inside the walls of a narrow street (half its width in from them).
            let inside = |x: f32, s: f64, half: f32| match pf.wall_x((s - walked) as f32) {
                Some(w) => x.signum() * x.abs().min((w - half - 0.15).max(0.0)),
                None => x,
            };
            let shrine_x = |img: &Arc<RgbaImage>, s: f64| inside(3.4 * self.world.side(), s, 0.5 * 1.9 * img.width() as f32 / img.height().max(1) as f32);
            placed.push((SHRINE.to_owned(), shrine.filter(|_| !tutoring).and_then(|img| { let s = self.world.shrine()?; prop(img, Some(s), shrine_x(img, s), 1.9, &mut stands) })));
            placed.push((KEEPER.to_owned(), None));
            let smith = self.world.smith_at().filter(|_| !tutoring).and_then(|s| npc(&self.smith_sheet, inside(-3.0 * self.world.side(), s, 0.45), (s - walked) as f32, 70.0, &mut stands));
            placed.push((SMITH.to_owned(), smith));
            // Gruk left of the middle, behind his counter; the board shows past him.
            let gruk = self.world.gruk_at().filter(|_| !tutoring);
            // The counter just before him, as high as his waist from where the hero stands.
            placed.push((GRUK.to_owned(), gruk.and_then(|s| figure(&self.gruk_sheet, inside(-1.4, s, 0.4), (s - walked) as f32, (70.0, 1.45), &mut stands))));
            placed.push((COUNTER.to_owned(), gruk.zip(self.counter.as_ref()).and_then(|(s, img)| {
                let half = 0.5 * 0.9 * img.width() as f32 / img.height().max(1) as f32;
                prop(img, Some(s - 0.15), inside(-1.4, s, half), 0.9, &mut stands)
            })));
            // Whoever waits on the road here, a little left of the middle, facing the hero.
            let visitor = self.world.visitor_at().filter(|_| !tutoring);
            placed.push((VISITOR.to_owned(), visitor.and_then(|(s, who)| figure(&self.visitor_sheet.1, inside(-1.3, s, 0.4), (s - walked) as f32, (70.0, who.height()), &mut stands))));
        }
        // On the island where the branches part.
        placed.push((SIGNPOST.to_owned(), signpost.filter(|_| !resting && !tutoring).and_then(|img| prop(img, self.world.signpost, self.world.signpost_x(walked), 2.4, &mut stands))));
        let stain = self.world.stain_at().filter(|_| !resting && !tutoring).and_then(|s| {
            let d = (s - walked) as f32;
            let (fx, fy) = pf.project_on(c, 0.0, 0.0, d)?;
            let w = 0.6 * pf.scale_on(c, d)?;
            (d > 0.8).then(|| (((fx - 0.5 * w, fy - 0.2 * w), (w, 0.2 * w)), self.art.stain.clone(), 2))
        });
        placed.push((STAIN.to_owned(), stain));
        // The place's cache: a chest at its metre, beside the path, shut until it is taken.
        let side = self.world.side();
        let chest = self.world.cache_at().filter(|_| !resting && !tutoring).zip(self.chest.as_ref()).and_then(|(at, (shut, open))| {
            let img = if self.world.cache_taken() { open } else { shut };
            prop(img, Some(at), 1.1 * side, 0.95, &mut stands)
        });
        placed.push((CHEST.to_owned(), chest));
        // The veil across the road before a standing boss, the walls' width (or the road's
        // and its verges'); in front of the hero once they are stepping through it.
        let veil = self.world.veil().filter(|_| !resting && !tutoring).and_then(|s| {
            let d = (s - walked) as f32;
            if !(0.3..=45.0).contains(&d) { return None; }
            let half = pf.wall_x(d).map_or(4.6, |w| w.min(6.0));
            let ((l, b), (r, _), (_, t)) = (pf.project_on(c, -half, 0.0, d)?, pf.project_on(c, half, 0.0, d)?, pf.project_on(c, 0.0, VEIL_H, d)?);
            let part = if let Mode::Cross(t) = self.mode { t as f32 / CROSS_T as f32 } else { 0.0 };
            let layer = if d > HERO_D - 0.1 { (50.0 - d * 2.0).round().clamp(1.0, 50.0) as i32 } else { 62 };
            Some((((l.min(r), t), ((r - l).abs(), b - t)), veil_picture(self.ticks as u32, part), layer))
        });
        placed.push((VEIL.to_owned(), veil));
        TMP_LAP.with(|l| l.borrow_mut().mark("p.props"));
        placed.extend((0..WAVES).map(|i| (wave_id(i), None)));
        // The people a cutscene shows on the road.
        let figures = self.scene.as_ref().filter(|_| matches!(self.mode, Mode::Scene(_))).map(Cutscene::figures).unwrap_or_default();
        for i in 0..CAMEOS {
            let spot = figures.get(i).and_then(|&(f, x, d)| {
                let s = self.cameos.iter().find(|(n, _)| *n == f.sheet).map(|(_, s)| s)?;
                let img = s.clip(f.clip)?.looped(self.ticks as f32, 48.0).clone();
                let r = (d > 0.8).then(|| sheet_at(s, x, d, f.tall)).flatten()?;
                stands.push((x, d, f.tall, 0.6));
                Some((r, lit(r, img, x, d), (50.0 - d * 2.0).round().clamp(1.0, 50.0) as i32))
            });
            placed.push((cameo_id(i), spot));
        }
        self.relit.borrow_mut().trim();
        TMP_LAP.with(|l| l.borrow_mut().mark("p.trim"));
        let shadow_img = self.shadows(c, pf, &stands, &seen.into_inner());
        TMP_LAP.with(|l| l.borrow_mut().mark("p.shadowpaint"));
        Self::show_shadows(c, shadow_img);
        TMP_LAP.with(|l| l.borrow_mut().mark("p.shadowshow"));
        if let (Some(p), Some(o)) = (head, c.get_game_object_mut(CALLOUT)) { o.position = p; }
        let looks: Vec<(String, Recolor)> = self.fight.enemies.iter().enumerate()
            .filter_map(|(i, e)| Some((enemy_id(i), Self::variant_look(e.kind, e.variant)?))).collect();
        Self::put(c, placed, &looks);
        TMP_LAP.with(|l| l.borrow_mut().mark("p.put"));
        Self::wreathe(c, &auras);
        self.pulse_rune(c, pulse_source.filter(|_| !h.dead));
        TMP_LAP.with(|l| l.borrow_mut().mark("p.wreathe"));
    }

    /// Show each object where it was placed (hidden where it was not), recoloured as `looks`
    /// says for its id.
    fn put(c: &mut Canvas, placed: Vec<(String, Option<(Rect, Arc<RgbaImage>, i32)>)>, looks: &[(String, Recolor)]) {
        for (id, spot) in placed {
            let Some(o) = c.get_game_object_mut(&id) else { continue };
            match spot {
                Some(((pos, size), img, layer)) => {
                    o.visible = true;
                    o.position = pos;
                    o.size = size;
                    o.layer = layer;
                    let recolor = looks.iter().find(|(of, _)| *of == id).map(|(_, r)| *r);
                    o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None, recolor });
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
        let view = Self::view_of(pf);
        let mut last = self.last_shadows.borrow_mut();
        let same_frame = view == last.2;
        if shadows == last.0 && casts == last.1 && (same_frame || shadows.is_empty()) { return None; }
        let img = if shadows.is_empty() && casts.is_empty() { None } else { pf.paint_shadows(&shadows, &casts).map(Arc::new) };
        *last = (shadows, casts, view);
        Some(img)
    }

    fn show_shadows(c: &mut Canvas, img: Option<Option<Arc<RgbaImage>>>) {
        let (Some(img), Some(o)) = (img, c.get_game_object_mut(SHADOWS)) else { return };
        match img {
            Some(img) => {
                set_pixel_art(&img, true);
                o.visible = true;
                let size = o.size;
                o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None, recolor: None });
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
            let glint = s.glint(&img);
            let img = if h.stone > 0 && !h.dead { self.stoned(img) } else { img };
            let r = sheet_at(s, hd, h.y, art::HERO_METRES)?;
            Some((r, self.sheen.borrow_mut().dress(lit(r, img, 0.0, hd), glint.as_ref(), &h.stats.kit.capped.worn, self.ticks), 60))
        });
        if hero.is_some() { stands.push((0.0, hd, art::HERO_METRES, 0.55)); }
        let head = hero.as_ref().map(|(((x, y), (w, _)), _, _)| (x + 0.5 * w - 900.0, (y - 150.0).max(240.0)));
        // The foes step out once the camera has come round (before, they would stand in its way):
        // the Warden as the boss, an ambush's as the road's enemies.
        // The unlit frames of dread foes drawn, by where they stand, for their auras (`wreathe`).
        let yard_auras: RefCell<Vec<(u32, Arc<RgbaImage>)>> = RefCell::new(Vec::new());
        let foe = |f: &side::Foe, sheet: Option<&Sheet>, metres: f32, layer: i32, stands: &mut Vec<(f32, f32, f32, f32)>| {
            let s = sheet.filter(|_| k > 0.8)?;
            let fd = ARENA_D + f.s;
            let img = self.foe_frame(s, f, h.s, dir)?;
            if f.variant == crate::fight::Variant::Dread && f.alive() { yard_auras.borrow_mut().push((f.s.to_bits(), img.clone())); }
            let img = if f.chill > 0 && f.alive() { self.frosted(img) } else { img };
            let img = if f.burn.left > 0 && f.alive() { crate::sheen::Sheen::smouldering(&img, self.ticks) } else { img };
            let r = sheet_at(s, fd, f.lift(), metres)?;
            stands.push((0.0, fd, metres, 0.45 * metres));
            Some((r, lit(r, img, 0.0, fd), layer))
        };
        // A boss or mini-boss is drawn from its own art; while that is missing, a mini-boss from its
        // kind's, a head taller, and a boss from the Warden's.
        // (One that melted away is gone once its smoke has risen.)
        let boss = (a.why == Why::Boss).then(|| a.side.foes.first().filter(|f| !(a.side.fled && f.down > 8)).and_then(|f| {
            let own = |folder: &str| self.boss_art.sheet(folder);
            let (sheet, metres) = match (f.breed.sprite.and_then(|(folder, m)| Some((own(folder)?, m))), f.breed.kind) {
                (Some((s, m)), _) => (Some(s), m),
                (None, Some(kind)) => (self.enemy_sheets.iter().find(|(s, _)| *s == kind).map(|(_, s)| s), kind_art(kind).1 * 1.2),
                (None, None) => (self.boss_sheet.as_ref(), BOSS_METRES),
            };
            foe(f, sheet, metres, 40, &mut stands)
        })).flatten();
        let mut placed: Vec<(String, Option<(Rect, Arc<RgbaImage>, i32)>)> = vec![(HERO.to_owned(), hero), (BOSS.to_owned(), boss)];
        for i in 0..world::MOST_FOES {
            let drawn = match (a.why, a.side.foes.get(i), a.side.allies.get(i)) {
                (Why::Ambush(_) | Why::Prologue, Some(f), _) => {
                    let kind = f.breed.kind.unwrap_or(Archetype::Husk);
                    let sheet = self.enemy_sheets.iter().find(|(s, _)| *s == kind).map(|(_, s)| s);
                    foe(f, sheet, kind_art(kind).1, 40 - i as i32, &mut stands)
                }
                // Whoever was called to the boss, in the slots its fight leaves free, once the
                // camera has come round.
                (Why::Boss, _, Some(ally)) if k > 0.8 => self.sheet_of(ally.who).and_then(|s| {
                    let metres = ally.who.style().sprite.1;
                    let img = self.ally_frame(s, ally, dir)?;
                    let fd = ARENA_D + ally.at;
                    let r = sheet_at(s, fd, 0.0, metres)?;
                    stands.push((0.0, fd, metres, 0.45 * metres));
                    Some((r, lit(r, img, 0.0, fd), 45))
                }),
                _ => None,
            };
            placed.push((enemy_id(i), drawn));
        }
        for id in [SHRINE, SIGNPOST, STAIN, KEEPER, SMITH, GRUK, COUNTER, VISITOR, CHEST, VEIL] { placed.push((id.to_owned(), None)); }
        placed.extend((0..CAMEOS).map(|i| (cameo_id(i), None)));
        // Each ring blow's two fronts, rolling out along the ground, the crest's leading face on
        // the front.
        let fronts: Vec<(f32, f32, [u8; 3], f32)> = a.side.foes.iter().filter(|f| f.alive()).filter_map(|f| Some((f.ring()?, f.blow()?.0)))
            .flat_map(|((at, r), b)| [-1.0f32, 1.0].map(|way| (at + way * r, way, warning_rgb(b.kind, b.parry), b.height.1))).collect();
        for i in 0..WAVES {
            let drawn = fronts.get(i).filter(|f| f.0.abs() < side::HALF + 0.3).and_then(|&(s, way, rgb, high)| {
                let d = ARENA_D + s;
                let (px, py) = pf.project_on(c, 0.0, 0.0, d)?;
                let m = pf.scale_on(c, d)?;
                let (w, ht) = (2.6 * m, (high + 0.75) * m);
                let mirror = way * dir < 0.0;
                let left = if mirror { px - 0.21 * w } else { px - 0.79 * w };
                Some((((left, py - ht), (w, ht)), crest_picture(self.ticks as u32, rgb, mirror), 44))
            });
            placed.push((wave_id(i), drawn));
        }
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
                    crate::gear::Ranged::Longbow => ((1.1 * m, 0.2 * m), arrow_picture([214, 112, 52])),
                };
                if b.dir * dir < 0.0 { img = Arc::new(image::imageops::flip_horizontal(&*img)); }
                Some((((px - 0.5 * size.0, py - 0.5 * size.1), size), img, 46))
            });
            // An ally's arrow, in the slots past the hero's.
            let drawn = drawn.or_else(|| {
                let (s, v) = a.side.allies.get(i.checked_sub(world::MOST_FOES + a.side.bolts.len() + a.side.hatchets.len())?)?.arrow?;
                let d = ARENA_D + s;
                let (px, py) = pf.project_on(c, 0.0, 1.0, d)?;
                let m = pf.scale_on(c, d)?;
                let size = (0.9 * m, 0.2 * m);
                let img = arrow_picture([214, 92, 70]);
                let img = if v * dir < 0.0 { Arc::new(image::imageops::flip_horizontal(&*img)) } else { img };
                Some((((px - 0.5 * size.0, py - 0.5 * size.1), size), img, 46))
            });
            let drawn = drawn.or_else(|| {
                let t = a.side.hatchets.get(i.checked_sub(world::MOST_FOES + a.side.bolts.len())?)?;
                let d = ARENA_D + t.s;
                let (px, py) = pf.project_on(c, 0.0, 1.2, d)?;
                let size = 0.8 * pf.scale_on(c, d)?;
                Some((((px - 0.5 * size, py - 0.5 * size), (size, size)), hatchet_picture(self.hero_sheet.as_ref()?, t.age)?, 46))
            });
            placed.push((shot_id(i), drawn));
        }
        self.relit.borrow_mut().trim();
        let shadow_img = self.shadows(c, pf, &stands, &seen.into_inner());
        Self::show_shadows(c, shadow_img);
        if let (Some(p), Some(o)) = (head, c.get_game_object_mut(CALLOUT)) { o.position = p; }
        let mut looks: Vec<(String, Recolor)> = a.side.foes.iter().enumerate().filter(|_| matches!(a.why, Why::Ambush(_)))
            .filter_map(|(i, f)| Some((enemy_id(i), Self::variant_look(f.breed.kind?, f.variant)?))).collect();
        // A hunter's Taint echo: his own art, bruised violet in the shadows and ashen in the light.
        if a.why == Why::Boss && a.side.foes.first().is_some_and(|f| crate::hunters::ECHOES.iter().any(|e| std::ptr::eq(e.boss.breed, f.breed))) {
            looks.push((BOSS.to_owned(), Recolor::graded(Color(22, 6, 36, 255), Color(196, 184, 228, 255), 0.55).toned(0.85, 0.85, 1.15)));
        }
        let traced = yard_auras.take();
        let auras: Vec<(String, Option<Arc<RgbaImage>>)> = (0..world::MOST_FOES).map(|i| {
            let frame = a.side.foes.get(i).filter(|_| matches!(a.why, Why::Ambush(_)))
                .and_then(|f| traced.iter().find(|(s, _)| *s == f.s.to_bits()).map(|(_, img)| img.clone()));
            (enemy_id(i), frame)
        }).collect();
        Self::put(c, placed, &looks);
        Self::wreathe(c, &auras);
    }

    /// The hero's side-on frame (the clips face right; mirrored when they face left on screen).
    fn side_hero_frame(&self, s: &Sheet, h: &side::SideHero, dir: f32) -> Option<Arc<RgbaImage>> {
        let mirror = h.facing * dir < 0.0;
        let kit = in_hand(&h.stats.kit, matches!(&self.mode, Mode::Side(a) if !a.side.hatchets.is_empty()));
        let at = |n: &str, k: f32| worn(s, &kit, n).map(|c| c.at(k.clamp(0.0, 0.999), mirror).clone());
        let total = |f: (u32, u32, u32)| (f.0 + f.1 + f.2).max(1) as f32;
        use side::Act;
        let class = kit.arms.class;
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
            Some((Act::Light, t)) => held(s, &kit, "e_", "light").map(|c| phased(c, Act::Light.frames(&kit), t, (3, 5), mirror)),
            Some((Act::Heavy, t)) => held(s, &kit, "e_", "heavy").map(|c| phased(c, Act::Heavy.frames(&kit), t, (5, 7), mirror)),
            // The shield's flick with one on the arm, else the weapon's.
            Some((Act::Parry, t)) => {
                let k = t as f32 / total(Act::Parry.frames(&kit));
                if kit.off.is_some() { at("e_parry", k) } else { at("e_parry_weapon", k).or_else(|| at("e_parry", k)) }
            }
            Some((Act::Flask, _)) => at("e_guard", 0.4),
            // Its own draw or cast where there is one; else the other hand raised as for a parry.
            Some((a @ (Act::Loose | Act::Cast), t)) => loosing(s, &kit, "e_", a == Act::Cast).map(|(c, keys)| phased(c, a.frames(&kit), t, keys, mirror))
                .or_else(|| at("e_parry", (t as f32 / total(a.frames(&kit))).min(0.45))),
            Some((Act::Art, t)) if kit.art.is_some_and(|a| a.light()) => held(s, &kit, "e_", "light").map(|c| phased(c, Act::Art.frames(&kit), t, (3, 5), mirror)),
            Some((Act::Art, t)) => held(s, &kit, "e_", "heavy").map(|c| phased(c, Act::Art.frames(&kit), t, (5, 7), mirror)),
            // Off the ground springing, knees coming up as it slows, tucked over the top, the knees
            // opening and the legs reaching down for the ground (the clip's 5, 6, 7).
            None if h.airborne() => {
                let rise = h.rise();
                let pose = if rise > 0.55 { 5 } else if rise > 0.2 { 6 } else if rise > -0.2 { 7 } else if h.y > 0.45 { 6 } else { 5 };
                worn(s, &kit, "e_jump").map(|c| c.at((pose as f32 + 0.5) / c.frames.len() as f32, mirror).clone())
            }
            None if h.guarding => at("e_guard", 1.0),
            None if h.walking => at("e_walk", (h.s / 1.6).rem_euclid(1.0)),
            // Down into the knees and back up out of a landing (the clip's crouch, 4 then 3).
            None if h.footing.landing > 0 => {
                let pose = if h.footing.landing > side::LAND_T / 2 { 4 } else { 3 };
                worn(s, &kit, "e_jump").map(|c| c.at((pose as f32 + 0.5) / c.frames.len() as f32, mirror).clone())
            }
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
        // A boss's entrance: the clip its cutscene asks for.
        if let Some((n, k)) = self.scene.as_ref().and_then(Cutscene::pose).filter(|_| f.breed.kind.is_none() && f.alive()) { return at(n, k); }
        match f.ai {
            // Without a fall of its own to play, a felled foe is gone.
            // A bloater's fall is its burst: once that is over, nothing is left of it.
            side::Ai::Dead if f.breed.kind == Some(Archetype::Bloater) && f.down > 66 => None,
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

    /// Ally `a`'s frame: its blow or dodge played through, rocked by a blow, its fall, else its
    /// walk or its idle. Its art faces left, as a boss's does.
    /// The art of whoever was called or came to the fight, loaded once.
    fn load_ally(&mut self, who: crate::allies::Summoner) {
        let folder = who.style().sprite.0;
        if self.sheet_of(who).is_some() { return; }
        if let Some(sheet) = load_sheet(folder) { self.ally_sheet.push((folder, sheet)); }
    }

    fn sheet_of(&self, who: crate::allies::Summoner) -> Option<&Sheet> {
        let folder = who.style().sprite.0;
        self.ally_sheet.iter().find(|(f, _)| *f == folder).map(|(_, sheet)| sheet)
    }

    /// Atrael at the Hollowmere Screamer, as in the book, once Olekeen is in the world: down in
    /// his ravens when it is a quarter beaten, and thrown through a shack wall at half.
    fn ravens(&mut self, notes: &mut Vec<Note>) {
        use crate::allies::{Doing, Summoner};
        if self.scene.is_some() || !self.world.ravens_come() { return; }
        let hero_top = self.fight.hero.stats.max_hp();
        let Mode::Side(a) = &mut self.mode else { return };
        let Some(foe) = a.side.foes.first() else { return };
        if a.why != Why::Boss || !std::ptr::eq(foe.breed, &side::SCREAMER) || !foe.alive() { return; }
        let beaten = foe.hp < 0.75 * foe.top;
        match a.side.allies.iter().find(|x| x.who == Summoner::Atrael).map(|x| x.doing) {
            None if beaten => {
                a.side.join(Summoner::Atrael, hero_top, 1.0);
                notes.push(Note::Called(Summoner::Atrael));
                self.load_ally(Summoner::Atrael);
            }
            Some(Doing::Down(1)) => notes.push(Note::Told("The Screamer flings Atrael through a shack wall. He does not get up.")),
            _ => {}
        }
    }

    fn ally_frame(&self, s: &Sheet, a: &crate::allies::Ally, dir: f32) -> Option<Arc<RgbaImage>> {
        use crate::allies::Doing;
        let mirror = a.facing * dir > 0.0;
        let st = a.who.style();
        let at = |n: &str, k: f32| s.clip(n).or_else(|| s.clip("idle")).map(|c| c.at(k.clamp(0.0, 0.999), mirror).clone());
        let (w, after, _) = st.frames;
        match a.doing {
            Doing::Down(t) => at("death", t as f32 / 60.0),
            Doing::Reel(n) => at("stagger", 1.0 - n as f32 / 18.0),
            Doing::Dodge(t, _) => at(st.clips.1, t as f32 / 20.0),
            Doing::Swing(t) => at(st.clips.0, t as f32 / (w + after) as f32),
            Doing::Ready(_) if a.walking => at("walk", (a.at / 1.4).rem_euclid(1.0)),
            Doing::Ready(_) => at("idle", (self.ticks as f32 / 90.0).fract()),
        }
    }

    /// The side-on fight's warnings on the ground, one for each foe winding up or live.
    /// This tick's presses for the side-on fight, joined to any waiting; held keys as they are.
    /// A and D are left and right on screen, whichever way the road runs across it.
    fn side_input(&mut self, c: &Canvas, presses: &[Press]) -> SideInput {
        self.taps(c, presses);
        let th = self.touch_held;
        let run = matches!(self.running, Some('a' | 'd')) || (th.run && (th.left || th.right));
        let dir = c.get_plugin::<PathForgePlugin>().map_or(1.0, screen_dir);
        let held = |ch: &str, n: NamedKey| c.is_key_held(&Key::Character(ch.into())) || c.is_key_held(&Key::Named(n));
        let (l, r) = (held("a", NamedKey::ArrowLeft) || th.left, held("d", NamedKey::ArrowRight) || th.right);
        let (left, right) = if dir > 0.0 { (l, r) } else { (r, l) };
        let p = &mut self.side_pending;
        for press in presses {
            match press {
                Press::Key(k, shift) => {
                    // Shift with A or D rolls that way; Space jumps. Not W: the touch stick pushed
                    // up is W, and a thumb leaning on it jumped by accident.
                    if *shift && matches!(way(k), Some('a' | 'd')) { p.roll = true; }
                    if is(k, &[" "], None) || is(k, &[], Some(NamedKey::Space)) { p.jump = true; }
                    if is(k, &["r"], None) { p.flask = true; }
                    if is(k, &["f"], None) { p.loose = true; }
                    if is(k, &["c"], None) { p.cast = true; }
                    if is(k, &["q"], None) { p.art = true; }
                }
                Press::Mouse(MouseButton::Left, false) => p.light = true,
                Press::Mouse(MouseButton::Left, true) => p.heavy = true,
                Press::Mouse(MouseButton::Right, true) => p.parry = true,
                Press::Mouse(..) => {}
                Press::Touch(t) => {
                    use crate::touch::Tap;
                    match t {
                        Tap::Light => p.light = true, Tap::Heavy => p.heavy = true, Tap::Parry => p.parry = true,
                        Tap::Flask => p.flask = true, Tap::Loose => p.loose = true, Tap::Cast => p.cast = true, Tap::Art => p.art = true,
                        Tap::Roll => p.roll = true, Tap::Jump => p.jump = true,
                    }
                }
            }
        }
        SideInput { left, right, run, guard: (c.is_mouse_held(MouseButton::Right) && !shift_held(c)) || th.guard, ..*p }
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
        // The art meter sits under the ward when there is one, in its place when not.
        let y = if h.stats.kit.ward.is_some() { 242.0 } else { 218.0 };
        for id in ["bar_art_back", "bar_art", "bar_art_ready"] { if let Some(o) = c.get_game_object_mut(id) { o.position.1 = y; } }
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
        let bulwark = h.stats.kit.ward.is_some();
        let ready = ward >= crate::fight::WARD_BURST;
        set("bar_ward_back", bulwark.then_some(1.0));
        set("bar_ward", (bulwark && !ready).then_some(ward / crate::fight::WARD_MAX));
        set("bar_ward_ready", (bulwark && ready).then_some(ward / crate::fight::WARD_MAX));
        // The art meter, only with a forged weapon in hand; under the ward when there is one.
        let meter = match &self.mode { Mode::Side(a) => a.side.hero.art, _ => h.art };
        let art = h.stats.kit.art;
        let paid = art.is_some_and(|a| meter >= a.cost());
        set("bar_art_back", art.map(|_| 1.0));
        set("bar_art", art.filter(|_| !paid).map(|_| meter / crate::arts::ART_MAX));
        set("bar_art_ready", art.filter(|_| paid).map(|_| meter / crate::arts::ART_MAX));
        let ally = match &self.mode { Mode::Side(a) => a.side.allies.first().filter(|x| x.standing()), _ => None };
        set("bar_ally_back", ally.map(|_| 1.0));
        set("bar_ally", ally.map(|x| x.hp / x.top));
        let name = if let Mode::Side(a) = &self.mode {
            // The boss; in an ambush, the nearest still standing.
            let b = a.side.nearest().map(|i| &a.side.foes[i]);
            set("bar_foe_back", b.map(|_| 1.0));
            set("bar_foe", b.map(|b| b.hp / b.max_hp()));
            set("bar_foe_stance", b.map(|b| if matches!(b.ai, side::Ai::Open(_)) { 0.0 } else { b.stance / b.max_stance() }));
            b.map(|b| b.breed.name.to_string())
        } else {
            set("bar_foe_back", foe.map(|_| 1.0));
            set("bar_foe", foe.map(|e| e.hp / e.top));
            set("bar_foe_stance", foe.map(|e| if matches!(e.ai, Ai::Staggered(_)) { 0.0 } else { e.stance / e.kind.stance() }));
            foe.map(|e| format!("{:?}", e.kind))
        };
        // Whose bar it is, over it, right-aligned to its end.
        let (Some(name), Some(face)) = (name.filter(|_| shown), self.title_font.clone().or_else(|| self.font.clone())) else {
            if let Some(o) = c.get_game_object_mut(FOE_NAME) { o.visible = false; }
            return;
        };
        let s = c.base_scale();
        let text = Text::new(vec![Span::new(name, 56.0 * s, Some(70.0 * s), face, Color(236, 222, 196, 235), 0.0)], Some(1200.0 * s), Align::Right, None);
        if let Some(o) = c.get_game_object_mut(FOE_NAME) { o.visible = true; o.set_drawable(Box::new(text)); }
    }

    fn hud(&mut self, c: &mut Canvas, walked: f64) {
        let Some(font) = self.font.clone() else { return };
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        let h = &self.fight.hero;
        let w = &self.world;
        let calling = self.calling.map_or("", |c| c.name());
        let line = match self.mode {
            Mode::Front(_) => String::new(),
            _ if self.prologue.is_some() => format!("Prologue  |  a remembered road  |  {calling}  |  flasks {}", h.flasks),
            Mode::Sanctum(_) => format!(
                "At rest  |  {calling}  level {}  |  {} {}  |  shards {}  |  weapon +{}  offhand +{}",
                h.stats.level(), world::CURRENCY, w.souls, w.shards_line(), h.stats.kit.main, h.stats.kit.off_level,
            ),
            _ => format!(
                "{}  |  {} {}  shards {}{}  |  level {}  |  flasks {}{}{}",
                w.here().title, world::CURRENCY, w.souls, w.shards_line(), w.taint_line(), h.stats.level(), h.flasks,
                {
                    let hand = match h.stats.kit.ranged {
                        Some(r) if r != crate::gear::Ranged::Frost => {
                            let (what, n) = if r.bolts() { ("bolts", h.quiver.bolts) } else { ("arrows", h.quiver.arrows) };
                            let imbued = if h.quiver.imbued > 0 { format!(" ({} imbued)", h.quiver.imbued) } else { String::new() };
                            format!("  {what} {n}{imbued}")
                        }
                        _ => String::new(),
                    };
                    let spell = match h.stats.kit.spell_cast() {
                        Some(sp) if !h.stats.spell_ready() => format!("  {} needs focus {}", sp.name(), sp.needs()[2]),
                        Some(_) => format!("  spell {}/{}", h.charges.floor(), h.stats.max_charges()),
                        // The one Blood Imbue: ready, or coming back once its shots are spent.
                        None if h.stats.kit.imbues() && h.quiver.imbued == 0 => if h.charges >= 1.0 { "  imbue ready".to_owned() } else { format!("  imbue {:.0}%", 100.0 * h.charges) },
                        None => String::new(),
                    };
                    // Short of what the weapon needs: said where it is seen mid-walk, not only at rest.
                    let weak = if h.stats.weak() { "  |  wielded weakly" } else { "" };
                    hand + &spell + weak
                },
                if self.show_dev { format!("  |  {} {:.1} ms  road {} fps  {:.0} m", pf.engine().to_uppercase(), pf.render_ms(), self.frames.1, walked) } else { String::new() },
            ),
        };
        // Text is laid out in window pixels: sized by the window's fit, like the box it sits in.
        let s = c.base_scale();
        let text = Text::new(vec![Span::new(line, 52.0 * s, Some(64.0 * s), font.clone(), Color(236, 228, 210, 255), 0.0)], Some(3600.0 * s), Align::Left, None);
        if let Some(o) = c.get_game_object_mut(HUD) { o.set_drawable(Box::new(text)); }
        let boss = matches!(self.mode, Mode::Side(_));
        let help = if let Mode::Side(a) = &self.mode { if a.why == Why::Boss { BOSS_HELP } else { AMBUSH_HELP } } else { "W/S walk   W W (held) run   A/D change lane   Shift+W/A/S/D roll   LMB strike   Shift+LMB heavy   RMB guard   Shift+RMB parry   R drink   F ranged attack   C cast   Q weapon art   E rest   T turn back   M map      warnings: gold parry  ·  orange roll or block  ·  purple roll" };
        // The controls stay out of the way until asked for.
        let (help, size, a) = if self.show_help { (help, 40.0, 190) } else { ("H  show controls", 36.0, 120) };
        let help = Text::new(vec![Span::new(help.into(), size * s, Some(size * 1.25 * s), font.clone(), Color(236, 228, 210, a), 0.0)], Some(3720.0 * s), Align::Left, None);
        if let Some(o) = c.get_game_object_mut(HELP) { o.visible = self.mode == Mode::Play || boss; o.set_drawable(Box::new(help)); }
        // The banner, fading out over its last second.
        let (line, left) = &self.banner;
        let a = (*left as f32 / 60.0).min(1.0);
        let face = self.title_font.clone().unwrap_or(font);
        // Never over a title or a card; over a menu, small, under the bars on the side away from
        // its box (the HUD's line is along the top).
        let style = self.panel().map(|(_, st, _)| st);
        let menu_left = matches!(&self.mode, Mode::Sanctum(s) if s.station() == shrine::Station::Smith) || self.mode == Mode::Map;
        let ((size, x, y), (w, align)) = if style == Some(Style::Menu) {
            ((64.0, if menu_left { 3840.0 - 60.0 - 1700.0 } else { 60.0 }, 380.0), (1700.0, if menu_left { Align::Right } else { Align::Left }))
        } else { ((100.0, 0.0, 560.0), (3840.0, Align::Center)) };
        let banner = Text::new(vec![Span::new(line.clone(), size * s, Some(size * 1.3 * s), face, Color(240, 226, 190, (255.0 * a) as u8), 0.0)], Some(w * s), align, None);
        if let Some(o) = c.get_game_object_mut(BANNER) {
            o.visible = *left > 0 && !matches!(style, Some(Style::Title | Style::Died));
            o.position = (x, y);
            o.set_drawable(Box::new(banner));
        }
    }

    /// The callout, while it lasts: popping in a little large, fading over its last third.
    fn draw_callout(&mut self, c: &mut Canvas) {
        let (word, rgb, left) = self.callout.clone();
        if left == 0 {
            if let Some(o) = c.get_game_object_mut(CALLOUT) { o.visible = false; }
            return;
        }
        self.callout.2 -= 1;
        let Some(font) = self.title_font.clone().or_else(|| self.font.clone()) else { return };
        let s = c.base_scale();
        let age = CALLOUT_T - left;
        let size = 76.0 * (1.0 + 0.25 * (1.0 - (age as f32 / 6.0).min(1.0)));
        let a = (left as f32 / (CALLOUT_T as f32 / 3.0)).min(1.0);
        let text = Text::new(vec![Span::new(word, size * s, Some(size * 1.15 * s), font, Color(rgb[0], rgb[1], rgb[2], (255.0 * a) as u8), 0.0)], Some(1800.0 * s), Align::Center, None);
        if let Some(o) = c.get_game_object_mut(CALLOUT) { o.visible = true; o.set_drawable(Box::new(text)); }
    }

    /// The boss at the gate begun: the camera swings round to the path's side, and the fight goes
    /// on along it. The first time, its entrance names it; after, it is waiting.
    fn begin_boss(&mut self, c: &mut Canvas, boss: &'static atlas::Boss) -> Mode {
        Self::swing(c, SIDE_VIEW);
        let h = &self.fight.hero;
        let mut side = SideFight::boss(h.stats, h.hp, h.stamina, h.flasks, boss.breed, self.world.scale());
        (side.hero.quiver, side.hero.charges, side.hero.bulwark, side.hero.art) = (h.quiver, h.charges, h.bulwark, h.art);
        if let Some(who) = self.world.called { side.summon(who, h.stats.max_hp()); }
        // What strikes from the dark comes with no entrance, and leaves before it can be killed.
        if self.world.strikes() {
            side.flee = Some(crate::alley::MELTS);
            self.world.face_boss();
        } else if boss.is_risen() {
            if !self.world.face_risen(boss.name()) { self.scene = Some(Cutscene::entrance(boss.name(), false)); }
        } else if !self.world.face_boss() { self.scene = Some(Cutscene::entrance(boss.name(), boss.mini)); }
        Mode::Side(Box::new(Arena { side, t: 0, done: 0, why: Why::Boss, rune: 0 }))
    }

    /// Whether the rune can be thrown as bait now: in hand, against the Hollowed Shrine's guardian
    /// (the Wraith Lord), which fixates on it as it does in the book.
    fn can_bait(&self) -> bool {
        let Mode::Side(a) = &self.mode else { return false };
        self.scene.is_none() && a.why == Why::Boss && a.rune == 0 && a.t >= ARENA_IN && self.world.keys.contains(&atlas::Key::Rune)
            && a.side.foes.first().is_some_and(|f| std::ptr::eq(f.breed, &side::WRAITH) && f.alive())
    }

    /// The rune thrown past the guardian: it turns after the stone, open, and the rune hums its
    /// way back to the hand `RUNE_BACK` ticks later.
    fn throw_rune(&mut self) -> Vec<Note> {
        let Mode::Side(a) = &mut self.mode else { return Vec::new() };
        a.side.lure();
        a.rune = RUNE_BACK;
        self.hum_until = self.ticks + RUNE_PULSE.2;
        vec![Note::Told("The rune skitters past it. Every eye in the stone turns after it.")]
    }

    /// What E would do where the hero stands now, on the road (or, against the guardian, in the fight).
    fn prompt_line(&self, c: &Canvas, walked: f64) -> Option<&'static str> {
        if self.can_bait() { return Some("Throw the rune"); }
        if self.mode != Mode::Play || self.prologue.is_some() { return None; }
        let pf = c.get_plugin::<PathForgePlugin>()?;
        let (w, f) = (&self.world, &self.fight);
        if w.can_rest(f, walked) { Some("Rest at the shrine") }
        else if w.can_meet_smith(f, walked) { Some("Speak with the smith") }
        else if w.can_meet_gruk(f, walked) { Some("Speak with Gruk") }
        else if w.can_meet_visitor(f, walked) { Some("Speak") }
        else if w.can_call(f, walked) { w.summons().map(crate::allies::Summoner::call_line) }
        else if w.can_bleed(f, walked, pf.state().choose_within) { Some("Give blood to the heart") }
        else if w.side_path_at(f, walked, &pf.state().side_paths).is_some() { Some("Turn down the side path") }
        else if w.can_come_back(f, walked) { Some("Go back to the road") }
        else if w.at_veil(f, walked) { Some("Step through the veil") }
        else { None }
    }

    /// The prompt plate: low in the middle, the key on a gold cap beside the words, easing in
    /// (rising a little) when there is something to do and out when there is not.
    fn draw_prompt(&mut self, c: &mut Canvas, walked: f64) {
        let want = self.prompt_line(c, walked);
        if let Some(t) = want { self.prompt.0 = t; }
        self.prompt.1 = if want.is_some() { (self.prompt.1 + 0.1).min(1.0) } else { (self.prompt.1 - 0.1).max(0.0) };
        let a = self.prompt.1;
        for id in [PROMPT_BG, PROMPT_KEY, PROMPT_E, PROMPT] { if let Some(o) = c.get_game_object_mut(id) { o.visible = a > 0.0; } }
        let (Some(font), Some(face)) = (self.font.clone(), self.title_font.clone().or_else(|| self.font.clone())) else { return };
        if a <= 0.0 { return; }
        let s = c.base_scale();
        let al = |v: f32| (v * a).round().clamp(0.0, 255.0) as u8;
        let size = 62.0;
        // Measured as set, unwrapped: the title face's small capitals run wider than a guess.
        let words = Text::new(vec![Span::new(self.prompt.0.into(), size * s, Some(size * 1.25 * s), face, Color(240, 230, 208, al(255.0)), 0.0)], None, Align::Left, None);
        let label_w = words.size().0 / s.max(1e-3);
        let (cap, gap, pad, h) = (96.0, 34.0, 44.0, 136.0);
        let w = pad + cap + gap + label_w + pad;
        let (x, y) = (1920.0 - 0.5 * w, PROMPT_Y + (1.0 - a) * 30.0);
        // The key breathes, so the eye finds it.
        let glow = 0.82 + 0.18 * (self.ticks as f32 * 0.07).sin();
        let plate = |o: &mut GameObject, at: (f32, f32), size: (f32, f32), r: f32, c: [u8; 4]| {
            o.position = at;
            o.size = size;
            o.set_image(Image { shape: ShapeType::RoundedRectangle(0.0, size, 0.0, r), image: quartz::white_pixel(), color: Some(Color(c[0], c[1], c[2], c[3])), recolor: None });
        };
        // On a touch screen the plate is the button: TAP on the cap, and lit while a thumb is on it.
        let held = self.touch_ui && self.pad.pressed(crate::touch::Btn::Use);
        let (cap_word, cap_size) = if self.touch_ui { ("TAP", 34.0) } else { ("E", 64.0) };
        if let Some(o) = c.get_game_object_mut(PROMPT_BG) { plate(o, (x, y), (w, h), 40.0, if held { [70, 58, 34, al(225.0)] } else { [12, 11, 16, al(205.0)] }); }
        if let Some(o) = c.get_game_object_mut(PROMPT_KEY) { plate(o, (x + pad, y + 0.5 * (h - cap)), (cap, cap), 18.0, [232, 200, 120, al(255.0 * glow)]); }
        let key = Text::new(vec![Span::new(cap_word.into(), cap_size * s, Some(1.25 * cap_size * s), font, Color(20, 16, 12, al(255.0)), 0.0)], Some(cap * s), Align::Center, None);
        if let Some(o) = c.get_game_object_mut(PROMPT_E) { (o.position, o.size) = ((x + pad, y + 0.5 * (h - 1.25 * cap_size)), (cap, 1.25 * cap_size)); o.set_drawable(Box::new(key)); }
        if let Some(o) = c.get_game_object_mut(PROMPT) { (o.position, o.size) = ((x + pad + cap + gap, y + 0.5 * (h - size * 1.25)), (label_w + pad, size * 1.25)); o.set_drawable(Box::new(words)); }
    }

    fn say(&mut self, notes: &[Note]) {
        for n in notes {
            if matches!(n, Note::Hums(_) | Note::Called(crate::allies::Summoner::Olekeen)) { self.hum_until = self.ticks + RUNE_PULSE.2; }
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
                Note::Hums(quartz_path_forge::Branch::Left) => "The rune hums toward the left.".into(),
                Note::Hums(quartz_path_forge::Branch::Right) => "The rune hums toward the right.".into(),
                Note::Barred(to, Some(k)) => format!("The way to {to} is barred. It wants the {}.", k.name()),
                Note::Barred(to, None) => format!("The way to {to} will not open from this side."),
                Note::Opened(to) => format!("A shortcut opens back to {to}."),
                Note::Turned => "You turn back.".into(),
                Note::Held(kind, false) => format!("The way on is held. The {kind:?} keeping it must fall."),
                Note::Held(_, true) => "The way on is held while any of them stands.".into(),
                Note::Recovered(n) => format!("{n} {} recovered", world::CURRENCY),
                Note::Forged(what, n) => format!("{what} reinforced to +{n}"),
                Note::Capped(what, cap) => format!("{what} gains {cap}"),
                Note::Met(who) => format!("{who} will wait at the shrines"),
                Note::Found(what) => format!("Found: {what}"),
                Note::Wore(what) => format!("Wearing {what}"),
                Note::WoreWeak(what, wants) => format!("Wearing {what}: too weak to wield it well (it wants {wants})"),
                Note::Grip(what, one) => format!("Holding the {what} in {}", if *one { "one hand" } else { "both hands" }),
                Note::Crafted(what) => format!("The smith forged the {what}"),
                Note::Fletched(n, what) => format!("The smith fletched {n} {what}"),
                Note::Vanquished(who, souls) => format!("{who} falls  ·  {souls} {}", world::CURRENCY),
                Note::Ambushed(false) => "Ambush!".into(),
                Note::Ambushed(true) => "The chest was a trap!".into(),
                Note::Pilgrimage(n) => format!("The road ends. The pilgrimage begins again ({n}), and the dead are stronger."),
                Note::Bounty(notice) => format!("Bounty taken: {notice}"),
                Note::Paid(who, n) => format!("{who} pays {n} {}", world::CURRENCY),
                Note::DeadEnd => "No way on from here. Turn back the way you came.".into(),
                Note::Tallied(n) => format!("The Tally collects: +{n} {}", world::CURRENCY),
                Note::Called(who) => who.answered().into(),
                Note::Fled(who) => format!("{who} melts into the dark."),
                Note::Bled => "The carved heart drinks. Stone shifts, and it opens.".into(),
                Note::Told(line) => (*line).into(),
            };
            // In the log too, so a hitch there can be put beside what the hero was told.
            #[cfg(target_os = "android")]
            log::info!("[note] {line}");
            // One at a time: a line said while another shows waits its turn (a boss's fall tells
            // its souls, its key and its remnant together), the oldest dropped past a few.
            if self.banner.1 > 0 {
                if self.banners.len() >= BANNERS_HELD { self.banners.pop_front(); }
                self.banners.push_back(line);
            } else {
                self.banner = (line, BANNER_T);
            }
        }
    }

    /// Whether the road shown is the sanctum's (no enemies, no road props).
    /// What the road ends at, walked into a dead end: a wall of its own walls' stone (the guild's
    /// board in its foot), or, out where it has no walls, fallen stones heaped across the path.
    fn end_barrier(&self) -> quartz_path_forge::Barrier {
        let scene = crate::assets::path(format!("scenes/{}", self.world.here().scene));
        let walled = std::fs::read(scene).ok().and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_none_or(|v| v["walls"]["enabled"].as_bool() != Some(false));
        let art = |rel: &str| crate::assets::path(rel).to_string_lossy().into_owned();
        if !walled { return quartz_path_forge::Barrier { wall: false, sprite: art(FALLEN_STONES_ART), ..quartz_path_forge::Barrier::default() }; }
        let board = self.world.gruk_at().is_some().then(|| art(BOARD_ART));
        quartz_path_forge::Barrier { wall: true, sprite: board.unwrap_or_default(), height: 2.5, ..quartz_path_forge::Barrier::default() }
    }

    fn at_rest(&self) -> bool { matches!(self.mode, Mode::Sanctum(_) | Mode::Cut(After::Sanctum) | Mode::Out(Next::Road)) }
    /// Whether the road's own things (enemies, bars) are shown: not at rest, not on the title.
    fn road_shown(&self) -> bool { !self.at_rest() && !matches!(self.mode, Mode::Front(_) | Mode::Out(Next::Start) | Mode::Out(Next::Opening(_)) | Mode::Scene(_)) }

    /// One tick of the cutscene playing, if one is: what its shot starts and cues do to the road
    /// and the game, the canvas camera closing in, and the bars, words, title, black and white over
    /// it all. A key that backs out or uses skips to the end. False when none is playing; it
    /// ends here, the road and the canvas put back.
    fn run_scene(&mut self, c: &mut Canvas, keys: &[shrine::Key]) -> bool {
        let Some(cs) = self.scene.as_mut() else { return false };
        if keys.iter().any(|k| matches!(k, shrine::Key::Back | shrine::Key::Use)) { cs.skip(); }
        if cs.done() { self.end_scene(c); return false; }
        // Its cuts got ready first; nothing else while it plays, since each holds up a frame.
        if cs.i == 0 && cs.t == 0 {
            if let Some(pf) = c.get_plugin_mut::<PathForgePlugin>() { pf.warm(&cs.stops()); pf.set_warming(false); }
            let wanted = cs.sheets();
            self.cameos.retain(|(name, _)| wanted.contains(name));
            for name in wanted {
                if self.cameos.iter().any(|(n, _)| *n == name) { continue; }
                if let Some(sheet) = load_sheet(name) { self.cameos.push((name, sheet)); }
            }
        }
        let Some(cs) = self.scene.as_mut() else { return false };
        let (start, cues) = (cs.starting().cloned(), cs.cues().into_iter().cloned().collect::<Vec<_>>());
        if let (Some(s), Some(pf)) = (start, c.get_plugin_mut::<PathForgePlugin>()) {
            if let Some((stop, at)) = s.stop { pf.warp_at(stop, at); }
            if let Some(h) = s.hour { pf.set_hour(h); }
            if let Some(g) = s.grade { pf.grade(g); }
        }
        for cue in cues {
            match cue {
                Cue::Swing(spec) => Self::swing(c, &spec),
                Cue::Sound(s, volume, pitch) => self.audio.play_at(c, s, volume, pitch),
                Cue::Flash([r, g, b, a], secs) => c.run(Action::CameraFlash { color: Color(r, g, b, a), duration: secs }),
                Cue::Shake(px, secs) => if let Some(cam) = c.camera_mut() { cam.shake(px, secs) },
                Cue::Slow(scale, secs) => self.slow = ((secs * crate::cutscene::TICKS) as u32, scale),
            }
        }
        let Some(cs) = self.scene.as_ref() else { return false };
        let (bars, black, white, push) = (cs.bars(), cs.black(), cs.white(), cs.push());
        let caption = cs.caption().map(|(w, a)| (w.to_owned(), a));
        let title = cs.title().map(|(b, s, a)| (b.to_owned(), s.to_owned(), a));
        // The canvas camera: closed in on the boss or the hero, or left whole.
        let focus = match push {
            Some((z, Focus::Boss)) => c.get_game_object(BOSS).filter(|o| o.visible).map(|o| (z, (o.position.0 + 0.5 * o.size.0, o.position.1 + 0.45 * o.size.1))),
            Some((z, Focus::Hero)) => c.get_game_object(HERO).map(|o| (z, (o.position.0 + 0.5 * o.size.0, o.position.1 + 0.45 * o.size.1))),
            Some((z, Focus::Middle)) => Some((z, (1920.0, 1080.0))),
            None => None,
        };
        let (z, at) = focus.unwrap_or((1.0, (1920.0, 1080.0)));
        frame_canvas(c, z, at);
        self.draw_cine(c, bars, black, white, caption, title);
        let going = self.scene.as_mut().is_some_and(Cutscene::advance);
        if !going { self.end_scene(c); }
        true
    }

    /// The cutscene over: the road's camera, hour and grade as they were (when it moved them), the
    /// canvas camera whole, its bars and words gone. Ending in white, the white stays for the next.
    fn end_scene(&mut self, c: &mut Canvas) {
        let Some(cs) = self.scene.take() else { return };
        if let Some(pf) = c.get_plugin_mut::<PathForgePlugin>() {
            pf.grade(None);
            pf.set_warming(true);
            if cs.owns_road { pf.set_hour(None); }
        }
        if cs.owns_road { Self::swing(c, r#"{"view":"home","seconds":0.02}"#); }
        frame_canvas(c, 1.0, (1920.0, 1080.0));
        self.slow.0 = self.slow.0.min(1);
        for id in [CINE_TOP, CINE_BOTTOM, CINE_CAPTION, CINE_TITLE, CINE_SUB, CINE_SKIP, CINE_SHADE] { if let Some(o) = c.get_game_object_mut(id) { o.visible = false; } }
        if !cs.ends_white() { if let Some(o) = c.get_game_object_mut(CINE_WHITE) { o.visible = false; } }
    }

    /// The cutscene's overlay: bars `bars` of the frame each, black and white (0..1), its words and
    /// title card with how clear they are.
    fn draw_cine(&mut self, c: &mut Canvas, bars: f32, black: f32, white: f32, caption: Option<(String, f32)>, title: Option<(String, String, f32)>) {
        // From past the canvas's edge to `h` inside it, top and bottom.
        let h = 2160.0 * bars;
        let (w, deep) = (3840.0 + 2.0 * CINE_OVER, CINE_OVER + h.max(1.0));
        for (id, y) in [(CINE_TOP, -CINE_OVER), (CINE_BOTTOM, 2160.0 - h)] {
            if let Some(o) = c.get_game_object_mut(id) {
                o.visible = h > 0.5;
                (o.position, o.size) = ((-CINE_OVER, y), (w, deep));
                o.set_image(Image { shape: ShapeType::Rectangle(0.0, (w, deep), 0.0), image: quartz::white_pixel(), color: Some(Color(0, 0, 0, 255)), recolor: None });
            }
        }
        if let Some(o) = c.get_game_object_mut(CINE_WHITE) {
            o.visible = white > 0.0;
            let size = (w, 2160.0 + 2.0 * CINE_OVER);
            o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: quartz::white_pixel(), color: Some(Color(255, 252, 244, (255.0 * white) as u8)), recolor: None });
        }
        // The band behind the title card, as clear as the card.
        if let Some(o) = c.get_game_object_mut(CINE_SHADE) {
            let a = title.as_ref().map_or(0.0, |t| t.2);
            o.visible = a > 0.01;
            (o.position, o.size) = ((-CINE_OVER, 1300.0), (w, 560.0));
            // One band image for the whole game, made clear or not by its tint: a new image a
            // frame would be a texture upload a frame.
            static BAND: std::sync::OnceLock<Arc<RgbaImage>> = std::sync::OnceLock::new();
            let band = BAND.get_or_init(|| Arc::new(RgbaImage::from_fn(1, 64, |_, y| { let u = (y as f32 + 0.5) / 64.0 * 2.0 - 1.0; image::Rgba([0, 0, 0, (210.0 * (1.0 - u * u).powf(1.5)) as u8]) })));
            if o.visible { o.set_image(Image { shape: ShapeType::Rectangle(0.0, (w, 560.0), 0.0), image: band.clone(), color: Some(Color(255, 255, 255, (255.0 * a) as u8)), recolor: None }); }
        }
        self.fade = (black, black);
        if let Some(o) = c.get_game_object_mut(FADE) {
            o.visible = black > 0.0;
            o.set_image(Image { shape: ShapeType::Rectangle(0.0, COVER, 0.0), image: quartz::white_pixel(), color: Some(Color(0, 0, 0, (255.0 * black) as u8)), recolor: None });
        }
        let s = c.base_scale();
        let (Some(title_face), Some(body)) = (self.title_font.clone().or_else(|| self.font.clone()), self.font.clone()) else { return };
        let words = |o: &mut GameObject, text: &str, size: f32, face: &Arc<Font>, rgb: (u8, u8, u8), a: f32, kern: f32| {
            o.visible = a > 0.01;
            let t = Text::new(vec![Span::new(text.to_owned(), size * s, Some(size * 1.3 * s), face.clone(), Color(rgb.0, rgb.1, rgb.2, (255.0 * a) as u8), kern)], Some(3000.0 * s), Align::Center, None);
            o.set_drawable(Box::new(t));
            o.position.0 = 420.0;
        };
        match (&caption, c.get_game_object_mut(CINE_CAPTION)) {
            (Some((text, a)), Some(o)) => words(o, text, 58.0, &title_face, (232, 222, 200), *a, 0.0),
            (None, Some(o)) => o.visible = false,
            _ => {}
        }
        match (&title, c.get_game_object_mut(CINE_TITLE)) {
            // As big as fits the card's width on one line.
            (Some((big, _, a)), Some(o)) => words(o, big, (2900.0 / (0.82 * big.chars().count().max(1) as f32)).min(190.0), &title_face, (240, 226, 190), *a, 10.0),
            (None, Some(o)) => o.visible = false,
            _ => {}
        }
        match (&title, c.get_game_object_mut(CINE_SUB)) {
            (Some((_, small, a)), Some(o)) => words(o, small, 66.0, &title_face, (214, 200, 172), *a * 0.9, 0.0),
            (None, Some(o)) => o.visible = false,
            _ => {}
        }
        if let Some(o) = c.get_game_object_mut(CINE_SKIP) {
            let t = Text::new(vec![Span::new(if self.touch_ui { "Tap  skip" } else { "Esc  skip" }.into(), 40.0 * s, Some(52.0 * s), body, Color(220, 210, 190, (110.0 * (bars / crate::cutscene::BAR)) as u8), 0.0)], Some(3700.0 * s), Align::Right, None);
            o.set_drawable(Box::new(t));
            o.visible = bars > 0.02;
        }
    }

    /// Whether the hero is on the road with the fight running (the capture waits for it).
    pub(crate) fn playing(&self) -> bool { self.mode == Mode::Play }
    /// Whether a cutscene is playing.
    pub(crate) fn in_scene(&self) -> bool { self.scene.is_some() }
    /// The cutscene playing: its name, shot and ticks into it.
    #[cfg(test)]
    pub(crate) fn scene_at(&self) -> Option<(&'static str, usize, u32)> { self.scene.as_ref().map(|s| (s.name, s.i, s.t)) }

    /// The black eases toward where it is headed.
    /// Whether the road is drawing steadily: its last four new frames each close on the one
    /// before, and the newest recent.
    fn road_flowing(&self) -> bool {
        let at = self.frame_at;
        at[0] > 0 && self.ticks.saturating_sub(at[3]) <= FLOW_GAP && at.windows(2).all(|w| w[1] - w[0] <= FLOW_GAP)
    }

    /// Whether the game is still getting ready: its art being read, or (once, at the start) the
    /// road yet to draw steadily for the title to stand on. Takes the art in when it is read.
    fn start_pending(&mut self) -> bool {
        if self.started { return false; }
        if self.art_due.as_ref().is_some_and(std::thread::JoinHandle::is_finished) {
            match self.art_due.take().map(std::thread::JoinHandle::join) {
                Some(Ok(read)) => self.take_art(read),
                _ => eprintln!("hollow_road: the art could not be read; the stand-ins are drawn"),
            }
        }
        self.started = self.art_due.is_none() && (self.road_flowing() || self.ticks >= u64::from(ROAD_WAIT));
        !self.started
    }

    /// A capture holding the loading screen up over the black (`shots::loading`).
    #[cfg(test)]
    pub(crate) fn hold_loading_for_test(&mut self) { (self.loading_held, self.fade) = (true, (0.99, 1.0)); }
    #[cfg(test)]
    fn loading_held(&self) -> bool { self.loading_held }
    #[cfg(not(test))]
    fn loading_held(&self) -> bool { false }

    fn take_art(&mut self, read: Sheets) {
        (self.hero_sheet, self.enemy_sheets, self.boss_sheet) = (read.hero, read.enemies, read.bell_warden);
        (self.keeper_sheet, self.smith_sheet, self.gruk_sheet) = (read.keeper, read.smith, read.gruk);
    }

    fn fade_step(&mut self, c: &mut Canvas) {
        let (now, to) = self.fade;
        if now == to { return; }
        let next = if to > now { (now + FADE_STEP).min(to) } else { (now - FADE_STEP).max(to) };
        self.fade.0 = next;
        if let Some(o) = c.get_game_object_mut(FADE) {
            o.visible = next > 0.0;
            o.set_image(Image { shape: ShapeType::Rectangle(0.0, COVER, 0.0), image: quartz::white_pixel(), color: Some(Color(0, 0, 0, (255.0 * next) as u8)), recolor: None });
        }
    }

    #[cfg(test)]
    pub(crate) fn touch_held_for_test(&self) -> crate::touch::Held { self.touch_held }
    #[cfg(test)]
    pub(crate) fn stick_for_test(&self) -> Option<((f32, f32), (f32, f32))> { self.pad.stick() }
    #[cfg(test)]
    pub(crate) fn touch_mode_for_test(&self) -> crate::touch::Ui { self.touch_mode() }
    /// Where choice `i` of the menu shown last stood (top, bottom).
    #[cfg(test)]
    pub(crate) fn menu_row_for_test(&self, i: usize) -> (f32, f32) {
        let rows = self.menu_rows.borrow();
        rows.iter().find(|r| r.2 == i && r.3).map(|r| (r.0, r.1)).unwrap_or_else(|| panic!("no choice {i}: {rows:?}"))
    }

    /// What the touch controls are for now.
    fn touch_mode(&self) -> crate::touch::Ui {
        use crate::touch::Ui;
        if self.in_scene() { return Ui::Tap; }
        let shots = |k: &crate::gear::Kit| (k.ranged.is_some(), k.spell_cast().is_some() || k.imbues(), k.art.is_some());
        match &self.mode {
            // The prologue's question of leaving is answered with a tap or BACK; its cards with a tap.
            Mode::Play | Mode::Side(_) if self.prologue.as_ref().is_some_and(|p| p.leaving) => Ui::Menu,
            Mode::Play if self.prologue.as_ref().is_some_and(|p| p.still()) => Ui::Tap,
            Mode::Play => { let (ranged, cast, art) = shots(&self.fight.hero.stats.kit); Ui::Road { prompt: self.prompt.1 > 0.5, ranged, cast, art, turn: self.turnable } }
            Mode::Side(a) => { let (ranged, cast, art) = shots(&a.side.hero.stats.kit); Ui::Side { prompt: self.can_bait(), ranged, cast, art } }
            // The map has nothing to choose but closes with BACK.
            Mode::Map => Ui::Menu,
            // Words with no choices (the title) go on with a tap.
            Mode::Front(_) | Mode::Sanctum(_) | Mode::Meet(_) => match self.panel() {
                Some((p, _, _)) if p.options.is_empty() => Ui::Tap,
                _ => Ui::Menu,
            },
            _ => Ui::None,
        }
    }

    /// The fingers' events since the last tick, through the touch controls, added to `presses`
    /// as the keyboard and mouse would have given them; and what they hold now.
    fn touch_presses(&mut self, presses: &mut Vec<Press>) {
        use crate::touch::{self, Out};
        let touch_ui = self.touch_ui;
        if let Some(p) = &mut self.prologue { p.touch = touch_ui; }
        let ui = self.touch_mode();
        let buttons = touch::layout(ui);
        let mut out = Vec::new();
        for (id, phase, at) in std::mem::take(&mut *self.fingers.lock().unwrap()) { out.extend(self.pad.feed(id, phase, at, ui, &buttons)); }
        if self.pad.seen { self.touch_ui = true; }
        if ui == touch::Ui::None { self.pad.clear(); }
        self.touch_held = self.pad.tick(&mut out);
        let key = |ch: char| Press::Key(Key::Character(ch.to_string()), false);
        for o in out {
            match o {
                Out::Tap(t) => presses.push(Press::Touch(t)),
                // Pause and BACK are Esc: Q is the weapon art in a fight.
                Out::Key('q') => presses.push(Press::Key(Key::Named(NamedKey::Escape), false)),
                Out::Key(ch) => presses.push(key(ch)),
                // A tap on a choice: up or down to it, then take it.
                Out::Pick((_, y)) => {
                    let Some((p, _, _)) = self.panel() else { continue };
                    // Nothing to choose (the prologue's question of leaving): a tap goes on.
                    if p.options.is_empty() { presses.push(key('e')); continue; }
                    let rows = self.menu_rows.borrow();
                    // The row under the thumb, or the nearest one within half a row of it.
                    let near = |&(top, bottom, _, _): &(f32, f32, usize, bool)| if y >= top && y < bottom { 0.0 } else { (top - y).max(y - bottom) / (bottom - top).max(1.0) };
                    let Some(r) = (0..rows.len()).filter(|&r| near(&rows[r]) <= 0.5).min_by(|&a, &b| near(&rows[a]).total_cmp(&near(&rows[b]))) else { continue };
                    let (i, takes) = (rows[r].2, rows[r].3);
                    let (way, n) = if i >= p.choice { ('s', i - p.choice) } else { ('w', p.choice - i) };
                    presses.extend((0..n).map(|_| key(way)));
                    // A line saying more are below (or above) scrolls to them, and takes nothing.
                    if takes { presses.push(key('e')); }
                }
            }
        }
    }

    /// The touch controls as they stand: the buttons for what the screen is for now, the held
    /// ones bright, and the stick's ring where the thumb came down (where one goes, before).
    fn draw_touch(&self, c: &mut Canvas) {
        use crate::touch::{self, Btn, Ui};
        let ui = if self.touch_ui { self.touch_mode() } else { Ui::None };
        let buttons = touch::layout(ui);
        let s = c.base_scale();
        let relay = self.touch_laid.get() != Some((ui, s));
        for (i, b) in Btn::ALL.iter().enumerate() {
            let at = buttons.iter().find(|x| x.btn == *b).copied();
            let down = at.is_some() && self.pad.pressed(*b);
            // Use has no ring of its own: the prompt plate is the button.
            let at = at.filter(|_| *b != Btn::Use);
            for (part, on) in [("ring", at.is_some() && !down), ("glow", down), ("label", at.is_some())] {
                let Some(o) = c.get_game_object_mut(&touch_id(part, i)) else { continue };
                o.visible = on;
                let Some(x) = at.filter(|_| relay) else { continue };
                if part == "label" {
                    let size = (x.r * 0.3).clamp(34.0, 60.0);
                    (o.position, o.size) = ((x.at.0 - x.r, x.at.1 - 0.62 * size), (2.0 * x.r, 1.25 * size));
                    if let Some(font) = &self.font {
                        o.set_drawable(Box::new(Text::new(vec![Span::new(b.label().into(), size * s, Some(size * 1.25 * s), font.clone(), Color(240, 232, 214, 235), 0.0)], Some(2.0 * x.r * s), Align::Center, None)));
                    }
                } else {
                    (o.position, o.size) = ((x.at.0 - x.r, x.at.1 - x.r), (2.0 * x.r, 2.0 * x.r));
                }
            }
        }
        self.touch_laid.set(Some((ui, s)));
        // The stick, on the road and in the yard.
        let stick = matches!(ui, Ui::Road { .. } | Ui::Side { .. });
        let (mid, thumb) = self.pad.stick().unwrap_or((STICK_REST, STICK_REST));
        let (dx, dy) = (thumb.0 - mid.0, thumb.1 - mid.1);
        let k = (touch::RIM / (dx * dx + dy * dy).sqrt().max(1e-3)).min(1.0);
        let knob = (mid.0 + dx * k, mid.1 + dy * k);
        for (id, at, r) in [(TOUCH_STICK, mid, touch::RIM), (TOUCH_KNOB, knob, 120.0)] {
            if let Some(o) = c.get_game_object_mut(id) { o.visible = stick; (o.position, o.size) = ((at.0 - r, at.1 - r), (2.0 * r, 2.0 * r)); }
        }
        // The keyboard's help line says nothing to a thumb.
        if self.touch_ui { if let Some(o) = c.get_game_object_mut(HELP) { o.visible = false; } }
    }

    /// The words and choices to show now, and how.
    fn panel(&self) -> Option<(Panel, Style, f32)> {
        if let Some(p) = self.prologue.as_ref().filter(|_| matches!(self.mode, Mode::Play | Mode::Side(_))) {
            let (panel, card) = p.panel(&self.fight);
            return Some((panel, if card { Style::Title } else { Style::Lesson }, 1.0));
        }
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

    /// The choices in view, `first..end`, of a menu whose choices stand `heights` tall with
    /// `room` for them: the view moves from where it was as little as keeps `choice` in it, and
    /// never leaves room empty at the bottom of a long list.
    fn menu_window(&self, heights: &[f32], room: f32, choice: usize) -> (usize, usize) {
        let n = heights.len();
        if n == 0 { return (0, 0); }
        let sum = |a: usize, b: usize| heights[a..b].iter().sum::<f32>();
        let choice = choice.min(n - 1);
        let mut top = self.menu_top.get().min(choice);
        while top < choice && sum(top, choice + 1) > room { top += 1; }
        while top > 0 && sum(top - 1, n) <= room { top -= 1; }
        let mut end = top + 1;
        while end < n && sum(top, end + 1) <= room { end += 1; }
        self.menu_top.set(top);
        (top, end.max(choice + 1))
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
        // A menu stands on the side away from who is spoken to (the keeper is on the left, the
        // smith on the right, by the road and at rest), as tall as its lines.
        let left = matches!(&self.mode, Mode::Sanctum(s) if s.station() == shrine::Station::Smith) || self.mode == Mode::Map;
        let pad = 60.0;
        // On a touch screen the choices are bigger and set further apart, each a thumb's target.
        let (cs, cl) = if self.touch_ui { (70.0, 1.75) } else { (54.0, 1.25) };
        let look = MenuLook::new(font.clone(), cs * s, cl, &p.options);
        // As wide as its longest choice's first line, within reason: a long one wraps.
        let mw = if style == Style::Lesson { 1320.0 } else { (look.widest(&p.options) / s.max(1e-6) + 2.0 * pad + 24.0).clamp(1560.0, 1900.0) };
        let face = self.title_font.clone().unwrap_or_else(|| font.clone());
        let (title_size, title_col) = match style { Style::Title => (190.0, gold), Style::Died => (220.0, Color(158, 14, 10, al(255.0))), _ => (66.0, gold) };
        let mut spans = vec![Span::new(format!("{}\n", p.title), title_size * s, Some(title_size * 1.25 * s), face, title_col, 0.0)];
        let text = if self.touch_ui { p.text.replace("Esc leaves it", "the pause button leaves it") } else { p.text.clone() };
        if !text.is_empty() { spans.push(span(format!("{}\n", text), if style == Style::Title { 64.0 } else { 52.0 }, cream)); }
        if !p.options.is_empty() { spans.push(span("\n".into(), 30.0, cream)); }
        // Each choice's spans: its first line on the columns, its details under it, warnings red.
        let choices: Vec<Vec<Span>> = p.options.iter().enumerate().map(|(i, (label, can))| {
            let picked = i == p.choice;
            let col = if picked && *can { gold } else if *can { cream } else if picked { Color(196, 176, 140, al(230.0)) } else { Color(170, 164, 156, al(220.0)) };
            let detail = if *can { Color(214, 206, 190, al(205.0)) } else { Color(170, 164, 156, al(180.0)) };
            look.choice(label, picked, (col, detail, Color(226, 88, 70, al(255.0))))
        }).collect();
        // A lesson's last line is how the last try went: warmer, to be read mid-fight.
        let hint = if style == Style::Lesson { Color(244, 214, 150, al(240.0)) } else { dim };
        let hint_text = if self.touch_ui { for_touch(&p.hint) } else { p.hint.clone() };
        let hint = (!hint_text.is_empty()).then(|| span(format!("\n{hint_text}"), if style == Style::Lesson { 44.0 } else { 40.0 }, hint));
        // A menu is as tall as its words laid out, wrapped lines and all.
        let boxed = matches!(style, Style::Menu | Style::Lesson);
        let (wide, align) = (Some(if boxed { mw - 2.0 * pad } else { 3840.0 } * s), if boxed { Align::Left } else { Align::Center });
        let laid = |spans: Vec<Span>| Text::new(spans, wide, align, None).with_tabs(look.tabs.clone());
        // A menu longer than the screen shows the choices that fit, a line for those out of view.
        let (first, end) = match style {
            Style::Menu => {
                let room = (1960.0 - 200.0 - 2.0 * pad) * s - 2.0 * 0.75 * 1.4 * look.size - laid(spans.iter().cloned().chain(hint.clone()).collect()).size().1;
                self.menu_window(&choices.iter().map(|c| laid(c.clone()).size().1).collect::<Vec<_>>(), room, p.choice)
            }
            _ => (0, choices.len()),
        };
        // Where each row's spans begin, the choice it is and whether a tap takes it.
        let mut marks = Vec::with_capacity(end - first + 3);
        if first > 0 { marks.push((spans.len(), first - 1, false)); spans.push(look.more(first, "above", dim)); }
        for (i, row) in choices.into_iter().enumerate().take(end).skip(first) { marks.push((spans.len(), i, true)); spans.extend(row); }
        if end < p.options.len() { marks.push((spans.len(), end, false)); spans.push(look.more(p.options.len() - end, "below", dim)); }
        marks.push((spans.len(), 0, false));
        spans.extend(hint);
        let text = laid(if self.touch_ui { spans.clone() } else { std::mem::take(&mut spans) });
        let mh = text.size().1 / s.max(1e-6) + 2.0 * pad;
        let mx = if left { 120.0 } else { 3840.0 - 120.0 - mw };
        let my = (1960.0 - mh).min(980.0).max(200.0);
        // Where each choice stands, for a tap to pick it: the words laid out up to it, and then
        // through it.
        if self.touch_ui {
            let top = match style { Style::Title => 520.0, Style::Menu => my + pad, Style::Lesson => LESSON_Y + pad, Style::Died => 870.0 };
            let below = |k: usize| top + laid(spans[..k].to_vec()).size().1 / s.max(1e-6);
            *self.menu_rows.borrow_mut() = marks.windows(2).map(|r| (below(r[0].0), below(r[1].0), r[0].1, r[0].2)).collect();
        }
        let (bg, bg_a, at, size) = match style {
            Style::Title => (((0.0, 0.0), (3840.0, 2160.0)), 165.0, (0.0, 520.0), (3840.0, 1400.0)),
            Style::Menu => (((mx, my), (mw, mh)), 215.0, (mx + pad, my + pad), (mw - 2.0 * pad, mh - 2.0 * pad)),
            Style::Died => (((0.0, 800.0), (3840.0, 480.0)), 215.0, (0.0, 870.0), (3840.0, 380.0)),
            // A lesson: on the left under the hero's bars, clear of the fight in the middle and of
            // the callouts and the foe's bar over it.
            Style::Lesson => (((60.0, LESSON_Y), (mw, mh)), 190.0, (60.0 + pad, LESSON_Y + pad), (mw - 2.0 * pad, mh - 2.0 * pad)),
        };
        if let Some(o) = c.get_game_object_mut(PANEL_BG) {
            o.visible = true;
            o.position = bg.0;
            o.size = bg.1;
            o.set_image(Image { shape: ShapeType::Rectangle(0.0, bg.1, 0.0), image: quartz::white_pixel(), color: Some(Color(8, 8, 12, al(bg_a))), recolor: None });
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
            o.set_image(Image { shape: ShapeType::Rectangle(0.0, size, 0.0), image: img, color: None, recolor: None });
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

    /// The bench asked for at launch, if any (dev only): `adb shell setprop debug.hr.bench opening`
    /// on Android, `HR_BENCH=opening` elsewhere. "opening" plays the opening cutscene in a
    /// pilgrimage of its own (`save_dir`) while the frame report runs.
    fn bench() -> Option<&'static str> {
        static BENCH: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
        BENCH.get_or_init(|| Self::dev_prop("debug.hr.bench", "HR_BENCH")).as_deref()
    }

    /// A dev setting: the system property `prop` on Android (`adb shell setprop`), the environment
    /// variable `var` elsewhere; None when unset or empty.
    fn dev_prop(prop: &str, var: &str) -> Option<String> {
        let asked = if cfg!(target_os = "android") {
            std::process::Command::new("getprop").arg(prop).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        } else {
            std::env::var(var).ok()
        };
        asked.filter(|v| !v.is_empty())
    }

    /// On Android, once a second to logcat: the game's updates a second and their spread (median,
/// 95th, worst gap, ms), the road's new frames a second, and PathForge's engine and render time.
#[cfg(target_os = "android")]
fn pace_report(ticks: u64, (road_fps, engine, render_ms): (u32, &str, f32), at: impl Fn() -> String) {
    use std::sync::Mutex;
    use std::time::Instant;
    static PACE: Mutex<Option<(Instant, Instant, Vec<f32>)>> = Mutex::new(None);
    let now = Instant::now();
    let mut p = PACE.lock().unwrap();
    let (start, last, gaps) = p.get_or_insert_with(|| (now, now, Vec::new()));
    gaps.push((now - *last).as_secs_f32() * 1000.0);
    *last = now;
    let secs = (now - *start).as_secs_f32();
    if secs < 1.0 { return; }
    gaps.sort_by(f32::total_cmp);
    let quantile = |q: f32| gaps[((gaps.len() - 1) as f32 * q) as usize];
    // Gaps past a frame and a half are the hitches a player sees.
    let long = gaps.iter().filter(|&&g| g > 25.0).count();
    log::info!("[pace] {:.1} updates/s  gap {:.1}/{:.1}/{:.1} ms  long {long}  road {road_fps} fps  {engine} {render_ms:.1} ms  tick {ticks}  at {}",
        gaps.len() as f32 / secs, quantile(0.5), quantile(0.95), quantile(1.0), at());
    *p = Some((now, now, Vec::new()));
}

/// Tell the road what the world asks.
    fn send(c: &mut Canvas, go: Go) {
        let Some(pf) = c.get_plugin_mut::<PathForgePlugin>() else { return };
        match go {
            Go::Next => pf.next(),
            Go::Choose(b) => { pf.choose(b); }
            Go::Warp(stop) => pf.warp(&stop),
            Go::WarpAt(stop, m) => pf.warp_at(&stop, m),
            Go::TurnInto(stop, m) => pf.turn_into(&stop, m),
            Go::WalkOn(stop, m) => pf.walk_on(&stop, m),
        }
    }

}

impl QuartzPlugin for Pilgrimage {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn name(&self) -> &str { "pilgrimage" }

    fn on_update(&mut self, c: &mut Canvas, dt: f32) {
        TMP_LAP.with(|l| l.borrow_mut().start());
        self.ticks += 1;
        self.dress_hero();
        // The boss fighting now, else the boss of the place the hero is in, drawn from its own
        // art: read on arrival, off this thread, so it is ready when the boss steps out (waited
        // for only if the fight is already on).
        let fighting = match &self.mode { Mode::Side(a) if a.why == Why::Boss => a.side.foes.first().and_then(|f| f.breed.sprite), _ => None };
        if let Some((folder, _)) = fighting.or_else(|| self.world.boss_here().and_then(|(_, b)| b.breed.sprite)) {
            if folder != "bell_warden" { self.boss_art.want(folder); }
        }
        self.boss_art.settle(fighting.is_some());
        // Whoever waits on the road here, read when first wanted (a few small frames).
        if let Some((_, who)) = self.world.visitor_at() {
            if self.visitor_sheet.0 != who.folder() { self.visitor_sheet = (who.folder(), load_sheet(who.folder())); }
        }
        TMP_LAP.with(|l| l.borrow_mut().mark("boss"));
        // Slow motion counts down in real time.
        if self.slow.0 > 0 {
            self.slow.0 -= 1;
            c.set_time_scale(if self.slow.0 > 0 { self.slow.1 } else { 1.0 });
        }
        let mut presses = std::mem::take(&mut *self.presses.lock().unwrap());
        self.touch_presses(&mut presses);
        let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
        // Where the frame shown stands (sprites and warnings go by it) and where the walk is headed
        // (the rules go by that, or a slow frame lets the hero walk on into an enemy).
        let walked = pf.state().walked;
        let ahead = pf.walked_ahead();
        let stop_now = pf.state().stop.clone();
        let side_path = self.world.side_path_at(&self.fight, ahead, &pf.state().side_paths);
        let ambush = if self.mode == Mode::Play { self.world.ambush_due(&self.fight, ahead) } else { None };
        self.turnable = self.prologue.is_none() && self.world.can_turn(&self.fight, ahead);
        // Count the road's new frames (its rate is its own, not the game's).
        if let Some(img) = pf.current_image() {
            if !self.last_frame.as_ref().is_some_and(|l| Arc::ptr_eq(l, img)) {
                self.frames.0 += 1;
                self.last_frame = Some(img.clone());
                self.frame_at.rotate_left(1);
                self.frame_at[3] = self.ticks;
            }
        }
        if self.ticks % 60 == 0 { self.frames = (0, self.frames.0); }
        #[cfg(target_os = "android")]
        Self::pace_report(self.ticks, (self.frames.1, pf.engine(), pf.render_ms()), || format!("{:?} {:?}", pf.state().stop, self.scene.as_ref().map(|s| (s.name, s.i, s.t))));
        let starting = self.start_pending();
        let cut_waiting = matches!(self.mode, Mode::Cut(_)) && self.cut_wait > CUT_SHOWS_LOADING;
        let held = self.loading_held();
        self.loading.draw(c, starting || cut_waiting || held, (starting || held).then_some("HOLLOW ROAD"), self.title_font.as_ref());
        // Nothing moves until the game is ready to be seen.
        if starting { return; }
        self.fade_step(c);
        // In a fight Q is the weapon art: only Esc (and Backspace) pause.
        let fighting = matches!(self.mode, Mode::Play | Mode::Side(_)) && !self.prologue.as_ref().is_some_and(|p| p.still() || p.leaving);
        let mut keys: Vec<shrine::Key> = presses.iter().filter(|p| !(fighting && matches!(p, Press::Key(k, _) if is(k, &["q"], None)))).filter_map(menu_key).collect();
        // A cutscene takes the keys while it plays (they skip it).
        if self.run_scene(c, &keys) { (keys, presses) = (Vec::new(), Vec::new()); }
        // Menus click as they are moved through, taken and backed out of.
        let in_menu = matches!(self.mode, Mode::Front(_) | Mode::Sanctum(_) | Mode::Meet(_) | Mode::Map) || self.prologue.as_ref().is_some_and(|p| p.still());
        if in_menu {
            for k in &keys {
                let s = match k { shrine::Key::Use => Sfx::UiConfirm, shrine::Key::Back => Sfx::UiBack, _ => Sfx::UiMove };
                self.audio.play(c, s);
            }
        }
        // The music follows what the game is doing; a fade to black keeps what was playing, and
        // the fall lets it die away.
        let tune = match &self.mode {
            _ if self.scene.as_ref().is_some_and(|s| s.tune().is_some()) => self.scene.as_ref().and_then(Cutscene::tune),
            Mode::Out(_) | Mode::Cut(_) | Mode::Scene(_) => None,
            Mode::Dying(_) => Some(None),
            Mode::Front(_) => Some(Some(Tune::Title)),
            Mode::Kneel(_) | Mode::Sanctum(_) | Mode::Rise(_) => Some(Some(Tune::Shrine)),
            Mode::Side(a) if a.why == Why::Boss => Some(Some(Tune::Boss)),
            _ if self.prologue.is_some() => Some(Some(Tune::Dark)),
            _ => Some(Some(Tune::of_scene(self.world.here().scene))),
        };
        if let Some(t) = tune { self.audio.music(c, t); }
        TMP_LAP.with(|l| l.borrow_mut().mark("music"));
        let pressed_e = presses.iter().any(|p| matches!(p, Press::Key(k, _) if is(k, &["e"], None)));
        let pressed_t = presses.iter().any(|p| matches!(p, Press::Key(k, _) if is(k, &["t"], None)));
        let pressed_m = presses.iter().any(|p| matches!(p, Press::Key(k, _) if is(k, &["m"], None)));
        // H shows or hides the controls, F3 the developer's numbers, anywhere past the title.
        if !matches!(self.mode, Mode::Front(_)) {
            let was = (self.show_help, self.show_dev);
            if presses.iter().any(|p| matches!(p, Press::Key(k, _) if is(k, &["h"], None))) { self.show_help = !self.show_help; }
            if presses.iter().any(|p| matches!(p, Press::Key(k, _) if is(k, &[], Some(NamedKey::F3)))) { self.show_dev = !self.show_dev; }
            if (self.show_help, self.show_dev) != was { write_settings(&self.saves, (self.show_help, self.show_dev)); }
        }
        let mut notes = Vec::new();
        if pressed_e && self.can_bait() { notes.extend(self.throw_rune()); }

        TMP_LAP.with(|l| l.borrow_mut().mark("pre"));
        // What each mode does with this tick, and what it becomes.
        let mode = std::mem::replace(&mut self.mode, Mode::Play);
        self.mode = match mode {
            Mode::Front(mut f) => {
                let slots = self.slots.clone();
                let chosen = keys.iter().fold(None, |got, k| got.or(f.press(*k, &slots)));
                // The opening bench begins a pilgrimage of its own at once.
                let chosen = chosen.or_else(|| (Self::bench() == Some("opening") && self.ticks > 30).then_some(Begin::New(0, Calling::Wayfarer, false)));
                match chosen {
                    Some(Begin::New(slot, calling, first)) => {
                        (self.slot, self.calling, self.played, self.kept) = (Some(slot), Some(calling), 0.0, None);
                        self.fade.1 = 1.0;
                        // The opening's places got ready while the screen goes dark.
                        if let Some(pf) = c.get_plugin_mut::<PathForgePlugin>() { pf.warm(&Cutscene::opening().stops()); }
                        Mode::Out(Next::Opening(first))
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
            // The prologue keeps the road to itself: no rests, turns, maps or pauses (Esc leaves it).
            Mode::Play if self.prologue.is_some() => Mode::Play,
            Mode::Play if self.fight.hero.dead => { self.banner.1 = 0; self.banners.clear(); Mode::Dying(0) }
            Mode::Play if pressed_e && self.world.can_rest(&self.fight, ahead) => Mode::Kneel(0),
            Mode::Play if pressed_e && self.world.can_meet_smith(&self.fight, ahead) => Mode::Meet(Meeting::smith()),
            Mode::Play if pressed_e && self.world.can_meet_gruk(&self.fight, ahead) => Mode::Meet(Meeting::gruk(&mut self.world)),
            Mode::Play if pressed_e && self.world.can_meet_visitor(&self.fight, ahead) => Meeting::visitor(&mut self.world).map_or(Mode::Play, Mode::Meet),
            Mode::Play if pressed_e && self.world.can_come_back(&self.fight, ahead) => { self.fade.1 = 1.0; Mode::Out(Next::Back) }
            // The alley's strike: no veil, no warning but the rune's flare.
            Mode::Play if self.world.strike_due(&self.fight, ahead).is_some() => match self.world.strike_due(&self.fight, ahead) {
                Some(boss) => { self.hum_until = self.ticks + RUNE_PULSE.2; self.begin_boss(c, boss) }
                None => Mode::Play,
            },
            Mode::Play if pressed_e && self.world.can_bleed(&self.fight, ahead, c.get_plugin::<PathForgePlugin>().and_then(|p| p.state().choose_within)) => {
                self.audio.play_at(c, Sfx::StruckHeavy, 0.8, 0.6);
                c.run(Action::CameraFlash { color: Color(120, 10, 16, 140), duration: 0.8 });
                notes.extend(self.world.bleed(&mut self.fight));
                Mode::Play
            }
            Mode::Play if pressed_e && self.world.can_call(&self.fight, ahead) => {
                // The call: a whistle's two notes, or the stone's hum; the answer is told.
                self.audio.play_at(c, Sfx::Twang, 0.6, 1.8);
                notes.extend(self.world.call());
                if let Some(who) = self.world.called { self.load_ally(who); }
                Mode::Play
            }
            Mode::Play if pressed_e && self.world.at_veil(&self.fight, ahead) => {
                // A knell, the veil burns open, the world darkens; the hero walks on through it.
                self.audio.play_at(c, Sfx::Knell, 1.0, 0.55);
                self.audio.play_at(c, Sfx::Roll, 0.8, 0.6);
                c.run(Action::CameraFlash { color: Color(18, 6, 22, 160), duration: 1.4 });
                Mode::Cross(0)
            }
            Mode::Cross(t) if t >= CROSS_T => match self.world.boss_due(&self.fight, ahead) {
                Some(boss) => self.begin_boss(c, boss),
                None => Mode::Play,
            },
            Mode::Cross(t) => Mode::Cross(t + 1),
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
            Mode::Play if ambush.is_some() => {
                // Foes spring out: the camera swings round, and they are fought along the road.
                let k = ambush.unwrap_or(0);
                let chest = self.world.here().ambushes[k].chest;
                let at: Vec<(&'static side::Breed, f32)> = self.world.ambush_foes(k).into_iter().map(|(kind, s)| (side::breed_of(kind), s)).collect();
                Self::swing(c, SIDE_VIEW);
                let h = &self.fight.hero;
                notes.push(Note::Ambushed(chest));
                let mut side = SideFight::ambush(h.stats, h.hp, h.stamina, h.flasks, &at);
                (side.hero.quiver, side.hero.charges, side.hero.bulwark, side.hero.art) = (h.quiver, h.charges, h.bulwark, h.art);
                let (scale, variant) = (self.world.scale(), self.world.variant());
                side.foes = side.foes.into_iter().map(|f| f.scaled(scale).of(variant)).collect();
                Mode::Side(Box::new(Arena { side, t: 0, done: 0, why: Why::Ambush(k), rune: 0 }))
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
                        self.world.gear.warden = saved.calling == Calling::Warden;
                        self.world.gear.take_old_levels(saved.old_levels.0, saved.old_levels.1);
                        (self.calling, self.played) = (Some(saved.calling), saved.played as f64);
                        self.fight = Fight::new(self.world.foes());
                        let mut stats = saved.stats;
                        stats.kit = self.world.kit();
                        self.fight.hero = crate::fight::Hero { quiver: self.world.ammo, ..crate::fight::Hero::with(stats) };
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
                // A pilgrimage begun at the first shrine, getting up off the ground; from the
                // prologue's road, once the walk is back at the first.
                self.set_out(walked);
                if stop_now.as_deref() != Some(world::START) {
                    self.world.warping = Some(world::START.into());
                    Self::send(c, Go::Warp(world::START.into()));
                    Mode::Cut(After::Wake)
                } else {
                    notes.push(Note::Arrived(self.world.here().title));
                    self.fade.1 = 0.0;
                    Mode::Wake(0)
                }
            }
            // Before a first pilgrimage, its opening; then the prologue or the road.
            Mode::Out(Next::Opening(first)) => { self.scene = Some(Cutscene::opening()); Mode::Scene(Then::Begin(first)) }
            Mode::Scene(then) if self.scene.is_some() => Mode::Scene(then),
            Mode::Scene(Then::Begin(first)) => { self.fade.1 = 1.0; Mode::Out(if first { Next::Prologue } else { Next::Start }) }
            // The trap at the Summit: everyone taken, the hero waking by the slab's fire.
            Mode::Scene(Then::Taken) => {
                let go = self.world.taken();
                Self::send(c, go);
                self.fade.1 = 1.0;
                Mode::Cut(After::Wake)
            }
            Mode::Scene(Then::Conclude) => {
                // The pilgrimage's end: the world begun again, waking at the first shrine.
                let (go, note) = self.world.conclude();
                notes.push(note);
                Self::send(c, go);
                self.fade.1 = 1.0;
                Mode::Cut(After::Wake)
            }
            Mode::Out(Next::Prologue) => {
                self.set_out(walked);
                Self::send(c, Go::Warp(prologue::STOP.into()));
                Mode::Cut(After::Prologue)
            }
            Mode::Cut(After::Prologue) if stop_now.as_deref() == Some(prologue::STOP) => {
                self.fight.enemies.clear();
                let h = &mut self.fight.hero;
                (h.lane, h.x) = (0, 0.0);
                let hand = h.stats.kit.ranged.is_some() || h.stats.kit.spell_cast().is_some();
                self.prologue = Some(Prologue::new(ahead, hand));
                self.fade.1 = 0.0;
                Mode::Play
            }
            Mode::Cut(After::Prologue) => Mode::Cut(After::Prologue),
            Mode::Out(Next::Respawn) => { let go = self.world.died(ahead); Self::send(c, go); Mode::Cut(After::Wake) }
            Mode::Out(Next::Sanctum) => {
                notes.extend(self.world.rest(&mut self.fight));
                // The shrine rested at kept at once, not at the next autosave: a window closed
                // straight after a rest wakes the pilgrimage here.
                self.keep();
                Self::send(c, Go::Warp(world::SANCTUM.into()));
                Mode::Cut(After::Sanctum)
            }
            // (Kept at once too: the fire may have sent the hero to wake at another shrine.)
            Mode::Out(Next::Road) => { let go = self.world.leave_rest(); self.keep(); Self::send(c, go); Mode::Cut(After::Rise) }
            Mode::Cut(After::Sanctum) if stop_now.as_deref() == Some(world::SANCTUM) => {
                let h = &mut self.fight.hero;
                (h.lane, h.x) = (0, 0.0);
                self.fade.1 = 0.0;
                Mode::Sanctum(Sanctum::new())
            }
            Mode::Cut(After::Sanctum) => Mode::Cut(After::Sanctum),
            // The world arrives when the walk shows the warp (`World::tick` below).
            Mode::Cut(after) if self.world.warping.is_some() => Mode::Cut(after),
            // And the road drawing steadily: a place's first frames on a phone come slowly while
            // it is readied, and shown, the hero could step aside but not on.
            Mode::Cut(after) if !self.road_flowing() && self.cut_wait < ROAD_WAIT => { self.cut_wait += 1; Mode::Cut(after) }
            Mode::Cut(after) => {
                self.cut_wait = 0;
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
            // A memory cannot kill: the bout begins again.
            Mode::Side(a) if a.side.hero.dead && a.why == Why::Prologue => {
                if let Some(p) = &mut self.prologue { p.word = ("Again: watch its marks, and roll or guard as they fill.".into(), 240); }
                let mut again = self.prologue_bout();
                again.t = ARENA_IN;
                Mode::Side(again)
            }
            Mode::Side(a) if a.side.hero.dead => {
                Self::swing(c, "home");
                let h = &mut self.fight.hero;
                (h.hp, h.dead) = (0.0, true);
                Mode::Dying(0)
            }
            Mode::Side(a) if a.side.won() && a.done >= if a.why == Why::Boss { ARENA_OUT } else { AMBUSH_OUT } && self.scene.is_none() => {
                // Exodus down is only the first of him: Voryn's Vessel gets up in his yard.
                let possessed = a.why == Why::Boss && a.side.foes.first().is_some_and(|f| std::ptr::eq(f.breed, &crate::order::EXODUS));
                let risen = if self.world.fins_saved() { &crate::order::VORYN_VESSEL } else { &crate::order::VORYN_WHOLE };
                match a.why {
                    Why::Boss if possessed => {}
                    Why::Boss => notes.extend(self.world.boss_felled()),
                    Why::Ambush(k) => {
                        // The risen a gravecaller called up leave nothing.
                        let fallen = a.side.foes.iter().filter(|f| !std::ptr::eq(f.breed, &side::RISEN)).filter_map(|f| f.breed.kind).collect();
                        notes.extend(self.world.ambush_cleared(k, fallen));
                    }
                    Why::Prologue => if let Some(p) = &mut self.prologue { p.begin(Lesson::End, &mut self.fight, ahead) },
                }
                if !possessed { Self::swing(c, "home"); }
                let h = &mut self.fight.hero;
                (h.hp, h.stamina, h.flasks, h.quiver, h.charges, h.bulwark) = (a.side.hero.hp, a.side.hero.stamina, a.side.hero.flasks, a.side.hero.quiver, a.side.hero.charges, a.side.hero.bulwark);
                h.art = a.side.hero.art;
                // The Archangel down: out of the white, the Eye made whole, and the trap.
                if possessed { self.begin_boss(c, risen) } else if a.why == Why::Boss && self.world.here().id == atlas::SUMMIT { self.scene = Some(Cutscene::trap(self.world.fins_with_you(), self.world.met("olekeen_bound"))); Mode::Scene(Then::Taken) } else { Mode::Play }
            }
            Mode::Side(a) => Mode::Side(a),
            Mode::Meet(mut m) => {
                if keys.iter().fold(false, |done, k| done || m.press(*k)) {
                    if m.with == shrine::Met::Smith { self.world.met_smith = true; }
                    notes.append(&mut m.then);
                    if let Some(id) = m.answer { notes.extend(self.world.answer(id)); }
                    // The Eye chosen for: the pilgrimage's end, as chosen.
                    if let Some(end) = m.answer.and_then(crate::finale::Ending::of) {
                        self.scene = Some(Cutscene::ending(end, &self.world.fates()));
                        Mode::Scene(Then::Conclude)
                    } else { Mode::Play }
                } else {
                    Mode::Meet(m)
                }
            }
        };

        // The fight runs on game time (slower in slow motion, not at all in a hitstop), and only
        // on the road: playing, or falling.
        let fighting = matches!(self.mode, Mode::Play | Mode::Dying(_));
        // A prologue card (or its question of leaving) holds the road and the fight still.
        let still = self.prologue.as_ref().is_some_and(|p| p.still());
        let mut events = Vec::new();
        let mut ran = 0;
        let mut input = Input::default();
        if fighting {
            input = self.input(c, if self.mode == Mode::Play && !still { &presses } else { &[] });
            if still { input = Input::default(); }
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
            let bi = if self.scene.is_some() { SideInput::default() } else { self.side_input(c, &presses) };
            self.owed += dt / Canvas::TICK;
            let mut first = true;
            while self.owed >= 1.0 {
                self.owed -= 1.0;
                ran += 1;
                let i = if first { bi } else { SideInput { left: bi.left, right: bi.right, run: bi.run, guard: bi.guard, ..SideInput::default() } };
                if first { self.side_pending = SideInput::default(); }
                first = false;
                let Mode::Side(a) = &mut self.mode else { break };
                // Nothing moves while the camera swings round, or while the prologue asks to leave.
                if a.t < ARENA_IN { a.t += 1; continue; }
                // A boss's entrance holds the yard still.
                if still || self.scene.as_ref().is_some_and(|s| s.freeze) { continue; }
                a.rune = a.rune.saturating_sub(1);
                if a.side.won() {
                    a.done += 1;
                    // Its last words, while it falls, before what it leaves is told.
                    if a.done == 1 && a.why == Why::Boss {
                        if let Some(words) = self.world.boss_here().and_then(|(_, b)| b.breed.last_words()) { self.banner = (words.to_string(), ARENA_OUT); }
                    }
                    // The Archangel falls into white, and the trap; Exodus falls, and the Eye is there.
                    if a.done == 1 && a.why == Why::Boss && self.scene.is_none() {
                        let here = self.world.here().id;
                        if here == atlas::SUMMIT { self.scene = Some(Cutscene::fall("The Archangel falls, and the last piece of the Eye is yours.", true)); }
                        let first = a.side.foes.first().is_some_and(|f| std::ptr::eq(f.breed, &crate::order::EXODUS));
                        let words = if self.world.fins_saved() { "The Vessel comes apart. What wore Exodus goes back into the Eye like smoke, and the Eye lies looking at nothing." }
                            else { "Unit 0 comes apart at every joint. The violet goes out of it last, back into the Eye, and the Eye lies looking at nothing." };
                        if here == atlas::FINALE { self.scene = Some(if first { Cutscene::possession(!self.world.fins_saved()) } else { Cutscene::fall(words, false) }); }
                    }
                }
                events.extend(a.side.tick(&i));
            }
            // The bars and the HUD read the road fight's hero.
            if let Mode::Side(a) = &self.mode {
                let (h, s) = (&mut self.fight.hero, &a.side.hero);
                (h.hp, h.stamina, h.flasks, h.quiver, h.charges, h.art) = (s.hp.max(0.0), s.stamina, s.flasks, s.quiver, s.charges, s.art);
            }
        } else {
            (self.owed, self.pending) = (0.0, Input::default());
        }
        TMP_LAP.with(|l| l.borrow_mut().mark("fight"));
        #[cfg(test)]
        events.append(&mut self.staged);
        // At a fork the walk waits at the signpost for a side to be picked.
        let held = self.world.holds_at_fork(c.get_plugin::<PathForgePlugin>().and_then(|p| p.state().choose_within), self.fight.hero.lane);
        let speed = if self.mode != Mode::Play || held || still { 0.0 } else { self.fight.speed(&input, ahead) };
        // Walking and running pick up and settle rather than starting and stopping dead; a move's
        // own travel (a roll, an art) and every hold below keep their exact speed.
        let speed = if self.mode == Mode::Play && !held && self.fight.hero.act.is_none_or(|a| matches!(a.mv, Move::Lane(_))) { self.eased_walk(speed, dt) } else { speed };
        // A way held: no further on at the place's end, nor through the veil before a
        // standing boss (back is still free); stepping through it, at its own pace.
        let speed = if speed > 0.0 && (self.world.held_at_end(&self.fight, ahead) || self.world.held_at_veil(ahead)) { 0.0 } else { speed };
        let speed = if matches!(self.mode, Mode::Cross(_)) { CROSS_SPEED } else { speed };
        // A cutscene on the road walks it at its own pace.
        let speed = self.scene.as_ref().filter(|s| s.owns_road).map_or(speed, Cutscene::walk);
        if let Some(pf) = c.get_plugin_mut::<PathForgePlugin>() { pf.set_speed(speed); }
        self.last_speed = speed;
        // The stride lengthens with the pace, so the feet keep to the ground through a pick-up.
        let pace = ((speed.abs() - crate::fight::WALK) / (crate::fight::RUN - crate::fight::WALK)).clamp(0.0, 1.0);
        self.stride = (self.stride + speed / 60.0 / (WALK_STRIDE + (RUN_STRIDE - WALK_STRIDE) * pace)).rem_euclid(1.0);
        // The prologue's lesson, judged on what the fight just did.
        let lesson_was = self.prologue.as_ref().map(|p| p.lesson);
        if let Some(p) = self.prologue.as_mut().filter(|_| matches!(self.mode, Mode::Play | Mode::Side(_))) {
            let road = if matches!(self.mode, Mode::Play) { &events[..] } else { &[] };
            match p.tick(&mut self.fight, ahead, road, &keys, self.last_speed >= crate::fight::RUN) {
                Some(Step::Side) => {
                    Self::swing(c, SIDE_VIEW);
                    self.fight.enemies.clear();
                    self.mode = Mode::Side(self.prologue_bout());
                }
                Some(Step::Done) => {
                    if matches!(self.mode, Mode::Side(_)) { Self::swing(c, "home"); }
                    self.prologue = None;
                    // The memory's road is not arrived at on the way out: nothing is told of it.
                    self.world.warping = Some(world::START.into());
                    self.fade.1 = 1.0;
                    self.mode = Mode::Out(Next::Start);
                }
                None => {}
            }
        }
        // A lesson learned (not a card read): a chime.
        if let (Some(was), Some(now)) = (lesson_was, self.prologue.as_ref().map(|p| p.lesson)) {
            if was != now && !matches!(was, Lesson::Memory | Lesson::Pace | Lesson::Turn) { self.audio.play(c, Sfx::Lesson); }
        }
        #[cfg(test)]
        self.log.extend(events.iter().map(|e| (self.ticks, *e)));
        self.show(c, &events);
        self.audio.on_events(c, &events);
        if matches!(self.mode, Mode::Play | Mode::Cross(_)) { self.audio.steps(c, walked, speed); }
        self.fx.tick(ran);
        // A follow-through runs on fight ticks, and gives way to whatever the hero does next.
        self.flourish = if self.fight.hero.act.is_some() || self.fight.hero.reeling > 0 { 0 } else { self.flourish.saturating_sub(ran) };
        match &self.mode {
            Mode::Side(a) => {
                let (hs, fw) = (a.side.hero.s, a.side.hero.facing);
                // A rune goes off where the yard puts it, which is road metres past the arena's.
                let events: Vec<Event> = events.iter().map(|e| match *e { Event::Burst { enemy, at } => Event::Burst { enemy, at: walked + ARENA_D as f64 + at },
                        Event::Rung { enemy, at } => Event::Rung { enemy, at: walked + ARENA_D as f64 + at },
                        Event::Slam { enemy, at, heavy } => Event::Slam { enemy, at: walked + ARENA_D as f64 + at, heavy },
                        Event::Popped { enemy, at } => Event::Popped { enemy, at: walked + ARENA_D as f64 + at }, e => e }).collect();
                self.fx.on_events_at(&events, (0.0, ARENA_D + hs), fw, |i| a.side.foes.get(i).map(|f| (0.0, walked + (ARENA_D + f.s) as f64)));
            }
            _ => self.fx.on_events(&events, &self.fight, ahead),
        }
        // Played time, and the pilgrimage kept as it goes.
        if !matches!(self.mode, Mode::Front(_)) { self.played += dt as f64; }
        if self.ticks % KEEP_EVERY == 0 { self.keep(); }
        TMP_LAP.with(|l| l.borrow_mut().mark("keep"));
        // Souls (and shards) for the fallen (the boss's come when the fight ends).
        let road = !matches!(self.mode, Mode::Side(_));
        let fallen: Vec<Archetype> = events.iter().filter(|_| road).filter_map(|e| match *e { Event::Felled { enemy } => self.fight.enemies.get(enemy).filter(|e| !e.grave.raised).map(|e| e.kind), _ => None }).collect();
        if road && events.iter().any(|e| matches!(e, Event::Parried { .. })) { self.world.parried = true; }
        let found = self.world.felled(fallen);
        self.dead_for = if let Mode::Dying(t) = self.mode { t } else { 0 };
        // The world follows the road everywhere but the sanctum (it is not a place).
        let tutoring = self.prologue.is_some() || matches!(self.mode, Mode::Out(Next::Prologue) | Mode::Cut(After::Prologue));
        // (Nor in a cutscene on its own: its cuts to other places are not the hero going there.)
        if !self.at_rest() && !matches!(self.mode, Mode::Front(_) | Mode::Scene(_) | Mode::Out(Next::Opening(_))) && !tutoring {
            let Some(pf) = c.get_plugin::<PathForgePlugin>() else { return };
            let st = pf.state();
            let (stop, within) = (st.stop.clone(), st.choose_within);
            self.world.follow(st.walked, crate::world::Underway { travel: st.travel, handover: st.handover });
            let (go, n) = self.world.tick(&mut self.fight, ahead, stop.as_deref(), within);
            notes.extend(n);
            if let Some(go) = go { Self::send(c, go); }
            // The gates on the fork ahead's ways, once it is in view: a wall across the way with a
            // gate (a key's) or a door (a shortcut's) in it, shut, or standing open once opened.
            // Sent again for each new fork.
            let gates = self.world.gates_ahead();
            let want = self.world.signpost.map(|_| (stop.clone().unwrap_or_default(),
                [quartz_path_forge::Branch::Left, quartz_path_forge::Branch::Right].map(|b| gates.iter().find(|g| g.0 == b).map(|&(_, keyed, open)| (keyed, open)))));
            if want != self.bar_sent {
                let bar = |g: Option<(bool, bool)>| g.map(|(k, open)| quartz_path_forge::Barrier {
                    wall: true, sprite: crate::assets::path(if k { BARRIER_ART.0 } else { BARRIER_ART.1 }).to_string_lossy().into_owned(), height: 3.0,
                    open, leaves: if k { BARRIER_LEAVES.0 } else { BARRIER_LEAVES.1 } });
                if let (Some((_, [l, r])), Some(pf)) = (&want, c.get_plugin_mut::<PathForgePlugin>()) { pf.bar(bar(*l), bar(*r)); }
                self.bar_sent = want;
            }
        }
        // The wall the road ends at, walked into a dead end (the guild's board in its foot).
        let end = self.world.end_wall().filter(|_| !self.at_rest() && !tutoring && !matches!(self.mode, Mode::Front(_) | Mode::Scene(_)));
        if end != self.end_sent {
            let wall = self.end_barrier();
            if let Some(pf) = c.get_plugin_mut::<PathForgePlugin>() { pf.set_end(end.map(|at| (at, wall))); }
            self.end_sent = end;
        }
        TMP_LAP.with(|l| l.borrow_mut().mark("world"));
        notes.extend(found);
        for n in &notes {
            match n {
                Note::Rose(_) => self.audio.play(c, Sfx::LevelUp),
                Note::Recovered(_) | Note::Paid(..) | Note::Tallied(_) => self.audio.play(c, Sfx::Coins),
                Note::Found(_) | Note::Crafted(_) | Note::Forged(..) | Note::Fletched(..) | Note::Wore(_) => self.audio.play(c, Sfx::Pickup),
                Note::Capped(..) => self.audio.play(c, Sfx::LevelUp),
                Note::WoreWeak(..) => self.audio.play(c, Sfx::UiBack),
                Note::Grip(..) => self.audio.play(c, Sfx::UiConfirm),
                Note::Vanquished(..) => self.audio.play_at(c, Sfx::Knell, 1.0, 0.8),
                _ => {}
            }
        }
        self.ravens(&mut notes);
        if !notes.is_empty() { self.say(&notes); }
        TMP_LAP.with(|l| l.borrow_mut().mark("say"));
        self.banner.1 = self.banner.1.saturating_sub(1);
        if self.banner.1 == 0 { if let Some(line) = self.banners.pop_front() { self.banner = (line, BANNER_T); } }
        self.draw_callout(c);
        self.paint_marks(c, walked);
        TMP_LAP.with(|l| l.borrow_mut().mark("marks"));
        self.place(c, walked);
        TMP_LAP.with(|l| l.borrow_mut().mark("place"));
        self.fx.place(c, walked, &self.fight, matches!(self.mode, Mode::Side(_)));
        TMP_LAP.with(|l| l.borrow_mut().mark("fx"));
        self.bars(c, walked);
        TMP_LAP.with(|l| l.borrow_mut().mark("bars"));
        self.draw_panel(c);
        self.draw_prompt(c, ahead);
        self.draw_map(c);
        TMP_LAP.with(|l| l.borrow_mut().mark("panels"));
        if self.ticks % 15 == 1 || self.banner.1 > 0 || self.mode != Mode::Play || events.contains(&Event::Died) { self.hud(c, walked); }
        TMP_LAP.with(|l| l.borrow_mut().mark("hud"));
        self.draw_touch(c);
        TMP_LAP.with(|l| l.borrow_mut().mark("touch"));
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

thread_local! { static TMP_LAP: std::cell::RefCell<TmpLap> = std::cell::RefCell::new(TmpLap::default()); }
#[derive(Default)]
struct TmpLap { at: Option<std::time::Instant>, laps: Vec<(&'static str, f32)> }
impl TmpLap {
    fn start(&mut self) {
        if let Some(at) = self.at.take() {
            let total: f32 = self.laps.iter().map(|l| l.1).sum();
            if total > 20.0 { eprintln!("[lap] {total:.1} ms: {}", self.laps.iter().filter(|l| l.1 >= 1.0).map(|(n, t)| format!("{n} {t:.1}")).collect::<Vec<_>>().join(" | ")); }
            let _ = at;
        }
        self.laps.clear();
        self.at = Some(std::time::Instant::now());
    }
    fn mark(&mut self, name: &'static str) {
        let now = std::time::Instant::now();
        if let Some(at) = self.at { self.laps.push((name, (now - at).as_secs_f32() * 1000.0)); }
        self.at = Some(now);
    }
}
