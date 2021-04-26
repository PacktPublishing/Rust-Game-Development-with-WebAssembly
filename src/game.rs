use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use web_sys::HtmlImageElement;

use crate::{
    browser,
    engine::{self, Game, KeyState, Point, Rect, Renderer, Sprite, Vector},
};

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

const GRAVITY: f32 = 1.5;

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
    velocity: Vector,
    state: RedHatBoy,
    sprite: Option<Sprite>,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog {
            image: None,
            sheet: None,
            frame: 0,
            background: None,
            position: Point { x: 0, y: 485 },
            velocity: Vector { x: 0.0, y: 0.0 },
            state: RedHatBoy::Idle,
            sprite: None,
        }
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&mut self) -> Result<()> {
        let json = browser::fetch_json("rhb.json").await?;

        let sheet = json.into_serde()?;
        let image = engine::load_image("rhb.png").await?;

        self.background = Some(engine::load_image("BG.png").await?);

        self.sprite = Some(Sprite::new(
            image,
            sheet,
            vec![
                "Idle".to_string(),
                "Run".to_string(),
                "Jump".to_string(),
                "Slide".to_string(),
            ],
        ));

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
                    self.velocity.x = 4.0;
                    self.frame = 0;
                }

                if keystate.is_pressed("ArrowLeft") {
                    self.state = RedHatBoy::Running;
                    self.velocity.x = -4.0;
                    self.frame = 0;
                }
            }
            RedHatBoy::Running => {
                if keystate.is_pressed("Space") {
                    self.velocity.y = -25.0;
                    self.state = RedHatBoy::Jumping;
                    self.frame = 0;
                }
                if keystate.is_pressed("ArrowDown") {
                    self.state = RedHatBoy::Sliding;
                    self.frame = 0;
                }
                if keystate.is_pressed("ArrowRight") {
                    if self.velocity.x != 4.0 {
                        self.velocity.x = 4.0;
                        self.frame = 0;
                    }
                }
                if keystate.is_pressed("ArrowLeft") {
                    if self.velocity.x != -4.0 {
                        self.velocity.x = -4.0;
                        self.frame = 0;
                    }
                }
            }
            RedHatBoy::Jumping => {
                self.velocity.y += GRAVITY;
                if self.position.y >= 478 {
                    self.velocity.y = 0.0;
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

        self.position.x += self.velocity.x as i16;
        self.position.y = self.position.y + self.velocity.y as i16;

        // Run at 20 FPS for the animation, not 60
        if self.frame < (frame_count * 3) - 1 {
            self.frame += 1;
        } else {
            self.frame = 0;
        }
        log!("Frame is {}", self.frame);
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 600.0,
        });

        self.draw_background(renderer);

        let animation = match &self.state {
            RedHatBoy::Idle => "Idle",
            RedHatBoy::Running => "Run",
            RedHatBoy::Jumping => "Jump",
            RedHatBoy::Sliding => "Slide",
        };

        if let Some(sprite) = &self.sprite {
            sprite.draw(
                renderer,
                animation,
                &(self.frame / 3).into(),
                &self.position,
            );
        }
        let additional_offset_y = match self.state {
            RedHatBoy::Sliding => 15,
            _ => 0,
        };
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
