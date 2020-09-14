//#![feature (nll)]
#![recursion_limit = "256"]

extern crate eliduprees_web_games;

#[cfg(any(target_arch = "wasm32", target_arch = "asmjs"))]
#[macro_use]
extern crate stdweb;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate derivative;
extern crate arrayvec;
extern crate nalgebra;
extern crate num;
#[cfg(test)]
#[macro_use]
extern crate proptest;
extern crate itertools;
extern crate siphasher;

#[cfg(any(target_arch = "wasm32", target_arch = "asmjs"))]
macro_rules! debug {
  ($($stuff: tt)*) => {{
    /*thread_local!{static DEBUG_LINES: std::cell::RefCell<usize> = std::cell::RefCell::new(0);}
    DEBUG_LINES.with(|lines| {
      let mut lines = lines.borrow_mut();
      *lines += 1;
      if (1f64 + (*lines) as f64/100f64).ln() as i32 > (1f64 + (*lines-1) as f64/100f64).ln() as i32 {
        println!("UHh");
        println!($($stuff)*);
      }
    });*/
    let string = format!($($stuff)*);
    js! {
      window.debug_length = window.debug_length || 0;
      if (window.debug_length < 10000) {
        window.debug_length += @{string.len() as u32};
        document.getElementById("debug").textContent += @{&string};
        console.log(@{&string});
      }
    }
  }}
}

#[cfg(not(any(target_arch = "wasm32", target_arch = "asmjs")))]
macro_rules! debug {
  ($($stuff: tt)*) => {
    eprintln!($($stuff)*)
  }
}

pub use eliduprees_web_games::*;
// hack-ish: modules marked pub to suppress dead code warnings from builds with different conditional compilation
pub mod flow_pattern;
pub mod misc;
#[macro_use]
pub mod machine_data;
pub mod geometry;
pub mod graph_algorithms;
pub mod modules;

#[cfg(any(target_arch = "wasm32", target_arch = "asmjs"))]
mod web_ui;

#[cfg(any(target_arch = "wasm32", target_arch = "asmjs"))]
fn main() {
  stdweb::initialize();
  println!("Starting emscripten build");

  web_ui::run_game();
}

#[cfg(not(any(target_arch = "wasm32", target_arch = "asmjs")))]
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

  let game: machine_data::Game = serde_json::from_reader(std::io::BufReader::new(
    std::fs::File::open("../data/test.json").unwrap(),
  ))
  .unwrap();
  let output_edges = game.map.output_edges(&game.machine_types);
  let ordering = game
    .map
    .topological_ordering_of_noncyclic_machines(&output_edges);
  let future = game.map.future(
    &game.machine_types,
    &output_edges,
    &ordering,
    &mut graph_algorithms::ModuleFutures::default(),
    &[],
  );
  println!("{:?}", future);
}
