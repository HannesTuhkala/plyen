use std::io;
use std::vec;
use std::io::prelude::*;
use std::net::TcpStream;
use std::net::TcpListener;
use std::collections::HashMap;
use serde_json;
use nalgebra::Point2;

mod messages;
mod player;
mod bullet;
mod gamestate;
mod constants;

use messages::{ServerMessage};
use player::Player;


fn send_server_message(msg: &ServerMessage, stream: &mut TcpStream)
    -> io::Result<()>
{
    let data = serde_json::to_string(msg)
        .expect("Failed to encode message");
    stream.write_all(data.as_bytes())?;
    stream.write_all(&[0])
}

fn main() {
    let mut connections = vec!();

    let listener = TcpListener::bind("127.0.0.1:30000")
        .unwrap();

    let mut state = gamestate::GameState::new();
    state.add_player(player::Player::new(1337, Point2::new(10., 10.)));

    listener.set_nonblocking(true).unwrap();

    println!("Listening on 127.0.0.1:30000");

    let mut players = vec::Vec::<Player>::new();
    let mut next_id: u64 = 0;
    loop {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    println!("Got new connection {}", next_id);
                    send_server_message(&ServerMessage::AssignId(next_id), &mut stream);
                    connections.push((next_id, stream));
                    let mut player = Player::new(next_id, Point2::new(10., 10.));
                    players.push(player);
                    next_id += 1;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // wait until network socket is ready, typically implemented
                    // via platform-specific APIs such as epoll or IOCP
                    break;
                }
                e => {e.expect("Socket listener error");}
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut clients_to_delete = vec!();
        for (id, ref mut client) in connections.iter_mut() {
            let result = send_server_message(
                &ServerMessage::GameState(state.clone()),
                client
            );

            if let Err(e) = result {
                match e.kind() {
                    io::ErrorKind::ConnectionReset => {
                        println!("Player {} disconnected", id);
                        clients_to_delete.push(*id);
                    }
                    e => {
                        panic!("Unhandled network issue: {:?}", e)
                    }
                }
            }
        }
        connections = connections.into_iter()
            .filter(|(id, _)| !clients_to_delete.contains(id))
            .collect();
    }
}

