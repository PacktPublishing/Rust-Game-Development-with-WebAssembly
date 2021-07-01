use anyhow::{anyhow, Result};
use async_trait::async_trait;

use crate::{
    browser,
    engine::{self, Animation, Game, Image, KeyState, Point, Rect, Renderer, SpriteSheet, Vector},
};

const GRAVITY: f32 = 1.0;
const FLOOR: i16 = 600;
const IDLE_ANIMATION: &str = "Idle";
const RUNNING_ANIMATION: &str = "Run";
const JUMPING_ANIMATION: &str = "Jump";
const SLIDING_ANIMATION: &str = "Slide";
const DEAD_ANIMATION: &str = "Dead";
const RUNNING_SPEED: i16 = 4;

pub enum WalkTheDog {
    Loading,
    Loaded(WalkTheDogGame),
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog::Loading {}
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&mut self) -> Result<Box<dyn Game>> {
        match self {
            WalkTheDog::Loading => {
                let game = WalkTheDogGame::initialize().await?;
                Ok(Box::new(WalkTheDog::Loaded(game)))
            }
            WalkTheDog::Loaded(_) => Err(anyhow!("WalkTheDog already loaded!")),
        }
    }

    fn update(&mut self, keystate: &KeyState) {
        match self {
            WalkTheDog::Loaded(game) => game.update(keystate),
            _ => {}
        }
    }

    fn draw(&self, renderer: &Renderer) {
        match self {
            WalkTheDog::Loaded(game) => game.draw(renderer),
            _ => {}
        }
    }
}

struct Platform {
    sheet: SpriteSheet,
    bounding_box: Rect,
    position: Point,
    sprites: Vec<String>,
}

impl Platform {
    fn draw(&self, renderer: &Renderer) {
        for (pos, sprite) in self.sprites.iter().enumerate() {
            let position = Point {
                x: self.position.x + (pos as i16 * 128), // FIXME: Width shouldn't be hard coded (probably)
                y: self.position.y,
            };
            self.sheet.draw(renderer, sprite, &position)
        }
    }

    fn bounding_box(&self) -> Rect {
        Rect {
            x: self.position.x.into(),
            y: self.position.y.into(),
            width: self.bounding_box.width,
            height: self.bounding_box.height,
        }
    }
}

pub struct WalkTheDogGame {
    background: Image,
    rock: Image,
    rhb: RedHatBoy,
    platforms: Vec<Platform>,
    velocity: i16,
}

impl WalkTheDogGame {
    async fn initialize() -> Result<WalkTheDogGame> {
        let background = Image::new(engine::load_image("BG.png").await?, Point { x: 0, y: 0 });

        let rock = Image::new(
            engine::load_image("Stone.png").await?,
            Point { x: 200, y: 546 },
        );

        let json = browser::fetch_json("rhb.json").await?;
        let sheet = json.into_serde()?;
        let image = engine::load_image("rhb.png").await?;

        let rhb = RedHatBoy::new(Animation::new(
            SpriteSheet::new(image, sheet),
            vec![
                IDLE_ANIMATION,
                RUNNING_ANIMATION,
                JUMPING_ANIMATION,
                SLIDING_ANIMATION,
                DEAD_ANIMATION,
            ],
        ));

        let json = browser::fetch_json("tiles.json").await?;
        let sheet = json.into_serde()?;
        let image = engine::load_image("tiles.png").await?;
        let platform_sheet = SpriteSheet::new(image, sheet);

        let first_platform = Platform {
            sheet: platform_sheet,
            bounding_box: Rect {
                x: 0.0,
                y: 0.0,
                width: 384.0,
                height: 90.0,
            },
            sprites: vec![
                "13.png".to_string(),
                "14.png".to_string(),
                "15.png".to_string(),
            ],
            position: Point { x: 220, y: 350 },
        };

        Ok(WalkTheDogGame {
            background,
            rock,
            rhb,
            platforms: vec![first_platform],
            velocity: 0,
        })
    }

    fn update(&mut self, keystate: &KeyState) {
        if keystate.is_pressed("ArrowRight") {
            self.rhb.run();
            self.velocity = -RUNNING_SPEED;
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

        for (_, platform) in self.platforms.iter().enumerate() {
            self.rhb.check_platform_collisions(platform);
        }

        if self.rhb.collides_with(&self.rock.bounding_box()) {
            self.rhb.kill();
        }

        if self.rhb.landing() {
            self.rhb.land_on(FLOOR);
        }

        self.background.move_horizontally(self.velocity);
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 600.0,
        });

        self.background.draw(renderer);
        self.rock.draw(renderer);
        self.rhb.draw(renderer);

        self.draw_platform(renderer);
    }

    fn draw_platform(&self, renderer: &Renderer) {
        self.platforms.first().unwrap().draw(renderer);
    }
}

struct RedHatBoy {
    state: RedHatBoyStateMachine,
    animation: Animation,
}

impl RedHatBoy {
    fn new(animation: Animation) -> Self {
        RedHatBoy {
            state: RedHatBoyStateMachine::Idle(RedHatBoyState::new()),
            animation,
        }
    }

    fn draw(&self, renderer: &Renderer) {
        self.animation.draw(
            renderer,
            self.animation_name(),
            &(self.frame() / 3).into(),
            &self.position(),
        );
    }

    fn bounding_box(&self) -> Rect {
        let bounding_box = self
            .animation
            .bounding_box_for(self.animation_name(), &((self.frame() / 3) as i16));

        Rect {
            x: self.position().x as f32 + bounding_box.x,
            y: self.position().y as f32 + bounding_box.y,
            width: bounding_box.width,
            height: bounding_box.height,
        }
    }

    fn check_platform_collisions(&mut self, platform: &Platform) {
        if self.landing_on(&platform.bounding_box()) {
            self.land_on(platform.position.y);
        } else if self.collides_with(&platform.bounding_box()) {
            self.kill();
        }
    }

    fn collides_with(&self, rect: &Rect) -> bool {
        self.bounding_box().intersects(rect)
    }

    fn landing(&self) -> bool {
        self.position().y as f32 + self.bounding_box().height > FLOOR as f32
    }

    fn landing_on(&self, rect: &Rect) -> bool {
        self.bounding_box().intersects(rect) && (self.position().y as f32) < rect.y
    }

    fn land_on(&mut self, y: i16) {
        self.state = self
            .state
            .land((y as f32 - self.bounding_box().height) as i16)
    }

    fn animation_name(&self) -> &str {
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
            RedHatBoyStateMachine::Idle(_) => IDLE_ANIMATION,
            RedHatBoyStateMachine::Running(_) => RUNNING_ANIMATION,
            RedHatBoyStateMachine::Jumping(_) => JUMPING_ANIMATION,
            RedHatBoyStateMachine::Sliding(_) => SLIDING_ANIMATION,
            RedHatBoyStateMachine::Crashing(_) => DEAD_ANIMATION,
            RedHatBoyStateMachine::GameOver(_) => DEAD_ANIMATION,
        }
    }

    fn run(self) -> Self {
        match self {
            RedHatBoyStateMachine::Idle(val) => RedHatBoyStateMachine::Running(val.into()),
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
            RedHatBoyStateMachine::Jumping(val) => RedHatBoyStateMachine::Crashing(val.into()),
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
            object: machine.object,
        }
    }
}

impl RedHatBoyState<Running> {
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

impl From<RedHatBoyState<Jumping>> for RedHatBoyState<Crashing> {
    fn from(machine: RedHatBoyState<Jumping>) -> Self {
        RedHatBoyState {
            _state: Crashing {},
            object: machine.object.reset_frame().kill(),
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
