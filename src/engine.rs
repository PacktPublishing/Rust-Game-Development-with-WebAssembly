use crate::browser::{self, LoopClosure};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::channel::{
    mpsc::{unbounded, UnboundedReceiver},
    oneshot::channel,
};
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Mutex};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlImageElement};

#[derive(Debug)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

#[derive(Debug)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
}

pub struct Dimen {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn intersects(&self, rect: &Rect) -> bool {
        self.x < (rect.x + rect.width)
            && self.x + self.width > rect.x
            && self.y < (rect.y + rect.height)
            && self.y + self.height > rect.y
    }
}

#[derive(Deserialize)]
struct SheetRect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cell {
    frame: SheetRect,
    sprite_source_size: SheetRect,
}

#[derive(Deserialize)]
pub struct Sheet {
    frames: HashMap<String, Cell>,
}

pub struct SpriteSheet {
    image: HtmlImageElement,
    sheet: Sheet,
    adjustments: HashMap<String, Vec<Point>>,
    pub dimensions: HashMap<String, Vec<Dimen>>,
}

impl SpriteSheet {
    pub fn new(image: HtmlImageElement, sheet: Sheet, animations: Vec<String>) -> Self {
        let mut adjustments = HashMap::new();
        let mut dimensions = HashMap::new();
        animations.iter().for_each(|animation| {
            if let Some(first_frame) = sheet.frames.get(&format!("{} (1).png", animation)) {
                let mut adjustments_vec = vec![Point { x: 0, y: 0 }];
                let mut dimensions_vec = vec![Dimen {
                    width: first_frame.sprite_source_size.w.into(),
                    height: first_frame.sprite_source_size.h.into(),
                }];

                let mut frame_index = 2;
                while let Some(frame) = sheet
                    .frames
                    .get(&format!("{} ({}).png", animation, frame_index))
                {
                    let adjustment = Point {
                        x: frame.sprite_source_size.x as i16
                            - first_frame.sprite_source_size.x as i16,
                        y: frame.sprite_source_size.y as i16
                            - first_frame.sprite_source_size.y as i16,
                    };
                    adjustments_vec.push(adjustment);
                    dimensions_vec.push(Dimen {
                        width: frame.sprite_source_size.w.into(),
                        height: frame.sprite_source_size.h.into(),
                    });
                    frame_index += 1;
                }
                adjustments.insert(animation.into(), adjustments_vec);
                dimensions.insert(animation.into(), dimensions_vec);
            }
        });

        SpriteSheet {
            image,
            sheet,
            adjustments,
            dimensions,
        }
    }

    pub fn draw(&self, renderer: &Renderer, animation: &str, frame: &i16, position: &Point) {
        let cell = format!("{} ({}).png", animation, frame + 1);
        let sprite = self
            .sheet
            .frames
            .get(&cell)
            .expect(&format!("Cell {} not found", cell));

        let adjustment = self
            .adjustments
            .get(animation)
            .and_then(|adjustments| adjustments.get(*frame as usize))
            .unwrap_or(&Point { x: 0, y: 0 });

        renderer.draw_image(
            &self.image,
            &Rect {
                x: sprite.frame.x.into(),
                y: sprite.frame.y.into(),
                width: sprite.frame.w.into(),
                height: sprite.frame.h.into(),
            },
            &Rect {
                x: (position.x as i16 + adjustment.x).into(),
                y: (position.y as i16 + adjustment.y).into(),
                width: sprite.frame.w.into(),
                height: sprite.frame.h.into(),
            },
        );
    }
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
            )
            .expect("Drawing is throwing exceptions! Unrecoverable error.");
    }

    pub fn draw_rect(&self, color: &str, rect: &Rect) {
        self.context.set_stroke_style(&JsValue::from_str(color));
        self.context.begin_path();
        self.context.rect(
            rect.x.into(),
            rect.y.into(),
            rect.width.into(),
            rect.height.into(),
        );
        self.context.stroke();
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

    complete_rx.await??;

    Ok(image)
}

