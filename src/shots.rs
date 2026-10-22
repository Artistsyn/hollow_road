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
    /// A game just launched, with no pilgrimage kept (the test slots cleared).
    fn new() -> Play {
        let _ = std::fs::remove_dir_all(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test_saves"));
        Play::launched()
    }

    /// A game just launched, with whatever the test slots hold.
    fn launched() -> Play {
        let (mut ctx, _r) = Context::new();
        let mut canvas = Canvas::new(&mut ctx, CanvasMode::Landscape);
        crate::game::setup(&mut canvas);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/shots");
        let _ = std::fs::create_dir_all(&dir);
        Play { ctx, canvas, renderer: pollster::block_on(HeadlessRenderer::new(screen().0 as u32, screen().1 as u32)).ok(), dir }
    }

    /// Past the title: the first slot, the Wayfarer chosen, awake at the first shrine and on the road.
    fn started() -> Play {
        let mut p = Play::new();
        p.ticks(5);
        p.tap("e");
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
    /// Where the walk and the game stand, for a failed capture's message.
    fn status(&mut self) -> String {
        let st = self.walk_state();
        let (stop, within, walked) = (st.stop.clone(), st.choose_within, st.walked);
        let g = self.game();
        format!("stop {stop:?} choose_within {within:?} walked {walked:.1} mode {:?} at {} warping {:?} sent {} lean {:?} chose {} lane {}",
            g.mode, g.world.at, g.world.warping, g.world.sent, g.world.lean, g.world.chose, g.fight.hero.lane)
    }

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
    p.shot("f01b_pilgrimages");
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
        if st.stop.as_deref() == Some("gate@vale") { break; }
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
    assert_eq!(p.walk_state().stop.as_deref(), Some(crate::world::START));
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
    assert!(p.walk_until(4000, true, |s| s.choose_within.is_some()), "no fork: {}", p.status());
    p.ticks(30);
    p.shot("j03_fork");
    // Lean left; the branch is taken at the last moment, the camera turning at the junction.
    p.tap("a");
    p.ticks(10);
    p.shot("j03b_lean_left");
    assert!(p.walk_until(4000, false, |s| s.progress.is_some_and(|t| t > 0.82)), "never reached the junction: {}", p.status());
    p.shot("j03c_junction");
    assert!(p.walk_until(4000, true, |s| s.stop.as_deref() == Some("vale_edge@road")), "never reached the marsh edge: {}", p.status());
    p.ticks(60);
    p.shot("j03d_marsh_edge");
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("vale@vale_edge")), "never reached the vale: {}", p.status());
    p.ticks(90);
    p.shot("j04_vale");
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("gate@vale")), "never reached the gate: {}", p.status());
    // The Bell Warden has its own capture (`boss`): here the hero turns round at the gate and
    // walks the way they came, every place laid out backwards, to the road's fork again.
    p.game().world.beaten.push("gate");
    p.ticks(60);
    p.shot("j05_gate");
    p.fell_all();
    p.key("w", KeyboardState::Released);
    p.ticks(20);
    p.tap("t");
    // The gate has three roads: turned round, it is walked in by another, and the way back to the
    // vale is one side of its fork.
    let back = crate::atlas::turned("gate", "vale");
    assert!(p.walk_until(1200, false, |s| s.stop.as_deref() == Some(back.as_str())), "never turned round: {:?}", p.walk_state().stop);
    p.key("w", KeyboardState::Pressed);
    p.ticks(60);
    p.shot("j05a_turned");
    let vale_left = crate::atlas::exits("gate", crate::atlas::came_from(&back).unwrap())[0] == "vale";
    assert!(p.walk_until(4000, true, |s| s.choose_within.is_some()), "no fork walking back: {}", p.status());
    p.tap(if vale_left { "a" } else { "d" });
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("vale@gate")), "never back in the vale: {}", p.status());
    p.ticks(90);
    p.shot("j05b_vale_backwards");
    assert!(p.walk_until(8000, true, |s| s.stop.as_deref() == Some("road@vale_edge")), "never back on the road: {}", p.status());
    let w = &p.game().world;
    eprintln!("[journey] souls {}, seen {:?}", w.souls, w.seen);
    assert!(p.walk_until(4000, true, |s| s.choose_within.is_some()), "no second fork: {}", p.status());
    // In the middle lane the walk waits at the signpost.
    assert!(p.walk_until(4000, false, |s| s.choose_within.is_some_and(|l| l < 1.6)), "never reached the last moment: {}", p.status());
    p.ticks(120);
    assert!(p.walk_state().choose_within.is_some(), "the junction chose for the player");
    p.shot("j05c_waiting_at_fork");
    // Walking back, the chapel lies to the left.
    p.tap("a");
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("chapel@chapel_edge")), "never reached the chapel: {}", p.status());
    p.ticks(90);
    p.shot("j06_chapel");
    // Fall here: wake at the road's shrine, the souls left in the chapel.
    let souls = p.game().world.souls;
    { let h = &mut p.game().fight.hero; h.hp = 0.0; h.dead = true; }
    assert!(p.walk_until(1200, false, |s| s.stop.as_deref() == Some(crate::world::START)), "did not wake at the shrine: {}", p.status());
    p.ticks(40);
    p.shot("j07_woke");
    let w = &p.game().world;
    assert_eq!((w.souls, w.stain.as_ref().map(|s| (s.stop.clone(), s.souls))), (0, Some(("chapel".into(), souls))));
    assert!(!p.fight().hero.dead && p.fight().enemies.iter().all(|e| e.alive()));
    p.key("w", KeyboardState::Released);
}

