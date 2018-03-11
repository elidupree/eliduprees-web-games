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
extern crate lyon;
extern crate arrayvec;

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


enum MenuState {
  Shown,
  Hidden,
  Appearing (f64),
  Disappearing (f64),
}

struct Game {
  state: State,
  last_ui_time: f64,
  menu_state: MenuState,
}


fn draw_game (game: & Game) {
  let radius: f64 = js! {
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
    let menu_game_opacity = 0.4;
    if duration_to_simulate > 0.0 { match game.menu_state {
      MenuState::Hidden => {
        js! {
          menu.css({display: "none"});
          $(canvas).css({opacity: @{1.0}});
        }
        game.state.simulate (duration_to_simulate);
        draw_game (& game);
      },
      MenuState::Appearing (progress) => {
        js! {
          menu.css({display: "block", opacity: @{progress}});
          $(canvas).css({opacity: @{menu_game_opacity + (1.0 - menu_game_opacity)*(1.0 - progress)}});
        }
        let new_progress = progress + duration_to_simulate;
        game.menu_state = if new_progress > 1.0 {MenuState::Shown} else {MenuState::Appearing (new_progress)};
      },
      MenuState::Disappearing (progress) => {
        js! {
          menu.css({display: "block", opacity: @{1.0 - progress}});
          $(canvas).css({opacity: @{menu_game_opacity + (1.0 - menu_game_opacity)*progress}});
        }
        let new_progress = progress + duration_to_simulate;
        game.menu_state = if new_progress > 1.0 {MenuState::Hidden} else {MenuState::Disappearing (new_progress)};
      },
      MenuState::Shown => {
        js! {
          menu.css({display: "block", opacity: 1.0});
          $(canvas).css({opacity: @{menu_game_opacity} });
        }
      },
    }}
  }
  web::window().request_animation_frame (move | time | main_loop (time, game));
}


