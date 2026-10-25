//! Hollow Road: a soulslike pilgrimage down one living road. PathForge draws the road live on
//! the GPU; the hero and everything they fight are Quartz sprites placed on it in road space
//! (`fight`), so they stand in the road's own perspective.

use quartz::*;
use ramp::prism;

mod anim;
mod audio;
mod cutscene;
mod art;
mod arts;
mod assets;
mod atlas;
mod fight;
mod fx;
mod gear;
mod map;
mod game;
mod profile;
mod prologue;
mod shrine;
mod side;
mod touch;
mod world;
#[cfg(test)]
mod shots;

pub struct App;

impl App {
    fn new(ctx: &mut Context) -> impl Drawable {
        let mut canvas = Canvas::new(ctx, CanvasMode::Landscape);
        game::setup(&mut canvas);
        canvas
    }
}

ramp::run! { []; |ctx: &mut Context| { App::new(ctx) } }
