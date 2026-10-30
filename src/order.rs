//! The Order of the Radiant Veil at home (`docs/story.md`, section 8): the two who stand between
//! the hero and the Eye in its stronghold. The Crimson Captain keeps the gate, a lance and the
//! gas pots its operatives carried up the Spire. Exodus Lightfoot waits in the Eye's chamber: the
//! monk who fought beside the hero, fists in strings, a hare's leap, gas rolled along the floor,
//! and past half his health the Radiant Hand, which runs straight into a string. He says what the
//! book only hints: the cloaked ones who first tore the Veil were the Order. Something came
//! through that first tear, a part of Voryn, and it has ridden the Order since as its Prophet.
//! When Exodus falls it comes through the door the Order built for it, Unit 0, and the fight goes
//! on against Voryn Made Whole; with Fin-S freed there is no door, and it takes Exodus instead,
//! as Voryn's Vessel (`finale`).

use crate::atlas::Boss;
use crate::fight::Kind::{GuardBreak, Physical, Unblockable};
use crate::side::{blow, boss, elite, Blow, Breed, Motion, LANCER_BASE, TPS};

/// The Captain: the agent's lance in threes, a lunge from afar, and a gas pot broken at its feet
/// that rolls out along the ground (jump it).
const CAPTAIN_BLOWS: [Blow; 4] = [
    blow("thrust", (22, 4, 12), (0.3, 2.6), (0.6, 1.8), 17.0, Physical, true, 22.0, Motion::Strike).then(2, 4, 8),
    blow("lunge", (30, 22, 30), (-0.4, 0.4), (0.0, 1.9), 20.0, GuardBreak, false, 32.0, Motion::Charge(8.5 / TPS)).art("charge"),
    blow("thrust again", (13, 4, 14), (0.3, 2.6), (0.6, 1.8), 17.0, Physical, true, 22.0, Motion::Strike).art("thrust").then(0, 0, 5),
    blow("gas pot", (40, 60, 34), (0.0, 0.0), (0.0, 0.55), 18.0, Unblockable, false, 0.0, Motion::Ring).art("thrust").tell(0.55),
];
pub static CAPTAIN: Breed = elite(&LANCER_BASE, "The Crimson Captain", 520.0, 3.2, &CAPTAIN_BLOWS,
    |near, r, _| match (near, r) { (true, 0..=5) => 0, (true, 6..=7) => 3, (true, _) => 2, (false, 0..=6) => 1, (false, _) => 3 }, ("order_agent", 2.6));

/// Exodus: quick strings to parry or block, a palm that breaks a guard, the same palm held back
/// (parry it late), a leap from afar onto where the hero stood, gas along the floor, a quick
/// answer to a beating, and past half his health the Radiant Hand straight into a string.
const EXODUS_BLOWS: [Blow; 8] = [
    blow("flurry", (20, 4, 12), (0.1, 1.5), (0.6, 1.9), 11.0, Physical, true, 16.0, Motion::Strike).art("strike").then(1, 4, 6),
    blow("flurry again", (14, 4, 14), (0.1, 1.5), (0.6, 1.9), 11.0, Physical, true, 16.0, Motion::Strike).art("strike").then(2, 4, 4),
    blow("open palm", (14, 5, 30), (0.1, 1.7), (0.0, 2.0), 22.0, GuardBreak, false, 34.0, Motion::Strike).art("strike"),
    blow("held palm", (52, 5, 32), (0.1, 1.7), (0.0, 2.0), 24.0, Physical, true, 28.0, Motion::Strike).art("strike").tell(0.36),
    blow("hare's leap", (34, 8, 26), (-1.0, 1.0), (0.0, 1.8), 26.0, GuardBreak, false, 36.0, Motion::Leap).art("dodge").tell(0.5).stuck(50),
    blow("gas along the floor", (40, 60, 30), (0.0, 0.0), (0.0, 0.55), 16.0, Unblockable, false, 0.0, Motion::Ring).art("dodge"),
    blow("answer", (12, 4, 22), (0.1, 1.7), (0.0, 2.2), 16.0, GuardBreak, false, 24.0, Motion::Strike).art("strike"),
    blow("radiant hand", (22, 5, 12), (0.1, 1.7), (0.0, 2.0), 20.0, Physical, true, 26.0, Motion::Strike).art("strike").then(0, 4, 10),
];
pub static EXODUS: Breed = Breed { half: 0.45, height: 1.8, walk: 1.7, keep: 1.4, open: 96, reply: Some(6),
    ..boss("Exodus Lightfoot", 1700.0, 190.0, 30, 2.6, &EXODUS_BLOWS,
        |near, r, wild| match (near, r, wild) {
            (true, 0..=3, _) => 0, (true, 4..=5, _) => 3, (true, 6..=7, _) => 2, (true, _, true) => 7, (true, _, _) => 0,
            (false, 0..=5, _) => 4, (false, _, _) => 5,
        }, 1.9, Some(7), ("exodus_side", 2.2)) };

