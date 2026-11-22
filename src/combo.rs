//! Combo routes: a light or heavy pressed in the recovery of the strike before cuts it short and
//! becomes the next of its string. Lights run three to a string (`fight::Spec::chained`); a heavy
//! out of one light is a breaker, out of two or more the weapon's finisher, out of a heavy a
//! follow-through; a light out of a heavy cancels its recovery. A finisher can be held to gather,
//! and gathered to the full it lands the weapon's own finishing blow (`Finish`). The road and the
//! yard read the same table.

use crate::fight::{Move, Spec, CHAIN, STUMBLE};
use crate::gear::{Class, Ranged};

/// What a strike is in its string.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Route {
    /// On its own, or a light of a string.
    #[default]
    Plain,
    /// A heavy straight out of one light (L H): quick to come and hard on stance.
    Breaker,
    /// A heavy out of two or more lights (L L H): the weapon's signature, harder and slower to
    /// recover from; it can be gathered, and gathered to the full lands its `Finish`.
    Finisher,
    /// A second heavy carried on the first's swing (H H): quicker to wind up, not gathered.
    FollowThrough,
}

/// What a press cut into a strike's recovery starts: a light of the string at its link, or a heavy
/// by its route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next { Light(u8), Heavy(Route) }

/// What a fully gathered finisher does besides its strength, by the weapon swung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finish {
    /// A sword's wheeling cut: the lanes either side too (in the yard, behind as well).
    Whirl,
    /// A rapier's lunging run-through: every foe in the lane it reaches.
    Pierce,
    /// A spear's lunge: half again as far, and what it meets is pinned off balance longer.
    Pin,
    /// An axe's overhead cleave: the stance broken outright (a boss only reels).
    Cleave,
    /// A greatsword's rising sweep: the lanes either side, all of them thrown off balance long.
    Sweep,
    /// An ultra greatsword's slam: a shockwave through every lane, further than its reach.
    Quake,
}

/// What a string remembers between presses: the other strike pressed while one already waits to
/// cut the swing short (a heavy after a light plays next, where it used to replace the light), and
/// the strike that just ended with the ticks since (a press within `CHAIN_GRACE` still follows on).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChainMemory<P: Copy> {
    pub queued: Option<P>,
    ended: Option<((Move, Route, u8), u32)>,
}

/// Ticks after a strike's recovery in which a press still follows on as the next of its string.
pub const CHAIN_GRACE: u32 = 10;

/// How much longer a pinned or swept foe stays off balance than a plain gathered blow leaves it.
const LONG_REEL: f32 = 1.6;
/// How much further than its reach an ultra greatsword's slam carries, metres.
const QUAKE_REACH: f32 = 2.0;

impl Route {
    /// What a press of `next` becomes, cut into the recovery of `prev` (its move, route and
    /// link); None when it does not follow on (it waits for the swing to end, as any press).
    pub fn follows((mv, route, link): (Move, Route, u8), next: Move) -> Option<Next> {
        match (mv, route, next) {
            (Move::Light, _, Move::Light) => (link + 1 < CHAIN).then_some(Next::Light(link + 1)),
            (Move::Light, _, Move::Heavy) => Some(Next::Heavy(if link == 0 { Route::Breaker } else { Route::Finisher })),
            (Move::Heavy, Route::Plain, Move::Light) => Some(Next::Light(0)),
            (Move::Heavy, Route::Plain, Move::Heavy) => Some(Next::Heavy(Route::FollowThrough)),
            _ => None,
        }
    }

