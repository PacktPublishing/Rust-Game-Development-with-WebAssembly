use crate::browser;
use anyhow::{anyhow, Result};
use futures::channel::oneshot::channel;
use std::{cell::RefCell, rc::Rc, sync::Mutex};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlImageElement};

pub async fn load_image(source: &str) -> Result<HtmlImageElement> {
    let image = browser::new_image()
        .map_err(|js_value| anyhow!("Could not create image {:#?}", js_value))?;

    let (success_tx, success_rx) = channel::<Result<(), JsValue>>();
    let success_tx = Rc::new(Mutex::new(Some(success_tx)));
    let error_tx = Rc::clone(&success_tx);
    let success_callback = browser::closure_once(move || {
        if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
            success_tx.send(Ok(()));
        }
    });

    let error_callback = browser::closure_once(move |err| {
        if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
            error_tx.send(Err(err));
        }
    });

    image.set_onload(Some(success_callback.as_ref().unchecked_ref()));
    image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));
    image.set_src(source);

    success_rx
        .await?
        .map_err(|js_value| anyhow!("Error loading image {} err: {:#?}", source, js_value))?;

    Ok(image)
}

// Sixty Frames per second, converted to a frame length in milliseconds
const FRAME_SIZE: f64 = 1.0 / 60.0 * 1000.0;
type LoopClosure = Closure<dyn FnMut(f64)>;
type SharedLoopClosure = Rc<RefCell<Option<LoopClosure>>>;

pub trait Game {
    fn update(&mut self);
    fn draw(&self, context: CanvasRenderingContext2d);
}

pub struct GameLoop {
    last_update: f64,
}

impl GameLoop {
    pub fn start(mut game: impl Game + 'static) -> Result<()> {
        let f: SharedLoopClosure = Rc::new(RefCell::new(None));
        let g = f.clone();
        let mut game_loop = GameLoop {
            last_update: browser::now()
                .map_err(|err| anyhow!("browser::now failed! {:#?}", err))?,
        };

        *g.borrow_mut() = Some(loop_fn(move |perf: f64| {
            let mut difference = perf - game_loop.last_update;
            while difference > 0.0 {
                game.update();
                difference -= FRAME_SIZE;
            }
            game_loop.last_update = perf;
            game.draw(browser::context().expect("No context found"));
            browser::request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
        }));

        browser::request_animation_frame(
            g.borrow()
                .as_ref()
                .ok_or(anyhow!("GameLoop: Loop is None"))?
                .as_ref()
                .unchecked_ref(),
        )
        .map_err(|value| anyhow!("JS error: {:#?}", value))?;
        Ok(())
    }
}

fn loop_fn(f: impl FnMut(f64) + 'static) -> LoopClosure {
    browser::closure_wrap(Box::new(f))
}
