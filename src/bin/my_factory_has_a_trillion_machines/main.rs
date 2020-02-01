//#![feature (nll)]
#![recursion_limit="256"]

extern crate eliduprees_web_games;

#[cfg (target_os = "emscripten")]
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate derivative;
extern crate num;
extern crate nalgebra;
extern crate arrayvec;
#[cfg (test)]
#[macro_use]
extern crate proptest;
extern crate siphasher;
extern crate itertools;

pub use eliduprees_web_games::*;
// hack-ish: modules marked pub to suppress dead code warnings from builds with different conditional compilation
pub mod misc;
pub mod flow_pattern;
#[macro_use]
pub mod machine_data;
pub mod geometry;
//pub mod modules;
pub mod graph_algorithms;

#[cfg (target_os = "emscripten")]
mod web_ui;

#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  println!( "Starting emscripten build");
  
  web_ui::run_game();
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  //println!( "Non-emscripten builds don't do anything right now");
  /*MachinesGraph::new (vec![
    (material_generator(), None, & []),
  ]).simulate_future();
  println!( "\n\n");
  
 MachinesGraph::new (vec![
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
 ]).simulate_future();*/
 
 let game: machine_data::Game = serde_json::from_reader (std::io::BufReader::new(std::fs::File::open ("../data/test.json").unwrap())).unwrap();
 let output_edges = game.map.output_edges(& game.machine_types) ;
 let ordering = game.map.topological_ordering_of_noncyclic_machines (& output_edges);
 let future = game.map.future (& game.machine_types, & output_edges, & ordering);
 println!("{:?}", future);
}
