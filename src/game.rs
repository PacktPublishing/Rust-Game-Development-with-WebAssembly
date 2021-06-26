use anyhow::Result;
use async_trait::async_trait;

use crate::{
    browser,
    engine::{self, Game, Image, KeyState, Point, Rect, Renderer, SpriteSheet, Vector},
};

const GRAVITY: f32 = 1.0;
const FLOOR: i16 = 600;

pub struct WalkTheDog {
    background: Option<Image>,
    rock: Option<Image>,
    rhb: Option<RedHatBoy>,
    platform: Option<SpriteSheet>,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog {
            background: None,
            rock: None,
            rhb: None,
            platform: None,
        }
    }

    fn draw_rock(&self, renderer: &Renderer) {
        if let Some(rock) = &self.rock {
            rock.draw(renderer);
        }
    }

    fn draw_background(&self, renderer: &Renderer) {
        if let Some(background) = &self.background {
            background.draw(renderer);
        }
    }

    fn draw_platform(&self, renderer: &Renderer) {
        if let Some(platform) = &self.platform {
            platform.draw(renderer, "13.png", &Point { x: 220, y: 400 });
            platform.draw(renderer, "14.png", &Point { x: 348, y: 400 });
            platform.draw(renderer, "15.png", &Point { x: 476, y: 400 });
        }
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&mut self) -> Result<()> {
        self.background = Some(Image::new(
            engine::load_image("BG.png").await?,
            Point { x: 0, y: 0 },
        ));

        self.rock = Some(Image::new(
            engine::load_image("Stone.png").await?,
            Point { x: 200, y: 546 },
        ));

        let json = browser::fetch_json("rhb.json").await?;
        let sheet = json.into_serde()?;
        let image = engine::load_image("rhb.png").await?;

        self.rhb = Some(RedHatBoy::new(SpriteSheet::new(image, sheet)));

        let json = browser::fetch_json("tiles.json").await?;
        let sheet = json.into_serde()?;
        let image = engine::load_image("tiles.png").await?;
        self.platform = Some(SpriteSheet::new(image, sheet));

        Ok(())
    }

    fn update(&mut self, keystate: &KeyState) {
        if keystate.is_pressed("ArrowRight") {
            self.rhb.as_mut().unwrap().run();
        }

        if keystate.is_pressed("ArrowLeft") {
            self.rhb.as_mut().unwrap().moonwalk();
        }

        if keystate.is_pressed("Space") {
            self.rhb.as_mut().unwrap().jump();
        }

        if keystate.is_pressed("ArrowDown") {
            self.rhb.as_mut().unwrap().slide();
        }

        self.rhb.as_mut().unwrap().update();

        let platform_box = Rect {
            x: 220.0,
            y: 400.0,
            width: 384.0,
            height: 128.0,
        };

        if self.rhb.as_ref().unwrap().landing_on(&platform_box) {
            self.rhb.as_mut().unwrap().land_on(platform_box.y as i16);
        }

        if self.rhb.as_ref().unwrap().landing() {
            self.rhb.as_mut().unwrap().land_on(FLOOR);
        }

        // Collisions
        if self
            .rhb
            .as_ref()
            .unwrap()
            .collides_with(&self.rock.as_ref().unwrap().bounding_box())
        {
            self.rhb.as_mut().unwrap().kill();
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
        self.draw_rock(renderer);

        self.rhb.as_ref().as_mut().unwrap().draw(renderer);

        self.draw_platform(renderer);
    }
}

struct RedHatBoy {
    state: RedHatBoyStateMachine,
    sprite_sheet: SpriteSheet,
}

impl RedHatBoy {
    fn new(sprite_sheet: SpriteSheet) -> Self {
        RedHatBoy {
            state: RedHatBoyStateMachine::Idle(RedHatBoyState::new()),
            sprite_sheet,
        }
    }

    fn draw(&self, renderer: &Renderer) {
        self.sprite_sheet.draw_frame(
            renderer,
            self.animation(),
            &(self.frame() / 3).into(),
            &self.position(),
        );
    }

