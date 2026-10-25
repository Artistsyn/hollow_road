//! What the hero wears and carries: a weapon, an offhand, four pieces of armour, two rings, a
//! talisman and a spell attuned. Each piece changes the fight through a [`Kit`] (the sum of what is worn), and its
//! weight counts against what the hero can carry: past 70% of it, rolls cost more and stamina
//! comes back slower. Pieces come off the dead (`World::felled`); the smith's +levels apply to
//! whatever weapon and armour are worn.

use crate::arts::WeaponArt;
use crate::fight::Archetype;

/// Where a piece goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot { Weapon, Offhand, Head, Body, Hands, Legs, Ring, Talisman, Spell }

/// The places to wear things, in the order the shrine lists them (two for rings).
/// The spell's is last, so a pilgrimage kept before there were spells reads as it was.
pub const SLOTS: [(Slot, &str); 10] = [
    (Slot::Weapon, "Weapon"), (Slot::Offhand, "Offhand"), (Slot::Head, "Head"), (Slot::Body, "Body"),
    (Slot::Hands, "Hands"), (Slot::Legs, "Legs"), (Slot::Ring, "Ring"), (Slot::Ring, "Ring"), (Slot::Talisman, "Talisman"),
    (Slot::Spell, "Spell"),
];

/// What everything worn does, together. Multipliers start at 1, additions at 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kit {
    /// Strike damage, and damage taken (before the smith's levels).
    pub damage: f32,
    pub taken: f32,
    /// Extra poise: more hits before a stagger.
    pub poise: f32,
    /// Stamina a blocked blow costs, and what a guard-breaking blow gets through a guard.
    pub guard: f32,
    pub chip: f32,
    /// Ticks the parry stays live past its own.
    pub parry: u32,
    /// Stamina a roll or backstep costs, and a strike.
    pub roll: f32,
    pub strike: f32,
    /// How fast stamina comes back.
    pub regen: f32,
    /// What a flask heals.
    pub heal: f32,
    /// Extra health and stamina.
    pub hp: f32,
    pub stamina: f32,
    /// Souls from the dead.
    pub souls: f32,
    /// Weight worn, against `Stats::capacity`.
    pub load: f32,
    /// The weapon held: what it needs and how it scales (`Stats::damage`).
    pub arms: Arms,
    /// What the other hand holds: None when it is empty or on the weapon's grip.
    pub off: Option<Off>,
    /// A bow or a crossbow in the other hand instead of a shield (F: `fight::Move::Loose`), and
    /// what it needs and how it scales.
    pub ranged: Option<Ranged>,
    pub shooting: Scaling,
    /// A catalyst in the other hand instead of a shield, without which no spell is cast; and the
    /// spell attuned (C: `fight::Move::Cast`). See `Kit::spell_cast`.
    pub catalyst: Option<Catalyst>,
    pub spell: Option<Spell>,
    /// Of the damage a strike deals, what comes back as health.
    pub leech: f32,
    /// Strike damage against a foe that is open, stumbling or staggered.
    pub opener: f32,
    /// A bulwark's ward (`fight::Bulwark`): how fast blocks fill it; 0, a shield with none.
    pub ward: f32,
    /// The weapon's art (Q: `fight::Move::Art`), a smith-forged weapon's own.
    pub art: Option<crate::arts::WeaponArt>,
}

/// How much a level of an attribute raises a weapon's damage, by the weapon's letter for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade { S, A, B, C, D, E, No }

impl Grade {
    /// Its share of `Stats::soft`: B takes it whole.
    pub fn share(self) -> f32 {
        match self { Grade::S => 1.4, Grade::A => 1.2, Grade::B => 1.0, Grade::C => 0.75, Grade::D => 0.5, Grade::E => 0.25, Grade::No => 0.0 }
    }
    pub fn letter(self) -> &'static str {
        match self { Grade::S => "S", Grade::A => "A", Grade::B => "B", Grade::C => "C", Grade::D => "D", Grade::E => "E", Grade::No => "-" }
    }
}

/// The attributes a weapon names, in the order of `needs` and `scales`.
pub const ATTRIBUTES: [&str; 3] = ["Might", "Dexterity", "Focus"];