    /// `spec` (a heavy's) as this route swings it.
    pub fn shape(self, spec: Spec) -> Spec {
        let times = |t: u32, k: f32| ((t as f32 * k).round() as u32).max(1);
        let (wind, live, rest) = spec.frames;
        match self {
            Route::Plain => spec,
            Route::Breaker => Spec { frames: (times(wind, 0.65), live, rest), damage: spec.damage * 0.9, stance: spec.stance * 1.5, stamina: spec.stamina * 0.9, ..spec },
            Route::Finisher => Spec { frames: (times(wind, 0.9), live, times(rest, 1.3)), damage: spec.damage * 1.25, stance: spec.stance * 1.35, stamina: spec.stamina * 1.2, ..spec },
            Route::FollowThrough => Spec { frames: (times(wind, 0.75), live, rest), damage: spec.damage * 0.95, stamina: spec.stamina * 0.9, ..spec },
        }
    }

    /// The clip a strike on this route swings in place of its plain light or heavy
    /// (`<weapon>_<clip>`, side-on `e_<weapon>_<clip>`): the last light of a string (`link`) and
    /// every heavy that ends one has a swing of its own; None swings the plain one.
    pub fn clip_for(self, mv: Move, link: u8) -> Option<&'static str> {
        match (mv, self) {
            (Move::Light, _) => (link + 1 >= CHAIN).then_some("third"),
            (Move::Heavy, Route::Breaker) => Some("breaker"),
            (Move::Heavy, Route::Finisher) => Some("finisher"),
            (Move::Heavy, Route::FollowThrough) => Some("follow"),
            _ => None,
        }
    }

    /// Whether a heavy on this route can be held to gather strength.
    pub fn gathers(self) -> bool { matches!(self, Route::Plain | Route::Finisher) }

    /// Whether `mv` on this route, from a kit shooting `shot` for a loose, can be held to gather:
    /// a heavy that `gathers`, or an arrow drawn on a bow (a crossbow and a spell go when ready).
    pub fn holds(self, mv: Move, shot: Option<Ranged>) -> bool {
        match mv {
            Move::Heavy => self.gathers(),
            Move::Loose => shot.is_some_and(|r| matches!(r, Ranged::Bow | Ranged::BloodBow | Ranged::Longbow)),
            _ => false,
        }
    }
}

/// The combo list's rows for the pause menu (`shrine::Front::Combos`), the finisher of a weapon
/// swung as `held` marked as the hero's own: each the buttons, a tab, what it is, and under it what
/// it does.
pub fn list(held: Class) -> Vec<String> {
    let mut rows = vec![
        "Strike, strike, strike\tthe string\nThree quick blows, each pressed as the one before recovers: safe, steady damage.".to_owned(),
        "Strike, heavy\tbreaker\nA heavy straight out of one strike: quick to come, and it cracks a guard's stance.".to_owned(),
        "Strike, strike, heavy\tfinisher\nYour weapon's signature heavy: harder, slower to recover from. Hold it to gather; gathered to the full it lands the weapon's finishing blow.".to_owned(),
        "Heavy, strike\trecovery cut\nA strike cut into a heavy's recovery: cover its slow ending.".to_owned(),
        "Heavy, heavy\tfollow-through\nA second heavy carried on the first's swing: quicker to wind up.".to_owned(),
    ];
    rows.extend(Finish::ALL.iter().map(|&(class, f)| {
        let yours = if class == held { "  (yours)" } else { "" };
        let mut weapon = class.name().to_owned();
        if let Some(first) = weapon.get_mut(..1) { first.make_ascii_uppercase(); }
        format!("{weapon} finisher\t{}{yours}\n{}", f.name(), f.does())
    }));
    rows.push("Hold a heavy\tgather\nHeld at the top of its swing, a heavy gathers up to twice its blow.".to_owned());
    rows.push("Hold the shot (a bow)\tfull draw\nA bow held drawn gathers up to twice the arrow; a crossbow and a spell go as soon as they are ready.".to_owned());
    rows
}

impl<P: Copy> Default for ChainMemory<P> {
    fn default() -> Self { Self { queued: None, ended: None } }
}

