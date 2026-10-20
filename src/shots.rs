//! Frames of the real game, drawn headless through Quartz's renderer while simulated keys play
//! it: `cargo test --release shoot -- --ignored --nocapture` writes target/shots/*.png.

use prism::canvas::HeadlessRenderer;
use prism::drawable::Drawable;
use prism::event::{Event, KeyboardEvent, KeyboardState, Modifiers, MouseEvent, MouseState, OnEvent, TickEvent};
use quartz::*;
use ramp::prism;

/// The window drawn into: a laptop window, neither the canvas's own 3840 x 2160 nor 16:9, so a
/// picture or a size left in virtual units shows (pictures a plugin set used to come out zoomed
/// by the window's scale, and only a 3840 x 2160 capture looked right). `HR_SCREEN=WxH` picks
/// another.
fn screen() -> (f32, f32) {
    std::env::var("HR_SCREEN").ok()
        .and_then(|s| { let (w, h) = s.split_once('x')?; Some((w.parse().ok()?, h.parse().ok()?)) })
        .unwrap_or((1728.0, 1117.0))
}

struct Play { ctx: Context, canvas: Canvas, renderer: Option<HeadlessRenderer>, dir: std::path::PathBuf }

impl Play {
    fn new() -> Play {
        let (mut ctx, _r) = Context::new();
        let mut canvas = Canvas::new(&mut ctx, CanvasMode::Landscape);
        crate::game::setup(&mut canvas);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/shots");
        let _ = std::fs::create_dir_all(&dir);
        Play { ctx, canvas, renderer: pollster::block_on(HeadlessRenderer::new(screen().0 as u32, screen().1 as u32)).ok(), dir }
    }

    /// Past the title: the Wayfarer chosen, awake at the first shrine and on the road.
    fn started() -> Play {
        let mut p = Play::new();
        p.ticks(5);
        p.tap("e");
        p.tap("e");
        assert!(p.until_playing(600), "never reached the road");
        p
    }

    fn key(&mut self, ch: &str, state: KeyboardState) { self.key_as(Key::Character(ch.into()), state); }
    fn key_as(&mut self, key: Key, state: KeyboardState) {
        let req = self.canvas.request_size();
        let sized = self.canvas.build(screen(), req);
        let evt = KeyboardEvent { key, state, modifiers: Modifiers::none() };
        OnEvent::on_event(&mut self.canvas, &mut self.ctx, &sized, Box::new(evt) as Box<dyn Event>);
    }
    fn mouse(&mut self, button: prism::event::MouseButton, state: MouseState) {
        let req = self.canvas.request_size();
        let sized = self.canvas.build(screen(), req);
        let evt = MouseEvent { position: Some((screen().0 * 0.5, screen().1 * 0.5)), state, button };
        OnEvent::on_event(&mut self.canvas, &mut self.ctx, &sized, Box::new(evt) as Box<dyn Event>);
    }
    fn click(&mut self, button: prism::event::MouseButton, shift: bool) {
        if shift { self.key_as(Key::Named(NamedKey::Shift), KeyboardState::Pressed); }
        self.mouse(button, MouseState::Pressed);
        self.mouse(button, MouseState::Released);
        if shift { self.key_as(Key::Named(NamedKey::Shift), KeyboardState::Released); }
    }

    /// `n` ticks, each given a few milliseconds so the road's worker keeps up as in a window.
    fn ticks(&mut self, n: u32) {
        for _ in 0..n {
            let req = self.canvas.request_size();
            let sized = self.canvas.build(screen(), req);
            OnEvent::on_event(&mut self.canvas, &mut self.ctx, &sized, Box::new(TickEvent) as Box<dyn Event>);
            std::thread::sleep(std::time::Duration::from_millis(6));
        }
    }

    fn shot(&mut self, name: &str) {
        let req = self.canvas.request_size();
        let sized = self.canvas.build(screen(), req);
        let items = self.canvas.draw(&sized, (0.0, 0.0), (0.0, 0.0, screen().0, screen().1));
        let Some(r) = self.renderer.as_mut() else { eprintln!("no GPU: {name} not drawn"); return };
        let (w, h, px) = r.render(items);
        let path = self.dir.join(format!("{name}.png"));
        let img = image::RgbaImage::from_raw(w, h, px).unwrap();
        // Big captures are saved at half size.
        let k = if w > 2000 { 2 } else { 1 };
        image::imageops::resize(&img, w / k, h / k, image::imageops::FilterType::Triangle).save(&path).unwrap();
        eprintln!("[shot] {}", path.display());
    }
}

