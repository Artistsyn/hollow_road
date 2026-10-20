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
    let mut p = Play::new();
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
    // Dodge right just before it falls: a just dodge, in slow motion.
    p.until(60, |f| f.enemies[0].attack().is_some_and(|a| a.2 > 0.82));
    eprintln!("[fight] dodging at {:?}", p.fight().enemies[0].ai);
    // D and Space together: a roll to the right.
    p.key("d", KeyboardState::Pressed);
    p.key_as(Key::Named(NamedKey::Space), KeyboardState::Pressed);
    p.key_as(Key::Named(NamedKey::Space), KeyboardState::Released);
    p.key("d", KeyboardState::Released);
    p.ticks(10);
    p.shot("03_just_dodge");
    eprintln!("[fight] after the dodge: hp {} lane {}", p.fight().hero.hp, p.fight().hero.lane);
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
