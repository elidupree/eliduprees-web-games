use super::*;

use nalgebra::Vector2;
use arrayvec::ArrayVec;

type Number = i64;
const MAX_COMPONENTS: usize = 32;
const RATE_DIVISOR: Number = 2*2*2*2*2*2 * 3*3*3 * 5*5;
const MAX_MACHINE_INPUTS: usize = 8;
type Inputs<T> = ArrayVec <[Number; MAX_MACHINE_INPUTS]>;
type Position = Vector2 <Number>;


pub trait Machine: Clone {
  // basic information
  fn num_inputs (&self)->usize;
  fn num_outputs (&self)->usize;
  type MaterialsState: Clone;
  
  // used to infer group input flow rates
  // property: with valid inputs, the returned values have the same length given by num_inputs/num_outputs
  // property: these are consistent with each other
  fn max_output_rates (&self, input_rates: Inputs <Number>)->Inputs <Number>;
  // note: this API implies that mergers must have fixed ratios
  fn min_input_rates_to_produce (&self, output_rates: Inputs <Number>)->Inputs <Number>;
  
  // property: if inputs don't change, current_output_rates doesn't change before next_output_change_time
  // property: when there is no next output change time, current_output_rates is equivalent to max_output_rates
  // maybe some property that limits the total amount of rate changes resulting from a single change by the player?
  fn with_inputs_changed (&self, old_state: Self::MaterialsState, when: Number, input_patterns: Inputs <FlowPattern>)->Self::MaterialsState;
  fn current_outputs_and_next_change (&self, state: Self::MaterialsState, input_patterns: Inputs <FlowPattern>)->(Inputs <FlowPattern>, Option <(Number, Self::MaterialsState)>);

}







pub struct FlowPattern {
  pub start_time: Number, //when the first item was disbursed as part of this flow
  pub rate: Number, //items per max cycle length
}

impl FlowPattern {
  pub fn num_disbursed_at_time (&self, time: Number)->bool {
    self.num_disbursed_before (time + 1) - self.num_disbursed_before (time)
  }
  pub fn num_disbursed_before (&self, time: Number)->Number {
    if time <= self.start_time {return 0;}
    1 + ((time - self.start_time)*self.rate)/RATE_DIVISOR
  }
  pub fn num_disbursed_between (&self, range: [Number; 2])->Number {
    self.num_disbursed_before (range [1]) - self.num_disbursed_before (range [0])
  }
  pub fn time_to_disburse_at_least (&self, collection_start_time: Number, amount: Number)->Number {
    
  }
}

/*
pub fn entire_future (machines:, max_time: time) {
  for machine_info in machine_dag.forwards_iter_mut() {
    let mut simulation_time = now;
    loop {
      let change = first of machine_info.inputs.changes, machine_info.machine.next_output_change_time()
      match change {
        None => break,
        Some (time, change) => {
          if time > max_time {break;} 
          match change {
            OutputChange => {
              machine_info.machine.next_output_change_time_reached();
              machine_info.changes.push (machine_info.machine.current_output_rates());
            }
            InputChange => machine_info.machine.inputs_changed(),
          }
        }
      }
    }
  }
  changes map
}*/

#[derive (Clone, Hash, Debug)]
struct StandardMachineInput {
  cost: Number,
}

#[derive (Clone, Hash, Debug)]
struct StandardMachineOutput {
  
}

#[derive (Clone, Hash, Debug)]
struct StandardMachine {
  inputs: Inputs <StandardMachineInput>,
  outputs: Inputs <StandardMachineOutput>,
  min_output_cycle_length: Number,
}

#[derive (Clone, Hash, Debug)]
struct MachineMaterialsStateInput {
  storage_at_last_pattern_start: Number,
}

#[derive (Clone, Hash, Debug)]
struct MachineMaterialsState {
  current_output_pattern: FlowPattern,
  inputs: Inputs <MachineMaterialsStateInput>,
}

impl StandardMachine {
  fn max_output_rate (&self)->Number {
    RATE_DIVISOR/map_state.min_output_cycle_length
  }
  fn max_output_rate_with_inputs <I: IntoIterator <Item = Number>> (&self, input_rates: I)->Number {
    let mut ideal_rate = self.max_output_rate();
    for (rate, input) in input_rates.into_iter().zip (self.inputs.iter()) {
      ideal_rate = min (ideal_rate, rate/input.cost);
    }
    ideal_rate
  }
  fn min_output_rate_to_produce <I: IntoIterator <Item = Number>> (&self, output_rates: I)->Number {
    output_rates.iter().max().unwrap_or(self.max_output_rate())
  }
}

impl Machine for StandardMachine {
  fn num_inputs (&self)->usize {self.inputs.len()}
  fn num_outputs (&self)->usize {self.outputs.len()}
  
  fn max_output_rates (&self, input_rates: Inputs <Number>)->Inputs <Number> {
    let ideal_rate = self.max_output_rate_with_inputs (input_rates);
    self.outputs.iter().map (| _output | ideal_rate).collect()
  }
  fn min_input_rates_to_produce (&self, output_rates: Inputs <Number>)->Inputs <Number> {
    let ideal_rate = self.min_output_rate_to_produce (output_rates);
    self.inputs.iter().map (| input | ideal_rate*input.cost).collect()
  }
  
  fn with_inputs_changed (&self, old_state: MachineMaterialsState, when: Number, input_patterns: Inputs <FlowPattern>)->MachineMaterialsState {
    old_state
  }
  fn current_outputs_and_next_change (&self, state: MachineMaterialsState, input_patterns: Inputs <FlowPattern>)->(Inputs <FlowPattern>, Option <(Number, MachineMaterialsState)>)
 {
    let ideal_rate = self.max_output_rate_with_inputs (input_patterns.iter().map (| pattern | pattern.rate));
    let last_change_time = input_patterns.map (| pattern | pattern.start_time).
    let mut when_enough_inputs_to_begin_output = last_change;
    for ((pattern, input), input_state) in input_patterns.iter().zip (self.inputs.iter()).zip (state.inputs.iter()) {
      let enough_to_start_amount = input.cost + 1;
      let storage_at_last_change =
        input_state.storage_at_pattern_start
        + pattern.num_disbursed_before (last_change_time)
        - input.cost*state.current_output_pattern.num_disbursed_between ([pattern.start_time, last_change_time]);
      let min_start_time = last_change_time + pattern.time_to_disburse_at_least (pattern.start_time, enough_to_start_amount - storage_at_last_change);
      when_enough_inputs_to_begin_output = max (time_to_begin_output, min_start_time);
    }
    let most_recent_output = state.current_output_pattern.last_disbursement_before (last_change_time).unwrap_or (last_change_time - self.min_output_cycle_length);
    let time_to_begin_output = max (when_enough_inputs_to_begin_output, most_recent_output + self.min_output_cycle_length);
    let output = FlowPattern {start_time: time_to_begin_output, rate: ideal_rate};
    
    let next_change = if output == state.current_output_pattern {
      None
    } else {
      Some ((input.start_time + 1, ConveyorMaterialsState {
        current_output: output
      }))
    };
    
    let current_outputs = self.outputs.iter().map (| _output | {
      FlowPattern {start_time: state.current_output_pattern.start_time + 1, rate: state.current_output_pattern.rate}
    }).collect();
    (current_outputs, next_change)
  }
}





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
  size: Vector2 <Length>,
  components: ArrayVec <[Component; MAX_COMPONENTS]>,
  average_color: [f64; 3],
}


pub struct Map {
  components: ArrayVec <[Component; MAX_COMPONENTS]>,
}


and