impl Play {
    fn fight(&self) -> &crate::fight::Fight { &self.canvas.get_plugin::<crate::game::Pilgrimage>().unwrap().fight }
    fn tap(&mut self, ch: &str) { self.key(ch, KeyboardState::Pressed); self.key(ch, KeyboardState::Released); }
    /// Tick until `pred` holds (at most `n` ticks); whether it did.
    fn until(&mut self, n: u32, pred: impl Fn(&crate::fight::Fight) -> bool) -> bool {
        for _ in 0..n { if pred(self.fight()) { return true; } self.ticks(1); }
        pred(self.fight())
    }
    /// Tick until the first husk winds up `name`, guarding through its other attack.
    fn until_attack(&mut self, name: &str) -> bool {
        self.mouse(prism::event::MouseButton::Right, MouseState::Pressed);
        let found = self.until_attack_unguarded(name);
        self.mouse(prism::event::MouseButton::Right, MouseState::Released);
        found
    }
    fn until_attack_unguarded(&mut self, name: &str) -> bool {
        for _ in 0..8 {
            if !self.until(600, |f| f.enemies[0].attack().is_some()) { return false; }
            if self.fight().enemies[0].attack().unwrap().0.name == name { return true; }
            self.until(300, |f| f.enemies[0].attack().is_none());
        }
        false
    }
    fn windup_t(&self) -> u32 { match self.fight().enemies[0].ai { crate::fight::Ai::Attack { t, .. } => t, _ => 0 } }
}

#[test]
#[ignore]
fn shoot() {
    use crate::fight::Ai;
    let mut p = Play::started();
    p.ticks(40);
    p.shot("01_start");
    // Walk up to the first husk and swing at it as soon as it is in reach, rested and fresh:
    // the light's swipe, a few ticks into its live frames.
    p.key("w", KeyboardState::Pressed);
    for _ in 0..600 {
        let walked = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
        if p.fight().enemies[0].gap(walked) <= crate::fight::ENGAGE + 0.05 { break; }
        p.ticks(1);
    }
    p.click(prism::event::MouseButton::Left, false);
    p.ticks(20);
    p.shot("02a_light_swing");
    // It closes in and winds up a chop, its warning on the road.
    assert!(p.until_attack("chop"), "no chop");
    p.until(60, |f| f.enemies[0].attack().is_some_and(|a| a.2 > 0.6));
    p.shot("02_chop_warning");
    // Roll right just before it falls: a perfect dodge, in slow motion, and the husk stumbles.
    p.until(60, |f| f.enemies[0].attack().is_some_and(|a| a.2 > 0.86));
    eprintln!("[fight] rolling at {:?}", p.fight().enemies[0].ai);
    p.tap("d");
    // The roll through, a frame every few ticks (slow motion once the chop misses).
    for (i, n) in [2, 3, 3, 3, 3, 4, 4].iter().enumerate() {
        p.ticks(*n);
        p.shot(&format!("03a_roll_{i}"));
    }
    p.ticks(4);
    p.shot("03_just_dodge");
    eprintln!("[fight] after the roll: hp {} lane {} husk {:?}", p.fight().hero.hp, p.fight().hero.lane, p.fight().enemies[0].ai);
    assert!(matches!(p.fight().enemies[0].ai, Ai::Stumbled(_)), "the husk stumbles");
    // Back into its lane, then parry the next chop and riposte.
    p.until(200, |f| f.enemies[0].attack().is_none());
    p.tap("a");
    p.ticks(40);
    use prism::event::MouseButton as B;
    assert!(p.until_attack("chop"), "no second chop");
    let (w, _, _) = p.fight().enemies[0].attack().unwrap().0.frames;
    let (pw, pa, _) = crate::fight::Move::Parry.spec().frames;
    let press = w - 1 - pw - pa / 2;
    while p.windup_t() < press { p.ticks(1); }
    p.click(B::Right, true);
    let hp = p.fight().hero.hp;
    p.until(40, move |f| matches!(f.enemies[0].ai, Ai::Staggered(_)) || f.hero.hp < hp);
    p.ticks(3);
    p.shot("04_parried");
    eprintln!("[fight] parry: {:?}, hp {}", p.fight().enemies[0].ai, p.fight().hero.hp);
    p.click(B::Left, false);
    p.until(80, |f| f.enemies[0].hurt > 8);
    p.shot("05_riposte");
    eprintln!("[fight] riposte: husk hp {}", p.fight().enemies[0].hp);
    // Guard the next attack.
    p.mouse(B::Right, MouseState::Pressed);
    p.until(600, |f| f.enemies[0].attack().is_some_and(|a| a.1 != crate::fight::Phase::Windup) || !f.enemies[0].alive());
    p.ticks(3);
    p.shot("06_guard");
    p.mouse(B::Right, MouseState::Released);
    p.key("w", KeyboardState::Released);
    for (t, e) in &p.canvas.get_plugin::<crate::game::Pilgrimage>().unwrap().log { eprintln!("[log] {t} {e:?}"); }
    eprintln!("[fight] end: hero hp {:.1} stamina {:.1}, husk {:?}", p.fight().hero.hp, p.fight().hero.stamina, p.fight().enemies[0].ai);
}

