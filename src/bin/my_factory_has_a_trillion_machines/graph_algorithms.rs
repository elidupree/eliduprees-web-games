use super::*;

use std::iter;
use std::cmp::{min, max};
use std::collections::HashMap;

use arrayvec::ArrayVec;

use geometry::{Number};
use flow_pattern::FlowPattern;
use machine_data::{Inputs, Material, MachineTypeTrait, MachineMaterialsState};
use map::{Map, Game, MAX_COMPONENTS};

pub type OutputEdges = ArrayVec<[Inputs<Option<(usize, usize)>>; MAX_COMPONENTS]>;
#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub struct MapFuture {
  pub machines: Vec<MachineFuture>,
  pub inventory: HashMap <Material, InventoryFuture>
}


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct MachineFuture {
  pub changes: Vec<(Number, MachineMaterialsState)>,
  pub inputs: Inputs <MachineInputFuture>,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct MachineInputFuture {
  pub changes: Vec<(Number, (FlowPattern, Material))>,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct InventoryFuture {
  pub flows: Vec<([Number; 2], FlowPattern)>,
}



impl MachineFuture {
  fn push_change (&mut self, change: (Number, MachineMaterialsState)) {
    if self.changes.last().map_or (false, | (time,_state) | *time == change.0) {
      self.changes.pop();
    }
    self.changes.push (change);
  }
}

impl Map {
  pub fn output_edges (&self)->OutputEdges {
    self.machines.iter().map (| machine | {
      machine.machine_type.output_locations(&machine.map_state).into_iter().map (| output_location | {
        self.machines.iter().enumerate().find_map(| (machine2_index, machine2) | {
          machine2.machine_type.input_locations(& machine2.map_state).into_iter().enumerate().find_map(| (input_index, input_location) | {
            if (input_location.0, Some(input_location.1)) == output_location {
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
        MachineFuture {
          changes: Default::default(),
          inputs: (0..machine.machine_type.num_inputs()).map (|_| Default::default()).collect()
        }
      }).collect(),
      inventory: Default::default(),
    };
    
    for &machine_index in topological_ordering {
      let machine = & self.machines [machine_index];
      let mut state = machine.materials_state.clone();
      let mut input_patterns: Inputs <_> = result.machines [machine_index].inputs.iter().map (| _input | Default::default()).collect();
      let mut outputs: Inputs <_> = iter::repeat (MachineInputFuture::default()).take(machine.machine_type.num_outputs()).collect();
      let mut last_change_time = machine.materials_state.last_flow_change-1;
      let mut total_changes = 0;
      loop {
        total_changes += total_changes;
        assert!(total_changes < 100, "a machine probably entered an infinite loop");
        let next_change_time = result.machines [machine_index].inputs.iter().filter_map (| input |
          input.changes.iter().map (| (time,_pattern) | *time).find (| &time | time > last_change_time)
        ).min().unwrap_or_else (Number::max_value);
        
        let future_output = machine.machine_type.future_output_patterns (& state, & input_patterns);
        
        for (delivered_output, output_future) in outputs.iter_mut().zip(future_output) {
          for (when, pattern) in output_future {
            if when < next_change_time {
              if pattern != delivered_output.changes.last().map_or_else (Default::default, | change | change.1) {
                delivered_output.changes.push ((when, pattern));
              }
            }
          }
        }
        
        if next_change_time == Number::max_value() { break }
        
        //eprintln!(" {:?} ", (next_change_time, last_change_time, &personal_change)) ;
        assert!(next_change_time > last_change_time);
        for (input_index, (_time, pattern)) in result.machines [machine_index].inputs.clone().iter().enumerate().filter_map (
              | (input_index, input) | input.changes.iter().find (| (time,_pattern) | *time == next_change_time).map (| whatever | (input_index, whatever))
            ) {
          state = machine.machine_type.with_inputs_changed (&state, next_change_time, & input_patterns);
          assert!(state.last_flow_change == next_change_time);
          result.machines [machine_index].push_change ((next_change_time, state.clone()));
          input_patterns [input_index] = *pattern;
        }
        
        last_change_time = next_change_time;
      }
      for ((output, destination), (_location, facing)) in outputs.into_iter().zip (output_edges [machine_index].iter()).zip (machine.machine_type.output_locations (&machine.map_state)) {
        if let Some ((destination_machine, destination_input)) = *destination {
          result.machines [destination_machine].inputs [destination_input] = output;
        }
        else if facing.is_none() {
          for ((start_time, (pattern, material)), next) in misc::with_optional_next (output.changes.iter()) {
            result.inventory.entry(*material).or_default().flows.push (([*start_time, next.map_or_else (Number::max_value, | next | next.0)], *pattern));
          }
        }
        //else {
          //println!("Machine {} outputted {:?}", machine_index, output);
        //}
      }
    }

    result
  }
  
  /*pub fn update_to (&mut self, future: & MapFuture, time: Number) {
    for (machine, future) in self.machines.iter_mut().zip (future.machines.iter()) {
      machine.materials_state = future.materials_state_at (time, & machine.materials_state);
    }
  }*/
  
  
}

impl Game {
  pub fn inventory_at (&self, future: & MapFuture, time: Number)->HashMap <Material, Number> {
    let mut inventory = self.inventory_before_last_change.clone();
    let interval = [self.map.last_change_time, time];
    for (material, future) in &future.inventory {
      *inventory.entry (*material).or_default() += future.accumulation_between (interval);
    }
    inventory
  }
}

impl MachineFuture {
  pub fn inputs_at (&self, time: Number)->Inputs <(FlowPattern, Material)> {
    self.inputs.iter().map (| future | future.changes.iter().rev().find (| (change_time,_) | *change_time <= time).map_or_else (Default::default, | (_, pattern) | *pattern)).collect()
  }
  pub fn materials_state_at (&self, time: Number, initial_state: & MachineMaterialsState)->MachineMaterialsState {
    self.changes.iter().rev().find (| (change_time,_) | *change_time <= time).map_or_else (| | initial_state.clone(), | (_change_time, state) | state.clone())
  }
}

impl InventoryFuture {
  pub fn accumulation_between(&self, interval: [Number; 2])->Number {
    self.flows.iter().map(| (pattern_interval, pattern) | {
      let final_interval = [max (interval [0], pattern_interval [0]), min (interval [1], pattern_interval [1])];
      if final_interval[0] > final_interval [1] {return 0;}
      pattern.num_disbursed_between (final_interval)
    }).sum()
  }
}

#[derive (Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct MachinesGraphInput {
  pub initial_value: FlowPattern,
  pub changes: Vec<(Number, FlowPattern)>,
}

/*
#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct MachinesGraphNode {
  pub machine: StandardMachine,
  pub original_index: usize,
  pub initial_state: MachineMaterialsState,
  pub inputs: Inputs <MachinesGraphInput>,
  pub output_locations: Inputs <Option <(usize, usize)>>
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct MachinesGraph {
  pub nodes: Vec<MachinesGraphNode>,
}

impl MachinesGraph {
  #[allow (clippy::type_complexity)]
  pub fn new (data: Vec<(StandardMachine, Option <MachineMaterialsState>, &[(i64, i64)])>)->MachinesGraph {
    MachinesGraph {nodes: data.into_iter().map (| (machine, initial_state, outputs) | {
      let inputs: Inputs <MachinesGraphInput> = machine.inputs.iter().map (|_input | Default::default()).collect();
      let output_locations: Inputs <Option <(usize, usize)>> = (0..machine.outputs.len()).map (| index | {
        outputs.get (index).and_then (| & (machine, input) | if machine == -1 {None} else {Some((machine as usize, input as usize))})
      }).collect();
      let initial_state = initial_state.unwrap_or_else (|| MachineMaterialsState::empty (& machine, 0));
      MachinesGraphNode {
        machine, initial_state, inputs, output_locations, original_index: usize::max_value(),
      }
    }).collect()}
  }
  
  pub fn simulate_future (&mut self) {
  
  for index in 0..self.nodes.len() {
    let mut outputs: Inputs <_>;
    let destinations;
    {
      let node = & self.nodes [index];
      let mut state = node.initial_state.clone();
      let mut input_patterns: Inputs <_> = node.inputs.iter().map (| input | input.initial_value).collect();
      outputs = node.machine.current_outputs_and_next_change (&state, & input_patterns).0.into_iter().map (| output | MachinesGraphInput {initial_value: output, changes: Vec::new()}).collect();
      destinations = node.output_locations.clone();
      let mut last_change_time = -1;
      let mut total_changes = 0;
      loop {
        total_changes += total_changes;
        assert!(total_changes < 100, "a machine probably entered an infinite loop");
        let (_current_outputs, personal_change) = node.machine.current_outputs_and_next_change (&state, & input_patterns);
        let next_change_time =
          personal_change.iter().map (| (time,_state) | *time).chain (
            node.inputs.iter().filter_map (| input | input.changes.iter().map (| (time,_pattern) | *time).find (| &time | time > last_change_time))
          ).min();
        let next_change_time = match next_change_time {
          None => break,
          Some (next_change_time) => next_change_time
        };
        //eprintln!(" {:?} ", (next_change_time, last_change_time, &personal_change)) ;
        assert!(next_change_time > last_change_time);
        for (index, (_time, pattern)) in node.inputs.iter().enumerate().filter_map (
              | (index, input) | input.changes.iter().find (| (time,_pattern) | *time == next_change_time).map (| whatever | (index, whatever))
            ) {
          state = node.machine.with_input_changed (&state, next_change_time, & input_patterns, index, *pattern);
          input_patterns [index] = *pattern;
        }
        let (_current_outputs, personal_change) = node.machine.current_outputs_and_next_change (&state, & input_patterns);
        if let Some ((time, new_state)) = personal_change {
          if time == next_change_time {
            state = new_state;
          }
        }
        let new_outputs = node.machine.current_outputs_and_next_change (&state, & input_patterns).0;
        for (output, new_pattern) in outputs.iter_mut().zip (new_outputs.into_iter()) {
          if new_pattern != output.changes.last().map_or (output.initial_value, | &(_time, pattern) | pattern) {
            output.changes.push ((next_change_time, new_pattern));
          }
        }
        last_change_time = next_change_time;
      }
    }
    for (output, destination) in outputs.into_iter().zip (destinations.into_iter()) {
      if let Some ((destination_machine, destination_input)) = destination {
        self.nodes [destination_machine].inputs [destination_input] = output;
      }
      else {
        println!("Machine {} outputted {:?}", index, output);
      }
    }
  }
  println!("Ending data:");
  for node in self.nodes.iter().enumerate() {
    println!("{:?}", node);
  }
  
  }
}*/
