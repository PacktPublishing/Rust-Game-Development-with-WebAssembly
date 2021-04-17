use crate::browser::{self, LoopClosure};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::channel::oneshot::channel;
use std::{cell::RefCell, rc::Rc, sync::Mutex};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlImageElement};

pub struct Point {
    pub x: i16,
    pub y: i16,
}

pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct Renderer {
    context: CanvasRenderingContext2d,
}

impl Renderer {
    pub fn clear(&self, rect: &Rect) {
        self.context.clear_rect(
            rect.x.into(),
            rect.y.into(),
            rect.width.into(),
            rect.height.into(),
        );
    }

    pub fn draw_image(&self, image: &HtmlImageElement, frame: &Rect, destination: &Rect) {
        self.context
            .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &image,
                frame.x.into(),
                frame.y.into(),
                frame.width.into(),
                frame.height.into(),
                destination.x.into(),
                destination.y.into(),
                destination.width.into(),
                destination.height.into(),
            );
    }
}

pub async fn load_image(source: &str) -> Result<HtmlImageElement> {
    let image = browser::new_image()?;

    let (complete_tx, complete_rx) = channel::<Result<()>>();
    let success_tx = Rc::new(Mutex::new(Some(complete_tx)));
    let error_tx = Rc::clone(&success_tx);
    let success_callback = browser::closure_once(move || {
        if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
            success_tx.send(Ok(()));
        }
    });

    let error_callback: Closure<dyn FnMut(JsValue)> = browser::closure_once(move |err| {
        if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
            error_tx.send(Err(anyhow!("Error Loading Image: {:#?}", err)));
        }
    });

    image.set_onload(Some(success_callback.as_ref().unchecked_ref()));
    image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));
    image.set_src(source);

    complete_rx.await?;

    Ok(image)
}

// Sixty Frames per second, converted to a frame length in milliseconds
const FRAME_SIZE: f64 = 1.0 / 60.0 * 1000.0;
type SharedLoopClosure = Rc<RefCell<Option<LoopClosure>>>;

#[async_trait(?Send)]
pub trait Game {
    async fn initialize(&mut self) -> Result<()>;
    fn update(&mut self);
    fn draw(&self, context: &Renderer);
}

pub struct GameLoop {
    last_update: f64,
}

impl GameLoop {
    pub async fn start(mut game: impl Game + 'static) -> Result<()> {
        game.initialize().await?;

        let f: SharedLoopClosure = Rc::new(RefCell::new(None));
        let g = f.clone();

        let mut game_loop = GameLoop {
            last_update: browser::now()?,
        };

        let renderer = Renderer {
            context: browser::context().expect("No context found"),
        };

        *g.borrow_mut() = Some(browser::loop_fn(move |perf: f64| {
            let mut difference = perf - game_loop.last_update;
            while difference > 0.0 {
                game.update();
                difference -= FRAME_SIZE;
            }
            game_loop.last_update = perf;
            game.draw(&renderer);

            browser::request_animation_frame(f.borrow().as_ref().unwrap());
        }));

        browser::request_animation_frame(
            g.borrow()
                .as_ref()
                .ok_or(anyhow!("GameLoop: Loop is None"))?,
        )?;
        Ok(())
    }
}