impl Play {
    fn mode(&self) -> crate::game::Mode { self.canvas.get_plugin::<crate::game::Pilgrimage>().unwrap().mode.clone() }
    fn until_mode(&mut self, n: u32, pred: impl Fn(&crate::game::Mode) -> bool) -> bool {
        for _ in 0..n { if pred(&self.mode()) { return true; } self.ticks(1); }
        pred(&self.mode())
    }
    fn until_playing(&mut self, n: u32) -> bool {
        for _ in 0..n { if self.canvas.get_plugin::<crate::game::Pilgrimage>().unwrap().playing() { return true; } self.ticks(1); }
        false
    }
    fn walk_state(&self) -> quartz_path_forge::WalkState { self.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().state().clone() }
    fn game(&mut self) -> &mut crate::game::Pilgrimage { self.canvas.get_plugin_mut::<crate::game::Pilgrimage>().unwrap() }
    /// Every enemy of the place down, as if fought (their souls taken).
    fn fell_all(&mut self) {
        let g = self.game();
        let kinds: Vec<_> = g.fight.enemies.iter().filter(|e| e.alive()).map(|e| e.kind).collect();
        let _ = g.world.felled(kinds);
        for e in g.fight.enemies.iter_mut() { e.ai = crate::fight::Ai::Dead; }
        // An ambush's foes too (the boss is fought in its own capture).
        if let crate::game::Mode::Side(a) = &mut g.mode {
            if a.why != crate::game::Why::Boss && a.t >= 140 {
                for f in &mut a.side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
            }
        }
    }
    /// Tick (clearing every place on the way when asked) until `pred` holds of the walk.
    fn walk_until(&mut self, n: u32, clear: bool, pred: impl Fn(&quartz_path_forge::WalkState) -> bool) -> bool {
        for _ in 0..n {
            if pred(&self.walk_state()) { return true; }
            if clear { self.fell_all(); }
            self.ticks(1);
        }
        pred(&self.walk_state())
    }
}

