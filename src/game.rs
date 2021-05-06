use anyhow::Result;
use async_trait::async_trait;
use std::mem::swap;
use web_sys::HtmlImageElement;

use crate::{
    browser,
    engine::{self, Game, KeyState, Point, Rect, Renderer, SpriteSheet, Vector},
};

const GRAVITY: f32 = 1.5;
const FLOOR: i16 = 485;

pub struct WalkTheDog {
    background: Option<HtmlImageElement>,
    sprite: Option<SpriteSheet>,
    rhb: RedHatBoy,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog {
            background: None,
            sprite: None,
            rhb: RedHatBoy::new(),
        }
    }

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
        if keystate.is_pressed("ArrowRight") {
            self.rhb.run();
        }

        if keystate.is_pressed("ArrowLeft") {
            self.rhb.moonwalk();
        }

        if keystate.is_pressed("Space") {
            self.rhb.jump();
        }

        if keystate.is_pressed("ArrowDown") {
            self.rhb.slide();
        }

        self.rhb.update();
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 600.0,
        });

        self.draw_background(renderer);

        let animation = &self.rhb.animation();

        if let Some(sprite) = &self.sprite {
            sprite.draw(
                renderer,
                animation,
                &(self.rhb.frame() / 3).into(),
                &self.rhb.position(),
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

struct RedHatBoy {
    state: RedHatBoyWrapper,
}

impl RedHatBoy {
    fn new() -> Self {
        RedHatBoy {
            state: RedHatBoyWrapper::Idle(RedHatBoyMachine::new()),
        }
    }

    fn animation(&self) -> &str {
        self.state.animation()
    }

    fn frame(&self) -> u8 {
        self.state.game_object().frame
    }

    fn position(&self) -> &Point {
        &self.state.game_object().position
    }

    fn run(&mut self) {
        let mut machine = RedHatBoyWrapper::Idle(RedHatBoyMachine::new());
        swap(&mut machine, &mut self.state);

        self.state = machine.run();
    }

    fn moonwalk(&mut self) {
        let mut machine = RedHatBoyWrapper::Idle(RedHatBoyMachine::new());
        swap(&mut machine, &mut self.state);

        self.state = machine.moonwalk();
    }

    fn jump(&mut self) {
        let mut machine = RedHatBoyWrapper::Idle(RedHatBoyMachine::new());
        swap(&mut machine, &mut self.state);

        self.state = machine.jump();
    }

    fn slide(&mut self) {
        let mut machine = RedHatBoyWrapper::Idle(RedHatBoyMachine::new());
        swap(&mut machine, &mut self.state);

        self.state = machine.slide();
    }

    fn update(&mut self) {
        let mut machine = RedHatBoyWrapper::Idle(RedHatBoyMachine::new());
        swap(&mut machine, &mut self.state);

        self.state = machine.update();
    }
}

enum RedHatBoyWrapper {
    Idle(RedHatBoyMachine<Idle>),
    Running(RedHatBoyMachine<Running>),
    Jumping(RedHatBoyMachine<Jumping>),
    Sliding(RedHatBoyMachine<Sliding>),
}

impl RedHatBoyWrapper {
    fn game_object(&self) -> &GameObject {
        match self {
            RedHatBoyWrapper::Idle(val) => &val.object,
            RedHatBoyWrapper::Running(val) => &val.object,
            RedHatBoyWrapper::Jumping(val) => &val.object,
            RedHatBoyWrapper::Sliding(val) => &val.object,
        }
    }

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

    fn run(self) -> Self {
        match self {
            RedHatBoyWrapper::Idle(val) => RedHatBoyWrapper::Running(val.into()),
            RedHatBoyWrapper::Running(val) => RedHatBoyWrapper::Running(val.go_right()),
            _ => self,
        }
    }

    fn jump(self) -> Self {
        match self {
            RedHatBoyWrapper::Running(val) => RedHatBoyWrapper::Jumping(val.into()),
            _ => self,
        }
    }

    fn slide(self) -> Self {
        match self {
            RedHatBoyWrapper::Running(val) => RedHatBoyWrapper::Sliding(val.into()),
            _ => self,
        }
    }

    fn moonwalk(self) -> Self {
        match self {
            RedHatBoyWrapper::Running(val) => RedHatBoyWrapper::Running(val.go_left()),
            _ => self,
        }
    }

    fn update(self) -> Self {
        let frame_count = self.frame_count();

        match self {
            RedHatBoyWrapper::Jumping(mut val) => {
                val.object = val.object.apply_gravity().update(frame_count);

                if val.object.landed() {
                    RedHatBoyWrapper::Running(val.into())
                } else {
                    RedHatBoyWrapper::Jumping(val)
                }
            }
            RedHatBoyWrapper::Sliding(mut val) => {
                val.object = val.object.update(frame_count);

                if val.object.animation_finished(frame_count) {
                    RedHatBoyWrapper::Running(val.into())
                } else {
                    RedHatBoyWrapper::Sliding(val)
                }
            }
            RedHatBoyWrapper::Idle(mut val) => {
                val.object = val.object.update(frame_count);

                RedHatBoyWrapper::Idle(val)
            }
            RedHatBoyWrapper::Running(mut val) => {
                val.object = val.object.update(frame_count);

                RedHatBoyWrapper::Running(val)
            }
        }
    }
}

struct RedHatBoyMachine<S> {
    _state: S,
    object: GameObject,
}

struct Idle;
struct Jumping;
struct Running;
struct Sliding;

impl RedHatBoyMachine<Idle> {
    fn new() -> Self {
        let game_object = GameObject {
            frame: 0,
            position: engine::Point { x: 0, y: FLOOR },
            velocity: Vector { x: 0.0, y: 0.0 },
        };

        RedHatBoyMachine {
            _state: Idle {},
            object: game_object,
        }
    }
}

impl From<RedHatBoyMachine<Idle>> for RedHatBoyMachine<Running> {
    fn from(machine: RedHatBoyMachine<Idle>) -> Self {
        RedHatBoyMachine {
            _state: Running {},
            object: machine.object.go_right(),
        }
    }
}

impl RedHatBoyMachine<Running> {
    fn go_right(mut self) -> Self {
        self.object = self.object.go_right();
        self
    }

    fn go_left(mut self) -> Self {
        self.object = self.object.go_left();
        self
    }
}

impl From<RedHatBoyMachine<Running>> for RedHatBoyMachine<Sliding> {
    fn from(machine: RedHatBoyMachine<Running>) -> Self {
        RedHatBoyMachine {
            _state: Sliding {},
            object: machine.object.reset_frame(),
        }
    }
}

impl From<RedHatBoyMachine<Running>> for RedHatBoyMachine<Jumping> {
    fn from(machine: RedHatBoyMachine<Running>) -> Self {
        RedHatBoyMachine {
            _state: Jumping {},
            object: machine.object.jump(),
        }
    }
}

impl From<RedHatBoyMachine<Jumping>> for RedHatBoyMachine<Running> {
    fn from(machine: RedHatBoyMachine<Jumping>) -> Self {
        RedHatBoyMachine {
            _state: Running {},
            object: machine.object.land(),
        }
    }
}

impl From<RedHatBoyMachine<Sliding>> for RedHatBoyMachine<Running> {
    fn from(machine: RedHatBoyMachine<Sliding>) -> Self {
        RedHatBoyMachine {
            _state: Running {},
            object: machine.object.reset_frame(),
        }
    }
}

struct GameObject {
    frame: u8,
    position: Point,
    velocity: Vector,
}

impl GameObject {
    fn go_left(mut self) -> GameObject {
        self.velocity.x -= 4.0;
        if self.velocity.x < -4.0 {
            self.velocity.x = -4.0;
        };
        self
    }

    fn go_right(mut self) -> GameObject {
        self.velocity.x += 4.0;
        if self.velocity.x > 4.0 {
            self.velocity.x = 4.0;
        };
        self
    }

    fn jump(self) -> GameObject {
        let mut jumping = self.reset_frame();
        jumping.velocity.y = -25.0;
        jumping
    }

    fn apply_gravity(mut self) -> Self {
        self.velocity.y += GRAVITY;
        self
    }

    fn update(mut self, frame_count: u8) -> Self {
        self.position.x += self.velocity.x as i16;
        self.position.y = self.position.y + self.velocity.y as i16;
        if self.frame < (frame_count * 3) - 1 {
            self.frame += 1;
        } else {
            self.frame = 0;
        };
        self
    }

    fn landed(&self) -> bool {
        self.position.y >= FLOOR
    }

    fn land(self) -> Self {
        let mut landed = self.reset_frame();
        landed.velocity.y = 0.0;
        landed.position.y = FLOOR;
        landed
    }

    fn reset_frame(mut self) -> Self {
        self.frame = 0;
        self
    }

    fn animation_finished(&self, frame_count: u8) -> bool {
        self.frame >= (frame_count * 3) - 1
    }
}
