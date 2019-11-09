mod messages;
mod assets;
mod player;
mod bullet;
mod gamestate;
mod constants;
mod math;

use std::io;
use std::vec;
use std::io::prelude::*;
use std::net::TcpStream;
use std::net::TcpListener;
use serde_json;
use nalgebra::Point2;
use nalgebra as na;
use std::time::Instant;

use messages::{ClientMessage, ServerMessage, MessageReader};
use player::Player;



fn send_server_message(msg: &ServerMessage, stream: &mut TcpStream)
    -> io::Result<()>
{
    let data = serde_json::to_string(msg)
        .expect("Failed to encode message");
    stream.write_all(data.as_bytes())?;
    stream.write_all(&[0])
}

fn update_player_position(player: &mut Player, x_input: f32, y_input: f32, delta: f32) {
    let mut dx = 0.;
    let mut dy = 0.;

    player.speed += y_input * constants::DEFAULT_ACCELERATION * delta;
    if player.speed > constants::MAX_SPEED {
        player.speed = constants::MAX_SPEED;
    }
    if player.speed < constants::MIN_SPEED {
        player.speed = constants::MIN_SPEED;
    }

    let rotation = x_input * constants::DEFAULT_AGILITY;

    dx += player.speed * (player.rotation - std::f32::consts::PI/2.).cos();
    dy += player.speed * (player.rotation - std::f32::consts::PI/2.).sin();
    player.velocity = na::Vector2::new(dx, dy) * delta;

    player.position = math::wrap_around(
        player.position + player.velocity
    );

    player.rotation = player.rotation + rotation;
}

struct Server {
    listener: TcpListener,
    connections: Vec<(u64, MessageReader<ClientMessage>)>,
    state: gamestate::GameState,
    next_id: u64,
    last_time: Instant,
}

impl Server {
    pub fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:30000")
            .unwrap();

        listener.set_nonblocking(true).unwrap();

        println!("Listening on 127.0.0.1:30000");

        Self {
            listener,
            connections: vec!(),
            next_id: 0,
            last_time: Instant::now(),
            state: gamestate::GameState::new()
        }
    }

    pub fn update(mut self) -> Self {
        let elapsed = self.last_time.elapsed();
        let delta_time = 1./100.;
        std::thread::sleep(std::time::Duration::from_millis(10) - elapsed);
        self.last_time = Instant::now();

        self.accept_new_connections();
        self.update_clients(delta_time)
    }

    fn accept_new_connections(&mut self) {
        // Read data from clients
        for stream in self.listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    stream.set_nonblocking(true).unwrap();
                    println!("Got new connection {}", self.next_id);
                    if let Err(_) = send_server_message(
                        &ServerMessage::AssignId(self.next_id),
                        &mut stream
                    ) {
                        println!("Could not send assign id message");
                        continue;
                    }
                    self.connections.push((
                        self.next_id,
                        MessageReader::<ClientMessage>::new(stream)
                    ));
                    let player = Player::new(self.next_id, Point2::new(10., 10.));
                    self.state.add_player(player);
                    self.next_id += 1;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // wait until network socket is ready, typically implemented
                    // via platform-specific APIs such as epoll or IOCP
                    break;
                }
                e => {e.expect("Socket listener error");}
            }
        }
    }

    fn update_clients(mut self, delta_time: f32) -> Self {
        for bullet in &mut self.state.bullets {
            bullet.update();
        }

        // Send data to clients
        let mut clients_to_delete = vec!();
        for (id, ref mut client) in self.connections.iter_mut() {
            macro_rules! remove_player_on_disconnect {
                ($op:expr) => {
                    match $op {
                        Ok(_) => {},
                        Err(e) => {
                            match e.kind() {
                                io::ErrorKind::ConnectionReset => {
                                    println!("Player {} disconnected", id);
                                    clients_to_delete.push(*id);
                                    break;
                                }
                                e => {
                                    panic!("Unhandled network issue: {:?}", e)
                                }
                            }
                        }
                    };
                }
            }
            remove_player_on_disconnect!(client.fetch_bytes());

            let mut player_input_x = 0.0;
            let mut player_input_y = 0.0;
            let mut shoot = false;

            // TODO: Use a real loop
            while let Some(message) = client.next() {
                match message {
                    ClientMessage::Ping => {},
                    ClientMessage::Shoot => { shoot = true },
                    ClientMessage::Input(input_x, input_y) => {
                        player_input_x = input_x;
                        player_input_y = input_y;
                    }
                }
            }

            let mut bullet = None;
            for mut player in &mut self.state.players {
                if player.id == *id {
                    update_player_position(
                        &mut player,
                        player_input_x,
                        player_input_y,
                        delta_time,
                    );

                    if shoot {
                        bullet = Some(player.shoot());
                    }
                }
            }

            if let Some(bullet) = bullet {
                self.state.add_bullet(bullet);
            }

            let result = send_server_message(
                &ServerMessage::GameState(self.state.clone()),
                &mut client.stream
            );
            remove_player_on_disconnect!(result);
        }
        self.state.players = self.state.players.into_iter()
            .filter(|player| !clients_to_delete.contains(&player.id))
            .collect();
        self.connections = self.connections.into_iter()
            .filter(|(id, _)| !clients_to_delete.contains(id))
            .collect();
        self
    }
}

fn main() {
    let mut server = Server::new();
    loop {
        server = server.update();
    }
}