impl<P: Copy> ChainMemory<P> {
    /// One tick on: what ended longer ago than `CHAIN_GRACE` is forgotten.
    pub fn tick(&mut self) {
        self.ended = self.ended.filter(|e| e.1 < CHAIN_GRACE).map(|(strike, age)| (strike, age + 1));
    }
    /// `strike` (its move, route and link) has run to the end of its recovery.
    pub fn ended(&mut self, strike: (Move, Route, u8)) {
        if matches!(strike.0, Move::Light | Move::Heavy) { self.ended = Some((strike, 0)); }
    }
    /// The strike a press now would follow on from, the hero being free.
    pub fn after(&self) -> Option<(Move, Route, u8)> { self.ended.map(|e| e.0) }
    /// A new move began: nothing is left to follow on from.
    pub fn began(&mut self) { self.ended = None; }
}

impl Next {
    /// The light link and the heavy route it sets for the press it stands for.
    pub fn parts(self) -> (u8, Route) {
        match self { Next::Light(link) => (link, Route::Plain), Next::Heavy(route) => (0, route) }
    }
}

impl Finish {
    /// Every weapon's finishing blow, by how it is swung, in the order the combo list shows them.
    pub const ALL: [(Class, Finish); 6] = [
        (Class::Sword, Finish::Whirl), (Class::Thrust, Finish::Pierce), (Class::Spear, Finish::Pin),
        (Class::Axe, Finish::Cleave), (Class::Greatsword, Finish::Sweep), (Class::Ultra, Finish::Quake),
    ];