/// How a weapon is swung: the light and heavy strikes it gives (`fight::Move::spec_for`), its
/// reach, and whether it takes both hands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Balanced cuts.
    Sword,
    /// Quick thrusts that reach far but break little stance (rapiers).
    Thrust,
    /// The longest reach, a step behind a sword for speed.
    Spear,
    /// Slow, short, heavy chops that break stance.
    Axe,
    /// Both hands: slow and long, its heavy sweeping the lanes beside; no shield, a deflect for a
    /// parry, and blocks with the blade cost more.
    Greatsword,
    /// An ultra greatsword, far too big for its wielder: both hands like a greatsword, but every
    /// swing is a long commitment, sweeps the lanes beside, and rides out a flinch.
    Ultra,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self { Class::Sword => "sword", Class::Thrust => "thrusting sword", Class::Spear => "spear", Class::Axe => "axe", Class::Greatsword => "greatsword", Class::Ultra => "ultra greatsword" }
    }
    /// Whether it takes both hands, leaving none for an offhand: its clips hold it in both
    /// (`family` "two").
    pub fn two_handed(self) -> bool { self.family() == "two" }
    /// Its attack clips' prefix (`<slug>_light`, `e_<slug>_heavy`, ...).
    pub fn slug(self) -> &'static str {
        match self { Class::Sword => "sword", Class::Thrust => "thrust", Class::Spear => "spear", Class::Axe => "axe", Class::Greatsword => "greatsword", Class::Ultra => "ultra" }
    }
    /// How it is held when not striking, which names the bare-handed clips it is drawn into
    /// (walking, guarding, reeling): "one" in the right hand, "two" in both.
    pub fn family(self) -> &'static str {
        match self { Class::Spear | Class::Greatsword | Class::Ultra => "two", _ => "one" }
    }
}

/// A weapon's side of a strike: its damage at no attribute, the levels of might, dexterity and
/// focus it needs (under any of them it is wielded weakly: `Stats::weak`), its grade in each, and
/// how it is swung; and its picture (`assets/pixellab/weapons/<art>.png`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arms { pub base: f32, pub needs: [u32; 3], pub scales: [Grade; 3], pub class: Class, pub art: &'static str }

/// The Wayfarer's sword: balanced, needing nothing. What a kit holds when no weapon is named.
pub const SWORD: Arms = Arms { base: 1.0, needs: [0, 0, 0], scales: [Grade::C, Grade::C, Grade::No], class: Class::Sword, art: "wayfarer_sword" };

/// The kind of thing held in the other hand, which decides how the hero parries and blocks
/// (`fight::Move::spec_for`, `Loadout::kit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guard {
    /// A small round shield gripped in the fist: the quick flick of a parry.
    Buckler,
    /// A small shield on the forearm: the same flick.
    Parma,
    /// A tall shield strapped to the forearm: blocks well, turns a parry slower.
    Kite,
}

/// An offhand piece: how it guards, and its picture (`assets/pixellab/offhand/<art>_<view>.png`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Off { pub guard: Guard, pub art: &'static str }

/// What flies down the lane from the hero: from a bow or a crossbow held in the other hand (F,
/// `fight::Move::Loose`), or a spell's (C, `fight::Move::Cast`). Each is paid for differently,
/// and none of them does what melee does: see `fight::Move::loose_spec`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ranged {
    /// Arrows from a quiver: refilled at the shrine, and a few taken back from each foe felled.
    Bow,
    /// The Bloodrunner's: every arrow is drawn from their own health.
    BloodBow,
    /// A hand crossbow whose bolts are drawn from health too: quick to aim, slow to wind again,
    /// each bolt heavier.
    BloodCrossbow,
    /// A frost bolt (`Spell::FrostBolt`), one charge each: charges come back at the shrine and from
    /// blows landed up close.
    Frost,
}

impl Ranged {
    /// How fast what it looses flies, metres a second.
    pub fn speed(self) -> f32 { match self { Ranged::Bow | Ranged::BloodBow => 24.0, Ranged::BloodCrossbow => 30.0, Ranged::Frost => 16.0 } }
    /// Its picture in the other hand (`assets/pixellab/offhand/<art>.png`); a spell's, the
    /// catalyst's.
    pub fn art(self) -> &'static str { match self { Ranged::Bow => "hunters_bow", Ranged::BloodBow => "blood_bow", Ranged::BloodCrossbow => "blood_crossbow", Ranged::Frost => "frost_focus" } }
    /// What each costs in health, for those drawn from blood.
    pub fn blood(self) -> f32 { match self { Ranged::BloodBow => crate::fight::BLOOD_COST, Ranged::BloodCrossbow => crate::fight::BLOOD_BOLT_COST, _ => 0.0 } }
}

