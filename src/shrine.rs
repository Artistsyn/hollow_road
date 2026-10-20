//! Everything off the road's fight: the title, choosing a calling, the sanctum where the hero sits
//! at rest, what is said there, and the menus for levels and the smith's forge. No drawing and no
//! engine here, so it is tested on its own; `game` sends it keys and draws the `Panel` it returns.
//!
//! The sanctum is its own scene (the journey's `sanctum` stop): a clearing ringed by mist. The walk
//! is held there; the lanes are where the hero can stand. On the left the keeper (levels, and her
//! words), in the middle the shrine (rise and walk on), on the right the smith once he has been met
//! on the road (his forge, and his words). Nothing leaves until the hero rises at the shrine.
//!
//! The keeper (Maren) and the smith (Brannoc) are stand-ins: their names and lines wait on the
//! story (the design doc's wanderers, its open question 11).

use crate::fight::{Fight, GEAR_MAX};
use crate::gear::{ITEMS, SLOTS};
use crate::world::{Note, World};

/// A calling: the direction a pilgrim starts in (design doc §8.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Calling { Wayfarer, Warden, Bloodrunner, Frostweaver }

impl Calling {
    pub const ALL: [Calling; 4] = [Calling::Wayfarer, Calling::Warden, Calling::Bloodrunner, Calling::Frostweaver];
    pub fn name(self) -> &'static str {
        match self { Calling::Wayfarer => "Wayfarer", Calling::Warden => "Warden", Calling::Bloodrunner => "Bloodrunner", Calling::Frostweaver => "Frostweaver" }
    }
    pub fn blurb(self) -> &'static str {
        match self {
            Calling::Wayfarer => "Sword and buckler. Rolls, parries and ripostes: the road's fight as it stands.",
            Calling::Warden => "Armoured shield-bearer. Blocks charge a ward; a perfect block bursts it.",
            Calling::Bloodrunner => "Hunter who spends their own blood on arrows: health as ammunition.",
            Calling::Frostweaver => "Rune caster. Focus feeds frost chains that bind and shields that turn a blow.",
        }
    }
    /// Whether it can be played yet (the others are listed so the road ahead is visible).
    pub fn playable(self) -> bool { self == Calling::Wayfarer }
}

/// The keys these screens listen to, from whatever device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key { Left, Right, Up, Down, Use, Back }

/// What to draw: a heading (who speaks, or where), the words, the choices with whether each can be
/// taken and which is picked, and a line of keys.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Panel {
    pub title: String,
    pub text: String,
    pub options: Vec<(String, bool)>,
    pub choice: usize,
    pub hint: String,
}

/// The title, then the calling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Front { Title, Callings(usize) }

impl Front {
    /// A key; the calling once one is chosen.
    pub fn press(&mut self, k: Key) -> Option<Calling> {
        match (*self, k) {
            (Front::Title, Key::Use) => *self = Front::Callings(0),
            (Front::Callings(i), Key::Up | Key::Left) => *self = Front::Callings(i.saturating_sub(1)),
            (Front::Callings(i), Key::Down | Key::Right) => *self = Front::Callings((i + 1).min(Calling::ALL.len() - 1)),
            (Front::Callings(i), Key::Use) if Calling::ALL[i].playable() => return Some(Calling::ALL[i]),
            (Front::Callings(_), Key::Back) => *self = Front::Title,
            _ => {}
        }
        None
    }

    pub fn panel(&self) -> Panel {
        match *self {
            Front::Title => Panel {
                title: "HOLLOW ROAD".into(),
                text: "The road to Hollowmere.".into(),
                hint: "E or Enter to begin".into(),
                ..Panel::default()
            },
            Front::Callings(i) => Panel {
                title: "Choose a calling".into(),
                text: Calling::ALL[i].blurb().into(),
                options: Calling::ALL.iter().map(|c| (if c.playable() { c.name().to_owned() } else { format!("{}  (not yet)", c.name()) }, c.playable())).collect(),
                choice: i,
                hint: "W/S choose   E begin   Esc back".into(),
            },
        }
    }
}

