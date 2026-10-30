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

    /// Past the title: the first slot, the Wayfarer chosen, awake at the first shrine, then on the
    /// Road to Hollowmere, the first road these captures walk (a pilgrimage begins in the Warrens).
    fn started() -> Play {
        let mut p = Play::new();
        p.ticks(5);
        p.tap("e");
        p.tap("e");
        p.skip_prologue();
        assert!(p.until_playing(600), "never reached the road");
        p.warp("road@warrens_lane");
        p
    }

    /// Cut to `stop`, the world with it, and play on from there.
    fn warp(&mut self, stop: &str) {
        self.game().world.warping = Some(stop.into());
        self.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: format!("warp:{stop}") });
        assert!(self.walk_until(3000, false, |s| s.stop.as_deref() == Some(stop)), "never reached {stop}");
        assert!(self.until_playing(600), "never played at {stop}");
    }

    /// Asked whether to play the prologue: no.
    fn skip_prologue(&mut self) { self.tap("s"); self.tap("e"); self.skip_opening(); }
/// A cutscene playing (a boss's entrance), skipped.
fn skip_scene(&mut self) { if self.canvas.get_plugin::<crate::game::Pilgrimage>().unwrap().in_scene() { self.tap("e"); self.ticks(1); } }
/// A pilgrimage just begun: its opening cutscene, skipped (E) once it plays.
fn skip_opening(&mut self) {
    for _ in 0..200 { if self.canvas.get_plugin::<crate::game::Pilgrimage>().unwrap().in_scene() { break; } self.ticks(1); }
    self.tap("e");
}

    fn key(&mut self, ch: &str, state: KeyboardState) { self.key_as(Key::Character(ch.into()), state); }
    fn key_as(&mut self, key: Key, state: KeyboardState) {
        let req = self.canvas.request_size();
        let sized = self.canvas.build(screen(), req);
        // With the modifiers held, as a window reports them (quartz holds Shift by what each key says).
        let held = |n| self.canvas.is_key_held(&Key::Named(n));
        let modifiers = Modifiers { shift: held(NamedKey::Shift), ctrl: held(NamedKey::Control), alt: held(NamedKey::Alt), meta: held(NamedKey::Meta) };
        let evt = KeyboardEvent { key, state, modifiers };
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

    /// A finger `id` doing `phase` at `at` (virtual units), sent as a phone sends it: the finger,
    /// then its mouse twin (`ramp`).
    fn finger(&mut self, id: u64, phase: prism::event::TouchPhase, at: (f32, f32)) {
        let (sw, sh) = screen();
        let k = (sw / 3840.0).min(sh / 2160.0);
        let pos = (at.0 * k + 0.5 * (sw - 3840.0 * k), at.1 * k + 0.5 * (sh - 2160.0 * k));
        let req = self.canvas.request_size();
        let sized = self.canvas.build(screen(), req);
        OnEvent::on_event(&mut self.canvas, &mut self.ctx, &sized, Box::new(prism::event::TouchEvent { id, phase, position: Some(pos) }) as Box<dyn Event>);
        let state = match phase {
            prism::event::TouchPhase::Started => Some(MouseState::Pressed),
            prism::event::TouchPhase::Ended | prism::event::TouchPhase::Cancelled => Some(MouseState::Released),
            prism::event::TouchPhase::Moved => None,
        };
        if let Some(state) = state {
            let evt = MouseEvent { position: Some(pos), state, button: prism::event::MouseButton::Left };
            OnEvent::on_event(&mut self.canvas, &mut self.ctx, &sized, Box::new(evt) as Box<dyn Event>);
        }
    }
    /// A tap of one finger at `at`.
    fn touch(&mut self, at: (f32, f32)) {
        use prism::event::TouchPhase as T;
        self.finger(9, T::Started, at);
        self.ticks(2);
        self.finger(9, T::Ended, at);
        self.ticks(2);
    }
    fn button(&mut self, b: crate::touch::Btn) -> (f32, f32) {
        let ui = self.game().touch_mode_for_test();
        crate::touch::layout(ui).into_iter().find(|x| x.btn == b).unwrap_or_else(|| panic!("no {b:?} for {ui:?}")).at
    }

    /// `n` ticks, each given a few milliseconds so the road's worker keeps up as in a window.
    fn ticks(&mut self, n: u32) {
        for _ in 0..n {
            let at = std::time::Instant::now();
            let req = self.canvas.request_size();
            let sized = self.canvas.build(screen(), req);
            OnEvent::on_event(&mut self.canvas, &mut self.ctx, &sized, Box::new(TickEvent) as Box<dyn Event>);
            // `HR_SLOW_TICK=ms`: name each tick (with the item list a frame draws) slower than that.
            if let Some(limit) = std::env::var("HR_SLOW_TICK").ok().and_then(|v| v.parse::<f32>().ok()) {
                let ticked = at.elapsed().as_secs_f32() * 1000.0;
                let req = self.canvas.request_size();
                let sized = self.canvas.build(screen(), req);
                let drawn = std::time::Instant::now();
                let _ = self.canvas.draw(&sized, (0.0, 0.0), (0.0, 0.0, screen().0, screen().1));
                let items = drawn.elapsed().as_secs_f32() * 1000.0;
                if ticked + items > limit { eprintln!("[slow tick] {ticked:.1} ms + items {items:.1} ms: {}", self.status()); }
            }
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
    /// Shift with a way: a roll that way.
    fn roll(&mut self, ch: &str) {
        self.key_as(Key::Named(NamedKey::Shift), KeyboardState::Pressed);
        self.tap(ch);
        self.key_as(Key::Named(NamedKey::Shift), KeyboardState::Released);
    }
    /// At the veil before a standing boss, held there: step through it (E). Whether it did.
    fn step_through_veil(&mut self) -> bool {
        let ahead = self.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
        let g = self.game();
        let at = g.playing() && g.world.at_veil(&g.fight, ahead);
        if at { self.tap("e"); }
        at
    }
    /// A way tapped twice and held: a run.
    fn run_on(&mut self, ch: &str) { self.tap(ch); self.key(ch, KeyboardState::Pressed); }
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
    // Walk up to the first husk until it notices, wait for it to come, and swing at it as soon
    // as it is in reach, rested and fresh: the light's swipe, a few ticks into its live frames.
    p.key("w", KeyboardState::Pressed);
    for _ in 0..600 {
        let walked = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
        if p.fight().enemies[0].gap(walked) <= crate::fight::ENGAGE + 0.05 { break; }
        if p.fight().enemies[0].idle.is_none() { p.key("w", KeyboardState::Released); }
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
    eprintln!("[fight] rolling at {:?}, hero {:?} stamina {:.1} hp {} lane {}", p.fight().enemies[0].ai, p.fight().hero.act.map(|a| a.mv), p.fight().hero.stamina, p.fight().hero.hp, p.fight().hero.lane);
    p.roll("d");
    // The roll through, a frame every few ticks (slow motion once the chop misses).
    for (i, n) in [2, 3, 3, 3, 3, 4, 4].iter().enumerate() {
        p.ticks(*n);
        p.shot(&format!("03a_roll_{i}"));
    }
    p.ticks(4);
    p.shot("03_just_dodge");
    eprintln!("[fight] after the roll: hp {} lane {} husk {:?}", p.fight().hero.hp, p.fight().hero.lane, p.fight().enemies[0].ai);
    if !matches!(p.fight().enemies[0].ai, Ai::Stumbled(_)) { for (t, e) in &p.canvas.get_plugin::<crate::game::Pilgrimage>().unwrap().log { eprintln!("[log] {t} {e:?}"); } }
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
        let foe = g.fight.enemies.first().map(|e| format!("  foe {:?} lane {} gap {:.2} hp {:.0}", e.ai, e.lane, e.gap(walked), e.hp)).unwrap_or_default();
        format!("stop {stop:?} choose_within {within:?} walked {walked:.1} mode {:?} at {} warping {:?} sent {} lean {:?} chose {} lane {}{foe}  {:?}",
            g.mode, g.world.at, g.world.warping, g.world.sent, g.world.lean, g.world.chose, g.fight.hero.lane, g.prologue)
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

/// The equipment panels with long lines in them: the weapons to wear (each its name, then what
/// it is and does) and the gear slots. `cargo test --release gear_panels -- --ignored --nocapture`.
#[test]
#[ignore]
fn gear_panels() {
    use crate::game::Mode;
    let mut p = Play::new();
    p.ticks(40);
    p.tap("e");
    p.ticks(2);
    p.tap("e");
    p.ticks(2);
    p.skip_prologue();
    assert!(p.until_mode(200, |m| matches!(m, Mode::Wake(_))));
    assert!(p.until_playing(300));
    // The first shrine, on the Warrens' narrow street: standing on its floor, inside the wall.
    p.ticks(20);
    p.shot("g00_shrine_on_the_street");
    // Every weapon there is, so the list is longer than the screen and scrolls.
    for (i, _) in crate::gear::ITEMS.iter().enumerate().filter(|(_, it)| it.arms.is_some()) { p.game().world.gear.gain(i); }
    p.tap("e");
    assert!(p.until_mode(400, |m| matches!(m, Mode::Sanctum(_))), "never sat down");
    p.ticks(40);
    p.tap("e");
    p.ticks(2);
    p.tap("s");
    p.tap("e");
    p.ticks(2);
    p.shot("g01_equipment");
    p.tap("e");
    p.ticks(2);
    p.shot("g02_weapons");
    for _ in 0..9 { p.tap("s"); }
    p.ticks(2);
    p.shot("g03_weapons_scrolled");
    for _ in 0..40 { p.tap("s"); }
    p.ticks(2);
    p.shot("g03_weapons_last_picked");
    for _ in 0..40 { p.tap("w"); }
    p.shot("g03b_weapon_lines");
    // The keepsakes: what is carried and not worn, each with its lines; then back to the weapons.
    { let w = &mut p.game().world; w.keys.extend([crate::atlas::Key::Hook, crate::atlas::Key::ShardI]); w.remnants.push("Screamer's Drowned Voice"); w.shards = [4, 1, 0]; }
    for k in ["q", "q", "s", "s", "e"] { p.tap(k); p.ticks(2); }
    p.shot("g03c_keepsakes");
    p.tap("s");
    p.ticks(2);
    p.shot("g03d_keepsakes_shard");
    for k in ["q", "w", "w", "e", "e"] { p.tap(k); p.ticks(2); }
    // The smith's forge: a row for each piece worn, its level, its price and what the next does.
    { let w = &mut p.game().world; (w.met_smith, w.souls, w.shards) = (true, 5000, [9, 2, 0]); w.gear.levels.insert(w.gear.worn[0].unwrap(), 3); }
    p.game().fight.hero.stats.kit = p.game().world.gear.kit();
    for k in ["q", "q", "q", "d"] { p.tap(k); p.ticks(2); }
    p.ticks(30);
    p.tap("e");
    p.ticks(2);
    p.tap("e");
    p.ticks(2);
    p.shot("g04_forge");
    // Back to the shrine, and up from it to the Warrens' shrine, by its narrow street's wall.
    for k in ["q", "q", "a"] { p.tap(k); p.ticks(2); }
    p.ticks(30);
    p.tap("e");
    p.ticks(2);
    p.tap("e");
    assert!(p.until_playing(400), "never rose");
    p.game().world.warping = Some("warrens@warrens_lane".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:warrens@warrens_lane".into() });
    p.ticks(120);
    p.shot("g05_warrens_shrine");
    // Rested at it walking the Warrens back from the guild hall: the shrine is near the walk's far
    // end, and waking stands the hero by it there.
    let at = "warrens@^";
    let w = &mut p.game().world;
    (w.checkpoint, w.rested) = (at.into(), vec![at.into()]);
    let go = p.game().world.resume();
    let pf = p.canvas.get_plugin_mut::<quartz_path_forge::PathForgePlugin>().unwrap();
    match go { crate::world::Go::WarpAt(s, m) => pf.warp_at(&s, m), crate::world::Go::Warp(s) => pf.warp(&s), _ => unreachable!() }
    p.ticks(120);
    let walked = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
    let g = p.game();
    assert!(g.world.can_rest(&g.fight, walked), "woke {} m from the shrine", g.world.shrine().unwrap() - walked);
    p.shot("g06_warrens_shrine_walked_back");
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
    // A first launch goes straight to the callings.
    p.tap("e");
    p.ticks(2);
    p.shot("f01b_callings_first_launch");
    p.tap("s");
    p.ticks(2);
    p.shot("f02_callings_warden");
    p.tap("w");
    p.tap("e");
    p.ticks(2);
    p.shot("f02b_prologue_asked");
    p.skip_prologue();
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
    p.run_on("w");
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
            eprintln!("[perf] {:5.1}s  {:<12} progress {:?}  road ms median {:6.1} max {:6.1}  churn {:?}", now, st.stop.as_deref().unwrap_or("-"), st.progress.map(|p| (p * 100.0).round()), ms[ms.len() / 2], ms[ms.len() - 1],
                p.renderer.as_mut().map(|r| r.take_image_churn(4)));
            ms.clear();
        }
    }
}

/// Textures uploaded a second on the road with enemies about (walking up to the first and
/// guarding): what churns. `cargo test --release perf_churn -- --ignored --nocapture`.
#[test]
#[ignore]
fn perf_churn() {
    let mut p = Play::started();
    p.game().world.souls = 0;
    p.key("w", KeyboardState::Pressed);
    let t0 = std::time::Instant::now();
    let mut last = 0.0;
    let mut uploaded = p.renderer.as_ref().map_or(0, |r| r.image_uploads());
    while t0.elapsed().as_secs_f32() < 12.0 {
        let tick = std::time::Instant::now();
        let req = p.canvas.request_size();
        let sized = p.canvas.build(screen(), req);
        OnEvent::on_event(&mut p.canvas, &mut p.ctx, &sized, Box::new(TickEvent) as Box<dyn Event>);
        let items = p.canvas.draw(&sized, (0.0, 0.0), (0.0, 0.0, screen().0, screen().1));
        if let Some(r) = p.renderer.as_mut() { let _ = r.render(items); }
        if let Some(rest) = std::time::Duration::from_micros(16_667).checked_sub(tick.elapsed()) { std::thread::sleep(rest); }
        let now = t0.elapsed().as_secs_f32();
        if now - last >= 1.0 {
            last = now;
            let up = p.renderer.as_ref().map_or(0, |r| r.image_uploads());
            eprintln!("[perf_churn] {now:5.1}s uploads {} churn {:?}  {}", up - uploaded, p.renderer.as_mut().map(|r| r.take_image_churn(5)), p.status());
            uploaded = up;
        }
    }
}

/// The opening's cost, played as a window plays it (a draw a tick, 60 a second): each second the
/// game's own frame (update, draw, render) and the road's render, median and worst.
/// `HR_SCREEN=2940x1846 cargo test --release perf_opening -- --ignored --nocapture`.
#[test]
#[ignore]
fn perf_opening() {
    let mut p = Play::new();
    p.ticks(5);
    for ch in ["e", "e", "s", "e"] { p.tap(ch); }
    for _ in 0..200 { if p.scene_at().is_some() { break; } p.ticks(1); }
    assert_eq!(p.scene_at().map(|s| s.0), Some("opening"), "no opening: {}", p.status());
    let t0 = std::time::Instant::now();
    let (mut last, mut game, mut road, mut parts) = (0.0, Vec::new(), Vec::new(), Vec::new());
    let mut uploaded = p.renderer.as_ref().map_or(0, |r| r.image_uploads());
    while p.scene_at().is_some() && t0.elapsed().as_secs_f32() < 60.0 {
        let tick = std::time::Instant::now();
        let req = p.canvas.request_size();
        let sized = p.canvas.build(screen(), req);
        OnEvent::on_event(&mut p.canvas, &mut p.ctx, &sized, Box::new(TickEvent) as Box<dyn Event>);
        let updated = tick.elapsed().as_secs_f32() * 1000.0;
        let items = p.canvas.draw(&sized, (0.0, 0.0), (0.0, 0.0, screen().0, screen().1));
        let drawn = tick.elapsed().as_secs_f32() * 1000.0;
        if let Some(r) = p.renderer.as_mut() { let _ = r.render(items); }
        game.push(tick.elapsed().as_secs_f32() * 1000.0);
        parts.push((updated, drawn - updated, game[game.len() - 1] - drawn));
        if let Some(rest) = std::time::Duration::from_micros(16_667).checked_sub(tick.elapsed()) { std::thread::sleep(rest); }
        road.push(p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().map_or(0.0, |pf| pf.render_ms()));
        let now = t0.elapsed().as_secs_f32();
        if now - last >= 1.0 {
            last = now;
            game.sort_by(f32::total_cmp);
            road.sort_by(f32::total_cmp);
            let mean = |pick: fn(&(f32, f32, f32)) -> f32| parts.iter().map(pick).sum::<f32>() / parts.len().max(1) as f32;
            eprintln!("[perf_opening] {now:5.1}s {:?}  game ms median {:5.1} max {:6.1} (update {:4.1} draw {:4.1} render {:4.1})  road ms median {:6.1} max {:6.1}  uploads {}", p.scene_at(), game[game.len() / 2], game[game.len() - 1],
                mean(|x| x.0), mean(|x| x.1), mean(|x| x.2), road[road.len() / 2], road[road.len() - 1], p.renderer.as_ref().map_or(0, |r| r.image_uploads()) - uploaded);
            uploaded = p.renderer.as_ref().map_or(0, |r| r.image_uploads());
            eprintln!("    churn {:?}", p.renderer.as_mut().map(|r| r.take_image_churn(6)));
            (game, road, parts) = (Vec::new(), Vec::new(), Vec::new());
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
    p.run_on("w");
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
    // The marsh edge forks at its end, the vale one way and the Drowned Fingers the other.
    let vale_ahead_left = crate::atlas::leads("vale_edge", "road").first().is_some_and(|l| l.road.1 == "vale");
    assert!(p.walk_until(6000, true, |s| s.choose_within.is_some()), "no fork on the marsh edge: {}", p.status());
    p.tap(if vale_ahead_left { "a" } else { "d" });
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
    // The gate has three roads: turned round, its trunk is walked back down, and at its end the
    // junction it came up from: back down the vale, or on to the mill.
    let back = crate::atlas::turned("gate", "vale");
    assert_eq!(back, "gate@^");
    assert!(p.walk_until(1200, false, |s| s.stop.as_deref() == Some(back.as_str())), "never turned round: {:?}", p.walk_state().stop);
    p.run_on("w");
    p.ticks(60);
    p.shot("j05a_turned");
    let vale_left = crate::atlas::leads("vale", "gate")[0].stop == "vale@^";
    assert!(p.walk_until(4000, true, |s| s.choose_within.is_some()), "no fork walking back: {}", p.status());
    p.tap(if vale_left { "a" } else { "d" });
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("vale@^")), "never back in the vale: {}", p.status());
    p.ticks(90);
    p.shot("j05b_vale_backwards");
    assert!(p.walk_until(8000, true, |s| s.stop.as_deref() == Some("vale_edge@vale")), "never back at the marsh edge: {}", p.status());
    let w = &p.game().world;
    eprintln!("[journey] souls {}, seen {:?}", w.souls, w.seen);
    assert!(p.walk_until(4000, true, |s| s.choose_within.is_some()), "no second fork: {}", p.status());
    // In the middle lane the walk waits at the signpost.
    assert!(p.walk_until(4000, false, |s| s.choose_within.is_some_and(|l| l < 1.6)), "never reached the last moment: {}", p.status());
    p.ticks(120);
    assert!(p.walk_state().choose_within.is_some(), "the junction chose for the player");
    p.shot("j05c_waiting_at_fork");
    // The marsh edge's own junction first: down its trunk to the road, not on to the Fingers.
    let trunk_left = crate::atlas::leads("vale_edge", "vale")[0].stop == "vale_edge@^";
    p.tap(if trunk_left { "a" } else { "d" });
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("vale_edge@^")), "never back down the marsh edge: {}", p.status());
    assert!(p.walk_until(4000, true, |s| s.choose_within.is_some()), "no fork at the road: {}", p.status());
    // Past the marsh edge, into the road's junction: down it home, or on to the chapel.
    let chapel_left = crate::atlas::leads("road", "vale_edge")[0].shown == "chapel_edge";
    p.tap(if chapel_left { "a" } else { "d" });
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
        p.step_through_veil();
        if matches!(p.mode(), Mode::Side(_)) { met = true; break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(met, "the chief never stepped out: {}", p.status());
    p.skip_scene();
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
    for (k, b) in [&MAGISTRATE, &WRAITH, &PRIEST, &STATUE, &ARCHANGEL, &COLLECTOR, &ALPHA, &AGENT, &SENTINEL, &BUTCHER, &STALKER, &crate::side::BOUNTY_ECHO, &crate::side::CONSTRUCT, &crate::side::DRAKE].into_iter().enumerate() {
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

/// The game played by touch alone, as on a phone: the title and the pilgrimage begun by taps on
/// the choices; on the road, a thumb on the stick walking on and stepping a lane while another
/// strikes and guards (a touch's mouse twin is no strike); the map and the pause menu opened and
/// left by their buttons. `cargo test --release touch_controls -- --ignored --nocapture` writes
/// target/shots/u*.png.
#[test]
#[ignore]
fn touch_controls() {
    use crate::game::Mode;
    use crate::touch::Btn;
    use prism::event::TouchPhase as T;
    let mut p = Play::new();
    p.game().touch_ui = true;
    p.ticks(5);
    p.shot("u00_title");
    // The title goes on with a tap; then the slot and "no prologue" are tapped.
    p.touch((1920.0, 1200.0));
    p.ticks(4);
    p.shot("u01_slots");
    let row = |p: &mut Play, i: usize| { let r = p.game().menu_row_for_test(i); (1920.0, 0.5 * (r.0 + r.1)) };
    let at = row(&mut p, 0);
    p.touch(at);
    p.ticks(4);
    p.shot("u02_prologue_asked");
    let at = row(&mut p, 1);
    p.touch(at);
    p.skip_opening();
    assert!(p.until_playing(600), "never reached the road by touch: {}", p.status());
    p.ticks(30);
    p.shot("u03_road");
    // The left thumb down and pushed up: walking on.
    let walked0 = p.walk_state().walked;
    p.finger(1, T::Started, (700.0, 1640.0));
    p.ticks(1);
    p.finger(1, T::Moved, (700.0, 1420.0));
    // The road walked is the worker's frames', which a loaded machine slows: wait on the distance.
    for _ in 0..600 { if p.walk_state().walked > walked0 + 1.0 { break; } p.ticks(1); }
    let walked1 = p.walk_state().walked;
    assert!(walked1 > walked0 + 1.0, "the stick did not walk on: {walked0} -> {walked1}: {}", p.status());
    // A strike by the right thumb while the left holds; its mouse twin is not a second one.
    let strikes = |p: &mut Play| p.game().log.iter().filter(|(_, e)| matches!(e, crate::fight::Event::Began(crate::fight::Move::Light))).count();
    let before = strikes(&mut p);
    let at = p.button(Btn::Light);
    p.finger(2, T::Started, at);
    p.ticks(3);
    p.shot("u05_walking_and_striking");
    p.finger(2, T::Ended, at);
    p.ticks(40);
    assert_eq!(strikes(&mut p) - before, 1, "one tap, one strike");
    // Guard held.
    let at = p.button(Btn::Guard);
    p.finger(3, T::Started, at);
    p.ticks(4);
    assert!(p.fight().hero.guarding, "the guard button does not guard");
    p.finger(3, T::Ended, at);
    // Over to the right lane: the stick pushed right steps across once.
    p.finger(1, T::Moved, (980.0, 1640.0));
    p.ticks(30);
    assert_eq!(p.fight().hero.lane, 1, "the stick did not step a lane");
    p.finger(1, T::Ended, (980.0, 1640.0));
    p.ticks(10);
    // The map, and back.
    let at = p.button(Btn::Map);
    p.touch(at);
    p.ticks(10);
    assert!(matches!(p.mode(), Mode::Map), "the map did not open");
    p.shot("u06_map");
    let at = p.button(Btn::Back);
    p.touch(at);
    p.ticks(10);
    assert!(matches!(p.mode(), Mode::Play), "the map did not close");
    // Paused, and back.
    let at = p.button(Btn::Pause);
    p.touch(at);
    p.ticks(10);
    assert!(matches!(p.mode(), Mode::Front(crate::shrine::Front::Pause(_))), "not paused: {}", p.status());
    p.shot("u07_paused");
    let at = p.button(Btn::Back);
    p.touch(at);
    p.ticks(10);
    assert!(matches!(p.mode(), Mode::Play), "the pause menu did not close: {}", p.status());
}

/// The prologue by touch: its card goes on with a tap, its lessons speak of the thumb and the
/// buttons, and the pause button asks to leave it, a tap answering.
/// `cargo test --release touch_prologue -- --ignored --nocapture` writes target/shots/u1*.png.
#[test]
#[ignore]
fn touch_prologue() {
    use crate::game::Mode;
    use crate::touch::Btn;
    let mut p = Play::new();
    p.game().touch_ui = true;
    p.ticks(5);
    p.touch((1920.0, 1200.0));
    p.ticks(4);
    let row = |p: &mut Play, i: usize| { let r = p.game().menu_row_for_test(i); (1920.0, 0.5 * (r.0 + r.1)) };
    let at = row(&mut p, 0);
    p.touch(at);
    p.ticks(4);
    // "Begin with the prologue".
    let at = row(&mut p, 0);
    p.touch(at);
    p.skip_opening();
    assert!(p.until_playing(600), "never reached the prologue: {}", p.status());
    let lesson = |p: &mut Play| p.game().prologue.as_ref().map(|q| q.lesson);
    assert_eq!(lesson(&mut p), Some(crate::prologue::Lesson::Memory));
    p.ticks(20);
    p.shot("u10_prologue_card");
    p.touch((1920.0, 900.0));
    p.ticks(20);
    assert_eq!(lesson(&mut p), Some(crate::prologue::Lesson::Walk), "a tap did not go on from the card");
    p.shot("u11_walk_lesson");
    let at = p.button(Btn::Pause);
    p.touch(at);
    p.ticks(6);
    assert!(p.game().prologue.as_ref().is_some_and(|q| q.leaving), "the pause button did not ask to leave");
    p.shot("u12_leave_asked");
    p.touch((1920.0, 900.0));
    p.ticks(10);
    p.skip_opening();
    assert!(p.until_playing(600), "never left the prologue: {}", p.status());
    assert!(p.game().prologue.is_none() && matches!(p.mode(), Mode::Play), "still in the prologue");
}

/// The same views painted and not: run once plain and once with `PF_PAINT=0` (every scene read
/// with no painterly brush); the shots are named for which. Writes target/shots/look_*.png.
#[test]
#[ignore]
fn look_paint() {
    let tag = std::env::var("PF_PAINT").map_or("painted".to_string(), |v| format!("paint{v}"));
    let mut p = Play::started();
    p.ticks(60);
    p.shot(&format!("look_{tag}_a_shrine"));
    // Up the road until the first husk is close.
    p.run_on("w");
    for _ in 0..1200 { if p.fight().enemies.first().is_some_and(|e| e.alive() && e.gap(p.walk_state().walked) < 4.0) { break; } p.ticks(1); }
    p.key("w", KeyboardState::Released);
    p.shot(&format!("look_{tag}_b_husk"));
    p.game().world.warping = Some("warrens@warrens_lane".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:warrens@warrens_lane".into() });
    p.ticks(150);
    p.shot(&format!("look_{tag}_c_warrens"));
}

/// The road rolls: forward away down the road (Shift+W) and backward toward the camera
/// (Shift+S), shot through each. `cargo test --release roll_moves -- --ignored --nocapture`
/// writes target/shots/v*.png.
#[test]
#[ignore]
fn roll_moves() {
    let mut p = Play::started();
    p.game().world.souls = 0;
    p.ticks(20);
    for (way, name, mv) in [("w", "forward", crate::fight::Move::Tumble), ("s", "back", crate::fight::Move::Backstep)] {
        p.roll(way);
        let mut seen = 0;
        for t in 0..30 {
            p.ticks(1);
            if p.fight().hero.act.is_some_and(|a| a.mv == mv) { seen += 1; if t % 2 == 1 { p.shot(&format!("v_{name}_{t:02}")); } }
        }
        assert!(seen > 5, "no {name} roll: {}", p.status());
        p.ticks(30);
    }
}

/// The Debt Collector in the guild hall, side-on: its lunge across the yard, the rap it answers a
/// beating with, its late chop held, and its waking string (chop, then the sweep).
/// `cargo test --release collector_moves -- --ignored --nocapture` writes target/shots/c*.png.
#[test]
#[ignore]
fn collector_moves() {
    use crate::game::Mode;
    use crate::side::Ai;
    let mut p = Play::started();
    let hall = "guild_hall@warrens";
    p.game().world.warping = Some(hall.into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: format!("warp:{hall}") });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some(hall)), "never reached the hall: {}", p.status());
    p.ticks(10);
    p.key("w", KeyboardState::Pressed);
    let mut met = false;
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        p.step_through_veil();
        if matches!(p.mode(), Mode::Side(_)) { met = true; break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(met, "the collector never stepped out: {}", p.status());
    p.skip_scene();
    assert!(p.until_mode(400, |m| matches!(m, Mode::Side(a) if a.t >= 140)));
    assert_eq!(arena(&mut p).side.foes[0].breed.name, "The Debt Collector");
    p.ticks(10);
    p.shot("c01_collector");
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
    let frames = |p: &mut Play, i: usize| arena(p).side.foes[0].timed(i).frames;
    // The lunge: warned across the yard, then the hand reaching mid-run.
    throw(&mut p, 4, 6.5);
    let (w, live, _) = frames(&mut p, 4);
    p.ticks(w * 8 / 10);
    p.shot("c02_lunge_warned");
    p.ticks(w * 2 / 10 + live / 3);
    p.shot("c03_lunge_running");
    p.ticks(120);
    // The rap: quick, from standing.
    throw(&mut p, 5, 1.6);
    let (w, _, _) = frames(&mut p, 5);
    p.ticks(w - 2);
    p.shot("c04_rap_falling");
    p.ticks(80);
    // The late chop held high past its tell.
    throw(&mut p, 3, 1.6);
    let (w, _, _) = frames(&mut p, 3);
    p.ticks(w * 6 / 10);
    p.shot("c05_late_chop_held");
    p.ticks(w * 4 / 10 + 80);
    // Its waking string: the chop, and the sweep straight after it.
    { let a = arena(&mut p); a.side.foes[0].hp = 0.3 * a.side.foes[0].top; }
    throw(&mut p, 6, 1.6);
    let (w, live, rec) = frames(&mut p, 6);
    p.ticks(w + 1);
    p.shot("c06_foreclosure_chop");
    let mut swept = false;
    for _ in 0..(live + rec + 40) {
        p.ticks(1);
        if arena(&mut p).side.foes[0].blow().is_some_and(|b| b.0.name == "sweep" && b.2 > 0.9) { swept = true; break; }
    }
    assert!(swept, "the sweep never followed the foreclosure");
    p.shot("c07_foreclosure_sweep");
    // The other bosses' new answers, in the same yard: the Butcher's hook in flight, the Statue's
    // tremor, the Warden's haft and the Chief's axe butt.
    for (k, (b, name, gap)) in [(&crate::side::BUTCHER, "hook and chain", 5.5), (&crate::side::STATUE, "tremor", 2.4), (&crate::side::WARDEN, "haft", 2.2), (&crate::side::CHIEF, "butt", 2.0)].into_iter().enumerate() {
        arena(&mut p).side.foes[0] = crate::side::Foe::new(b, 2.0, -1.0, 400);
        let i = b.blows.iter().position(|x| x.name == name).unwrap();
        throw(&mut p, i, gap);
        let (w, live, _) = frames(&mut p, i);
        p.ticks(w + live / 3);
        p.shot(&format!("c{:02}_{}_{}", 8 + k, b.sprite.unwrap().0, name.replace(' ', "_")));
        p.ticks(120);
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
    // The veil across the road before the Warden: from down the road, held at it, and
    // stepping through it.
    let mut far = false;
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        let ahead = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
        let veil = p.game().world.veil();
        if !far && veil.is_some_and(|v| v - ahead < 14.0) { p.shot("b00a_veil"); far = true; }
        let at = { let g = p.game(); g.playing() && g.world.at_veil(&g.fight, ahead) };
        if at {
            p.ticks(20);
            p.shot("b00b_at_the_veil");
            // The hand pressing into it from beyond, at its deepest (`veil_picture`: 128 ticks into
            // each 512, for 64).
            while p.game().ticks % 512 != 160 { p.ticks(1); }
            p.shot("b00b2_pressed_from_beyond");
            p.tap("e");
            p.ticks(14);
            p.shot("b00c_stepping_through");
            p.ticks(18);
            p.shot("b00d_burning_open");
        }
        if matches!(p.mode(), Mode::Side(_)) { met = true; break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(met, "the boss never stepped out");
    p.ticks(40);
    p.shot("b01_swing");
    p.skip_scene();
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
    // Space jumps in the yard (W does not: a thumb on the touch stick pushed it).
    let hp = arena(&mut p).side.hero.hp;
    p.tap(" ");
    p.ticks(12);
    assert!(arena(&mut p).side.hero.airborne(), "no jump");
    p.shot("b04_jumped_the_sweep");
    p.ticks(40);
    assert_eq!(arena(&mut p).side.hero.hp, hp, "the sweep passed under");
    throw(&mut p, 2, 3.0);
    let w = windup(&mut p, 2);
    p.ticks(w + 2);
    p.shot("b05a_knell_struck");
    p.ticks(10);
    p.shot("b05_knell_ring");
    p.ticks(14);
    p.shot("b05b_knell_rolling");
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
/// A or D held on the road keeps stepping lanes that way until the road's edge, and a tap
/// steps one: `cargo test --release shots::lanes_held -- --ignored`.
#[test]
#[ignore]
fn lanes_held() {
    let mut p = Play::started();
    p.ticks(60);
    assert_eq!(p.game().fight.hero.lane, 0);
    p.key("d", KeyboardState::Pressed);
    p.ticks(1);
    p.key("d", KeyboardState::Released);
    p.ticks(40);
    assert_eq!(p.game().fight.hero.lane, 1, "a tap steps one lane");
    p.key("a", KeyboardState::Pressed);
    p.ticks(90);
    assert_eq!(p.game().fight.hero.lane, -1, "held, it carries on across the road");
    p.ticks(40);
    assert_eq!(p.game().fight.hero.lane, -1, "and stops at its edge");
    p.key("a", KeyboardState::Released);
}

/// The loading screen over the fade's black, a moment apart:
/// `cargo test --release shots::loading -- --ignored` writes target/shots/loading_<k>.png
/// (`HR_SCREEN=2400x1080` for a phone's shape, where nothing of the road may show at the sides).
#[test]
#[ignore]
fn loading() {
    let mut p = Play::new();
    p.ticks(5);
    p.game().hold_loading_for_test();
    for k in 0..3 {
        p.ticks(if k == 0 { 40 } else { 9 });
        p.shot(&format!("loading_{k}"));
    }
}

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

/// Every thrust drawn tick by tick from wind-up to recovery, its effect over it:
/// `cargo test --release shots::thrusts -- --ignored` writes target/shots/t_<weapon>_<move>_<tick>.png.
#[test]
#[ignore]
fn thrusts() {
    use crate::fight::{Act, Move};
    let mut p = Play::started();
    p.ticks(240);
    for name in ["Duelist's Rapier", "Lancer's Shortspear"] {
        let g = p.game();
        let i = crate::gear::find(name);
        g.world.gear.gain(i);
        assert!(g.world.gear.wear(0, Some(i)), "{name} would not go on");
        g.fight.hero.stats.kit = g.world.gear.kit();
        for e in g.fight.enemies.iter_mut() { e.ai = crate::fight::Ai::Dead; }
        let class = g.fight.hero.stats.kit.arms.class;
        for (mv, label) in [(Move::Light, "light"), (Move::Heavy, "heavy")] {
            let spec = mv.spec_for(class, None);
            let all = spec.frames.0 + spec.frames.1 + spec.frames.2;
            for t in (0..all).step_by(2) {
                let h = &mut p.game().fight.hero;
                h.act = Some(Act { mv, spec, t, landed: true, from_lane: h.lane, from_x: h.x });
                p.ticks(1);
                p.shot(&format!("t_{}_{label}_{t:02}", class.slug()));
            }
        }
        p.game().fight.hero.act = None;
    }
}

/// Each weapon's light and heavy caught at four frames of its strike effect, a picture each:
/// `cargo test --release strike_effects -- --ignored` writes target/shots/fx_<weapon>_<move>_<k>.png.
#[test]
#[ignore]
fn strike_effects() {
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
            for k in 0..4 {
                let h = &mut p.game().fight.hero;
                h.act = Some(Act { mv, spec, t: spec.frames.0 + 3 * k + 1, landed: true, from_lane: h.lane, from_x: h.x });
                p.ticks(1);
                p.shot(&format!("fx_{}_{label}_{k}", class.slug()));
            }
        }
        p.game().fight.hero.act = None;
    }
}

/// The capstones' looks on the hero, on the road: each weapon's with a shield's, a bow's and a
/// catalyst's, armour's across the body, standing and mid-swing.
/// `cargo test --release capstone_looks -- --ignored --nocapture` writes target/shots/cap_*.png.
#[test]
#[ignore]
fn capstone_looks() {
    use crate::capstone::Capstone;
    use crate::fight::{Act, Move};
    let mut p = Play::started();
    p.ticks(240);
    let sets = [
        ("Wayfarer's Sword", "Pilgrim's Buckler", Capstone::Ember, Capstone::Bastion),
        ("Husk Cleaver", "Warden's Bulwark", Capstone::Rime, Capstone::Sanctuary),
        ("Duelist's Rapier", "Pilgrim's Buckler", Capstone::Thirst, Capstone::Thorns),
        ("Wayfarer's Sword", "Hunter's Shortbow", Capstone::Ember, Capstone::Searing),
        ("Husk Cleaver", "Frostweaver's Focus", Capstone::Rime, Capstone::Winter),
    ];
    for (weapon, off, main_cap, off_cap) in sets {
        let g = p.game();
        let (wi, oi) = (crate::gear::find(weapon), crate::gear::find(off));
        g.world.gear.gain(wi);
        g.world.gear.gain(oi);
        assert!(g.world.gear.wear(0, Some(wi)) && g.world.gear.wear(1, Some(oi)), "{weapon} and {off} would not go on");
        let gear = &mut g.world.gear;
        gear.capstones.clear();
        for p in 0..6 { if let Some(i) = gear.worn[p] { gear.levels.insert(i, crate::gear::ITEMS[i].slot.most()); } }
        let caps = [main_cap, off_cap, Capstone::Stalwart, Capstone::Vigil, Capstone::Fleet, Capstone::Fleet];
        for p in 0..6 { if let Some(i) = gear.worn[p] { gear.capstones.insert(i, caps[p]); } }
        g.fight.hero.stats.kit = g.world.gear.kit();
        for e in g.fight.enemies.iter_mut() { e.ai = crate::fight::Ai::Dead; }
        p.ticks(90);
        let name = format!("cap_{}", main_cap.name().split(' ').next().unwrap_or("x").to_lowercase());
        let tag = off_cap.name().split(' ').next().unwrap_or("x").to_lowercase();
        p.shot(&format!("{name}_{tag}_stand"));
        let class = p.game().fight.hero.stats.kit.arms.class;
        let spec = Move::Light.spec_for(class, None);
        let h = &mut p.game().fight.hero;
        h.act = Some(Act { mv: Move::Light, spec, t: spec.frames.0 + 2, landed: true, from_lane: h.lane, from_x: h.x });
        p.ticks(1);
        p.shot(&format!("{name}_{tag}_swing"));
        p.game().fight.hero.act = None;
    }
}

/// Each road kind as its regions breed it, plain, blighted and dread, side by side in the three
/// lanes: `cargo test --release variant_looks -- --ignored --nocapture` writes target/shots/var_*.png.
#[test]
#[ignore]
fn variant_looks() {
    use crate::fight::{Ai, Archetype, Enemy, Variant};
    let mut p = Play::started();
    p.ticks(240);
    for kind in Archetype::ALL {
        let walked = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
        let at = walked + (crate::fight::HERO_D + 3.2) as f64;
        p.game().fight.enemies = [(-1, Variant::Plain), (0, Variant::Blighted), (1, Variant::Dread)].into_iter()
            .map(|(lane, v)| Enemy { ai: Ai::Wait(u32::MAX), ..Enemy::new(kind, at, lane).of(v) }).collect();
        p.ticks(4);
        p.shot(&format!("var_{kind:?}").to_lowercase());
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

/// On the road: a felled bloater swelling (its warning on the ground) and bursting, then a
/// gravecaller's raise and its risen. `cargo test --release plague_road -- --ignored` writes
/// target/shots/pl_*.png.
#[test]
#[ignore]
fn plague_road() {
    use crate::fight::{Ai, Archetype, Enemy};
    let mut p = Play::started();
    p.ticks(240);
    let walked = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
    let near = walked + (crate::fight::HERO_D + 1.8) as f64;
    let far = walked + (crate::fight::HERO_D + 7.0) as f64;
    let mut bloater = Enemy::new(Archetype::Bloater, near, 0);
    (bloater.hp, bloater.ai, bloater.grave.swell) = (0.0, Ai::Dead, 1);
    let caller = Enemy { ai: Ai::Wait(u32::MAX), ..Enemy::new(Archetype::Gravecaller, far, 1) };
    p.game().fight.enemies = vec![bloater, caller];
    p.game().fight.hero.hp = 1_000.0;
    p.ticks(30);
    p.shot("pl_swell");
    p.ticks(21);
    p.shot("pl_burst");
    // Its raise (the gravecaller's first attack).
    p.game().fight.enemies[1].ai = Ai::Attack { i: 0, t: 0, wide: 0, landed: false, aim: far };
    for _ in 0..200 { if p.game().fight.enemies.len() == 4 { break; } p.ticks(1); }
    assert_eq!(p.game().fight.enemies.len(), 4, "the raise called up two");
    p.ticks(12);
    p.shot("pl_risen");
}

/// The newer kinds side-on: the bandits' hounds sprung in the Bandit Hollows, the brute in the
/// Violet Road's yard, and a bloater and a gravecaller on the Sunken Steps. `cargo test --release new_kinds_yard -- --ignored` writes target/shots/nk_*.png.
#[test]
#[ignore]
fn new_kinds_yard() {
    use crate::game::Mode;
    for (stop, tag) in [("bandit_hollows@ridge_camp", "hounds"), ("violet_road@magistrate", "brute"), ("sunken_steps@bridge_ward", "plague")] {
        let mut p = Play::started();
        p.game().world.warping = Some(stop.into());
        p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: format!("warp:{stop}") });
        assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some(stop)), "never reached {stop}");
        p.ticks(10);
        p.key("w", KeyboardState::Pressed);
        let mut sprung = false;
        for _ in 0..8000 {
            for e in p.game().fight.enemies.iter_mut() { e.ai = crate::fight::Ai::Dead; }
            p.ticks(1);
            if matches!(p.mode(), Mode::Side(_)) { sprung = true; break; }
        }
        p.key("w", KeyboardState::Released);
        assert!(sprung, "the ambush at {stop} never sprang");
        assert!(p.until_mode(400, |m| matches!(m, Mode::Side(a) if a.t >= 140)));
        { let a = arena(&mut p); a.side.hero.hp = 1_000.0; }
        p.ticks(30);
        p.shot(&format!("nk_{tag}_yard"));
        eprintln!("[new_kinds] {stop} foes {:?}", arena(&mut p).side.foes.iter().map(|f| (f.breed.name, f.s)).collect::<Vec<_>>());
        for _ in 0..600 {
            if arena(&mut p).side.foes.iter().any(|f| f.blow().is_some_and(|b| b.1 == crate::fight::Phase::Active)) { break; }
            p.ticks(1);
        }
        p.shot(&format!("nk_{tag}_blow"));
        for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
        p.ticks(30);
        p.shot(&format!("nk_{tag}_fallen"));
    }
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
    // The hero's own first, each as worn in the other hand: a bow's arrow (F), a frost bolt cast
    // through the focus (C), a blood crossbow's bolt (F), away up the lane at a husk.
    let holding = |piece: &str| { let mut l = crate::gear::Loadout::starting(); let i = crate::gear::find(piece); l.gain(i); l.wear(1, Some(i)); l.kit() };
    // Levels enough to hold and cast all of them.
    let levelled = |st: &mut crate::fight::Stats| (st.might, st.dexterity, st.focus) = (4, 3, 3);
    for (piece, key, name) in [("Hunter's Shortbow", "f", "h01_arrow_away"), ("Frostweaver's Focus", "c", "h02_frost_away"), ("Blood Crossbow", "f", "h04_crossbow_away")] {
        road(&mut p, Archetype::Husk, 10.0, 0);
        p.game().fight.enemies[0].ai = crate::fight::Ai::Wait(10_000);
        { let h = &mut p.game().fight.hero; h.stats.kit = holding(piece); levelled(&mut h.stats); *h = crate::fight::Hero { quiver: crate::fight::Quiver::SETTING_OUT, ..crate::fight::Hero::with(h.stats) }; }
        let r = p.fight().hero.stats.kit.shot(key == "c").expect("nothing to shoot");
        p.tap(key);
        p.until(30, |f| f.hero.act.is_some_and(|a| a.t >= 20));
        p.shot(&format!("{name}_draw"));
        assert!(p.until(120, |f| f.bolts.first().is_some_and(|b| b.age >= 10)), "nothing loosed");
        p.shot(name);
        p.until(120, |f| f.bolts.is_empty());
        // Struck by frost, the husk is chilled: frosted over, a third slower.
        if r == crate::gear::Ranged::Frost {
            assert!(p.fight().enemies[0].chill > 0, "the frost bolt did not chill");
            p.ticks(10);
            p.shot("h03_frost_chilled");
        }
    }
    // Blood Imbue through the Bloodrunner's bow (C), then an imbued arrow away, red down its length.
    road(&mut p, Archetype::Husk, 10.0, 0);
    p.game().fight.enemies[0].ai = crate::fight::Ai::Wait(10_000);
    { let h = &mut p.game().fight.hero; h.stats.kit = holding("Bloodrunner's Bow"); levelled(&mut h.stats); *h = crate::fight::Hero { quiver: crate::fight::Quiver::SETTING_OUT, ..crate::fight::Hero::with(h.stats) }; }
    p.tap("c");
    p.until(30, |f| f.hero.act.is_some_and(|a| a.t >= 22));
    p.shot("h05_imbue");
    assert!(p.until(80, |f| f.hero.quiver.imbued > 0 && f.hero.act.is_none()), "nothing imbued");
    p.tap("f");
    assert!(p.until(120, |f| f.bolts.first().is_some_and(|b| b.imbued && b.age >= 10)), "no imbued arrow loosed");
    p.shot("h06_imbued_away");
    p.until(120, |f| f.bolts.is_empty());
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
    // The hero's own, side-on: the bow at full draw, the focus thrust out, the crossbow aimed.
    for (piece, key, name) in [("Hunter's Shortbow", "f", "y05_bow_drawn"), ("Frostweaver's Focus", "c", "y06_focus_cast"), ("Blood Crossbow", "f", "y07_crossbow_aimed")] {
        for f in arena(&mut p).side.foes.iter_mut().filter(|f| f.ai != crate::side::Ai::Dead) { f.ai = crate::side::Ai::Wait(10_000); }
        { let h = &mut arena(&mut p).side.hero; h.stats.kit = holding(piece); levelled(&mut h.stats); (h.quiver, h.charges) = (crate::fight::Quiver { arrows: 12, bolts: 12, imbued: 0 }, 3.0); h.hp = h.stats.max_hp(); }
        // Up off the ground (the hexer's repel floors them) and free.
        p.until_mode(400, |m| matches!(m, Mode::Side(a) if a.side.hero.act.is_none() && a.side.hero.reel == crate::fight::Reel::None));
        p.tap(key);
        let act = if key == "c" { crate::side::Act::Cast } else { crate::side::Act::Loose };
        let at = if piece == "Hunter's Shortbow" { 0.8 } else { 1.02 };
        assert!(p.until_mode(200, |m| matches!(m, Mode::Side(a) if a.side.hero.act.is_some_and(|(x, t)| x == act
            && t as f32 >= at * act.frames(&a.side.hero.stats.kit).0 as f32))), "{name}: never loosed");
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
        g.world.gear.warden = true;
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
    p.tap("s");
    p.tap("e");
    p.skip_prologue();
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
    p.skip_prologue();
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

/// The shrine last rested at is what a pilgrimage carried on wakes at: rested at the Warrens'
/// shrine (walked back from the guild hall, so near the far end), the window closed at once with
/// no autosave since, launched again and carried on, the hero is up beside that shrine.
/// `cargo test --release resume_at_last_shrine -- --ignored --nocapture` writes target/shots/r*.png.
#[test]
#[ignore]
fn resume_at_last_shrine() {
    use crate::game::Mode;
    use crate::profile;
    let mut p = Play::started();
    p.game().keep();
    let dir = p.game().saves.clone();
    assert_eq!(profile::read(&dir, 0).expect("nothing kept").world["checkpoint"].as_str(), Some(crate::world::START));
    // Beside the Warrens' shrine walking back from the hall (sent as a wake would send the hero,
    // the kept shrine left as it was).
    let hall = "warrens@^";
    let go = { let w = &mut p.game().world; w.checkpoint = hall.into(); let go = w.resume(); w.checkpoint = crate::world::START.into(); go };
    let pf = p.canvas.get_plugin_mut::<quartz_path_forge::PathForgePlugin>().unwrap();
    match go { crate::world::Go::WarpAt(s, m) => pf.warp_at(&s, m), crate::world::Go::Warp(s) => pf.warp(&s), _ => unreachable!() }
    assert!(p.walk_until(600, false, |s| s.stop.as_deref() == Some(hall)), "never reached the warrens: {}", p.status());
    p.ticks(30);
    let walked = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
    { let g = p.game(); assert!(g.world.can_rest(&g.fight, walked), "not by the shrine"); }
    // Rest, and read the save the moment the sanctum is sent for.
    p.tap("e");
    let mut rested = false;
    for _ in 0..400 {
        p.ticks(1);
        if matches!(p.mode(), Mode::Cut(_) | Mode::Sanctum(_)) && p.game().world.checkpoint == hall { rested = true; break; }
    }
    assert!(rested, "never rested: {}", p.status());
    let kept = profile::read(&dir, 0).expect("nothing kept");
    assert_eq!(kept.world["checkpoint"].as_str(), Some(hall), "the rest was not kept at once");
    // The window closed: no pause menu, no autosave.
    drop(p);
    let mut p = Play::launched();
    p.ticks(5);
    p.tap("e");
    p.ticks(2);
    p.tap("e");
    p.ticks(2);
    p.tap("e");
    assert!(p.until_mode(400, |m| matches!(m, Mode::Wake(_))), "never woke");
    assert!(p.until_playing(400), "never back on the road");
    p.ticks(20);
    assert_eq!(p.walk_state().stop.as_deref(), Some(hall), "woke somewhere else");
    let walked = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
    let g = p.game();
    assert!(g.world.can_rest(&g.fight, walked), "woke {} m from the shrine", g.world.shrine().unwrap() - walked);
    p.shot("r01_carried_on_at_the_last_shrine");
}

/// Leaving from the pause menu keeps the pilgrimage as it is that moment, without waiting for
/// the autosave's next look: what was done just before is in the save, and the list shows it.
#[test]
#[ignore]
fn pause_keeps() {
    use crate::game::Mode;
    use crate::profile;
    let mut p = Play::started();
    p.game().keep();
    let dir = p.game().saves.clone();
    // Done since the last look, then paused straight away.
    { let g = p.game(); g.world.souls = 4321; g.fight.hero.stats.vigor = 7; }
    // Q is the weapon art on the road, not a pause; Esc pauses.
    p.tap("q");
    p.ticks(2);
    assert!(matches!(p.mode(), Mode::Play), "Q paused the road");
    p.key_as(Key::Named(NamedKey::Escape), KeyboardState::Pressed);
    p.key_as(Key::Named(NamedKey::Escape), KeyboardState::Released);
    p.ticks(2);
    assert!(p.until_mode(10, |m| matches!(m, Mode::Front(crate::shrine::Front::Pause(_)))), "never paused");
    p.shot("q01_paused");
    // Change profile.
    p.tap("s");
    p.tap("e");
    p.ticks(2);
    let saved = profile::read(&dir, 0).expect("nothing kept");
    assert_eq!((saved.world["souls"].as_u64(), saved.stats.vigor), (Some(4321), 7), "the save missed what was done before pausing");
    let list = profile::summaries(&dir);
    assert!(matches!(&list[0], profile::Slot::Kept(s) if s.level == saved.stats.level()), "{list:?}");
    p.shot("q02_list_after_leaving");
}

impl Play {
    fn lesson(&mut self) -> Option<crate::prologue::Lesson> { self.game().prologue.as_ref().map(|p| p.lesson) }
    /// Tick until the prologue is on lesson `l` (at most `n` ticks); whether it got there.
    fn until_lesson(&mut self, n: u32, l: crate::prologue::Lesson) -> bool {
        for _ in 0..n { if self.lesson() == Some(l) { return true; } self.ticks(1); }
        self.lesson() == Some(l)
    }
    /// The husk's blow under way has wound up to `t` ticks.
    fn until_windup(&mut self, t: u32) -> bool { self.until(900, |f| matches!(f.enemies.first().map(|e| e.ai), Some(crate::fight::Ai::Attack { t: at, .. }) if at >= t)) }
    fn until_still(&mut self) -> bool { self.until(400, |f| f.enemies.first().is_none_or(|e| e.attack().is_none())) }
    /// Every banner line shown over the next `n` ticks, in order.
    fn banners_over(&mut self, n: u32) -> Vec<String> {
        let mut told: Vec<String> = Vec::new();
        for _ in 0..n {
            self.ticks(1);
            let line = &self.game().banner.0;
            if !line.is_empty() && told.last() != Some(line) { told.push(line.clone()); }
        }
        told
    }
}

/// The prologue played through by keys, lesson by lesson: the memory, walking, running, lanes,
/// strikes and the weapon's pace, guarding, rolling a sweep and a held blow at the right moment,
/// a parry and its riposte, a flask, the side-on bout, and waking at the first shrine; then a
/// second one left with Esc. `cargo test --release --lib -- --ignored --exact shots::prologue`
/// writes target/shots/t*.png.
#[test]
#[ignore]
fn prologue() {
    use crate::game::Mode;
    use crate::prologue::Lesson::*;
    use prism::event::MouseButton::{Left, Right};
    let mut p = Play::new();
    p.ticks(5);
    p.tap("e");
    p.tap("e");
    p.ticks(2);
    p.shot("t00_asked");
    p.tap("e");
    p.skip_opening();
    assert!(p.until_lesson(900, Memory), "the prologue never began: {}", p.status());
    assert_eq!(p.walk_state().stop.as_deref(), Some(crate::prologue::STOP));
    p.ticks(40);
    p.shot("t01_memory");
    p.tap("e");
    assert!(p.until_lesson(5, Walk), "the card never gave way");
    p.key("w", KeyboardState::Pressed);
    p.ticks(30);
    p.shot("t02_walk");
    assert!(p.until_lesson(600, Run), "never walked: {}", p.status());
    p.key("w", KeyboardState::Released);
    p.ticks(2);
    p.run_on("w");
    p.ticks(30);
    p.shot("t03_run");
    assert!(p.until_lesson(600, Lanes), "never ran: {}", p.status());
    p.key("w", KeyboardState::Released);
    p.tap("a");
    p.ticks(30);
    p.tap("d");
    p.ticks(20);
    p.tap("d");
    assert!(p.until_lesson(200, Strike), "lanes never done: {}", p.status());
    // Up to the husk, then three light blows and a heavy.
    p.key("w", KeyboardState::Pressed);
    p.ticks(260);
    p.key("w", KeyboardState::Released);
    p.shot("t04_strike");
    for k in 0..4 {
        p.click(Left, k == 3);
        p.ticks(if k == 3 { 90 } else { 45 });
    }
    assert!(p.until_lesson(200, Pace), "strikes never counted: {} {:?}", p.status(), p.game().prologue);
    p.shot("t05_pace");
    p.tap("e");
    // Guard through two chops.
    p.mouse(Right, MouseState::Pressed);
    assert!(p.until_windup(30), "never swung for the guard lesson: {}", p.status());
    p.shot("t06_guard");
    assert!(p.until_lesson(1200, Roll), "never blocked twice: {} {:?}", p.status(), p.game().prologue);
    p.mouse(Right, MouseState::Released);
    p.until_still();
    // Rolls made as the sweep's mark fills (50-tick windup), left and right in turn.
    for k in 0..4 {
        if p.lesson() != Some(Roll) { break; }
        assert!(p.until_windup(44), "never swept: {}", p.status());
        if k == 0 { p.shot("t07_roll_mark"); }
        p.roll(if k % 2 == 0 { "d" } else { "a" });
        p.ticks(4);
        if k == 0 { p.shot("t07b_rolling"); }
        p.until_still();
    }
    assert!(p.until_lesson(200, Late), "never rolled clear: {:?}", p.game().prologue);
    // The held blow (74 ticks): too soon once, then at the glint.
    assert!(p.until_windup(30), "never held a blow: {}", p.status());
    p.roll("a");
    p.until_still();
    p.ticks(2);
    p.shot("t08_too_soon");
    for k in 0..3 {
        if p.lesson() != Some(Late) { break; }
        assert!(p.until_windup(68), "never held a blow: {}", p.status());
        p.roll(if k % 2 == 0 { "d" } else { "a" });
        p.until_still();
    }
    assert!(p.until_lesson(200, Parry), "never rolled the held blow: {:?}", p.game().prologue);
    // A parry as the chop's mark fills (42-tick windup), then the riposte.
    for _ in 0..4 {
        for _ in 0..900 {
            if p.lesson() != Some(Parry) || matches!(p.fight().enemies[0].ai, crate::fight::Ai::Attack { t, .. } if t >= 34) { break; }
            p.ticks(1);
        }
        if p.lesson() != Some(Parry) { break; }
        p.click(Right, true);
        p.ticks(10);
        p.shot("t09_parried");
        p.click(Left, false);
        p.ticks(60);
        p.until_still();
    }
    assert!(p.until_lesson(200, Flask), "never parried and riposted: {:?}", p.game().prologue);
    p.shot("t10_flask");
    p.tap("r");
    assert!(p.until_lesson(300, Turn), "never drank: {:?}", p.game().prologue);
    p.shot("t11_turn");
    p.tap("e");
    assert!(p.until_mode(60, |m| matches!(m, Mode::Side(_))), "the bout never began");
    p.ticks(170);
    p.shot("t12_side");
    if let Mode::Side(a) = &mut p.game().mode { for f in &mut a.side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); } }
    assert!(p.until_lesson(400, End), "the bout never ended: {}", p.status());
    p.ticks(20);
    p.shot("t13_end");
    p.tap("e");
    // The memory's road is not announced on the way out: the first word is the road itself.
    let told = p.banners_over(900);
    assert_eq!(told.first().map(String::as_str), Some("The Ashen Warrens"), "{told:?}");
    assert!(p.until_mode(900, |m| matches!(m, Mode::Wake(_) | Mode::Play)), "never woke: {}", p.status());
    assert!(p.game().prologue.is_none());
    assert_eq!(p.walk_state().stop.as_deref(), Some(crate::world::START), "woke on the prologue's road");
    p.ticks(40);
    p.shot("t14_awake");
    assert!(p.until_playing(300));
    let h = &p.fight().hero;
    assert_eq!((h.hp, h.flasks), (h.stats.max_hp(), crate::fight::FLASKS), "set out whole");
    drop(p);

    // A second, left with Esc: asked first, then the pilgrimage at the first shrine.
    let mut p = Play::new();
    p.ticks(5);
    p.tap("e");
    p.tap("e");
    p.tap("e");
    p.skip_opening();
    assert!(p.until_lesson(900, Memory), "the prologue never began");
    p.tap("e");
    assert!(p.until_lesson(5, Walk));
    p.key_as(Key::Named(NamedKey::Escape), KeyboardState::Pressed);
    p.ticks(2);
    p.shot("t15_leave_asked");
    assert!(p.game().prologue.as_ref().is_some_and(|x| x.leaving));
    p.tap("e");
    // The memory's road is not announced on the way out: the first word is the road itself.
    let told = p.banners_over(900);
    assert_eq!(told.first().map(String::as_str), Some("The Ashen Warrens"), "{told:?}");
    assert!(p.until_mode(900, |m| matches!(m, Mode::Wake(_) | Mode::Play)), "never set out: {}", p.status());
    assert!(p.game().prologue.is_none());
    assert_eq!(p.walk_state().stop.as_deref(), Some(crate::world::START));
}

/// Enemies on the road before they notice the hero: pacing across it, watching from its edge,
/// each on its own time; then walking up until the nearest notices and squares up to its lane.
/// `cargo test --release --lib -- --ignored --exact shots::idle` writes target/shots/i*.png.
#[test]
#[ignore]
fn idle() {
    let mut p = Play::started();
    let at = |p: &Play| p.fight().enemies.iter().map(|e| format!("{:?} lane {} drift {:.2} {:?}", e.kind, e.lane, e.drift, e.idle.map(|i| i.way))).collect::<Vec<_>>().join("; ");
    p.ticks(20);
    p.shot("i01_idle");
    let first = at(&p);
    p.ticks(150);
    p.shot("i02_idle_later");
    let later = at(&p);
    assert_ne!(first, later, "nothing moved while idling");
    assert!(p.fight().enemies.iter().any(|e| e.idle.is_some()), "every enemy already alert: {later}");
    // Up the road until the nearest notices.
    p.key("w", KeyboardState::Pressed);
    let noticed = p.until(900, |f| f.enemies.iter().any(|e| e.alive() && e.idle.is_none()));
    p.key("w", KeyboardState::Released);
    assert!(noticed, "none noticed the hero: {}", at(&p));
    p.ticks(6);
    p.shot("i03_noticed");
    eprintln!("[idle] {first}\n[idle] {later}\n[idle] {}", at(&p));
}

/// Running by: a lane over and at a run past the road's first husk, a late step aside when it
/// cuts across; then standing, so it catches up and swings from behind, warned of at the
/// bottom of the screen.
#[test]
#[ignore]
fn run_by() {
    let mut p = Play::started();
    p.ticks(30);
    let walked = |p: &Play| p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
    p.tap("d");
    p.ticks(20);
    p.run_on("w");
    let mut stepped = 0;
    let mut past = false;
    for _ in 0..900 {
        let w = walked(&p);
        let (gap, lane, hero) = (p.fight().enemies[0].gap(w), p.fight().enemies[0].lane, p.fight().hero.lane);
        if gap < -1.0 { past = true; break; }
        // It steps across into the hero's lane: aside again, late.
        if lane == hero && (0.0..4.0).contains(&gap) && p.fight().hero.act.is_none() {
            p.tap(if hero <= 0 { "d" } else { "a" });
            stepped += 1;
            if stepped == 1 { p.ticks(2); p.shot("x01_cuts_across"); }
        }
        p.ticks(1);
    }
    eprintln!("[run_by] past {past} after {stepped} steps aside: {}", p.status());
    assert!(past, "never got past: {}", p.status());
    p.ticks(10);
    p.shot("x02_past");
    p.key("w", KeyboardState::Released);
    // Standing: it comes after the hero, and the warning rises behind them.
    p.ticks(30);
    let w0 = walked(&p);
    assert!(p.until(400, move |f| f.enemies[0].chasing(w0)), "no chase: {}", p.status());
    assert!(p.until(600, move |f| f.chasers(w0).first().is_some_and(|c| c.blow.is_some_and(|b| b.2 > 0.5))), "no blow from behind: {}", p.status());
    p.shot("x03_behind_windup");
    let hp = p.fight().hero.hp;
    p.until(120, move |f| f.hero.hp < hp);
    p.ticks(4);
    p.shot("x04_struck_from_behind");
    let now = p.fight().hero.hp;
    eprintln!("[run_by] hp {hp} -> {now}: {}", p.status());
    assert!(now < hp, "the blow from behind missed");
}

impl Play {
    /// `secs` of a cutscene at about the pace it plays in a window (the road's worker renders in
    /// real time, so frames taken at the capture's usual pace would lag their shots).
    fn watch(&mut self, secs: f32) {
        for _ in 0..(secs * 60.0) as u32 { self.ticks(1); std::thread::sleep(std::time::Duration::from_millis(10)); }
    }
    fn scene_at(&self) -> Option<(&'static str, usize, u32)> {
        self.canvas.get_plugin::<crate::game::Pilgrimage>().unwrap().scene_at()
    }
}

/// The Warrens fork taken toward the Guild Hall, shot across the turn (each in a pair of frames
/// back to back, so a wall fighting another shows as the pair differing), for the inner corner.
/// `cargo test --release --lib -- --ignored --exact shots::fork_corner --nocapture` writes target/shots/k*.png.
#[test]
#[ignore]
fn fork_corner() {
    let mut p = Play::started();
    p.game().world.warping = Some("warrens@warrens_lane".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:warrens@warrens_lane".into() });
    p.ticks(120);
    p.run_on("w");
    let (mut took, mut n, mut last) = (false, 0, f64::MIN);
    for _ in 0..6000 {
        let s = p.walk_state();
        if !took && s.choose_within.is_some_and(|m| m < 6.0) { p.tap("d"); took = true; }
        p.fell_all();
        p.ticks(1);
        let s = p.walk_state();
        let near = s.choose_within.is_some_and(|m| m < 16.0) || (took && s.walked - last < 40.0);
        if near && s.walked - last >= 2.5 {
            let img = |p: &Play| p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().and_then(|f| f.current_image()).map_or(0, |a| std::sync::Arc::as_ptr(a) as usize);
            p.shot(&format!("k{n:02}a"));
            let first = img(&p);
            for _ in 0..30 { if img(&p) != first { break; } std::thread::sleep(std::time::Duration::from_millis(5)); p.ticks(1); }
            p.shot(&format!("k{n:02}b"));
            eprintln!("[fork] k{n:02} walked {:.1} {}", s.walked, p.status());
            (n, last) = (n + 1, s.walked);
            if n >= 14 { break; }
        }
    }
    assert!(n > 4, "never came to the fork: {}", p.status());
}

/// Where the road stalls between the Warrens and the Guild Hall: walked at a window's pace, the
/// slowest ticks (the game's own work) and the longest waits for a new road frame, each with
/// the stop it was at. `cargo test --release --lib -- --ignored --exact shots::probe_lag --nocapture`
#[test]
#[ignore]
fn probe_lag() {
    let mut p = Play::started();
    p.game().world.warping = Some("warrens@warrens_lane".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:warrens@warrens_lane".into() });
    p.ticks(120);
    p.run_on("w");
    let (mut ticks, mut gaps) = (Vec::new(), Vec::new());
    let (mut last_img, mut last_new) = (0usize, std::time::Instant::now());
    let mut took = false;
    for i in 0..5400 {
        let s = p.walk_state();
        let at = format!("{} {:.2}", s.stop.as_deref().unwrap_or("-"), s.progress.unwrap_or(-1.0));
        if !took && s.choose_within.is_some_and(|m| m < 6.0) { p.tap("d"); took = true; }
        p.fell_all();
        let t0 = std::time::Instant::now();
        p.ticks(1);
        ticks.push((t0.elapsed().as_secs_f32() * 1000.0 - 6.0, i, at.clone()));
        let img = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().and_then(|f| f.current_image()).map_or(0, |a| std::sync::Arc::as_ptr(a) as usize);
        if img != last_img {
            let rms = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().map_or(0.0, |f| f.render_ms());
            gaps.push((last_new.elapsed().as_secs_f32() * 1000.0, i, format!("{at} render {rms:.1}")));
            (last_img, last_new) = (img, std::time::Instant::now());
        }
    }
    // Per stretch of road: the median and 95th percentile wait for a frame.
    for part in ["warrens@warrens_lane", "~guild_hall@warrens", "guild_hall@warrens"] {
        let mut w: Vec<f32> = gaps.iter().filter(|g| g.2.starts_with(&format!("{part} "))).map(|g| g.0).collect();
        w.sort_by(f32::total_cmp);
        if w.is_empty() { continue; }
        eprintln!("[lag] {part}: {} frames, median {:.1} ms, p95 {:.1} ms", w.len(), w[w.len() / 2], w[w.len() * 95 / 100]);
    }
    ticks.sort_by(|a, b| b.0.total_cmp(&a.0));
    gaps.sort_by(|a, b| b.0.total_cmp(&a.0));
    eprintln!("[lag] ticks {} frames {} — slowest ticks (ms over the 6 ms pause):", ticks.len(), gaps.len());
    for t in ticks.iter().take(12) { eprintln!("[lag]   tick {:>5.1} at {} ({})", t.0, t.1, t.2); }
    eprintln!("[lag] longest waits for a road frame:");
    for g in gaps.iter().take(12) { eprintln!("[lag]   wait {:>5.1} at {} ({})", g.0, g.1, g.2); }
    eprintln!("[lag] end {}", p.status());
}

/// The opening, before a first pilgrimage: the Spire at dusk, the angels' ruins, the marsh at
/// night, Hollowmere and Varnholt as rumour, the hero on the road at the first shrine, the Warrens,
/// the title over the guild hall; then the road at the first shrine.
/// `cargo test --release --lib -- --ignored --exact shots::opening` writes target/shots/c0*.png.
#[test]
#[ignore]
fn opening() {
    use crate::game::Mode;
    let mut p = Play::new();
    p.ticks(5);
    p.tap("e");
    p.tap("e");
    p.tap("s");
    p.tap("e");
    for _ in 0..200 { if p.scene_at().is_some() { break; } p.ticks(1); }
    assert_eq!(p.scene_at().map(|s| s.0), Some("opening"), "no opening: {}", p.status());
    let cuts = [(4.5, "c01_spire"), (8.5, "c02_the_eye"), (7.5, "c03_the_guardians"), (7.0, "c04_the_tear"), (7.5, "c05_hollowmere"),
        (6.0, "c06_varnholt"), (6.0, "c07_rise_whole"), (5.0, "c08_waking"), (3.5, "c09_you"), (5.5, "c10_the_warrens"), (6.0, "c11_title")];
    for (secs, name) in cuts {
        p.watch(secs);
        eprintln!("[opening] {name} at {:?}", p.scene_at());
        p.shot(name);
    }
    assert!(p.until_mode(900, |m| matches!(m, Mode::Wake(_))), "never woke on the road: {}", p.status());
    assert!(p.scene_at().is_none());
    p.ticks(60);
    p.shot("c12_road");
}

/// The Screamer speaks into the head as it is named, and its last words as it falls, before what
/// it leaves is told. `cargo test --release --lib -- --ignored --exact shots::screamer_words`.
#[test]
#[ignore]
fn screamer_words() {
    use crate::game::Mode;
    let mut p = Play::started();
    p.game().world.warping = Some("hollowmere@gate".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:hollowmere@gate".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("hollowmere@gate")), "never reached Hollowmere");
    p.ticks(10);
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 { p.fell_all(); p.ticks(1); p.step_through_veil(); if matches!(p.mode(), Mode::Side(_)) { break; } }
    p.key("w", KeyboardState::Released);
    assert_eq!(p.scene_at().map(|s| s.0), Some("entrance"), "no entrance: {}", p.status());
    p.watch(3.6);
    p.shot("w01_screamer_whispers");
    for _ in 0..240 { if p.scene_at().is_none() { break; } p.ticks(1); }
    for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    p.ticks(40);
    p.shot("w02_screamer_last_words");
    assert!(p.until_mode(400, |m| matches!(m, Mode::Play)), "the yard never let go");
    p.ticks(10);
    p.shot("w03_screamer_falls");
    // What it leaves, told after, one line at a time.
    p.ticks(200);
    p.shot("w04_the_rune");
    p.ticks(200);
    p.shot("w05_the_remnant");
}

/// The East Gate both ways. Gone out early: the Keening Cage named and felled, its bone carried
/// off, and once the story reaches it Olekeen comes for his bounty and finds it dead. In the
/// story's order: Olekeen over his own kill. `cargo test --release --lib -- --ignored --exact
/// shots::east_gate` writes target/shots/x*.png.
#[test]
#[ignore]
fn east_gate() {
    use crate::game::Mode;
    fn to_gate(p: &mut Play) {
        p.game().world.warping = Some("east_gate@warrens_lane".into());
        p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:east_gate@warrens_lane".into() });
        assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("east_gate@warrens_lane")), "never reached the east gate");
    }
    fn to_olekeen(p: &mut Play, name: &str) {
        p.key("w", KeyboardState::Pressed);
        let near = |p: &mut Play| { let walked = p.walk_state().walked; let g = p.game(); g.world.can_meet_visitor(&g.fight, walked) };
        for _ in 0..3000 { p.fell_all(); p.ticks(1); if near(p) { break; } }
        p.key("w", KeyboardState::Released);
        assert!(near(p), "Olekeen is out of reach: {}", p.status());
        p.ticks(20);
        p.shot(&format!("{name}_olekeen"));
        p.tap("e");
        p.ticks(4);
        for k in 0..5 { p.shot(&format!("{name}_olekeen_{k}")); p.tap("e"); p.ticks(4); }
        assert!(matches!(p.mode(), Mode::Play), "still talking: {}", p.status());
    }
    let mut p = Play::started();
    to_gate(&mut p);
    p.ticks(30);
    p.shot("x01_east_gate");
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 { p.fell_all(); p.ticks(1); p.step_through_veil(); if matches!(p.mode(), Mode::Side(_)) { break; } }
    p.key("w", KeyboardState::Released);
    assert_eq!(p.scene_at().map(|s| s.0), Some("entrance"), "no entrance: {}", p.status());
    p.watch(4.0);
    p.shot("x02_the_keening_cage");
    for _ in 0..240 { if p.scene_at().is_none() { break; } p.ticks(1); }
    p.ticks(90);
    p.shot("x03_the_keening_cage_fights");
    for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    assert!(p.until_mode(400, |m| matches!(m, Mode::Play)), "the yard never let go");
    assert!(p.game().world.remnants.contains(&"Violet-Veined Bone"));
    // The story reaches the gate: Gruk pays the Hollowmere bounty, and Olekeen comes too late.
    { let w = &mut p.game().world; w.guild.posted = 1; w.guild.paid = 1; w.story_moves(); }
    to_gate(&mut p);
    to_olekeen(&mut p, "x04_early");
    // In the story's order: the beast is his, and he is still standing over it.
    let mut p = Play::started();
    { let w = &mut p.game().world; w.guild.posted = 1; w.guild.paid = 1; w.story_moves(); }
    to_gate(&mut p);
    to_olekeen(&mut p, "x05_late");
    assert!(!matches!(p.mode(), Mode::Side(_)), "no beast to fight");
}

/// Down the side path off the Bandit Hollows into the crew's hole under the ridge, before their
/// chief has fallen: the stash's guards spring at the chest, and the Caravan Signet is in it.
/// `cargo test --release --lib -- --ignored --exact shots::bandit_cave` writes target/shots/bc*.png.
#[test]
#[ignore]
fn bandit_cave() {
    use crate::game::Mode;
    let mut p = Play::started();
    p.game().world.warping = Some("bandit_hollows@ridge_camp".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:bandit_hollows@ridge_camp".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("bandit_hollows@ridge_camp")), "never reached the hollows");
    p.ticks(10);
    p.key("w", KeyboardState::Pressed);
    let mut mouth = false;
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        let st = p.walk_state();
        if st.side_paths.iter().any(|(d, _)| (d - crate::fight::HERO_D).abs() < crate::world::SIDE_REACH * 0.5) { mouth = true; break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(mouth, "no side path opened: {}", p.status());
    p.ticks(4);
    p.shot("bc01_mouth");
    p.tap("e");
    assert!(p.until_mode(20, |m| matches!(m, Mode::Peek(..))), "E did not turn down it: {:?}", p.mode());
    assert!(p.walk_until(1200, false, |s| s.stop.as_deref() == Some("bandit_cave")), "never reached the cave");
    assert!(p.until_playing(400));
    p.ticks(30);
    p.shot("bc02_the_bandits_hole");
    p.key("w", KeyboardState::Pressed);
    let mut trap = false;
    for _ in 0..6000 {
        if !trap && matches!(p.mode(), Mode::Side(_)) {
            trap = true;
            p.key("w", KeyboardState::Released);
            p.until_mode(300, |m| matches!(m, Mode::Side(a) if a.t >= 170));
            p.shot("bc03_the_stash_guarded");
            p.key("w", KeyboardState::Pressed);
        }
        p.fell_all();
        p.ticks(1);
        if p.game().world.cache_taken() { break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(trap, "the stash's guards never sprang");
    assert!(p.until_playing(400));
    p.ticks(20);
    p.shot("bc04_the_signet");
    let w = &p.game().world;
    assert!(w.gear.owns(crate::gear::find("Caravan Signet")) && w.stash_taken_by_hero() == Some(true));
}

/// Past the East Gate into the black water, early: the Mother named (in a drowned woman's voice)
/// and felled, Mara's whistle found among what she kept, and Krix at the guild knowing it.
/// `cargo test --release --lib -- --ignored --exact shots::black_water` writes target/shots/bw*.png.
#[test]
#[ignore]
fn black_water() {
    use crate::game::Mode;
    let mut p = Play::started();
    p.game().world.beaten.extend(["east_gate", "guild_hall"]);
    p.game().world.warping = Some("black_water@east_gate".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:black_water@east_gate".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("black_water@east_gate")), "never reached the black water");
    p.ticks(30);
    p.shot("bw01_black_water");
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 { p.fell_all(); p.ticks(1); p.step_through_veil(); if matches!(p.mode(), Mode::Side(_)) { break; } }
    p.key("w", KeyboardState::Released);
    assert_eq!(p.scene_at().map(|s| s.0), Some("entrance"), "no entrance: {}", p.status());
    p.watch(2.0);
    p.shot("bw02_the_mother_named");
    p.watch(2.0);
    p.shot("bw03_her_voice");
    for _ in 0..240 { if p.scene_at().is_none() { break; } p.ticks(1); }
    p.ticks(90);
    p.shot("bw04_the_mother_fights");
    for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    p.ticks(30);
    p.shot("bw05_last_words");
    assert!(p.until_mode(400, |m| matches!(m, Mode::Play)), "the yard never let go");
    assert!(p.game().world.remnants.contains(&"The Mother's Hooked Finger"));
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 { p.fell_all(); p.ticks(1); if p.game().world.cache_taken() { break; } }
    p.key("w", KeyboardState::Released);
    assert!(p.game().world.cache_taken(), "Mara's whistle was never found");
    p.ticks(20);
    p.shot("bw06_maras_whistle");
    // Krix, at the guild, sees what was brought back.
    p.game().world.warping = Some("guild_hall@warrens".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:guild_hall@warrens".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("guild_hall@warrens")), "never reached the guild hall");
    p.key("w", KeyboardState::Pressed);
    let near = |p: &mut Play| { let walked = p.walk_state().walked; let g = p.game(); g.world.can_meet_visitor(&g.fight, walked) };
    for _ in 0..3000 { p.fell_all(); p.ticks(1); if near(&mut p) { break; } }
    p.key("w", KeyboardState::Released);
    assert!(near(&mut p), "Krix is out of reach: {}", p.status());
    p.ticks(20);
    p.shot("bw07_krix");
    p.tap("e");
    p.ticks(4);
    for k in 0..5 { p.shot(&format!("bw08_krix_{k}")); p.tap("e"); p.ticks(4); }
    assert!(matches!(p.mode(), Mode::Play), "still talking: {}", p.status());
    assert!(p.game().world.meetings.contains(&"krix_beaten_whistle"));
}

/// The fog hollow off the Bandit Hollows once the chief is down: Lurz at its mouth, the horror
/// named and felled with no fall, and Krix telling it after, both hunters alive; then Lurz at
/// Stonehollow's market with his spear.
/// `cargo test --release --lib -- --ignored --exact shots::fog_hollow` writes target/shots/fh*.png.
#[test]
#[ignore]
fn fog_hollow() {
    use crate::game::Mode;
    let mut p = Play::started();
    p.game().world.beaten.extend(["chief_hold"]);
    p.game().world.warping = Some("fog_hollow@bandit_hollows".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:fog_hollow@bandit_hollows".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("fog_hollow@bandit_hollows")), "never reached the fog hollow");
    p.ticks(30);
    p.shot("fh01_fog_hollow");
    let near = |p: &mut Play| { let walked = p.walk_state().walked; let g = p.game(); g.world.can_meet_visitor(&g.fight, walked) };
    p.key("w", KeyboardState::Pressed);
    for _ in 0..3000 { p.fell_all(); p.ticks(1); if near(&mut p) { break; } }
    p.key("w", KeyboardState::Released);
    assert!(near(&mut p), "Lurz is out of reach: {}", p.status());
    p.ticks(20);
    p.shot("fh02_lurz");
    p.tap("e");
    p.ticks(4);
    for _ in 0..5 { p.tap("e"); p.ticks(4); }
    assert!(matches!(p.mode(), Mode::Play), "still talking: {}", p.status());
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 { p.fell_all(); p.ticks(1); p.step_through_veil(); if matches!(p.mode(), Mode::Side(_)) { break; } }
    p.key("w", KeyboardState::Released);
    assert_eq!(p.scene_at().map(|s| s.0), Some("entrance"), "no entrance: {}", p.status());
    p.watch(2.0);
    p.shot("fh03_the_horror_named");
    for _ in 0..240 { if p.scene_at().is_none() { break; } p.ticks(1); }
    p.ticks(90);
    p.shot("fh04_the_horror_fights");
    for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    p.ticks(30);
    p.shot("fh05_last_words");
    assert!(p.until_mode(400, |m| matches!(m, Mode::Play)), "the yard never let go");
    assert!(p.game().world.remnants.contains(&"The Horror's Eye-Crack"));
    assert_eq!(p.game().world.hunters, crate::hunters::Hunters { lost: 0, settled: true });
    p.key("w", KeyboardState::Pressed);
    for _ in 0..1200 { p.ticks(1); if near(&mut p) { break; } }
    p.key("w", KeyboardState::Released);
    assert!(near(&mut p), "Krix never came out of the fog: {}", p.status());
    p.ticks(150);
    p.shot("fh06_krix_after");
    p.tap("e");
    p.ticks(4);
    p.shot("fh07_krix_words");
    for _ in 0..8 { if matches!(p.mode(), Mode::Play) { break; } p.tap("e"); p.ticks(4); }
    assert!(p.game().world.meetings.contains(&"hollow_both_live"));
    p.game().world.warping = Some("stone_market@lower_stairs".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:stone_market@lower_stairs".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("stone_market@lower_stairs")), "never reached the market");
    p.key("w", KeyboardState::Pressed);
    for _ in 0..3000 { p.fell_all(); p.ticks(1); if near(&mut p) { break; } }
    p.key("w", KeyboardState::Released);
    assert!(near(&mut p), "Lurz is out of reach at the market: {}", p.status());
    p.ticks(20);
    p.shot("fh08_lurz_at_the_market");
    p.tap("e");
    for _ in 0..6 { p.ticks(4); p.tap("e"); }
    p.ticks(10);
    assert!(p.game().world.gear.owns(crate::gear::find("Lurz's Spear")), "the spear was never given");
}

/// Tusk and Vren once the hero carries the rune and the shaken hunter found the blade first: Tusk
/// at his Warrens cart, then down the Warrens' side path to the Broken Spire, where Vren shows it.
/// `cargo test --release --lib -- --ignored --exact shots::tusk_and_vren` writes target/shots/tv*.png.
#[test]
#[ignore]
fn tusk_and_vren() {
    use crate::game::Mode;
    let mut p = Play::started();
    { let w = &mut p.game().world; w.keys.push(crate::atlas::Key::Rune); w.guild.paid = 1; w.story_moves(); }
    p.game().world.warping = Some("warrens@warrens_lane".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:warrens@warrens_lane".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("warrens@warrens_lane")), "never reached the Warrens");
    let near = |p: &mut Play| { let walked = p.walk_state().walked; let g = p.game(); g.world.can_meet_visitor(&g.fight, walked) };
    p.key("w", KeyboardState::Pressed);
    for _ in 0..3000 { p.fell_all(); p.ticks(1); if near(&mut p) { break; } }
    p.key("w", KeyboardState::Released);
    assert!(near(&mut p), "Tusk is out of reach: {}", p.status());
    p.ticks(20);
    p.shot("tv01_tusk");
    p.tap("e");
    p.ticks(4);
    p.shot("tv02_tusk_words");
    for _ in 0..8 { if matches!(p.mode(), Mode::Play) { break; } p.tap("e"); p.ticks(4); }
    assert!(p.game().world.meetings.contains(&"tusk_sold_it"));
    // Back the way it was walked to the side path's mouth: turned round, down the street.
    p.game().world.warping = Some("warrens@warrens_lane".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:warrens@warrens_lane".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("warrens@warrens_lane")), "never back in the Warrens");
    p.key("w", KeyboardState::Pressed);
    let mut mouth = false;
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        let st = p.walk_state();
        if st.side_paths.iter().any(|(d, _)| (d - crate::fight::HERO_D).abs() < crate::world::SIDE_REACH * 0.5) { mouth = true; break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(mouth, "no side path opened: {}", p.status());
    p.tap("e");
    assert!(p.until_mode(20, |m| matches!(m, Mode::Peek(..))), "E did not turn down it: {:?}", p.mode());
    assert!(p.walk_until(1200, false, |s| s.stop.as_deref() == Some("broken_spire")), "never reached the Broken Spire");
    assert!(p.until_playing(400));
    p.ticks(30);
    p.shot("tv03_broken_spire");
    p.key("w", KeyboardState::Pressed);
    for _ in 0..3000 { p.fell_all(); p.ticks(1); if near(&mut p) { break; } }
    p.key("w", KeyboardState::Released);
    assert!(near(&mut p), "Vren is out of reach: {}", p.status());
    p.ticks(20);
    p.shot("tv04_vren");
    p.tap("e");
    p.ticks(4);
    p.tap("e");
    p.ticks(4);
    p.tap("e");
    p.ticks(4);
    p.shot("tv05_vren_a_call");
    for _ in 0..8 { if matches!(p.mode(), Mode::Play) { break; } p.tap("e"); p.ticks(4); }
    assert!(p.game().world.meetings.contains(&"vren_shows_blade"));
}

/// The Taint echoes of Krix and Lurz, for a hero the fog hollow cost both: Lurz's on the Violet
/// Road, named, fought and felled for his spear; Krix's at the Blackwood's edge, named.
/// `cargo test --release --lib -- --ignored --exact shots::taint_echoes` writes target/shots/te*.png.
#[test]
#[ignore]
fn taint_echoes() {
    use crate::game::Mode;
    let mut p = Play::started();
    { let w = &mut p.game().world; w.beaten.extend(["chief_hold", "fog_hollow", "magistrate"]); w.hunters = crate::hunters::Hunters { lost: 2, settled: true }; }
    for (stop, tag) in [("violet_road@magistrate", "te0"), ("blackwood@cathedral", "te1")] {
        p.game().world.warping = Some(stop.into());
        p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: format!("warp:{stop}") });
        assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some(stop)), "never reached {stop}");
        p.key("w", KeyboardState::Pressed);
        let duel = |p: &mut Play| matches!(p.mode(), Mode::Side(a) if a.why == crate::game::Why::Boss);
        for _ in 0..12000 {
            p.fell_all();
            p.ticks(1);
            p.step_through_veil();
            if duel(&mut p) { break; }
            // The road's own ambushes are not what this is about.
            if matches!(p.mode(), Mode::Side(_)) { for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); } }
        }
        p.key("w", KeyboardState::Released);
        assert!(duel(&mut p), "no duel at {stop}: {}", p.status());
        p.watch(2.0);
        p.shot(&format!("{tag}1_named"));
        for _ in 0..240 { if p.scene_at().is_none() { break; } p.ticks(1); }
        p.ticks(60);
        p.shot(&format!("{tag}2_fights"));
        for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
        p.ticks(30);
        p.shot(&format!("{tag}3_last_words"));
        assert!(p.until_mode(400, |m| matches!(m, Mode::Play)), "the yard never let go at {stop}");
    }
    let w = &p.game().world;
    assert!(w.gear.owns(crate::gear::find("Lurz's Spear")) && w.gear.owns(crate::gear::find("Krix's Notched Axe")), "the echoes left nothing");
}

/// A boss's entrance: the swing round to the gate's yard, the Bell Warden's toll as the view
/// closes on it, its name, and the view opening back out for the fight.
/// `cargo test --release --lib -- --ignored --exact shots::entrance` writes target/shots/e0*.png.
#[test]
#[ignore]
fn entrance() {
    use crate::game::Mode;
    let mut p = Play::started();
    p.game().world.warping = Some("gate@vale".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:gate@vale".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("gate@vale")), "never reached the gate");
    p.ticks(10);
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 { p.fell_all(); p.ticks(1); p.step_through_veil(); if matches!(p.mode(), Mode::Side(_)) { break; } }
    p.key("w", KeyboardState::Released);
    assert_eq!(p.scene_at().map(|s| s.0), Some("entrance"), "no entrance: {}", p.status());
    for (secs, name) in [(1.6, "e01_swing"), (1.3, "e02_bars"), (1.3, "e03_toll"), (0.9, "e04_closer"), (1.8, "e05_named"), (2.2, "e06_open")] {
        p.watch(secs);
        eprintln!("[entrance] {name} at {:?}", p.scene_at());
        p.shot(name);
    }
    for _ in 0..120 { if p.scene_at().is_none() { break; } p.ticks(1); }
    assert!(p.scene_at().is_none(), "the entrance never ended");
    // The fight goes on from where it stood.
    let s = arena(&mut p).side.foes[0].s;
    p.ticks(120);
    assert!(arena(&mut p).side.foes[0].s != s || !matches!(arena(&mut p).side.foes[0].ai, crate::side::Ai::Wait(_)), "the Warden never moved");
    p.shot("e07_fight");
}

/// The Archangel falls: slow, a flash, the view closing on it into white; out of the white the
/// Eye made whole and the trap at the Summit, and waking by the Sealed Slab's fire (the last act
/// and its endings are `finale`). `cargo test --release --lib -- --ignored --exact shots::ending`
/// writes target/shots/z0*.png.
#[test]
#[ignore]
fn ending() {
    use crate::game::Mode;
    let mut p = Play::started();
    p.game().world.warping = Some("spire_summit@sealed_slab".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:spire_summit@sealed_slab".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("spire_summit@sealed_slab")), "never reached the summit");
    p.ticks(10);
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 { p.fell_all(); p.ticks(1); p.step_through_veil(); if matches!(p.mode(), Mode::Side(_)) { break; } }
    p.key("w", KeyboardState::Released);
    assert_eq!(arena(&mut p).side.foes[0].breed.name, "The Archangel");
    p.watch(2.4);
    p.skip_scene();
    p.until_mode(200, |m| matches!(m, Mode::Side(a) if a.t >= 140));
    p.ticks(30);
    { let f = &mut arena(&mut p).side.foes[0]; (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    p.ticks(3);
    assert_eq!(p.scene_at().map(|s| s.0), Some("fall"), "no fall: {}", p.status());
    for (secs, name) in [(0.4, "z01_flash"), (2.6, "z02_falling"), (3.2, "z03_white"), (2.6, "z04_whole")] {
        p.watch(secs);
        eprintln!("[ending] {name} at {:?}", p.scene_at());
        p.shot(name);
    }
    for _ in 0..600 { if p.scene_at().is_some_and(|s| s.0 == "trap") { break; } p.ticks(1); }
    assert_eq!(p.scene_at().map(|s| s.0), Some("trap"), "no trap after the fall: {}", p.status());
    for (secs, name) in [(2.6, "z05_whole"), (6.0, "z06_voices"), (8.0, "z07_olekeen"), (6.0, "z08_exodus"), (6.5, "z09_gas")] {
        p.watch(secs);
        eprintln!("[ending] {name} at {:?}", p.scene_at());
        p.shot(name);
    }
    assert!(p.until_mode(2400, |m| matches!(m, Mode::Wake(_))), "never woke: {}", p.status());
    assert_eq!((p.game().world.pilgrimages, p.game().world.at.as_str()), (0, "sealed_slab@spire_stair"));
    p.ticks(60);
    p.shot("z10_awake_at_the_slab");
}

/// Turning round anywhere on the way: past a place's end short of the next boundary, at once
/// again, past the boundary, and past a fork's junction. Each time the way back runs to the same
/// boundary or junction, as far off as it was behind. `cargo test --release turn_anywhere --
/// --ignored --test-threads=1`.
#[test]
#[ignore]
fn turn_anywhere() {
    use quartz_path_forge::WalkState;
    let under = |s: &WalkState| s.travel.zip(s.handover);
    let turn = |p: &mut Play| {
        p.key("w", KeyboardState::Released);
        p.fell_all();
        p.ticks(2);
        let walked = p.walk_state().walked;
        let offered = { let g = p.game(); g.world.can_turn(&g.fight, walked) };
        assert!(offered, "TURN is not offered: {}", p.status());
        p.tap("t");
        assert!(p.until_mode(60, |m| *m != crate::game::Mode::Play), "the turn never began: {}", p.status());
        assert!(p.until_playing(900), "the turn never ended: {}", p.status());
    };
    let mut p = Play::started();
    p.ticks(20);
    // From the start back down the road toward Stilt Lane (`road@^`: a crossing at its end).
    turn(&mut p);
    p.run_on("w");
    // Past the road's end, short of the boundary: the road walked the other way, the boundary
    // behind the hero by as much as it was ahead.
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("road@^") && under(s).is_some_and(|(t, h)| t > 4.0 && t < h - 4.0)), "never neared Stilt Lane: {}", p.status());
    let (travel, handover) = under(&p.walk_state()).unwrap_or_default();
    turn(&mut p);
    let st = p.walk_state();
    let into = st.walked - p.game().world.base;
    assert_eq!(st.stop.as_deref(), Some("road@warrens_lane"), "{}", p.status());
    assert!((into + f64::from(travel)).abs() < 1.5, "{into:.1} m into the road, not {:.1}: {}", -travel, p.status());
    // Turned again at once: the crossing toward Stilt Lane, as far along as it was.
    turn(&mut p);
    let st = p.walk_state();
    assert_eq!(st.stop.as_deref(), Some("road@^"), "{}", p.status());
    let (again, at) = under(&st).expect("the crossing under way");
    assert!((again - travel).abs() < 1.5 && at == handover, "{again:.1} m into the crossing, not {travel:.1}: {}", p.status());
    // Past the boundary: Stilt Lane walked the other way, the crossing back to the road under
    // way as far short of the boundary as the hero is past it.
    p.run_on("w");
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("road@^") && under(s).is_some_and(|(t, h)| t > h + 3.0)), "never crossed into Stilt Lane: {}", p.status());
    let behind = under(&p.walk_state()).map_or(0.0, |(t, h)| t - h);
    turn(&mut p);
    let st = p.walk_state();
    // Stilt Lane forks (the Warrens, the East Gate), so walked back toward the road it is walked
    // down its trunk.
    assert_eq!(st.stop.as_deref(), Some("warrens_lane@^"), "{}", p.status());
    let (t, h) = under(&st).expect("the crossing back under way");
    assert!((h - t - behind).abs() < 1.5, "the boundary {:.1} m ahead, not {behind:.1}: {}", h - t, p.status());
    p.ticks(60);
    p.shot("t01_turned_past_the_boundary");
    // On over it and up the road to its fork; left, toward the marsh, past the junction.
    p.run_on("w");
    assert!(p.walk_until(8000, true, |s| s.stop.as_deref() == Some("road@warrens_lane") && s.choose_within.is_some()), "no fork: {}", p.status());
    p.tap("a");
    assert!(p.walk_until(6000, true, |s| s.choose_within.is_none() && under(s).is_some_and(|(t, h)| t > h + 4.0)), "never passed the junction: {}", p.status());
    let behind = under(&p.walk_state()).map_or(0.0, |(t, h)| t - h);
    turn(&mut p);
    let st = p.walk_state();
    let back = crate::atlas::stop("road", "vale_edge");
    assert_eq!(st.stop.as_deref(), Some(back.as_str()), "{}", p.status());
    let (t, h) = under(&st).expect("the fork under way");
    assert!(st.choose_within.is_some(), "the junction is ahead to choose at: {}", p.status());
    assert!((h - t - behind).abs() < 1.5, "the junction {:.1} m ahead, not {behind:.1}: {}", h - t, p.status());
    p.ticks(60);
    p.shot("t02_turned_past_the_junction");
}

/// Through the door at the guild hall's end, Gruk's counting room is a dead end: walked to its end
/// the walk stops there, Gruk is spoken to and TURN is offered; turned, the room and the hall are
/// walked back out and the Warrens' junction is at the hall's end, the trunk down to Stilt Lane
/// taken (the sewers' way is shut from this side). `cargo test --release dead_end --
/// --ignored --nocapture --test-threads=1` writes target/shots/d*.png.
#[test]
#[ignore]
fn dead_end() {
    let mut p = Play::started();
    p.ticks(20);
    let start = "guild_hall@warrens";
    p.game().world.beaten.push("guild_hall");
    let go = { let w = &mut p.game().world; w.checkpoint = start.into(); let go = w.resume(); w.checkpoint = crate::world::START.into(); go };
    let pf = p.canvas.get_plugin_mut::<quartz_path_forge::PathForgePlugin>().unwrap();
    match go { crate::world::Go::WarpAt(s, m) => pf.warp_at(&s, m), crate::world::Go::Warp(s) => pf.warp(&s), _ => unreachable!() }
    assert!(p.walk_until(600, false, |s| s.stop.as_deref() == Some(start)), "never reached the hall: {}", p.status());
    p.ticks(30);
    p.shot("d00_into_the_hall");
    p.run_on("w");
    p.fell_all();
    // On to the door at the hall's end, and through it into Gruk's counting room.
    assert!(p.walk_until(2400, true, |s| s.stop.as_deref() == Some(start) && s.handover.zip(s.travel).is_some_and(|(at, t)| at - t < 9.0)), "no door ahead: {}", p.status());
    p.shot("d00b_the_door_to_gruks_room");
    let office = "guild_office@guild_hall";
    // Walked on and on: the walk stops at the room's end, nothing under way.
    p.walk_until(2400, true, |_| false);
    let st = p.walk_state();
    let (into, length) = (st.walked - p.game().world.base, crate::atlas::place_of("guild_office").map_or(0.0, |pl| pl.length));
    eprintln!("[dead end] at {:?}, {into:.1} m in of {length}, under way {:?}", st.stop, st.travel);
    assert!(st.stop.as_deref() == Some(office) && st.travel.is_none() && (into - f64::from(length)).abs() < 1.0, "not held at the end: {}", p.status());
    p.key("w", KeyboardState::Released);
    p.ticks(30);
    p.shot("d01_the_halls_end");
    let walked = p.walk_state().walked;
    // Gruk behind his counter: met, his words read through, the Hollowmere bounty taken.
    let near = { let g = p.game(); g.world.can_meet_gruk(&g.fight, walked) };
    assert!(near, "Gruk is out of reach at the end: {}", p.status());
    p.tap("e");
    assert!(p.until_mode(30, |m| matches!(m, crate::game::Mode::Meet(_))), "no word with Gruk: {}", p.status());
    p.ticks(10);
    p.shot("d01b_gruk");
    for _ in 0..12 { if !matches!(p.game().mode, crate::game::Mode::Meet(_)) { break; } p.tap("e"); p.ticks(2); }
    assert_eq!(p.game().world.guild.open(), Some(crate::guild::Bounty::Hollowmere), "{}", p.status());
    p.ticks(10);
    p.shot("d01c_bounty_taken");
    let offered = { let g = p.game(); g.world.can_turn(&g.fight, walked) };
    assert!(offered, "TURN is not offered at the end: {}", p.status());
    p.tap("t");
    assert!(p.until_mode(60, |m| *m != crate::game::Mode::Play), "the turn never began: {}", p.status());
    assert!(p.until_playing(900), "the turn never ended: {}", p.status());
    assert_eq!(p.walk_state().stop.as_deref(), Some("guild_office@^"), "{}", p.status());
    // Back out through the door and down the hall, the junction at its end: on down the
    // Warrens' trunk.
    p.run_on("w");
    assert!(p.walk_until(3000, true, |s| s.stop.as_deref() == Some("guild_hall@guild_office")), "never back in the hall: {}", p.status());
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("warrens@guild_hall") && s.choose_within.is_some()), "no junction out of the hall: {}", p.status());
    p.ticks(20);
    p.shot("d02_the_junction_out_of_the_hall");
    assert!(p.walk_until(9000, true, |s| s.stop.as_deref() == Some("warrens@^")), "never down the Warrens' trunk: {}", p.status());
    p.ticks(60);
    p.shot("d03_down_the_warrens");
}

/// Out of the guild hall the walk crosses into the Warrens' rain before their junction comes in
/// view, and the fork is on the alley's ground. Turned round just past it (toward Stilt Lane),
/// the junction is ahead again: the signpost stands its own way past it, and neither it nor the
/// camera has turned before the walk does. `cargo test --release turn_out_of_the_hall --
/// --ignored --nocapture --test-threads=1` writes target/shots/h*.png.
#[test]
#[ignore]
fn turn_out_of_the_hall() {
    use quartz_path_forge::WalkState;
    let under = |s: &WalkState| s.travel.zip(s.handover);
    let mut p = Play::started();
    p.ticks(20);
    let start = "guild_hall@guild_office";
    let go = { let w = &mut p.game().world; w.checkpoint = start.into(); let go = w.resume(); w.checkpoint = crate::world::START.into(); go };
    let pf = p.canvas.get_plugin_mut::<quartz_path_forge::PathForgePlugin>().unwrap();
    match go { crate::world::Go::WarpAt(s, m) => pf.warp_at(&s, m), crate::world::Go::Warp(s) => pf.warp(&s), _ => unreachable!() }
    assert!(p.walk_until(600, false, |s| s.stop.as_deref() == Some(start)), "never in the hall: {}", p.status());
    p.ticks(30);
    p.run_on("w");
    // No fork in the hall: the junction comes in view only once in the Warrens.
    let forked_in_hall = std::cell::Cell::new(false);
    let at_fork = p.walk_until(12000, true, |s| {
        if s.stop.as_deref() == Some(start) && s.choose_within.is_some() { forked_in_hall.set(true); }
        s.stop.as_deref() == Some("warrens@guild_hall") && s.choose_within.is_some()
    });
    assert!(at_fork && !forked_in_hall.get(), "the fork came in the hall, or never: {}", p.status());
    p.ticks(10);
    p.shot("h01_the_junction_out_of_the_hall");
    // Down the trunk toward Stilt Lane, 3 m past the junction, and turned round.
    let left = { let g = p.game(); g.world.ahead()[0].stop == "warrens@^" };
    p.tap(if left { "a" } else { "d" });
    assert!(p.walk_until(12000, true, |s| s.choose_within.is_none() && under(s).is_some_and(|(t, h)| t > h + 3.0)), "never past the junction: {}", p.status());
    p.key("w", KeyboardState::Released);
    p.fell_all();
    p.ticks(2);
    let walked = p.walk_state().walked;
    { let g = p.game(); assert!(g.world.can_turn(&g.fight, walked), "TURN not offered: {}", p.status()); }
    p.tap("t");
    assert!(p.until_mode(60, |m| *m != crate::game::Mode::Play) && p.until_playing(900), "the turn never ran: {}", p.status());
    p.ticks(2);
    let st = p.walk_state();
    assert_eq!(st.stop.as_deref(), Some("warrens@warrens_lane"), "{}", p.status());
    let junction = under(&st).map(|(t, h)| f64::from(h - t)).expect("the fork under way");
    let sign = p.game().world.signpost.expect("the signpost") - st.walked;
    assert!((sign - junction - f64::from(crate::world::SIGNPOST_PAST)).abs() < 0.3, "the signpost {sign:.1} m ahead, the junction {junction:.1}: {}", p.status());
    p.shot("h02_turned_past_the_junction");
}

/// Turning round past a fork's junction, on its own walk and on the branch's lane beyond: the way
/// back is the junction's, as far ahead as the hero is past it. The road's fork, and the Warrens'
/// both ways. `cargo test --release turn_on_a_lane -- --ignored --nocapture --test-threads=1`.
#[test]
#[ignore]
fn turn_on_a_lane() {
    use quartz_path_forge::WalkState;
    let under = |s: &WalkState| s.travel.zip(s.handover);
    // (where the walk starts, the way taken at its fork, the stop turned back onto)
    let cases = [
        ("road@warrens_lane", "vale_edge", "road@vale_edge"),
        ("warrens@warrens_lane", "guild_hall", "warrens@guild_hall"),
        ("warrens@guild_hall", "warrens_lane", "warrens@warrens_lane"),
    ];
    let mut failed = Vec::new();
    for (start, way, back) in cases {
        for lane in [false, true] {
            let mut p = Play::started();
            p.ticks(20);
            if start != crate::world::START {
                let go = { let w = &mut p.game().world; w.checkpoint = start.into(); let go = w.resume(); w.checkpoint = crate::world::START.into(); go };
                let pf = p.canvas.get_plugin_mut::<quartz_path_forge::PathForgePlugin>().unwrap();
                match go { crate::world::Go::WarpAt(s, m) => pf.warp_at(&s, m), crate::world::Go::Warp(s) => pf.warp(&s), _ => unreachable!() }
                assert!(p.walk_until(600, false, |s| s.stop.as_deref() == Some(start)), "never reached {start}: {}", p.status());
                p.ticks(30);
            }
            p.run_on("w");
            assert!(p.walk_until(12000, true, |s| s.stop.as_deref() == Some(start) && s.choose_within.is_some()), "no fork from {start}: {}", p.status());
            let left = { let g = p.game(); g.world.ahead()[0].road.1 == way };
            p.tap(if left { "a" } else { "d" });
            let junction = std::cell::Cell::new(None);
            let past = p.walk_until(12000, true, |s| {
                if junction.get().is_none() { if let Some((t, h)) = under(s).filter(|(t, h)| t >= h) { junction.set(Some(s.walked - f64::from(t - h))); } }
                if lane { s.stop.as_deref().is_some_and(crate::atlas::is_lane) && under(s).is_some_and(|(t, _)| t > 2.0) }
                else { s.choose_within.is_none() && under(s).is_some_and(|(t, h)| t > h + 3.0) }
            });
            assert!(past, "never past the junction from {start} toward {way}: {}", p.status());
            p.key("w", KeyboardState::Released);
            p.fell_all();
            p.ticks(2);
            let walked = p.walk_state().walked;
            let (offered, handed, at) = { let g = p.game(); (g.world.can_turn(&g.fight, walked), g.world.handed, g.world.at.clone()) };
            let case = format!("{start} -> {way} ({}), at {at}, handed {handed:?}, junction {junction:?}", if lane { "lane" } else { "fork" }, junction = junction.get());
            if !offered { failed.push(format!("{case}: TURN not offered")); continue; }
            p.tap("t");
            if !p.until_mode(60, |m| *m != crate::game::Mode::Play) || !p.until_playing(900) { failed.push(format!("{case}: the turn never ran")); continue; }
            let st = p.walk_state();
            let behind = walked - junction.get().unwrap_or(walked);
            // Under way, the junction is its hand-over ahead; short of the walk on, the rest of
            // the stop and then the hand-over.
            let into = st.walked - p.game().world.base;
            let rest = f64::from(crate::atlas::length_of(st.stop.as_deref().unwrap_or_default())) - into;
            let ahead = st.handover.map(|h| f64::from(h) - st.travel.map_or(-rest, f64::from));
            let line = format!("{case}: landed {:?} the junction {ahead:?} ahead, {behind:.1} behind", st.stop);
            eprintln!("[lane] {line}");
            if st.stop.as_deref() != Some(back) || ahead.is_none_or(|a| (a - behind).abs() > 1.5) { failed.push(line); }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// Into the guild hall and up to its boss, every slow tick named (`HR_SLOW_TICK=30 cargo test
/// --release arrival_ticks -- --ignored --nocapture --test-threads=1`): what a scene change costs
/// the game thread.
#[test]
#[ignore]
fn arrival_ticks() {
    let mut p = Play::started();
    p.ticks(20);
    p.tap("t");
    p.ticks(30);
    p.run_on("w");
    assert!(p.walk_until(9000, true, |s| s.stop.as_deref() == Some("warrens@warrens_lane")), "never reached the Warrens: {}", p.status());
    eprintln!("[arrival] the Warrens");
    assert!(p.walk_until(6000, true, |s| s.choose_within.is_some()), "no fork: {}", p.status());
    eprintln!("[arrival] the fork");
    p.tap("d");
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("guild_hall@warrens")), "never reached the guild hall: {}", p.status());
    eprintln!("[arrival] the guild hall");
    for _ in 0..900 { p.ticks(1); if !p.game().playing() { eprintln!("[arrival] mode {:?}", p.game().mode); break; } }
    p.ticks(120);
}

/// Back from the start through Stilt Lane into the Warrens, to its fork: the sewers' way is
/// barred from this side (a door across its mouth, told as the fork comes in view), so leaning
/// into it the walk takes the guild hall's, the signpost swinging off with the junction.
#[test]
#[ignore]
fn barred() {
    let mut p = Play::started();
    p.ticks(20);
    p.tap("t");
    p.ticks(30);
    p.run_on("w");
    // Stilt Lane into the Warrens: the alley opens in a street front, not two walls out of the ground.
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("warrens_lane@road") && s.progress.is_some_and(|t| t > 0.35)), "never came to the Warrens: {}", p.status());
    p.shot("b00a_warrens_front_far");
    // Stilt Lane forks at its end, the Warrens one way and the East Gate the other: lean the
    // Warrens' way.
    let warrens_left = crate::atlas::leads("warrens_lane", "road").first().is_some_and(|l| l.road.1 == "warrens");
    assert!(p.walk_until(6000, true, |s| s.choose_within.is_some()), "no fork on Stilt Lane: {}", p.status());
    p.tap(if warrens_left { "a" } else { "d" });
    assert!(p.walk_until(6000, true, |s| s.progress.is_some_and(|t| t > 0.75)), "never neared the Warrens: {}", p.status());
    p.shot("b00b_warrens_front_near");
    assert!(p.walk_until(6000, true, |s| s.stop.as_deref() == Some("warrens@warrens_lane")), "never reached the Warrens: {}", p.status());
    assert!(p.walk_until(6000, true, |s| s.choose_within.is_some_and(|m| m < 24.0)), "no fork: {}", p.status());
    p.key("w", KeyboardState::Released);
    p.ticks(20);
    p.shot("b01_fork_barred_left");
    p.tap("a");
    p.ticks(10);
    p.shot("b02_lean_into_the_barred_way");
    p.key("w", KeyboardState::Pressed);
    // `BARRED_SEQ=1`: from 2 m short of the junction, a frame every 6 ticks (48), to watch the shut way's mouth.
    let seq = std::env::var("BARRED_SEQ").is_ok();
    let mut n = 0;
    if seq { p.walk_until(4000, false, |s| s.choose_within.is_some_and(|m| m < 2.0)); }
    for i in 0..4000 {
        if p.game().world.chose && !seq { break; }
        if seq && i % 6 == 0 { p.shot(&format!("bs{n:02}")); n += 1; if n == 48 { break; } }
        p.ticks(1);
    }
    assert!(p.game().world.chose && p.game().world.took == Some(quartz_path_forge::Branch::Right), "the open way was not taken: {}", p.status());
    p.ticks(25);
    p.shot("b03_taken_the_open_way");
    p.ticks(50);
    p.shot("b04_turned_onto_it");
}

/// The forged weapons' arts (Q): each on the road against husks set where it reaches, as it winds
/// up and as it goes off (the Hatchet in flight, Sentence's blow falling, Wingbeat's dive); then
/// three in the yard at the gate.
#[test]
#[ignore]
fn arts() {
    use crate::arts::{WeaponArt, DIVE_AT, SENTENCE_DELAY};
    use crate::fight::{Ai, Archetype, Enemy};
    use crate::game::Mode;
    let mut p = Play::started();
    p.ticks(40);
    let forged = |piece: &str| {
        let mut l = crate::gear::Loadout::starting();
        let i = crate::gear::find(piece);
        l.gain(i);
        assert!(l.wear(0, Some(i)));
        l.kit()
    };
    let road: [(&str, &str, &[(f32, i32)]); 8] = [
        ("Bell-Hammer", "art01_toll", &[(2.2, 0), (2.6, 1)]),
        ("Drowned Wail", "art02_wail", &[(4.0, 0), (5.0, -1)]),
        ("Chieftain's Cleaver", "art03_hatchet", &[(6.0, 0)]),
        ("Gavel of Sentence", "art04_sentence", &[(4.5, 0)]),
        ("Reaper's Glaive", "art05_reap", &[(2.4, -1), (2.4, 0), (2.4, 1)]),
        ("Lance of Light", "art06_lance", &[(2.8, 0), (3.8, 0)]),
        ("Stoneheart Greatsword", "art07_stone_skin", &[(2.2, 0)]),
        ("Feather of the Fallen Light", "art08_wingbeat", &[(3.5, 0)]),
    ];
    for (piece, name, at) in road {
        let walked = p.canvas.get_plugin::<quartz_path_forge::PathForgePlugin>().unwrap().walked_ahead();
        {
            let g = p.game();
            g.fight.enemies = at.iter().map(|&(d, lane)| Enemy { ai: Ai::Wait(10_000), ..Enemy::new(Archetype::Husk, walked + crate::fight::HERO_D as f64 + d as f64, lane) }).collect();
            g.fight.missiles.clear();
            let h = &mut g.fight.hero;
            h.stats.kit = forged(piece);
            (h.stats.might, h.stats.dexterity) = (10, 10);
            *h = crate::fight::Hero::with(h.stats);
            h.art = 100.0;
        }
        let art = p.fight().hero.stats.kit.art.expect("no art");
        let fires = art.fires();
        p.tap("q");
        assert!(p.until(60, |f| f.hero.act.is_some_and(|a| a.mv == crate::fight::Move::Art && a.t + 6 >= fires)), "{piece}: the art never began");
        p.shot(&format!("{name}_windup"));
        p.until(30, |f| f.hero.act.is_some_and(|a| a.t >= fires + 2));
        p.shot(name);
        let late = match art {
            WeaponArt::Hatchet => p.until(120, |f| f.hatchets.first().is_some_and(|t| t.back)),
            WeaponArt::Sentence => p.until(SENTENCE_DELAY + 20, |f| f.hero.mark.is_some_and(|(_, n)| n <= 1)),
            WeaponArt::Wingbeat => p.until(60, |f| f.hero.act.is_some_and(|a| a.t >= DIVE_AT)),
            _ => false,
        };
        if late { p.ticks(2); p.shot(&format!("{name}_after")); }
        p.until(240, |f| f.hero.act.is_none() && f.hatchets.is_empty() && f.hero.mark.is_none());
        p.ticks(30);
    }
    // The yard at the gate: the Toll, the Lance through the boss, the Hatchet across.
    p.game().fight.enemies.clear();
    p.game().world.warping = Some("gate@vale".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:gate@vale".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("gate@vale")), "never reached the gate");
    p.ticks(10);
    p.key("w", KeyboardState::Pressed);
    let mut met = false;
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        p.step_through_veil();
        if matches!(p.mode(), Mode::Side(_)) { met = true; break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(met, "the boss never stepped out");
    p.ticks(40);
    p.skip_scene();
    assert!(p.until_mode(400, |m| matches!(m, Mode::Side(a) if a.t >= 140)));
    for (piece, name, gap, after) in [("Bell-Hammer", "art10_yard_toll", 1.6, 2), ("Lance of Light", "art11_yard_lance", 1.8, 6), ("Chieftain's Cleaver", "art12_yard_hatchet", 4.0, 14)] {
        let kit = forged(piece);
        {
            let a = arena(&mut p);
            let h = &mut a.side.hero;
            (h.stats.kit, h.stats.might, h.stats.dexterity) = (kit, 10, 10);
            (h.hp, h.art, h.reel, h.reeling, h.act) = (h.stats.max_hp(), 100.0, crate::fight::Reel::None, 0, None);
            h.s = -2.5;
            h.facing = 1.0;
            a.side.foes[0].s = h.s + gap;
            a.side.foes[0].facing = -1.0;
            a.side.foes[0].ai = crate::side::Ai::Wait(10_000);
        }
        p.game().fight.hero.stats.kit = kit;
        p.ticks(4);
        p.tap("q");
        let fires = kit.art.expect("no art").fires();
        assert!(p.until_mode(80, |m| matches!(m, Mode::Side(a) if a.side.hero.act.is_some_and(|(_, t)| t >= fires + after))), "{piece}: no art in the yard");
        p.shot(name);
        p.until_mode(200, |m| matches!(m, Mode::Side(a) if a.side.hero.act.is_none() && a.side.hatchets.is_empty()));
        p.ticks(20);
    }
}

/// Krix home from the fog, asked about his axe and let keep it; then at the Magistrate's veil
/// Mara's whistle blown twice, and Krix in the yard on the boss's far side, fighting with the
/// hero and falling back from what comes at him.
/// `cargo test --release --lib -- --ignored --exact shots::summon_krix` writes target/shots/sk*.png.
#[test]
#[ignore]
fn summon_krix() {
    use crate::game::Mode;
    let mut p = Play::started();
    {
        let w = &mut p.game().world;
        w.beaten.extend(["guild_hall", "chief_hold", "fog_hollow", "black_water"]);
        w.taken.push("black_water");
        w.hunters = crate::hunters::Hunters { lost: 0, settled: true };
    }
    let near = |p: &mut Play| { let walked = p.walk_state().walked; let g = p.game(); g.world.can_meet_visitor(&g.fight, walked) };
    p.game().world.warping = Some("guild_hall@warrens".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:guild_hall@warrens".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("guild_hall@warrens")), "never reached the hall");
    p.key("w", KeyboardState::Pressed);
    for _ in 0..3000 { p.fell_all(); p.ticks(1); if near(&mut p) { break; } }
    p.key("w", KeyboardState::Released);
    assert!(near(&mut p), "Krix is out of reach: {}", p.status());
    p.ticks(20);
    p.tap("e");
    for _ in 0..2 { p.ticks(4); p.tap("e"); }
    p.ticks(6);
    p.shot("sk01_the_axe_asked");
    p.tap("s");
    p.ticks(4);
    p.shot("sk02_keep_it_picked");
    p.tap("e");
    p.ticks(6);
    p.shot("sk03_his_answer");
    for _ in 0..6 { if matches!(p.mode(), Mode::Play) { break; } p.tap("e"); p.ticks(4); }
    assert!(p.game().world.answers.contains(&"krix_keeps_axe"), "the answer was never given: {:?}", p.game().world.answers);
    assert!(!p.game().world.gear.owns(crate::gear::find("Krix's Notched Axe")), "the axe was taken anyway");
    p.game().world.warping = Some("magistrate@stone_upper".into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: "warp:magistrate@stone_upper".into() });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some("magistrate@stone_upper")), "never reached the court");
    let calls = |p: &mut Play| { let walked = p.walk_state().walked; let g = p.game(); g.playing() && g.world.can_call(&g.fight, walked) };
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        if calls(&mut p) { break; }
        if matches!(p.mode(), Mode::Side(_)) { for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); } }
    }
    p.key("w", KeyboardState::Released);
    assert!(calls(&mut p), "no call at the veil: {}", p.status());
    p.ticks(20);
    p.shot("sk04_the_whistle_offered");
    p.tap("e");
    p.ticks(30);
    p.shot("sk05_two_notes");
    assert_eq!(p.game().world.called, Some(crate::allies::Summoner::Krix));
    for _ in 0..600 { p.ticks(1); p.step_through_veil(); if matches!(p.mode(), Mode::Side(_)) { break; } }
    assert!(matches!(p.mode(), Mode::Side(a) if a.why == crate::game::Why::Boss && a.side.allies.len() == 1), "Krix never came: {}", p.status());
    for _ in 0..300 { if p.scene_at().is_none() { break; } p.ticks(1); }
    p.ticks(60);
    p.shot("sk06_krix_across_the_yard");
    for (k, secs) in [(7, 2.5), (8, 2.5), (9, 3.0)] {
        // The hero stands and takes it (kept on their feet): this is Krix's fight to show.
        let h = &mut arena(&mut p).side.hero;
        h.hp = h.stats.max_hp();
        p.watch(secs);
        p.shot(&format!("sk0{k}_fighting"));
    }
    let a = arena(&mut p);
    eprintln!("[summon] boss {:.0}/{:.0}, Krix {:.0}/{:.0} {:?}", a.side.foes[0].hp, a.side.foes[0].top, a.side.allies[0].hp, a.side.allies[0].top, a.side.allies[0].doing);
    assert!(a.side.foes[0].hp < a.side.foes[0].top, "nobody touched the Magistrate");
}

/// The later cast, in the story's order: the registrar paying for the Blackridge helms, Steel
/// reading the rune, the hooded thing striking in the alley and melting away, Helena naming the
/// Shrine; Exodus in its halls and the heart that wants blood; Atrael at Varnholt's barns, and
/// Fin-S asleep in the roots on the violet road.
/// `cargo test --release --lib -- --ignored --exact shots::later_cast` writes target/shots/lc*.png.
#[test]
#[ignore]
fn later_cast() {
    use crate::game::Mode;
    let mut p = Play::started();
    {
        let w = &mut p.game().world;
        w.beaten.extend(["guild_hall", "gate", "hollowmere", "chief_hold"]);
        w.keys.extend([crate::atlas::Key::Rune, crate::atlas::Key::Hook, crate::atlas::Key::Lantern]);
        (w.guild.posted, w.guild.paid) = (2, 1);
    }
    let near = |p: &mut Play| { let walked = p.walk_state().walked; let g = p.game(); g.world.can_meet_visitor(&g.fight, walked) };
    let go = |p: &mut Play, stop: &str| {
        p.game().world.warping = Some(stop.into());
        p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: format!("warp:{stop}") });
        assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some(stop)), "never reached {stop}");
    };
    // Walks on to whoever waits here, shows them and what they say, and reads it all.
    let meet = |p: &mut Play, tag: &str| {
        p.key("w", KeyboardState::Pressed);
        for _ in 0..4000 { p.fell_all(); p.ticks(1); if near(p) { break; } if matches!(p.mode(), Mode::Side(_)) { for f in &mut arena(p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); } } }
        p.key("w", KeyboardState::Released);
        assert!(near(p), "nobody to meet for {tag}: {}", p.status());
        p.ticks(20);
        p.shot(&format!("{tag}a"));
        p.tap("e");
        p.ticks(6);
        p.shot(&format!("{tag}b"));
        for _ in 0..10 { if matches!(p.mode(), Mode::Play) { break; } p.tap("e"); p.ticks(4); }
    };
    go(&mut p, "lower_stairs@stone_gate");
    meet(&mut p, "lc01_registrar");
    assert_eq!(p.game().world.guild.paid, 2, "the helms were never paid for");
    go(&mut p, "stone_upper@stone_market");
    meet(&mut p, "lc02_steel");
    go(&mut p, "sewers@lower_stairs");
    p.key("w", KeyboardState::Pressed);
    for _ in 0..4000 { p.fell_all(); p.ticks(1); if matches!(p.mode(), Mode::Side(a) if a.why == crate::game::Why::Boss) { break; } if matches!(p.mode(), Mode::Side(_)) { for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); } } }
    p.key("w", KeyboardState::Released);
    assert!(matches!(p.mode(), Mode::Side(a) if a.side.flee.is_some()), "the alley never struck: {}", p.status());
    p.ticks(90);
    p.shot("lc03_the_hooded_thing");
    { let f = &mut arena(&mut p).side.foes[0]; f.hp = 0.55 * f.top; }
    p.ticks(10);
    p.shot("lc04_it_melts_away");
    assert!(p.until_mode(500, |m| matches!(m, Mode::Play)), "the yard never let go");
    assert!(p.game().world.meetings.contains(&crate::cast::STRUCK));
    meet(&mut p, "lc05_helena");
    go(&mut p, "vine_halls@sunken_steps");
    meet(&mut p, "lc06_exodus");
    p.key("w", KeyboardState::Pressed);
    let bleeds = |p: &mut Play| { let walked = p.walk_state().walked; let within = p.walk_state().choose_within; let g = p.game(); g.playing() && g.world.can_bleed(&g.fight, walked, within) };
    for _ in 0..3000 { p.fell_all(); p.ticks(1); if bleeds(&mut p) { break; } }
    p.key("w", KeyboardState::Released);
    assert!(bleeds(&mut p), "no heart at the fork: {}", p.status());
    p.ticks(10);
    p.shot("lc07_the_heart");
    p.tap("e");
    p.ticks(20);
    p.shot("lc08_blood_given");
    assert!(p.game().world.keys.contains(&crate::atlas::Key::Heart));
    p.game().world.beaten.push("wraith_sanctum");
    go(&mut p, "varnholt@violet_road");
    meet(&mut p, "lc09_atrael");
    go(&mut p, "violet_road@varnholt");
    meet(&mut p, "lc10_fin_s");
}

