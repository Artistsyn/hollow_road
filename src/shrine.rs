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

use crate::fight::{Fight, Stats, GEAR_MAX};
use crate::gear::{Arms, Grade, ATTRIBUTES, ITEMS, SLOTS};
use crate::profile::Slot;
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
            Calling::Warden => "Sword and a tall bulwark, the steadiest on their feet: blocks fill a ward, and a block raised at the last moment lets it burst into the foe.",
            Calling::Bloodrunner => "Sword and a bow, no shield: arrows drawn from their own blood (F, ranged attack). Wear a foe down from afar, finish it up close.",
            Calling::Frostweaver => "Sword and a frost focus, a catalyst, no shield: frost bolts that crack a foe's stance and chill it (C, cast), their charges filled by blows landed up close.",
        }
    }
    /// What it sets out with: the weapon and what the other hand holds, and its attributes.
    pub fn outfit(self) -> String {
        let mut gear = crate::gear::Loadout::starting();
        if let Some(name) = self.offhand() { let i = crate::gear::find(name); gear.gain(i); gear.wear(1, Some(i)); }
        let held: Vec<&str> = gear.worn[..2].iter().flatten().map(|&i| ITEMS[i].name).collect();
        let st = crate::fight::Stats::default();
        let [m, d, f] = self.leaning();
        format!("Sets out with {}.   Might {}   Dexterity {}   Focus {}", held.join(" and "), st.might + m, st.dexterity + d, st.focus + f)
    }
    /// Whether it can be played yet (one that cannot is listed so the road ahead is visible):
    /// all of them now.
    pub fn playable(self) -> bool { true }
    /// What it sets out holding in the other hand in place of the buckler.
    pub fn offhand(self) -> Option<&'static str> {
        match self { Calling::Warden => Some("Warden's Bulwark"), Calling::Bloodrunner => Some("Bloodrunner's Bow"), Calling::Frostweaver => Some("Frostweaver's Focus"), _ => None }
    }
    /// The levels it starts with past the Wayfarer's: might, dexterity, focus.
    pub fn leaning(self) -> [u32; 3] {
        match self { Calling::Warden => [3, 0, 0], Calling::Bloodrunner => [0, 3, 0], Calling::Frostweaver => [0, 0, 3], _ => [0, 0, 0] }
    }
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

/// The title, then the pilgrimages kept (`profile::Slot`), then a kept one's choices or, for a
/// free slot, the calling to set out in. Slots are numbered from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Front {
    Title,
    /// The list, the slot chosen.
    Profiles(usize),
    /// A paused game menu with the road still visible behind it.
    Pause(usize),
    /// A kept pilgrimage: carry it on, forget it, or go back (the choice).
    Kept(usize, usize),
    /// Asked whether to forget it for good: keep it (0) or forget it (1).
    Forget(usize, usize),
    /// A free slot: the calling to set out in.
    Callings(usize, usize),
    /// The calling chosen (by index): with the prologue first (0) or without it (1).
    Prologue(usize, usize, usize),
}

/// What the title has decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Begin {
    /// A new pilgrimage in this slot, in this calling, with the prologue first or not.
    New(usize, Calling, bool),
    /// Carry on the one kept in this slot.
    Continue(usize),
    /// Empty this slot for good.
    Forget(usize),
    /// Leave the pause menu and return to the road.
    Resume,
    /// Return to the profile select menu.
    Profile,
    /// Quit the game entirely.
    Quit,
}

const KEPT: [&str; 3] = ["Carry on", "Forget this pilgrimage", "Back"];
const FORGET: [&str; 2] = ["Keep it", "Forget it for good"];
const PAUSE: [&str; 3] = ["Resume", "Change profile", "Quit game"];
const PROLOGUE: [&str; 2] = ["Begin with the prologue", "Skip it: straight to the pilgrimage"];

impl Front {
    /// The list, the newest pilgrimage chosen (else the first free slot).
    pub fn profiles(slots: &[Slot]) -> Front {
        Front::Profiles(crate::profile::newest(slots).or_else(|| slots.iter().position(|s| *s == Slot::Empty)).unwrap_or(0))
    }

