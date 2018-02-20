#![recursion_limit="256"]

extern crate eliduprees_web_games;

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate derivative;
extern crate nalgebra;
extern crate rand;

use rand::Rng;
use stdweb::web;
use stdweb::unstable::TryInto;

use std::rc::Rc;
use std::cell::RefCell;

type Vector3 = nalgebra::Vector3 <f64>;
type Vector2 = nalgebra::Vector2 <f64>;

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


#[derive (Debug, Default, Deserialize)]
struct Constants {
  visible_components: i32,
  visible_length: f64,
  perspective: CylindricalPerspective,
  
  player_position: f64,
  player_max_speed: f64,
  
  spawn_radius: f64,
  spawn_distance: f64,
  
  monster_density: f64,
  tree_density: f64,
  chest_density: f64,
  reward_density: f64,
  
  speech_fade_duration: f64,
  speech_duration: f64,
}
js_deserializable! (Constants);

#[derive (Debug)]
struct Mountain {
  fake_peak_location: Vector3,
  base_screen_radius: f64,
  view_distance_range: [f64; 2],
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
  radius: f64,
  statements: Vec<Statement>,
  last_statement_start_time: f64,
  
  automatic_statements: Vec<AutomaticStatement>,
  #[derivative (Default (value = "Kind::Person"))]
  kind: Kind,
}
#[derive (Debug)]
struct Statement {
  text: String,
  start_time: f64,
  response: Option <String>,
}
#[derive (Debug)]
struct AutomaticStatement {
  text: String,
  distances: [f64; 2],
  last_stated: Option <f64>,
}
#[derive (Debug)]
enum Kind {
  Person,
  Chest,
  Reward,
  Monster (Monster),
  Tree,
}
#[derive (Debug)]
struct Monster {
  velocity: Vector2,
}

#[derive (Debug, Default)]
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
  temporary_pain: f64,
  transient_pain: f64,
  
  last_click: Option <Click>,
  
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
}



impl Object {
  fn say (&mut self, statement: Statement) {
    self.last_statement_start_time = statement.start_time;
    self.statements.push (statement);
  }
}

