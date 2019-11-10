mod player;
mod assets;
mod map;
mod bullet;
mod gamestate;
mod constants;
mod messages;
mod powerups;
mod math;

use std::io::prelude::*;
use std::net::TcpStream;
use std::env;
use std::path;
use std::time::Instant;

use ggez;
use ggez::event::{self, EventHandler};
use ggez::event::winit_event::{Event, KeyboardInput, WindowEvent, ElementState};
use ggez::graphics;
use ggez::nalgebra as na;
use ggez::input::keyboard;
use ears::AudioController;

use assets::Assets;
use messages::{MessageReader, ClientMessage, ServerMessage, SoundEffect};

use whoami;

const PLANES: [player::PlaneType; 4] = [
    player::PlaneType::SukaBlyat,
    player::PlaneType::HowdyCowboy,
    player::PlaneType::ElPolloRomero,
    player::PlaneType::AchtungBlitzKrieg,
];

const COLORS: [player::Color; 5] = [
    player::Color::Red,
    player::Color::Green,
    player::Color::Blue,
    player::Color::Yellow,
    player::Color::Purple,
];

struct KeyStates {
    forward: ElementState,
    back: ElementState,
    left: ElementState,
    right: ElementState,
    shooting: ElementState,
}

impl KeyStates {
    fn new() -> Self {
        KeyStates {
            forward: ElementState::Released,
            back: ElementState::Released,
            left: ElementState::Released,
            right: ElementState::Released,
            shooting: ElementState::Released,
        }
    }
}

fn send_client_message(msg: &ClientMessage, stream: &mut TcpStream) {
    let data = serde_json::to_string(msg)
        .expect("Failed to encode message");
    stream.write(data.as_bytes())
        .expect("Failed to send message to server");
    stream.write(&[0])
        .expect("Failed to send message to server");
}

struct MainState {
    my_id: u64,
    camera_position: na::Point2<f32>,
    server_reader: MessageReader<ServerMessage>,
    game_state: gamestate::GameState,
    map: map::Map,
    assets: Assets,
    key_states: KeyStates,
    last_time: Instant,
    powerup_rotation: f32,
}

struct MenuState<'a> {
    plane: player::PlaneType,
    name: String,
    color: player::Color,
    assets: &'a Assets,
    color_selection: usize,
    plane_selection: usize
}

struct EndState<'a> {
    assets: &'a Assets
}

impl MainState {
    fn new(my_id: u64, stream: MessageReader<ServerMessage>, assets: Assets)
        -> ggez::GameResult<MainState>
    {
        let s = MainState {
            server_reader: stream,
            my_id,
            camera_position: na::Point2::new(0., 0.),
            game_state: gamestate::GameState::new(),
            map: map::Map::new(),
            assets: assets,
            key_states: KeyStates::new(),
            last_time: Instant::now(),
            powerup_rotation: 0.,
        };
        Ok(s)
    }
}

impl<'a> MenuState<'a> {
    fn new(assets: &Assets) -> MenuState {
        MenuState {
            name: String::from(whoami::username()),
            plane: player::PlaneType::SukaBlyat,
            color: player::Color::Red,
            assets: assets,
            color_selection: 0,
            plane_selection: 0
        }
    }
}

impl<'a> EndState<'a> {
    fn new(assets: &Assets) -> EndState {
        EndState {
            assets: assets,
        }
    }
}

impl<'a> event::EventHandler for EndState<'a> {
    fn update(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        if keyboard::is_key_pressed(ctx, keyboard::KeyCode::Return) {
            ctx.continuing = false;
        }
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        graphics::clear(ctx, [0.1, 0.1, 0.1, 1.0].into());
        graphics::draw(
            ctx, &self.assets.end_background,
            (na::Point2::new(
                    -constants::WINDOW_SIZE/2.,
                    -constants::WINDOW_SIZE/2.,
                    ),)).unwrap();
        graphics::present(ctx)?;
        Ok(())
    }
}

impl<'a> event::EventHandler for MenuState<'a> {
    fn update(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        self.plane = PLANES[self.plane_selection].clone();
        self.color = COLORS[self.color_selection].clone();
        if keyboard::is_key_pressed(ctx, keyboard::KeyCode::Return)
            || keyboard::is_key_pressed(ctx, keyboard::KeyCode::Space) {
            ctx.continuing = false;
        }
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        graphics::clear(ctx, [0.1, 0.1, 0.1, 1.0].into());
        graphics::draw(ctx, &self.assets.menu_background,
                       (na::Point2::new(0., 0.),)).unwrap();
        self.draw_selected_plane(ctx, self.assets);
        self.draw_selected_color(ctx, self.assets);
        self.draw_player_name(ctx, self.assets);
        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut ggez::Context,
                               _button: ggez::input::mouse::MouseButton,
                               x: f32, y: f32) {
        let (px, py) = constants::PLANE_SELECTION_POS;
        let s = constants::PLANE_SELECTION_SIZE;
        if x > px && x < px + s * 1.25 && y > py && y < py + s {
            self.plane_selection = (self.plane_selection + 1) % 4;
        }

        let (cx, cy) = constants::COLOR_SELECTION_POS;
        let s = constants::COLOR_SELECTION_SIZE;
        if x > cx && x < cx + s && y > cy && y < cy + s {
            self.color_selection = (self.color_selection + 1) % 5;
        }
    }
}