    /// A key, with the slots as they stand; what has been decided, once it is.
    pub fn press(&mut self, k: Key, slots: &[Slot]) -> Option<Begin> {
        let last = slots.len().saturating_sub(1);
        match (*self, k) {
            // A first launch, nothing kept: straight to choosing a calling in the first place.
            (Front::Title, Key::Use) if slots.iter().all(|s| *s == Slot::Empty) => *self = Front::Callings(0, 0),
            (Front::Title, Key::Use) => *self = Front::profiles(slots),
            (Front::Profiles(i), Key::Up | Key::Left) => *self = Front::Profiles(i.saturating_sub(1)),
            (Front::Profiles(i), Key::Down | Key::Right) => *self = Front::Profiles((i + 1).min(last)),
            (Front::Profiles(i), Key::Use) => match slots.get(i) {
                Some(Slot::Kept(_)) => *self = Front::Kept(i, 0),
                Some(Slot::Empty) => *self = Front::Callings(i, 0),
                // A file this game cannot read is never written over.
                _ => {}
            },
            (Front::Profiles(_), Key::Back) => *self = Front::Title,
            (Front::Pause(i), Key::Up | Key::Left) => *self = Front::Pause(i.saturating_sub(1)),
            (Front::Pause(i), Key::Down | Key::Right) => *self = Front::Pause((i + 1).min(PAUSE.len() - 1)),
            (Front::Pause(i), Key::Use) => {
                return match i {
                    0 => Some(Begin::Resume),
                    1 => Some(Begin::Profile),
                    _ => Some(Begin::Quit),
                };
            }
            (Front::Pause(_), Key::Back) => return Some(Begin::Resume),
            (Front::Kept(i, c), Key::Up | Key::Left) => *self = Front::Kept(i, c.saturating_sub(1)),
            (Front::Kept(i, c), Key::Down | Key::Right) => *self = Front::Kept(i, (c + 1).min(KEPT.len() - 1)),
            (Front::Kept(i, 0), Key::Use) => return Some(Begin::Continue(i)),
            (Front::Kept(i, 1), Key::Use) => *self = Front::Forget(i, 0),
            (Front::Kept(i, _), Key::Use | Key::Back) => *self = Front::Profiles(i),
            (Front::Forget(i, c), Key::Up | Key::Left) => *self = Front::Forget(i, c.saturating_sub(1)),
            (Front::Forget(i, c), Key::Down | Key::Right) => *self = Front::Forget(i, (c + 1).min(FORGET.len() - 1)),
            (Front::Forget(i, 1), Key::Use) => { *self = Front::Profiles(i); return Some(Begin::Forget(i)); }
            (Front::Forget(i, _), Key::Use | Key::Back) => *self = Front::Kept(i, 1),
            (Front::Callings(s, i), Key::Up | Key::Left) => *self = Front::Callings(s, i.saturating_sub(1)),
            (Front::Callings(s, i), Key::Down | Key::Right) => *self = Front::Callings(s, (i + 1).min(Calling::ALL.len() - 1)),
            (Front::Callings(s, i), Key::Use) if Calling::ALL[i].playable() => *self = Front::Prologue(s, i, 0),
            (Front::Callings(s, _), Key::Back) => *self = Front::Profiles(s),
            (Front::Prologue(s, i, c), Key::Up | Key::Left) => *self = Front::Prologue(s, i, c.saturating_sub(1)),
            (Front::Prologue(s, i, c), Key::Down | Key::Right) => *self = Front::Prologue(s, i, (c + 1).min(PROLOGUE.len() - 1)),
            (Front::Prologue(s, i, c), Key::Use) => return Some(Begin::New(s, Calling::ALL[i], c == 0)),
            (Front::Prologue(s, i, _), Key::Back) => *self = Front::Callings(s, i),
            _ => {}
        }
        None
    }

