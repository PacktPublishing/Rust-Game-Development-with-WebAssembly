use anyhow::Result;
use async_trait::async_trait;
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
    state: Option<RedHatBoyStateMachine>,
}

impl RedHatBoy {
    fn new() -> Self {
        RedHatBoy {
            state: Some(RedHatBoyStateMachine::Idle(RedHatBoyState::new())),
        }
    }

    fn animation(&self) -> &str {
        self.state
            .as_ref()
            .map(|state| state.animation())
            .unwrap_or("")
    }

    fn frame(&self) -> u8 {
        self.state
            .as_ref()
            .map(|state| state.game_object().frame)
            .unwrap_or(0)
    }

    fn position(&self) -> &Point {
        self.state
            .as_ref()
            .map(|state| &state.game_object().position)
            .unwrap_or(&Point { x: 0, y: 0 })
    }

    fn run(&mut self) {
        if let Some(state) = self.state.take() {
            self.state.replace(state.run());
        }
    }

    fn moonwalk(&mut self) {
        if let Some(state) = self.state.take() {
            self.state.replace(state.moonwalk());
        }
    }

    fn jump(&mut self) {
        if let Some(state) = self.state.take() {
            self.state.replace(state.jump());
        }
    }

    fn slide(&mut self) {
        if let Some(state) = self.state.take() {
            self.state.replace(state.slide());
        }
    }

    fn update(&mut self) {
        if let Some(state) = self.state.take() {
            self.state.replace(state.update());
        }
    }
}

enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
    Jumping(RedHatBoyState<Jumping>),
    Sliding(RedHatBoyState<Sliding>),
}

impl RedHatBoyStateMachine {
    fn game_object(&self) -> &GameObject {
        match self {
            RedHatBoyStateMachine::Idle(val) => &val.object,
            RedHatBoyStateMachine::Running(val) => &val.object,
            RedHatBoyStateMachine::Jumping(val) => &val.object,
            RedHatBoyStateMachine::Sliding(val) => &val.object,
        }
    }

    fn frame_count(&self) -> u8 {
        match self {
            RedHatBoyStateMachine::Idle(_) => 10,
            RedHatBoyStateMachine::Running(_) => 8,
            RedHatBoyStateMachine::Jumping(_) => 12,
            RedHatBoyStateMachine::Sliding(_) => 5,
        }
    }

    fn animation(&self) -> &str {
        match self {
            RedHatBoyStateMachine::Idle(_) => "Idle",
            RedHatBoyStateMachine::Running(_) => "Run",
            RedHatBoyStateMachine::Jumping(_) => "Jump",
            RedHatBoyStateMachine::Sliding(_) => "Slide",
        }
    }

    fn run(self) -> Self {
        match self {
            RedHatBoyStateMachine::Idle(val) => RedHatBoyStateMachine::Running(val.into()),
            RedHatBoyStateMachine::Running(val) => RedHatBoyStateMachine::Running(val.go_right()),
            _ => self,
        }
    }

    fn jump(self) -> Self {
        match self {
            RedHatBoyStateMachine::Running(val) => RedHatBoyStateMachine::Jumping(val.into()),
            _ => self,
        }
    }

    fn slide(self) -> Self {
        match self {
            RedHatBoyStateMachine::Running(val) => RedHatBoyStateMachine::Sliding(val.into()),
            _ => self,
        }
    }

    fn moonwalk(self) -> Self {
        match self {
            RedHatBoyStateMachine::Running(val) => RedHatBoyStateMachine::Running(val.go_left()),
            _ => self,
        }
    }

    fn update(self) -> Self {
        let frame_count = self.frame_count();

        match self {
            RedHatBoyStateMachine::Jumping(mut val) => {
                val.object = val.object.apply_gravity().update(frame_count);

                if val.object.landed() {
                    RedHatBoyStateMachine::Running(val.into())
                } else {
                    RedHatBoyStateMachine::Jumping(val)
                }
            }
            RedHatBoyStateMachine::Sliding(mut val) => {
                val.object = val.object.update(frame_count);

                if val.object.animation_finished(frame_count) {
                    RedHatBoyStateMachine::Running(val.into())
                } else {
                    RedHatBoyStateMachine::Sliding(val)
                }
            }
            RedHatBoyStateMachine::Idle(mut val) => {
                val.object = val.object.update(frame_count);

                RedHatBoyStateMachine::Idle(val)
            }
            RedHatBoyStateMachine::Running(mut val) => {
                val.object = val.object.update(frame_count);

                RedHatBoyStateMachine::Running(val)
            }
        }
    }
}

struct RedHatBoyState<S> {
    _state: S,
    object: GameObject,
}

struct Idle;
struct Jumping;
struct Running;
struct Sliding;

impl RedHatBoyState<Idle> {
    fn new() -> Self {
        let game_object = GameObject {
            frame: 0,
            position: engine::Point { x: 0, y: FLOOR },
            velocity: Vector { x: 0.0, y: 0.0 },
        };

        RedHatBoyState {
            _state: Idle {},
            object: game_object,
        }
    }
}

impl From<RedHatBoyState<Idle>> for RedHatBoyState<Running> {
    fn from(machine: RedHatBoyState<Idle>) -> Self {
        RedHatBoyState {
            _state: Running {},
            object: machine.object.go_right(),
        }
    }
}

impl RedHatBoyState<Running> {
    fn go_right(mut self) -> Self {
        self.object = self.object.go_right();
        self
    }

    fn go_left(mut self) -> Self {
        self.object = self.object.go_left();
        self
    }
}

impl From<RedHatBoyState<Running>> for RedHatBoyState<Sliding> {
    fn from(machine: RedHatBoyState<Running>) -> Self {
        RedHatBoyState {
            _state: Sliding {},
            object: machine.object.reset_frame(),
        }
    }
}

impl From<RedHatBoyState<Running>> for RedHatBoyState<Jumping> {
    fn from(machine: RedHatBoyState<Running>) -> Self {
        RedHatBoyState {
            _state: Jumping {},
            object: machine.object.jump(),
        }
    }
}

impl From<RedHatBoyState<Jumping>> for RedHatBoyState<Running> {
    fn from(machine: RedHatBoyState<Jumping>) -> Self {
        RedHatBoyState {
            _state: Running {},
            object: machine.object.land(),
        }
    }
}

impl From<RedHatBoyState<Sliding>> for RedHatBoyState<Running> {
    fn from(machine: RedHatBoyState<Sliding>) -> Self {
        RedHatBoyState {
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