    fn bounding_box(&self, sheet: &SpriteSheet) -> Rect {
        let bounding_box = sheet.bounding_box_for(self.animation(), &((self.frame() / 3) as i16));
        Rect {
            x: self.position().x as f32 + bounding_box.x,
            y: self.position().y as f32 + bounding_box.y,
            width: bounding_box.width,
            height: bounding_box.height,
        }
    }

    fn collides_with(&self, rect: &Rect) -> bool {
        self.bounding_box(&self.sprite_sheet).intersects(rect)
    }

    fn landing(&self) -> bool {
        self.position().y as f32 + self.bounding_box(&self.sprite_sheet).height > FLOOR as f32
    }

    fn landing_on(&self, rect: &Rect) -> bool {
        self.bounding_box(&self.sprite_sheet).intersects(rect)
            && (self.position().y as f32) < rect.y
    }

    fn land_on(&mut self, y: i16) {
        self.state = self
            .state
            .land((y as f32 - self.bounding_box(&self.sprite_sheet).height) as i16)
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
        self.state = self.state.run();
    }

    fn kill(&mut self) {
        self.state = self.state.kill();
    }

    fn moonwalk(&mut self) {
        self.state = self.state.moonwalk();
    }

    fn jump(&mut self) {
        self.state = self.state.jump();
    }

    fn slide(&mut self) {
        self.state = self.state.slide();
    }

    fn update(&mut self) {
        self.state = self.state.update();
    }
}

#[derive(Copy, Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
    Jumping(RedHatBoyState<Jumping>),
    Sliding(RedHatBoyState<Sliding>),
    Crashing(RedHatBoyState<Crashing>),
    GameOver(RedHatBoyState<GameOver>),
}

impl RedHatBoyStateMachine {
    fn game_object(&self) -> &GameObject {
        match self {
            RedHatBoyStateMachine::Idle(val) => &val.object,
            RedHatBoyStateMachine::Running(val) => &val.object,
            RedHatBoyStateMachine::Jumping(val) => &val.object,
            RedHatBoyStateMachine::Sliding(val) => &val.object,
            RedHatBoyStateMachine::Crashing(val) => &val.object,
            RedHatBoyStateMachine::GameOver(val) => &val.object,
        }
    }

    fn frame_count(&self) -> u8 {
        match self {
            RedHatBoyStateMachine::Idle(_) => 10,
            RedHatBoyStateMachine::Running(_) => 8,
            RedHatBoyStateMachine::Jumping(_) => 12,
            RedHatBoyStateMachine::Sliding(_) => 5,
            RedHatBoyStateMachine::Crashing(_) => 10,
            RedHatBoyStateMachine::GameOver(_) => 29,
        }
    }

