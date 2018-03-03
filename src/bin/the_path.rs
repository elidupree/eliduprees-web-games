#![recursion_limit="256"]
#![feature (slice_patterns)]

extern crate eliduprees_web_games;

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate derivative;
extern crate nalgebra;
extern crate rand;
extern crate ordered_float;
extern crate boolinator;

use rand::Rng;
use stdweb::web;
use stdweb::unstable::TryInto;
use ordered_float::OrderedFloat;
use boolinator::Boolinator;

use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::str::FromStr;

type Vector3 = nalgebra::Vector3 <f64>;
type Rotation3 = nalgebra::Rotation3 <f64>;
type Vector2 = nalgebra::Vector2 <f64>;
type Rotation2 = nalgebra::Rotation2 <f64>;

const TURN: f64 = ::std::f64::consts::PI*2.0;

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
fn auto_constant (name: & str, default: f64)->f64 {
  (js!{
    var value = window.auto_constants [@{name}];
    if (value === undefined) {
      return window.auto_constants [@{name}] = @{default};
    }
    return value;
  }).try_into().unwrap()
}


#[derive (Debug, Default, Deserialize)]
struct Constants {
  visible_components: i32,
  visible_length: f64,
  perspective: CylindricalPerspective,
  
  player_position: f64,
  player_max_speed: f64,
  
  spawn_radius: f64,
  spawn_distance: f64,
  
  mountain_spawn_radius: f64,
  mountain_spawn_distance: f64,
  mountain_viewable_distances_radius: f64,
  mountain_density: f64,
  
  monster_density: f64,
  tree_density: f64,
  chest_density: f64,
  reward_density: f64,
  
  fadein_distance: f64,
  
  speech_fade_duration: f64,
  speech_duration: f64,
  
  fall_duration: f64,
}
js_deserializable! (Constants);

#[derive (Debug)]
struct Mountain {
  fake_peak_location: Vector3,
  base_screen_radius: f64,
}
#[derive (Debug)]
struct Sky {
  screen_position: Vector2,
  steepness: f64,
}
#[derive (Debug, Derivative)]
#[derivative (Default)]
struct Object {
  #[derivative (Default (value = "Vector2::new(0.0,0.0)"))]
  center: Vector2,
  #[derivative (Default (value = "Vector2::new(0.0,0.0)"))]
  velocity: Vector2,
  radius: f64,
  statements: Vec<Statement>,
  last_statement_start_time: Option <f64>,
  falling: Option <Fall>,
  collect_progress: f64,
  
  automatic_statements: Vec<AutomaticStatement>,
  #[derivative (Default (value = "Kind::Tree"))]
  kind: Kind,
}
#[derive (Debug)]
struct Statement {
  text: String,
  start_time: f64,
  response: Option <String>,
  direction: Cell <f64>,
}
#[derive (Debug)]
struct AutomaticStatement {
  text: String,
  distances: [f64; 2],
  last_stated: Option <f64>,
}
#[derive (Debug)]
enum Kind {
  Person (Person),
  Chest,
  Reward,
  Monster (Monster),
  Tree,
}
#[derive (Debug)]
struct Monster {
}
#[derive (Debug)]
struct Person {
  planted_foot: usize,
  feet: [Vector2; 2],
}
#[derive (Clone, Debug)]
struct Fall {
  progress: f64,
  distance: f64,
}

#[derive (Clone, Debug, Default)]
struct Path {
  components: Vec<Component>,
  max_speed: f64,
  radius: f64,
}
#[derive (Clone, Debug)]
struct Component {
  center: Vector2,
  
  // per unit distance forward
  velocity: f64,
  acceleration: f64,
}
#[derive (Debug)]
struct Click {
  location: Vector2,
  player_location: Vector2,
  distance_traveled: f64,
  time: f64,
}

#[derive (Derivative)]
#[derivative (Default)]
struct State {
  mountains: Vec<Mountain>,
  skies: Vec<Sky>,
  objects: Vec<Object>,
  path: Path,
  player: Object,
  companion: Object,
  
  permanent_pain: f64,
  permanent_pain_smoothed: f64,
  temporary_pain: f64,
  temporary_pain_smoothed: f64,
  
  last_click: Option <Click>,
  distance_traveled: f64,
  
  stars_collected: i32,
  
  #[derivative (Default (value = "Box::new(::rand::ChaChaRng::new_unseeded())"))]
  generator: Box <Rng>,
  constants: Rc<Constants>,
  now: f64,
}

#[derive (Debug, Default, Deserialize)]
struct CylindricalPerspective {
  width_at_closest: f64,
  camera_distance_along_tangent: f64,
  radians_visible: f64,
  horizon_drop: f64,
}
impl CylindricalPerspective {
  fn coordinates_on_circle_relative_to_camera (&self, fraction_of_visible: f64)->Vector2 {
    let radians = self.radians_visible*(1.0 - fraction_of_visible);
    Vector2::new (
      self.camera_distance_along_tangent - radians.sin(),
      1.0 - radians.cos()
    )
  }
  
  
  fn scale (&self, fraction_of_visible: f64)->f64 {
    let coordinates = self.coordinates_on_circle_relative_to_camera (fraction_of_visible);
    let coordinates0 = self.coordinates_on_circle_relative_to_camera (0.0);
    coordinates0.norm()/coordinates.norm()/self.width_at_closest
  }
  fn ground_screen_drop (&self, fraction_of_visible: f64)->f64 {
    let coordinates = self.coordinates_on_circle_relative_to_camera (fraction_of_visible);
    let coordinates0 = self.coordinates_on_circle_relative_to_camera (0.0);
    self.horizon_drop + (1.0 - self.horizon_drop)*coordinates [1].atan2(coordinates [0])/coordinates0 [1].atan2(coordinates0 [0])
  }
  
