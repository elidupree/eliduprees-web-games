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
pub use eliduprees_web_games::*;


enum MenuState {
  Playing,
}

struct Game {
  state: State,
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
  game.state.draw();
  js! {
    context.restore();
  }
}


fn new_game()->State {
  State {
  
    generator: Box::new(rand::thread_rng()),
        
    .. Default::default()
  }
}

#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();

  js! {
    window.canvas = document.getElementById ("game_canvas");
    window.context = canvas.getContext ("2d");
    window.constants = @{Constants::default()};
  }
    
  let game = Rc::new (RefCell::new (
    Game {
      menu_state: MenuState::Playing,
      state: new_game(),
    }
  ));
  
  run(move |inputs| {
    let mut game = game.borrow_mut();
    game.state.constants = Rc::new (js! {return window.constants;}.try_into().unwrap());
    let duration_to_simulate = min(inputs.last_frame_duration, 50.0)/1000.0;
    if duration_to_simulate > 0.0 {
      match game.menu_state {
        MenuState::Playing => {
          game.state.simulate (duration_to_simulate);
        },
      }
    }
    if inputs.resized_last_frame {
      js! {
        canvas.setAttribute ("width", window.innerWidth);
        canvas.setAttribute ("height", window.innerHeight);
      }
    }
    draw_game (& game);
  })
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this game natively");
}
