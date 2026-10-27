//! What a piece reinforced to its top is given besides, chosen once at the smith from the three
//! its kind can take: a power (`Capped`, read by both fights), and a look of its own on the hero,
//! a glow and motes on the piece in the colour and motion of the choice (`game` draws it).

use crate::gear::{Item, Kit, Slot};

/// A capstone: three for each kind of piece, a weapon's, a shield's, a bow's, a catalyst's and
/// armour's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capstone {
    Ember, Rime, Thirst,
    Bastion, Thorns, Sanctuary,
    Searing, Hoarfrost, Keen,
    Wellspring, Winter, Siphon,
    Stalwart, Vigil, Fleet,
}

/// How a capstone's motes move on its piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motes {
    /// Sparks rising off it and going out, as off a coal.
    Rise,
    /// Glints winking on and off where they are, as frost catching the light.
    Twinkle,
    /// Drops gathering on it and falling.
    Drip,
    /// No motes: the glow alone, breathing slowly.
    Glow,
}

/// Ticks a burn lasts, and what of a blow it deals again over them (`Capstone::Ember`, `Searing`).
pub const BURN_TICKS: u32 = 180;
pub const BURN_SHARE: f32 = 0.35;
/// Ticks a rimed strike chills for, and a hoarfrost shot.
pub const RIME_CHILL: u32 = 90;
pub const HOARFROST_CHILL: u32 = 120;

/// What the capstones on what is worn give together, and which they are, by `gear::SLOTS` place
/// (for their look).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capped {
    /// Of a strike, and of a shot, what burns on after it (`fight::Burn`).
    pub burn: f32,
    pub shot_burn: f32,
    /// Ticks a strike chills for, and a shot (0: not at all, past a frost bolt's own).
    pub rime: u32,
    pub shot_chill: u32,
    /// Every arrow and bolt goes on through what it strikes.
    pub keen: bool,
    /// Of a blow blocked, what goes back into its striker.
    pub thorns: f32,
    /// Of full health, what a parry or a perfect block mends.
    pub mend: f32,
    /// Of what a spell deals, what comes back as health; and how much harder spells hit.
    pub spell_drink: f32,
    pub spell_more: f32,
    /// A spell's chill, times its own.
    pub winter: u32,
    /// The capstones worn, by place.
    pub worn: [Option<Capstone>; 6],
}

impl Capped {
    pub const NONE: Capped = Capped { burn: 0.0, shot_burn: 0.0, rime: 0, shot_chill: 0, keen: false, thorns: 0.0, mend: 0.0, spell_drink: 0.0, spell_more: 1.0, winter: 1, worn: [None; 6] };
}

impl Capstone {
    pub const ALL: [Capstone; 15] = [
        Capstone::Ember, Capstone::Rime, Capstone::Thirst, Capstone::Bastion, Capstone::Thorns, Capstone::Sanctuary,
        Capstone::Searing, Capstone::Hoarfrost, Capstone::Keen, Capstone::Wellspring, Capstone::Winter, Capstone::Siphon,
        Capstone::Stalwart, Capstone::Vigil, Capstone::Fleet,
    ];