  fn screen_drop_to_fraction_of_visible (&self, screen_drop: f64)->f64 {
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


fn move_to (location: Vector2) {
  js! {context.moveTo (@{location [0]},@{location [1]});}
}
fn line_to (location: Vector2) {
  js! {context.lineTo (@{location [0]},@{location [1]});}
}
fn translate (location: Vector2) {
  js! {context.translate (@{location [0]},@{location [1]});}
}
fn quadratic_curve (control: Vector2, location: Vector2) {
  js! {context.quadraticCurveTo (@{control [0]},@{control [1]},@{location [0]},@{location [1]});}
}
/*fn sigmoidneg11(input: f64)->f64 {
  (input*(TURN/4.0)).sin()
}
fn sigmoid01(input: f64)->f64 {
  (sigmoidneg11((input*2.0)-1.0)+1.0)/2.0
}*/

fn min (first: f64, second: f64)->f64 {if first < second {first} else {second}}
fn max (first: f64, second: f64)->f64 {if first > second {first} else {second}}

fn as_ground (location: Vector2)->Vector3 {Vector3::new (location [0], location [1], 0.0)}


impl Fall {
  fn info (&self, constants: & Constants, velocity: Vector2)->(Vector2, f64) {
      let hit_ground = auto_constant ("hit_ground", 0.3);
      let start_getting_up = auto_constant ("start_getting_up", 1.5);
      let start_moving_fraction = auto_constant ("start_moving_fraction", 0.1);
      let finish = constants.fall_duration;
      let fallen_angle = self.distance.signum()*TURN/4.0;
      if self.progress < hit_ground {
        let fraction = self.progress/hit_ground;
        (Vector2::new (self.distance/hit_ground, 0.0), fraction*fraction*fallen_angle) 
      }
      else if self.progress < start_getting_up {
        (Vector2::new (0.0, 0.0), fallen_angle)
      }
      else {
        let fraction = (self.progress - start_getting_up)/(finish - start_getting_up);
        let velocity_factor = if fraction < start_moving_fraction {0.0} else {(fraction - start_moving_fraction)/(1.0 - start_moving_fraction)};
        let steepness = auto_constant ("rise_steepness", 4.0);
        let angle_frac = 
          fraction*fraction*fraction*fraction*0.0
          + 4.0*fraction*fraction*fraction*(1.0 - fraction)*0.0
          + 6.0*fraction*fraction*(1.0 - fraction)*(1.0 - fraction)*((0.5-fraction)*steepness + 0.5)
          + 4.0*fraction*(1.0 - fraction)*(1.0 - fraction)*(1.0 - fraction)*1.0
          + (1.0 - fraction)*(1.0 - fraction)*(1.0 - fraction)*(1.0 - fraction)*1.0;
        (velocity*velocity_factor, angle_frac*fallen_angle)
      }
  }
}

impl Object {
  fn say (&mut self, statement: Statement) {
    self.last_statement_start_time = Some(statement.start_time);
    self.statements.push (statement);
  }
  fn move_object (&mut self, movement: Vector2) {
    self.center += movement;
    match self.kind {
      Kind::Person (ref mut person) => {
        // hack: subdivide movement to reduce error in foot switching
        let mut perpendicular = Vector2::new (movement [1], - movement [0]);
        let norm = perpendicular.norm();
        if norm != 0.0 {
          perpendicular /= norm;
          let limit = self.radius*auto_constant("feet_motion_limit", 0.8);
          let parts = (movement.norm()*10.0/limit).ceil();
          for _ in 0..parts as usize {
            let planted_foot = person.planted_foot;
            let moving_foot = 1 - planted_foot;
            person.feet [moving_foot] += movement/parts;
            person.feet [planted_foot] -= movement/parts;
            
            let mut perpendicular_component_size = person.feet [moving_foot].dot(& perpendicular);
            if perpendicular_component_size < 0.0 {
              perpendicular_component_size = -perpendicular_component_size;
              perpendicular = -perpendicular;
            }
            let mut adjustment_size = (movement.norm()/parts)*auto_constant("feet_perpendicular_adjustement_ratio", 0.4);
            if adjustment_size > perpendicular_component_size {adjustment_size = perpendicular_component_size;}
            person.feet [moving_foot] -= perpendicular * adjustment_size;
            if person.feet [moving_foot].norm() > limit && person.feet [moving_foot].dot (&movement) > 0.0 {
              person.planted_foot = moving_foot;
            }
          }
        }
      },
      _=>(),
    }
  }
}

impl Path {
  fn closest_components (&self, vertical_position: f64)->[Option <&Component>; 2] {
    let lower = match self.components.binary_search_by_key (&OrderedFloat (vertical_position), | component | OrderedFloat (component.center [1])) {
      Ok(i)=>i, Err(i)=>i,
    };
    [self.components.get (lower), self.components.get (lower + 1)]
  }
  
  fn horizontal_center (&self, vertical_position: f64)->f64 {
    match self.closest_components (vertical_position) {
      [None, None] => unreachable!(),
      [None, Some(component)] | [Some(component), None] => component.center [0],
      [Some(first), Some(second)] => {
        let fraction = (vertical_position - first.center [1])/(second.center [1] - first.center [1]);
        first.center [0]*(1.0 - fraction) + second.center [0]*fraction
      },
    }
  }
  
  fn extend (&mut self, until: f64, player_horizontal: f64, generator: &mut Box<Rng>, constants: & Constants) {
    while self.components.last().unwrap().center [1] <until {
      let previous = self.components.last().unwrap().clone();
      let distance = constants.visible_length/constants.visible_components as f64;
      let mut new = Component {
        center: previous.center + Vector2::new (distance*previous.velocity, distance),
        velocity: previous.velocity + distance*previous.acceleration,
        acceleration: previous.acceleration,
      };
      
      let default_acceleration_change_radius = self.max_speed*216.0*distance;
      let mut bias = - previous.velocity*36.0*distance;
      // The path secretly follows the player if the player moves too far away,
      // for both gameplay and symbolism reasons.
      let player_offset = player_horizontal - previous.center [0];
      if player_offset > 0.7 {
        bias += (player_offset - 0.7)*self.max_speed*40.0*distance;
      }
      if player_offset < -0.7 {
        bias += (player_offset + 0.7)*self.max_speed*40.0*distance;
      }
      
      let limits_1 = [
        previous.acceleration - default_acceleration_change_radius + bias,
        previous.acceleration + default_acceleration_change_radius + bias,
      ];
      // It's forbidden to accelerate to higher than max speed.
      // To keep things smooth, we never accelerate more than a fraction of the way to max speed at a time.
      // TODO: make this formula less dependent on the component size
      let limits_2 = [
        (-self.max_speed - previous.velocity)*200.0,
        (self.max_speed - previous.velocity)*200.0,
      ];
      let acceleration_limits = [
        if limits_1 [0] > limits_2 [0] {limits_1 [0]} else {limits_2 [0]},
        if limits_1 [1] < limits_2 [1] {limits_1 [1]} else {limits_2 [1]},
      ];
      
      //println!("{:?}", (limits_1, limits_2, acceleration_limits));
      if acceleration_limits[0] < acceleration_limits[1] {
        new.acceleration = generator.gen_range (acceleration_limits [0], acceleration_limits [1]);
      }
      else {
        new.acceleration = (acceleration_limits[0] + acceleration_limits[1]) /2.0;
      }
      
      self.components.push (new);
    }
  }
}

impl State {
  fn do_spawns_impl <G: FnMut(f64, &mut Box<Rng>)->f64, F: FnMut()->Object> (&mut self, advance_distance: f64, average_number: f64, mut horizontal_position_generator: G, mut object_generator: F) {
    let attempts = (average_number*10.0).ceil() as usize;
    for _ in 0..attempts {
      if self.generator.gen::<f64>() < average_number/attempts as f64 {
        let mut object = (object_generator)();
        let vertical_position = self.player.center [1] + self.constants.spawn_distance - self.generator.gen_range(0.0, advance_distance);
        object.center = Vector2::new ((horizontal_position_generator)(vertical_position, &mut self.generator), vertical_position);
        self.objects.push (object);
      }
    }
  }
  fn do_spawns <F: FnMut()->Object> (&mut self, advance_distance: f64, density: f64, object_generator: F) {
    let spawn_area = advance_distance*self.constants.spawn_radius*2.0;
    let average_number = spawn_area*density;
    let radius = self.constants.spawn_radius;
    let player_center = self.player.center [0];
    self.do_spawns_impl (advance_distance, average_number, |_, generator| player_center + generator.gen_range (-radius, radius), object_generator);
  }
  fn mountain_screen_peak (&self, mountain: & Mountain)->Vector2 {
    let distance = mountain.fake_peak_location [1] - self.player.center [1];
    let highest_distance = self.constants.mountain_spawn_distance - self.constants.mountain_viewable_distances_radius;
    let distance_from_highest = (distance - highest_distance).abs() / self.constants.mountain_viewable_distances_radius;
    Vector2::new (
      (mountain.fake_peak_location [0] - self.player.center [0])/distance,
      self.constants.perspective.horizon_drop - (mountain.fake_peak_location [2])*(distance_from_highest*(TURN/4.0)).cos()
    )
  }
  fn spawn_mountains (&mut self, advance_distance: f64) {
    let spawn_area = advance_distance*self.constants.mountain_spawn_radius*2.0;
    let average_number = spawn_area*self.constants.mountain_density;
    let attempts = (average_number*10.0).ceil() as usize;
    for _ in 0..attempts {
      if self.generator.gen::<f64>() < average_number/attempts as f64 {
        let location = Vector3::new (
          self.player.center [0] + self.generator.gen_range (- self.constants.mountain_spawn_radius, self.constants.mountain_spawn_radius),
          self.player.center [1] + self.constants.mountain_spawn_distance - self.generator.gen_range(0.0, advance_distance),
          self.generator.gen_range (0.1, 0.2),
        );
        let mountain = Mountain {
          fake_peak_location: location,
          base_screen_radius: self.generator.gen_range (0.3, 0.6),
        };
        let screen_peak = self.mountain_screen_peak (& mountain)[0];
        let partial_radius = mountain.base_screen_radius * 0.8;
        if screen_peak + partial_radius < 0.0 || 0.0 < screen_peak - partial_radius {
          self.mountains.push (mountain);
        }
      }
    }
  }
  
  fn visible_range (&self)->(f64, f64) {
    let min_visible_position = self.player.center [1] - self.constants.player_position;
    let max_visible_position = min_visible_position + self.constants.visible_length;
    (min_visible_position, max_visible_position)
  }
  
  fn simulate (&mut self, duration: f64) {
    let tick_start = self.now;
    self.now += duration;
    let now = self.now;
    let constants = self.constants.clone();
    for sky in self.skies.iter_mut() {
      sky.screen_position [0] += 0.005*duration*self.generator.gen_range (-1.0, 1.0);
      sky.screen_position [1] += 0.005*duration*self.generator.gen_range (-1.0, 1.0);
      sky.screen_position [0] -= (sky.screen_position [0])*0.00006*duration;
      sky.screen_position [1] -= (sky.screen_position [1] - 0.7*self.constants.perspective.horizon_drop)*0.00003*duration;
    }
    
    // hack: initialization stuff
    if tick_start == 0.0 {
      // hack: make the player start on the path even though the path isn't generated yet
      let (_min_visible_position, max_visible_position) = self.visible_range();
      self.path.extend(max_visible_position, self.player.center [0], &mut self.generator, &constants);
      self.player.center [0] = self.path.horizontal_center (self.player.center [1]);
      self.companion.center [0] = self.path.horizontal_center (self.companion.center [1]);
      
      // generate as many mountains as would be visible
      self.spawn_mountains (constants.mountain_viewable_distances_radius*2.0);
    }
    
    let movement_direction = if let Some(click) = self.last_click.as_ref() {click.location - click.player_location} else {Vector2::new (0.0, 1.0)};
    self.player.velocity = movement_direction*constants.player_max_speed/movement_direction.norm();
    
    if let Some(ref fall) = self.player.falling {
      let (velocity,_) = fall.info (& constants, self.player.velocity);
      self.player.velocity = velocity;
    }
    
    self.player.falling = self.player.falling.take().and_then (| mut fall | {
      fall.progress += duration;
      (fall.progress < constants.fall_duration).as_some (fall)
    });

        
    let mut time_moved = duration;
    let mut collision = None;
    for (index, object) in self.objects.iter().enumerate() {
      let relative_velocity = object.velocity - self.player.velocity;
      let relative_location = object.center - self.player.center;
      if relative_velocity [1] < 0.0 && relative_location [1] > 0.0 {
        let time_to_collide = -relative_location [1] / relative_velocity [1];
        let horizontal_difference_when_aligned = relative_location[0] + relative_velocity[0]*time_to_collide;
        if horizontal_difference_when_aligned.abs() < object.radius + self.player.radius {
          if time_to_collide < time_moved {
            time_moved = time_to_collide - 0.0001;
            collision = Some(index);
          }
        }
      }
    }
    
    let movement_vector = self.player.velocity*time_moved;
    let advance_distance = movement_vector [1];
    
    self.player.move_object (movement_vector);
    self.distance_traveled += movement_vector.norm();
    
    let player_center = self.player.center;
    let (min_visible_position, max_visible_position) = self.visible_range();
    
    self.spawn_mountains (advance_distance);
    self.mountains.retain (| mountain | {
      let distance = mountain.fake_peak_location [1] - player_center [1];
      distance > constants.mountain_spawn_distance - constants.mountain_viewable_distances_radius*2.0
    });
    self.path.extend(max_visible_position, self.player.center [0], &mut self.generator, &constants);
    self.path.components.retain (| component | component.center [1] >= min_visible_position - constants.visible_length/constants.visible_components as f64);
    
    self.do_spawns (advance_distance, constants.tree_density, || Object {kind: Kind::Tree, radius: 0.05, .. Default::default()});
    self.do_spawns (advance_distance, constants.monster_density, || Object {kind: Kind::Monster (Monster {}), radius: 0.05, .. Default::default()});
    self.do_spawns (advance_distance, constants.chest_density, || Object {kind: Kind::Chest, radius: 0.03, .. Default::default()});
    self.do_spawns (advance_distance, constants.reward_density, || Object {kind: Kind::Reward, radius: 0.03, .. Default::default()});
    
    {
    // hack-ish: chests and rewards appear more frequently on or near the path.
    // Symbolism-wise, it should be the path that slightly steers towards rewards,
    // not the rewards that appear in the path.
    // But this way is simpler, and they're not very distinguishable to the player in practice.
    let hack = self.path.clone();
    let factor = auto_constant ("path_reward_frequency_factor", 0.1);
    self.do_spawns_impl (advance_distance,
      advance_distance*constants.reward_density*factor,
      | vertical, generator | {
        hack.horizontal_center (vertical)
        + generator.gen_range (-hack.radius, hack.radius)
        + generator.gen_range (-hack.radius, hack.radius)
      },
      || Object {kind: Kind::Reward, radius: 0.03, .. Default::default()}
    );
    self.do_spawns_impl (advance_distance,
      advance_distance*constants.chest_density*factor,
      | vertical, generator | {
        hack.horizontal_center (vertical)
        + generator.gen_range (-hack.radius, hack.radius)
        + generator.gen_range (-hack.radius, hack.radius)
      },
      || Object {kind: Kind::Reward, radius: 0.03, .. Default::default()}
    );
    }
    
    for object in self.objects.iter_mut() {
      object.center += object.velocity*duration;
      match object.kind {
        Kind::Monster (ref mut monster) => {
          object.velocity += random_vector_within_length (&mut self.generator, 0.1*duration);
          if object.velocity.norm() > 0.1 {
            object.velocity *= 0.5f64.powf (duration/1.0);
          }
        },
        _=>(),
      };
    }
    let mut companion_say = None;
    let player_distance_from_path = (self.path.horizontal_center (self.player.center [1]) - self.player.center [0]).abs()/self.path.radius;
    for object in self.objects.iter_mut().chain(::std::iter::once(&mut self.player)).chain(::std::iter::once(&mut self.companion)) {
      for statement in object.statements.iter_mut() {
        if now - statement.start_time > auto_constant ("response_time", 1.0) && statement.response.is_some() {
          companion_say = statement.response.take();
        }
      }
      object.statements.retain (| statement | statement.start_time + constants.speech_duration > now);
    }
    
    let companion_movement_vector = Vector2::new (
      self.path.horizontal_center (self.companion.center [1] + advance_distance) - self.companion.center [0],
      advance_distance,
    );
    self.companion.move_object (companion_movement_vector);
    let companion_center = self.companion.center;
    
    if let Some(index) = collision {
      {
      let object = &mut self.objects [index];
      match object.kind {
        Kind::Tree => {
          let center_distance = self.player.center [0] - object.center [0];
          let direction = if center_distance <0.0 {- 1.0} else {1.0};
          let minimal_escape_distance = (self.player.radius + object.radius) *direction - center_distance;
          self.player.falling = Some(Fall {
            distance: minimal_escape_distance + self.player.radius*0.25 *direction,
            progress: 0.0,
          });
          self.player.statements.push (Statement {
            text: String::from_str ("Ow, it hurts").unwrap(),
            start_time: now,
            response: Some(String::from_str (if player_distance_from_path < 1.2 {"That's just part of life"} else {"It's your fault for straying"}).unwrap()),
            direction: Cell::new (direction),
          });
          self.temporary_pain += 1.0;
        },
        Kind::Reward => {
          if object.collect_progress == 0.0 {
            self.permanent_pain -= 0.10;
            self.player.statements.push (Statement {
              text: String::from_str ("Yay!").unwrap(),
              start_time: now,
              response: Some(String::from_str (if player_distance_from_path < 1.2 {"I'm proud of you"} else {"That's not good for you"}).unwrap()),
              direction: Cell::new (1.0),
            });
          }
          object.collect_progress += duration*0.7;
          
        },
        Kind::Chest => {
          if object.collect_progress == 0.0 {
            //self.permanent_pain -= 0.05;
            self.player.statements.push (Statement {
              text: String::from_str ("What's inside?").unwrap(),
              start_time: now,
              response: None,
              direction: Cell::new (-1.0),
            });
          }
          object.collect_progress += duration*1.5;
        },
        Kind::Monster(_) => {
          self.permanent_pain += 0.22;
          self.temporary_pain += 1.4;
          self.player.statements.push (Statement {
            text: String::from_str ("Ow, it hurts!").unwrap(),
            start_time: now,
            response: Some(String::from_str (if player_distance_from_path < 1.2 {"Liar, that would never happen on the path"} else {"It's your fault for straying"}).unwrap()),
            direction: Cell::new (1.0),
          });
        },
        Kind::Person(_) => unreachable!(),
      }
      }
      if self.objects [index].collect_progress >= 1.0 {
        self.objects.remove (index);
      }
    }
    self.objects.retain (| object | {
      object.center [1] > player_center[1] - 0.5
    });
    
    self.permanent_pain_smoothed = self.permanent_pain +
      (self.permanent_pain_smoothed - self.permanent_pain) * 0.5f64.powf(duration/auto_constant ("permanent_pain_smoothed_halflife", 0.7));
    self.temporary_pain = self.permanent_pain_smoothed +
      (self.temporary_pain - self.permanent_pain_smoothed) * 0.5f64.powf(duration/auto_constant ("temporary_pain_halflife", 1.4));
    self.temporary_pain_smoothed = self.temporary_pain +
      (self.temporary_pain_smoothed - self.temporary_pain) * 0.5f64.powf(duration/auto_constant ("temporary_pain_smoothed_halflife", 0.1));
    
    
    if companion_say.is_none() {for statement in self.companion.automatic_statements.iter_mut() {
      if self.companion.last_statement_start_time.map_or (true, | when | now > when + 5.0)
          && statement.last_stated.map_or (true, | when | now > when + 100.0) {
        let distance = player_distance_from_path;
        if distance >= statement.distances [0] && distance <= statement.distances [1] {
          statement.last_stated = Some(now);
          companion_say = Some(statement.text.clone());
          break
        }
      }
    }}
    if let Some(companion_say) = companion_say {
      let virtual_location = player_center [0] + self.player.statements.iter().map(| statement | statement.direction.get()).sum::<f64>() / (self.player.statements.len() as f64 + 0.01) * 0.15;
      let distance = companion_center [0] - virtual_location;
      let direction = if distance < 0.0 {1.0} else {- 1.0} * if distance.abs() < 0.25 {-1.0} else { 1.0};
      self.companion.say (Statement {text: companion_say, start_time: now, response: None, direction: Cell::new (direction) });
    }
  }
  
  
  
  
  

  fn fraction_of_visible (&self, location: Vector3)->f64 {
    (location [1] - self.player.center [1] + self.constants.player_position)/self.constants.visible_length
  }
  fn draw_scale (&self, location: Vector3)->f64 {
    let fraction_of_visible = self.fraction_of_visible (location);
    self.constants.perspective.scale (fraction_of_visible)
  }
  fn draw_position (&self, location: Vector3)->Vector2 {
    let fraction_of_visible = self.fraction_of_visible (location);
    let horizontal_distance = location [0] - self.player.center [0];

    let scale = self.constants.perspective.scale (fraction_of_visible);
    let drop = self.constants.perspective.ground_screen_drop (fraction_of_visible);
    
    Vector2::new (
      horizontal_distance*scale,
      drop - location [2]*scale,
    )
  }
  
  fn screen_to_ground (&self, screen_coordinates: Vector2)->Vector2 {
    let fraction_of_visible = self.constants.perspective.screen_drop_to_fraction_of_visible(screen_coordinates [1]);
    let scale = self.constants.perspective.scale (fraction_of_visible);
    Vector2::new (
      screen_coordinates [0]/scale + self.player.center [0],
      (fraction_of_visible*self.constants.visible_length) + self.player.center [1] - self.constants.player_position,
    )
  }
  
  fn draw_object (&self, object: & Object, visible_radius: f64, speech_layer: bool) {
    let mut alpha = (self.player.center [1] + self.constants.spawn_distance - object.center [1])/self.constants.fadein_distance;
    if alpha < 0.0 {alpha = 0.0;}
    if alpha > 1.0 {alpha = 1.0;}
    js! {
      context.save(); 
      context.globalAlpha = @{alpha*(1.0 - object.collect_progress)};
    }
    let raw_position = Vector3::new (
      object.center [0],
      object.center [1],
      0.0,
    );
    let scale = self.draw_scale (raw_position);
    let scaled_radius = scale*object.radius;
    let position = self.draw_position (raw_position);
    match object.kind {
      Kind::Tree => {
        js! {
          var tree = tree_shape.clone({insert: false});
          tree.scale (@{scaled_radius}, [0,0]);
          tree.translate (@{position [0]}, @{position [1]});
          context.fillStyle = "rgb(70, 70, 70)";
          context.fill(new Path2D(tree.pathData));
        }
      },
      Kind::Reward => {
        js! {
          var reward = reward_shape.clone({insert: false});
          reward.rotate(@{360.0*object.collect_progress}, [0,0]);
          reward.translate (0,-reward_shape.bounds.bottom);
          reward.scale (@{scaled_radius}, [0,0]);
          reward.translate (@{position [0]}, @{position [1] - object.radius*2.0*object.collect_progress});
          var path = new Path2D(reward.pathData);
          context.fillStyle = "rgb(255, 255, 255)";
          context.strokeStyle = "rgb(0, 0, 0)";
          context.lineWidth = @{scaled_radius}*0.1;
          context.fill(path);
          context.stroke(path);
        }
      },
      Kind::Person (ref person) => {
        let mut rotation = 0.0;
        if let Some(ref fall) = object.falling {
          let (_,r) = fall.info (& self.constants, object.velocity);
          rotation = r;
        }
        let transformation1 = Rotation3::new (Vector3::new (0.0, rotation, 0.0));
        //let transformation2 = Rotation3::new (Vector3::new (0.0, 0.0, -rotation));
        let transformation = transformation1;//*transformation2;
        let transform = | vector: Vector3 | transformation*vector;
        let body_base_vector = transform(Vector3::new (0.0, 0.0, auto_constant ("body_base_height", 1.0)*object.radius));
        let body_base = raw_position + body_base_vector;
        let body_peak = body_base + transform(Vector3::new (0.0, 0.0, auto_constant ("body_height", 2.0)*object.radius));
        let body_side_vector = transform(Vector3::new (object.radius, 0.0, 0.0));
        let leg_side_vector = transform(Vector3::new (auto_constant ("leg_side", 11.0/24.0)*object.radius, 0.0, 0.0));
        let leg_inner_radius_vector = transform(Vector3::new (auto_constant ("leg_inner_radius", 8.0/24.0)*object.radius, 0.0, 0.0));
        let leg_outer_radius_vector = transform(Vector3::new (auto_constant ("leg_outer_radius", 7.0/24.0)*object.radius, 0.0, 0.0));
        let head_center = body_base + transform(Vector3::new (0.0, 0.0, auto_constant ("head_height", 1.7)*object.radius));
        let head_position = self.draw_position (head_center);
        let head_radius = auto_constant ("head_radius", 0.7)*scaled_radius;
        
        if !speech_layer {
        js! {
          context.fillStyle = "rgb(255, 255, 255)";
          context.strokeStyle = "rgb(0, 0, 0)";
          context.lineWidth = @{scaled_radius}*0.1;
        }

        let mut feet = [(-1.0, &person.feet[0]), (1.0, &person.feet[1])];
        feet.sort_by_key (| foot | OrderedFloat (-foot.1 [1]));
        for &(direction, foot) in feet.iter() {
          let foot = transform(Vector3::new (foot [0], foot [1], 0.0));
          js! { context.beginPath(); }
          move_to(self.draw_position (body_base + (leg_side_vector + leg_outer_radius_vector) * direction));
          line_to(self.draw_position (body_base + (leg_side_vector - leg_inner_radius_vector) * direction));
          line_to(self.draw_position (raw_position + leg_side_vector * direction + foot));
          js! { context.closePath(); context.fill(); context.stroke(); }
        }
        
        js! { context.beginPath(); }
        move_to(self.draw_position (body_peak));
        line_to(self.draw_position (body_base + body_side_vector));
        line_to(self.draw_position (body_base - body_side_vector));
        js! { context.closePath(); context.fill(); context.stroke(); }
        
        js! {
          context.beginPath();
          context.arc (@{head_position[0]}, @{head_position[1]}, @{head_radius}, 0, turn, true);
          context.fill(); context.stroke();
        }
        }
        else { // speech layer
        
    for statement in object.statements.iter() {
      let mut distortion = 0.0;
      let age = self.now - statement.start_time;
      let countdown = self.constants.speech_duration - age;
      let fade = self.constants.speech_fade_duration;
      if age < fade { distortion = (fade - age)/fade; }
      if countdown < fade { distortion = (countdown - fade)/fade; }
      
      let big_factor = 10000.0;
      
      js! {
        context.save();
        context.textBaseline = "middle";
        context.scale(0.0001,0.0001);
      }
      // try drawing, but sometimes we need to switch direction
      loop {
      let direction = statement.direction.get();
      let mut tail_tip_position = head_position+ Vector2::new (head_radius*auto_constant ("speech_distance_from_head", 1.4)*direction, 0.0);
      let limit = auto_constant ("speech_position_limit", 0.005);
      let distance_below_limit = -visible_radius + limit - tail_tip_position[0]*direction;
      if distance_below_limit > 0.0 {
        tail_tip_position[0] += distance_below_limit*direction;
      }
      
      let text_height = auto_constant ("text_height", 0.03) * big_factor;
      js! {
        context.font = @{text_height}+"px Arial, Helvetica, sans-serif";
      }
      let text_width: f64 = js! {
        return context.measureText (@{&statement.text}).width;
      }.try_into().unwrap();
      
      let padding = max(text_height/2.0, text_width/13.0);
      let bubble_left = -padding;
      let bubble_right = text_width + padding;
      let bubble_bottom = auto_constant ("bubble_bottom", -0.016) * big_factor;
      let text_middle = bubble_bottom - padding - text_height/2.0;
      let bubble_top = text_middle - padding - text_height/2.0;
      
      let tail_left_join_x = auto_constant ("tail_left_join_x", 0.017) * big_factor;
      let tail_right_join_x = auto_constant ("tail_right_join_x", 0.03) * big_factor;
      
      if head_position[0]*direction > 0.0 && tail_tip_position[0]*direction + bubble_right/big_factor > visible_radius {
        statement.direction.set (direction * -1.0);
        continue
      }
      
      translate (tail_tip_position*big_factor);
      js! {
        context.rotate(@{distortion*TURN/17.0});
        context.scale(@{direction}, 1);
        context.globalAlpha = @{1.0 - distortion.abs()};
        
        context.beginPath();
        
      }
      
      move_to(Vector2::new (0.0, 0.0));
      quadratic_curve (
        Vector2::new (tail_left_join_x, auto_constant ("tail_left_control_y", -0.005)),
        Vector2::new (tail_left_join_x, bubble_bottom),
      );
      quadratic_curve (
        Vector2::new (bubble_left, bubble_bottom),
        Vector2::new (bubble_left, text_middle),
      );
      quadratic_curve (
        Vector2::new (bubble_left, bubble_top),
        Vector2::new (text_width*0.5, bubble_top),
      );
      quadratic_curve (
        Vector2::new (bubble_right, bubble_top),
        Vector2::new (bubble_right, text_middle),
      );
      quadratic_curve (
        Vector2::new (bubble_right, bubble_bottom),
        Vector2::new (tail_right_join_x, bubble_bottom),
      );
      quadratic_curve (
        Vector2::new (tail_right_join_x, auto_constant ("tail_right_control_y", -0.005)),
        Vector2::new (0.0, 0.0),
      );
      js! {
        context.closePath();
        context.fillStyle = "rgb(255, 255, 255)";
        context.strokeStyle = "rgb(0, 0, 0)";
        context.lineWidth = @{auto_constant ("speech_stroke_width", 0.002)*big_factor};
        context.fill(); context.stroke();
        context.fillStyle = "rgb(0, 0, 0)";
      }
      if direction <0.0 {
        js! {
          context.scale(@{direction}, 1);
          context.translate (@{- text_width}, 0);
        }
      }
      js! {
        context.fillText (@{&statement.text}, 0, @{text_middle});
      }
        break;
      }
      js! {context.restore();}
    }
        }
      },
      _=> {
        let first_corner = self.draw_position (Vector3::new (object.center [0] - object.radius, object.center [1], object.radius));
        let second_corner = self.draw_position (Vector3::new (object.center [0] + object.radius, object.center [1], 0.0));
        let size = second_corner - first_corner;
        //println!("{:?}", (object, first_corner, second_corner, size));
        js! {
          context.fillStyle = "rgb(255,255,255)";
          context.fillRect (@{first_corner[0]}, @{first_corner[1]}, @{size[0]}, @{size[1]});
        }
      }
    };
    js! {
      context.restore(); 
    }
  }
  
  fn pain_radius (&self, pain: f64)->f64 {
    let fraction = 0.5 - pain.atan()/(TURN/2.0);
    // allow it to go a bit outside the boundaries of the screen,
    // don't allow it to reduce to a 0 size
    0.2 + fraction*0.4
  }
  
  fn draw (&self, visible_radius: f64) {
    //let (min_visible_position, max_visible_position) = self.visible_range();
    
    let temporary_pain_radius = self.pain_radius (self.temporary_pain_smoothed);
    
    let permanent_pain_speed = (self.permanent_pain_smoothed - self.permanent_pain).abs();
    if permanent_pain_speed > auto_constant ("permanent_pain_threshold", 0.0001) {
      let permanent_pain_radius = self.pain_radius (self.permanent_pain_smoothed);
      js! {
        window.permanent_pain_ellipse = new paper.Path.Ellipse ({center: [0.0, 0.5], radius: [@{permanent_pain_radius*visible_radius*2.0},@{permanent_pain_radius}], insert: false, });
        context.lineWidth = @{permanent_pain_speed*auto_constant ("permanent_pain_factor", 0.025)};
        context.strokeStyle = "rgb(255,255,255)";
        context.stroke(new Path2D(permanent_pain_ellipse.pathData));
      }
    }
    
    let no_sky: bool = js! {return auto_constants.no_sky = auto_constants.no_sky || false}.try_into().unwrap();
    if !no_sky {
    js! {
      //$(document.body).text(@{self.objects.len() as u32});
      window.visible_sky = new paper.Path.Rectangle ({point: [@{-visible_radius}, 0.0], size: [@{visible_radius*2.0},@{self.constants.perspective.horizon_drop}], insert: false, });
      
      window.temporary_pain_ellipse = new paper.Path.Ellipse ({center: [0.0, 0.5], radius: [@{temporary_pain_radius*visible_radius*2.0},@{temporary_pain_radius}], insert: false, });
      
      context.save();
      context.clip(new Path2D(temporary_pain_ellipse.pathData));
      
      window.visible_sky = window.visible_sky.intersect (temporary_pain_ellipse);
    }
    for mountain in self.mountains.iter() {
      let screen_peak = self.mountain_screen_peak (& mountain);
      let visible_base_radius = mountain.base_screen_radius*(self.constants.perspective.horizon_drop - screen_peak [1])/mountain.fake_peak_location [2];
      if screen_peak [0] + visible_base_radius < -visible_radius ||
         screen_peak [0] - visible_base_radius > visible_radius {continue;}
      js! {
        var pos = [@{screen_peak[0]}, @{screen_peak[1]}];
        var height =@{mountain.fake_peak_location [2]};
        var radius =@{mountain.base_screen_radius};
        var segments = [];
        segments.push(pos);
        segments.push([pos[0] + radius, pos[1] + height]);
        segments.push([pos[0] - radius, pos[1] + height]);
        var mountain = new paper.Path({ segments: segments, insert: false });
        mountain.closed = true;

        window.visible_sky = window.visible_sky.subtract (mountain);
      }
    }
    for sky in self.skies.iter() {
      let pos = sky.screen_position;
      js! {
        var pos = [@{pos[0]}, @{pos[1]}];
        var visible_radius = @{visible_radius};
        var steepness = @{sky.steepness};
        var segments = [];
        segments.push([
            pos,
            [-0.4, 0],
            [0.4, 0]
          ]);
        segments.push([
            [Math.max (visible_radius, pos[0]+1.0), pos[1] + steepness],
            [-0.4, 0],
            [0, 0]
          ]);
        segments.push([Math.max (visible_radius, pos[0]+1.0), pos[1] + steepness + constants.perspective.horizon_drop]);
        segments.push([Math.min (-visible_radius, pos[0]-1.0), pos[1] + steepness + constants.perspective.horizon_drop]);
        segments.push([
            [Math.min (-visible_radius, pos[0]-1.0), pos[1] + steepness],
            [0, 0],
            [0.4, 0]
          ]);
        var sky = new paper.Path({ segments: segments, insert: false });
        sky.closed = true;
        context.fillStyle = "rgba(255,255,255, 0.05)";
        context.fill(new Path2D(sky.intersect (visible_sky).pathData));
      }
    }
    }
    
    js! {
      context.beginPath();
    }
    let mut began = false;
    for component in self.path.components[0..self.path.components.len()-1].iter() {
      let mut endpoint = self.draw_position (Vector3::new (component.center [0] - self.path.radius, component.center [1], 0.0));
      
      // hack: work around a polygon display glitch that existed only in chromium, not Firefox
      if endpoint [0] < -visible_radius - 0.02 { endpoint [0] = -visible_radius - 0.02; }
      if endpoint [0] >  visible_radius + 0.01 { endpoint [0] =  visible_radius + 0.01; }
      
      if began {
        line_to (endpoint);
      }
      else {
        move_to (endpoint);
        began = true;
      }
    }
    /*{
      let last = &self.path.components[self.path.components.len()-2..self.path.components.len()];
      let distance = last [1].center - last [0].center;
      let horizon_distance = max_visible_position - last [0].center [1];
      let horizon_center = last [0].center + distance*horizon_distance/distance [1];
      let endpoint = self.draw_position (Vector3::new (horizon_center [0] - self.path.radius, max_visible_position, 0.0));
      line_to (endpoint);
      let endpoint = self.draw_position (Vector3::new (horizon_center [0] + self.path.radius, max_visible_position, 0.0));
      line_to (endpoint);
    }*/
    for component in self.path.components[0..self.path.components.len()-1].iter().rev() {
      let mut endpoint = self.draw_position (Vector3::new (component.center [0] + self.path.radius, component.center [1], 0.0));
      
      // hack: work around a polygon display glitch that existed only in chromium, not Firefox
      if endpoint [0] < -visible_radius - 0.01 { endpoint [0] = -visible_radius - 0.01; }
      if endpoint [0] >  visible_radius + 0.02 { endpoint [0] =  visible_radius + 0.02; }
      
      line_to (endpoint);
    }
    js! {
      context.fillStyle = "rgb(255,255,255)";
      context.fill();
    }
    
    if let Some(click) = self.last_click.as_ref() {
      let offset = click.location - click.player_location;
      let forward = offset/offset.norm();
      let perpendicular = Vector2::new (- forward[1], forward[0]);
      let segment_length = auto_constant ("movement_segment_length", 0.025);
      let segment_period = 2.0*segment_length;
      let segment_radius = auto_constant ("movement_segment_radius ", 0.0025);
      
      let initial_offset = -(click.distance_traveled % segment_period);
      let segments = ((offset.norm() - initial_offset)/segment_period).ceil();

      for index in 0..segments as usize {
        let first = click.player_location + forward*(initial_offset + index as f64*segment_period);
        let second = first + forward*segment_length;
        js! { context.beginPath(); }
        move_to (self.draw_position (as_ground (
          first - perpendicular*segment_radius
        )));
        line_to (self.draw_position (as_ground (
          first + perpendicular*segment_radius
        )));
        line_to (self.draw_position (as_ground (
          second + perpendicular*segment_radius
        )));
        line_to (self.draw_position (as_ground (
          second - perpendicular*segment_radius
        )));
        js! {
          context.fillStyle = "rgb(255,255,255)";
          context.fill();
        }
      }
    }
    
    let mut objects: Vec<_> = self.objects.iter().collect();
    objects.push (&self.player);
    objects.push (&self.companion);
    objects.sort_by_key (| object | OrderedFloat(-object.center [1]));
    for object in objects.iter() {self.draw_object (object, visible_radius, false) ;}
    
    js!{ context.restore();}
    
    self.draw_object (& self.player, visible_radius, true);
    self.draw_object (& self.companion, visible_radius, true);
  }
}


struct Game {
  state: State,
  last_ui_time: f64,
}


fn draw_game (game: & Game) {
  let radius: f64 = js! {
    var size = Math.min (window.innerHeight, window.innerWidth);
    canvas.setAttribute ("width", window.innerWidth);
    canvas.setAttribute ("height", window.innerHeight);
    context.clearRect (0, 0, canvas.width, canvas.height);
    context.save();
    context.scale (canvas.height,canvas.height);
    var visible_radius = canvas.width / canvas.height / 2.0;
    context.translate (visible_radius, 0);
    return visible_radius;
  }.try_into().unwrap();
  game.state.draw(radius);
  js! {
    context.restore();
  }
}

fn main_loop (time: f64, game: Rc<RefCell<Game>>) {
  {
    //js! {console.clear();}
    let mut game = game.borrow_mut();
    game.state.constants = Rc::new (js! {return window.constants;}.try_into().unwrap());
    let observed_duration = time - game.last_ui_time;
    let duration_to_simulate = min(observed_duration, 50.0)/1000.0;
    game.last_ui_time = time;
    if duration_to_simulate > 0.0 {
      game.state.simulate (duration_to_simulate);
      draw_game (& game);
    }
  }
  web::window().request_animation_frame (move | time | main_loop (time, game));
}


#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  js! {
    var game_container = window.game_container = $("<div>");
    var canvas = window.canvas = document.createElement ("canvas");
    $(document.querySelector("main") || document.body).append (game_container[0]).css("background-color", "black");
    game_container.append(canvas);
    window.context = canvas.getContext ("2d");
    window.turn = Math.PI*2;
    
    paper.setup ([640, 480]);
    
    var width_at_closest = 0.5;
    var visible_length = 2.4;
    
    window.constants = {
      visible_components: 1200,
      visible_length: visible_length,
      perspective: {
        width_at_closest: width_at_closest,
        camera_distance_along_tangent: 0.11,
        radians_visible: 0.1,
        horizon_drop: 0.36,
      },
  
      player_position: 0.16,
      player_max_speed: 0.1,
      
      spawn_radius: width_at_closest*0.5 + visible_length * 2.0,
      spawn_distance: visible_length*0.8,
      
      mountain_spawn_radius: 35.0,
      mountain_spawn_distance: 35.0,
      mountain_viewable_distances_radius: 5.0,
      mountain_density: 0.10,
  
      monster_density: 0.7,
      tree_density: 5.0,
      chest_density: 1.5,
      reward_density: 1.5,
      
      fadein_distance: visible_length*0.2,
        
      speech_fade_duration: 0.25,
      speech_duration: 3.5,
      
      fall_duration: 3.2,
    };
    window.auto_constants = {};
  }
  
