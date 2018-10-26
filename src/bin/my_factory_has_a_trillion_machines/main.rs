#![recursion_limit="256"]

extern crate eliduprees_web_games;

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
//#[macro_use]
//extern crate derivative;
extern crate num;
extern crate nalgebra;
extern crate arrayvec;
#[cfg (test)]
#[macro_use]
extern crate proptest;
#[macro_use]
extern crate glium;

mod simulation;
pub use simulation::*;
pub use eliduprees_web_games::*;


#[cfg (target_os = "emscripten")]
mod web_ui;

#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();

  web_ui::run_game();
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
   (conveyor(), None, & [(3, 0)]),
   (conveyor(), None, & [(4, 0)]),
   (conveyor(), None, & [(5, 0)]),
   (splitter(), None, & [(6, 0), (10, 0)]),
   (splitter(), None, & [(7, 0), (8, 0)]),
   (slow_machine(), None, & [(9, 0)]),
   (slow_machine(), None, & [(9, 1)]),
   (merger(), None, & [(10, 1)]),
   (merger(), None, & [(11, 0)]),
   (consumer(), None, & []),
 ]));
}