    pub fn name(self) -> &'static str {
        match self {
            Finish::Whirl => "wheeling cut", Finish::Pierce => "run-through", Finish::Pin => "lunge",
            Finish::Cleave => "cleave", Finish::Sweep => "rising sweep", Finish::Quake => "ground slam",
        }
    }

    /// What it does, as the combo list says it.
    pub fn does(self) -> &'static str {
        match self {
            Finish::Whirl => "Carried round from far back: the lanes either side too, and in a yard the foe behind.",
            Finish::Pierce => "A full lunge: through every foe in the lane it reaches.",
            Finish::Pin => "Half again as far; what it meets stays off balance longer.",
            Finish::Cleave => "Breaks the stance outright; a boss only reels.",
            Finish::Sweep => "The lanes either side, all of them thrown off balance long.",
            Finish::Quake => "A shockwave through every lane, further than its reach.",
        }
    }

    /// The finishing blow of a weapon swung as `class`.
    pub fn of(class: Class) -> Finish {
        Finish::ALL.iter().find(|(c, _)| *c == class).map_or(Finish::Whirl, |&(_, f)| f)
    }

    /// `spec` as the finishing blow reaches: wider, or further.
    pub fn widen(self, spec: Spec) -> Spec {
        match self {
            Finish::Whirl | Finish::Sweep => Spec { wide: spec.wide.max(1), ..spec },
            Finish::Pin => Spec { reach: spec.reach * 1.5, ..spec },
            Finish::Quake => Spec { wide: spec.wide.max(1), reach: spec.reach + QUAKE_REACH, ..spec },
            Finish::Pierce | Finish::Cleave => spec,
        }
    }

    /// Whether it strikes every foe it reaches in its lane, not only the nearest.
    pub fn is_through(self) -> bool { matches!(self, Finish::Pierce | Finish::Quake) }

    /// Whether it strikes a foe standing behind the hero (in the yard, where there are no lanes).
    pub fn is_round(self) -> bool { matches!(self, Finish::Whirl | Finish::Quake) }

    /// Whether it breaks the stance of what it strikes outright.
    pub fn is_breaking(self) -> bool { self == Finish::Cleave }

    /// How much longer than a plain gathered blow what it strikes stays off balance.
    pub fn reel_scale(self) -> f32 { if matches!(self, Finish::Pin | Finish::Sweep) { LONG_REEL } else { 1.0 } }

    /// Ticks a road enemy it strikes stumbles.
    pub fn stumble(self) -> u32 { (STUMBLE as f32 * self.reel_scale()).round() as u32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_of_routes_reads_as_the_buttons_do() {
        let light = |link| (Move::Light, Route::Plain, link);
        assert_eq!(Route::follows(light(0), Move::Light), Some(Next::Light(1)));
        assert_eq!(Route::follows(light(CHAIN - 1), Move::Light), None, "three lights to a string");
        assert_eq!(Route::follows(light(0), Move::Heavy), Some(Next::Heavy(Route::Breaker)));
        assert_eq!(Route::follows(light(1), Move::Heavy), Some(Next::Heavy(Route::Finisher)));
        assert_eq!(Route::follows(light(2), Move::Heavy), Some(Next::Heavy(Route::Finisher)));
        assert_eq!(Route::follows((Move::Heavy, Route::Plain, 0), Move::Light), Some(Next::Light(0)));
        assert_eq!(Route::follows((Move::Heavy, Route::Plain, 0), Move::Heavy), Some(Next::Heavy(Route::FollowThrough)));
        // A breaker, finisher or follow-through ends the string: the hero is left to recover.
        for r in [Route::Breaker, Route::Finisher, Route::FollowThrough] {
            assert_eq!((Route::follows((Move::Heavy, r, 0), Move::Light), Route::follows((Move::Heavy, r, 0), Move::Heavy)), (None, None));
        }
    }

    #[test]
    fn a_breaker_comes_quick_and_a_finisher_hits_hard_and_leaves_the_hero_open() {
        let heavy = Move::Heavy.spec_for(Class::Sword, None);
        let (b, f) = (Route::Breaker.shape(heavy), Route::Finisher.shape(heavy));
        assert!(b.frames.0 < heavy.frames.0 && b.stance > heavy.stance);
        assert!(f.damage > heavy.damage && f.frames.2 > heavy.frames.2);
        assert!(Route::Finisher.gathers() && !Route::Breaker.gathers() && !Route::FollowThrough.gathers());
        assert!(Route::Plain.holds(Move::Loose, Some(Ranged::Longbow)) && !Route::Plain.holds(Move::Loose, Some(Ranged::BloodCrossbow)));
    }

    #[test]
    fn a_strike_just_ended_is_followed_on_from_for_a_moment_then_forgotten() {
        let mut memory = ChainMemory::<Move>::default();
        memory.ended((Move::Light, Route::Plain, 1));
        for _ in 0..CHAIN_GRACE { memory.tick(); }
        assert_eq!(memory.after(), Some((Move::Light, Route::Plain, 1)));
        memory.tick();
        assert_eq!(memory.after(), None, "past the grace it is gone");
        memory.ended((Move::Parry, Route::Plain, 0));
        assert_eq!(memory.after(), None, "only a strike is followed on from");
    }

    #[test]
    fn every_string_ends_on_a_swing_of_its_own() {
        assert_eq!(Route::Plain.clip_for(Move::Light, 0), None);
        assert_eq!(Route::Plain.clip_for(Move::Light, CHAIN - 1), Some("third"));
        assert_eq!(Route::Plain.clip_for(Move::Heavy, 0), None);
        let heavies = [Route::Breaker, Route::Finisher, Route::FollowThrough].map(|r| r.clip_for(Move::Heavy, 0));
        assert_eq!(heavies, [Some("breaker"), Some("finisher"), Some("follow")]);
    }

    #[test]
    fn the_combo_list_names_every_route_and_marks_the_weapon_held() {
        let rows = list(Class::Axe);
        assert_eq!(rows.iter().filter(|r| r.contains("(yours)")).count(), 1);
        assert!(rows.iter().any(|r| r.starts_with("Axe finisher\tcleave  (yours)")), "{rows:?}");
        assert!(rows.iter().all(|r| r.contains('\t') && r.contains('\n')));
        assert!(Finish::ALL.iter().all(|&(c, f)| Finish::of(c) == f));
    }
}