/// The journey: rest at the road's shrine, walk to the fork, step left into the marsh, on to the
/// gate, the road again from its end, right into the chapel, and a death that wakes at the shrine.
/// `cargo test --release journey -- --ignored --nocapture` writes target/shots/j*.png.
/// The game around the fight: the title, the callings, waking, resting in the sanctum (the
/// keeper's first words, levels, the shrine), the smith met on the road and at the next rest, and
/// a death. `cargo test --release flow -- --ignored --nocapture` writes target/shots/f*.png.
#[test]
#[ignore]
fn flow() {
    use crate::game::Mode;
    let mut p = Play::new();
    p.ticks(40);
    p.shot("f01_title");
    p.tap("e");
    p.ticks(2);
    p.tap("s");
    p.ticks(2);
    p.shot("f02_callings_warden");
    p.tap("w");
    p.tap("e");
    assert!(p.until_mode(200, |m| matches!(m, Mode::Wake(_))));
    p.ticks(30);
    p.shot("f03_waking");
    assert!(p.until_playing(300));
    p.game().world.souls = 400;
    // Rest at the first shrine.
    p.tap("e");
    p.ticks(30);
    p.shot("f04_kneeling");
    assert!(p.until_mode(400, |m| matches!(m, Mode::Sanctum(_))), "never sat down");
    p.ticks(40);
    p.shot("f05_sanctum");
    // To the keeper: her first words, then her choices, then levels.
    p.tap("a");
    p.ticks(40);
    p.tap("e");
    p.ticks(2);
    p.shot("f06_keeper_first_words");
    for _ in 0..3 { p.tap("e"); }
    p.ticks(2);
    p.shot("f07_keeper_choices");
    p.tap("e");
    p.ticks(2);
    p.tap("s");
    p.tap("e");
    p.ticks(2);
    p.shot("f08_levels");
    assert_eq!(p.game().fight.hero.stats.endurance, 1, "bought a level");
    // Back out to the shrine and rise.
    p.tap("q");
    p.tap("q");
    p.tap("d");
    p.ticks(40);
    p.tap("e");
    p.ticks(2);
    p.shot("f09_shrine_choices");
    // Equipment: what a shieldbearer left, offered for the offhand.
    let _ = p.canvas.get_plugin_mut::<crate::game::Pilgrimage>().unwrap().world.felled([crate::fight::Archetype::Shieldbearer, crate::fight::Archetype::Wretch]);
    p.tap("s");
    p.tap("e");
    p.ticks(2);
    p.shot("f09b_equipment");
    p.tap("s");
    p.tap("e");
    p.ticks(2);
    p.shot("f09c_offhand");
    p.tap("q");
    p.tap("q");
    p.tap("w");
    p.ticks(2);
    p.tap("e");
    assert!(p.until_playing(600), "never rose");
    p.ticks(2);
    p.shot("f10_risen");
    // On to the smith by the road, every enemy on the way down.
    p.key("w", KeyboardState::Pressed);
    for _ in 0..4000 {
        let ahead = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
        if p.game().world.smith_at().is_some_and(|s| s - ahead < 5.5) { break; }
        p.fell_all();
        p.ticks(1);
    }
    p.key("w", KeyboardState::Released);
    p.ticks(30);
    p.shot("f11_smith_by_the_road");
    p.tap("e");
    p.ticks(2);
    p.shot("f12_smith_first_words");
    for _ in 0..3 { p.tap("e"); }
    p.ticks(2);
    assert!(p.game().world.met_smith, "met him");
    // A death: the fall, the words, the dark, and up again at the shrine.
    { let h = &mut p.game().fight.hero; h.hp = 0.0; h.dead = true; }
    assert!(p.until_mode(200, |m| matches!(m, Mode::Dying(t) if *t > 80)));
    p.shot("f13_you_died");
    assert!(p.until_mode(800, |m| matches!(m, Mode::Wake(t) if *t > 30)), "never woke");
    p.shot("f14_waking_after_death");
}