/// What a bow, a crossbow or a catalyst needs and how it scales, as a weapon's `Arms` do, in the
/// souls games' way: its strength at no attribute (an arrow's damage, or for a catalyst every
/// spell's, times this), the levels of might, dexterity and focus it needs (short of any, it is
/// held weakly: `fight::WEAK` of itself), and its grade in each (`fight::Stats::graded`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scaling { pub base: f32, pub needs: [u32; 3], pub scales: [Grade; 3] }

/// Needing nothing and raised by nothing: a crossbow's, whose bolt is its own weight whoever
/// holds it.
pub const UNSCALED: Scaling = Scaling { base: 1.0, needs: [0; 3], scales: [Grade::No; 3] };

/// A catalyst, held in the other hand: what a spell is cast through. What it needs and how it
/// scales (which sets how hard every spell cast through it strikes), frost charges past the
/// usual, its own spell (cast when none is attuned), its picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Catalyst { pub scaling: Scaling, pub charges: f32, pub own: Option<Spell>, pub art: &'static str }

/// A spell: found and attuned (`Slot::Spell`), or a catalyst's own; cast with C through a
/// catalyst, a charge each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spell {
    /// A frost bolt down the lane that cracks a stance and chills (`fight::CHILL`).
    FrostBolt,
}

impl Spell {
    /// What it sends down the lane (its damage there: `fight::Move::loose_spec`).
    pub fn shot(self) -> Ranged { match self { Spell::FrostBolt => Ranged::Frost } }
    pub fn name(self) -> &'static str { match self { Spell::FrostBolt => "Frost Bolt" } }
    /// The levels of might, dexterity and focus it takes to cast at all (short of them it cannot
    /// be cast, whatever catalyst is held).
    pub fn needs(self) -> [u32; 3] { match self { Spell::FrostBolt => [0, 0, 3] } }
}

impl Kit {
    /// The spell C casts: the one attuned, else the catalyst's own; none without a catalyst.
    pub fn spell_cast(&self) -> Option<Spell> { self.catalyst.and_then(|c| self.spell.or(c.own)) }
    /// What a shot of the hand's (F) or a cast (C) sends, if anything.
    pub fn shot(&self, cast: bool) -> Option<Ranged> { if cast { self.spell_cast().map(Spell::shot) } else { self.ranged } }
}

/// The Pilgrim's Buckler: what a kit holds beside the sword when nothing is named.
pub const BUCKLER: Off = Off { guard: Guard::Buckler, art: "pilgrim_buckler" };

impl Default for Kit {
    fn default() -> Kit {
        Kit { damage: 1.0, taken: 1.0, poise: 0.0, guard: 1.0, chip: 1.0, parry: 0, roll: 1.0, strike: 1.0, regen: 1.0, heal: 1.0, hp: 0.0, stamina: 0.0, souls: 1.0, load: 0.0, arms: SWORD, off: Some(BUCKLER), leech: 0.0, opener: 1.0, ranged: None, shooting: UNSCALED, catalyst: None, spell: None, ward: 0.0, art: None }
    }
}

/// A piece: its name, where it goes, its weight, a line saying what it does, and what it does;
/// a weapon also its `Arms`.
pub struct Item {
    pub name: &'static str,
    pub slot: Slot,
    pub weight: f32,
    pub text: &'static str,
    pub apply: fn(&mut Kit),
    pub arms: Option<Arms>,
    pub off: Option<Off>,
    pub ranged: Option<Ranged>,
    /// A bow's or a crossbow's needs and grades.
    pub shooting: Option<Scaling>,
    pub catalyst: Option<Catalyst>,
    pub spell: Option<Spell>,
}

const fn item(name: &'static str, slot: Slot, weight: f32, text: &'static str, apply: fn(&mut Kit)) -> Item {
    Item { name, slot, weight, text, apply, arms: None, off: None, ranged: None, shooting: None, catalyst: None, spell: None }
}

const fn weapon(name: &'static str, weight: f32, arms: Arms, text: &'static str, apply: fn(&mut Kit)) -> Item {
    Item { arms: Some(arms), ..item(name, Slot::Weapon, weight, text, apply) }
}

/// A bow or a crossbow: in the other hand, where a shield would be.
const fn ranged(name: &'static str, weight: f32, r: Ranged, sc: Scaling, text: &'static str) -> Item {
    Item { ranged: Some(r), shooting: Some(sc), ..item(name, Slot::Offhand, weight, text, |_| {}) }
}

