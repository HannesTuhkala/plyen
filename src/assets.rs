use std::collections::HashMap;
use std::iter::FromIterator;

use ggez;
use ggez::graphics::{Image};

use crate::powerups::PowerUpKind;


pub struct Assets {
    pub cessna: Image,
    pub background: Image,
    pub powerups: HashMap<PowerUpKind, Image>,
    pub bullet: Image,
}

impl Assets {
    pub fn new(ctx: &mut ggez::Context) -> Assets {
        let powerups = HashMap::from_iter(vec!{
            ( PowerUpKind::Missile, Image::new(ctx, "/powerups/afterburner.png")
                .expect("Could not load generic powerup image")),
            ( PowerUpKind::Afterburner, Image::new(ctx, "/powerups/missile.png")
                .expect("could not load missile powerup asset")),
        });
        Assets {
            cessna: Image::new(ctx, "/cessna.png").
                expect("Could not find cessna image!"),
            background: Image::new(ctx, "/background.png").
                expect("Could not find background image!"),
            powerups,
            bullet: Image::new(ctx, "/bullet.png").
                expect("Could not find bullet image!"),
        }
    }
}
