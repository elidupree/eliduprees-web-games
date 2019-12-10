use super::*;

//use std::cmp::{min, max};
use std::collections::HashMap;

use arrayvec::ArrayVec;

use geometry::{Number};
use flow_pattern::{MaterialFlow};
use machine_data::{Inputs, Material, MachineTypeTrait, Map, Game, InputLocation, MachineObservedInputs, MAX_COMPONENTS};

pub type OutputEdges = ArrayVec<[Inputs<Option<(usize, usize)>>; MAX_COMPONENTS]>;
pub struct MapFuture {
  pub machines: Vec<MachineFuture>,
  pub dumped: Vec<(InputLocation, MaterialFlow)>,
}

#[derive (Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct MachineFuture {
  pub inputs: Inputs <Option <MaterialFlow>>,
}



impl Map {
  pub fn output_edges (&self)->OutputEdges {
    self.machines.iter().map (| machine | {
      machine.output_locations().map (| output_location | {
        self.machines.iter().enumerate().find_map(| (machine2_index, machine2) | {
          machine2.input_locations().enumerate().find_map(| (input_index, input_location) | {
            if input_location == output_location {
              Some((machine2_index, input_index))
            }
            else {
              None
            }
          })
        })
      }).collect()
    }).collect()
  }
  
  pub fn topological_ordering_of_noncyclic_machines (&self, output_edges: & OutputEdges)->Vec<usize> {
    let mut num_inputs: ArrayVec<[usize; MAX_COMPONENTS]> = self.machines.iter().map (|_| 0).collect();
    let mut result = Vec::with_capacity(MAX_COMPONENTS);
    let mut starting_points = Vec::with_capacity(MAX_COMPONENTS);
    for machine in output_edges {
      for output in machine {
        if let Some(output) = output {
          num_inputs[output.0] += 1
        }
      }
    }
    
    for (index, inputs) in num_inputs.iter().enumerate() {
      if *inputs == 0 {
        starting_points.push (index);
      }
    }
    
    while let Some (starting_point) = starting_points.pop() {
      result.push (starting_point);
      for destination in & output_edges [starting_point] {
        if let Some((machine, _input)) = *destination {
          num_inputs [machine] -= 1;
          if num_inputs [machine] == 0 {
            starting_points.push (machine);
          }
        }
      }
    }
    result
  }
  
  pub fn future (&self, output_edges: & OutputEdges, topological_ordering: & [usize])->MapFuture {
    let mut result = MapFuture {
      machines: self.machines.iter().map (| machine | {
        MachineFuture::default()
      }).collect(),
      dumped: Default::default(),
    };
    
    for &machine_index in topological_ordering {
      let machine = & self.machines [machine_index];
      let inputs = MachineObservedInputs {
        input_flows: & result.machines [machine_index].inputs,
        start_time: machine.state.last_disturbed_time,
      };
      let outputs = machine.machine_type.output_flows (inputs);
      for ((flow, destination), location) in outputs.into_iter().zip (& output_edges [machine_index]).zip (machine.output_locations()) {
        match destination {
          None => if let Some(flow) = flow {result.dumped.push ((location, flow))},
          Some ((destination_machine, destination_input)) => result.machines [*destination_machine].inputs [*destination_input] = flow,
        }
      }
    }

    result
  }
}

impl Game {
  pub fn inventory_at (&self, future: & MapFuture, time: Number)->HashMap <Material, Number> {
    let mut inventory = self.inventory_before_last_change.clone();
    let interval = [self.map.last_change_time, time];
    for (location, material_flow) in &future.dumped {
      *inventory.entry (material_flow.material).or_default() += material_flow.flow.num_disbursed_between (interval);
    }
    inventory
  }
}