/// The road's render time through the fork and the marsh with the game ticking flat out (no
/// sleep between ticks, as a window runs it): `HR_SCREEN=2940x1846 cargo test --release perf --
/// --ignored --nocapture` prints a line a second.
#[test]
#[ignore]
fn perf() {
    let mut p = Play::started();
    p.key_as(Key::Named(NamedKey::Shift), KeyboardState::Pressed);
    p.key("w", KeyboardState::Pressed);
    let t0 = std::time::Instant::now();
    let mut last = 0.0;
    let mut ms: Vec<f32> = Vec::new();
    while t0.elapsed().as_secs_f32() < 240.0 {
        let st = p.walk_state();
        if st.stop.as_deref() == Some("gate") { break; }
        // Lean left as soon as the fork is in view.
        if st.choose_within.is_some() && p.fight().hero.lane == 0 { p.tap("a"); }
        p.fell_all();
        let tick = std::time::Instant::now();
        let req = p.canvas.request_size();
        let sized = p.canvas.build(screen(), req);
        OnEvent::on_event(&mut p.canvas, &mut p.ctx, &sized, Box::new(TickEvent) as Box<dyn Event>);
        // Draw as a window would, every tick, at 60 a second.
        let items = p.canvas.draw(&sized, (0.0, 0.0), (0.0, 0.0, screen().0, screen().1));
        if let Some(r) = p.renderer.as_mut() { let _ = r.render(items); }
        if let Some(rest) = std::time::Duration::from_micros(16_667).checked_sub(tick.elapsed()) { std::thread::sleep(rest); }
        let pf = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap();
        ms.push(pf.render_ms());
        let now = t0.elapsed().as_secs_f32();
        if now - last >= 1.0 {
            last = now;
            ms.sort_by(f32::total_cmp);
            eprintln!("[perf] {:5.1}s  {:<12} progress {:?}  road ms median {:6.1} max {:6.1}", now, st.stop.as_deref().unwrap_or("-"), st.progress.map(|p| (p * 100.0).round()), ms[ms.len() / 2], ms[ms.len() - 1]);
            ms.clear();
        }
    }
}

/// Every way a blow can meet the hero, each read at a glance: staged one after another on the
/// hero (their reel set, the event shown as if the fight had made it) and shot a few ticks in.
#[test]
#[ignore]
fn reads() {
    use crate::fight::{Event, Reel};
    let mut p = Play::started();
    p.ticks(40);
    let cases: [(&str, Reel, Event, &[u32]); 9] = [
        ("r01_flinch", Reel::Flinch, Event::Struck { damage: 10.0, heavy: false }, &[5]),
        ("r02_recoil", Reel::Recoil, Event::Struck { damage: 28.0, heavy: true }, &[9]),
        ("r03_stagger", Reel::Stagger, Event::Staggered { damage: 24.0 }, &[8, 20]),
        ("r04_knockdown", Reel::Down, Event::KnockedDown { damage: 34.0 }, &[16, 30, 30, 18]),
        ("r05_blocked", Reel::BlockJolt, Event::Blocked { chip: 0.0 }, &[4]),
        ("r06_chip_blocked", Reel::ChipJolt, Event::Blocked { chip: 3.0 }, &[6]),
        ("r07_guard_broken", Reel::GuardBreak, Event::GuardBroken { damage: 17.0 }, &[10, 16]),
        ("r08_parried", Reel::None, Event::Parried { enemy: 0 }, &[4]),
        ("r09_missed_parry", Reel::Fumble, Event::ParryMissed { damage: 14.0 }, &[5, 16]),
    ];
    for (name, reel, ev, waits) in cases {
        {
            let g = p.game();
            g.fight.hero.reel = reel;
            g.fight.hero.reeling = reel.ticks();
            g.staged.push(ev);
        }
        for (i, n) in waits.iter().enumerate() {
            p.ticks(*n);
            p.shot(&if waits.len() > 1 { format!("{name}_{i}") } else { name.to_owned() });
        }
        // Let it play out before the next.
        p.ticks(reel.ticks() + 30);
    }
}

