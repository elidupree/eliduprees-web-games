use super::*;

use arrayvec::ArrayVec;

#[derive (Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct MachinesGraphInput {
  pub initial_value: FlowPattern,
  pub changes: Vec<(Number, FlowPattern)>,
}

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
      let initial_state = initial_state.unwrap_or_else (|| MachineMaterialsState::empty (& machine));
      MachinesGraphNode {
        machine, initial_state, inputs, output_locations, original_index: usize::max_value(),
      }
    }).collect()}
  }
  
  pub fn from_map (data: & [StatefulMachine])->MachinesGraph {
    let connections: ArrayVec<[Inputs<Option<(usize, usize)>>; MAX_COMPONENTS]> = data.iter().map (| machine | {
      machine.machine_type.output_locations(&machine.map_state).into_iter().map (| output_location | {
        data.iter().enumerate().find_map(| (machine2_index, machine2) | {
          machine2.machine_type.input_locations(& machine2.map_state).into_iter().enumerate().find_map(| (input_index, input_location) | {
            if input_location == output_location {
              Some((machine2_index, input_index))
            }
            else {
              None
            }
          })
        })
      }).collect()
    }).collect();
    
    let mut levels: ArrayVec<[usize; MAX_COMPONENTS]> = data.iter().map (|_| usize::max_value()).collect();
    let mut num_inputs: ArrayVec<[usize; MAX_COMPONENTS]> = data.iter().map (|_| 0).collect();
    let mut nodes: Vec<MachinesGraphNode> = Vec::with_capacity(MAX_COMPONENTS);
    let mut data_to_node = (0..data.len()).map (| _index | None).collect();
    let mut node_to_data = (0..data.len()).map (| _index | None).collect();
    for machine in &connections {
      for output in machine {
        if let Some(output) = output {
          num_inputs[output.0] += 1
        }
      }
    }
    
    fn push_node (nodes: &mut Vec<MachinesGraphNode>, data_to_node: &mut Vec<Option <usize>>, node_to_data: &mut Vec<Option <usize>>, levels: &mut ArrayVec<[usize; MAX_COMPONENTS]>, data_index: usize, machine: & StatefulMachine, level: usize) {
      let current_level = levels [data_index];
      //eprintln!(" {:?} ", (current_level, level));
      if current_level < level {
          //TODO: cycle handling
          eprintln!(" I don't know how to handle cycles yet!");
          return;
      } else if current_level == level {
          // already recorded
          return;
      } else if current_level != usize::max_value() {
        unreachable!()
      }
      data_to_node [data_index] = Some (nodes.len());
      node_to_data [nodes.len()] = Some (data_index);
      levels [data_index] = level;
      let inputs: Inputs <MachinesGraphInput> = machine.machine_type.inputs.iter().map (|_input | Default::default()).collect();
      nodes.push(MachinesGraphNode {
        machine: machine.machine_type.clone(), original_index: data_index, initial_state: machine.materials_state.clone(), inputs, output_locations: Default::default(),
      });
    }
    for (index, inputs) in num_inputs.iter().enumerate() {
      if *inputs == 0 {
        push_node (&mut nodes, &mut data_to_node, &mut node_to_data, &mut levels, index, & data [index], 0);
      }
    }
    
    for node_index in 0.. {
      if node_index >= nodes.len() {break}
      
      let data_index = node_to_data [node_index].unwrap();
      let level = levels [data_index];
      for destination in &connections[data_index] {
        if let Some((target_data_index, _input_index)) = destination {
          push_node (&mut nodes, &mut data_to_node, &mut node_to_data, &mut levels, *target_data_index, & data [*target_data_index], level + 1);
        }
      }
    }
    
    for (node_index, node) in nodes.iter_mut().enumerate() {
      node.output_locations =
        connections [node_to_data [node_index].unwrap()]
        .iter().map (| destination |
          destination.and_then (| (machine, input) |
            data_to_node [machine].map (| index | (index , input))
          )
        ).collect();
    }
    
    MachinesGraph {nodes}
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
}