impl State {
  fn spawn(&mut self, advance_distance: f64, mut object: Object) {
    object.center = Vector2::new (self.player.center [0] + self.generator.gen_range (- self.constants.spawn_radius, self.constants.spawn_radius), self.player.center [1] + self.constants.spawn_distance + self.generator.gen_range(0.0, advance_distance));
    self.objects.push (object);
  }
  fn do_spawns <F: FnMut()->Object> (&mut self, advance_distance: f64, density: f64, mut object_generator: F) {
    let spawn_area = advance_distance*self.constants.spawn_radius*2.0;
    let average_number = spawn_area*density;
    let attempts = (average_number*10.0).ceil() as usize;
    for _ in 0..attempts {
      if self.generator.gen::<f64>() < average_number/attempts as f64 {
        self.spawn (advance_distance, (object_generator)());
      }
    }
  }
  fn simulate (&mut self, duration: f64) {
    self.now += duration;
    let now = self.now;
    let constants = self.constants.clone();
    for sky in self.skies.iter_mut() {
      sky.screen_position [0] += 0.05*duration*self.generator.gen_range (-1.0, 1.0);
      sky.screen_position [1] += 0.05*duration*self.generator.gen_range (-1.0, 1.0);
      sky.screen_position [0] -= (sky.screen_position [0] - 0.5)*0.0006*duration;
      sky.screen_position [1] -= (sky.screen_position [1] - 0.7*self.constants.perspective.horizon_drop)*0.0003*duration;
    }
    
    let advance_distance = constants.player_max_speed*duration;
    
    self.player.center [1] += advance_distance;
    self.companion.center [1] += advance_distance;
    
    let player_center = self.player.center;
    let min_visible_position = player_center [1] - constants.player_position;
    let max_visible_position = min_visible_position + constants.visible_length;
    
    self.mountains.retain (| mountain | {
      (mountain.fake_peak_location [1] - player_center[1]) > mountain.view_distance_range[0]
    });
    while self.path.components.last().unwrap().center [1] < max_visible_position {
      let previous = self.path.components.last().unwrap().clone();
      let distance = constants.visible_length/constants.visible_components as f64;
      let mut new = Component {
        center: previous.center + Vector2::new (distance*previous.velocity, distance),
        velocity: previous.velocity + distance*previous.acceleration,
        acceleration: previous.acceleration,
      };
      
      let default_acceleration_change_radius = self.path.max_speed*216.0*distance;
      let mut bias = - previous.velocity*36.0*distance;
      // The path secretly follows the player if the player moves too far away,
      // for both gameplay and symbolism reasons.
      let player_offset = player_center [0] - previous.center [0];
      if player_offset > 0.7 {
        bias += (player_offset - 0.7)*self.path.max_speed*40.0*distance;
      }
      if player_offset < -0.7 {
        bias += (player_offset + 0.7)*self.path.max_speed*40.0*distance;
      }
      
      let limits_1 = [
        previous.acceleration - default_acceleration_change_radius + bias,
        previous.acceleration + default_acceleration_change_radius + bias,
      ];
      // It's forbidden to accelerate to higher than max speed.
      // To keep things smooth, we never accelerate more than a fraction of the way to max speed at a time.
      // TODO: make this formula less dependent on the component size
      let limits_2 = [
        (-self.path.max_speed - previous.velocity)*200.0,
        (self.path.max_speed - previous.velocity)*200.0,
      ];
      let acceleration_limits = [
        if limits_1 [0] > limits_2 [0] {limits_1 [0]} else {limits_2 [0]},
        if limits_1 [1] < limits_2 [1] {limits_1 [1]} else {limits_2 [1]},
      ];
      
      //println!("{:?}", (limits_1, limits_2, acceleration_limits));
      if acceleration_limits[0] < acceleration_limits[1] {
        new.acceleration = self.generator.gen_range (acceleration_limits [0], acceleration_limits [1]);
      }
      else {
        new.acceleration = (acceleration_limits[0] + acceleration_limits[1]) /2.0;
      }
      
      self.path.components.push (new);
    }
    self.path.components.retain (| component | component.center [1] >= min_visible_position - constants.visible_length/constants.visible_components as f64);
    
    self.do_spawns (advance_distance, constants.tree_density, || Object {kind: Kind::Tree, radius: 0.05, .. Default::default()});
    self.do_spawns (advance_distance, constants.monster_density, || Object {kind: Kind::Monster (Monster {velocity: Vector2::new (0.0, 0.0)}), radius: 0.05, .. Default::default()});
    self.do_spawns (advance_distance, constants.chest_density, || Object {kind: Kind::Chest, radius: 0.03, .. Default::default()});
    self.do_spawns (advance_distance, constants.reward_density, || Object {kind: Kind::Reward, radius: 0.03, .. Default::default()});
    
    for object in self.objects.iter_mut() {
      match object.kind {
        Kind::Monster (ref mut monster) => {
          object.center += monster.velocity*duration;
          monster.velocity += random_vector_within_length (&mut self.generator, 0.1*duration);
          if monster.velocity.norm() > 0.1 {
            monster.velocity *= 0.5f64.powf (duration/1.0);
          }
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
    
    let mut companion_say = None;
    for statement in self.companion.automatic_statements.iter_mut() {
      if self.now > self.companion.last_statement_start_time + 5.0
          && statement.last_stated.map_or (true, | when | now > when + 100.0) {
        statement.last_stated = Some(now);
        companion_say = Some(statement.text.clone());
      }
    }
    if let Some(companion_say) = companion_say {
      self.companion.say (Statement {text: companion_say, start_time: now, response: None});
    }
  }

  fn draw_position (&self, location: Vector3)->Vector2 {
    let fraction_of_visible = (location [1] - self.player.center [1] + self.constants.player_position)/self.constants.visible_length;
    let horizontal_distance = location [0] - self.player.center [0];

    let scale = self.constants.perspective.scale (fraction_of_visible);
    let drop = self.constants.perspective.ground_screen_drop (fraction_of_visible);
    
    Vector2::new (
      0.5 + horizontal_distance*scale,
      drop - location [2]*scale,
    )
  }
  
  fn draw_object (&self, object: & Object) {
    let first_corner = self.draw_position (Vector3::new (object.center [0] - object.radius, object.center [1], object.radius));
    let second_corner = self.draw_position (Vector3::new (object.center [0] + object.radius, object.center [1], 0.0));
    let size = second_corner - first_corner;
    //println!("{:?}", (object, first_corner, second_corner, size));
    js! {
      context.fillStyle = "rgb(255,255,255)";
      context.fillRect (@{first_corner[0]}, @{first_corner[1]}, @{size[0]}, @{size[1]});
    }
  }
  
  fn draw (&self) {
    let min_visible_position = self.player.center [1] - self.constants.player_position;
    let max_visible_position = min_visible_position + self.constants.visible_length;
    
    js! {
      window.visible_sky = new paper.Path.Rectangle ({point: [0.0, 0.0], size: [1.0,@{self.constants.perspective.horizon_drop}]});
    }
    for sky in self.skies.iter() {
      let pos = sky.screen_position;
      js! {
        var pos = [@{pos[0]}, @{pos[1]}];
        var steepness = @{sky.steepness};
        var segments = [];
        segments.push([
            pos,
            [-0.4, 0],
            [0.4, 0]
          ]);
        segments.push([
            [pos[0]+1.0, pos[1] + steepness],
            [-0.4, 0],
            [0, 0]
          ]);
        segments.push([pos[0]+1.0, pos[1] + steepness + constants.perspective.horizon_drop]);
        segments.push([pos[0]-1.0, pos[1] + steepness + constants.perspective.horizon_drop]);
        segments.push([
            [pos[0]-1.0, pos[1] + steepness],
            [0, 0],
            [0.4, 0]
          ]);
        /*segments.push(new paper.Segment (
            new paper.Point (pos[0]-1.0, pos[1] + steepness),
            new paper.Point (pos[0]-1.0, pos[1] + steepness),
            new paper.Point (pos[0]-0.6, pos[1] + steepness)
          ));
        [
          ,
          ,
          ,
          ,
          ,
        ]*/
        var sky = new paper.Path(segments);
        sky.closed = true;
        context.fillStyle = "rgba(255,255,255, 0.1)";
        context.fill(new Path2D(sky.intersect (visible_sky).pathData));
      }
    }
    
    js! {
      context.beginPath();
    }
    let mut began = false;
    for component in self.path.components[0..self.path.components.len()-1].iter() {
      let endpoint = self.draw_position (Vector3::new (component.center [0] - self.path.radius, component.center [1], 0.0));
      if began {
        js! {context.lineTo(@{endpoint [0]},@{endpoint [1]});}
      }
      else {
        js! {context.moveTo(@{endpoint [0]},@{endpoint [1]});}
        began = true;
      }
    }
    {
      let last = &self.path.components[self.path.components.len()-2..self.path.components.len()];
      let distance = last [1].center - last [0].center;
      let horizon_distance = max_visible_position - last [0].center [1];
      let horizon_center = last [0].center + distance*horizon_distance/distance [1];
      let endpoint = self.draw_position (Vector3::new (horizon_center [0] - self.path.radius, max_visible_position, 0.0));
      js! {context.lineTo(@{endpoint [0]},@{endpoint [1]});}
      let endpoint = self.draw_position (Vector3::new (horizon_center [0] + self.path.radius, max_visible_position, 0.0));
      js! {context.lineTo(@{endpoint [0]},@{endpoint [1]});}
    }
    for component in self.path.components[0..self.path.components.len()-1].iter().rev() {
      let endpoint = self.draw_position (Vector3::new (component.center [0] + self.path.radius, component.center [1], 0.0));
      js! {context.lineTo(@{endpoint [0]},@{endpoint [1]});}
    }
    js! {
      context.fillStyle = "rgb(255,255,255)";
      context.fill();
    }
    
    for object in self.objects.iter() {self.draw_object (object) ;}
    self.draw_object (& self.player);
    self.draw_object (& self.companion);
  }
}


struct Game {
  state: State,
  last_ui_time: f64,
}


fn draw_game (game: & Game) {
  let canvas_width: f64 = js! {return canvas.width;}.try_into().unwrap();
  let scale = canvas_width;
  js! {
    var size = Math.min (window.innerHeight, window.innerWidth);
    canvas.setAttribute ("width", size);
    canvas.setAttribute ("height", size);
    context.clearRect (0, 0, canvas.width, canvas.height);
    context.save();
    context.scale (@{scale},@{scale});
  }
  game.state.draw();
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
    let duration_to_simulate = if observed_duration < 100.0 {observed_duration} else {100.0}/1000.0;
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
    
    paper.setup ([640, 480]);
    
    window.constants = {
      visible_components: 1200,
      visible_length: 2.0,
      perspective: {
        width_at_closest: 1.0,
        camera_distance_along_tangent: 0.11,
        radians_visible: 0.1,
        horizon_drop: 0.36,
      },
  
      player_position: 0.16,
      player_max_speed: 0.1,
      
      spawn_radius: 20.0,
      spawn_distance: 1.54,
  
      monster_density: 0.7,
      tree_density: 5.0,
      chest_density: 1.0,
      reward_density: 1.0,
  
      speech_fade_duration: 0.25,
      speech_duration: 3.5,
    };
  }
  
  let mut skies = Vec::new();
  for _ in 0..15 {
    skies.push (Sky {screen_position: Vector2::new (rand::thread_rng().gen(), rand::thread_rng().gen::<f64>()*0.36), steepness: rand::thread_rng().gen_range(0.1,0.2)});
  }
  
  let game = Rc::new (RefCell::new (
    Game {
      last_ui_time: 0.0,
      state: State {
        path: Path {max_speed: 1.0, radius: 0.12, components: vec![Component {center: Vector2::new (0.0, - 0.5), velocity: 0.0, acceleration: 0.0}], .. Default::default()},
        player: Object {center: Vector2::new (0.0, 0.0), radius: 0.02, .. Default::default()},
        companion: Object {center: Vector2::new (0.0, -0.1), radius: 0.025, .. Default::default()},
        
        skies: skies,
  
        permanent_pain: 0.4,
        temporary_pain: 0.4,
        transient_pain: 0.4,
  
        generator: Box::new(rand::thread_rng()),
        
        .. Default::default()
      }
    }
  ));
  
  {
    let game = game.clone();
    let click_callback = move |x: f64,y: f64 | {
      //let mut game = game.borrow_mut();
    };
    js! {
      var callback = @{click_callback};
      canvas.addEventListener ("click", function (event) {
        var offset = canvas.getBoundingClientRect();
        callback (
          (event.clientX - offset.left)/offset.width,
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
