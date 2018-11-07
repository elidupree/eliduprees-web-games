use super::*;

use std::cmp::{min, max};
use std::iter::{self, FromIterator};
use std::ops::Neg;

use nalgebra::Vector2;
use arrayvec::ArrayVec;

pub type Number = i64;
pub const MAX_COMPONENTS: usize = 32;
pub const RATE_DIVISOR: Number = 2*2*2*2*2*2 * 3*3*3 * 5*5;
pub const MAX_MACHINE_INPUTS: usize = 8;
pub const TIME_TO_MOVE_MATERIAL: Number = 3;
pub const MAX_IMPLICIT_OUTPUT_FLOW_CHANGES: usize = 3;
pub type Inputs<T> = ArrayVec <[T; MAX_MACHINE_INPUTS]>;
macro_rules! inputs {
  ($($whatever:tt)*) => {Inputs::from_iter ([$($whatever)*].iter().cloned())};
}
pub type Vector = Vector2 <Number>;
pub type Facing = u8;

pub struct DrawnMachine {
  pub icon: & 'static str,
  pub position: Vector,
  pub size: Vector,
  pub facing: Facing,
}


pub trait MachineTypeTrait: Clone {
  // basic information
  fn name (&self)->& str;
  fn num_inputs (&self)->usize;
  fn num_outputs (&self)->usize;
  
  fn input_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)>;
  fn output_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)>;
  
  fn displayed_storage (&self, map_state: & MachineMapState, materials_state: & MachineMaterialsState, input_patterns: & [FlowPattern], time: Number)->Inputs <(Vector, Number)>;
  fn drawn_machine (&self, map_state: & MachineMapState)->DrawnMachine;
  
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
  fn future_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->Inputs <ArrayVec<[(Number, FlowPattern); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>>;

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
  
  fn displayed_storage (&self, map_state: & MachineMapState, materials_state: & MachineMaterialsState, input_patterns: & [FlowPattern], time: Number)->Inputs <(Vector, Number)> {match self {$(MachineType::$Variant (value) => value.displayed_storage (map_state, materials_state, input_patterns, time ),)*}}
  fn drawn_machine (&self, map_state: & MachineMapState)->DrawnMachine {match self {$(MachineType::$Variant (value) => value.drawn_machine (map_state),)*}}
  
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number> {match self {$(MachineType::$Variant (value) => value.max_output_rates (input_rates ),)*}}
  fn reduced_input_rates_that_can_still_produce (&self, input_rates: & [Number], output_rates: & [Number])->Inputs <Number> {match self {$(MachineType::$Variant (value) => value.reduced_input_rates_that_can_still_produce (input_rates, output_rates ),)*}}
  
  fn with_input_changed (&self, old_state: &MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern], changed_index: usize, new_pattern: FlowPattern)->MachineMaterialsState {match self {$(MachineType::$Variant (value) => value.with_input_changed (old_state, change_time, old_input_patterns, changed_index, new_pattern ),)*}}
  fn future_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->Inputs <ArrayVec<[(Number, FlowPattern); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>> {match self {$(MachineType::$Variant (value) => value.future_output_patterns (state, input_patterns ),)*}}
}
  
  };
}

machine_type_enum! {
  StandardMachine, Conveyor,
}

pub trait Rotate90 {
  fn rotate_90 (self, facing: Facing)->Self;
}

impl <T: ::nalgebra::Scalar + Neg<Output=T>> Rotate90 for Vector2 <T> {
  fn rotate_90 (self, facing: Facing)->Self {
    match facing {
      0 => self,
      1 => Vector2::new (-self[1],  self[0]),
      2 => Vector2::new (-self[0], -self[1]),
      3 => Vector2::new ( self[1], -self[0]),
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
  pub relative_location: (Vector, Facing),
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct StandardMachine {
  pub name: & 'static str,
  pub icon: & 'static str,
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
    name: "Splitter", icon: "splitter",
    inputs: inputs! [StandardMachineInput {cost: 2, relative_location: (Vector::new (0, 0), 0)}],
    outputs: inputs! [
      StandardMachineOutput {relative_location: (Vector::new (0,  1), 1)},
      StandardMachineOutput {relative_location: (Vector::new (0, -1), 3)},
    ],
    min_output_cycle_length: TIME_TO_MOVE_MATERIAL,
  })
}

pub fn slow_machine()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Slow machine", icon: "machine",
    inputs: inputs! [StandardMachineInput {cost: 1, relative_location: (Vector::new (0, 0), 0)}],
    outputs: inputs! [StandardMachineOutput {relative_location: (Vector::new (1, 0), 0)}],
    min_output_cycle_length: 10*TIME_TO_MOVE_MATERIAL,
  })
}

