use super::*;

use rand::Rng;
use stdweb::unstable::TryInto;
use ordered_float::OrderedFloat;

pub type Vector3 = nalgebra::Vector3 <f64>;
pub type Rotation3 = nalgebra::Rotation3 <f64>;
pub type Vector2 = nalgebra::Vector2 <f64>;
pub type Rotation2 = nalgebra::Rotation2 <f64>;

pub const TURN: f64 = ::std::f64::consts::PI*2.0;

pub fn random_vector_exact_length <G: Rng> (generator: &mut G, length: f64)->Vector2 {
  loop {
    let vector = Vector2::new (
      generator.gen_range (- length, length),
      generator.gen_range (- length, length),);
    let test_length = vector.norm();
    if test_length <= length && test_length*2.0 >= length {
      return vector*length/vector.norm();
    }
  }
}
pub fn random_vector_within_length <G: Rng> (generator: &mut G, length: f64)->Vector2 {
  loop {
    let vector = Vector2::new (
      generator.gen_range (- length, length),
      generator.gen_range (- length, length),);
    let test_length = vector.norm();
    if test_length <= length && test_length != 0.0 {
      return vector;
    }
  }
}
pub fn auto_constant (name: & str, default: f64)->f64 {
  (js!{
    var value = window.auto_constants [@{name}];
    if (value === undefined) {
      return window.auto_constants [@{name}] = @{default};
    }
    return value;
  }).try_into().unwrap()
}

#[derive (Debug, Default, Deserialize)]
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
    let coordinates0 = self.coordinates_on_circle_relative_to_camera (0.0);
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

