use super::*;

use ordered_float::OrderedFloat;
use arrayvec::ArrayVec;

pub type Vector3 = nalgebra::Vector3 <f64>;
pub type Rotation3 = nalgebra::Rotation3 <f64>;
pub type Vector2 = nalgebra::Vector2 <f64>;
pub type Rotation2 = nalgebra::Rotation2 <f64>;

pub const TURN: f64 = ::std::f64::consts::PI*2.0;
#[derive (Debug, Default, Serialize, Deserialize)]
pub struct CylindricalPerspective {
  pub width_at_closest: f64,
  pub camera_distance_along_tangent: f64,
  pub radians_visible: f64,
  pub horizon_drop: f64,
}
impl CylindricalPerspective {
  pub fn coordinates_on_circle_relative_to_camera (&self, fraction_of_visible: f64)->Vector2 {
    let radians = self.radians_visible*(1.0 - fraction_of_visible);
    Vector2::new (
      self.camera_distance_along_tangent - radians.sin(),
      1.0 - radians.cos()
    )
  }
  
  
  pub fn scale (&self, fraction_of_visible: f64)->f64 {
    let coordinates = self.coordinates_on_circle_relative_to_camera (fraction_of_visible);
    let coordinates0 = self.coordinates_on_circle_relative_to_camera (0.0);
    coordinates0.norm()/coordinates.norm()/self.width_at_closest
  }
  pub fn ground_screen_drop (&self, fraction_of_visible: f64)->f64 {
    let coordinates = self.coordinates_on_circle_relative_to_camera (fraction_of_visible);
    let coordinates0 = self.coordinates_on_circle_relative_to_camera (0.0);
    self.horizon_drop + (1.0 - self.horizon_drop)*coordinates [1].atan2(coordinates [0])/coordinates0 [1].atan2(coordinates0 [0])
  }
  
  pub fn screen_drop_to_fraction_of_visible (&self, screen_drop: f64)->f64 {
    //let coordinates0 = self.coordinates_on_circle_relative_to_camera (0.0);
    if screen_drop < self.horizon_drop {return 1.0;}
    //let camera_angle = (screen_drop - self.horizon_drop)/(1.0 - self.horizon_drop)*coordinates0 [1].atan2(coordinates0 [0]);
    //eh, forget figuring out the formulas, this is an infrequent operation
    let mut min = 0.0;
    let mut max = 1.0;
    while max - min > 0.0001 {
      let mid = (max + min)/2.0;
      let test_drop = self.ground_screen_drop (mid);
      if test_drop > screen_drop { min = mid; } else { max = mid; }
    }
    min
  }
}

pub fn segment_intersection (first: [Vector2; 2], second: [Vector2; 2])->Option <Vector2> {
  type Point2D = ::lyon::geom::euclid::Point2D<f64>;
  type LineSegment = ::lyon::geom::LineSegment<f64>;
  fn convert (vector: Vector2)->Point2D {
    ::lyon::math::point(vector[0], vector[1])
  }
  fn convert_back (vector: Point2D)->Vector2 {
    Vector2::new (vector.x, vector.y)
  }
  fn convert_segment (segment: [Vector2; 2])->LineSegment {
    LineSegment {from: convert (segment[0]), to: convert (segment[1])}
  }
  convert_segment (first).intersection (& convert_segment (second)).map (| vector | convert_back (vector))
}

pub struct ScreenMountain {
  pub peak: Vector2,
  pub radius: f64,
}
impl ScreenMountain {
  pub fn sample (&self, horizontal: f64)->Option <f64> {
    let distance = (self.peak [0] - horizontal).abs();
    if distance > self.radius {return None;}
    let fraction = distance/self.radius;
    Some(self.peak [1]*(1.0 - fraction))
  }
  pub fn key_points (&self, visible_radius: f64)->[Vector2; 3] {
    [
      self.peak,
      if self.peak [0] - self.radius < -visible_radius { Vector2::new (-visible_radius, self.sample (-visible_radius).unwrap_or (0.0))} else {Vector2::new (self.peak [0] - self.radius, 0.0)},
      if self.peak [0] + self.radius >  visible_radius { Vector2::new ( visible_radius, self.sample ( visible_radius).unwrap_or (0.0))} else {Vector2::new (self.peak [0] + self.radius, 0.0)},
    ]
  }
  pub fn intersection_points (&self, other: & ScreenMountain, visible_radius: f64)->ArrayVec<[Vector2; 4]> {
    let mut result = ArrayVec::new();
    let distance = (self.peak [0] - other.peak [0]).abs();
    if distance > self.radius + other.radius {return result;}
    let my_points = self.key_points(visible_radius);
    let other_points = other.key_points(visible_radius);
    if let Some(intersection) = segment_intersection ([self.peak, my_points [1]], [other.peak, other_points [1]]) {
      result.push (intersection);
    }
    if let Some(intersection) = segment_intersection ([self.peak, my_points [2]], [other.peak, other_points [1]]) {
      result.push (intersection);
    }
    if let Some(intersection) = segment_intersection ([self.peak, my_points [1]], [other.peak, other_points [2]]) {
      result.push (intersection);
    }
    if let Some(intersection) = segment_intersection ([self.peak, my_points [2]], [other.peak, other_points [2]]) {
      result.push (intersection);
    }
    result.retain (| intersection | intersection [0].abs() <= visible_radius);
    result
  }
}

