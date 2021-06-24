use crate::game::{Game, UPDATE_DURATION};
use crate::mechanisms::{Mechanism, MechanismImmutableContext, MechanismUpdateContext};
use crate::movers::{Mover, MoverId, MoverImmutableContext, MoverUpdateContext};
use crate::ui_glue::Draw;
use extend::ext;
use guard::guard;
use live_prop_test::{live_prop_test, lpt_assert_eq};
use nalgebra::Vector2;
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

pub fn nearest_grid_center(coordinate: f64) -> i32 {
  (coordinate * 0.5).round() as i32 * 2
}
pub fn moved_towards(coordinate: f64, target: f64, distance: f64) -> f64 {
  if coordinate > target + distance {
    coordinate - distance
  } else if coordinate < target - distance {
    coordinate + distance
  } else {
    target
  }
}

#[ext(pub, name = FloatingVectorExtension)]
impl FloatingVector {
  fn containing_tile(&self) -> GridVector {
    GridVector::new(nearest_grid_center(self[0]), nearest_grid_center(self[1]))
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
  pub const CLOCKWISE: Rotation = Rotation(1);
  pub const COUNTERCLOCKWISE: Rotation = Rotation(3);
  pub const IDENTITY: Rotation = Rotation(0);
  pub const U_TURN: Rotation = Rotation(2);
  pub fn quarter_turns_from_posx_towards_posy(self) -> u8 {
    self.0
  }
}
impl Facing {
  pub fn as_index(self) -> usize {
    self.0 as usize
  }
  pub fn from_index(index: usize) -> Self {
    Facing(index as u8)
  }
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
  pub tiles: Tiles,
  pub movers: Movers,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct TilesBounds {
  min: GridVector,
  size: Vector2<usize>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Tiles {
  bounds: TilesBounds,
  tiles: Vec<Tile>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Movers {
  movers: HashMap<MoverId, Mover>,
  next_id: usize,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Tile {
  pub mechanism: Option<Mechanism>,
  pub movers: Vec<MoverId>,
}

#[live_prop_test]
impl TilesBounds {
  //#[live_prop_test(postcondition = "result.map_or(true, |index| self.position(index) == position)")]
  fn index(&self, position: GridVector) -> Option<usize> {
    let coords = (position - self.min) / TILE_WIDTH;
    assert_eq!(
      self.min + coords * TILE_WIDTH,
      position,
      "something gave wrong-parity coordinates to tile query"
    );
    if (0..2).all(|dim| (0..self.size[dim]).contains(&(coords[dim] as usize))) {
      Some(coords[0] as usize + coords[1] as usize * self.size[0])
    } else {
      None
    }
  }

  //#[live_prop_test(postcondition = "self.index(result) == Some(index)")]
  fn position(&self, index: usize) -> GridVector {
    let y = index / self.size[0];
    let x = index % self.size[0];
    self.min + GridVector::new(x as i32 * TILE_WIDTH, y as i32 * TILE_WIDTH)
  }

  //#[live_prop_test(postcondition = "self.index(result.containing_tile()).is_some()")]
  pub fn clamp(&self, position: FloatingVector) -> FloatingVector {
    let epsilon = 0.0000001;
    let mut result = position;
    for dim in 0..2 {
      result[dim] = position[dim].clamp(
        ((self.min[dim] - TILE_RADIUS) as f64) + epsilon,
        ((self.min[dim] + (self.size[dim] as i32 * TILE_WIDTH) - TILE_RADIUS) as f64) - epsilon,
      );
    }
    result
  }
}
pub type TilesIter<'a> = impl Iterator<Item = (GridVector, &'a Tile)> + 'a;
pub type TilesIterMut<'a> = impl Iterator<Item = (GridVector, &'a mut Tile)> + 'a;
#[live_prop_test]
impl Tiles {
  pub fn new(min: GridVector, size: Vector2<usize>) -> Self {
    Tiles {
      bounds: TilesBounds { min, size },
      tiles: (0..size[0] * size[1]).map(|_| Default::default()).collect(),
    }
  }
  pub fn get(&self, position: GridVector) -> Option<&Tile> {
    self
      .bounds
      .index(position)
      .map(move |index| self.tiles.get(index).unwrap())
  }
  pub fn get_mut(&mut self, position: GridVector) -> Option<&mut Tile> {
    self
      .bounds
      .index(position)
      .map(move |index| self.tiles.get_mut(index).unwrap())
  }
  pub fn iter(&self) -> TilesIter {
    let bounds = &self.bounds;
    self
      .tiles
      .iter()
      .enumerate()
      .map(move |(index, tile)| (bounds.position(index), tile))
  }
  pub fn iter_mut(&mut self) -> TilesIterMut {
    let bounds = &self.bounds;
    self
      .tiles
      .iter_mut()
      .enumerate()
      .map(move |(index, tile)| (bounds.position(index), tile))
  }
  pub fn bounds(&self) -> &TilesBounds {
    &self.bounds
  }
}

impl<'a> IntoIterator for &'a Tiles {
  type Item = (GridVector, &'a Tile);
  type IntoIter = TilesIter<'a>;

  fn into_iter(self) -> TilesIter<'a> {
    self.iter()
  }
}

impl<'a> IntoIterator for &'a mut Tiles {
  type Item = (GridVector, &'a mut Tile);
  type IntoIter = TilesIterMut<'a>;

  fn into_iter(self) -> TilesIterMut<'a> {
    self.iter_mut()
  }
}

pub type MoversIter<'a> = impl Iterator<Item = (MoverId, &'a Mover)> + 'a;
impl Movers {
  pub fn new() -> Self {
    Default::default()
  }
  pub fn get(&self, id: MoverId) -> Option<&Mover> {
    self.movers.get(&id)
  }
  pub fn iter(&self) -> MoversIter {
    self.movers.iter().map(|(&id, mover)| (id, mover))
  }
  pub fn ids(&self) -> impl Iterator<Item = MoverId> + '_ {
    self.movers.keys().copied()
  }
}

impl<'a> IntoIterator for &'a Movers {
  type Item = (MoverId, &'a Mover);
  type IntoIter = MoversIter<'a>;

  fn into_iter(self) -> MoversIter<'a> {
    self.iter()
  }
}

#[live_prop_test]
impl Map {
  pub fn check_invariants(&self) -> Result<(), String> {
    for (id, mover) in &self.movers {
      guard!(let Some(tile) = self.tiles.get(mover.position.containing_tile()) else {return Err("mover is outside of grid".to_string())});
      if !tile.movers.contains(&id) {
        return Err(format!(
          "Mover does not have a record in its tile: {:?}",
          mover
        ));
      }
    }
    for (position, tile) in &self.tiles {
      for &id in &tile.movers {
        guard!(let Some(mover) = self.movers.get(id) else {return Err("tile retained reference to missing mover".to_string())});
        lpt_assert_eq!(mover.position.containing_tile(), position);
      }
    }
    Ok(())
  }

