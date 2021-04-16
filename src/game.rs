use std::collections::HashMap;

use serde::Deserialize;
use web_sys::HtmlImageElement;

use crate::engine::{Game, Rect, Renderer};

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
pub struct Sheet {
    frames: HashMap<String, Cell>,
}

pub struct WalkTheDog {
    image: HtmlImageElement,
    sheet: Sheet,
    frame_count: u8,
    current_frame: u8,
}

impl WalkTheDog {
    pub fn new(image: HtmlImageElement, sheet: Sheet) -> Self {
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