/// Voryn's Vessel: Exodus swollen and hollowed, his strings grown into claws (parry the third
/// late), a palm that breaks a guard and the same palm held, a leap across the chamber, the Taint
/// rolled out along the floor (jump it), the Eye's gaze at chest height (roll under it), a grasp
/// for whoever turtles, and past half its health Voryn wakes in it, straight into the claws.
const VESSEL_BLOWS: [Blow; 10] = [
    blow("rending claws", (28, 6, 18), (0.2, 2.4), (0.0, 2.6), 18.0, Physical, true, 26.0, Motion::Strike).art("sweep").then(1, 8, 5),
    blow("claws again", (18, 6, 22), (0.2, 2.4), (0.0, 2.6), 20.0, Physical, true, 26.0, Motion::Strike).art("sweep").then(2, 8, 4),
    blow("hollow palm", (24, 6, 34), (0.2, 2.6), (0.0, 2.8), 30.0, GuardBreak, false, 44.0, Motion::Strike).art("toll"),
    blow("held palm", (60, 6, 36), (0.2, 2.6), (0.0, 2.8), 32.0, Physical, true, 34.0, Motion::Strike).art("toll").tell(0.36),
    blow("the hunger's leap", (54, 8, 30), (-1.6, 1.6), (0.0, 3.0), 36.0, GuardBreak, false, 50.0, Motion::Leap).art("leap").tell(0.55).stuck(60),
    blow("taint tide", (46, 70, 36), (0.0, 0.0), (0.0, 0.6), 20.0, Unblockable, false, 0.0, Motion::Ring).art("knell"),
    blow("the eye's gaze", (40, 90, 30), (0.0, 0.0), (0.7, 2.0), 22.0, Physical, false, 30.0, Motion::Shot(9.0 / TPS)).art("throw"),
    blow("grasp", (30, 6, 46), (0.2, 2.2), (0.0, 2.8), 40.0, Unblockable, false, 0.0, Motion::Strike).art("grab").stuck(30),
    blow("answer", (14, 4, 24), (0.2, 2.4), (0.0, 2.6), 18.0, GuardBreak, false, 26.0, Motion::Strike).art("sweep"),
    blow("voryn wakes", (24, 6, 12), (0.2, 2.6), (0.0, 2.8), 24.0, Physical, true, 30.0, Motion::Strike).art("toll").then(0, 6, 10),
];
pub static VESSEL: Breed = Breed { half: 0.9, height: 3.3, walk: 1.3, keep: 2.4, grab: Some(7), reply: Some(8),
    ..boss("Voryn's Vessel", 1900.0, 220.0, 34, 2.8, &VESSEL_BLOWS,
        |near, r, wild| match (near, r, wild) {
            (true, 0..=2, _) => 0, (true, 3..=4, _) => 3, (true, 5..=6, _) => 2, (true, 7, _) => 7, (true, _, true) => 9, (true, _, _) => 0,
            (false, 0..=5, _) => 6, (false, 6..=7, _) => 4, (false, _, _) => 5,
        }, 2.6, Some(9), ("voryn_vessel", 4.2)) };

