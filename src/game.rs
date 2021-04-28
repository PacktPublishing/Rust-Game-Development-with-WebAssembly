use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use std::{collections::HashMap, mem::swap};
use web_sys::HtmlImageElement;

use crate::{
    browser,
    engine::{self, Game, KeyState, Point, Rect, Renderer, SpriteSheet, Vector},
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

struct RedHatBoyMachine<S> {
    frame: u8,
    velocity: Vector,
    position: Point,
    state: S,
}

struct Idle;
struct Jumping;
struct Running;
struct Sliding;

impl RedHatBoyMachine<Idle> {
    fn new() -> Self {
        RedHatBoyMachine {
            frame: 0,
            velocity: Vector { x: 0.0, y: 0.0 },
            position: Point { x: 0, y: 485 },
            state: Idle {},
        }
    }
}

impl From<RedHatBoyMachine<Idle>> for RedHatBoyMachine<Running> {
    fn from(machine: RedHatBoyMachine<Idle>) -> Self {
        RedHatBoyMachine {
            frame: 0,
            position: machine.position,
            velocity: Vector { x: 4.0, y: 0.0 },
            state: Running {},
        }
    }
}

impl From<RedHatBoyMachine<Running>> for RedHatBoyMachine<Sliding> {
    fn from(machine: RedHatBoyMachine<Running>) -> Self {
        RedHatBoyMachine {
            frame: 0,
            position: machine.position,
            velocity: machine.velocity,
            state: Sliding {},
        }
    }
}

impl From<RedHatBoyMachine<Running>> for RedHatBoyMachine<Jumping> {
    fn from(machine: RedHatBoyMachine<Running>) -> Self {
        RedHatBoyMachine {
            frame: 0,
            position: machine.position,
            velocity: Vector {
                x: machine.velocity.x,
                y: -25.0,
            },
            state: Jumping {},
        }
    }
}

impl From<RedHatBoyMachine<Jumping>> for RedHatBoyMachine<Running> {
    fn from(machine: RedHatBoyMachine<Jumping>) -> Self {
        RedHatBoyMachine {
            frame: 0,
            position: machine.position,
            velocity: Vector {
                x: machine.velocity.x,
                y: 0.0,
            },
            state: Running {},
        }
    }
}

impl From<RedHatBoyMachine<Sliding>> for RedHatBoyMachine<Running> {
    fn from(machine: RedHatBoyMachine<Sliding>) -> Self {
        RedHatBoyMachine {
            frame: 0,
            position: machine.position,
            velocity: Vector {
                x: machine.velocity.x,
                y: 0.0,
            },
            state: Running {},
        }
    }
}

enum RedHatBoyWrapper {
    Idle(RedHatBoyMachine<Idle>),
    Running(RedHatBoyMachine<Running>),
    Jumping(RedHatBoyMachine<Jumping>),
    Sliding(RedHatBoyMachine<Sliding>),
}

impl RedHatBoyWrapper {
    fn frame_count(&self) -> u8 {
        match self {
            RedHatBoyWrapper::Idle(_) => 10,
            RedHatBoyWrapper::Running(_) => 8,
            RedHatBoyWrapper::Jumping(_) => 12,
            RedHatBoyWrapper::Sliding(_) => 5,
        }
    }

    fn animation(&self) -> &str {
        match self {
            RedHatBoyWrapper::Idle(_) => "Idle",
            RedHatBoyWrapper::Running(_) => "Run",
            RedHatBoyWrapper::Jumping(_) => "Jump",
            RedHatBoyWrapper::Sliding(_) => "Slide",
        }
    }

    fn frame(&self) -> u8 {
        match self {
            RedHatBoyWrapper::Idle(val) => val.frame,
            RedHatBoyWrapper::Running(val) => val.frame,
            RedHatBoyWrapper::Jumping(val) => val.frame,
            RedHatBoyWrapper::Sliding(val) => val.frame,
        }
    }

    fn position(&self) -> &Point {
        match self {
            RedHatBoyWrapper::Idle(val) => &val.position,
            RedHatBoyWrapper::Running(val) => &val.position,
            RedHatBoyWrapper::Jumping(val) => &val.position,
            RedHatBoyWrapper::Sliding(val) => &val.position,
        }
    }

    fn run(mut self) -> Self {
        self = match self {
            RedHatBoyWrapper::Idle(val) => RedHatBoyWrapper::Running(val.into()),
            RedHatBoyWrapper::Running(mut val) => {
                val.velocity.x += 4.0;
                if val.velocity.x > 4.0 {
                    val.velocity.x = 4.0;
                }
                RedHatBoyWrapper::Running(val)
            }
            RedHatBoyWrapper::Jumping(_) | RedHatBoyWrapper::Sliding(_) => self,
        };
        self
    }

    fn jump(mut self) -> Self {
        self = match self {
            RedHatBoyWrapper::Running(val) => RedHatBoyWrapper::Jumping(val.into()),
            RedHatBoyWrapper::Idle(_)
            | RedHatBoyWrapper::Jumping(_)
            | RedHatBoyWrapper::Sliding(_) => self,
        };
        self
    }

    fn slide(mut self) -> Self {
        self = match self {
            RedHatBoyWrapper::Running(val) => RedHatBoyWrapper::Sliding(val.into()),
            RedHatBoyWrapper::Idle(_)
            | RedHatBoyWrapper::Jumping(_)
            | RedHatBoyWrapper::Sliding(_) => self,
        };
        self
    }

    fn moonwalk(mut self) -> Self {
        self = match self {
            RedHatBoyWrapper::Running(mut val) => {
                val.velocity.x -= 4.0;
                // TODO Update Rust, use f32::clamp
                if val.velocity.x < -4.0 {
                    val.velocity.x = -4.0;
                }
                RedHatBoyWrapper::Running(val)
            }
            RedHatBoyWrapper::Idle(_)
            | RedHatBoyWrapper::Jumping(_)
            | RedHatBoyWrapper::Sliding(_) => self,
        };
        self
    }

    fn update(mut self) -> Self {
        let frame_count = self.frame_count();

        self = match self {
            RedHatBoyWrapper::Jumping(mut val) => {
                val.velocity.y += GRAVITY;
                val.position.x += val.velocity.x as i16;
                val.position.y = val.position.y + val.velocity.y as i16;
                if val.frame < (frame_count * 3) - 1 {
                    val.frame += 1;
                } else {
                    val.frame = 0;
                }

                if val.position.y >= 478 {
                    RedHatBoyWrapper::Running(val.into())
                } else {
                    RedHatBoyWrapper::Jumping(val)
                }
            }
            RedHatBoyWrapper::Sliding(mut val) => {
                val.position.x += val.velocity.x as i16;
                val.position.y = val.position.y + val.velocity.y as i16;
                if val.frame < (frame_count * 3) - 1 {
                    val.frame += 1;
                } else {
                    val.frame = 0;
                }

                if val.frame >= (frame_count * 3) - 1 {
                    RedHatBoyWrapper::Running(val.into())
                } else {
                    RedHatBoyWrapper::Sliding(val)
                }
            }
            RedHatBoyWrapper::Idle(mut val) => {
                val.position.x += val.velocity.x as i16;
                val.position.y = val.position.y + val.velocity.y as i16;
                if val.frame < (frame_count * 3) - 1 {
                    val.frame += 1;
                } else {
                    val.frame = 0;
                }

                RedHatBoyWrapper::Idle(val)
            }
            RedHatBoyWrapper::Running(mut val) => {
                val.position.x += val.velocity.x as i16;
                val.position.y = val.position.y + val.velocity.y as i16;
                if val.frame < (frame_count * 3) - 1 {
                    val.frame += 1;
                } else {
                    val.frame = 0;
                }

                RedHatBoyWrapper::Running(val)
            }
        };

        self
    }
}

pub struct WalkTheDog {
    background: Option<HtmlImageElement>,
    sprite: Option<SpriteSheet>,
    state_machine: RedHatBoyWrapper,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog {
            background: None,
            sprite: None,
            state_machine: RedHatBoyWrapper::Idle(RedHatBoyMachine::new()),
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

        self.sprite = Some(SpriteSheet::new(
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
        let frame_count = self.state_machine.frame_count();

        let mut machine = RedHatBoyWrapper::Idle(RedHatBoyMachine::new());
        swap(&mut machine, &mut self.state_machine);

        if keystate.is_pressed("ArrowRight") {
            machine = machine.run();
        }

        if keystate.is_pressed("ArrowLeft") {
            machine = machine.moonwalk();
        }

        if keystate.is_pressed("Space") {
            machine = machine.jump();
        }

        if keystate.is_pressed("ArrowDown") {
            machine = machine.slide();
        }

        machine = machine.update();
        self.state_machine = machine;
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 600.0,
        });

        self.draw_background(renderer);

        let animation = &self.state_machine.animation();

        if let Some(sprite) = &self.sprite {
            sprite.draw(
                renderer,
                animation,
                &(self.state_machine.frame() / 3).into(),
                &self.state_machine.position(),
            );
        }
        /*
        let additional_offset_y = match self.state {
            RedHatBoy::Sliding => 15,
            _ => 0,
        };
        */
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