  {
        let triangles = [
          [[-0.3, 0.0], [0.3, 0.0], [0.0, 2.0]],
          [[-1.0, 1.0], [1.0, 1.0], [0.0, 2.8]],
          [[-0.8, 2.0], [0.8, 2.0], [0.0, 3.5]],
        ];
        js! { tree_shape = null; }
        for triangle in triangles.iter() {
          js! { segments = []; }
          for vertex in triangle.iter() {
            js! { segments.push([@{vertex [0]},@{-vertex [1]}]); }
          }
          js! {
            var triangle = new paper.Path({ segments: segments, insert: false });
            triangle.closed = true;
            if (tree_shape) {tree_shape= tree_shape.unite (triangle);} else {tree_shape = triangle;}
          }
        }
        
        js! {
    var points = [];
    for (var index = 0; index <5;++index) {
      points.push ([
        Math.sin (turn*(index/5)),
        -Math.cos (turn*(index/5))
      ]);
      points.push ([
        Math.sin (turn*(0.1 + index/5))/Math.sqrt (5),
        -Math.cos (turn*(0.1 + index/5))/Math.sqrt (5)
      ]);
    }
    window.reward_shape = new paper.Path({ segments: points, insert: false });
    reward_shape.closed = true;
    reward_shape.scale (2.0/reward_shape.bounds.width, [0,0]);
        }
  }
  