pub fn material_generator()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Material generator", icon: "mine",
    inputs: inputs! [],
    outputs: inputs! [StandardMachineOutput {relative_location: (Vector::new (1, 0), 0)}],
    min_output_cycle_length: TIME_TO_MOVE_MATERIAL,
  })
}

pub fn consumer()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Consumer", icon: "chest",
    inputs: inputs! [StandardMachineInput {cost: 1, relative_location: (Vector::new (0, 0), 0)}],
    outputs: inputs! [],
    min_output_cycle_length: TIME_TO_MOVE_MATERIAL,
  })
}



#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct MachineMaterialsState {
  pub last_flow_change: Number,
  pub input_storage_before_last_flow_change: Inputs <Number>,
  pub retained_output_pattern: FlowPattern,
}

impl MachineMaterialsState {
  pub fn empty <M: MachineTypeTrait> (machine: & M, time: Number)->MachineMaterialsState {
    MachineMaterialsState {
      retained_output_pattern: Default::default(),
      last_flow_change: time,
      input_storage_before_last_flow_change: ArrayVec::from_iter (iter::repeat (0).take (machine.num_inputs())),
    }
  }
  
  pub fn next_legal_output_change_time (&self, latency: Number)->Number {
    match self.retained_output_pattern.last_disbursement_before (self.last_flow_change) {
      None => self.last_flow_change,
      Some (disbursement_time) => {
        let next_time = disbursement_time + latency;
        //assert!(next_time >= self.last_flow_change, "we should only be retaining output if it lingers past the change time);
        max(next_time, self.last_flow_change)
      }
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
    output_rates.into_iter().max().unwrap_or_else(|| self.max_output_rate())
  }
  
  fn input_storage_before_impl (&self, input_patterns: & [FlowPattern], output_pattern: FlowPattern, starting_storage: Inputs <Number>, interval: [Number; 2])->Inputs <Number> {
    let output_disbursements = output_pattern.num_disbursed_between (interval);
    
    self.inputs.iter().zip (starting_storage).zip (input_patterns).map (| ((input, storage), pattern) | {
      let accumulated = pattern.num_disbursed_between (interval);
      let spent = output_disbursements*input.cost;
      storage + accumulated - spent
    }).collect()
  }
  
  pub fn input_storage_before (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern], time: Number)->Inputs <Number> {
    let (start_time, output_pattern, starting_storage) = self.future_internal_output_patterns (state, input_patterns).into_iter().rev().find (| (start_time, _, _) | *start_time < time).unwrap();
    
    self.input_storage_before_impl (input_patterns, output_pattern, starting_storage, [max (start_time, state.last_flow_change), time])
  }
  