/// The last act, through the game: the Archangel falls and the trap is sprung at the Summit; the
/// hero wakes at the slab; Olekeen in the cells, Fin-S in the foundry, Olekeen called to Exodus
/// and the fight; the Eye handed over, the three called, and the ending as the road left it.
/// `cargo test --release --lib -- --ignored --exact shots::finale --test-threads=1` writes target/shots/fn*.png.
#[test]
#[ignore]
fn finale() {
    use crate::game::Mode;
    let mut p = Play::started();
    {
        let w = &mut p.game().world;
        w.beaten.extend(["guild_hall", "gate", "hollowmere", "chief_hold", "magistrate", "wraith_sanctum", "cathedral", "statue_court", "east_gate", "black_water", "fog_hollow"]);
        w.keys.extend([crate::atlas::Key::Rune, crate::atlas::Key::Hook, crate::atlas::Key::Lantern, crate::atlas::Key::Sigil, crate::atlas::Key::Heart, crate::atlas::Key::ShardI, crate::atlas::Key::ShardII]);
        w.answers.extend(["fins_woken", "atrael_stopped", "olekeen_hunts_with"]);
        w.meetings.extend(["steel_reads", "olekeen_bound", "fins_sleeps"]);
        w.hunters = crate::hunters::Hunters { lost: 0, settled: true };
    }
    let go = |p: &mut Play, stop: &str| {
        p.game().world.warping = Some(stop.into());
        p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: format!("warp:{stop}") });
        assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some(stop)), "never reached {stop}");
    };
    let clear = |p: &mut Play| if matches!(p.mode(), Mode::Side(a) if a.why != crate::game::Why::Boss) { for f in &mut arena(p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); } };
    // Walks on to the veil, steps through it, and waits out the entrance.
    let to_boss = |p: &mut Play| {
        p.key("w", KeyboardState::Pressed);
        for _ in 0..6000 { p.fell_all(); p.ticks(1); clear(p); p.step_through_veil(); if matches!(p.mode(), Mode::Side(a) if a.why == crate::game::Why::Boss) { break; } }
        p.key("w", KeyboardState::Released);
        assert!(matches!(p.mode(), Mode::Side(a) if a.why == crate::game::Why::Boss), "no boss: {}", p.status());
        for _ in 0..400 { if p.scene_at().is_none() { break; } p.ticks(1); }
    };
    let near = |p: &mut Play| { let walked = p.walk_state().walked; let g = p.game(); g.world.can_meet_visitor(&g.fight, walked) };
    let meet = |p: &mut Play, tag: &str| {
        p.key("w", KeyboardState::Pressed);
        for _ in 0..4000 { p.fell_all(); p.ticks(1); clear(p); if near(p) { break; } }
        p.key("w", KeyboardState::Released);
        assert!(near(p), "nobody to meet for {tag}: {}", p.status());
        p.ticks(20);
        p.shot(&format!("{tag}a"));
        p.tap("e");
        p.ticks(6);
        p.shot(&format!("{tag}b"));
    };
    go(&mut p, "spire_summit@sealed_slab");
    to_boss(&mut p);
    { let f = &mut arena(&mut p).side.foes[0]; (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    p.ticks(150);
    p.shot("fn01_the_archangel_falls");
    assert!(p.until_mode(900, |m| matches!(m, Mode::Scene(crate::game::Then::Taken))), "no trap: {}", p.status());
    for (k, secs) in [(2, 4.0), (3, 7.5), (4, 9.0), (5, 6.5), (6, 4.5), (7, 5.0), (8, 4.0)] {
        p.ticks((secs * 60.0) as u32);
        p.shot(&format!("fn0{k}_the_trap"));
    }
    assert!(p.until_mode(3000, |m| matches!(m, Mode::Play)), "never woke: {}", p.status());
    assert!(p.game().world.met(crate::finale::STRUCK) && p.walk_state().stop.as_deref() == Some("sealed_slab@spire_stair"), "{}", p.status());
    p.ticks(30);
    p.shot("fn09_awake_at_the_slab");
    go(&mut p, "penitent_cells@radiant_gate");
    meet(&mut p, "fn10_olekeen");
    for _ in 0..10 { if matches!(p.mode(), Mode::Play) { break; } p.tap("e"); p.ticks(4); }
    go(&mut p, "unit_foundry@penitent_cells");
    meet(&mut p, "fn11_unit_zero");
    for _ in 0..10 { if matches!(p.mode(), Mode::Play) { break; } p.tap("e"); p.ticks(4); }
    assert!(p.game().world.answers.contains(&"fins_free"), "{:?}", p.game().world.answers);
    go(&mut p, "eye_chamber@penitent_cells");
    let calls = |p: &mut Play| { let walked = p.walk_state().walked; let g = p.game(); g.playing() && g.world.can_call(&g.fight, walked) };
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 { p.ticks(1); if calls(&mut p) { break; } }
    p.key("w", KeyboardState::Released);
    assert!(calls(&mut p), "Olekeen does not answer at Exodus's veil: {}", p.status());
    p.ticks(20);
    p.shot("fn12_the_stone_raised");
    p.tap("e");
    to_boss(&mut p);
    p.ticks(60);
    p.shot("fn13_exodus");
    let h = &mut arena(&mut p).side.hero;
    h.hp = h.stats.max_hp();
    p.watch(3.0);
    p.shot("fn14_fighting_exodus");
    { let f = &mut arena(&mut p).side.foes[0]; f.hp = 0.0; }
    p.ticks(150);
    p.shot("fn15_exodus_falls");
    for (k, secs) in [("a", 4.0), ("b", 5.5), ("c", 5.5), ("d", 5.0)] {
        p.ticks((secs * 60.0) as u32);
        p.shot(&format!("fn15{k}_possession"));
    }
    assert!(p.until_mode(1500, |m| matches!(m, Mode::Side(a) if std::ptr::eq(a.side.foes[0].breed, &crate::order::VESSEL))), "Voryn never took Exodus: {}", p.status());
    for _ in 0..600 { if p.scene_at().is_none() { break; } p.ticks(1); }
    p.watch(2.0);
    p.shot("fn15e_voryns_vessel");
    { let f = &mut arena(&mut p).side.foes[0]; f.hp = 0.0; }
    p.ticks(150);
    p.shot("fn15f_the_vessel_falls");
    assert!(p.until_mode(1500, |m| matches!(m, Mode::Play)), "the chamber never let go: {}", p.status());
    assert!(p.game().world.beaten.contains(&crate::atlas::FINALE));
    meet(&mut p, "fn16_the_eye");
    for _ in 0..10 { if matches!(p.mode(), Mode::Scene(_)) { break; } p.tap("e"); p.ticks(4); }
    assert!(matches!(p.mode(), Mode::Scene(crate::game::Then::Conclude)), "no ending: {} {:?}", p.status(), p.game().world.answers);
    assert!(p.game().world.answers.contains(&"eye_sealed"));
    for k in 17..26 {
        p.ticks(6 * 60);
        p.shot(&format!("fn{k}_the_end"));
    }
    assert!(p.until_mode(4000, |m| matches!(m, Mode::Play)), "the pilgrimage never began again: {}", p.status());
    assert_eq!(p.game().world.pilgrimages, 1);
    assert!(!p.game().world.met(crate::finale::STRUCK), "the last act is to be played again");
}

/// The other last boss: Fin-S never freed from his plinth, so when Exodus falls the Eye finishes
/// Unit 0 and Voryn comes through it whole. `cargo test --release --lib -- --ignored --exact
/// shots::finale_unfinished --test-threads=1` writes target/shots/fu*.png.
#[test]
#[ignore]
fn finale_unfinished() {
    use crate::game::Mode;
    let mut p = Play::started();
    {
        let w = &mut p.game().world;
        w.beaten.extend(["chief_hold", "magistrate", "wraith_sanctum", "cathedral", "statue_court", "spire_summit"]);
        w.keys.extend([crate::atlas::Key::Rune, crate::atlas::Key::ShardI, crate::atlas::Key::ShardII, crate::atlas::Key::ShardIII]);
        w.answers.push("fins_woken");
        w.meetings.extend([crate::finale::STRUCK, "fins_taken"]);
    }
    let stop = "eye_chamber@penitent_cells";
    p.game().world.warping = Some(stop.into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: format!("warp:{stop}") });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some(stop)), "never reached the chamber");
    let boss = |p: &mut Play| matches!(p.mode(), Mode::Side(a) if a.why == crate::game::Why::Boss);
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        if matches!(p.mode(), Mode::Side(_)) && !boss(&mut p) { for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); } }
        p.step_through_veil();
        if boss(&mut p) { break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(boss(&mut p), "no Exodus: {}", p.status());
    for _ in 0..400 { if p.scene_at().is_none() { break; } p.ticks(1); }
    p.ticks(60);
    { let f = &mut arena(&mut p).side.foes[0]; (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    p.ticks(150);
    for (k, secs) in [(1, 4.0), (2, 5.5), (3, 5.5), (4, 5.0)] {
        p.ticks((secs * 60.0) as u32);
        p.shot(&format!("fu0{k}_the_door_opens"));
    }
    assert!(p.until_mode(1500, |m| matches!(m, Mode::Side(a) if std::ptr::eq(a.side.foes[0].breed, &crate::order::WHOLE))), "Voryn never came through Unit 0: {}", p.status());
    p.ticks(100);
    p.shot("fu05_entrance");
    for _ in 0..600 { if p.scene_at().is_none() { break; } p.ticks(1); }
    arena(&mut p).side.hero.hp = 1_000.0;
    p.watch(3.0);
    p.shot("fu06_voryn_made_whole");
    { let f = &mut arena(&mut p).side.foes[0]; (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); }
    p.ticks(150);
    p.shot("fu07_unit_zero_falls");
    assert!(p.until_mode(1500, |m| matches!(m, Mode::Play)), "the chamber never let go: {}", p.status());
    assert!(p.game().world.beaten.contains(&crate::atlas::FINALE));
    assert_eq!(p.game().world.fates().fin_s, crate::finale::Unit::Fell);
}

/// Every area as it is walked up to: from the stop before it, with every boss beaten and the
/// road cleared, the crossing shot as it begins, halfway and at the threshold, so a place that
/// reads as floating in nothing, or a join that does not line up, shows. `cargo test --release
/// --lib -- --ignored --exact shots::approaches --test-threads=1` writes target/shots/ap*.png;
/// `APPROACH=a,b` shoots only the stops naming one of them.
#[test]
#[ignore]
fn approaches() {
    use crate::game::Mode;
    const FROM: [&str; 43] = ["blackwood@cathedral", "void_wood@^", "blackwood@void_wood", "starlit_wastes@statue_court", "sky_peaks@starlit_wastes", "stone_gate@lower_stairs", "east_gate@warrens_lane", "cathedral@varnholt", "sunken_steps@vine_halls",
        "blackwood@angel_ruins", "chapel@^", "chapel_edge@road", "cliff_path@^", "chief_hold@ridge_camp", "spire_summit@sealed_slab", "drowned_fingers@vale_edge",
        "black_water@^", "guild_office@^", "guild_hall@warrens", "drowned_mill@^", "hollowmere@^", "stone_gate@bandit_hollows", "violet_road@varnholt",
        "unit_foundry@^", "ridge_camp@^", "crimson_stair@spire_summit", "bandit_hollows@^", "warrens_lane@^", "spire_stair@sky_peaks", "statue_court@angel_ruins",
        "crimson_stair@radiant_gate", "spire_stair@sealed_slab", "lower_stairs@^", "bridge_ward@^", "magistrate@^", "vine_halls@^", "vale@^", "violet_road@magistrate",
        "sunken_steps@bridge_ward", "varnholt@^", "guild_hall@guild_office", "warrens@^", "gate@^"];
    let only = std::env::var("APPROACH").ok();
    let mut p = Play::started();
    {
        let w = &mut p.game().world;
        w.beaten.extend(crate::atlas::PLACES.iter().map(|place| place.id));
        w.keys.extend(crate::atlas::Key::ALL);
        w.hunters = crate::hunters::Hunters { lost: 0, settled: true };
    }
    let clear = |p: &mut Play| if matches!(p.mode(), Mode::Side(_)) { for f in &mut arena(p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); } };
    for (k, stop) in FROM.iter().enumerate() {
        if only.as_deref().is_some_and(|o| !o.split(',').any(|part| stop.contains(part))) { continue; }
        p.game().world.warping = Some((*stop).into());
        p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: format!("warp:{stop}") });
        if !p.walk_until(3000, false, |s| s.stop.as_deref() == Some(*stop)) { eprintln!("{stop}: never reached"); continue; }
        p.key("w", KeyboardState::Pressed);
        let tag = stop.replace(['@', '^'], "_");
        for (at, name) in [(0.02, "a"), (0.5, "b"), (0.92, "c")] {
            let mut seen = false;
            for _ in 0..6000 {
                p.fell_all();
                p.ticks(1);
                clear(&mut p);
                if p.walk_state().progress.is_some_and(|t| t >= at) { seen = true; break; }
                if p.walk_state().stop.as_deref() != Some(*stop) { break; }
            }
            if !seen { eprintln!("{stop}: no crossing seen at {at}"); break; }
            p.shot(&format!("ap{k:02}{name}_{tag}"));
        }
        p.key("w", KeyboardState::Released);
    }
}