/// A catalyst: in the other hand, where a shield would be.
const fn catalyst(name: &'static str, weight: f32, c: Catalyst, text: &'static str) -> Item {
    Item { catalyst: Some(c), ..item(name, Slot::Offhand, weight, text, |_| {}) }
}

/// A spell, attuned in its own place; it weighs nothing.
const fn spell(name: &'static str, sp: Spell, text: &'static str) -> Item {
    Item { spell: Some(sp), ..item(name, Slot::Spell, 0.0, text, |_| {}) }
}

const fn shield(name: &'static str, weight: f32, off: Off, text: &'static str, apply: fn(&mut Kit)) -> Item {
    Item { off: Some(off), ..item(name, Slot::Offhand, weight, text, apply) }
}

use Grade::{A, B, C, D, E, No, S};

/// Every piece there is. Indices are what a [`Loadout`] holds, so add to the end.
pub const ITEMS: &[Item] = &[
    // What the hero sets out with.
    weapon("Wayfarer's Sword", 3.0, SWORD, "balanced", |_| {}),
    shield("Pilgrim's Buckler", 1.5, BUCKLER, "quick parries", |_| {}),
    item("Wayfarer's Hood", Slot::Head, 0.5, "light", |_| {}),
    item("Wayfarer's Cloak", Slot::Body, 2.0, "light", |_| {}),
    item("Leather Wraps", Slot::Hands, 0.5, "light", |_| {}),
    item("Road Boots", Slot::Legs, 1.0, "light", |_| {}),
    // Off the dead.
    weapon("Husk Cleaver", 6.0, Arms { base: 1.0, needs: [4, 0, 0], scales: [A, No, No], class: Class::Axe, art: "husk_cleaver" }, "chops that break a guard", |_| {}),
    weapon("Lancer's Shortspear", 4.0, Arms { base: 1.0, needs: [1, 3, 0], scales: [D, B, No], class: Class::Spear, art: "lancer_spear" }, "reaches past their blades", |_| {}),
    shield("Kite Shield", 6.0, Off { guard: Guard::Kite, art: "kite_shield" }, "blocks cost 35% less, half the chip; slower parries", |k| { k.guard *= 0.65; k.chip *= 0.5; }),
    shield("Warden's Bulwark", 7.0, Off { guard: Guard::Kite, art: "warden_bulwark" }, "a ward: blocks fill it, a block raised at the last moment lets it go; blocks cost 30% less, +20 poise", |k| { k.ward = 1.0; k.guard *= 0.7; k.chip *= 0.5; k.poise += 20.0; }),
    shield("Duelist's Parma", 1.0, Off { guard: Guard::Parma, art: "duelist_parma" }, "parry live 4 ticks longer", |k| k.parry += 4),
    item("Iron Helm", Slot::Head, 3.0, "-5% damage taken, +5 poise", |k| { k.taken *= 0.95; k.poise += 5.0; }),
    item("Chain Hauberk", Slot::Body, 9.0, "-12% damage taken, +12 poise", |k| { k.taken *= 0.88; k.poise += 12.0; }),
    item("Shieldbearer's Gauntlets", Slot::Hands, 2.5, "blocks cost 12% less, -3% damage taken", |k| { k.guard *= 0.88; k.taken *= 0.97; }),
    item("Lancer's Greaves", Slot::Legs, 3.0, "+6 poise, -3% damage taken", |k| { k.poise += 6.0; k.taken *= 0.97; }),
    item("Ember Ring", Slot::Ring, 0.0, "+10% damage", |k| k.damage *= 1.10),
    item("Ring of the Quiet Step", Slot::Ring, 0.0, "rolls cost 25% less", |k| k.roll *= 0.75),
    item("Vigil Ring", Slot::Ring, 0.0, "stamina back 20% faster", |k| k.regen *= 1.2),
    item("Hoarder's Ring", Slot::Ring, 0.0, "+20% echoes", |k| k.souls *= 1.2),
    item("Pilgrim's Token", Slot::Talisman, 0.0, "flasks heal 30% more", |k| k.heal *= 1.3),
    item("Keeper's Sigil", Slot::Talisman, 0.0, "+15 health, +10 stamina", |k| { k.hp += 15.0; k.stamina += 10.0; }),
    // The Bell Warden's.
    item("Warden's Bell", Slot::Talisman, 1.0, "+10 poise, +8% damage", |k| { k.poise += 10.0; k.damage *= 1.08; }),
    // Weapons for the attributes past might.
    weapon("Duelist's Rapier", 2.5, Arms { base: 1.0, needs: [0, 6, 0], scales: [No, A, No], class: Class::Thrust, art: "duelist_rapier" }, "quick thrusts", |_| {}),
    weapon("Rimed Sabre", 3.5, Arms { base: 1.0, needs: [0, 2, 5], scales: [No, D, B], class: Class::Sword, art: "rimed_sabre" }, "cold-forged, runs on focus", |_| {}),
    weapon("Executioner's Greatsword", 9.0, Arms { base: 1.0, needs: [8, 2, 0], scales: [B, D, No], class: Class::Greatsword, art: "executioner_greatsword" }, "both hands; its heavy sweeps the lanes beside", |_| {}),
    // Forged by the smith from what the region bosses leave (`RECIPES`), each with a way of its
    // own and an art (Q, `arts::WeaponArt`) echoing the boss it came from.
    weapon("Bell-Hammer", 10.0, Arms { base: 1.15, needs: [10, 0, 0], scales: [A, No, No], class: Class::Greatsword, art: "bell_hammer" }, "rings the stunned: +40% to the open, +12 poise. Art (Q), Toll: a slam that rings through all near, throwing them off balance", |k| { k.opener *= 1.4; k.poise += 12.0; k.art = Some(WeaponArt::Toll); }),
    weapon("Drowned Wail", 3.5, Arms { base: 1.1, needs: [0, 3, 8], scales: [No, D, A], class: Class::Sword, art: "drowned_wail" }, "drinks: 6% of each strike back as health. Art (Q), Wail: a scream that breaks every windup in reach, drinks from all it catches and drops arrows", |k| { k.leech += 0.06; k.art = Some(WeaponArt::Wail); }),
    weapon("Chieftain's Cleaver", 7.0, Arms { base: 1.15, needs: [8, 3, 0], scales: [A, D, No], class: Class::Axe, art: "chieftains_cleaver" }, "+10% damage, +8 poise. Art (Q), Hatchet: thrown down the lane, bites the first foe and comes back, biting again", |k| { k.damage *= 1.1; k.poise += 8.0; k.art = Some(WeaponArt::Hatchet); }),
    weapon("Gavel of Sentence", 8.0, Arms { base: 1.15, needs: [9, 0, 3], scales: [B, No, C], class: Class::Axe, art: "gavel_of_sentence" }, "judges the fallen: +60% to the open. Art (Q), Sentence: marks the foe ahead; a moment later a blow no guard turns falls on it", |k| { k.opener *= 1.6; k.art = Some(WeaponArt::Sentence); }),
    weapon("Reaper's Glaive", 5.0, Arms { base: 1.15, needs: [3, 8, 4], scales: [D, A, D], class: Class::Spear, art: "reapers_glaive" }, "reaps: 8% of each strike back as health. Art (Q), Reap: a sweep across every lane that drinks from all it cuts", |k| { k.leech += 0.08; k.art = Some(WeaponArt::Reap); }),
    weapon("Lance of Light", 3.0, Arms { base: 1.2, needs: [0, 7, 9], scales: [No, C, A], class: Class::Thrust, art: "lance_of_light" }, "weightless: strikes cost 25% less stamina. Art (Q), Light Lance: a dash down the lane, untouchable, piercing what it meets", |k| { k.strike *= 0.75; k.art = Some(WeaponArt::Lance); }),
    weapon("Stoneheart Greatsword", 14.0, Arms { base: 1.25, needs: [15, 2, 0], scales: [S, No, No], class: Class::Ultra, art: "stoneheart_greatsword" }, "stone: -10% damage taken, +20 poise. Art (Q), Stone Skin: ten seconds of stone (no blow staggers you, 30% less gets through) and a stomp that throws back what is near", |k| { k.taken *= 0.9; k.poise += 20.0; k.art = Some(WeaponArt::StoneSkin); }),
    weapon("Feather of the Fallen Light", 3.0, Arms { base: 1.25, needs: [7, 7, 7], scales: [C, C, C], class: Class::Sword, art: "feather_of_the_fallen_light" }, "+15% damage, 5% back as health, +25% to the open. Art (Q), Wingbeat: a gust throws everything ahead back, then a dive strikes down on the nearest", |k| { k.damage *= 1.15; k.leech += 0.05; k.opener *= 1.25; k.art = Some(WeaponArt::Wingbeat); }),
    // Left where it was laid down (`atlas::Place::loot`).
    weapon("Gravewarden's Slab", 18.0, Arms { base: 1.0, needs: [16, 0, 0], scales: [A, No, No], class: Class::Ultra, art: "gravewarden_slab" }, "both hands; every swing sweeps the lanes beside and rides out a flinch", |_| {}),
    // In the other hand instead of a shield (F: a ranged attack; a catalyst, C: a spell): no
    // parry flick, blocks on the blade.
    ranged("Hunter's Shortbow", 1.5, Ranged::Bow, Scaling { base: 1.0, needs: [0, 2, 0], scales: [E, B, No] }, "F: an arrow down the lane, 12 to a quiver, two taken back from each foe felled; a long draw any blow cuts off; dexterity"),
    ranged("Bloodrunner's Bow", 1.5, Ranged::BloodBow, Scaling { base: 1.0, needs: [0, 3, 0], scales: [No, B, D] }, "F: an arrow drawn from your own blood, 5 health each; a long draw any blow cuts off; dexterity, a little focus"),
    catalyst("Frostweaver's Focus", 1.0, Catalyst { scaling: Scaling { base: 1.0, needs: [0, 0, 2], scales: [No, No, B] }, charges: 0.0, own: Some(Spell::FrostBolt), art: "frost_focus" }, "a catalyst, C: the spell attuned, else its own frost bolt; a charge each, filled by blows landed up close and at the shrine; focus"),
    // Found later.
    ranged("Blood Crossbow", 2.5, Ranged::BloodCrossbow, Scaling { needs: [4, 2, 0], ..UNSCALED }, "F: a heavy bolt drawn from your own blood, 7 health each; quick to aim, slow to wind again; raised by no attribute, as a crossbow is"),
    spell("Frost Bolt", Spell::FrostBolt, "a spell (focus 3 to cast), C through a catalyst: a frost bolt that cracks a stance and chills a foe (a third slower, its stance held) for four seconds; a charge"),
];

