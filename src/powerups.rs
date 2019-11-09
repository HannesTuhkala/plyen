use nalgebra as na;

use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub enum PowerUpKind {
    Missile,
    Afterburner,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PowerUp {
    pub kind: PowerUpKind,
    pub position: na::Point2<f32>,
}

impl PowerUp {
    pub fn new(kind: PowerUpKind, position: na::Point2<f32>) -> Self {
        Self {
            kind,
            position
        }
    }
}