/// Atrael at the Hollowmere Screamer once Olekeen is in the world: down in his ravens at a
/// quarter beaten, then thrown out at half. `cargo test --release --lib -- --ignored --exact
/// shots::atrael_ravens --test-threads=1` writes target/shots/ar*.png.
#[test]
#[ignore]
fn atrael_ravens() {
    use crate::game::Mode;
    let mut p = Play::started();
    p.game().world.meetings.push("olekeen_gate_his_kill");
    p.game().world.guild.posted = 1;
    let stop = "hollowmere@gate";
    p.game().world.warping = Some(stop.into());
    p.canvas.run(Action::RunPlugin { name: "path_forge".into(), data: format!("warp:{stop}") });
    assert!(p.walk_until(3000, false, |s| s.stop.as_deref() == Some(stop)), "never reached Hollowmere");
    let boss = |p: &mut Play| matches!(p.mode(), Mode::Side(a) if a.why == crate::game::Why::Boss);
    p.key("w", KeyboardState::Pressed);
    for _ in 0..6000 {
        p.fell_all();
        p.ticks(1);
        if matches!(p.mode(), Mode::Side(_)) && !boss(&mut p) { for f in &mut arena(&mut p).side.foes { (f.hp, f.ai) = (0.0, crate::side::Ai::Dead); } }
        p.step_through_veil();
        if boss(&mut p) { break; }
    }
    p.key("w", KeyboardState::Released);
    assert!(boss(&mut p), "no Screamer: {}", p.status());
    for _ in 0..600 { if p.scene_at().is_none() { break; } p.ticks(1); }
    p.ticks(60);
    let hurt = |p: &mut Play, share: f32| { let a = arena(p); a.side.hero.hp = 1_000.0; let f = &mut a.side.foes[0]; f.hp = share * f.top; };
    hurt(&mut p, 0.7);
    p.ticks(30);
    assert!(arena(&mut p).side.allies.iter().any(|a| a.who == crate::allies::Summoner::Atrael), "no ravens");
    p.shot("ar01_ravens");
    p.watch(3.0);
    p.shot("ar02_atrael_fights");
    hurt(&mut p, 0.44);
    p.ticks(40);
    p.shot("ar03_flung");
    assert!(arena(&mut p).side.allies.iter().all(|a| !a.standing()), "never thrown out");
}