impl<'a> MenuState<'a> {
    fn draw_player_name(&mut self, ctx: &mut ggez::Context, assets: &Assets) {
        let (nx, ny) = constants::NAME_POS;
        let mut text = graphics::Text::new(format!(
            "Helo comrade {}", self.name.clone())
        );
        text.set_font(assets.font, graphics::Scale::uniform(15.));
        graphics::draw(ctx, &text,
                       (na::Point2::new(nx + 10., ny + 10.),)).unwrap();
    }

    fn draw_selected_plane(&mut self, ctx: &mut ggez::Context,
                           assets: &Assets) {
        let sprite = assets.planes[&self.plane].clone();
        let text = self.plane.name();
        let (px, py) = constants::PLANE_SELECTION_POS;
        let mut ggez_text = graphics::Text::new(text);
        ggez_text.set_font(assets.font, graphics::Scale::uniform(15.));
        let background_rect = &graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            graphics::Rect::new(
                px, py,
                constants::PLANE_SELECTION_SIZE*1.25,
                constants::PLANE_SELECTION_SIZE
                ),
            [0., 0., 0., 0.5].into()
        ).unwrap();
        graphics::draw(
            ctx, background_rect,
            (na::Point2::new(0., 0.),)
        ).unwrap();
        graphics::draw(ctx, &ggez_text,
                       (na::Point2::new(px + 10., py + 10.),)).unwrap();
        let mut instruction = graphics::Text::new("click to change plane blyat:");
        instruction.set_font(assets.font, graphics::Scale::uniform(15.));
        graphics::draw(ctx, &instruction,
                       (na::Point2::new(px, py - 20.),)).unwrap();
        graphics::draw(ctx, &sprite,
                       (na::Point2::new(
                               px
                               + constants::PLANE_SELECTION_SIZE/3.
                               - (constants::PLANE_SIZE as f32)*2.,
                               py
                               + constants::PLANE_SELECTION_SIZE/2.
                               - constants::PLANE_SIZE as f32,
                       ),)).unwrap();

        let mut plane_specs = graphics::Text::new(format!(
            "Agility: {}\nFirepower: {}\nAcceleration: {}\nHealth: {}\nResilience: {}",
            self.plane.agility(),
            self.plane.firepower(),
            self.plane.acceleration().trunc(),
            self.plane.health(),
            self.plane.resilience()));
        plane_specs.set_font(assets.font, graphics::Scale::uniform(15.));
        graphics::draw(ctx, &plane_specs,
                       (na::Point2::new(
                               px + constants::PLANE_SELECTION_SIZE/2.4,
                               py + constants::PLANE_SELECTION_SIZE/3.),))
            .unwrap();
    }

    fn draw_selected_color(
        &mut self, ctx: &mut ggez::Context, assets: &Assets
        ) {
        let (cx, cy) = constants::COLOR_SELECTION_POS;
        let background_rect = &graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            graphics::Rect::new(
                cx, cy,
                constants::COLOR_SELECTION_SIZE,
                constants::COLOR_SELECTION_SIZE
                ),
            self.color.rgba().into()
        ).unwrap();
        graphics::draw(
            ctx, background_rect, (na::Point2::new(0., 0.),)).unwrap();
        let mut instruction = graphics::Text::new("click to change color:");
        instruction.set_font(assets.font, graphics::Scale::uniform(15.));
        graphics::draw(ctx, &instruction,
                       (na::Point2::new(cx, cy - 20.),)).unwrap();
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        let elapsed = self.last_time.elapsed();
        self.last_time = Instant::now();

        self.server_reader.fetch_bytes().unwrap();
        // TODO: Use a real loop
        while let Some(message) = self.server_reader.next() {
            match message {
                ServerMessage::AssignId(_) => {panic!("Got new ID after intialisation")}
                ServerMessage::GameState(state) => {
                    self.game_state = state
                },
                ServerMessage::PlaySound(sound, pos) => {
                    match sound {
                        SoundEffect::Powerup => {
                            self.assets.powerup.play_at(pos);
                        }
                        SoundEffect::Gun => {
                            self.assets.gun.play_at(pos);
                        }
                        SoundEffect::Explosion => {
                            self.assets.explosion.play_at(pos);
                        }
                    }
                }
                ServerMessage::YouDied => {
                    ctx.continuing = false
                }
            }
        }

        ears::listener::set_position([self.camera_position.x, 0., self.camera_position.y]);

        let mut y_input = 0.0;
        if self.key_states.forward == ElementState::Pressed {
            y_input += 1.0;
        }
        if self.key_states.back == ElementState::Pressed {
            y_input -= 1.0;
        }

        let mut x_input = 0.0;
        if self.key_states.left == ElementState::Pressed {
            x_input -= 1.0;
        } 
        if self.key_states.right == ElementState::Pressed {
            x_input += 1.0;
        }

        self.map.update_particles(elapsed.as_secs_f32(), &self.game_state);

        let shooting = self.key_states.shooting == ElementState::Pressed;
        let input_message = ClientMessage::Input{ x_input, y_input, shooting };
        send_client_message(&input_message, &mut self.server_reader.stream);

        self.powerup_rotation += constants::POWERUP_SPEED;
        Ok(())
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        graphics::clear(ctx, [0.1, 0.1, 0.1, 1.0].into());

        if let Some(my_player) = self.game_state.get_player_by_id(self.my_id) {
            self.camera_position = my_player.position;
        }

        self.map.draw(
            self.my_id,
            ctx,
            self.camera_position,
            &self.game_state,
            &self.assets,
            self.powerup_rotation,
        );
        graphics::present(ctx)?;
        Ok(())
    }
}