/// What the smith forges from a boss's remnant: the remnant, the piece, and its price in echoes
/// and shards. A large sum: a remnant is the start of a weapon, not the whole of it.
pub struct Recipe { pub remnant: &'static str, pub item: &'static str, pub echoes: u32, pub shards: u32 }
pub const RECIPES: &[Recipe] = &[
    Recipe { remnant: "Bell Warden's Clapper", item: "Bell-Hammer", echoes: 2500, shards: 6 },
    Recipe { remnant: "Screamer's Drowned Voice", item: "Drowned Wail", echoes: 3500, shards: 8 },
    Recipe { remnant: "Chief's Notched Axe-Head", item: "Chieftain's Cleaver", echoes: 4500, shards: 10 },
    Recipe { remnant: "Magistrate's Seal", item: "Gavel of Sentence", echoes: 6000, shards: 12 },
    Recipe { remnant: "Wraith Lord's Hollow Crown", item: "Reaper's Glaive", echoes: 7500, shards: 14 },
    Recipe { remnant: "Shard of the Priest's Halo", item: "Lance of Light", echoes: 9000, shards: 16 },
    Recipe { remnant: "The Statue's Stone Heart", item: "Stoneheart Greatsword", echoes: 11000, shards: 18 },
    Recipe { remnant: "A Feather of the Archangel", item: "Feather of the Fallen Light", echoes: 15000, shards: 20 },
];

