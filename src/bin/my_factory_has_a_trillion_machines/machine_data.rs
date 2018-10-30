use super::*;

use std::cmp::{min, max};
use std::iter::{self, FromIterator};

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
pub type Facing = u8;


pub trait MachineTypeTrait: Clone {
  // basic information
  fn name (&self)->& str;
  fn num_inputs (&self)->usize;
  fn num_outputs (&self)->usize;
  
  fn input_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)>;
  fn output_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)>;
  
  // used to infer group input flow rates
  // property: with valid inputs, the returned values have the same length given by num_inputs/num_outputs
  // property: these are consistent with each other
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number>;
  fn reduced_input_rates_that_can_still_produce (&self, input_rates: & [Number], output_rates: & [Number])->Inputs <Number>;
  
  // property: if inputs don't change, current_output_rates doesn't change before next_output_change_time
  // property: when there is no next output change time, current_output_rates is equivalent to max_output_rates
  // maybe some property that limits the total amount of rate changes resulting from a single change by the player?
  fn with_input_changed (&self, old_state: &MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern], changed_index: usize, new_pattern: FlowPattern)->MachineMaterialsState;
  // property: next_change is not the same time twice in a row
  fn current_outputs_and_next_change (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->(Inputs <FlowPattern>, Option <(Number, MachineMaterialsState)>);

}

macro_rules! machine_type_enum {
  ($($Variant: ident,)*) => {
  

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub enum MachineType {
  $($Variant ($Variant),)*
}

impl MachineTypeTrait for MachineType {
  fn name (&self)->& str {match self {$(MachineType::$Variant (value) => value.name (),)*}}
  fn num_inputs (&self)->usize {match self {$(MachineType::$Variant (value) => value.num_inputs(),)*}}
  fn num_outputs (&self)->usize {match self {$(MachineType::$Variant (value) => value.num_outputs(),)*}}
  
  fn input_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)> {match self {$(MachineType::$Variant (value) => value.input_locations (state ),)*}}
  fn output_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)> {match self {$(MachineType::$Variant (value) => value.output_locations (state ),)*}}
  
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number> {match self {$(MachineType::$Variant (value) => value.max_output_rates (input_rates ),)*}}
  fn reduced_input_rates_that_can_still_produce (&self, input_rates: & [Number], output_rates: & [Number])->Inputs <Number> {match self {$(MachineType::$Variant (value) => value.reduced_input_rates_that_can_still_produce (input_rates, output_rates ),)*}}
  
  fn with_input_changed (&self, old_state: &MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern], changed_index: usize, new_pattern: FlowPattern)->MachineMaterialsState {match self {$(MachineType::$Variant (value) => value.with_input_changed (old_state, change_time, old_input_patterns, changed_index, new_pattern ),)*}}
  fn current_outputs_and_next_change (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->(Inputs <FlowPattern>, Option <(Number, MachineMaterialsState)>) {match self {$(MachineType::$Variant (value) => value.current_outputs_and_next_change (state, input_patterns ),)*}}
}
  
  };
}

machine_type_enum! {
  StandardMachine, Conveyor,
}

pub trait Rotate90 {
  fn rotate_90 (self, facing: Facing)->Self;
}

impl Rotate90 for Vector {
  fn rotate_90 (self, facing: Facing)->Vector {
    match facing {
      0 => self,
      1 => Vector::new (-self[1],  self[0]),
      2 => - self,
      3 => Vector::new ( self[1], -self[0]),
      _=> unreachable!()
    }
  }
}
impl Rotate90 for Facing {
  fn rotate_90 (self, facing: Facing)->Facing {
    (self + facing) % 4
  }
}
impl <T: Rotate90, U: Rotate90> Rotate90 for (T, U) {
  fn rotate_90 (self, facing: Facing)->Self {
    (self.0.rotate_90(facing), self.1.rotate_90(facing))
  }
}


#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct StandardMachineInput {
  pub cost: Number,
  pub relative_location: (Vector, Facing),
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct StandardMachineOutput {
  pub amount: Number,
  pub relative_location: (Vector, Facing),
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct StandardMachine {
  pub name: & 'static str,
  pub inputs: Inputs <StandardMachineInput>,
  pub outputs: Inputs <StandardMachineOutput>,
  pub min_output_cycle_length: Number,
}


#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct Conveyor;

pub fn conveyor()->MachineType {
  MachineType::Conveyor (Conveyor)
}

pub fn splitter()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Splitter",
    inputs: inputs! [StandardMachineInput {cost: 2, relative_location: (Vector::new (0, 0), 0)}],
    outputs: inputs! [
      StandardMachineOutput {amount: 1, relative_location: (Vector::new (0,  1), 1)},
      StandardMachineOutput {amount: 1, relative_location: (Vector::new (0, -1), 3)},
    ],
    min_output_cycle_length: 1,
  })
}

pub fn slow_machine()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Slow machine",
    inputs: inputs! [StandardMachineInput {cost: 1, relative_location: (Vector::new (0, 0), 0)}],
    outputs: inputs! [StandardMachineOutput {amount: 1, relative_location: (Vector::new (1, 0), 0)}],
    min_output_cycle_length: 10,
  })
}

pub fn material_generator()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Material generator",
    inputs: inputs! [],
    outputs: inputs! [StandardMachineOutput {amount: 1, relative_location: (Vector::new (1, 0), 0)}],
    min_output_cycle_length: 1,
  })
}

