#[macro_use]
mod browser;
mod engine;

use engine::{Game, GameLoop, Rect, Renderer};
use serde::Deserialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::HtmlImageElement;

#[derive(Deserialize)]
struct SheetRect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[derive(Deserialize)]
struct Cell {
    frame: SheetRect,
}

#[derive(Deserialize)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

struct WalkTheDog {
    image: HtmlImageElement,
    sheet: Sheet,
    frame_count: u8,
    current_frame: u8,
}

impl WalkTheDog {
    fn new(image: HtmlImageElement, sheet: Sheet) -> Self {
        WalkTheDog {
            image,
            sheet,
            current_frame: 0,
            frame_count: 0,
        }
    }
}

impl Game for WalkTheDog {
    fn update(&mut self) {
        if self.frame_count < 24 {
            self.frame_count += 1;
        } else {
            self.frame_count = 0;
        }
        if (self.frame_count % 3) == 0 {
            self.current_frame = (self.current_frame + 1) % 8;
        }
    }

    fn draw(&self, renderer: &Renderer) {
        let frame_name = format!("Run ({}).png", self.current_frame + 1);
        let sprite = self.sheet.frames.get(&frame_name).expect("Cell not found");

        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 600.0,
        });
        renderer.draw_image(
            &self.image,
            &Rect {
                x: sprite.frame.x.into(),
                y: sprite.frame.y.into(),
                width: sprite.frame.w.into(),
                height: sprite.frame.h.into(),
            },
            &Rect {
                x: 300.0,
                y: 300.0,
                width: sprite.frame.w.into(),
                height: sprite.frame.h.into(),
            },
        );
    }
}

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let context = browser::context()?;

    browser::spawn_local(async move {
        let json = browser::fetch_json("rhb.json")
            .await
            .expect("Could not fetch rhb.json");

        let sheet: Sheet = json
            .into_serde()
            .expect("Could not convert rhb.json into a Sheet structure");

        let image = engine::load_image("rhb.png")
            .await
            .expect("Could not load rhb.png");

        let game = WalkTheDog::new(image, sheet);

        GameLoop::start(game).expect("Could not start game loop");
    });

    Ok(())
}
