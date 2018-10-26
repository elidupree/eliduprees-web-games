use super::*;

use std::cmp::{min, max};
use std::iter::{self, FromIterator};

use nalgebra::Vector2;
use arrayvec::ArrayVec;

type Number = i64;
const MAX_COMPONENTS: usize = 32;
const RATE_DIVISOR: Number = 2*2*2*2*2*2 * 3*3*3 * 5*5;
const MAX_MACHINE_INPUTS: usize = 8;
type Inputs<T> = ArrayVec <[T; MAX_MACHINE_INPUTS]>;
macro_rules! inputs {
  ($($whatever:tt)*) => {Inputs::from_iter ([$($whatever)*].iter().cloned())};
}
type Vector = Vector2 <Number>;


pub trait Machine: Clone {
  // basic information
  fn num_inputs (&self)->usize;
  fn num_outputs (&self)->usize;
  
  fn input_locations (&self, state: MachineMapState)->Inputs <Vector>;
  fn output_locations (&self, state: MachineMapState)->Inputs <Vector>;
  
  // used to infer group input flow rates
  // property: with valid inputs, the returned values have the same length given by num_inputs/num_outputs
  // property: these are consistent with each other
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number>;
  // note: this API implies that mergers must have fixed ratios
  fn min_input_rates_to_produce (&self, output_rates: & [Number])->Inputs <Number>;
  
  // property: if inputs don't change, current_output_rates doesn't change before next_output_change_time
  // property: when there is no next output change time, current_output_rates is equivalent to max_output_rates
  // maybe some property that limits the total amount of rate changes resulting from a single change by the player?
  fn with_input_changed (&self, old_state: MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern], changed_index: usize, new_pattern: FlowPattern)->MachineMaterialsState;
  // property: next_change is not the same time twice in a row
  fn current_outputs_and_next_change (&self, state: MachineMaterialsState, input_patterns: & [FlowPattern])->(Inputs <FlowPattern>, Option <(Number, MachineMaterialsState)>);

}

pub fn rotate_90 (vector: Vector, angle: u8)->Vector {
  match angle % 4 {
    0 => vector,
    1 => Vector::new (-vector [1],  vector [0]),
    2 => - vector,
    3 => Vector::new ( vector [1], -vector [0]),
    _=> unreachable!()
  }
}




#[derive (Copy, Clone, Eq, Hash, Debug, Default)]
pub struct FlowPattern {
  pub start_time: Number, //when the first item was disbursed as part of this flow
  pub rate: Number, //items per max cycle length
}

impl FlowPattern {
  fn fractional_progress_before (&self, time: Number)->Number {
    if time <= self.start_time {return RATE_DIVISOR - 1;}
    ((time - self.start_time)*self.rate + RATE_DIVISOR - 1)
  }
  pub fn num_disbursed_at_time (&self, time: Number)->Number {
    self.num_disbursed_before (time + 1) - self.num_disbursed_before (time)
  }
  pub fn num_disbursed_before (&self, time: Number)->Number {
    self.fractional_progress_before (time)/RATE_DIVISOR
  }
  pub fn num_disbursed_between (&self, range: [Number; 2])->Number {
    self.num_disbursed_before (range [1]) - self.num_disbursed_before (range [0])
  }
  pub fn last_disbursement_before (&self, time: Number)->Option <Number> {
    if time <= self.start_time || self.rate <= 0 {return None;}
    let fractional_part = self.fractional_progress_before (time) % RATE_DIVISOR;
    let time_not_disbursing = fractional_part/self.rate;
    Some(time-1 - time_not_disbursing)
  }
  pub fn when_disburses_at_least (&self, amount: Number)->Option <Number> {
    if amount <= 0 {return Some (Number::min_value());}
    if self.rate <= 0 {return None;}
    Some (self.start_time + ((amount-1)*RATE_DIVISOR)/self.rate)
  }
  pub fn from_when_will_always_disburse_at_least_amount_plus_ideal_rate (&self, amount: Number)->Option <Number> {
    if amount <= 0 {return Some (Number::min_value());}
    if self.rate <= 0 {return None;}
    Some (self.start_time + (amount*RATE_DIVISOR + self.rate - 1)/self.rate)
  }

}

impl PartialEq for FlowPattern {
  fn eq (&self, other: & Self)->bool {
    self.rate == other.rate && (self.rate == 0 || self.start_time == other.start_time)
  }
}


