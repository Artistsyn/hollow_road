//! What the hero wears and carries: a weapon, an offhand, four pieces of armour, two rings and a
//! talisman. Each piece changes the fight through a [`Kit`] (the sum of what is worn), and its
//! weight counts against what the hero can carry: past 70% of it, rolls cost more and stamina
//! comes back slower. Pieces come off the dead (`World::felled`); the smith's +levels apply to
//! whatever weapon and armour are worn.

use crate::fight::Archetype;

/// Where a piece goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot { Weapon, Offhand, Head, Body, Hands, Legs, Ring, Talisman }

/// The places to wear things, in the order the shrine lists them (two for rings).
pub const SLOTS: [(Slot, &str); 9] = [
    (Slot::Weapon, "Weapon"), (Slot::Offhand, "Offhand"), (Slot::Head, "Head"), (Slot::Body, "Body"),
    (Slot::Hands, "Hands"), (Slot::Legs, "Legs"), (Slot::Ring, "Ring"), (Slot::Ring, "Ring"), (Slot::Talisman, "Talisman"),
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

/// A weapon's side of a strike: its damage at no attribute, the levels of might, dexterity and
/// focus it needs (under any of them it is wielded weakly: `Stats::weak`), and its grade in each.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arms { pub base: f32, pub needs: [u32; 3], pub scales: [Grade; 3] }

/// The Wayfarer's sword: balanced, needing nothing. What a kit holds when no weapon is named.
pub const SWORD: Arms = Arms { base: 1.0, needs: [0, 0, 0], scales: [Grade::C, Grade::C, Grade::No] };

impl Default for Kit {
    fn default() -> Kit {
        Kit { damage: 1.0, taken: 1.0, poise: 0.0, guard: 1.0, chip: 1.0, parry: 0, roll: 1.0, strike: 1.0, regen: 1.0, heal: 1.0, hp: 0.0, stamina: 0.0, souls: 1.0, load: 0.0, arms: SWORD }
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
}

const fn item(name: &'static str, slot: Slot, weight: f32, text: &'static str, apply: fn(&mut Kit)) -> Item {
    Item { name, slot, weight, text, apply, arms: None }
}

const fn weapon(name: &'static str, weight: f32, arms: Arms, text: &'static str, apply: fn(&mut Kit)) -> Item {
    Item { name, slot: Slot::Weapon, weight, text, apply, arms: Some(arms) }
}

use Grade::{A, B, D, No};

/// Every piece there is. Indices are what a [`Loadout`] holds, so add to the end.
pub const ITEMS: &[Item] = &[
    // What the hero sets out with.
    weapon("Wayfarer's Sword", 3.0, SWORD, "balanced", |_| {}),
    item("Pilgrim's Buckler", Slot::Offhand, 1.5, "parries", |_| {}),
    item("Wayfarer's Hood", Slot::Head, 0.5, "light", |_| {}),
    item("Wayfarer's Cloak", Slot::Body, 2.0, "light", |_| {}),
    item("Leather Wraps", Slot::Hands, 0.5, "light", |_| {}),
    item("Road Boots", Slot::Legs, 1.0, "light", |_| {}),
    // Off the dead.
    weapon("Husk Cleaver", 6.0, Arms { base: 1.22, needs: [4, 0, 0], scales: [A, No, No] }, "heavy, strikes cost 20% more", |k| k.strike *= 1.2),
    weapon("Lancer's Shortspear", 4.0, Arms { base: 1.08, needs: [1, 3, 0], scales: [D, B, No] }, "quick, strikes cost 15% less", |k| k.strike *= 0.85),
    item("Kite Shield", Slot::Offhand, 6.0, "blocks cost 35% less, half the chip", |k| { k.guard *= 0.65; k.chip *= 0.5; }),
    item("Duelist's Parma", Slot::Offhand, 1.0, "parry live 4 ticks longer", |k| k.parry += 4),
    item("Iron Helm", Slot::Head, 3.0, "-5% damage taken, +5 poise", |k| { k.taken *= 0.95; k.poise += 5.0; }),
    item("Chain Hauberk", Slot::Body, 9.0, "-12% damage taken, +12 poise", |k| { k.taken *= 0.88; k.poise += 12.0; }),
    item("Shieldbearer's Gauntlets", Slot::Hands, 2.5, "blocks cost 12% less, -3% damage taken", |k| { k.guard *= 0.88; k.taken *= 0.97; }),
    item("Lancer's Greaves", Slot::Legs, 3.0, "+6 poise, -3% damage taken", |k| { k.poise += 6.0; k.taken *= 0.97; }),
    item("Ember Ring", Slot::Ring, 0.0, "+10% damage", |k| k.damage *= 1.10),
    item("Ring of the Quiet Step", Slot::Ring, 0.0, "rolls cost 25% less", |k| k.roll *= 0.75),
    item("Vigil Ring", Slot::Ring, 0.0, "stamina back 20% faster", |k| k.regen *= 1.2),
    item("Hoarder's Ring", Slot::Ring, 0.0, "+20% souls", |k| k.souls *= 1.2),
    item("Pilgrim's Token", Slot::Talisman, 0.0, "flasks heal 30% more", |k| k.heal *= 1.3),
    item("Keeper's Sigil", Slot::Talisman, 0.0, "+15 health, +10 stamina", |k| { k.hp += 15.0; k.stamina += 10.0; }),
    // The Bell Warden's.
    item("Warden's Bell", Slot::Talisman, 1.0, "+10 poise, +8% damage", |k| { k.poise += 10.0; k.damage *= 1.08; }),
    // Weapons for the attributes past might.
    weapon("Duelist's Rapier", 2.5, Arms { base: 0.95, needs: [0, 6, 0], scales: [No, A, No] }, "light, strikes cost 10% less", |k| k.strike *= 0.9),
    weapon("Rimed Sabre", 3.5, Arms { base: 1.0, needs: [0, 2, 5], scales: [No, D, B] }, "cold-forged, runs on focus", |_| {}),
];

/// The index of the piece called `name`.
pub fn find(name: &str) -> usize { ITEMS.iter().position(|i| i.name == name).unwrap_or_else(|| panic!("no piece called {name}")) }

/// What each kind of enemy can leave behind, likeliest first. The first of each kind felled
/// leaves the first piece the hero lacks; after that, one in eight.
pub fn drops(k: Archetype) -> &'static [&'static str] {
    match k {
        Archetype::Husk => &["Husk Cleaver", "Pilgrim's Token", "Hoarder's Ring"],
        Archetype::Lancer => &["Lancer's Shortspear", "Lancer's Greaves", "Ring of the Quiet Step", "Duelist's Rapier"],
        Archetype::Shieldbearer => &["Kite Shield", "Iron Helm", "Shieldbearer's Gauntlets", "Chain Hauberk"],
        Archetype::Wretch => &["Ember Ring", "Duelist's Parma", "Vigil Ring", "Keeper's Sigil", "Rimed Sabre"],
    }
}

/// What the hero owns and what of it they wear (by `SLOTS` place).
#[derive(Clone, Debug, PartialEq)]
pub struct Loadout {
    pub owned: Vec<usize>,
    pub worn: [Option<usize>; 9],
}

impl Loadout {
    /// What the hero sets out with, all worn.
    pub fn starting() -> Loadout {
        let mut l = Loadout { owned: (0..6).collect(), worn: [None; 9] };
        for i in 0..6 { l.worn[i] = Some(i); }
        l
    }

    /// Everything worn, together.
    pub fn kit(&self) -> Kit {
        let mut k = Kit::default();
        for &i in self.worn.iter().flatten() {
            (ITEMS[i].apply)(&mut k);
            k.load += ITEMS[i].weight;
            if let Some(a) = ITEMS[i].arms { k.arms = a; }
        }
        k
    }

    pub fn owns(&self, i: usize) -> bool { self.owned.contains(&i) }

    /// Pieces owned that could go in place `p`, not worn in another place.
    pub fn fits(&self, p: usize) -> Vec<usize> {
        let slot = SLOTS[p].0;
        self.owned.iter().copied().filter(|&i| ITEMS[i].slot == slot && !self.worn.iter().enumerate().any(|(q, w)| q != p && *w == Some(i))).collect()
    }

    /// Wear `item` in place `p` (None: take off what is there). False if it does not go there.
    pub fn wear(&mut self, p: usize, item: Option<usize>) -> bool {
        if let Some(i) = item { if !self.fits(p).contains(&i) { return false; } }
        self.worn[p] = item;
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
    fn every_drop_names_a_piece() {
        for k in [Archetype::Husk, Archetype::Lancer, Archetype::Shieldbearer, Archetype::Wretch] {
            for n in drops(k) { find(n); }
        }
    }
}
