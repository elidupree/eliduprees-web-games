use crate::game::UPDATE_DURATION;
use crate::mechanisms::{
  Mechanism, MechanismImmutableContext, MechanismTrait, MechanismUpdateContext,
};
use crate::ui_glue::Draw;
use eliduprees_web_games_lib::auto_constant;
use extend::ext;
use nalgebra::Vector2;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{Add, AddAssign, Sub};

pub type GridVector = Vector2<i32>;
pub type FloatingVector = Vector2<f64>;
pub const TILE_RADIUS: i32 = 1;
pub const TILE_WIDTH: i32 = TILE_RADIUS * 2;
pub const TILE_SIZE: GridVector = GridVector::new(TILE_WIDTH, TILE_WIDTH);

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

#[ext(pub, name = FloatingVectorExtension)]
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
  fn move_towards(&mut self, target: FloatingVector, change_size: f64) {
    let difference = target - *self;
    let difference_magnitude = difference.magnitude();
    if change_size >= difference_magnitude {
      *self = target;
    } else {
      *self += difference * (change_size / difference_magnitude);
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
  pub const CLOCKWISE: Rotation = Rotation(3);
  pub const COUNTERCLOCKWISE: Rotation = Rotation(1);
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
  pub const ALL_FACINGS: [Facing; 4] = [Facing(0), Facing(1), Facing(2), Facing(3)];
}

impl Add<Rotation> for Facing {
  type Output = Facing;
  fn add(self, other: Rotation) -> Facing {
    Facing((self.0 + other.0) % 4)
  }
}
impl AddAssign<Rotation> for Facing {
  fn add_assign(&mut self, other: Rotation) {
    *self = *self + other;
  }
}

impl Sub<Facing> for Facing {
  type Output = Rotation;
  fn sub(self, other: Facing) -> Rotation {
    Rotation((4 + self.0 - other.0) % 4)
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Map {
  pub tiles: HashMap<GridVector, Tile>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Tile {
  pub mechanism: Option<Mechanism>,
  pub materials: Vec<Material>,
  pub monsters: Vec<Monster>,
}
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
    let former = self.clone();

    let mechanism_updates: Vec<_> = self
      .tiles
      .iter()
      .filter_map(|(&tile_position, tile)| {
        tile
          .mechanism
          .as_ref()
          .map(|m| (tile_position, m.mechanism_type.clone()))
      })
      .collect();

    for (position, mechanism_type) in mechanism_updates {
      mechanism_type.update(MechanismUpdateContext {
        position,
        map: self,
        former: &former,
      });
    }

    let materials: Vec<_> = self
      .tiles
      .iter_mut()
      .flat_map(|(_, t)| t.materials.drain(..))
      .collect();
    for material in materials {
      self
        .tiles
        .entry(material.position.containing_tile())
        .or_insert_with(Default::default)
        .materials
        .push(material);
    }
  }
  pub fn draw(&mut self, draw: &mut impl Draw) {
    for (&tile_position, tile) in &self.tiles {
      if let Some(mechanism) = &tile.mechanism {
        mechanism.mechanism_type.draw(
          MechanismImmutableContext {
            position: tile_position,
            map: self,
          },
          draw,
        );
      }
    }

    for (&tile_position, tile) in &self.tiles {
      for material in &tile.materials {
        draw.rectangle_on_map(
          20,
          material.position,
          TILE_SIZE.to_floating() * 0.25,
          "#fff",
        );
      }
    }
  }
}