#[derive (Clone, PartialEq, Eq, Hash, Debug)]
struct StandardMachineInput {
  cost: Number,
  relative_location: Vector,
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
struct StandardMachineOutput {
  amount: Number,
  relative_location: Vector,
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct StandardMachine {
  inputs: Inputs <StandardMachineInput>,
  outputs: Inputs <StandardMachineOutput>,
  min_output_cycle_length: Number,
}


pub fn conveyor()->StandardMachine {
  StandardMachine {
    inputs: inputs! [StandardMachineInput {cost: 1, relative_location: Vector::new (0, 0)}],
    outputs: inputs! [StandardMachineOutput {amount: 1, relative_location: Vector::new (1, 0)}],
    min_output_cycle_length: 1,
  }
}

pub fn splitter()->StandardMachine {
  StandardMachine {
    inputs: inputs! [StandardMachineInput {cost: 2, relative_location: Vector::new (0, 0)}],
    outputs: inputs! [
      StandardMachineOutput {amount: 1, relative_location: Vector::new ( 1, 0)},
      StandardMachineOutput {amount: 1, relative_location: Vector::new (-1, 0)},
    ],
    min_output_cycle_length: 1,
  }
}
pub fn merger()->StandardMachine {
  StandardMachine {
    inputs: inputs! [
      StandardMachineInput {cost: 1, relative_location: Vector::new ( 1, 0)},
      StandardMachineInput {cost: 1, relative_location: Vector::new (-1, 0)},
     ],
    outputs: inputs! [StandardMachineOutput {amount: 2, relative_location: Vector::new (1, 0)}],
    min_output_cycle_length: 1,
  }
}

pub fn slow_machine()->StandardMachine {
  StandardMachine {
    inputs: inputs! [StandardMachineInput {cost: 1, relative_location: Vector::new (0, 0)}],
    outputs: inputs! [StandardMachineOutput {amount: 1, relative_location: Vector::new (1, 0)}],
    min_output_cycle_length: 10,
  }
}

pub fn material_generator()->StandardMachine {
  StandardMachine {
    inputs: inputs! [],
    outputs: inputs![StandardMachineOutput {amount: 1, relative_location: Vector::new (1, 0)}],
    min_output_cycle_length: 1,
  }
}

pub fn consumer()->StandardMachine {
  StandardMachine {
    inputs: inputs! [StandardMachineInput {cost: 1, relative_location: Vector::new (0, 0)}],
    outputs: inputs! [],
    min_output_cycle_length: 1,
  }
}


#[derive (Clone, PartialEq, Eq, Hash, Debug, Default)]
struct MachineMaterialsStateInput {
  storage_at_pattern_start: Number,
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct MachineMaterialsState {
  current_output_pattern: FlowPattern,
  inputs: Inputs <MachineMaterialsStateInput>,
}

impl MachineMaterialsState {
  pub fn empty <M: Machine> (machine: & M)->MachineMaterialsState {
    MachineMaterialsState {
      current_output_pattern: Default::default(),
      inputs: ArrayVec::from_iter (iter::repeat (Default::default()).take (machine.num_inputs())),
    }
  }
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct MachineMapState {
  position: Vector,
  facing: u8,
}

impl StandardMachine {
  fn max_output_rate (&self)->Number {
    RATE_DIVISOR/self.min_output_cycle_length
  }
  fn max_output_rate_with_inputs <I: IntoIterator <Item = Number>> (&self, input_rates: I)->Number {
    let mut ideal_rate = self.max_output_rate();
    for (rate, input) in input_rates.into_iter().zip (self.inputs.iter()) {
      ideal_rate = min (ideal_rate, rate/input.cost);
    }
    ideal_rate
  }
  fn min_output_rate_to_produce <I: IntoIterator <Item = Number>> (&self, output_rates: I)->Number {
    output_rates.into_iter().zip (self.outputs.iter()).map (| (rate, output) | (rate + output.amount - 1)/output.amount).max().unwrap_or(self.max_output_rate())
  }
}

impl Machine for StandardMachine {
  fn num_inputs (&self)->usize {self.inputs.len()}
  fn num_outputs (&self)->usize {self.outputs.len()}
  