/// Where the hero can stand in the sanctum, by lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Station { Keeper, Shrine, Smith }

impl Station {
    pub fn lane(self) -> i32 { match self { Station::Keeper => -1, Station::Shrine => 0, Station::Smith => 1 } }
    pub fn at(lane: i32) -> Station { match lane { l if l < 0 => Station::Keeper, 0 => Station::Shrine, _ => Station::Smith } }
    pub fn name(self) -> &'static str { match self { Station::Keeper => "Maren, the keeper", Station::Shrine => "The shrine", Station::Smith => "Brannoc, the smith" } }
}

/// What a press in the sanctum did that the game must act on.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Up from rest: back to the road.
    Rise,
    /// Something to put on the banner (a level, an upgrade).
    Note(Note),
}

#[derive(Clone, Debug, PartialEq)]
enum View {
    /// Walking between the stations.
    Walk,
    /// Words, a page at a time; the station's choices after.
    Words { pages: Vec<String>, page: usize },
    Choices(usize),
    Levels(usize),
    Forge(usize),
    /// The places to wear things, and the pieces that could go in place `p`.
    Gear(usize),
    Wear(usize, usize),
}

/// The hero at rest.
#[derive(Clone, Debug, PartialEq)]
pub struct Sanctum {
    /// The lane the hero stands in (the station in front of them).
    pub lane: i32,
    view: View,
}

const KEEPER_FIRST: [&str; 3] = [
    "You are awake. Good. The road takes most who walk it long before they find a fire like this one.",
    "I keep the shrines on the way to Hollowmere. Bring me the souls of the fallen and I can make you more than you are.",
    "Rest when you must. The dead you put down rise again while you sit. That is the fire's price.",
];
const KEEPER_TALK: [&str; 3] = [
    "The Veil is thin past the marsh. If you hear singing in the reeds, do not answer it.",
    "Hollowmere was a town of bells. Now it has only the one, and something rings it at night.",
    "Every shrine remembers who rested at it. So do I.",
];
const KEEPER_SMITH: &str = "A smith walks this road. Stubborn old man. Find him, and he will not stay far from a warm fire.";
pub const SMITH_FIRST: [&str; 3] = [
    "Hah. Another one still breathing. Sit, sit. No? None of you ever do.",
    "Brannoc. I mend what the road breaks. Bring me shards off those armoured dead and I'll put an edge on that blade worth the name.",
    "I'll find your fire. I always find the fires.",
];
const SMITH_TALK: [&str; 3] = [
    "Shieldbearers carry the good iron. Lancers too, if you get past the point.",
    "Every edge I forge, the road blunts. That's the trade.",
    "I forged for an Order once. Don't ask which.",
];

impl Sanctum {
    /// Sat down at the shrine, facing it.
    pub fn new() -> Sanctum { Sanctum { lane: 0, view: View::Walk } }

    pub fn station(&self) -> Station { Station::at(self.lane) }
    /// Whether a menu or words are open (the hero stands still).
    pub fn talking(&self) -> bool { self.view != View::Walk }

