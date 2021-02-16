#![recursion_limit = "256"]
#![feature(slice_patterns)]

extern crate eliduprees_web_games_lib;

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate derivative;
extern crate arrayvec;
extern crate boolinator;
extern crate lyon;
extern crate nalgebra;
extern crate ordered_float;
extern crate rand;
extern crate rand_xoshiro;

use rand::{Rng, SeedableRng};
use stdweb::unstable::TryInto;

use nalgebra::Vector2;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

mod draw;
mod misc;
mod simulation;
pub use draw::*;
pub use eliduprees_web_games_lib::*;
pub use misc::*;
pub use simulation::*;

enum MenuState {
  Playing,
}

struct Game {
  state: State,
  menu_state: MenuState,
}

fn draw_game(game: &Game) {
  js! {
    context.clearRect (0, 0, canvas.width, canvas.height);
    context.save();
  }
  game.state.draw();
  js! {
    context.restore();
  }
}

fn new_game() -> State {
  State {
    generator: Generator::from_rng(rand::thread_rng()).unwrap(),

    ..Default::default()
  }
}

#[cfg(target_os = "emscripten")]
fn main() {
  stdweb::initialize();

  js! {
    window.canvas = document.getElementById ("game_canvas");
    window.context = canvas.getContext ("2d");
    window.constants = @{Constants::default()};
  }

  let game = Rc::new(RefCell::new(Game {
    menu_state: MenuState::Playing,
    state: new_game(),
  }));

  {
    let mut game = game.borrow_mut();
    game.state.create_entity(Entity {
      is_object: true,
      is_unit: true,
      size: Vector2::new(1.2, 2.0),
      position: EntityPosition::Physical(EntityPhysicalPosition::Map {
        center: Vector2::new(3.0, 3.0),
      }),
      velocity: Vector2::new(0.1, 0.0),
      inventory: None,
    });
  }

  macro_rules! mouse_callback {
    ([$game: ident, $location: ident $($args: tt)*] $contents: expr) => {{ let game = game.clone(); move |x: f64, y: f64 $($args)*| {
      #[allow (unused_variables)]
      let $location = Vector2::new (x,y);
      let mut $game = game.borrow_mut();
      $contents
    }}}
  }

  let mousedown_callback = mouse_callback!([ game, location, button: u16 ] {
    game.state.cancel_gesture();
    if button == 0 {
      let entity = game.state.entities_at_screen_location (location).first().cloned();
      game.state.pointer_state = PointerState::PossibleClick {start: location, entity: entity};
    }
  });
  let mousemove_callback = mouse_callback!([ game, location ] {
    match game.state.pointer_state {
      PointerState::Nowhere => (),
      PointerState::PossibleClick {start, entity} => {
        let new_entity = game.state.entities_at_screen_location (location).first().cloned();
        if let Some(entity) = entity {
          if new_entity != Some(entity) {
            game.state.pointer_state = PointerState::DragEntity {entity: entity, current: location};
          }
        }
        else if entity.is_none() && (location - start).norm() > auto_constant ("drag_distance_threshold", 4.0) {
          game.state.pointer_state = PointerState::DragSelect {start: start, current: location};
        }
      },
      PointerState::DragEntity {..} => (),
      PointerState::DragSelect {..} => (),
    }
    match game.state.pointer_state {
      PointerState::DragEntity {ref mut current,..} => {
        *current = location;
      },
      PointerState::DragSelect {ref mut current,..} => {
        *current = location;
      },
      _=>()
    }
  });
  let mouseup_callback = mouse_callback!([ game, location, _button: u16 ] {
    game.state.finish_gesture();
  });
  js! {
    var mousedown_callback = @{mousedown_callback};
    var mousemove_callback = @{mousemove_callback};
    var mouseup_callback = @{mouseup_callback};
    canvas.addEventListener ("mousedown", function (event) {
      mousedown_callback (event.clientX, event.clientY, event.button) ;
    });
    canvas.addEventListener ("mousemove", function (event) {
      mousemove_callback (event.clientX, event.clientY) ;
    });
    canvas.addEventListener ("mouseup", function (event) {
      mouseup_callback (event.clientX, event.clientY, event.button) ;
    });
  }

  run(move |inputs| {
    let mut game = game.borrow_mut();
    game.state.constants = Rc::new(js! {return window.constants;}.try_into().unwrap());
    let duration_to_simulate = min(inputs.last_frame_duration, 50.0) / 1000.0;
    if duration_to_simulate > 0.0 {
      match game.menu_state {
        MenuState::Playing => {
          game.state.simulate(duration_to_simulate);
        }
      }
    }
    if inputs.resized_last_frame {
      js! {
        canvas.setAttribute ("width", window.innerWidth);
        canvas.setAttribute ("height", window.innerHeight);
      }
    }
    draw_game(&game);
  })
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this game natively");
}