    pub fn panel(&self, slots: &[Slot]) -> Panel {
        let kept = |i: usize| match slots.get(i) { Some(Slot::Kept(k)) => k.line(), _ => String::new() };
        match *self {
            Front::Title => Panel {
                title: "HOLLOW ROAD".into(),
                text: "The road to Hollowmere.".into(),
                hint: "E or Enter to begin".into(),
                ..Panel::default()
            },
            Front::Profiles(i) => Panel {
                title: "Pilgrimages".into(),
                text: "Each is kept apart: carry one on, or begin another in a free place to walk the road another way.".into(),
                options: slots.iter().enumerate().map(|(n, s)| match s {
                    Slot::Kept(k) => (format!("{}.  {}", n + 1, k.line()), true),
                    Slot::Empty => (format!("{}.  free: begin a new pilgrimage", n + 1), true),
                    Slot::Unreadable => (format!("{}.  kept by another version of the game", n + 1), false),
                }).collect(),
                choice: i,
                hint: "W/S choose   E open   Esc back".into(),
            },
            Front::Pause(i) => Panel {
                title: "Paused".into(),
                text: "Take a breath, switch pilgrimages, or leave Hollowmere for now.".into(),
                options: PAUSE.iter().map(|o| (o.to_string(), true)).collect(),
                choice: i,
                hint: "W/S choose   E confirm   Esc back".into(),
            },
            Front::Kept(i, c) => Panel {
                title: format!("Pilgrimage {}", i + 1),
                text: kept(i),
                options: KEPT.iter().map(|o| (o.to_string(), true)).collect(),
                choice: c,
                hint: "W/S choose   E confirm   Esc back".into(),
            },
            Front::Forget(i, c) => Panel {
                title: format!("Forget pilgrimage {}?", i + 1),
                text: format!("{}.  It cannot be brought back.", kept(i)),
                options: FORGET.iter().map(|o| (o.to_string(), true)).collect(),
                choice: c,
                hint: "W/S choose   E confirm   Esc back".into(),
            },
            Front::Callings(_, i) => Panel {
                title: "Choose a calling".into(),
                text: format!("{}\n{}", Calling::ALL[i].blurb(), Calling::ALL[i].outfit()),
                options: Calling::ALL.iter().map(|c| (if c.playable() { c.name().to_owned() } else { format!("{}  (not yet)", c.name()) }, c.playable())).collect(),
                choice: i,
                hint: "W/S choose   E begin   Esc back".into(),
            },
            Front::Prologue(_, i, c) => Panel {
                title: format!("{}: a prologue first?", Calling::ALL[i].name()),
                text: "A short remembered fight that teaches the road: walking, striking, guarding, rolling and parrying, and a side-on bout. Nothing of the story is lost without it, and Esc leaves it at any time.".into(),
                options: PROLOGUE.iter().map(|o| (o.to_string(), true)).collect(),
                choice: c,
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
    /// The shrines rested at, to rise at instead.
    Travel(usize),
    /// What the smith can forge from the bosses' remnants.
    Craft(usize),
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
    "I keep the shrines on the way to Hollowmere. Bring me the echoes of the fallen and I can make you more than you are.",
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
            (View::Choices(_), Station::Shrine) => vec![("Rise and walk on".into(), true), ("Equipment".into(), true), ("Travel".into(), w.travel_list().len() > 1), ("Stay a while".into(), true)],
            (View::Travel(_), _) => {
                let here = crate::atlas::node(&w.checkpoint);
                w.travel_list().into_iter()
                    .map(|(title, stop)| (if crate::atlas::node(&stop) == here { format!("{title}   (here)") } else { title.to_string() }, true))
                    .chain([("Back".to_string(), true)]).collect()
            }
            (View::Choices(_), Station::Smith) => vec![("Forge".into(), true), ("Craft from a remnant".into(), !w.craftable().is_empty()), ("Talk".into(), true), ("Step away".into(), true)],
            (View::Craft(_), _) => w.craftable().into_iter()
                .map(|(r, can)| (format!("{}   {}\n      from the {}   {} {}, {} shards", r.item, ITEMS[crate::gear::find(r.item)].text, r.remnant, r.echoes, crate::world::CURRENCY, r.shards), can))
                .chain([("Back".to_string(), true)]).collect(),
            (View::Levels(_), _) => {
                let can = w.souls >= st.level_cost();
                // Each line says what one more level would change, as the hero is now equipped.
                let up = |f: fn(&mut Stats)| { let mut n = st; f(&mut n); n };
                let strikes = |name: &str, lvl: u32, n: Stats| (format!("{name} {lvl}\tstrikes x{:.2} -> x{:.2}", st.damage(), n.damage()), can);
                vec![
                    (format!("Vigor {}\thealth {:.0} -> {:.0}", st.vigor, st.max_hp(), up(|s| s.vigor += 1).max_hp()), can),
                    (format!("Endurance {}\tstamina {:.0} -> {:.0}", st.endurance, st.max_stamina(), up(|s| s.endurance += 1).max_stamina()), can),
                    strikes("Might", st.might, up(|s| s.might += 1)),
                    strikes("Dexterity", st.dexterity, up(|s| s.dexterity += 1)),
                    strikes("Focus", st.focus, up(|s| s.focus += 1)),
                    ("Back".into(), true),
                ]
            }
            (View::Forge(_), _) => {
                let line = |what: &str, lvl: u32| match World::forge_cost(lvl) {
                    Some((souls, shards)) => (format!("{what} +{lvl} to +{}   {souls} {}, {shards} shards", lvl + 1, crate::world::CURRENCY), w.souls >= souls && w.shards >= shards),
                    None => (format!("{what} +{GEAR_MAX}   as good as it gets"), false),
                };
                vec![line("Weapon", st.weapon), line("Armour", st.armour), ("Back".into(), true)]
            }
            (View::Gear(_), _) => SLOTS.iter().zip(&w.gear.worn)
                .map(|((_, place), worn)| (format!("{place}\t{}", worn.map_or("nothing", |i| ITEMS[i].name)), true))
                .chain([("Back".to_string(), true)]).collect(),
            (View::Wear(p, _), _) => Self::wearable(w, *p).into_iter()
                .map(|i| (match i {
                    Some(i) => match (ITEMS[i].arms, held_line(&ITEMS[i], &st)) {
                        // The name and what it does, and under it what it is.
                        (Some(a), _) => format!("{}   {}\n      {}", ITEMS[i].name, ITEMS[i].text, arms_line(&a, &st)),
                        (None, Some(line)) => format!("{}   {}\n      {}", ITEMS[i].name, ITEMS[i].text, line),
                        (None, None) => format!("{}   {}", ITEMS[i].name, ITEMS[i].text),
                    },
                    None => "Nothing".into(),
                }, true))
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
            (View::Choices(i) | View::Levels(i) | View::Forge(i) | View::Gear(i) | View::Wear(_, i) | View::Travel(i) | View::Craft(i), Key::Up) => self.set_choice(i.saturating_sub(1)),
            (View::Choices(i) | View::Levels(i) | View::Forge(i) | View::Gear(i) | View::Wear(_, i) | View::Travel(i) | View::Craft(i), Key::Down) => self.set_choice((i + 1).min(n.saturating_sub(1))),
            (View::Choices(_), Key::Back) => self.view = View::Walk,
            (View::Levels(_) | View::Forge(_), Key::Back) => self.view = View::Choices(0),
            (View::Gear(_), Key::Back) => self.view = View::Choices(1),
            (View::Travel(_), Key::Back) => self.view = View::Choices(2),
            (View::Craft(_), Key::Back) => self.view = View::Choices(1),
            (View::Craft(i), Key::Use) => match w.craftable().get(i).map(|(r, _)| *r) {
                Some(r) => { let out = w.craft(r); Self::refit(w, f); if w.craftable().is_empty() { self.view = View::Choices(1); } return out.map(Outcome::Note); }
                None => self.view = View::Choices(1),
            },
            // Rise at the shrine chosen.
            (View::Travel(i), Key::Use) => match w.travel_list().get(i) {
                Some((_, stop)) => { let stop = stop.clone(); w.travel(&stop); self.view = View::Walk; return Some(Outcome::Rise); }
                None => self.view = View::Choices(2),
            },
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
            (View::Levels(i), Key::Use) if i < 5 => return w.rise(f, i).map(Outcome::Note),
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
            View::Gear(_) => View::Gear(i), View::Wear(p, _) => View::Wear(p, i), View::Travel(_) => View::Travel(i), View::Craft(_) => View::Craft(i), ref v => v.clone(),
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
            (Station::Shrine, 2) if w.travel_list().len() > 1 => self.view = View::Travel(0),
            (Station::Shrine, 2) => {}
            (Station::Smith, 0) => self.view = View::Forge(0),
            (Station::Smith, 1) if !w.craftable().is_empty() => self.view = View::Craft(0),
            (Station::Smith, 1) => {}
            (Station::Smith, 2) => { let k = w.talks[1]; w.talks[1] += 1; self.view = words(&SMITH_TALK, k); }
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
                // What the attribute picked does, so a level is bought knowing.
                const DOES: [&str; 5] = [
                    "Vigor is health: more of it, more blows survived.",
                    "Endurance is stamina: every strike, roll, run and blocked blow spends it.",
                    "Might feeds heavy weapons (axes, greatswords): they need it, and hit harder for it.",
                    "Dexterity feeds quick weapons and bows: swords, spears and arrows grow with it.",
                    "Focus feeds catalysts and spells: a spell needs it to be cast, and hits harder and holds more charges with it.",
                ];
                p.text = format!("{} {}.   Level {}: the next costs {}.{}", crate::world::CURRENCY_TITLE, w.souls, f.hero.stats.level(), f.hero.stats.level_cost(),
                    DOES.get(*i).map_or(String::new(), |d| format!("\n{d}")));
                p.choice = *i;
                p.hint = "W/S choose   E buy   Esc back".into();
            }
            View::Forge(i) => {
                p.title = "The forge".into();
                p.text = format!("{} {}.   Shards {}.", crate::world::CURRENCY_TITLE, w.souls, w.shards);
                p.choice = *i;
                p.hint = "W/S choose   E forge   Esc back".into();
            }
            View::Craft(i) => {
                p.title = "Forge from a remnant".into();
                p.text = format!("{} {}.   Shards {}.   \"Bring me what's left of a thing that should not have lived, and I'll make you a weapon worth the name.\"", crate::world::CURRENCY_TITLE, w.souls, w.shards);
                p.choice = *i;
                p.hint = "W/S choose   E forge   Esc back".into();
            }
            View::Travel(i) => {
                p.title = "Travel".into();
                p.text = "The fire remembers every shrine you have rested at. Rise at another.".into();
                p.choice = *i;
                p.hint = "W/S choose   E rise there   Esc back".into();
            }
            View::Gear(i) | View::Wear(_, i) => {
                let st = &f.hero.stats;
                p.title = match &self.view { View::Wear(q, _) => SLOTS[*q].1.into(), _ => "Equipment".into() };
                let weak = st.lacking().iter().map(|&a| format!("{} {}", ATTRIBUTES[a], st.kit.arms.needs[a])).collect::<Vec<_>>();
                p.text = format!(
                    "Carrying {:.1} of {:.0}{}.   Health {:.0}  stamina {:.0}  poise {:.0}  strikes x{:.2}  blows x{:.2}{}",
                    st.kit.load, st.capacity(), if st.burdened() { ", burdened: rolls cost more" } else { "" },
                    st.max_hp(), st.max_stamina(), st.poise(), st.damage(), st.taken(),
                    if weak.is_empty() { String::new() } else { format!("\nToo weak for this weapon: it wants {}.", weak.join(", ")) },
                );
                p.choice = *i;
                p.hint = "W/S choose   E wear   Esc back".into();
            }
        }
        p
    }
}

