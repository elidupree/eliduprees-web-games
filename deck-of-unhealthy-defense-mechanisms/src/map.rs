use extend::ext;
use nalgebra::Vector2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type GridVector = Vector2<i32>;
pub type FloatingVector = Vector2<f64>;
pub const TILE_WIDTH: f64 = 2.0;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct Facing(u8);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct Rotation(u8);

#[ext(pub, name = GridVectorExtension)]
impl GridVector {
  fn to_floating(&self) -> FloatingVector {
    Vector2::new(self[0] as f64, self[1] as f64)
  }
  fn exact_facing(&self) -> Option<Facing> {
    match (self[0].signum(), self[1].signum()) {
      (1, 0) => Some(Facing(0)),
      (0, 1) => Some(Facing(1)),
      (-1, 0) => Some(Facing(2)),
      (0, -1) => Some(Facing(3)),
      _ => None,
    }
  }
}

#[ext(pub, name = FloatVectorExtension)]
impl FloatingVector {
  fn containing_tile(&self) -> GridVector {
    Vector2::new(
      (self[0] * 0.5).round() as i32 * 2,
      (self[1] * 0.5).round() as i32 * 2,
    )
  }
  fn closest_facing(&self) -> Option<Facing> {
    if self[0] > self[1].abs() {
      Some(Facing(0))
    } else if self[1] > self[0].abs() {
      Some(Facing(1))
    } else if -self[0] > self[1].abs() {
      Some(Facing(2))
    } else if -self[1] > self[0].abs() {
      Some(Facing(3))
    } else {
      None
    }
  }
  fn apply_friction(&mut self, magnitude_reduction: f64) {
    let magnitude = self.magnitude();
    if magnitude_reduction >= magnitude {
      *self = FloatingVector::zeros();
    } else {
      *self *= (magnitude - magnitude_reduction) / magnitude;
    }
  }
  fn limit_magnitude(&mut self, magnitude_limit: f64) {
    let magnitude = self.magnitude();
    if magnitude > magnitude_limit {
      *self *= magnitude_limit / magnitude;
    }
  }
}

impl Rotation {
  pub fn quarter_turns_from_posx_towards_posy(self) -> u8 {
    self.0
  }
}
impl Facing {
  pub fn unit_vector(self) -> GridVector {
    match self.0 {
      0 => GridVector::new(1, 0),
      1 => GridVector::new(0, 1),
      2 => GridVector::new(-1, 0),
      3 => GridVector::new(0, -1),
      _ => unreachable!(),
    }
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Map {
  pub tiles: HashMap<GridVector, Tile>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Tile {
  pub mechanism: Option<Mechanism>,
  pub materials: Vec<Material>,
  pub monsters: Vec<Monster>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Mechanism {}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Material {
  pub position: FloatingVector,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Monster {
  pub position: FloatingVector,
  pub hitpoints: i32,
}
impl Map {
  pub fn update(&mut self) {
    for (tile_position, tile) in &mut self.tiles {
      for monster in &mut tile.monsters {}
    }
  }
}