  fn input_locations (&self, state: MachineMapState)->Inputs <Vector> {
    self.inputs.iter().map (| input | rotate_90 (input.relative_location, state.facing) + state.position).collect()
  }
  fn output_locations (&self, state: MachineMapState)->Inputs <Vector> {
    self.inputs.iter().map (| input | rotate_90 (input.relative_location, state.facing) + state.position).collect()
  }
  
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number> {
    let ideal_rate = self.max_output_rate_with_inputs (input_rates.iter().cloned());
    self.outputs.iter().map (| output | ideal_rate*output.amount).collect()
  }
  fn min_input_rates_to_produce (&self, output_rates: & [Number])->Inputs <Number> {
    let ideal_rate = self.min_output_rate_to_produce (output_rates.iter().cloned());
    self.inputs.iter().map (| input | ideal_rate*input.cost).collect()
  }
  
  fn with_input_changed (&self, old_state: MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern], changed_index: usize, _new_pattern: FlowPattern)->MachineMaterialsState {
    let mut new_state = old_state;
    new_state.inputs [changed_index].storage_at_pattern_start += old_input_patterns [changed_index].num_disbursed_before (change_time);
    new_state
  }
  fn current_outputs_and_next_change (&self, state: MachineMaterialsState, input_patterns: & [FlowPattern])->(Inputs <FlowPattern>, Option <(Number, MachineMaterialsState)>)
 {
    let ideal_rate = self.max_output_rate_with_inputs (input_patterns.iter().map (| pattern | pattern.rate));
    let last_change_time = input_patterns.iter().map (| pattern | pattern.start_time).max().unwrap_or (0);
    let time_to_switch_output = match state.current_output_pattern.last_disbursement_before (last_change_time) {
      Some (time) => max (last_change_time, time + self.min_output_cycle_length),
      None => last_change_time,
    };
    let mut time_to_begin_output = time_to_switch_output;
    if ideal_rate > 0 {
    let mut when_enough_inputs_to_begin_output = last_change_time;
    for ((pattern, input), input_state) in input_patterns.iter().zip (self.inputs.iter()).zip (state.inputs.iter()) {
      let enough_to_start_amount = input.cost + 1;
      let total_disbursed_so_far = pattern.num_disbursed_before (last_change_time);
      let storage_at_last_change =
        input_state.storage_at_pattern_start
        + total_disbursed_so_far
        - input.cost*state.current_output_pattern.num_disbursed_between ([pattern.start_time, last_change_time]);
      let remaining_need = enough_to_start_amount - storage_at_last_change;
      let min_start_time = pattern.from_when_will_always_disburse_at_least_amount_plus_ideal_rate (input.cost - 1 - input_state.storage_at_pattern_start).unwrap();
      when_enough_inputs_to_begin_output = max (when_enough_inputs_to_begin_output, min_start_time);
    }
      time_to_begin_output = max (time_to_begin_output, when_enough_inputs_to_begin_output);
    }
    
    let output = FlowPattern {start_time: time_to_begin_output, rate: ideal_rate};
    
    let next_change = if output == state.current_output_pattern {
      None
    } else {
      let mut new_state = state.clone();
      new_state.current_output_pattern = output;
      Some ((time_to_switch_output, new_state))
    };
    
    let current_outputs = self.outputs.iter().map (| output | {
      FlowPattern {start_time: state.current_output_pattern.start_time + 1, rate: state.current_output_pattern.rate*output.amount}
    }).collect();
    (current_outputs, next_change)
  }
}



#[derive (Clone, PartialEq, Eq, Hash, Debug, Default)]
struct MachinesGraphInput {
  initial_value: FlowPattern,
  changes: Vec<(Number, FlowPattern)>,
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
struct MachinesGraphNode {
  machine: StandardMachine,
  initial_state: MachineMaterialsState,
  inputs: Inputs <MachinesGraphInput>,
  output_locations: Inputs <Option <(usize, usize)>>
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct MachinesGraph {
  nodes: Vec<MachinesGraphNode>,
}

impl MachinesGraph {
  pub fn new (data: Vec<(StandardMachine, Option <MachineMaterialsState>, &[(i64, i64)])>)->MachinesGraph {
    MachinesGraph {nodes: data.into_iter().map (| (machine, initial_state, outputs) | {
      let inputs: Inputs <MachinesGraphInput> = machine.inputs.iter().map (|_input | Default::default()).collect();
      let output_locations: Inputs <Option <(usize, usize)>> = (0..machine.outputs.len()).map (| index | {
        outputs.get (index).and_then (| & (machine, input) | if machine == -1 {None} else {Some((machine as usize, input as usize))})
      }).collect();
      let initial_state = initial_state.unwrap_or (MachineMaterialsState::empty (& machine));
      MachinesGraphNode {
        machine, initial_state, inputs, output_locations,
      }
    }).collect()}
  }
  