impl Default for Sanctum { fn default() -> Self { Sanctum::new() } }

/// A weapon as the list shows it: its kind, what it needs (a need the hero lacks marked with !), its grade
/// in each attribute, and what it would strike for in the hero's hands.
pub fn arms_line(a: &Arms, st: &Stats) -> String {
    const SHORT: [&str; 3] = ["Mt", "Dx", "Fc"];
    let at = st.attributes();
    let needs: Vec<String> = (0..3).filter(|&i| a.needs[i] > 0).map(|i| format!("{} {}{}", SHORT[i], a.needs[i], if at[i] < a.needs[i] { "!" } else { "" })).collect();
    let grades: Vec<String> = (0..3).filter(|&i| a.scales[i] != Grade::No).map(|i| format!("{} {}", SHORT[i], a.scales[i].letter())).collect();
    let held = Stats { kit: crate::gear::Kit { arms: *a, ..st.kit }, ..*st };
    let kind = if a.class.two_handed() { format!("{}, both hands", a.class.name()) } else { a.class.name().to_string() };
    format!("{kind}  needs {}  scales {}  x{:.2}", if needs.is_empty() { "nothing".into() } else { needs.join(" ") }, grades.join(" "), held.damage())
}

/// A bow, a crossbow or a catalyst as the list shows it, as a weapon is: what it needs (a need
/// lacked marked !), its grades, and what it would strike for in the hero's hands; a spell, what
/// it takes to cast.
pub fn held_line(item: &crate::gear::Item, st: &Stats) -> Option<String> {
    const SHORT: [&str; 3] = ["Mt", "Dx", "Fc"];
    let at = st.attributes();
    let needs = |n: [u32; 3]| -> String {
        let v: Vec<String> = (0..3).filter(|&i| n[i] > 0).map(|i| format!("{} {}{}", SHORT[i], n[i], if at[i] < n[i] { "!" } else { "" })).collect();
        if v.is_empty() { "nothing".into() } else { v.join(" ") }
    };
    if let Some(sp) = item.spell { return Some(format!("spell  needs {}", needs(sp.needs()))); }
    let (kind, sc) = match (item.shooting, item.catalyst) {
        (Some(sc), _) => ("bow", sc),
        (None, Some(c)) => ("catalyst", c.scaling),
        _ => return None,
    };
    let kind = if item.ranged == Some(crate::gear::Ranged::BloodCrossbow) { "crossbow" } else { kind };
    let grades: Vec<String> = (0..3).filter(|&i| sc.scales[i] != Grade::No).map(|i| format!("{} {}", SHORT[i], sc.scales[i].letter())).collect();
    let k = st.graded(sc.base, sc.needs, sc.scales);
    Some(format!("{kind}  needs {}  scales {}  x{k:.2}", needs(sc.needs), if grades.is_empty() { "-".into() } else { grades.join(" ") }))
}

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
    fn every_calling_can_be_chosen_and_sets_out_with_its_own() {
        let free = vec![Slot::Empty; crate::profile::SLOTS];
        for (k, c) in Calling::ALL.into_iter().enumerate() {
            // Nothing kept: the title goes straight to the callings, in the first place.
            let mut f = Front::Title;
            assert_eq!(f.press(Key::Use, &free), None);
            assert_eq!(f, Front::Callings(0, 0));
            for _ in 0..k { f.press(Key::Down, &free); }
            assert!(Front::Callings(0, k).panel(&free).options[k].1, "{} is listed as playable", c.name());
            assert_eq!(f.press(Key::Use, &free), None);
            assert_eq!(f, Front::Prologue(0, k, 0), "asked whether to play the prologue");
            let mut skip = f;
            skip.press(Key::Down, &free);
            assert_eq!(skip.press(Key::Use, &free), Some(Begin::New(0, c, false)));
            assert_eq!(f.press(Key::Use, &free), Some(Begin::New(0, c, true)));
            // What it holds in the other hand is a real piece.
            if let Some(name) = c.offhand() { let _ = crate::gear::find(name); }
        }
        assert_eq!(Calling::Warden.offhand(), Some("Warden's Bulwark"));
    }

    #[test]
    fn the_title_offers_the_newest_pilgrimage_and_forgetting_one_asks_twice() {
        use crate::profile::Summary;
        let kept = |saved: u64| Slot::Kept(Summary { calling: Calling::Warden, level: 3, place: "The Road to Hollowmere", played: 60, pilgrimages: 0, saved });
        let slots = vec![kept(10), Slot::Empty, kept(30), Slot::Unreadable, Slot::Empty];
        let mut f = Front::Title;
        f.press(Key::Use, &slots);
        assert_eq!(f, Front::Profiles(2), "the newest first");
        assert_eq!({ let mut g = f; g.press(Key::Use, &slots) }, None, "opening it decides nothing yet");
        f.press(Key::Use, &slots);
        assert_eq!(f, Front::Kept(2, 0));
        assert_eq!({ let mut g = f; g.press(Key::Use, &slots) }, Some(Begin::Continue(2)));
        // Forget: asked again, and "keep it" first, so a double press forgets nothing.
        f.press(Key::Down, &slots);
        f.press(Key::Use, &slots);
        assert_eq!(f, Front::Forget(2, 0));
        assert_eq!({ let mut g = f; g.press(Key::Use, &slots) }, None);
        f.press(Key::Down, &slots);
        assert_eq!(f.press(Key::Use, &slots), Some(Begin::Forget(2)));
        assert_eq!(f, Front::Profiles(2));
        // A free slot begins a new one; an unreadable one cannot be opened at all.
        let mut g = Front::Profiles(1);
        g.press(Key::Use, &slots);
        assert_eq!(g, Front::Callings(1, 0));
        let mut u = Front::Profiles(3);
        u.press(Key::Use, &slots);
        assert_eq!(u, Front::Profiles(3));
        assert!(!Front::Profiles(3).panel(&slots).options[3].1);
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
        for _ in 0..3 { s.press(Key::Down, &mut w, &mut f); }
        assert_eq!(s.press(Key::Use, &mut w, &mut f), None, "stay a while");
        assert!(!s.talking());
        s.press(Key::Use, &mut w, &mut f);
        assert_eq!(s.press(Key::Use, &mut w, &mut f), Some(Outcome::Rise));
    }

    #[test]
    fn the_fire_lets_the_hero_rise_at_any_shrine_rested_at() {
        let (mut s, mut w, mut f) = setup();
        w.rested = vec![crate::world::START.into()];
        s.press(Key::Use, &mut w, &mut f);
        for _ in 0..2 { s.press(Key::Down, &mut w, &mut f); }
        assert_eq!(s.press(Key::Use, &mut w, &mut f), None, "one shrine: nowhere to travel");
        w.rested.push("ridge_camp@pine_road".into());
        s.press(Key::Use, &mut w, &mut f);
        assert_eq!(s.panel(&w, &f).options.len(), 3);
        s.press(Key::Down, &mut w, &mut f);
        assert_eq!(s.press(Key::Use, &mut w, &mut f), Some(Outcome::Rise));
        assert_eq!(w.checkpoint, "ridge_camp@pine_road");
        assert_eq!(w.leave_rest(), crate::world::Go::Warp("ridge_camp@pine_road".into()));
    }

    #[test]
    fn the_smith_forges_a_bosss_remnant_into_its_weapon_for_a_large_price() {
        let (mut s, mut w, mut f) = setup();
        w.met_smith = true;
        s.lane = 1;
        s.press(Key::Use, &mut w, &mut f);
        s.press(Key::Down, &mut w, &mut f);
        assert_eq!(s.press(Key::Use, &mut w, &mut f), None, "nothing to forge from yet");
        w.remnants.push("Bell Warden's Clapper");
        (w.souls, w.shards) = (2000, 10);
        s.press(Key::Use, &mut w, &mut f);
        let opts = s.panel(&w, &f).options;
        assert!(opts[0].0.starts_with("Bell-Hammer") && !opts[0].1, "too dear yet: {opts:?}");
        assert_eq!(s.press(Key::Use, &mut w, &mut f), None);
        w.souls = 2600;
        assert_eq!(s.press(Key::Use, &mut w, &mut f), Some(Outcome::Note(Note::Crafted("Bell-Hammer"))));
        assert!(w.gear.owns(crate::gear::find("Bell-Hammer")) && w.remnants.is_empty());
        assert_eq!((w.souls, w.shards), (100, 4));
    }

    #[test]
    fn the_smith_met_on_the_road_speaks_his_pages() {
        let mut m = Meeting::smith();
        assert!(!m.press(Key::Use) && !m.press(Key::Use));
        assert!(m.press(Key::Use));
        assert!(m.panel().text.starts_with("I'll find your fire"));
    }
}