pub fn consumer()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Consumer",
    inputs: inputs! [StandardMachineInput {cost: 1, relative_location: (Vector::new (0, 0), 0)}],
    outputs: inputs! [],
    min_output_cycle_length: 1,
  })
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
  pub fn empty <M: MachineTypeTrait> (machine: & M, time: Number)->MachineMaterialsState {
    MachineMaterialsState {
      current_output_pattern: Default::default(),
      last_flow_change: time,
      inputs: ArrayVec::from_iter (iter::repeat (Default::default()).take (machine.num_inputs())),
    }
  }
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct MachineMapState {
  pub position: Vector,
  pub facing: Facing,
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

impl MachineTypeTrait for StandardMachine {
  fn name (&self)->& str {self.name}
  fn num_inputs (&self)->usize {self.inputs.len()}
  fn num_outputs (&self)->usize {self.outputs.len()}
  
  fn input_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)> {
    self.inputs.iter().map (| input | {
      let (position, facing) = input.relative_location.rotate_90 (state.facing);
      (position + state.position, facing)
    }).collect()
  }
  fn output_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)> {
    self.outputs.iter().map (| output | {
      let (position, facing) = output.relative_location.rotate_90 (state.facing);
      (position + state.position, facing)
    }).collect()
  }
  
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number> {
    let ideal_rate = self.max_output_rate_with_inputs (input_rates.iter().cloned());
    self.outputs.iter().map (| output | ideal_rate*output.amount).collect()
  }
  fn reduced_input_rates_that_can_still_produce (&self, _input_rates: & [Number], output_rates: & [Number])->Inputs <Number> {
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




impl MachineTypeTrait for Conveyor {
  fn name (&self)->& str {"Conveyor"}
  fn num_inputs (&self)->usize {3}
  fn num_outputs (&self)->usize {1}
  
  fn input_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)> {
    (1..=3).map (| direction | (state.position, direction.rotate_90 (2).rotate_90 (state.facing))).collect()
  }
  fn output_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)> {
    inputs! [(state.position + Vector::new (1, 0).rotate_90 (state.facing), state.facing)]
  }
  
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number> {
    inputs! [input_rates.iter().sum()]
  }
  fn reduced_input_rates_that_can_still_produce (&self, input_rates: & [Number], output_rates: & [Number])->Inputs <Number> {
    let result = input_rates.iter().cloned().collect();
    //let total = input_rates().iter().sum();
    //let excess = max (0, total - RATE_DIVISOR);
    result
  }
  
  fn with_input_changed (&self, old_state: &MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern], _changed_index: usize, _new_pattern: FlowPattern)->MachineMaterialsState {
    let mut new_state = old_state.clone();
    
    // hack – just infer the consumed output from what's given to the next title, by subtracting 1 time
    let mut output_pattern = self.current_outputs_and_next_change(old_state, old_input_patterns).0[0];
    output_pattern.start_time -= 1;
    
    let interval = [old_state.last_flow_change, change_time];
    new_state.inputs [0].storage_before_last_flow_change += old_input_patterns.iter().map (| pattern | pattern.num_disbursed_between (interval)).sum::<Number>() - output_pattern.num_disbursed_between (interval);
    
    new_state
  }
  fn current_outputs_and_next_change (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->(Inputs <FlowPattern>, Option <(Number, MachineMaterialsState)>)
 {
    let mut sorted_input_patterns: Inputs <FlowPattern> = input_patterns.iter().cloned().collect();
    sorted_input_patterns.sort_by_key (| pattern | pattern.start_time);
    let mut last_output_pattern = FlowPattern {start_time: Number::min_value(), rate: 0};
    let mut storage_before = state.inputs [0].storage_before_last_flow_change;
    let mut last_change_time = Number::min_value();
    let mut next_change = None;
    for num_patterns_started in 1..=3 {
      let active_patterns = &sorted_input_patterns [0..num_patterns_started];
      let latest_pattern = & sorted_input_patterns [num_patterns_started - 1];
      if latest_pattern.rate == 0 {continue}
      let change_time = latest_pattern.start_time;
      assert!(change_time >= last_change_time);
      let interval = [last_change_time, change_time];
      let already_disbursed = input_patterns.iter().map (| pattern | pattern.num_disbursed_before (change_time)).sum::<Number>();
      assert_eq!(already_disbursed, active_patterns.iter().map (| pattern | pattern.num_disbursed_before (change_time)).sum::<Number>());
      
      let consumed = last_output_pattern.num_disbursed_between (interval);
      
      storage_before += input_patterns.iter().map (| pattern | pattern.num_disbursed_between (interval)).sum::<Number>() - consumed;
      
      if change_time > state.last_flow_change {
        let mut new_state = state.clone();
        new_state.last_flow_change = change_time;
        new_state.inputs [0].storage_before_last_flow_change = storage_before;
        next_change = Some ((change_time, new_state));
        break
      }
      
      let ideal_rate = min (RATE_DIVISOR, active_patterns.iter().map (| pattern | pattern.rate).sum ());
      let legal_output_start_time = time_from_which_patterns_will_always_disburse_at_least_amount_plus_ideal_rate_in_total (active_patterns.iter().cloned(), already_disbursed - storage_before).unwrap();
      let output_pattern = FlowPattern {start_time: legal_output_start_time, rate: ideal_rate};
      last_output_pattern = output_pattern;
      last_change_time = change_time;
    }
        
    let current_outputs = inputs! [FlowPattern {start_time: last_output_pattern.start_time + 1, rate: last_output_pattern.rate}];
    
    (current_outputs, next_change)
  }
}



#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct StatefulMachine {
  pub machine_type: MachineType,
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
  facing: Facing,
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
