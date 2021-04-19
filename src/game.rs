use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use web_sys::HtmlImageElement;

use crate::{
    browser,
    engine::{self, Game, KeyState, Point, Rect, Renderer},
};

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

const GRAVITY: i16 = 1;

enum RedHatBoy {
    Idle,
    Running,
    Jumping,
    Sliding,
}

pub struct WalkTheDog {
    image: Option<HtmlImageElement>,
    background: Option<HtmlImageElement>,
    sheet: Option<Sheet>,
    frame: u8,
    position: Point,
    velocity: Point,
    state: RedHatBoy,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog {
            image: None,
            sheet: None,
            frame: 0,
            background: None,
            position: Point { x: 0, y: 464 },
            velocity: Point { x: 0, y: 0 },
            state: RedHatBoy::Idle,
        }
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&mut self) -> Result<()> {
        let json = browser::fetch_json("rhb.json").await?;

        self.sheet = json.into_serde()?;
        self.image = Some(engine::load_image("rhb.png").await?);

        self.background = Some(engine::load_image("BG.png").await?);

        Ok(())
    }

    fn update(&mut self, keystate: &KeyState) {
        let frame_count = match &self.state {
            RedHatBoy::Idle => 10,
            RedHatBoy::Running => 8,
            RedHatBoy::Jumping => 12,
            RedHatBoy::Sliding => 5,
        };

        match &self.state {
            RedHatBoy::Idle => {
                if keystate.is_pressed("ArrowRight") {
                    self.state = RedHatBoy::Running;
                    self.frame = 0;
                }
            }
            RedHatBoy::Running => {
                if keystate.is_pressed("Space") {
                    self.velocity.y = -25;
                    self.state = RedHatBoy::Jumping;
                    self.frame = 0;
                }
                if keystate.is_pressed("ArrowDown") {
                    self.state = RedHatBoy::Sliding;
                    self.frame = 0;
                }
            }
            RedHatBoy::Jumping => {
                self.velocity.y += GRAVITY;
                if self.position.y >= 478 {
                    self.velocity.y = 0;
                    self.position.y = 478;
                    self.state = RedHatBoy::Running;
                    self.frame = 0;
                }
            }
            RedHatBoy::Sliding => {
                if self.frame >= (frame_count * 3) - 1 {
                    self.frame = 0;
                    self.state = RedHatBoy::Idle;
                }
            }
        }

        self.position.x += self.velocity.x;
        self.position.y = self.position.y + self.velocity.y;

        // Run at 20 FPS for the animation, not 60
        if self.frame < ((frame_count * 3) - 1) {
            self.frame += 1;
        } else {
            self.frame = 0;
        }
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 600.0,
        });

        self.draw_background(renderer);

        let prefix = match &self.state {
            RedHatBoy::Idle => "Idle",
            RedHatBoy::Running => "Run",
            RedHatBoy::Jumping => "Jump",
            RedHatBoy::Sliding => "Slide",
        };
        let frame_name = format!("({}).png", (self.frame / 3) + 1);
        let frame_name = format!("{} {}", prefix, frame_name);
        let sprite = self
            .sheet
            .as_ref()
            .and_then(|sheet| sheet.frames.get(&frame_name))
            .expect("Cell not found");

        self.image.as_ref().map(|image| {
            renderer.draw_image(
                &image,
                &Rect {
                    x: sprite.frame.x.into(),
                    y: sprite.frame.y.into(),
                    width: sprite.frame.w.into(),
                    height: sprite.frame.h.into(),
                },
                &Rect {
                    x: self.position.x.into(),
                    y: self.position.y.into(),
                    width: sprite.frame.w.into(),
                    height: sprite.frame.h.into(),
                },
            );
        });
    }
}

impl WalkTheDog {
    fn draw_background(&self, renderer: &Renderer) {
        if let Some(background) = &self.background {
            renderer.draw_image(
                &background,
                &Rect {
                    x: 0.0,
                    y: 51.0,
                    width: 600.0,
                    height: 600.0,
                },
                &Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 600.0,
                    height: 600.0,
                },
            );
        }
    }
}