#[test]
#[ignore]
fn journey() {
    let mut p = Play::started();
    p.ticks(40);
    p.shot("j01_road_shrine");
    assert_eq!(p.walk_state().stop.as_deref(), Some("road"));
    // Rest: kneel, the dark, the sanctum; then up at the shrine and on.
    p.tap("e");
    assert!(p.until_mode(400, |m| matches!(m, crate::game::Mode::Sanctum(_))), "never sat down");
    p.ticks(40);
    p.shot("j02_resting");
    p.tap("e");
    p.tap("e");
    assert!(p.until_playing(600), "never rose");
    // Run on, every place cleared, to the fork.
    p.key_as(Key::Named(NamedKey::Shift), KeyboardState::Pressed);
    p.key("w", KeyboardState::Pressed);
    assert!(p.walk_until(4000, true, |s| s.choose_within.is_some()), "no fork");
    p.ticks(30);
    p.shot("j03_fork");
    // Lean left; the branch is taken at the last moment, the camera turning at the junction.
    p.tap("a");
    p.ticks(10);
    p.shot("j03b_lean_left");
    assert!(p.walk_until(4000, false, |s| s.progress.is_some_and(|t| t > 0.82)), "never reached the junction");
    p.shot("j03c_junction");
    assert!(p.walk_until(4000, true, |s| s.stop.as_deref() == Some("vale_edge")), "never reached the marsh edge");
    p.ticks(60);
    p.shot("j03d_marsh_edge");
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("vale")), "never reached the vale");
    p.ticks(90);
    p.shot("j04_vale");
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("gate")), "never reached the gate");
    // The Bell Warden has its own capture (`boss`): here the gate is walked through.
    p.game().world.boss_beaten = true;
    p.ticks(60);
    p.shot("j05_gate");
    // The journey's end: the road again, then the other branch.
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("road")), "the road did not begin again");
    let w = &p.game().world;
    eprintln!("[journey] pilgrimages {}, souls {}", w.pilgrimages, w.souls);
    assert!(p.walk_until(4000, true, |s| s.choose_within.is_some()), "no second fork");
    // In the middle lane the walk waits at the signpost.
    assert!(p.walk_until(4000, false, |s| s.choose_within.is_some_and(|l| l < 1.6)), "never reached the last moment");
    p.ticks(120);
    assert!(p.walk_state().choose_within.is_some(), "the junction chose for the player");
    p.shot("j05b_waiting_at_fork");
    p.tap("d");
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("chapel")), "never reached the chapel");
    p.ticks(90);
    p.shot("j06_chapel");
    // Fall here: wake at the road's shrine, the souls left in the chapel.
    let souls = p.game().world.souls;
    { let h = &mut p.game().fight.hero; h.hp = 0.0; h.dead = true; }
    assert!(p.walk_until(1200, false, |s| s.stop.as_deref() == Some("road")), "did not wake at the shrine");
    p.ticks(40);
    p.shot("j07_woke");
    let w = &p.game().world;
    assert_eq!((w.souls, w.stain.as_ref().map(|s| (s.stop.clone(), s.souls))), (0, Some(("chapel".into(), souls))));
    assert!(!p.fight().hero.dead && p.fight().enemies.iter().all(|e| e.alive()));
    p.key("w", KeyboardState::Released);
}