/// The index of the piece called `name`.
pub fn find(name: &str) -> usize { ITEMS.iter().position(|i| i.name == name).unwrap_or_else(|| panic!("no piece called {name}")) }

/// What each kind of enemy can leave behind, likeliest first. The first of each kind felled
/// leaves the first piece the hero lacks; after that, one in eight.
pub fn drops(k: Archetype) -> &'static [&'static str] {
    match k {
        Archetype::Husk => &["Husk Cleaver", "Pilgrim's Token", "Hoarder's Ring"],
        Archetype::Lancer => &["Lancer's Shortspear", "Lancer's Greaves", "Ring of the Quiet Step", "Duelist's Rapier"],
        Archetype::Shieldbearer => &["Kite Shield", "Iron Helm", "Shieldbearer's Gauntlets", "Chain Hauberk", "Executioner's Greatsword"],
        Archetype::Wretch => &["Ember Ring", "Duelist's Parma", "Vigil Ring", "Keeper's Sigil", "Rimed Sabre"],
        Archetype::Archer => &["Hunter's Shortbow", "Ring of the Quiet Step", "Bloodrunner's Bow", "Lancer's Greaves", "Blood Crossbow", "Vigil Ring"],
        Archetype::Hexer => &["Frostweaver's Focus", "Frost Bolt", "Keeper's Sigil", "Ember Ring", "Pilgrim's Token"],
    }
}