/// A first pilgrimage set out from its opening, played through: the only place named is the one
/// it begins in (the opening's roads and the fade after them are not the hero's), and the hero is
/// never drawn over a road the opening has cut away from. `cargo test --release new_game_begins
/// -- --ignored --test-threads=1`.
#[test]
#[ignore]
fn new_game_begins_where_the_opening_says() {
    let mut p = Play::new();
    p.ticks(5);
    p.tap("e");
    p.tap("e");
    p.tap("s");
    p.tap("e");
    let mut told: Vec<String> = Vec::new();
    for _ in 0..8000 {
        p.ticks(1);
        let line = p.game().banner.0.clone();
        let quiet = matches!(p.mode(), crate::game::Mode::Scene(_) | crate::game::Mode::Out(_));
        assert!(!quiet || line.is_empty(), "\"{line}\" said in {:?}, on {:?}", p.mode(), p.walk_state().stop);
        if !line.is_empty() && told.last() != Some(&line) { told.push(line); }
        let road = p.canvas.get_plugin::<crate::game::Pilgrimage>().unwrap().scene_road();
        let shown = p.canvas.get_game_object("hero").is_some_and(|o| o.visible);
        if let (Some(road), true) = (road, shown) {
            assert_eq!(p.walk_state().stop.as_deref(), Some(road), "the hero drawn over another road: {}", p.status());
        }
        if matches!(p.mode(), crate::game::Mode::Play) { break; }
    }
    assert!(matches!(p.mode(), crate::game::Mode::Play), "never set out: {}", p.status());
    assert_eq!(told, vec!["The Ashen Warrens".to_string()], "named on the way in");
    assert_eq!(p.walk_state().stop.as_deref(), Some(crate::world::START));
}