  fn update_last_flow_change (&self, state: &mut MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern]) {
    let (start_time, output_pattern, starting_storage) = self.future_internal_output_patterns (state, old_input_patterns).into_iter().rev().find (| (time,_,_) | *time < change_time).unwrap();
    state.input_storage_before_last_flow_change = self.input_storage_before_impl (old_input_patterns, output_pattern, starting_storage, [max (start_time, state.last_flow_change), change_time]);
    state.retained_output_pattern = output_pattern;
    state.last_flow_change = change_time;
  }
  
  fn push_output_pattern_impl (&self, result: &mut ArrayVec<[(Number, FlowPattern, Inputs <Number>); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>, state: &MachineMaterialsState, input_patterns: & [FlowPattern], time: Number, pattern: FlowPattern) {
      let (start_time, output_pattern, starting_storage) = result.last().cloned().unwrap();
      assert!(time >= start_time) ;
      if time == start_time {
        result.pop();
        result.push ((time, pattern, starting_storage));
      }
      else {
        let new_storage = self.input_storage_before_impl (input_patterns, output_pattern, starting_storage, [max (start_time, state.last_flow_change), time]);
        result.push ((time, pattern, new_storage));
      }
    }
  
  fn future_internal_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->ArrayVec<[(Number, FlowPattern, Inputs <Number>); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]> {
    let mut result = ArrayVec::new();
    result.push ((Number::min_value(), state.retained_output_pattern, state.input_storage_before_last_flow_change.clone()));
    
    let time_to_switch_output = state.next_legal_output_change_time (self.min_output_cycle_length);
    self.push_output_pattern_impl (&mut result, state, input_patterns, time_to_switch_output, FlowPattern::default());
    
    let ideal_rate = self.max_output_rate_with_inputs (input_patterns.iter().map (| pattern | pattern.rate()));
    if ideal_rate > 0 {
      let mut when_enough_inputs_to_begin_output = time_to_switch_output;
      let inputs = result.last().unwrap().2.clone();
      for ((pattern, input), storage) in input_patterns.iter().zip (self.inputs.iter()).zip (inputs) {
        let min_start_time = pattern.time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (time_to_switch_output, (input.cost - 1) - storage).unwrap();
        when_enough_inputs_to_begin_output = max (when_enough_inputs_to_begin_output, min_start_time);
      }
      let ideal_output = FlowPattern::new (when_enough_inputs_to_begin_output, ideal_rate);
      self.push_output_pattern_impl (&mut result, state, input_patterns, when_enough_inputs_to_begin_output, ideal_output);
    }
    
    result
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
  
  fn displayed_storage (&self, map_state: & MachineMapState, materials_state: & MachineMaterialsState, input_patterns: & [FlowPattern], time: Number)->Inputs <(Vector, Number)> {
    self.input_storage_before (materials_state, input_patterns, time).into_iter().zip (self.input_locations (map_state)).map (| (amount, (position,_facing)) | (position, amount)).collect()
  }
  fn drawn_machine (&self, map_state: & MachineMapState)->DrawnMachine {
    DrawnMachine {
      icon: self.icon,
      position: map_state.position,
      size: Vector::new (1, 1),
      facing: map_state.facing,
    }
  }
  
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number> {
    let ideal_rate = self.max_output_rate_with_inputs (input_rates.iter().cloned());
    self.outputs.iter().map (| _output | ideal_rate).collect()
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
  
  fn future_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->Inputs <ArrayVec<[(Number, FlowPattern); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>> {
    let internal = self.future_internal_output_patterns (state, input_patterns);
    
    self.outputs.iter().map (| _output | {
      internal.iter().map (| (start, pattern, _storage) | {
        (max (start + TIME_TO_MOVE_MATERIAL, state.last_flow_change), pattern.delayed_by (TIME_TO_MOVE_MATERIAL))
      }).collect()
    }).collect()
  }
}








impl Conveyor {
  fn max_output_rate (&self)->Number {
    RATE_DIVISOR/TIME_TO_MOVE_MATERIAL
  }
  fn max_output_rate_with_inputs <I: IntoIterator <Item = Number>> (&self, input_rates: I)->Number {
    min(self.max_output_rate(), input_rates.into_iter().sum())
  }
  
  fn input_storage_before_impl (&self, input_patterns: & [FlowPattern], output_pattern: FlowPattern, starting_storage: Number, interval: [Number; 2])->Number {
    let output_disbursements = output_pattern.num_disbursed_between (interval);
    let spent = output_disbursements;
    starting_storage - spent + input_patterns.iter().map (| pattern | pattern.num_disbursed_between (interval)).sum::<Number>()
  }
  
  pub fn input_storage_before (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern], time: Number)->Number {
    let (start_time, output_pattern, starting_storage) = self.future_internal_output_patterns (state, input_patterns).into_iter().rev().find (| (start_time, _, _) | *start_time < time).unwrap();
    
    self.input_storage_before_impl (input_patterns, output_pattern, starting_storage, [max (start_time, state.last_flow_change), time])
  }
  
  fn update_last_flow_change (&self, state: &mut MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern]) {
    let (start_time, output_pattern, starting_storage) = self.future_internal_output_patterns (state, old_input_patterns).into_iter().rev().find (| (time,_,_) | *time < change_time).unwrap();
    state.input_storage_before_last_flow_change = inputs! [self.input_storage_before_impl (old_input_patterns, output_pattern, starting_storage, [max (start_time, state.last_flow_change), change_time])];
    state.retained_output_pattern = output_pattern;
    state.last_flow_change = change_time;
  }
  
  fn push_output_pattern_impl (&self, result: &mut ArrayVec<[(Number, FlowPattern, Number); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>, state: &MachineMaterialsState, input_patterns: & [FlowPattern], time: Number, pattern: FlowPattern) {
      let (start_time, output_pattern, starting_storage) = result.last().cloned().unwrap();
      assert!(time >= start_time) ;
      if time == start_time {
        result.pop();
        result.push ((time, pattern, starting_storage));
      }
      else {
        let new_storage = self.input_storage_before_impl (input_patterns, output_pattern, starting_storage, [max (start_time, state.last_flow_change), time]);
        result.push ((time, pattern, new_storage));
      }
    }
  
  fn future_internal_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->ArrayVec<[(Number, FlowPattern, Number); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]> {
    let mut result = ArrayVec::new();
    result.push ((Number::min_value(), state.retained_output_pattern, state.input_storage_before_last_flow_change [0]));
    
    let time_to_switch_output = state.next_legal_output_change_time (TIME_TO_MOVE_MATERIAL);
    self.push_output_pattern_impl (&mut result, state, input_patterns, time_to_switch_output, FlowPattern::default());
    
    let ideal_rate = self.max_output_rate_with_inputs (input_patterns.iter().map (| pattern | pattern.rate()));
    if ideal_rate > 0 {
      let storage_before = result.last().unwrap().2;
      let when_enough_inputs_to_begin_output = time_from_which_patterns_will_always_disburse_at_least_amount_plus_ideal_rate_in_total (input_patterns.iter().cloned(), time_to_switch_output, -storage_before).unwrap();
      
      let ideal_output = FlowPattern::new (when_enough_inputs_to_begin_output, ideal_rate);
      self.push_output_pattern_impl (&mut result, state, input_patterns, max(time_to_switch_output, when_enough_inputs_to_begin_output), ideal_output);
    }
    
    result
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
  
  fn displayed_storage (&self, map_state: & MachineMapState, materials_state: & MachineMaterialsState, input_patterns: & [FlowPattern], time: Number)->Inputs <(Vector, Number)> {
    inputs! [(map_state.position, self.input_storage_before (materials_state, input_patterns, time))]
  }
  fn drawn_machine (&self, map_state: & MachineMapState)->DrawnMachine {
    DrawnMachine {
      icon: "conveyor",
      position: map_state.position,
      size: Vector::new (1, 1),
      facing: map_state.facing,
    }
  }
  
  fn max_output_rates (&self, input_rates: & [Number])->Inputs <Number> {
    inputs! [self.max_output_rate_with_inputs (input_rates.iter().cloned())]
  }
  fn reduced_input_rates_that_can_still_produce (&self, input_rates: & [Number], _output_rates: & [Number])->Inputs <Number> {
    let result = input_rates.iter().cloned().collect();
    //let total = input_rates().iter().sum();
    //let excess = max (0, total - RATE_DIVISOR);
    result
  }
  
    
  fn with_input_changed (&self, old_state: &MachineMaterialsState, change_time: Number, old_input_patterns: & [FlowPattern], _changed_index: usize, _new_pattern: FlowPattern)->MachineMaterialsState {
    let mut new_state = old_state.clone();
    self.update_last_flow_change (&mut new_state, change_time, old_input_patterns);    
    new_state
  }
  
  fn future_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [FlowPattern])->Inputs <ArrayVec<[(Number, FlowPattern); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>> {
    let internal = self.future_internal_output_patterns (state, input_patterns);
    
    inputs! [
      internal.iter().map (| (start, pattern, _storage) | {
        (max (start + TIME_TO_MOVE_MATERIAL, state.last_flow_change), pattern.delayed_by (TIME_TO_MOVE_MATERIAL))
      }).collect()
    ]
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
