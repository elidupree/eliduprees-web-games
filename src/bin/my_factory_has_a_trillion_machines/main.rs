#![recursion_limit="256"]

extern crate eliduprees_web_games;

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate derivative;
extern crate nalgebra;
extern crate arrayvec;

mod simulation;
pub use simulation::*;
pub use eliduprees_web_games::*;



#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();

  run(move |inputs| {
  
  })
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this game natively");
}
