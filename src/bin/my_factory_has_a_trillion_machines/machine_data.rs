use super::*;

use std::cmp::{min, max};
use std::iter::{self, FromIterator};
use std::hash::{Hash, Hasher};

use num::Integer;
use nalgebra::Vector2;
use arrayvec::ArrayVec;

pub type Number = i64;
pub const MAX_COMPONENTS: usize = 32;
pub const RATE_DIVISOR: Number = 2*2*2*2*2*2 * 3*3*3 * 5*5;
pub const MAX_MACHINE_INPUTS: usize = 8;
pub type Inputs<T> = ArrayVec <[T; MAX_MACHINE_INPUTS]>;
macro_rules! inputs {
  ($($whatever:tt)*) => {Inputs::from_iter ([$($whatever)*].iter().cloned())};
}
pub type Vector = Vector2 <Number>;


pub trait MachineType: Clone {
  // basic information
  fn num_inputs (&self)->usize;
  fn num_outputs (&self)->usize;
  
  fn input_locations (&self, state: &MachineMapState)->Inputs <Vector>;
  fn output_locations (&self, state: &MachineMapState)->Inputs <Vector>;
  
  // used to infer group input flow rates
  // property: with valid inputs, the returned values have the same length given by num_inputs/num_outputs
  // property: these are consistent with each other
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number>;
  // note: this API implies that mergers must have fixed ratios
  fn min_input_rates_to_produce (&self, output_rates: & [Number])->Inputs <Number>;
  
  // property: if inputs don't change, current_output_rates doesn't change before next_output_change_time
  // property: when there is no next output change time, current_output_rates is equivalent to max_output_rates
  // maybe some property that limits the total amount of rate changes resulting from a single change by the player?
  fn with_input_changed (&self, old_state: &MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern], changed_index: usize, new_pattern: FlowPattern)->MachineMaterialsState;
  // property: next_change is not the same time twice in a row
  fn current_outputs_and_next_change (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->(Inputs <FlowPattern>, Option <(Number, MachineMaterialsState)>);

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




#[derive (Copy, Clone, Eq, Debug, Default)]
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
  pub fn time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (&self, amount: Number)->Option <Number> {
    if self.rate <= 0 && amount > 0 {return None;}
    Some (self.start_time + (amount*RATE_DIVISOR + self.rate - 1).div_floor(&self.rate))
  }

}

impl PartialEq for FlowPattern {
  fn eq (&self, other: & Self)->bool {
    self.rate == other.rate && (self.rate == 0 || self.start_time == other.start_time)
  }
}
impl Hash for FlowPattern {
  fn hash<H: Hasher> (&self, hasher: &mut H) {
    self.rate.hash (hasher);
    if self.rate != 0 {
      self.start_time.hash (hasher);
    }
  }
}


#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct StandardMachineInput {
  pub cost: Number,
  pub relative_location: Vector,
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct StandardMachineOutput {
  pub amount: Number,
  pub relative_location: Vector,
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct StandardMachine {
  pub name: & 'static str,
  pub inputs: Inputs <StandardMachineInput>,
  pub outputs: Inputs <StandardMachineOutput>,
  pub min_output_cycle_length: Number,
}


pub fn conveyor()->StandardMachine {
  StandardMachine {
    name: "Conveyor",
    inputs: inputs! [StandardMachineInput {cost: 1, relative_location: Vector::new (0, 0)}],
    outputs: inputs! [StandardMachineOutput {amount: 1, relative_location: Vector::new (1, 0)}],
    min_output_cycle_length: 1,
  }
}

pub fn splitter()->StandardMachine {
  StandardMachine {
    name: "Splitter",
    inputs: inputs! [StandardMachineInput {cost: 2, relative_location: Vector::new (0, 0)}],
    outputs: inputs! [
      StandardMachineOutput {amount: 1, relative_location: Vector::new (0,  1)},
      StandardMachineOutput {amount: 1, relative_location: Vector::new (0, -1)},
    ],
    min_output_cycle_length: 1,
  }
}
pub fn merger()->StandardMachine {
  StandardMachine {
    name: "Merger",
    inputs: inputs! [
      StandardMachineInput {cost: 1, relative_location: Vector::new (0,  1)},
      StandardMachineInput {cost: 1, relative_location: Vector::new (0, -1)},
     ],
    outputs: inputs! [StandardMachineOutput {amount: 2, relative_location: Vector::new (1, 0)}],
    min_output_cycle_length: 1,
  }
}

pub fn slow_machine()->StandardMachine {
  StandardMachine {
    name: "Slow machine",
    inputs: inputs! [StandardMachineInput {cost: 1, relative_location: Vector::new (0, 0)}],
    outputs: inputs! [StandardMachineOutput {amount: 1, relative_location: Vector::new (1, 0)}],
    min_output_cycle_length: 10,
  }
}

pub fn material_generator()->StandardMachine {
  StandardMachine {
    name: "Material generator",
    inputs: inputs! [],
    outputs: inputs![StandardMachineOutput {amount: 1, relative_location: Vector::new (1, 0)}],
    min_output_cycle_length: 1,
  }
}

pub fn consumer()->StandardMachine {
  StandardMachine {
    name: "Consumer",
    inputs: inputs! [StandardMachineInput {cost: 1, relative_location: Vector::new (0, 0)}],
    outputs: inputs! [],
    min_output_cycle_length: 1,
  }
}


#[derive (Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct MachineMaterialsStateInput {
  pub storage_before_last_flow_change: Number,
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct MachineMaterialsState {
  pub current_output_pattern: FlowPattern,
  pub inputs: Inputs <MachineMaterialsStateInput>,
  pub last_flow_change: Number,
}

impl MachineMaterialsState {
  pub fn empty <M: MachineType> (machine: & M)->MachineMaterialsState {
    MachineMaterialsState {
      current_output_pattern: Default::default(),
      last_flow_change: 0,
      inputs: ArrayVec::from_iter (iter::repeat (Default::default()).take (machine.num_inputs())),
    }
  }
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct MachineMapState {
  pub position: Vector,
  pub facing: u8,
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
    output_rates.into_iter().zip (self.outputs.iter()).map (| (rate, output) | (rate + output.amount - 1)/output.amount).max().unwrap_or_else(|| self.max_output_rate())
  }
  
  pub fn input_storage_at (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern], time: Number)->Inputs <Number> {
    let interval = [state.last_flow_change, time];
    let output_disbursements = state.current_output_pattern.num_disbursed_between (interval);
    
    self.inputs.iter().zip (state.inputs.iter()).zip (input_patterns).map (| ((input, materials), pattern) | {
      let accumulated = pattern.num_disbursed_between (interval);
      let spent = output_disbursements*input.cost;
      materials.storage_before_last_flow_change + accumulated - spent
    }).collect()
  }
  
  fn update_last_flow_change (&self, state: &mut MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern]) {
    let storages =self.input_storage_at (state, old_input_patterns, change_time);
    for (input, storage) in state.inputs.iter_mut().zip (storages) {
      input.storage_before_last_flow_change = storage;
    }
    
    state.last_flow_change = change_time;
  }
}

