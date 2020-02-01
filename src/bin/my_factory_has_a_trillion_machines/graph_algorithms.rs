//use std::cmp::{min, max};
use std::collections::HashMap;

use arrayvec::ArrayVec;

use geometry::{Number};
use flow_pattern::{MaterialFlow, FlowCollection};
use machine_data::{Inputs, Material, Map, Game, InputLocation, MachineObservedInputs, MachineTypes, StatefulMachine, MAX_COMPONENTS};

pub type OutputEdges = ArrayVec<[Inputs<Option<(usize, usize)>>; MAX_COMPONENTS]>;
#[derive (Debug)]
pub struct MapFuture {
  pub machines: Vec<MachineFuture>,
  pub dumped: Vec<(InputLocation, MaterialFlow)>,
}

#[derive (Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct MachineFuture {
  pub inputs: Inputs <Option <MaterialFlow>>,
}



impl Map {
  pub fn output_edges (&self, machine_types: &MachineTypes)->OutputEdges {
    self.machines.iter().map (| machine | {
      machine_types.output_locations(machine).map (| output_location | {
        self.machines.iter().enumerate().find_map(| (machine2_index, machine2) | {
          machine_types.input_locations(machine2).enumerate().find_map(| (input_index, input_location) | {
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
  
  pub fn build_machines (&mut self, machine_types: &mut MachineTypes, machines: impl IntoIterator <Item = StatefulMachine>, now: Number) {
    let old_length = self.machines.len();
    self.machines.extend (machines);
    let mut disturbed = Vec::with_capacity (self.machines.len());
    disturbed.extend (old_length..self.machines.len());
    self.disturb_downstream (machine_types, &self.output_edges (machine_types), disturbed, now);
  }
  
  pub fn remove_machines (&mut self, machine_types: &mut MachineTypes, machines: Vec<usize>, now: Number) {
    let mut disturbed = Vec::with_capacity (self.machines.len());
    disturbed.extend_from_slice (& machines);
    self.disturb_downstream (machine_types, & self.output_edges (machine_types), disturbed, now);
    let mut index = 0;
    self.machines.retain (| machine | {
      let result = !machines.contains (& index);
      index += 1;
      result
    });
  }
  
  pub fn modify_machines (&mut self, machine_types: &mut MachineTypes, machines: Vec<usize>, now: Number, mut modify: impl FnMut (&mut StatefulMachine)) {
    let mut disturbed = Vec::with_capacity (self.machines.len());
    disturbed.extend_from_slice (& machines);
    self.disturb_downstream (machine_types, & self.output_edges (machine_types), disturbed, now);
    for (index, machine) in self.machines.iter_mut().enumerate() {
      if machines.contains (& index) {
        (modify) (machine);
      }
    }
  }
  
  pub fn disturb_downstream (&mut self, machine_types: &mut MachineTypes, output_edges: & OutputEdges, starting_points: Vec<usize>, now: Number) {
    let mut stack = starting_points;
    let mut visited: Vec<bool> = vec![false; self.machines.len()];
    while let Some (index) = stack.pop() {
      let machine = &mut self.machines [index];
      machine.state.last_disturbed_time = now;
      /*if let MachineType::ModuleMachine (machine) = machine_types.get (machine.type_id) {
        self.canonicalize_module (machine_types, machine);
      }*/
      for &(destination_machine_index,_) in output_edges [index].iter().flatten() {
        if !visited [destination_machine_index] {
          visited [destination_machine_index] = true;
          stack.push (destination_machine_index);
        }
      }
    }
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
  
  pub fn future (&self, machine_types: &MachineTypes, output_edges: & OutputEdges, topological_ordering: & [usize])->MapFuture {
    let mut result = MapFuture {
      machines: self.machines.iter().map (|machine| MachineFuture{
        inputs: (0..machine_types.get(machine.type_id).num_inputs()).map(|_| None).collect(),
      }).collect(),
      dumped: Default::default(),
    };
    
    for &machine_index in topological_ordering {
      let machine = & self.machines [machine_index];
      let inputs = MachineObservedInputs {
        input_flows: & result.machines [machine_index].inputs,
        start_time: machine.state.last_disturbed_time,
      };
      let machine_type = machine_types.get(machine.type_id);
      let outputs = machine_type.output_flows (inputs);
      //println!("{:?}\n{:?}\n{:?}\n\n", machine, inputs , outputs);
      for ((flow, destination), location) in outputs.into_iter().zip (& output_edges [machine_index]).zip (machine_type.output_locations(machine.state.position)) {
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
    let interval = [self.last_change_time, time];
    for (_location, material_flow) in &future.dumped {
      *inventory.entry (material_flow.material).or_default() += material_flow.flow.num_disbursed_between (interval);
    }
    inventory
  }
}