pub fn main() -> ggez::GameResult {
    let mut should_continue = true;
    while should_continue {
        let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            let mut path = path::PathBuf::from(manifest_dir);
            path.push("resources");
            path
        } else {
            path::PathBuf::from("./resources")
        };

        let host = std::env::var("SERVER")
            .unwrap_or(String::from("localhost:4444"));
        let stream = TcpStream::connect(host)?;
        println!("Connected to server");

        stream.set_nonblocking(true)?;
        let mut reader = MessageReader::new(stream);

        let msg = loop {
            reader.fetch_bytes().unwrap();
            if let Some(msg) = reader.next() {
                break msg;
            }
        };

        let my_id = if let ServerMessage::AssignId(id) = msg {
            println!("Received the id {}", id);
            id
        } else {
            panic!("Expected to get an id from server")
        };

        let (ctx, event_loop) = &mut ggez::ContextBuilder::new("super_simple", "ggez")
            .window_setup(ggez::conf::WindowSetup::default()
                          .title("Flying broccoli"))
            .window_mode(ggez::conf::WindowMode::default()
                         .dimensions(constants::WINDOW_SIZE,
                                     constants::WINDOW_SIZE))
            .add_resource_path(resource_dir)
            .build()?;

        let assets = Assets::new(ctx);
        let state = &mut MenuState::new(&assets);
        event::run(ctx, event_loop, state)?;
        ctx.continuing = true;
        send_client_message(
            &ClientMessage::JoinGame { 
                name: state.name.clone(),
                plane: state.plane.clone(),
                color: state.color.clone()
            },
            &mut reader.stream
        );

        let mut coords = graphics::screen_coordinates(ctx);
        coords.translate(
            na::Vector2::new(
                -coords.w / 2.0, -coords.h / 2.0
            )
        );
        graphics::set_screen_coordinates(
            ctx, coords
        ).expect("Could not set screen coordinates");

        let state = &mut MainState::new(my_id, reader, assets)?;
        while ctx.continuing {
            // Tell the timer stuff a frame has happened.
            // Without this the FPS timer functions and such won't work.
            ctx.timer_context.tick();

            event_loop.poll_events(|event| {
                // This tells `ggez` to update it's internal states, should the event require that.
                // These include cursor position, view updating on resize, etc.
                ctx.process_event(&event);

                match event {
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::CloseRequested => {
                            should_continue = false;
                            event::quit(ctx);
                        },
                        WindowEvent::KeyboardInput {
                            input: KeyboardInput {
                                scancode,
                                state: key_state,
                                virtual_keycode: keycode,
                                ..
                            },
                            ..
                        } => {
                            match scancode {
                                constants::SCANCODE_W => { state.key_states.forward = key_state },
                                constants::SCANCODE_S => { state.key_states.back = key_state },
                                constants::SCANCODE_A => { state.key_states.left = key_state },
                                constants::SCANCODE_D => { state.key_states.right = key_state },
                                _ => {} // Handle other key events here
                            }

                            if keycode == Some(keyboard::KeyCode::Space) {
                                state.key_states.shooting = key_state;
                            }
                        }

                        // Add other window event handling here
                        _ => {}
                    },

                    // Add other event handling here
                    _ => {}
                }
            });

            state.update(ctx)?;
            state.draw(ctx)?;
        }
        ctx.continuing = true;
        let state = &mut EndState::new(&state.assets);
        event::run(ctx, event_loop, state);
    }
    Ok(())
}