impl MachineType for StandardMachine {
  fn num_inputs (&self)->usize {self.inputs.len()}
  fn num_outputs (&self)->usize {self.outputs.len()}
  
  fn input_locations (&self, state: &MachineMapState)->Inputs <Vector> {
    self.inputs.iter().map (| input | rotate_90 (input.relative_location, state.facing) + state.position).collect()
  }
  fn output_locations (&self, state: &MachineMapState)->Inputs <Vector> {
    self.outputs.iter().map (| input | rotate_90 (input.relative_location, state.facing) + state.position).collect()
  }
  
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number> {
    let ideal_rate = self.max_output_rate_with_inputs (input_rates.iter().cloned());
    self.outputs.iter().map (| output | ideal_rate*output.amount).collect()
  }
  fn min_input_rates_to_produce (&self, output_rates: & [Number])->Inputs <Number> {
    let ideal_rate = self.min_output_rate_to_produce (output_rates.iter().cloned());
    self.inputs.iter().map (| input | ideal_rate*input.cost).collect()
  }
  
  fn with_input_changed (&self, old_state: &MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern], _changed_index: usize, _new_pattern: FlowPattern)->MachineMaterialsState {
    let mut new_state = old_state.clone();
    self.update_last_flow_change (&mut new_state, change_time, old_input_patterns);
    new_state
  }
  fn current_outputs_and_next_change (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->(Inputs <FlowPattern>, Option <(Number, MachineMaterialsState)>)
 {
    let ideal_rate = self.max_output_rate_with_inputs (input_patterns.iter().map (| pattern | pattern.rate));
    let time_to_switch_output = match state.current_output_pattern.last_disbursement_before (state.last_flow_change) {
      Some (time) => max (state.last_flow_change, time + self.min_output_cycle_length),
      None => state.last_flow_change,
    };
    let mut time_to_begin_output = time_to_switch_output;
    if ideal_rate > 0 {
      let mut when_enough_inputs_to_begin_output = state.last_flow_change;
      for ((pattern, input), input_state) in input_patterns.iter().zip (self.inputs.iter()).zip (state.inputs.iter()) {
        let already_disbursed = pattern.num_disbursed_before (state.last_flow_change);
        let min_start_time = pattern.time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (already_disbursed + ((input.cost - 1) - input_state.storage_before_last_flow_change)).unwrap();
        //eprintln!(" {:?} ", (already_disbursed, min_start_time));
        when_enough_inputs_to_begin_output = max (when_enough_inputs_to_begin_output, min_start_time);
      }
      time_to_begin_output = max (time_to_begin_output, when_enough_inputs_to_begin_output);
    }
    
    let output = FlowPattern {start_time: time_to_begin_output, rate: ideal_rate};
    //eprintln!(" {:?} ", (self, state, input_patterns, output));
    
    let next_change = if output == state.current_output_pattern {
      None
    } else {
      let mut new_state = state.clone();
      self.update_last_flow_change (&mut new_state, time_to_switch_output, input_patterns);
      new_state.current_output_pattern = output;
      Some ((time_to_switch_output, new_state))
    };
    
    let current_outputs = self.outputs.iter().map (| output | {
      FlowPattern {start_time: state.current_output_pattern.start_time + 1, rate: state.current_output_pattern.rate*output.amount}
    }).collect();
    (current_outputs, next_change)
  }
}


#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct StatefulMachine {
  pub machine_type: StandardMachine,
  pub map_state: MachineMapState,
  pub materials_state: MachineMaterialsState,
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

*/

pub struct Map {
  pub machines: ArrayVec <[StatefulMachine; MAX_COMPONENTS]>,
}


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
    fn randomly_test_time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (start in 0i64..1000000, rate in 1..=RATE_DIVISOR, amount in -100000i64..1000000, duration in 0i64..1000000) {
      let pattern = FlowPattern {start_time: start, rate: rate};
      let observed = pattern.time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (amount).unwrap();
      let ideal_count_rounded_up = amount + (rate*(duration+1) + RATE_DIVISOR - 1)/RATE_DIVISOR;
      let observed_count = pattern.num_disbursed_before (observed + duration + 1);
      println!("{}, {}, {}", observed, ideal_count_rounded_up, observed_count);
      prop_assert!(observed_count >= ideal_count_rounded_up);
    }

  }
}