/// Voryn Made Whole, in Unit 0 finished: blade limbs in strings (parry the third late), a blow
/// that splinters a guard and the same blow held, stars from the wrist slits high then low (roll,
/// then jump), the Eye opening along the floor, a carved lunge across the chamber, and past half
/// its health the whole of Voryn waking in it, straight into the blades.
const WHOLE_BLOWS: [Blow; 10] = [
    blow("blade arm", (30, 6, 18), (0.2, 2.6), (0.0, 2.8), 19.0, Physical, true, 26.0, Motion::Strike).art("sweep").then(1, 6, 5),
    blow("blade arm again", (20, 6, 22), (0.2, 2.6), (0.0, 2.8), 19.0, Physical, true, 26.0, Motion::Strike).art("sweep").then(2, 6, 4),
    blow("splintering blow", (24, 6, 34), (0.2, 2.6), (0.0, 2.8), 30.0, GuardBreak, false, 44.0, Motion::Strike).art("toll"),
    blow("wrist stars", (30, 80, 24), (0.0, 0.0), (0.7, 1.9), 14.0, Physical, false, 20.0, Motion::Shot(11.0 / TPS)).art("throw").then(4, 14, 4),
    blow("low stars", (26, 80, 26), (0.0, 0.0), (0.0, 0.55), 12.0, Physical, false, 20.0, Motion::Shot(11.0 / TPS)).art("throw"),
    blow("the eye opens", (48, 80, 36), (0.0, 0.0), (0.0, 0.6), 22.0, Unblockable, false, 0.0, Motion::Ring).art("knell"),
    blow("carved lunge", (32, 24, 30), (-0.5, 0.5), (0.0, 2.4), 26.0, GuardBreak, false, 36.0, Motion::Charge(9.0 / TPS)).art("leap").stuck(40),
    blow("held blade", (62, 6, 36), (0.2, 2.6), (0.0, 2.8), 32.0, Physical, true, 34.0, Motion::Strike).art("toll").tell(0.36),
    blow("answer", (14, 4, 24), (0.2, 2.6), (0.0, 2.8), 18.0, GuardBreak, false, 26.0, Motion::Strike).art("sweep"),
    blow("made whole", (28, 6, 14), (0.2, 2.6), (0.0, 2.8), 22.0, Physical, true, 30.0, Motion::Strike).art("toll").then(0, 8, 7),
];
pub static WHOLE: Breed = Breed { half: 0.8, height: 3.4, walk: 1.4, keep: 2.4, reply: Some(8),
    ..boss("Voryn Made Whole", 1900.0, 230.0, 34, 2.8, &WHOLE_BLOWS,
        |near, r, wild| match (near, r, wild) {
            (true, 0..=2, _) => 0, (true, 3..=4, _) => 7, (true, 5..=6, _) => 2, (true, 7, _) => 3, (true, _, true) => 9, (true, _, _) => 0,
            (false, 0..=4, _) => 3, (false, 5..=7, _) => 6, (false, _, _) => 5,
        }, 2.6, Some(9), ("voryn_whole", 4.4)) };

pub static CRIMSON_CAPTAIN: Boss = Boss { breed: &CAPTAIN, souls: 4800, key: None, mini: true, remnant: None, sought: true };
/// The chamber's boss. Felled, he is not beaten: what falls is only the first of him, and what
/// the chamber pays (echoes and the lens) it pays when the Vessel falls.
pub static EXODUS_LIGHTFOOT: Boss = Boss { breed: &EXODUS, souls: 16000, key: None, mini: false, remnant: Some("Exodus's Red Lens"), sought: true };
pub static VORYN_VESSEL: Boss = Boss { breed: &VESSEL, souls: 0, key: None, mini: false, remnant: None, sought: true };
pub static VORYN_WHOLE: Boss = Boss { breed: &WHOLE, souls: 0, key: None, mini: false, remnant: None, sought: true };

impl Boss {
    /// Whether it is what Voryn rises in once Exodus falls (it has an entrance of its own).
    pub fn is_risen(&self) -> bool { std::ptr::eq(self, &VORYN_VESSEL) || std::ptr::eq(self, &VORYN_WHOLE) }
}