/// What the hero owns and what of it they wear (by `SLOTS` place).
#[derive(Clone, Debug, PartialEq)]
pub struct Loadout {
    pub owned: Vec<usize>,
    pub worn: [Option<usize>; 10],
}

impl Loadout {
    /// What the hero sets out with, all worn.
    pub fn starting() -> Loadout {
        let mut l = Loadout { owned: (0..6).collect(), worn: [None; 10] };
        for i in 0..6 { l.worn[i] = Some(i); }
        l
    }

    /// Everything worn, together.
    pub fn kit(&self) -> Kit {
        // The other hand holds only what is worn in it.
        let mut k = Kit { off: None, ..Kit::default() };
        for &i in self.worn.iter().flatten() {
            (ITEMS[i].apply)(&mut k);
            k.load += ITEMS[i].weight;
            if let Some(a) = ITEMS[i].arms { k.arms = a; }
            if let Some(o) = ITEMS[i].off { k.off = Some(o); }
            if let Some(r) = ITEMS[i].ranged { k.ranged = Some(r); }
            if let Some(sc) = ITEMS[i].shooting { k.shooting = sc; }
            if let Some(c) = ITEMS[i].catalyst { k.catalyst = Some(c); }
            if let Some(sp) = ITEMS[i].spell { k.spell = Some(sp); }
        }
        // Nothing in the other hand: a blow is blocked on the weapon, worse than on a shield.
        if k.off.is_none() { k.guard *= 1.25; k.chip *= 1.5; }
        k
    }

    /// Whether the weapon worn takes both hands.
    pub fn two_handed(&self) -> bool { self.worn[0].and_then(|i| ITEMS[i].arms).is_some_and(|a| a.class.two_handed()) }

    pub fn owns(&self, i: usize) -> bool { self.owned.contains(&i) }

    /// Pieces owned that could go in place `p`, not worn in another place.
    pub fn fits(&self, p: usize) -> Vec<usize> {
        let slot = SLOTS[p].0;
        // Nothing goes beside a weapon held in both hands.
        if slot == Slot::Offhand && self.two_handed() { return Vec::new(); }
        self.owned.iter().copied().filter(|&i| ITEMS[i].slot == slot && !self.worn.iter().enumerate().any(|(q, w)| q != p && *w == Some(i))).collect()
    }

    /// Wear `item` in place `p` (None: take off what is there). False if it does not go there.
    pub fn wear(&mut self, p: usize, item: Option<usize>) -> bool {
        if let Some(i) = item { if !self.fits(p).contains(&i) { return false; } }
        self.worn[p] = item;
        // Taking up a weapon in both hands puts the offhand away.
        if p == 0 && self.two_handed() { self.worn[1] = None; }
        true
    }