#[cfg(test)]
mod held_line_tests {
    use super::*;

    /// The shrine shows a bow, a crossbow, a catalyst and a spell as it shows a weapon: needs
    /// (lacked marked !), grades, and what it would strike for now.
    #[test]
    fn the_list_shows_what_ranged_pieces_and_spells_need_and_how_they_scale() {
        let line = |name: &str, focus: u32| held_line(&ITEMS[crate::gear::find(name)], &Stats { focus, ..Stats::default() }).unwrap();
        for (name, focus) in [("Hunter's Shortbow", 0), ("Bloodrunner's Bow", 0), ("Blood Crossbow", 0), ("Frostweaver's Focus", 3), ("Frost Bolt", 0), ("Frost Bolt", 3)] {
            eprintln!("[held] {name} (focus {focus}): {}", line(name, focus));
        }
        assert_eq!(line("Hunter's Shortbow", 0), "bow  needs Dx 2!  scales Mt E Dx B  x0.60");
        assert_eq!(line("Blood Crossbow", 0), "crossbow  needs Mt 4! Dx 2!  scales -  x0.60");
        assert_eq!(line("Frost Bolt", 0), "spell  needs Fc 3!");
        assert_eq!(line("Frost Bolt", 3), "spell  needs Fc 3");
        assert!(line("Frostweaver's Focus", 3).starts_with("catalyst  needs Fc 2  scales Fc B  x1.18"));
    }
}