  let mut skies = Vec::new();
  for _ in 0..15 {
    skies.push (Sky {screen_position: Vector2::new (rand::thread_rng().gen_range(-0.5, 0.5), rand::thread_rng().gen::<f64>()*0.36), steepness: rand::thread_rng().gen_range(0.1,0.2)});
  }
  
  let game = Rc::new (RefCell::new (
    Game {
      last_ui_time: 0.0,
      state: State {
        path: Path {max_speed: 1.0, radius: 0.12, components: vec![Component {center: Vector2::new (0.0, - 0.5), velocity: 0.0, acceleration: 0.0}], .. Default::default()},
        player: Object {
          center: Vector2::new (0.0, 0.0),
          radius: 0.02,
          kind: Kind::Person (Person {
            planted_foot: 0,
            feet: [Vector2::new (0.0, 0.0), Vector2::new (0.0, 0.0)],
          }),
          .. Default::default()
        },
        companion: Object {
          center: Vector2::new (0.0, -0.1),
          radius: 0.025,
          kind: Kind::Person (Person {
            planted_foot: 0,
            feet: [Vector2::new (0.0, 0.0), Vector2::new (0.0, 0.0)],
          }),
          automatic_statements: vec![
            AutomaticStatement {
              text: String::from_str ("Don't stray from the path").unwrap(),
              last_stated: None,
              distances: [0.9, 1.1],
            },
            AutomaticStatement {
              text: String::from_str ("It's dangerous out there").unwrap(),
              last_stated: None,
              distances: [2.0, 10000.0],
            },
          ],
          .. Default::default()
        },
        
        skies: skies,
  
        permanent_pain: 0.0,
        temporary_pain: 0.0,
        permanent_pain_smoothed: 0.0,
        temporary_pain_smoothed: 0.0,
  
        generator: Box::new(rand::thread_rng()),
        
        .. Default::default()
      }
    }
  ));
  