pub fn skyline (visible_radius: f64, mountains: & [ScreenMountain])->Vec<Vector2> {
  let mut points = Vec::with_capacity(mountains.len()*10);
  points.push (Vector2::new (-visible_radius, 0.0));
  points.push (Vector2::new (visible_radius, 0.0));
  for (index, mountain) in mountains.iter().enumerate() {
    points.extend (mountain.key_points(visible_radius).iter().cloned());
    for other in mountains [index + 1..].iter() {
      points.extend (mountain.intersection_points (other, visible_radius));
    }
  }
  points.retain (| point | point [0].abs() <= visible_radius && mountains.iter().all (| mountain | {
    point [1] > mountain.sample (point [0]).unwrap_or (0.0) - 0.00001
  }));
  points.sort_by_key (| point | OrderedFloat (point [0]));
  points
}



pub fn move_to (location: Vector2) {
  js! {context.moveTo (@{location [0]},@{location [1]});}
}
pub fn line_to (location: Vector2) {
  js! {context.lineTo (@{location [0]},@{location [1]});}
}
pub fn translate (location: Vector2) {
  js! {context.translate (@{location [0]},@{location [1]});}
}
pub fn quadratic_curve (control: Vector2, location: Vector2) {
  js! {context.quadraticCurveTo (@{control [0]},@{control [1]},@{location [0]},@{location [1]});}
}
/*pub fn sigmoidneg11(input: f64)->f64 {
  (input*(TURN/4.0)).sin()
}
pub fn sigmoid01(input: f64)->f64 {
  (sigmoidneg11((input*2.0)-1.0)+1.0)/2.0
}*/

pub fn min (first: f64, second: f64)->f64 {if first < second {first} else {second}}
pub fn max (first: f64, second: f64)->f64 {if first > second {first} else {second}}
pub fn safe_normalize (vector: Vector2)->Vector2 {
  let norm = vector.norm();
  if norm == 0.0 {vector} else {vector/norm}
}

pub fn as_ground (location: Vector2)->Vector3 {Vector3::new (location [0], location [1], 0.0)}

impl Path {
  pub fn closest_components (&self, vertical_position: f64)->[Option <&Component>; 2] {
    let next = match self.components.binary_search_by_key (&OrderedFloat (vertical_position), | component | OrderedFloat (component.center [1])) {
      Ok(i)=>i, Err(i)=>i,
    };
    [self.components.get (next.wrapping_sub (1)), self.components.get (next)]
  }
  
  pub fn horizontal_center (&self, vertical_position: f64)->f64 {
    match self.closest_components (vertical_position) {
      [None, None] => unreachable!(),
      [None, Some(component)] | [Some(component), None] => component.center [0],
      [Some(first), Some(second)] => {
        let fraction = (vertical_position - first.center [1])/(second.center [1] - first.center [1]);
        first.center [0]*(1.0 - fraction) + second.center [0]*fraction
      },
    }
  }
}

impl State {
  pub fn visible_range (&self)->(f64, f64) {
    let min_visible_position = self.player.center [1] - self.constants.player_position;
    let max_visible_position = min_visible_position + self.constants.visible_length;
    (min_visible_position, max_visible_position)
  }
  
  pub fn fraction_of_visible (&self, location: Vector3)->f64 {
    (location [1] - self.player.center [1] + self.constants.player_position)/self.constants.visible_length
  }
  pub fn draw_scale (&self, location: Vector3)->f64 {
    let fraction_of_visible = self.fraction_of_visible (location);
    self.constants.perspective.scale (fraction_of_visible)
  }
  pub fn draw_position (&self, location: Vector3)->Vector2 {
    let fraction_of_visible = self.fraction_of_visible (location);
    let horizontal_distance = location [0] - self.player.center [0];

    let scale = self.constants.perspective.scale (fraction_of_visible);
    let drop = self.constants.perspective.ground_screen_drop (fraction_of_visible);
    
    Vector2::new (
      horizontal_distance*scale,
      drop - location [2]*scale,
    )
  }
  
  pub fn screen_to_ground (&self, screen_coordinates: Vector2)->Vector2 {
    let fraction_of_visible = self.constants.perspective.screen_drop_to_fraction_of_visible(screen_coordinates [1]);
    let scale = self.constants.perspective.scale (fraction_of_visible);
    Vector2::new (
      screen_coordinates [0]/scale + self.player.center [0],
      (fraction_of_visible*self.constants.visible_length) + self.player.center [1] - self.constants.player_position,
    )
  }
}