    fn options(&self, w: &World, f: &Fight) -> Vec<(String, bool)> {
        let st = f.hero.stats;
        match (&self.view, self.station()) {
            (View::Choices(_), Station::Keeper) => vec![("Grow stronger".into(), true), ("Talk".into(), true), ("Step away".into(), true)],
            (View::Choices(_), Station::Shrine) => vec![("Rise and walk on".into(), true), ("Equipment".into(), true), ("Stay a while".into(), true)],
            (View::Choices(_), Station::Smith) => vec![("Forge".into(), true), ("Talk".into(), true), ("Step away".into(), true)],
            (View::Levels(_), _) => {
                let can = w.souls >= st.level_cost();
                vec![
                    (format!("Vigor {}   health {:.0}", st.vigor, st.max_hp()), can),
                    (format!("Endurance {}   stamina {:.0}", st.endurance, st.max_stamina()), can),
                    (format!("Might {}   strikes x{:.2}", st.might, 1.0 + 0.06 * st.might as f32), can),
                    ("Back".into(), true),
                ]
            }
            (View::Forge(_), _) => {
                let line = |what: &str, lvl: u32| match World::forge_cost(lvl) {
                    Some((souls, shards)) => (format!("{what} +{lvl} to +{}   {souls} souls, {shards} shards", lvl + 1), w.souls >= souls && w.shards >= shards),
                    None => (format!("{what} +{GEAR_MAX}   as good as it gets"), false),
                };
                vec![line("Weapon", st.weapon), line("Armour", st.armour), ("Back".into(), true)]
            }
            (View::Gear(_), _) => SLOTS.iter().zip(&w.gear.worn)
                .map(|((_, place), worn)| (format!("{place:<9} {}", worn.map_or("nothing", |i| ITEMS[i].name)), true))
                .chain([("Back".to_string(), true)]).collect(),
            (View::Wear(p, _), _) => Self::wearable(w, *p).into_iter()
                .map(|i| (match i { Some(i) => format!("{}   {}", ITEMS[i].name, ITEMS[i].text), None => "Nothing".into() }, true))
                .chain([("Back".to_string(), true)]).collect(),
            _ => Vec::new(),
        }
    }

    /// A key. Moving between stations, opening one, paging words, picking choices.
    pub fn press(&mut self, k: Key, w: &mut World, f: &mut Fight) -> Option<Outcome> {
        let n = self.options(w, f).len();
        match (self.view.clone(), k) {
            (View::Walk, Key::Left) => self.lane = (self.lane - 1).max(-1),
            // The smith's side is empty until he has been met.
            (View::Walk, Key::Right) if self.lane < 0 || w.met_smith => self.lane = (self.lane + 1).min(1),
            (View::Walk, Key::Use) => {
                self.view = if self.station() == Station::Keeper && !w.met_keeper {
                    w.met_keeper = true;
                    View::Words { pages: KEEPER_FIRST.iter().map(|s| s.to_string()).collect(), page: 0 }
                } else {
                    View::Choices(0)
                };
            }
            (View::Words { pages, page }, Key::Use | Key::Right) if page + 1 < pages.len() => self.view = View::Words { pages, page: page + 1 },
            (View::Words { .. }, Key::Use | Key::Right | Key::Back) => self.view = View::Choices(0),
            (View::Choices(i) | View::Levels(i) | View::Forge(i) | View::Gear(i) | View::Wear(_, i), Key::Up) => self.set_choice(i.saturating_sub(1)),
            (View::Choices(i) | View::Levels(i) | View::Forge(i) | View::Gear(i) | View::Wear(_, i), Key::Down) => self.set_choice((i + 1).min(n.saturating_sub(1))),
            (View::Choices(_), Key::Back) => self.view = View::Walk,
            (View::Levels(_) | View::Forge(_), Key::Back) => self.view = View::Choices(0),
            (View::Gear(_), Key::Back) => self.view = View::Choices(1),
            (View::Wear(p, _), Key::Back) => self.view = View::Gear(p),
            (View::Gear(i), Key::Use) if i < SLOTS.len() => self.view = View::Wear(i, 0),
            (View::Gear(_), Key::Use) => self.view = View::Choices(1),
            (View::Wear(p, i), Key::Use) => {
                self.view = View::Gear(p);
                let item = *Self::wearable(w, p).get(i)?;
                if !w.gear.wear(p, item) { return None; }
                Self::refit(w, f);
                return Some(Outcome::Note(Note::Wore(item.map_or("nothing", |i| ITEMS[i].name))));
            }
            (View::Choices(i), Key::Use) => return self.choose(i, w),
            (View::Levels(i), Key::Use) if i < 3 => return w.rise(f, i).map(Outcome::Note),
            (View::Forge(i), Key::Use) if i < 2 => return w.forge(f, i).map(Outcome::Note),
            (View::Levels(_), Key::Use) => self.view = View::Choices(0),
            (View::Forge(_), Key::Use) => self.view = View::Choices(0),
            _ => {}
        }
        None
    }