    fn animation(&self) -> &str {
        match self {
            RedHatBoyStateMachine::Idle(_) => "Idle",
            RedHatBoyStateMachine::Running(_) => "Run",
            RedHatBoyStateMachine::Jumping(_) => "Jump",
            RedHatBoyStateMachine::Sliding(_) => "Slide",
            RedHatBoyStateMachine::Crashing(_) => "Dead",
            RedHatBoyStateMachine::GameOver(_) => "Dead",
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

    fn kill(self) -> Self {
        match self {
            RedHatBoyStateMachine::Running(val) => RedHatBoyStateMachine::Crashing(val.into()),
            _ => self,
        }
    }

    fn land(self, on: i16) -> Self {
        match self {
            RedHatBoyStateMachine::Jumping(mut val) => {
                val.object = val.object.set_on(on);
                RedHatBoyStateMachine::Running(val.into())
            }
            RedHatBoyStateMachine::Idle(mut val) => {
                val.object = val.object.set_on(on);
                RedHatBoyStateMachine::Idle(val)
            }
            RedHatBoyStateMachine::Running(mut val) => {
                val.object = val.object.set_on(on);
                RedHatBoyStateMachine::Running(val)
            }
            RedHatBoyStateMachine::Sliding(mut val) => {
                val.object = val.object.set_on(on);
                RedHatBoyStateMachine::Sliding(val)
            }
            RedHatBoyStateMachine::Crashing(mut val) => {
                val.object = val.object.set_on(on);
                RedHatBoyStateMachine::Crashing(val)
            }
            RedHatBoyStateMachine::GameOver(mut val) => {
                val.object = val.object.set_on(on);
                RedHatBoyStateMachine::GameOver(val)
            }
        }
    }

    fn update(self) -> Self {
        let frame_count = self.frame_count();

        match self {
            RedHatBoyStateMachine::Jumping(mut val) => {
                val.object = val.object.update(frame_count);

                RedHatBoyStateMachine::Jumping(val)
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
            RedHatBoyStateMachine::Crashing(mut val) => {
                val.object = val.object.update(frame_count);

                if val.object.animation_finished(frame_count) {
                    RedHatBoyStateMachine::GameOver(val.into())
                } else {
                    RedHatBoyStateMachine::Crashing(val)
                }
            }
            RedHatBoyStateMachine::GameOver(mut val) => {
                val.object.frame = frame_count;

                RedHatBoyStateMachine::GameOver(val)
            }
        }
    }
}

#[derive(Copy, Clone)]
struct RedHatBoyState<S> {
    _state: S,
    object: GameObject,
}

#[derive(Copy, Clone)]
struct Idle;
#[derive(Copy, Clone)]
struct Jumping;
#[derive(Copy, Clone)]
struct Running;
#[derive(Copy, Clone)]
struct Sliding;
#[derive(Copy, Clone)]
struct Crashing;
#[derive(Copy, Clone)]
struct GameOver;

impl RedHatBoyState<Idle> {
    fn new() -> Self {
        let game_object = GameObject {
            frame: 0,
            position: engine::Point { x: 0, y: 485 },
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
            object: machine.object.reset_frame().slide(),
        }
    }
}

impl From<RedHatBoyState<Running>> for RedHatBoyState<Jumping> {
    fn from(machine: RedHatBoyState<Running>) -> Self {
        RedHatBoyState {
            _state: Jumping {},
            object: machine.object.reset_frame().jump(),
        }
    }
}

impl From<RedHatBoyState<Running>> for RedHatBoyState<Crashing> {
    fn from(machine: RedHatBoyState<Running>) -> Self {
        RedHatBoyState {
            _state: Crashing {},
            object: machine.object.reset_frame().kill(),
        }
    }
}

impl From<RedHatBoyState<Jumping>> for RedHatBoyState<Running> {
    fn from(machine: RedHatBoyState<Jumping>) -> Self {
        RedHatBoyState {
            _state: Running {},
            object: machine.object.reset_frame().land(),
        }
    }
}

impl From<RedHatBoyState<Sliding>> for RedHatBoyState<Running> {
    fn from(machine: RedHatBoyState<Sliding>) -> Self {
        RedHatBoyState {
            _state: Running {},
            object: machine.object.reset_frame().stand_up(),
        }
    }
}

impl From<RedHatBoyState<Crashing>> for RedHatBoyState<GameOver> {
    fn from(machine: RedHatBoyState<Crashing>) -> Self {
        RedHatBoyState {
            _state: GameOver {},
            object: machine.object,
        }
    }
}

#[derive(Debug, Clone, Copy)]
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

    fn set_on(mut self, y: i16) -> GameObject {
        self.position.y = y;
        self
    }

    fn jump(mut self) -> Self {
        self.velocity.y = -25.0;
        self
    }

    fn update(mut self, frame_count: u8) -> Self {
        if self.velocity.y < 20.0 {
            self.velocity.y += GRAVITY;
        }

        self.position.x += self.velocity.x as i16;
        self.position.y += self.velocity.y as i16;
        if self.frame < (frame_count * 3) - 1 {
            self.frame += 1;
        } else {
            self.frame = 0;
        };
        self
    }

    fn land(mut self) -> Self {
        self.velocity.y = 0.0;
        self
    }

    fn slide(mut self) -> Self {
        self.position.y += 15;
        self
    }

    fn stand_up(mut self) -> Self {
        self.position.y -= 15;
        self
    }

    fn reset_frame(mut self) -> Self {
        self.frame = 0;
        self
    }

    fn kill(mut self) -> Self {
        self.velocity = Vector { x: 0.0, y: 0.0 };
        self
    }

    fn animation_finished(&self, frame_count: u8) -> bool {
        self.frame >= (frame_count * 3) - 1
    }
}
