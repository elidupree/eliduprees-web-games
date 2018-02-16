#![recursion_limit="128"]

extern crate eliduprees_web_games;

#[macro_use]
extern crate stdweb;
extern crate nalgebra;
extern crate rand;

use rand::Rng;

use std::rc::Rc;

type Vector3 = nalgebra::Vector3 <f64>;
type Vector2 = nalgebra::Vector2 <f64>;


struct Constants {
  visible_components: i32,
  visible_length: f64,
  
  player_position: f64,
  player_max_speed: f64,
  
  monster_density: f64,
  tree_density: f64,
  test_density: f64,
  reward_density: f64,
  
  speech_fade_duration: f64,
  speech_duration: f64,
}

struct Mountain {
  fake_peak_location: Vector3,
  base_screen_radius: f64,
  view_distance_range: [f64; 2],
}
struct Sky {
  screen_position: Vector2,
  steepness: f64,
}
struct Object {
  center: Vector2,
  radius: f64,
  statements: Vec<Statement>,
  last_statement_start_time: f64,
  
  automatic_statements: Vec<AutomaticStatement>,
  kind: Kind,
}
struct Statement {
  text: String,
  start_time: f64,
  response: Option <String>,
}
struct AutomaticStatement {
  text: String,
  distances: [f64; 2],
  last_stated: Option <f64>,
}
enum Kind {
  Person,
  Chest,
  Reward,
  Monster,
}

struct Path {
  components: Vec<Component>,
  max_speed: f64,
}
struct Component {
  position: f64,
  velocity: f64,
  acceleration: f64,
}
struct Click {
  location: Vector2,
  player_location: Vector2,
  time: f64,
}

struct State {
  mountains: Vec<Mountain>,
  skies: Vec<Sky>,
  objects: Vec<Object>,
  path: Path,
  player: Object,
  companion: Object,
  
  permanent_pain: f64,
  temporary_pain: f64,
  transient_pain: f64,
  
  last_click: Option <Click>,
  
  stars_collected: i32,
  
  generator: Box <Rng>,
  constants: Rc<Constants>,
  now: f64,
}

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
}



impl Object {
  fn say (&mut self, statement: Statement) {
    self.last_statement_start_time = statement.start_time;
    self.statements.push (statement);
  }
}

impl State {
  fn simulate (&mut self, duration: f64) {
    self.now += duration;
    let now = self.now;
    let constants = self.constants.clone();
    for sky in self.skies.iter_mut() {
      sky.screen_position [0] += 0.05*duration*self.generator.gen_range (-1.0, 1.0);
      sky.screen_position [1] += 0.05*duration*self.generator.gen_range (-1.0, 1.0);
      sky.screen_position [0] -= (sky.screen_position [0] - 0.7)*0.0003*duration;
      sky.screen_position [1] -= (sky.screen_position [1] - 0.5)*0.0006*duration;
    }
    
    self.player.center [1] += constants.player_max_speed;
    
    let player_center = self.player.center;
    
    self.mountains.retain (| mountain | {
      (mountain.fake_peak_location [1] - player_center[1]) > mountain.view_distance_range[0]
    });
    for object in self.objects.iter_mut() {
      match object.kind {
        Kind::Monster => {
        
        },
        _=>(),
      };
      object.statements.retain (| statement | statement.start_time + constants.speech_duration > now);
    }
    self.objects.retain (| object | {
      object.center [1] > player_center[1] - 0.5
    });
    
    self.temporary_pain = self.permanent_pain + (self.temporary_pain - self.permanent_pain) * 0.5f64.powf(duration/1.4);
    self.transient_pain = self.temporary_pain + (self.transient_pain - self.temporary_pain) * 0.5f64.powf(duration/0.03);
    
    for statement in self.companion.automatic_statements.iter_mut() {
      if self.now > self.companion.last_statement_start_time + 5.0
          && statement.last_stated.map_or (true, | when | now > when + 100.0) {
        statement.last_stated = Some(now);
        self.companion.say (Statement {text: statement.text.clone(), start_time: now, response: None});
      }
    }
  }

  fn draw_position (&self, location: Vector3)->Vector2 {
    let perspective = CylindricalPerspective {
      width_at_closest: 1.0,
      camera_distance_along_tangent: 0.11,
      radians_visible: 0.1,
      horizon_drop: 0.36,
    };
    let fraction_of_visible = (location [1] - self.player.center [1] - self.constants.player_position)/self.constants.visible_length;
    let horizontal_distance = location [0] - self.player.center [0];

    let scale = perspective.scale (fraction_of_visible);
    let drop = perspective.ground_screen_drop (fraction_of_visible);
    
    Vector2::new (
      0.5 + horizontal_distance*scale,
      drop - location [2]*scale,
    )
  }
  
  fn draw (&self) {
  
  }
}

fn main() {
  println!("not yet implemented :-P");
}