#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  
  
  
  let mut skies = Vec::new();
  for _ in 0..15 {
    skies.push (Sky {screen_position: Vector2::new (rand::thread_rng().gen_range(-0.5, 0.5), rand::thread_rng().gen::<f64>()*0.36), steepness: rand::thread_rng().gen_range(0.1,0.2)});
  }
  
  let game = Rc::new (RefCell::new (
    Game {
      last_ui_time: 0.0,
      menu_state: MenuState::Shown,
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
  
  js! { window.orientation_hack = function() {
var fullscreen = (
    document.body.requestFullScreen ? { request: "requestFullScreen", change: "fullscreenchange" } :
    document.body.webkitRequestFullScreen ? { request: "webkitRequestFullScreen", change: "webkitfullscreenchange" } :
    document.body.mozRequestFullScreen ? { request: "mozRequestFullScreen", change: "mozfullscreenchange" } :
    document.body.msRequestFullScreen ? { request: "msRequestFullScreen", change: "MSFullscreenChange" } :
    null);
if (window.innerHeight > window.innerWidth && window.screen.height > window.screen.width &&
  fullscreen && window.screen.orientation && window.screen.orientation.lock) {
  var handler = function() {
    document.removeEventListener (fullscreen.change, handler);
    window.screen.orientation.lock ("landscape");
  };
  document.addEventListener (fullscreen.change, handler);
  document.body [fullscreen.request] ();
}
  };}
  
  let start_playing_callback = {
    let game = game.clone();
    move | | {
      let mut game = game.borrow_mut();
      if let MenuState::Shown = game.menu_state {
        game.menu_state = MenuState::Disappearing (0.0);
      }
    }
  };

  js! {
    window.game_container = $("#game_container");
    window.canvas = document.getElementById ("game_canvas");
    window.context = canvas.getContext ("2d");
    window.turn = Math.PI*2;
    
    paper.setup ([640, 480]);
  }
  js! {
    window.menu = $("#menu");
    window.constants = @{Constants::default()};
    window.auto_constants = {};
  }
  js! {
    var start_playing_callback = @{start_playing_callback};
    window.content_warnings = $("#content_warnings").text ("Show content warnings").click (function() {
      content_warnings.text ("Content warning: a voice victim-blames you for stuff").removeClass("clickable").css({color: "white"}).css({color: "black", transition: "color 0.6s"});
    });
    $("#start_playing").css({"font-size": "150%", "font-weight": "bold"}).click (function() {orientation_hack(); start_playing_callback();});
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
    
    window.chest_shape = new paper.CompoundPath({ pathData: "m -6.093328e-4,-0.1882 c -0.0357946672,0 -0.0637591772,0.0186 -0.0637591772,0.0418 0,0.0232 0.02796451,0.0418 0.06264051,0.0418 0.03467602,0 0.06264052,-0.0185 0.06264052,-0.0418 0,-0.0225 -0.0279645,-0.0418 -0.0615218528,-0.0418 z M 0.01635534,-0.2729 c 0.0017368,-0.0463 0.0054063,-0.0799 0.04791269,-0.115 0.0682335,-0.05 0.0682335,-0.05 0.08613068,-0.068 0.0279646,-0.0276 0.0413875,-0.056 0.0413875,-0.0859 0,-0.0657 -0.0794191,-0.112 -0.19351448,-0.112 -0.10402801,0 -0.18904015,0.0463 -0.18904015,0.10462 0,0.0321 0.0268461,0.056 0.061522,0.056 0.026846,0 0.048099,-0.0135 0.048099,-0.0307 0,-0.008 -0.0044741,-0.0165 -0.01454213,-0.0253 -0.01901583,-0.0179 -0.01520472,-0.0153 -0.01520472,-0.0259 0,-0.0276 0.04441173,-0.0504 0.09810356,-0.0504 0.06711485,0 0.09831086,0.0309 0.09831086,0.087 0,0.0335 -0.0067111,0.0486 -0.05704766,0.11577 -0.04138751,0.056 -0.05394521,0.0762 -0.05638462,0.1501 -8.6346e-4,0.0263 0.04327445,0.0264 0.04426706,-9e-5 z M 0.5,0 h -1 V -0.55538 C -0.498967,-0.79887 -0.12639936,-0.79999 -2.2774707e-4,-0.8 0.12594386,-0.80001 0.50057754,-0.798 0.49977226,-0.55542 Z", insert: false });
    window.chest_shape.scale(2.0, [0,0]);
        }
  }
  
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
      var handle_thing = function (thing) {
        var offset = canvas.getBoundingClientRect();
        callback (
          ((thing.clientX - offset.left)/offset.width-0.5)*offset.width/offset.height,
          (thing.clientY - offset.top)/offset.height
        );
      };
      canvas.addEventListener ("mousemove", function (event) {
        handle_thing (event);
      });
      var touch_callback = function (event) {
        var touches = event.changedTouches;
        for (var index = 0; index < touches.length; ++index) {
          handle_thing (touches [index]);
        }
      };
      canvas.addEventListener ("touchstart", touch_callback);
      canvas.addEventListener ("touchmove", touch_callback);
      canvas.addEventListener ("contextmenu", function(event) {event.preventDefault();});
    }
  }
  
  {
    let game = game.clone();
    let update_dimensions_callback = move || {
      js! {
        canvas.setAttribute ("width", window.innerWidth);
        canvas.setAttribute ("height", window.innerHeight);
        var size = 4;
        do {
          menu.css({ "font-size": (size/2)+"em" }).css({ "font-size": size+"vh" });
          size *= 0.9;
        } while (size > 0.5 && menu.height() > window.innerHeight);
      }
      draw_game (& game.borrow());
    };
    js! {
      window.update_dimensions = @{update_dimensions_callback};
      window.resizes = 0;
      $(window).resize (function() {window.resizes++; update_dimensions();});
    }
  }
  
  {
    let mut game = game.borrow_mut();
    game.state.simulate (0.0001);
  }
  js!{update_dimensions();}
  
  web::window().request_animation_frame (move | time | main_loop (time, game));

  stdweb::event_loop();
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this game natively");
}