  {
    let game = game.clone();
    let mousemove_callback = move |x: f64,y: f64 | {
      let mut game = game.borrow_mut();
      let mut location = game.state.screen_to_ground (Vector2::new (x,y));
      let player_center = game.state.player.center;
      let mut offset = location - player_center;
      let limit = auto_constant ("angle_limit", TURN/6.0);
      if offset [1] < 0.0 {
        offset = Rotation2::new (-limit*2.0*x)*Vector2::new (0.0, 0.3);
      }
      if offset.norm() < game.state.player.radius {
        return;
      }
      let angle = (-offset [0]).atan2(offset[1]);
      if angle >  limit { offset = Rotation2::new ( limit - angle)*offset; }
      if angle < -limit { offset = Rotation2::new (-limit - angle)*offset; }
      location = player_center + offset;
      
      game.state.last_click = Some(Click {
        location: location,
        player_location: game.state.player.center,
        distance_traveled: game.state.distance_traveled,
        time: game.state.now,
      });
    };
    js! {
      var callback = @{mousemove_callback};
      canvas.addEventListener ("mousemove", function (event) {
        var offset = canvas.getBoundingClientRect();
        callback (
          ((event.clientX - offset.left)/offset.width-0.5)*offset.width/offset.height,
          (event.clientY - offset.top)/offset.height
        );
        event.preventDefault();
      });
    }
  }
  
  web::window().request_animation_frame (move | time | main_loop (time, game));

  stdweb::event_loop();
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this game natively");
}