  pub fn from_map (data: & [(StandardMachine, MachineMapState, MachineMaterialsState)]) {
    let connections: ArrayVec<[Inputs<Option<(usize, usize)>>; MAX_COMPONENTS]> = data.iter().map (| (machine, map,_) | {
      machine.output_locations(map.clone()).into_iter().map (| output_location | {
        data.iter().enumerate().find_map(| (machine2_index, (machine2, map2, _)) | {
          machine2.input_locations(map2.clone()).into_iter().enumerate().find_map(| (input_index, input_location) | {
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
    
    fn push_node (nodes: &mut Vec<MachinesGraphNode>, data_to_node: &mut Vec<Option <usize>>, node_to_data: &mut Vec<Option <usize>>, levels: &mut ArrayVec<[usize; MAX_COMPONENTS]>, data_index: usize, (machine, _, materials): &(StandardMachine, MachineMapState, MachineMaterialsState), level: usize) {
      let current_level = levels [data_index];
      if current_level < level {
          //TODO: cycle handling
          return;
      } else if current_level == level {
          // already recorded
          return;
      } else if level != usize::max_value() {
        unreachable!()
      }
      data_to_node [data_index] = Some (nodes.len());
      node_to_data [nodes.len()] = Some (data_index);
      levels [data_index] = level;
      let inputs: Inputs <MachinesGraphInput> = machine.inputs.iter().map (|_input | Default::default()).collect();
      nodes.push(MachinesGraphNode {
        machine: machine.clone(), initial_state: materials.clone(), inputs, output_locations: Default::default(),
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
  }
  
  
  
}


pub fn print_future (mut graph: MachinesGraph) {
  for index in 0..graph.nodes.len() {
    let mut outputs: Inputs <_>;
    let destinations;
    {
      let node = & graph.nodes [index];
      let mut state = node.initial_state.clone();
      let mut input_patterns: Inputs <_> = node.inputs.iter().map (| input | input.initial_value).collect();
      outputs = node.machine.current_outputs_and_next_change (state.clone(), & input_patterns).0.into_iter().map (| output | MachinesGraphInput {initial_value: output, changes: Vec::new()}).collect();
      destinations = node.output_locations.clone();
      let mut last_change_time = -1;
      let mut total_changes = 0;
      loop {
        total_changes = total_changes + 1;
        assert!(total_changes < 100, "a machine probably entered an infinite loop");
        let (_current_outputs, personal_change) = node.machine.current_outputs_and_next_change (state.clone(), & input_patterns);
        let next_change_time =
          personal_change.iter().map (| (time,_state) | *time).chain (
            node.inputs.iter().filter_map (| input | input.changes.iter().map (| (time,_pattern) | *time).find (| &time | time > last_change_time))
          ).min();
        let next_change_time = match next_change_time {
          None => break,
          Some (next_change_time) => next_change_time
        };
        assert!(next_change_time > last_change_time);
        for (index, (_time, pattern)) in node.inputs.iter().enumerate().filter_map (
              | (index, input) | input.changes.iter().find (| (time,_pattern) | *time == next_change_time).map (| whatever | (index, whatever))
            ) {
          state = node.machine.with_input_changed (state, next_change_time, & input_patterns, index, *pattern);
          input_patterns [index] = *pattern;
        }
        if let Some ((time, new_state)) = personal_change {
          if time == next_change_time {
            state = new_state;
          }
        }
        let new_outputs = node.machine.current_outputs_and_next_change (state.clone(), & input_patterns).0;
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
        graph.nodes [destination_machine].inputs [destination_input] = output;
      }
      else {
        println!("Machine {} outputted {:?}", index, output);
      }
    }
  }
  println!("Ending data:");
  for node in graph.nodes.iter().enumerate() {
    println!("{:?}", node);
  }
}


/*

enum SingularComponentType {
  Conveyor,
  Producer,
  Consumer,
}

enum ComponentType {
  Singular (SingularComponentType),
  Group (u16),
}

pub struct Component {
  position: Vector2 <Number>,
  scale: u8,
  facing: u8,
  component_type: ComponentType,
}

pub struct Group {
  size: Position,
  components: ArrayVec <[Component; MAX_COMPONENTS]>,
  average_color: [f64; 3],
}


pub struct Map {
  components: ArrayVec <[Component; MAX_COMPONENTS]>,
}

*/

#[cfg (test)]
mod tests {
  use super::*;
  
  fn assert_flow_pattern (rate: Number, prefix: & [Number]) {
    assert_eq! (
      prefix,
      (0..prefix.len()).map (| index | FlowPattern {start_time: 0, rate: rate}.num_disbursed_at_time (index as Number)).collect::<Vec <_>>().as_slice()
    );
  }
  
  #[test]
  fn flow_pattern_unit_tests() {
    assert_flow_pattern (RATE_DIVISOR, &[1, 1, 1, 1]);
    assert_flow_pattern (RATE_DIVISOR/2, &[1, 0, 1, 0, 1, 0, 1, 0]);
    assert_flow_pattern (RATE_DIVISOR/3, &[1, 0, 0, 1, 0, 0, 1, 0]);
    assert_flow_pattern (RATE_DIVISOR*2/3, &[1, 1, 0, 1, 1, 0, 1, 1]);
  }
  
  proptest! {
    #[test]
    fn randomly_test_flow_pattern_density_property(start in 0i64..1000000, rate in 0..=RATE_DIVISOR, initial_time in 0i64..1000000, duration in 0i64..1000000) {
      let initial_time = initial_time + start;
      let ideal_rounded_down = rate*duration/RATE_DIVISOR;
      let ideal_rounded_up = (rate*duration + RATE_DIVISOR - 1)/RATE_DIVISOR;
      let observed = FlowPattern {start_time: start, rate: rate}.num_disbursed_between ([initial_time, initial_time + duration]);
      prop_assert!(observed >= ideal_rounded_down);
      prop_assert!(observed <= ideal_rounded_up);
    }
    
    #[test]
    fn randomly_test_flow_pattern_density_rounds_up_from_beginning (start in 0i64..1000000, rate in 0..=RATE_DIVISOR, duration in 0i64..1000000) {
      let ideal_rounded_up = (rate*duration + RATE_DIVISOR - 1)/RATE_DIVISOR;
      let observed = FlowPattern {start_time: start, rate: rate}.num_disbursed_before (start + duration);
      prop_assert_eq!(observed, ideal_rounded_up);
    }
    
    #[test]
    fn randomly_test_last_disbursement_before (start in 0i64..1000000, rate in 1..=RATE_DIVISOR, initial_time in 1i64..1000000) {
      let initial_time = initial_time + start;
      let pattern = FlowPattern {start_time: start, rate: rate};
      let observed = pattern.last_disbursement_before (initial_time).unwrap();
      println!("{}", observed);
      prop_assert! (observed <initial_time) ;
      prop_assert_eq!(pattern.num_disbursed_between ([observed+1, initial_time]), 0);
      prop_assert_eq!(pattern.num_disbursed_between ([observed, initial_time]), 1);
    }
    
    #[test]
    fn randomly_test_when_disburses_at_least (start in 0i64..1000000, rate in 1..=RATE_DIVISOR, amount in 1i64..1000000) {
      let pattern = FlowPattern {start_time: start, rate: rate};
      let observed = pattern.when_disburses_at_least(amount).unwrap();
      println!("{}, {}, {}", observed, pattern.num_disbursed_before (observed), pattern.num_disbursed_before (observed + 1));
      prop_assert_eq!(pattern.num_disbursed_before (observed), amount - 1);
      prop_assert_eq!(pattern.num_disbursed_before (observed + 1), amount);
    }
    
    #[test]
    fn randomly_test_from_when_will_always_disburse_at_least_amount_plus_ideal_rate (start in 0i64..1000000, rate in 1..=RATE_DIVISOR, amount in 1i64..1000000, duration in 0i64..1000000) {
      let pattern = FlowPattern {start_time: start, rate: rate};
      let observed = pattern.from_when_will_always_disburse_at_least_amount_plus_ideal_rate (amount).unwrap();
      let ideal_count_rounded_up = amount + (rate*(duration+1) + RATE_DIVISOR - 1)/RATE_DIVISOR;
      let observed_count = pattern.num_disbursed_before (observed + duration + 1);
      println!("{}, {}, {}", observed, ideal_count_rounded_up, observed_count);
      prop_assert!(observed_count >= ideal_count_rounded_up);
    }

  }
}