    /// Take a piece found. False if it was owned already.
    pub fn gain(&mut self, i: usize) -> bool {
        if self.owns(i) { return false; }
        self.owned.push(i);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_forged_weapon_has_its_own_art_and_says_what_it_does() {
        let mut seen = Vec::new();
        for i in ITEMS.iter() {
            let mut k = Kit::default();
            (i.apply)(&mut k);
            let Some(a) = k.art else { continue };
            assert!(i.text.contains(&format!("Art (Q), {}: {}", a.name(), a.text())), "{}: {}", i.name, i.text);
            assert!(!seen.contains(&a), "{a:?} twice");
            seen.push(a);
        }
        assert_eq!(seen.len(), WeaponArt::ALL.len());
    }

    #[test]
    fn the_starting_kit_changes_nothing_and_a_piece_changes_what_it_says() {
        let mut l = Loadout::starting();
        let k = l.kit();
        assert_eq!(Kit { load: 0.0, ..k }, Kit::default(), "the sword is the kit's own default");
        assert!(k.load > 5.0 && k.load < 10.0, "{}", k.load);
        let mail = find("Chain Hauberk");
        assert!(!l.wear(3, Some(mail)), "not owned yet");
        assert!(l.gain(mail) && !l.gain(mail));
        assert!(l.wear(3, Some(mail)));
        let k = l.kit();
        assert!((k.taken - 0.88).abs() < 1e-6 && k.poise == 12.0);
        assert!(!l.wear(0, Some(mail)), "mail is not a weapon");
    }

    #[test]
    fn the_weapon_held_brings_its_needs_and_grades() {
        let mut l = Loadout::starting();
        assert_eq!(l.kit().arms, SWORD);
        let cleaver = find("Husk Cleaver");
        l.gain(cleaver);
        assert!(l.wear(0, Some(cleaver)));
        assert_eq!((l.kit().arms.needs, l.kit().arms.scales[0]), ([4, 0, 0], Grade::A));
        for i in ITEMS { assert_eq!(i.arms.is_some(), i.slot == Slot::Weapon, "{}", i.name); }
    }

    #[test]
    fn the_offhand_worn_is_what_guards_and_an_empty_hand_blocks_on_the_blade() {
        let mut l = Loadout::starting();
        assert_eq!(l.kit().off.map(|o| o.guard), Some(Guard::Buckler));
        let kite = find("Kite Shield");
        l.gain(kite);
        assert!(l.wear(1, Some(kite)));
        assert_eq!(l.kit().off.map(|o| o.guard), Some(Guard::Kite));
        assert!(l.wear(1, None));
        let k = l.kit();
        assert_eq!(k.off, None);
        assert!((k.guard - 1.25).abs() < 1e-6 && (k.chip - 1.5).abs() < 1e-6, "{k:?}");
        for i in ITEMS {
            assert_eq!(i.off.is_some() || i.ranged.is_some() || i.catalyst.is_some(), i.slot == Slot::Offhand, "{}", i.name);
            assert_eq!(i.spell.is_some(), i.slot == Slot::Spell, "{}", i.name);
        }
        // A bow in the other hand is no shield: it blocks on the blade.
        let bow = find("Hunter's Shortbow");
        l.gain(bow);
        assert!(l.wear(1, Some(bow)));
        let k = l.kit();
        assert_eq!((k.off, k.ranged), (None, Some(Ranged::Bow)));
        assert!((k.guard - 1.25).abs() < 1e-6, "{k:?}");
    }

    #[test]
    fn a_spear_is_held_in_both_hands_so_it_puts_the_shield_away_too() {
        let mut l = Loadout::starting();
        let spear = find("Lancer's Shortspear");
        l.gain(spear);
        assert!(l.wear(0, Some(spear)));
        assert_eq!((l.worn[1], l.kit().off), (None, None));
        assert!(l.fits(1).is_empty());
    }

    #[test]
    fn a_two_handed_weapon_puts_the_shield_away_and_keeps_it_off() {
        let mut l = Loadout::starting();
        let great = find("Executioner's Greatsword");
        l.gain(great);
        assert!(l.worn[1].is_some());
        assert!(l.wear(0, Some(great)));
        assert_eq!(l.worn[1], None, "the buckler went back on the belt");
        assert!(l.fits(1).is_empty() && !l.wear(1, Some(find("Pilgrim's Buckler"))));
        assert!(l.kit().guard > 1.2, "blocking on the blade costs more");
        assert!(l.wear(0, Some(find("Wayfarer's Sword"))) && l.wear(1, Some(find("Pilgrim's Buckler"))));
    }

    #[test]
    fn one_ring_cannot_be_worn_twice() {
        let mut l = Loadout::starting();
        let ember = find("Ember Ring");
        l.gain(ember);
        assert!(l.wear(6, Some(ember)));
        assert!(l.fits(7).is_empty(), "the other ring place has nothing left to take");
        assert!(!l.wear(7, Some(ember)));
        assert!((l.kit().damage - 1.1).abs() < 1e-6);
    }

    #[test]
    fn every_region_boss_leaves_a_remnant_the_smith_can_forge() {
        for p in crate::atlas::PLACES {
            let Some((_, b)) = p.boss else { continue };
            if b.mini { assert!(b.remnant.is_none(), "{}", b.name()); continue; }
            let r = b.remnant.unwrap_or_else(|| panic!("{} leaves no remnant", b.name()));
            let recipe = RECIPES.iter().find(|x| x.remnant == r).unwrap_or_else(|| panic!("nothing forges {r}"));
            let item = &ITEMS[find(recipe.item)];
            assert!(item.arms.is_some(), "{} is a weapon", item.name);
        }
        // Dearer the deeper the boss.
        assert!(RECIPES.windows(2).all(|w| w[0].echoes < w[1].echoes));
    }

    #[test]
    fn every_drop_names_a_piece() {
        for k in [Archetype::Husk, Archetype::Lancer, Archetype::Shieldbearer, Archetype::Wretch] {
            for n in drops(k) { find(n); }
        }
    }
}
