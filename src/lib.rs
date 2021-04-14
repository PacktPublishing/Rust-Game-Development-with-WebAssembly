use std::collections::HashMap;

use engine::{Game, GameLoop};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlElement, HtmlImageElement};

#[macro_use]
mod browser;
mod engine;

#[derive(Deserialize)]
struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[derive(Deserialize)]
struct Cell {
    frame: Rect,
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
    frame: u8,
}

impl Game for WalkTheDog {
    fn update(&mut self) {
        self.frame = (self.frame + 1) % 8;
    }

    fn draw(&self, context: CanvasRenderingContext2d) {
        let frame_name = format!("Run ({}).png", self.frame + 1);
        let sprite = self.sheet.frames.get(&frame_name).expect("Cell not found");

        context.clear_rect(0.0, 0.0, 600.0, 600.0);
        context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &self.image,
            sprite.frame.x.into(),
            sprite.frame.y.into(),
            sprite.frame.w.into(),
            sprite.frame.h.into(),
            300.0,
            300.0,
            sprite.frame.w.into(),
            sprite.frame.h.into(),
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

        let game = WalkTheDog {
            image,
            sheet,
            frame: 0,
        };

        GameLoop::start(game).expect("Could not start game loop");
    });

    Ok(())
}