    fn set_choice(&mut self, i: usize) {
        self.view = match self.view {
            View::Choices(_) => View::Choices(i), View::Levels(_) => View::Levels(i), View::Forge(_) => View::Forge(i),
            View::Gear(_) => View::Gear(i), View::Wear(p, _) => View::Wear(p, i), ref v => v.clone(),
        };
    }

    /// What could go in place `p`: the pieces owned that fit, and nothing (but never no weapon).
    fn wearable(w: &World, p: usize) -> Vec<Option<usize>> {
        let mut v: Vec<Option<usize>> = w.gear.fits(p).into_iter().map(Some).collect();
        if p != 0 { v.push(None); }
        v
    }

    /// The hero as now dressed: the kit's changes, health and stamina kept within their new tops.
    fn refit(w: &World, f: &mut Fight) {
        let h = &mut f.hero;
        h.stats.kit = w.gear.kit();
        h.hp = h.hp.min(h.stats.max_hp());
        h.stamina = h.stamina.min(h.stats.max_stamina());
        h.poise = h.poise.min(h.stats.poise());
    }

    fn choose(&mut self, i: usize, w: &mut World) -> Option<Outcome> {
        let words = |list: &[&str], k: u32| View::Words { pages: vec![list[k as usize % list.len()].to_string()], page: 0 };
        match (self.station(), i) {
            (Station::Keeper, 0) => self.view = View::Levels(0),
            (Station::Keeper, 1) => {
                let k = w.talks[0];
                w.talks[0] += 1;
                // Every third word from her is about the smith until he has been met.
                self.view = if !w.met_smith && k % 3 == 2 { View::Words { pages: vec![KEEPER_SMITH.into()], page: 0 } } else { words(&KEEPER_TALK, k) };
            }
            (Station::Shrine, 0) => { self.view = View::Walk; return Some(Outcome::Rise); }
            (Station::Shrine, 1) => self.view = View::Gear(0),
            (Station::Smith, 0) => self.view = View::Forge(0),
            (Station::Smith, 1) => { let k = w.talks[1]; w.talks[1] += 1; self.view = words(&SMITH_TALK, k); }
            _ => self.view = View::Walk,
        }
        None
    }

    /// What to draw now.
    pub fn panel(&self, w: &World, f: &Fight) -> Panel {
        let st = self.station();
        let mut p = Panel { title: st.name().into(), options: self.options(w, f), ..Panel::default() };
        match &self.view {
            View::Walk => {
                p.text = match st {
                    Station::Keeper => "She watches the fire, and you.".into(),
                    Station::Shrine => "The fire burns low and steady. Nothing follows you here.".into(),
                    Station::Smith => "He works an edge by the firelight, muttering.".into(),
                };
                p.hint = "A/D move   E approach".into();
            }
            View::Words { pages, page } => { p.text = pages[*page].clone(); p.hint = "E continue".into(); }
            View::Choices(i) => { p.choice = *i; p.hint = "W/S choose   E take   Esc back".into(); }
            View::Levels(i) => {
                p.title = "Grow stronger".into();
                p.text = format!("Souls {}.   Level {}: the next costs {}.", w.souls, f.hero.stats.level(), f.hero.stats.level_cost());
                p.choice = *i;
                p.hint = "W/S choose   E buy   Esc back".into();
            }
            View::Forge(i) => {
                p.title = "The forge".into();
                p.text = format!("Souls {}.   Shards {}.", w.souls, w.shards);
                p.choice = *i;
                p.hint = "W/S choose   E forge   Esc back".into();
            }
            View::Gear(i) | View::Wear(_, i) => {
                let st = &f.hero.stats;
                p.title = match &self.view { View::Wear(q, _) => SLOTS[*q].1.into(), _ => "Equipment".into() };
                p.text = format!(
                    "Carrying {:.1} of {:.0}{}.   Health {:.0}  stamina {:.0}  poise {:.0}  strikes x{:.2}  blows x{:.2}",
                    st.kit.load, st.capacity(), if st.burdened() { ", burdened: rolls cost more" } else { "" },
                    st.max_hp(), st.max_stamina(), st.poise(), st.damage(), st.taken(),
                );
                p.choice = *i;
                p.hint = "W/S choose   E wear   Esc back".into();
            }
        }
        p
    }
}

