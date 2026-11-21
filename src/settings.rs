//! The player's settings, kept between launches in `settings.json` beside the profiles: what
//! the screen shows (the controls, the developer's numbers), how loud the game is, and how much
//! of the road is drawn (`Quality`, for older or slower devices).

use quartz_path_forge::Quality;
use std::path::{Path, PathBuf};

/// Steps of a volume: 0 (silent) to `FULL`.
pub const FULL: u8 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub show_controls: bool,
    pub show_dev: bool,
    /// Every sound, then music and effects under it, in tenths.
    pub master: u8,
    pub music: u8,
    pub effects: u8,
    pub quality: Quality,
    /// The way-finding line under the HUD: where the road leads and the ways on found so far.
    pub show_way: bool,
    /// How strongly a blow's ground is lit in how it is met (`Hints`).
    pub hints: Hints,
}

/// The coloured light on the ground under a blow under way (gold: parry; red-orange: roll or
/// block; purple: roll), over what the blow itself shows: none, a faint tint, or all of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hints { Off, Subtle, Full }

impl Hints {
    const ALL: [Hints; 3] = [Hints::Off, Hints::Subtle, Hints::Full];

    pub fn name(self) -> &'static str { match self { Hints::Off => "Off", Hints::Subtle => "Subtle", Hints::Full => "Full" } }

    fn parse(s: &str) -> Option<Hints> { Self::ALL.into_iter().find(|h| h.name().eq_ignore_ascii_case(s)) }

    /// How much of a telegraph's light shows: Subtle only a hint of tint on the ground.
    pub fn strength(self) -> f32 { match self { Hints::Off => 0.0, Hints::Subtle => 0.18, Hints::Full => 1.0 } }
}

impl Default for Settings {
    fn default() -> Self {
        Settings { show_controls: false, show_dev: false, master: FULL, music: FULL, effects: FULL, quality: Quality::High, show_way: false, hints: Hints::Subtle }
    }
}

/// A row of the settings menu, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row { Master, Music, Effects, Graphics, Way, Hints, Back }

impl Row {
    pub const ALL: [Row; 7] = [Row::Master, Row::Music, Row::Effects, Row::Graphics, Row::Way, Row::Hints, Row::Back];
}

impl Settings {
    fn path(saves: &Path) -> PathBuf { saves.parent().unwrap_or(saves).join("settings.json") }

