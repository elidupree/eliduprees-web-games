#![recursion_limit="256"]

extern crate eliduprees_web_games;

//#[macro_use]
//extern crate stdweb;
//#[macro_use]
//extern crate serde_derive;
//#[macro_use]
//extern crate derivative;
extern crate nalgebra;
extern crate arrayvec;
#[cfg (test)]
#[macro_use]
extern crate proptest;

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
  print_future (MachinesGraph::new (vec![
    (material_generator(), None, & []),
  ]));
  println!( "\n\n");
  
 print_future (MachinesGraph::new (vec![
   (material_generator(), None, & [(1, 0)]),
   (conveyor(), None, & [(2, 0)]),
   (conveyor(), None, & []),
 ]));
}
