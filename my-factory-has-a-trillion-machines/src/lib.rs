#![feature(min_type_alias_impl_trait, iter_partition_in_place)]
#![recursion_limit = "256"]

#[cfg(test)]
#[macro_use]
extern crate proptest;

#[macro_use]
pub mod machine_data;
#[macro_use]
pub mod graph_algorithms;
// hack-ish: modules marked pub to suppress dead code warnings from builds with different conditional compilation
pub mod flow_pattern;
pub mod geometry;
pub mod misc;
pub mod modules;
pub mod primitive_machines;
pub mod ui;
pub mod undo_history;

/*#[cfg(not(any(target_arch = "wasm32", target_arch = "asmjs")))]
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

  /*let game: machine_data::Game = serde_json::from_reader(std::io::BufReader::new(
    std::fs::File::open("../data/test.json").unwrap(),
  ))
  .unwrap();
  let output_edges = game.global_region.output_edges(&game.machine_types);
  let ordering = game
    .global_region
    .topological_ordering_of_noncyclic_machines(&output_edges);
  let future = game.global_region.future(
    &game.machine_types,
    &output_edges,
    &ordering,
    &mut graph_algorithms::ModuleFutures::default(),
    &[],
  );
  println!("{:?}", future);*/
}*/