/// The boss at the gate, side-on: the camera's swing round, each blow's warning and how it is
/// met (a jump over the sweep and the ring, a roll through the charge, a parry of the toll), the
/// fall, and the road again.
#[test]
#[ignore]
fn boss() {
    use crate::game::Mode;
    use crate::side::Ai;
    let mut p = Play::started();
    p.game().world.warping = Some("gate".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:gate".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("gate")), "never reached the gate");
    p.ticks(10);
    p.key("w", KeyboardState::Pressed);
    let mut met = false;
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        if matches!(p.mode(), Mode::Side(_)) { met = true; break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(met, "the boss never stepped out");
    p.ticks(40);
    p.shot("b01_swing");
    assert!(p.until_mode(400, |m| matches!(m, Mode::Side(a) if a.t >= 140)));
    p.ticks(20);
    {
        let pf = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap();
        let d = 0.0;
        eprintln!("[frame] size {:?}", pf.frame_size());
        for (name, x, y, dd) in [("left end", 0.0, 0.0, d - 6.0), ("right end", 0.0, 0.0, d + 6.0), ("middle", 0.0, 0.0, d), ("head", 0.0, 1.85, d), ("near edge", 2.4, 0.0, d), ("far edge", -2.4, 0.0, d), ("boss top", 0.0, 3.4, d)] {
            eprintln!("[frame] {name}: {:?}", pf.project(x, y, dd));
        }
    }
    p.shot("b02_arena");
    // Each blow from where it would be thrown, the hero in front of it.
    let throw = |p: &mut Play, i: usize, gap: f32| {
        let a = arena(p);
        a.side.foes[0].s = a.side.hero.s + gap;
        a.side.foes[0].facing = -1.0;
        a.side.hero.facing = 1.0;
        a.side.foes[0].ai = Ai::Blow { i, t: 0, hit: false, from: a.side.foes[0].s };
    };
    let windup = |p: &mut Play, i: usize| arena(p).side.foes[0].timed(i).frames.0;
    throw(&mut p, 1, 2.2);
    let w = windup(&mut p, 1);
    p.ticks(w * 8 / 10);
    p.shot("b03_sweep_warning");
    p.tap("w");
    p.ticks(12);
    p.shot("b04_jumped_the_sweep");
    let hp = arena(&mut p).side.hero.hp;
    p.ticks(40);
    assert_eq!(arena(&mut p).side.hero.hp, hp, "the sweep passed under");
    throw(&mut p, 2, 3.0);
    let w = windup(&mut p, 2);
    p.ticks(w + 8);
    p.shot("b05_knell_ring");
    p.ticks(60);
    throw(&mut p, 3, 4.5);
    let w = windup(&mut p, 3);
    p.ticks(w + 4);
    p.shot("b06_charge");
    p.ticks(60);
    {
        let a = arena(&mut p);
        a.side.hero.hp = a.side.hero.stats.max_hp();
        (a.side.hero.reel, a.side.hero.reeling, a.side.hero.act) = (crate::fight::Reel::None, 0, None);
        a.side.hero.s = a.side.hero.s.clamp(-4.0, 3.0);
        a.side.foes[0].s = a.side.hero.s + 1.6;
        a.side.foes[0].ai = Ai::Open(150);
    }
    p.ticks(10);
    p.shot("b07_open");
    arena(&mut p).side.foes[0].hp = 5.0;
    p.click(prism::event::MouseButton::Left, false);
    assert!(p.until_mode(120, |m| matches!(m, Mode::Side(a) if a.side.won())), "the last blow did not fell it");
    p.ticks(50);
    p.shot("b08_felled");
    let souls = p.game().world.souls;
    assert!(p.until_playing(400), "never back on the road");
    let g = p.game();
    assert!(g.world.boss_beaten && g.world.souls > souls && g.world.gear.owns(crate::gear::find("Warden's Bell")));
    p.ticks(160);
    p.shot("b09_road_again");
}

/// A side path off the chapel road: its mouth, the camera turning to look down it, the hollow at
/// its end and what was left there, and the road again where it was left.
/// `cargo test --release side_path -- --ignored --nocapture` writes target/shots/s*.png.
#[test]
#[ignore]
fn side_path() {
    use crate::game::Mode;
    let mut p = Play::started();
    p.game().world.warping = Some("chapel".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:chapel".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("chapel")), "never reached the chapel");
    p.ticks(10);
    // Walk until a side path opens at the hero.
    p.key("w", KeyboardState::Pressed);
    let mut at = None;
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        let st = p.walk_state();
        if let Some(side) = st.side_paths.iter().find(|(d, _)| (d - crate::fight::HERO_D).abs() < crate::world::SIDE_REACH * 0.5).map(|&(_, s)| s) { at = Some(side); break; }
    }
    p.key("w", KeyboardState::Released);
    let side = at.expect("no side path opened");
    eprintln!("[side] mouth on side {side}, side paths {:?}", p.walk_state().side_paths);
    p.ticks(4);
    p.shot("s01_mouth");
    p.tap("e");
    assert!(p.until_mode(20, |m| matches!(m, Mode::Peek(..))), "E did not turn down it: {:?}", p.mode());
    p.ticks(40);
    p.shot("s02_looking_down_it");
    assert!(p.walk_until(1200, false, |s| s.stop.as_deref() == Some("hollow")), "never reached the hollow");
    assert!(p.until_playing(400));
    p.ticks(30);
    p.shot("s03_hollow");
    let road_dead = p.game().world.detour.as_ref().map(|d| d.foes.iter().filter(|e| !e.alive()).count());
    // Clear it and walk to its end, where something was left.
    // Its chest is a trap: walking up to it springs an ambush, and only then does it open.
    let owned = p.game().world.gear.owned.len();
    p.key("w", KeyboardState::Pressed);
    let mut trap = false;
    for _ in 0..6000 {
        if !trap && matches!(p.mode(), crate::game::Mode::Side(_)) {
            trap = true;
            p.key("w", KeyboardState::Released);
            assert!(!p.game().world.cache_taken, "not before the trap is beaten");
            p.until_mode(300, |m| matches!(m, crate::game::Mode::Side(a) if a.t >= 170));
            p.shot("s04a_chest_trap");
            p.key("w", KeyboardState::Pressed);
        }
        p.fell_all();
        p.ticks(1);
        if p.game().world.cache_taken { break; }
    }
    assert!(trap, "the chest never sprang");
    assert!(p.game().world.cache_taken, "the cache was never reached");
    assert!(p.until_playing(400));
    p.ticks(20);
    p.shot("s04_found");
    let g = p.game();
    assert!(g.world.gear.owned.len() > owned, "a piece was left there");
    eprintln!("[side] found {}", crate::gear::ITEMS[*g.world.gear.owned.last().unwrap()].name);
    let mut back = false;
    for _ in 0..1200 {
        let ahead = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
        let g = p.game();
        if g.world.can_come_back(&g.fight, ahead) { back = true; break; }
        p.ticks(1);
    }
    assert!(back, "never at the hollow's end");
    p.key("w", KeyboardState::Released);
    p.ticks(4);
    p.shot("s05_turn_back");
    p.tap("e");
    assert!(p.walk_until(1200, false, |s| s.stop.as_deref() == Some("chapel")), "never back on the chapel road");
    assert!(p.until_playing(400));
    p.ticks(40);
    p.shot("s06_road_again");
    let w = &p.game().world;
    assert!(w.detour.is_none() && w.explored.contains(&"hollow"));
    let dead = p.fight().enemies.iter().filter(|e| !e.alive()).count();
    assert_eq!(Some(dead), road_dead, "the road's dead stayed down");
}

/// An ambush in the Wretched Vale: the spring, the yard with a pack on both sides, a warning, a
/// foe felled, the last down, and the road again.
/// `cargo test --release ambush -- --ignored --nocapture` writes target/shots/a*.png.
#[test]
#[ignore]
fn ambush() {
    use crate::game::{Mode, Why};
    let mut p = Play::started();
    p.game().world.warping = Some("vale".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:vale".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("vale")), "never reached the vale");
    p.ticks(10);
    p.key("w", KeyboardState::Pressed);
    let mut sprung = false;
    for _ in 0..8000 {
        // Clear the road's own enemies, not the ambush.
        let g = p.game();
        for e in g.fight.enemies.iter_mut() { e.ai = crate::fight::Ai::Dead; }
        p.ticks(1);
        if matches!(p.mode(), Mode::Side(_)) { sprung = true; break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(sprung, "the ambush never sprang");
    assert!(matches!(arena(&mut p).why, Why::Ambush(0)));
    p.ticks(20);
    p.shot("a01_sprung");
    assert!(p.until_mode(400, |m| matches!(m, Mode::Side(a) if a.t >= 140)));
    p.ticks(30);
    p.shot("a02_yard");
    eprintln!("[ambush] foes {:?}", arena(&mut p).side.foes.iter().map(|f| (f.breed.name, f.s)).collect::<Vec<_>>());
    // Wait for a warning on the ground.
    for _ in 0..600 {
        if arena(&mut p).side.foes.iter().any(|f| f.warning().is_some_and(|w| w.4 > 0.6)) { break; }
        p.ticks(1);
    }
    p.shot("a03_warning");
    // One down, then the rest.
    { let a = arena(&mut p); a.side.hero.hp = a.side.hero.stats.max_hp(); let f = &mut a.side.foes[0]; (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    p.ticks(40);
    p.shot("a04_one_down");
    let souls = p.game().world.souls;
    for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    p.ticks(50);
    p.shot("a05_cleared");
    assert!(p.until_playing(400), "never back on the road");
    let w = &p.game().world;
    assert!(w.cleared.contains(&("vale", 0)) && w.souls > souls, "remembered, and paid: {:?} {}", w.cleared, w.souls);
    p.ticks(160);
    p.shot("a06_road_again");
    // Beaten stays beaten.
    p.key("w", KeyboardState::Pressed);
    p.ticks(120);
    p.key("w", KeyboardState::Released);
    assert!(matches!(p.mode(), Mode::Play));
}

/// The boss fight under way in the capture.
fn arena(p: &mut Play) -> &mut crate::game::Arena {
    match &mut p.game().mode { crate::game::Mode::Side(a) => a, _ => panic!("not in the boss fight") }
}