    /// The settings kept, each missing one (an older file's) at its default.
    pub fn read(saves: &Path) -> Settings {
        let v: serde_json::Value = std::fs::read_to_string(Self::path(saves)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let base = Settings::default();
        let tenths = |key: &str, or: u8| v[key].as_u64().map_or(or, |n| n.min(u64::from(FULL)) as u8);
        Settings {
            show_controls: v["show_controls"].as_bool().unwrap_or(base.show_controls),
            show_dev: v["show_dev"].as_bool().unwrap_or(base.show_dev),
            master: tenths("master", base.master),
            music: tenths("music", base.music),
            effects: tenths("effects", base.effects),
            quality: v["quality"].as_str().and_then(Quality::parse).unwrap_or(base.quality),
            show_way: v["show_way"].as_bool().unwrap_or(base.show_way),
            hints: v["hints"].as_str().and_then(Hints::parse).unwrap_or(base.hints),
        }
    }

    /// Keep them; a failure is returned to be told, never fatal.
    pub fn write(&self, saves: &Path) -> Result<(), String> {
        let path = Self::path(saves);
        if let Some(dir) = path.parent() { std::fs::create_dir_all(dir).map_err(|e| e.to_string())?; }
        let json = serde_json::json!({
            "show_controls": self.show_controls, "show_dev": self.show_dev,
            "master": self.master, "music": self.music, "effects": self.effects, "quality": self.quality.name(), "show_way": self.show_way, "hints": self.hints.name(),
        });
        std::fs::write(&path, json.to_string()).map_err(|e| e.to_string())
    }

    /// How loud music plays, of its own level: master times music.
    pub fn music_level(&self) -> f32 { f32::from(self.master) * f32::from(self.music) / f32::from(FULL * FULL) }
    /// How loud the fight, feet and menus play, of their own levels.
    pub fn effects_level(&self) -> f32 { f32::from(self.master) * f32::from(self.effects) / f32::from(FULL * FULL) }

    /// The road's frame size at this quality: fewer, larger pixels draw faster.
    pub fn road_px(&self) -> (u32, u32) {
        match self.quality { Quality::High => (480, 270), Quality::Medium => (400, 225), Quality::Low => (320, 180) }
    }

    /// The most road frames a second at this quality (None: the engine's own cap). A phone
    /// already draws at an even 30.
    pub fn road_fps(&self) -> Option<f32> {
        match (self.quality, cfg!(target_os = "android")) {
            (Quality::High, true) | (Quality::Medium, true) => Some(30.0),
            (Quality::Low, true) => Some(24.0),
            (Quality::High, false) => None,
            (Quality::Medium, false) => Some(45.0),
            (Quality::Low, false) => Some(30.0),
        }
    }

    /// `row` stepped one way (`up`): a volume a tenth, wrapping round when `wrap` (a tap); the
    /// graphics through Low, Medium and High; way-finding on or off.
    pub fn step(&mut self, row: Row, up: bool, wrap: bool) {
        let tenth = |v: &mut u8| *v = match (up, *v) {
            (true, FULL) if wrap => 0,
            (true, n) => (n + 1).min(FULL),
            (false, 0) if wrap => FULL,
            (false, n) => n.saturating_sub(1),
        };
        match row {
            Row::Master => tenth(&mut self.master),
            Row::Music => tenth(&mut self.music),
            Row::Effects => tenth(&mut self.effects),
            Row::Graphics => {
                let at = Quality::ALL.iter().position(|q| *q == self.quality).unwrap_or(2);
                let next = if up { (at + 1) % Quality::ALL.len() } else { (at + Quality::ALL.len() - 1) % Quality::ALL.len() };
                self.quality = Quality::ALL[next];
            }
            Row::Way => self.show_way = !self.show_way,
            Row::Hints => {
                let at = Hints::ALL.iter().position(|h| *h == self.hints).unwrap_or(1);
                let n = Hints::ALL.len();
                self.hints = Hints::ALL[if up { (at + 1) % n } else { (at + n - 1) % n }];
            }
            Row::Back => {}
        }
    }

    /// `row` as the menu lists it.
    pub fn line(&self, row: Row) -> String {
        let pct = |v: u8| format!("{}%", u32::from(v) * 10);
        match row {
            Row::Master => format!("Volume  {}", pct(self.master)),
            Row::Music => format!("Music  {}", pct(self.music)),
            Row::Effects => format!("Effects  {}", pct(self.effects)),
            Row::Graphics => format!("Graphics  {}", self.quality.name()),
            Row::Way => format!("Way-finding  {}", if self.show_way { "On" } else { "Off" }),
            Row::Hints => format!("Attack hints  {}", self.hints.name()),
            Row::Back => "Back".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_step_wrap_and_come_back_from_their_file() {
        let mut s = Settings::default();
        s.step(Row::Music, true, true);
        assert_eq!(s.music, 0, "a tap past full wraps to silent");
        s.step(Row::Music, true, false);
        s.step(Row::Master, false, false);
        assert_eq!((s.music, s.master), (1, 9));
        s.step(Row::Graphics, true, true);
        s.step(Row::Way, true, true);
        assert!(s.show_way, "a tap turns way-finding on");
        assert_eq!(Settings::default().hints, Hints::Subtle, "attack hints start faint");
        s.step(Row::Hints, true, true);
        assert_eq!(s.hints, Hints::Full);
        assert_eq!(s.quality, Quality::Low, "High steps round to Low");
        assert!((s.music_level() - 0.09).abs() < 1e-6 && (s.effects_level() - 0.9).abs() < 1e-6);
        let dir = std::env::temp_dir().join(format!("hr_settings_{}", std::process::id())).join("profiles");
        s.write(&dir).unwrap();
        assert_eq!(Settings::read(&dir), s);
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
        assert!(Settings::default().road_px().0 > Settings { quality: Quality::Low, ..Settings::default() }.road_px().0);
    }
}
