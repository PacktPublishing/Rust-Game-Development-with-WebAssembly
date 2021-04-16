#[macro_use]
mod browser;
mod engine;
mod game;

use engine::GameLoop;
use game::{Sheet, WalkTheDog};
use wasm_bindgen::prelude::*;

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

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