/// The bosses side-on, past the gate: the Bandit Chief in his hold, his hatchet in flight, his
/// grab, the Hollowmere Screamer's leap and scream from the same yard, then every other boss
/// and mini-boss in its own art.
#[test]
#[ignore]
fn boss_moves() {
    use crate::game::Mode;
    use crate::side::Ai;
    let mut p = Play::started();
    p.game().world.warping = Some("chief_hold@ridge_camp".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:chief_hold@ridge_camp".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("chief_hold@ridge_camp")), "never reached the hold: {}", p.status());
    p.ticks(10);
    p.key("w", KeyboardState::Pressed);
    let mut met = false;
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        if matches!(p.mode(), Mode::Side(_)) { met = true; break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(met, "the chief never stepped out: {}", p.status());
    assert!(p.until_mode(400, |m| matches!(m, Mode::Side(a) if a.t >= 140)));
    assert_eq!(arena(&mut p).side.foes[0].breed.name, "The Bandit Chief");
    p.ticks(10);
    p.shot("m01_chief");
    let throw = |p: &mut Play, i: usize, gap: f32| {
        let a = arena(p);
        a.side.hero.hp = a.side.hero.stats.max_hp();
        (a.side.hero.reel, a.side.hero.reeling, a.side.hero.act) = (crate::fight::Reel::None, 0, None);
        a.side.hero.s = -3.5;
        a.side.foes[0].s = a.side.hero.s + gap;
        a.side.foes[0].facing = -1.0;
        a.side.hero.facing = 1.0;
        a.side.foes[0].ai = Ai::Blow { i, t: 0, hit: false, from: a.side.foes[0].s, aim: a.side.hero.s };
    };
    let windup = |p: &mut Play, i: usize| arena(p).side.foes[0].timed(i).frames.0;
    // The hatchet: its path warned, then in flight.
    throw(&mut p, 3, 6.5);
    let w = windup(&mut p, 3);
    p.ticks(w * 8 / 10);
    p.shot("m02_hatchet_warning");
    p.ticks(w * 2 / 10 + 14);
    assert!(arena(&mut p).side.foes[0].shot().is_some(), "the hatchet is in flight");
    p.shot("m03_hatchet_in_flight");
    p.ticks(80);
    // The headsman's drop: it tells early and holds.
    throw(&mut p, 2, 2.0);
    let w = windup(&mut p, 2);
    p.ticks(w / 2);
    p.shot("m04_headsmans_drop_held");
    p.ticks(w / 2 + 60);
    // The Screamer, in the same yard: its leap's landing warned, then its scream.
    {
        let a = arena(&mut p);
        a.side.foes[0] = crate::side::Foe::new(&crate::side::SCREAMER, 2.0, -1.0, 400);
    }
    throw(&mut p, 4, 5.0);
    let w = windup(&mut p, 4);
    p.ticks(w * 7 / 10);
    p.shot("m05_leap_landing_warned");
    p.ticks(w * 3 / 10 + 4);
    p.shot("m06_leap_lands");
    p.ticks(100);
    throw(&mut p, 3, 3.0);
    let w = windup(&mut p, 3);
    p.ticks(w + 14);
    p.shot("m07_scream");
    p.ticks(100);
    // The other region bosses and the mini-bosses, each in its own art: standing, then near the
    // end of one of its own moves' windups.
    use crate::side::{AGENT, ALPHA, ARCHANGEL, BUTCHER, COLLECTOR, MAGISTRATE, PRIEST, SENTINEL, STALKER, STATUE, WRAITH};
    for (k, b) in [&MAGISTRATE, &WRAITH, &PRIEST, &STATUE, &ARCHANGEL, &COLLECTOR, &ALPHA, &AGENT, &SENTINEL, &BUTCHER, &STALKER].into_iter().enumerate() {
        arena(&mut p).side.foes[0] = crate::side::Foe::new(b, 2.0, -1.0, 400);
        let i = b.blows.iter().position(|x| ["grab", "knell", "leap", "throw"].contains(&x.art)).unwrap_or(0);
        throw(&mut p, i, 3.5);
        p.ticks(1);
        p.shot(&format!("m{:02}_{}_stands", 8 + 2 * k, b.sprite.unwrap().0));
        let w = windup(&mut p, i);
        p.ticks(w * 9 / 10);
        p.shot(&format!("m{:02}_{}_{}", 9 + 2 * k, b.sprite.unwrap().0, b.blows[i].art));
        p.ticks(w / 10 + 100);
    }
    // The hero side-on with an ultra greatsword, at the live frame of its light and its heavy.
    let slab = crate::gear::find("Gravewarden's Slab");
    p.game().world.gear.gain(slab);
    assert!(p.game().world.gear.wear(0, Some(slab)));
    let kit = p.game().world.gear.kit();
    for (act, label) in [(crate::side::Act::Light, "light"), (crate::side::Act::Heavy, "heavy")] {
        let a = arena(&mut p);
        a.side.foes[0].ai = Ai::Wait(400);
        a.side.hero.stats.kit = kit;
        let f = act.strike_spec(kit.arms.class).frames;
        a.side.hero.act = Some((act, f.0 + f.1 / 2 - 1));
        p.ticks(1);
        p.shot(&format!("m40_ultra_{label}"));
        arena(&mut p).side.hero.act = None;
        p.ticks(20);
    }
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
    p.game().world.warping = Some("gate@vale".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:gate@vale".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("gate@vale")), "never reached the gate");
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
        a.side.foes[0].ai = Ai::Blow { i, t: 0, hit: false, from: a.side.foes[0].s, aim: a.side.hero.s };
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
    assert!(g.world.boss_beaten() && g.world.souls > souls && g.world.gear.owns(crate::gear::find("Warden's Bell")));
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
    p.game().world.warping = Some("chapel@chapel_edge".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:chapel@chapel_edge".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("chapel@chapel_edge")), "never reached the chapel");
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
            assert!(!p.game().world.cache_taken(), "not before the trap is beaten");
            p.until_mode(300, |m| matches!(m, crate::game::Mode::Side(a) if a.t >= 170));
            p.shot("s04a_chest_trap");
            p.key("w", KeyboardState::Pressed);
        }
        p.fell_all();
        p.ticks(1);
        if p.game().world.cache_taken() { break; }
    }
    assert!(trap, "the chest never sprang");
    assert!(p.game().world.cache_taken(), "the cache was never reached");
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
    assert!(p.walk_until(1200, false, |s| s.stop.as_deref() == Some("chapel@chapel_edge")), "never back on the chapel road");
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
/// Each weapon held on the road, caught at the live frame of its light and its heavy:
/// `cargo test --release movesets -- --ignored --nocapture` writes target/shots/m_<weapon>_<move>.png,
/// and m_<weapon>_<move>_fx.png two frames into its strike effect.
#[test]
#[ignore]
fn movesets() {
    use crate::fight::{Act, Move};
    let mut p = Play::started();
    p.ticks(240);
    for name in ["Wayfarer's Sword", "Duelist's Rapier", "Lancer's Shortspear", "Husk Cleaver", "Executioner's Greatsword", "Gravewarden's Slab"] {
        let g = p.game();
        let i = crate::gear::find(name);
        g.world.gear.gain(i);
        assert!(g.world.gear.wear(0, Some(i)), "{name} would not go on");
        g.fight.hero.stats.kit = g.world.gear.kit();
        for e in g.fight.enemies.iter_mut() { e.ai = crate::fight::Ai::Dead; }
        let class = g.fight.hero.stats.kit.arms.class;
        for (mv, label) in [(Move::Light, "light"), (Move::Heavy, "heavy")] {
            let spec = mv.spec_for(class, None);
            let h = &mut p.game().fight.hero;
            // Halfway through the live ticks: the blow at full stretch.
            // (one tick before it: the frame is chosen as the tick runs)
            h.act = Some(Act { mv, spec, t: spec.frames.0 + spec.frames.1 / 2 - 1, landed: true, from_lane: h.lane, from_x: h.x });
            p.ticks(1);
            p.shot(&format!("m_{}_{label}", class.slug()));
            // Two frames into the weapon's strike effect (three fight ticks a frame).
            let h = &mut p.game().fight.hero;
            h.act = Some(Act { mv, spec, t: spec.frames.0 + 6 - 1, landed: true, from_lane: h.lane, from_x: h.x });
            p.ticks(1);
            p.shot(&format!("m_{}_{label}_fx", class.slug()));
        }
        p.game().fight.hero.act = None;
    }
}

/// `cargo test --release ambush -- --ignored --nocapture` writes target/shots/a*.png.
#[test]
#[ignore]
fn ambush() {
    use crate::game::{Mode, Why};
    let mut p = Play::started();
    p.game().world.warping = Some("vale@vale_edge".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:vale@vale_edge".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("vale@vale_edge")), "never reached the vale");
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

/// The ranged kinds on the road and in the yard: an archer's drawn bow and its lane, the arrow in
/// flight; a hexer's hex coming down the lane and its rune drawing itself in under the hero and
/// going off; then the Violet Road's ambush, a hexer behind husks.
#[test]
#[ignore]
fn ranged() {
    use crate::fight::{Archetype, Enemy, Phase};
    use crate::game::Mode;
    let mut p = Play::started();
    p.ticks(40);
    let walked = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
    let road = |p: &mut Play, kind: Archetype, ahead: f64, lane: i32| {
        let g = p.game();
        g.fight.enemies = vec![Enemy::new(kind, walked + crate::fight::HERO_D as f64 + ahead, lane)];
        g.fight.missiles.clear();
    };
    let attacking = |name: &'static str, at: f32| move |f: &crate::fight::Fight| f.enemies[0].attack().is_some_and(|(a, ph, k)| a.art == name && ph == Phase::Windup && k >= at);
    // The hero's own first: a bow's arrow, then a frost bolt, away up the lane at a husk.
    for (r, name) in [(crate::gear::Ranged::Bow, "h01_arrow_away"), (crate::gear::Ranged::Catalyst, "h02_frost_away")] {
        road(&mut p, Archetype::Husk, 10.0, 0);
        p.game().fight.enemies[0].ai = crate::fight::Ai::Wait(10_000);
        { let h = &mut p.game().fight.hero; h.stats.kit.off = None; h.stats.kit.ranged = Some(r); *h = crate::fight::Hero::with(h.stats); }
        p.tap("f");
        p.until(30, |f| f.hero.act.is_some_and(|a| a.t >= 20));
        p.shot(&format!("{name}_draw"));
        assert!(p.until(120, |f| f.bolts.first().is_some_and(|b| b.age >= 10)), "nothing loosed");
        p.shot(name);
        p.until(120, |f| f.bolts.is_empty());
    }
    { let kit = p.game().world.gear.kit(); let h = &mut p.game().fight.hero; h.stats.kit = kit; *h = crate::fight::Hero::with(h.stats); }
    road(&mut p, Archetype::Archer, 9.0, 0);
    assert!(p.until(900, attacking("loose", 0.5)), "the archer never drew");
    p.shot("r01_bow_drawn");
    assert!(p.until(200, |f| f.missiles.first().is_some_and(|m| m.age >= 12)), "no arrow loosed");
    p.shot("r02_arrow_flying");
    p.until(60, |f| f.missiles.first().is_some_and(|m| m.age >= 24));
    p.shot("r03_arrow_near");
    road(&mut p, Archetype::Hexer, 7.0, 0);
    { let h = &mut p.game().fight.hero; h.hp = h.stats.max_hp(); }
    assert!(p.until(900, attacking("hex", 0.4)), "the hexer never cast its hex");
    p.shot("r04_hex_gathering");
    p.until(200, |f| f.missiles.first().is_some_and(|m| m.age >= 20));
    p.shot("r05_hex_flying");
    for _ in 0..6 {
        { let h = &mut p.game().fight.hero; h.hp = h.stats.max_hp(); }
        if p.until(600, attacking("rune", 0.3)) { break; }
    }
    p.shot("r06_rune_drawing");
    p.until(200, attacking("rune", 0.85));
    p.shot("r07_rune_closing");
    p.until(80, |f| f.enemies[0].attack().is_some_and(|(_, ph, _)| ph == Phase::Active));
    p.ticks(2);
    p.shot("r08_rune_burst");
    // The yard: the Violet Road's ambush.
    p.game().fight.enemies.clear();
    p.game().world.warping = Some("violet_road@magistrate".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:violet_road@magistrate".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("violet_road@magistrate")), "never reached the violet road");
    p.ticks(10);
    p.key("w", KeyboardState::Pressed);
    let mut sprung = false;
    for _ in 0..8000 {
        for e in p.game().fight.enemies.iter_mut() { e.ai = crate::fight::Ai::Dead; }
        p.ticks(1);
        if matches!(p.mode(), Mode::Side(_)) { sprung = true; break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(sprung, "the ambush never sprang");
    assert!(p.until_mode(400, |m| matches!(m, Mode::Side(a) if a.t >= 140)));
    // The husks out of the way, so the hexer casts.
    for f in arena(&mut p).side.foes.iter_mut().filter(|f| f.breed.name != "Hexer") { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    let mut seen = std::collections::HashSet::new();
    for _ in 0..3000 {
        { let a = arena(&mut p); a.side.hero.hp = a.side.hero.stats.max_hp(); }
        p.ticks(1);
        let a = arena(&mut p);
        let Some(f) = a.side.foes.iter().find(|f| f.breed.name == "Hexer") else { break };
        let Some((b, ph, k)) = f.blow() else { continue };
        let name = match (b.name, ph) {
            ("rune", Phase::Windup) if k > 0.7 => "y01_rune_drawing",
            ("rune", Phase::Active) => "y02_rune_burst",
            ("hex", _) if f.shot().is_some() => "y03_hex_flying",
            ("repel", Phase::Active) => "y04_repel",
            _ => continue,
        };
        if seen.insert(name) { p.shot(name); }
        if seen.len() == 4 { break; }
    }
    eprintln!("[ranged] yard shots {seen:?}");
    // The hero's own, side-on: the bow at full draw, then the focus thrust out.
    for (r, name) in [(crate::gear::Ranged::Bow, "y05_bow_drawn"), (crate::gear::Ranged::Catalyst, "y06_focus_cast")] {
        for f in arena(&mut p).side.foes.iter_mut().filter(|f| f.ai != crate::side::Ai::Dead) { f.ai = crate::side::Ai::Wait(10_000); }
        { let h = &mut arena(&mut p).side.hero; h.stats.kit.off = None; h.stats.kit.ranged = Some(r); (h.arrows, h.charges) = (12, 3.0); h.hp = h.stats.max_hp(); }
        // Up off the ground (the hexer's repel floors them) and free.
        p.until_mode(400, |m| matches!(m, Mode::Side(a) if a.side.hero.act.is_none() && a.side.hero.reel == crate::fight::Reel::None));
        p.tap("f");
        let at = if r == crate::gear::Ranged::Bow { 0.8 } else { 1.02 };
        assert!(p.until_mode(200, |m| matches!(m, Mode::Side(a) if a.side.hero.act.is_some_and(|(act, t)| act == crate::side::Act::Loose
            && t as f32 >= at * crate::side::Act::Loose.frames(&a.side.hero.stats.kit).0 as f32))), "{name}: never loosed");
        p.shot(name);
        p.ticks(90);
    }
}

/// The boss fight under way in the capture.
fn arena(p: &mut Play) -> &mut crate::game::Arena {
    match &mut p.game().mode { crate::game::Mode::Side(a) => a, _ => panic!("not in the boss fight") }
}

/// The Warden on the road: the Bulwark's ward filled by an ordinary block, then a block raised
/// at the last moment against a chop, the ward bursting into the husk.
#[test]
#[ignore]
fn warden() {
    use crate::fight::{Ai, Archetype, Enemy, Phase};
    use prism::event::MouseButton as B;
    let mut p = Play::started();
    p.ticks(40);
    {
        let g = p.game();
        let b = crate::gear::find("Warden's Bulwark");
        g.world.gear.gain(b);
        g.world.gear.wear(1, Some(b));
        let h = &mut g.fight.hero;
        h.stats.kit = g.world.gear.kit();
        *h = crate::fight::Hero::with(h.stats);
    }
    let walked = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
    p.game().fight.enemies = vec![Enemy::new(Archetype::Husk, walked + crate::fight::HERO_D as f64 + 1.2, 0)];
    // Held up through a chop: blocked, and the ward fills.
    assert!(p.until_attack("chop"), "the husk never chopped");
    p.mouse(B::Right, MouseState::Pressed);
    assert!(p.until(200, |f| f.hero.bulwark.ward > 0.0), "the ward never filled");
    p.ticks(4);
    p.shot("w01_ward_filling");
    p.mouse(B::Right, MouseState::Released);
    // Stored past a burst's worth, the guard down a while, then raised as the chop falls.
    p.game().fight.hero.bulwark.ward = 70.0;
    { let h = &mut p.game().fight.hero; h.stamina = h.stats.max_stamina(); }
    assert!(p.until_attack_unguarded("chop"), "no second chop");
    assert!(p.until(200, |f| f.enemies[0].attack().is_some_and(|(a, ph, k)| ph == Phase::Windup && k >= 1.0 - 5.0 / a.frames.0 as f32)), "the chop never neared");
    p.mouse(B::Right, MouseState::Pressed);
    assert!(p.until(30, |f| matches!(f.enemies[0].ai, Ai::Staggered(_))), "no ward burst");
    p.ticks(3);
    p.shot("w02_ward_burst");
    p.ticks(14);
    p.shot("w03_ward_spent");
    p.mouse(B::Right, MouseState::Released);
}

/// Save profiles: a Warden's pilgrimage kept as it goes, the game launched again, the list with
/// it on, carried on at its shrine with its levels and what it had; then a second pilgrimage
/// begun in a free slot beside it, and the first forgotten (asked twice).
#[test]
#[ignore]
fn profiles() {
    use crate::game::Mode;
    use crate::profile::{self, Slot};
    let mut p = Play::new();
    p.ticks(5);
    p.tap("e");
    p.tap("e");
    p.tap("s");
    p.tap("e");
    assert!(p.until_playing(600), "never reached the road");
    {
        let g = p.game();
        assert_eq!(g.slot, Some(0));
        g.world.souls = 777;
        g.fight.hero.stats.vigor = 5;
    }
    p.game().keep();
    let dir = p.game().saves.clone();
    let level = p.game().fight.hero.stats.level();
    let list = profile::summaries(&dir);
    let Slot::Kept(first) = &list[0] else { panic!("nothing kept: {list:?}") };
    assert_eq!((first.calling, first.level), (crate::shrine::Calling::Warden, level));
    drop(p);
    // Launched again: the list opens on it; carried on, it wakes at its shrine as it was.
    let mut p = Play::launched();
    p.ticks(5);
    p.tap("e");
    p.ticks(2);
    p.shot("p01_pilgrimages");
    p.tap("e");
    p.ticks(2);
    p.shot("p02_carry_on");
    p.tap("e");
    assert!(p.until_mode(400, |m| matches!(m, Mode::Wake(_))), "never woke");
    assert!(p.until_playing(400), "never back on the road");
    p.shot("p03_carried_on");
    {
        let g = p.game();
        assert_eq!((g.world.souls, g.fight.hero.stats.vigor), (777, 5));
        assert_eq!(g.fight.hero.stats.kit.off.map(|o| o.art), Some("warden_bulwark"));
    }
    drop(p);
    // Another calling in the free slot beside it: both kept.
    let mut p = Play::launched();
    p.ticks(5);
    p.tap("e");
    p.tap("s");
    p.tap("e");
    p.ticks(2);
    p.shot("p04_new_in_slot_two");
    p.tap("s");
    p.tap("s");
    p.tap("e");
    assert!(p.until_playing(600), "the second never reached the road");
    assert_eq!(p.game().slot, Some(1));
    p.game().keep();
    let list = profile::summaries(&dir);
    assert!(matches!((&list[0], &list[1]), (Slot::Kept(a), Slot::Kept(b)) if a.calling == crate::shrine::Calling::Warden && b.calling == crate::shrine::Calling::Bloodrunner), "{list:?}");
    drop(p);
    // Forget the first: asked again, kept unless "forget it for good" is chosen.
    let mut p = Play::launched();
    p.ticks(5);
    p.tap("e");
    p.tap("w");
    p.tap("e");
    p.tap("s");
    p.tap("e");
    p.ticks(2);
    p.shot("p05_forget_asks");
    p.tap("e");
    p.ticks(2);
    assert!(matches!(profile::summaries(&dir)[0], Slot::Kept(_)), "keep it was chosen first");
    p.tap("e");
    p.tap("s");
    p.tap("e");
    p.ticks(2);
    let list = profile::summaries(&dir);
    assert!(list[0] == Slot::Empty && matches!(list[1], Slot::Kept(_)), "{list:?}");
    p.shot("p06_forgotten");
}