// Sixty Frames per second, converted to a frame length in milliseconds
const FRAME_SIZE: f32 = 1.0 / 60.0 * 1000.0;

#[async_trait(?Send)]
pub trait Game {
    async fn initialize(&mut self) -> Result<()>;
    fn update(&mut self, keystate: &KeyState);
    fn draw(&self, context: &Renderer);
}

pub struct GameLoop {
    last_frame: f64,
    accumulated_delta: f32,
}
type SharedLoopClosure = Rc<RefCell<Option<LoopClosure>>>;

impl GameLoop {
    pub async fn start(mut game: impl Game + 'static) -> Result<()> {
        let mut keyevent_receiver = prepare_input()?;
        game.initialize().await?;

        let mut game_loop = GameLoop {
            last_frame: browser::now()?,
            accumulated_delta: 0.0,
        };

        let renderer = Renderer {
            context: browser::context()?,
        };

        let f: SharedLoopClosure = Rc::new(RefCell::new(None));
        let g = f.clone();

        let mut keystate = KeyState::new();
        *g.borrow_mut() = Some(browser::create_raf_closure(move |perf: f64| {
            process_input(&mut keystate, &mut keyevent_receiver);

            game_loop.accumulated_delta += (perf - game_loop.last_frame) as f32;
            while game_loop.accumulated_delta > FRAME_SIZE {
                game.update(&keystate);
                game_loop.accumulated_delta -= FRAME_SIZE;
            }
            game_loop.last_frame = perf;
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

pub struct KeyState {
    pressed_keys: HashMap<String, web_sys::KeyboardEvent>,
}

impl KeyState {
    fn new() -> Self {
        return KeyState {
            pressed_keys: HashMap::new(),
        };
    }

    pub fn is_pressed(&self, code: &str) -> bool {
        self.pressed_keys.contains_key(code)
    }

    fn set_pressed(&mut self, code: &str, event: web_sys::KeyboardEvent) {
        self.pressed_keys.insert(code.into(), event);
    }

    fn set_released(&mut self, code: &str) {
        self.pressed_keys.remove(code.into());
    }
}

enum KeyPress {
    KeyUp(web_sys::KeyboardEvent),
    KeyDown(web_sys::KeyboardEvent),
}

fn process_input(state: &mut KeyState, keyevent_receiver: &mut UnboundedReceiver<KeyPress>) {
    loop {
        match keyevent_receiver.try_next() {
            Ok(None) => break,
            Err(_err) => break,
            Ok(Some(evt)) => match evt {
                KeyPress::KeyUp(evt) => state.set_released(&evt.code()),
                KeyPress::KeyDown(evt) => state.set_pressed(&evt.code(), evt),
            },
        };
    }
}

fn prepare_input() -> Result<UnboundedReceiver<KeyPress>> {
    let (keydown_sender, keyevent_receiver) = unbounded();
    let keydown_sender = Rc::new(RefCell::new(keydown_sender));
    let keyup_sender = Rc::clone(&keydown_sender);
    let onkeydown = browser::closure_wrap(Box::new(move |keycode: web_sys::KeyboardEvent| {
        keydown_sender
            .borrow_mut()
            .start_send(KeyPress::KeyDown(keycode));
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

    let onkeyup = browser::closure_wrap(Box::new(move |keycode: web_sys::KeyboardEvent| {
        keyup_sender
            .borrow_mut()
            .start_send(KeyPress::KeyUp(keycode));
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

    browser::canvas()?.set_onkeydown(Some(onkeydown.as_ref().unchecked_ref()));
    browser::canvas()?.set_onkeyup(Some(onkeyup.as_ref().unchecked_ref()));
    onkeydown.forget();
    onkeyup.forget();

    Ok(keyevent_receiver)
}