impl Default for Sanctum { fn default() -> Self { Sanctum::new() } }

/// Words with someone met on the road, a page at a time.
#[derive(Clone, Debug, PartialEq)]
pub struct Meeting { pub who: &'static str, pub pages: Vec<String>, pub page: usize }

impl Meeting {
    pub fn smith() -> Meeting { Meeting { who: Station::Smith.name(), pages: SMITH_FIRST.iter().map(|s| s.to_string()).collect(), page: 0 } }
    /// E turns the page; true once the last is read.
    pub fn press(&mut self, k: Key) -> bool {
        if !matches!(k, Key::Use | Key::Right | Key::Back) { return false; }
        if self.page + 1 < self.pages.len() && k != Key::Back { self.page += 1; false } else { true }
    }
    pub fn panel(&self) -> Panel {
        Panel { title: self.who.into(), text: self.pages[self.page].clone(), hint: "E continue".into(), ..Panel::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fight::Fight;

    fn setup() -> (Sanctum, World, Fight) { (Sanctum::new(), World::new(0.0), Fight::new(Vec::new())) }

    #[test]
    fn only_a_playable_calling_can_be_chosen() {
        let mut f = Front::Title;
        assert_eq!(f.press(Key::Use), None);
        assert_eq!(f, Front::Callings(0));
        f.press(Key::Down);
        assert_eq!(f.press(Key::Use), None, "the Warden is not yet");
        f.press(Key::Up);
        assert_eq!(f.press(Key::Use), Some(Calling::Wayfarer));
        assert_eq!(Front::Callings(1).panel().options[1].1, false);
    }

    #[test]
    fn the_keeper_speaks_first_then_sells_levels() {
        let (mut s, mut w, mut f) = setup();
        w.souls = 200;
        s.press(Key::Left, &mut w, &mut f);
        assert_eq!(s.station(), Station::Keeper);
        s.press(Key::Use, &mut w, &mut f);
        assert!(w.met_keeper && s.panel(&w, &f).text.starts_with("You are awake"));
        for _ in 0..3 { s.press(Key::Use, &mut w, &mut f); }
        assert_eq!(s.panel(&w, &f).options[0].0, "Grow stronger");
        s.press(Key::Use, &mut w, &mut f);
        s.press(Key::Down, &mut w, &mut f);
        let out = s.press(Key::Use, &mut w, &mut f);
        assert_eq!((out, f.hero.stats.endurance, w.souls), (Some(Outcome::Note(Note::Rose(1))), 1, 140));
        // Too poor for the next: it shows, and nothing is bought.
        w.souls = 10;
        assert!(!s.panel(&w, &f).options[0].1);
        assert_eq!(s.press(Key::Use, &mut w, &mut f), None);
        // Back out, and a second visit goes straight to her choices.
        s.press(Key::Back, &mut w, &mut f);
        s.press(Key::Back, &mut w, &mut f);
        assert!(!s.talking());
        s.press(Key::Use, &mut w, &mut f);
        assert_eq!(s.panel(&w, &f).options.len(), 3);
    }

    #[test]
    fn the_shrine_dresses_the_hero_in_what_the_dead_left() {
        let (mut s, mut w, mut f) = setup();
        let _ = w.felled([crate::fight::Archetype::Shieldbearer]);
        assert!(w.gear.owns(crate::gear::find("Kite Shield")));
        // The shrine is the middle station: Equipment, then the offhand, then the shield.
        assert_eq!(s.station(), Station::Shrine);
        s.press(Key::Use, &mut w, &mut f);
        s.press(Key::Down, &mut w, &mut f);
        s.press(Key::Use, &mut w, &mut f);
        assert!(s.panel(&w, &f).options[1].0.contains("Pilgrim's Buckler"), "{:?}", s.panel(&w, &f).options);
        s.press(Key::Down, &mut w, &mut f);
        s.press(Key::Use, &mut w, &mut f);
        let opts = s.panel(&w, &f).options;
        let k = opts.iter().position(|o| o.0.starts_with("Kite Shield")).expect("the shield is offered");
        for _ in 0..k { s.press(Key::Down, &mut w, &mut f); }
        assert_eq!(s.press(Key::Use, &mut w, &mut f), Some(Outcome::Note(Note::Wore("Kite Shield"))));
        assert!(f.hero.stats.kit.guard < 0.7, "blocks cost less with it");
        assert!(s.panel(&w, &f).options[1].0.contains("Kite Shield"), "back at the list, wearing it");
        // The weapon place cannot be left empty.
        s.press(Key::Up, &mut w, &mut f);
        s.press(Key::Use, &mut w, &mut f);
        assert!(!s.panel(&w, &f).options.iter().any(|o| o.0 == "Nothing"));
    }

    #[test]
    fn the_smiths_side_is_empty_until_he_is_met_and_his_forge_takes_souls_and_shards() {
        let (mut s, mut w, mut f) = setup();
        s.press(Key::Right, &mut w, &mut f);
        assert_eq!(s.station(), Station::Shrine, "no one there yet");
        w.met_smith = true;
        s.press(Key::Right, &mut w, &mut f);
        assert_eq!(s.station(), Station::Smith);
        (w.souls, w.shards) = (500, 3);
        s.press(Key::Use, &mut w, &mut f);
        s.press(Key::Use, &mut w, &mut f);
        let out = s.press(Key::Use, &mut w, &mut f);
        assert_eq!(out, Some(Outcome::Note(Note::Forged("Weapon", 1))));
        assert_eq!((f.hero.stats.weapon, w.souls, w.shards), (1, 420, 2));
        assert!(f.hero.stats.damage() > 1.07);
        // Armour next: +1 costs the same as the weapon's first.
        s.press(Key::Down, &mut w, &mut f);
        assert_eq!(s.press(Key::Use, &mut w, &mut f), Some(Outcome::Note(Note::Forged("Armour", 1))));
        assert!(f.hero.stats.taken() < 1.0);
    }

    #[test]
    fn only_the_shrine_lets_the_hero_rise() {
        let (mut s, mut w, mut f) = setup();
        s.press(Key::Use, &mut w, &mut f);
        s.press(Key::Down, &mut w, &mut f);
        s.press(Key::Down, &mut w, &mut f);
        assert_eq!(s.press(Key::Use, &mut w, &mut f), None, "stay a while");
        assert!(!s.talking());
        s.press(Key::Use, &mut w, &mut f);
        assert_eq!(s.press(Key::Use, &mut w, &mut f), Some(Outcome::Rise));
    }

    #[test]
    fn the_smith_met_on_the_road_speaks_his_pages() {
        let mut m = Meeting::smith();
        assert!(!m.press(Key::Use) && !m.press(Key::Use));
        assert!(m.press(Key::Use));
        assert!(m.panel().text.starts_with("I'll find your fire"));
    }
}
