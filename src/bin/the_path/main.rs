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

use std::rc::Rc;
use std::cell::RefCell;
use std::str::FromStr;


mod draw;
mod simulation;
mod misc;
pub use draw::*;
pub use simulation::*;
pub use misc::*;


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
  
      monster_density: 0.5,
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
    
    window.chest_shape = new paper.CompoundPath({ pathData: "M 0.01270427,-0.1784 C 0.01404097,-0.2141 0.01686497,-0.2399 0.04957772,-0.2669 0.10209,-0.3054 0.10209,-0.3054 0.11586361,-0.3192 c 0.0215214,-0.0212 0.0318517,-0.0431 0.0318517,-0.0661 0,-0.0506 -0.06112074,-0.0862 -0.1489281,-0.0862 -0.08005961,0 -0.14548467,0.0357 -0.14548467,0.0805 0,0.0247 0.0206606,0.0431 0.04734714,0.0431 0.02066059,0 0.03701683,-0.0104 0.03701683,-0.0236 0,-0.006 -0.0034432,-0.0127 -0.01119158,-0.0195 -0.01463452,-0.0138 -0.0117015,-0.0118 -0.0117015,-0.0199 0,-0.0212 0.03417912,-0.0388 0.07550018,-0.0388 0.05165137,0 0.07565972,0.0238 0.07565972,0.0669 0,0.0258 -0.0051648,0.0374 -0.0439037,0.0891 -0.03185169,0.0431 -0.04151605,0.0586 -0.04339341,0.11551 -6.6452e-4,0.0202 0.03330387,0.0203 0.03406778,-6e-5 z m -0.0130559525,0.0443 c -0.0275474575,0 -0.0490688475,0.0143 -0.0490688475,0.0322 0,0.0178 0.02152139,0.0322 0.04820793,0.0322 0.02668654,0 0.04820793,-0.0143 0.04820793,-0.0322 0,-0.0173 -0.02152139,-0.0322 -0.0473470125,-0.0322 z M -0.5,-0.56743 C -0.498967,-0.79887 -0.12639936,-0.79999 -2.2774707e-4,-0.8 0.12594386,-0.80001 0.50057754,-0.798 0.49954451,-0.56752 Z m 0,0.0241 h 1 V 0 h -1 z", insert: false });
    window.chest_shape.scale(2.0, [0,0]);
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
        //offset = Rotation2::new (-limit*2.0*x)*Vector2::new (0.0, 0.3);
        offset [1] *= -1.0;
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
