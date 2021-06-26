use extend::ext;
use live_prop_test::{live_prop_test, lpt_assert_eq};
use nalgebra::Vector2;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Sub};

pub type GridVector = Vector2<i32>;
pub type FloatingVector = Vector2<f64>;
pub const TILE_RADIUS: i32 = 1;
pub const TILE_WIDTH: i32 = TILE_RADIUS * 2;
pub const TILE_SIZE: GridVector = GridVector::new(TILE_WIDTH, TILE_WIDTH);
pub const EPSILON: f64 = 0.000001;

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

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct GridBounds {
  min_tile_center: GridVector,
  size_in_tiles: Vector2<usize>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Grid<T> {
  bounds: GridBounds,
  tiles: Vec<T>,
}

#[live_prop_test]
impl GridBounds {
  pub fn containing(bounds: [FloatingVector; 2]) -> GridBounds {
    let min_tile_center = bounds[0].containing_tile();
    let max_tile_center = bounds[1].containing_tile();
    GridBounds {
      min_tile_center,
      size_in_tiles: (max_tile_center - min_tile_center).map(|d| (d / TILE_WIDTH) as usize + 1),
    }
  }

  //#[live_prop_test(postcondition = "result.map_or(true, |index| self.position(index) == position)")]
  fn index(&self, position: GridVector) -> Option<usize> {
    let coords = (position - self.min_tile_center) / TILE_WIDTH;
    assert_eq!(
      self.min_tile_center + coords * TILE_WIDTH,
      position,
      "something gave wrong-parity coordinates to tile query"
    );
    if (0..2).all(|dim| (0..self.size_in_tiles[dim]).contains(&(coords[dim] as usize))) {
      Some(coords[0] as usize + coords[1] as usize * self.size_in_tiles[0])
    } else {
      None
    }
  }

  //#[live_prop_test(postcondition = "self.index(result) == Some(index)")]
  fn position(&self, index: usize) -> GridVector {
    let y = index / self.size_in_tiles[0];
    let x = index % self.size_in_tiles[0];
    self.min_tile_center + GridVector::new(x as i32 * TILE_WIDTH, y as i32 * TILE_WIDTH)
  }

  //#[live_prop_test(postcondition = "self.index(result.containing_tile()).is_some()")]
  pub fn clamp(&self, position: FloatingVector) -> FloatingVector {
    let epsilon = 0.0000001;
    let mut result = position;
    for dim in 0..2 {
      result[dim] = position[dim].clamp(
        ((self.min_tile_center[dim] - TILE_RADIUS) as f64) + epsilon,
        ((self.min_tile_center[dim] + (self.size_in_tiles[dim] as i32 * TILE_WIDTH) - TILE_RADIUS)
          as f64)
          - epsilon,
      );
    }
    result
  }

  pub fn tile_centers(&self) -> impl Iterator<Item = GridVector> + '_ {
    (0..self.size_in_tiles[0] * self.size_in_tiles[1]).map(move |index| self.position(index))
  }

  pub fn min_tile_corner(&self) -> GridVector {
    self.min_tile_center.map(|d| d - TILE_RADIUS)
  }

  pub fn max_tile_corner(&self) -> GridVector {
    self.min_tile_center.zip_map(&self.size_in_tiles, |d, s| {
      d + s as i32 * TILE_WIDTH - TILE_RADIUS
    })
  }

  pub fn min_tile_center(&self) -> GridVector {
    self.min_tile_center
  }

  pub fn max_tile_center(&self) -> GridVector {
    self
      .min_tile_center
      .zip_map(&self.size_in_tiles, |d, s| d + (s as i32 - 1) * TILE_WIDTH)
  }
}
pub type GridIter<'a, T: 'a> = impl Iterator<Item = (GridVector, &'a T)> + 'a;
pub type GridIterMut<'a, T: 'a> = impl Iterator<Item = (GridVector, &'a mut T)> + 'a;
#[live_prop_test]
impl<T: Default> Grid<T> {
  pub fn new(min_tile_center: GridVector, size_in_tiles: Vector2<usize>) -> Self {
    Grid {
      bounds: GridBounds {
        min_tile_center,
        size_in_tiles,
      },
      tiles: (0..size_in_tiles[0] * size_in_tiles[1])
        .map(|_| Default::default())
        .collect(),
    }
  }
}
impl<T> Grid<T> {
  pub fn get(&self, position: GridVector) -> Option<&T> {
    self
      .bounds
      .index(position)
      .map(move |index| self.tiles.get(index).unwrap())
  }
  pub fn get_mut(&mut self, position: GridVector) -> Option<&mut T> {
    self
      .bounds
      .index(position)
      .map(move |index| self.tiles.get_mut(index).unwrap())
  }
  pub fn iter(&self) -> GridIter<T> {
    let bounds = &self.bounds;
    self
      .tiles
      .iter()
      .enumerate()
      .map(move |(index, tile)| (bounds.position(index), tile))
  }
  pub fn iter_mut(&mut self) -> GridIterMut<T> {
    let bounds = &self.bounds;
    self
      .tiles
      .iter_mut()
      .enumerate()
      .map(move |(index, tile)| (bounds.position(index), tile))
  }
  pub fn bounds(&self) -> &GridBounds {
    &self.bounds
  }

  pub fn tiles_near(
    &self,
    position: FloatingVector,
    range: f64,
  ) -> impl Iterator<Item = (GridVector, &T)> {
    let range_squared = range * range;
    let x_range = (nearest_grid_center(position[0] - range)
      ..=nearest_grid_center(position[0] + range))
      .step_by(TILE_WIDTH as usize);
    let y_range = (nearest_grid_center(position[1] - range)
      ..=nearest_grid_center(position[1] + range))
      .step_by(TILE_WIDTH as usize);
    x_range.flat_map(move |x| {
      y_range.clone().filter_map(move |y| {
        let closest = FloatingVector::new(
          moved_towards(x as f64, position[0], TILE_RADIUS as f64),
          moved_towards(y as f64, position[1], TILE_RADIUS as f64),
        );
        if (closest - position).magnitude_squared() > range_squared {
          return None;
        }
        let tile_position = GridVector::new(x, y);
        self.get(tile_position)
      })
    })
  }
}

impl<'a, T: 'a> IntoIterator for &'a Grid<T> {
  type Item = (GridVector, &'a T);
  type IntoIter = GridIter<'a, T>;

  fn into_iter(self) -> GridIter<'a, T> {
    self.iter()
  }
}

impl<'a, T: 'a> IntoIterator for &'a mut Grid<T> {
  type Item = (GridVector, &'a mut T);
  type IntoIter = GridIterMut<'a, T>;

  fn into_iter(self) -> GridIterMut<'a, T> {
    self.iter_mut()
  }
}