    /// The three `item` can take, by what it is: none for a ring, a talisman or a spell.
    pub fn choices(item: &Item) -> &'static [Capstone] {
        use Capstone::*;
        match item.slot {
            Slot::Weapon => &[Ember, Rime, Thirst],
            Slot::Offhand if item.off.is_some() => &[Bastion, Thorns, Sanctuary],
            Slot::Offhand if item.catalyst.is_some() => &[Wellspring, Winter, Siphon],
            Slot::Offhand if item.ranged.is_some() => &[Searing, Hoarfrost, Keen],
            Slot::Head | Slot::Body | Slot::Hands | Slot::Legs => &[Stalwart, Vigil, Fleet],
            _ => &[],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Capstone::Ember => "Ember Edge", Capstone::Rime => "Rime Edge", Capstone::Thirst => "Thirsting Edge",
            Capstone::Bastion => "Bastion", Capstone::Thorns => "Thorned Face", Capstone::Sanctuary => "Sanctuary",
            Capstone::Searing => "Searing Shot", Capstone::Hoarfrost => "Hoarfrost Shot", Capstone::Keen => "Keen Shot",
            Capstone::Wellspring => "Wellspring", Capstone::Winter => "Deep Winter", Capstone::Siphon => "Siphon",
            Capstone::Stalwart => "Stalwart", Capstone::Vigil => "Vigil", Capstone::Fleet => "Fleet",
        }
    }

    /// What it does, in a line.
    pub fn text(self) -> &'static str {
        match self {
            Capstone::Ember => "strikes set foes burning: a third of the blow again over three seconds",
            Capstone::Rime => "strikes chill: the foe a third slower for a second and a half",
            Capstone::Thirst => "strikes drink 6% of what they deal back as health",
            Capstone::Bastion => "blocks cost a quarter less stamina and let half as much through",
            Capstone::Thorns => "a blow blocked hurts its striker for half of what it would have dealt",
            Capstone::Sanctuary => "a parry or a perfect block mends 6% of your health",
            Capstone::Searing => "shots set foes burning: a third of the shot again over three seconds",
            Capstone::Hoarfrost => "shots chill: the foe a third slower for two seconds",
            Capstone::Keen => "every arrow and bolt goes on through what it strikes",
            Capstone::Wellspring => "one spell charge more",
            Capstone::Winter => "spells hit 15% harder and chill twice as long",
            Capstone::Siphon => "spells drink 20% of what they deal back as health",
            Capstone::Stalwart => "10 more poise: more blows before a stagger",
            Capstone::Vigil => "15 more health",
            Capstone::Fleet => "15 more stamina",
        }
    }

    /// The colour of its glow and motes.
    pub fn colour(self) -> [u8; 3] {
        match self {
            Capstone::Ember | Capstone::Searing => [255, 128, 40],
            Capstone::Rime | Capstone::Hoarfrost | Capstone::Winter => [150, 215, 255],
            Capstone::Thirst | Capstone::Siphon => [240, 45, 60],
            Capstone::Bastion => [255, 205, 90],
            Capstone::Thorns => [150, 200, 70],
            Capstone::Sanctuary => [235, 240, 255],
            Capstone::Keen => [245, 245, 220],
            Capstone::Wellspring => [175, 110, 255],
            Capstone::Stalwart => [215, 150, 75],
            Capstone::Vigil => [255, 95, 85],
            Capstone::Fleet => [110, 235, 150],
        }
    }

    pub fn motes(self) -> Motes {
        match self {
            Capstone::Ember | Capstone::Searing | Capstone::Wellspring => Motes::Rise,
            Capstone::Rime | Capstone::Hoarfrost | Capstone::Winter | Capstone::Keen | Capstone::Sanctuary => Motes::Twinkle,
            Capstone::Thirst | Capstone::Siphon | Capstone::Thorns => Motes::Drip,
            Capstone::Bastion | Capstone::Stalwart | Capstone::Vigil | Capstone::Fleet => Motes::Glow,
        }
    }

    /// Worn at `place`, onto `k`.
    pub fn apply(self, k: &mut Kit, place: usize) {
        let capped = &mut k.capped;
        capped.worn[place] = Some(self);
        match self {
            Capstone::Ember => capped.burn = BURN_SHARE,
            Capstone::Rime => capped.rime = RIME_CHILL,
            Capstone::Thirst => k.leech += 0.06,
            Capstone::Bastion => { k.guard *= 0.75; k.chip *= 0.5; }
            Capstone::Thorns => capped.thorns = 0.5,
            Capstone::Sanctuary => capped.mend = 0.06,
            Capstone::Searing => capped.shot_burn = BURN_SHARE,
            Capstone::Hoarfrost => capped.shot_chill = HOARFROST_CHILL,
            Capstone::Keen => capped.keen = true,
            Capstone::Wellspring => k.charges += 1.0,
            Capstone::Winter => { capped.spell_more = 1.15; capped.winter = 2; }
            Capstone::Siphon => capped.spell_drink = 0.2,
            Capstone::Stalwart => k.poise += 10.0,
            Capstone::Vigil => k.hp += 15.0,
            Capstone::Fleet => k.stamina += 15.0,
        }
    }

    /// Its name as a save keeps it, and back.
    pub fn named(name: &str) -> Option<Capstone> { Self::ALL.into_iter().find(|c| c.name() == name) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gear::{find, ITEMS};

    #[test]
    fn each_kind_of_piece_takes_its_own_three_and_each_name_reads_back() {
        let of = |name: &str| Capstone::choices(&ITEMS[find(name)]);
        assert_eq!(of("Hunter's Shortbow"), &[Capstone::Searing, Capstone::Hoarfrost, Capstone::Keen]);
        assert_eq!(of("Pilgrim's Buckler")[0], Capstone::Bastion);
        assert_eq!(of("Wayfarer's Hood")[1], Capstone::Vigil);
        assert!(ITEMS.iter().filter(|i| i.slot.most() > 0).all(|i| Capstone::choices(i).len() == 3), "every piece the smith can take to its top has three");
        assert!(ITEMS.iter().filter(|i| i.slot.most() == 0).all(|i| Capstone::choices(i).is_empty()));
        assert!(Capstone::ALL.iter().all(|&c| Capstone::named(c.name()) == Some(c)));
    }
}