  pub fn create_mover(&mut self, mover: Mover) -> MoverId {
    let id = MoverId(self.movers.next_id);
    self
      .tiles
      .get_mut(mover.position.containing_tile())
      .unwrap()
      .movers
      .push(id);
    self.movers.movers.insert(id, mover);
    self.movers.next_id += 1;
    id
  }
  pub fn remove_mover(&mut self, id: MoverId) -> Option<Mover> {
    let result = self.movers.movers.remove(&id);
    if let Some(mover) = &result {
      self
        .tiles
        .get_mut(mover.position.containing_tile())
        .unwrap()
        .movers
        .retain(|&m| m != id);
    }
    result
  }
  pub fn mutate_mover<R, F: FnOnce(&mut Mover) -> R>(&mut self, id: MoverId, f: F) -> Option<R> {
    if let Some(mover) = self.movers.movers.get_mut(&id) {
      let old_tile_position = mover.position.containing_tile();
      let result = f(mover);
      let new_tile_position = mover.position.containing_tile();
      if new_tile_position != old_tile_position {
        self
          .tiles
          .get_mut(old_tile_position)
          .unwrap()
          .movers
          .retain(|&m| m != id);
        self
          .tiles
          .get_mut(new_tile_position)
          .unwrap()
          .movers
          .push(id);
      }
      Some(result)
    } else {
      None
    }
  }

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn update(&mut self, former_game: &Game) {
    let mechanism_updates: Vec<_> = self
      .tiles
      .iter()
      .filter_map(|(tile_position, tile)| {
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
        former_game,
      });
    }

    for mover_id in self.movers.ids().collect::<Vec<_>>() {
      if let Some(mover) = self.movers.get(mover_id) {
        let behavior = mover.behavior.clone();
        let context = MoverUpdateContext {
          id: mover_id,
          map: self,
          former_game,
        };
        behavior.update(context);
      }
    }

    for mover_id in self.movers.ids().collect::<Vec<_>>() {
      let bounds = self.tiles.bounds().clone();
      self.mutate_mover(mover_id, |mover| {
        mover.position += mover.velocity * UPDATE_DURATION;
        mover.position = bounds.clamp(mover.position);
      });
    }
  }
  pub fn draw(&self, game: &Game, draw: &mut impl Draw) {
    for (tile_position, tile) in &self.tiles {
      if let Some(mechanism) = &tile.mechanism {
        mechanism.mechanism_type.draw(
          MechanismImmutableContext {
            position: tile_position,
            game,
          },
          draw,
        );
      }
      for &mover_id in &tile.movers {
        self
          .movers
          .get(mover_id)
          .unwrap()
          .behavior
          .draw(MoverImmutableContext { id: mover_id, game }, draw);
      }
    }
  }

  pub fn movers_near(
    &self,
    position: FloatingVector,
    range: f64,
  ) -> impl Iterator<Item = (MoverId, &Mover)> {
    let range_squared = range * range;
    let x_range = (nearest_grid_center(position[0] - range)
      ..=nearest_grid_center(position[0] + range))
      .step_by(TILE_WIDTH as usize);
    let y_range = (nearest_grid_center(position[1] - range)
      ..=nearest_grid_center(position[1] + range))
      .step_by(TILE_WIDTH as usize);
    x_range.flat_map(move |x| {
      y_range
        .clone()
        .filter_map(move |y| {
          let closest = FloatingVector::new(
            moved_towards(x as f64, position[0], TILE_RADIUS as f64),
            moved_towards(y as f64, position[1], TILE_RADIUS as f64),
          );
          if (closest - position).magnitude_squared() > range_squared {
            return None;
          }
          let tile_position = GridVector::new(x, y);
          self.tiles.get(tile_position)
        })
        .flat_map(move |tile| {
          tile.movers.iter().filter_map(move |&mover_id| {
            let mover = self.movers.get(mover_id).unwrap();
            if (mover.position - position).magnitude_squared() <= range_squared {
              Some((mover_id, mover))
            } else {
              None
            }
          })
        })
    })
  }
}
