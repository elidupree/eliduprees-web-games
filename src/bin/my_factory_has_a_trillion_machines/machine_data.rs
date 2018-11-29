use super::*;

use std::cmp::{min, max};
use std::iter::{self, FromIterator};
use std::ops::Neg;
use std::collections::HashMap;
use std::ops::{Mul, Div};

use nalgebra::Vector2;
use arrayvec::ArrayVec;

pub type Number = i64;
pub const MAX_COMPONENTS: usize = 256;
pub const RATE_DIVISOR: Number = TIME_TO_MOVE_MATERIAL * 2*2*2*2*2*2 * 3*3*3 * 5*5;
pub const MAX_MACHINE_INPUTS: usize = 8;
pub const TIME_TO_MOVE_MATERIAL: Number = 60;
pub const MAX_IMPLICIT_OUTPUT_FLOW_CHANGES: usize = 3;
pub type Inputs<T> = ArrayVec <[T; MAX_MACHINE_INPUTS]>;
macro_rules! inputs {
  ($($whatever:tt)*) => {::std::iter::FromIterator::from_iter ([$($whatever)*].iter().cloned())};
}
pub type Vector = Vector2 <Number>;
pub type Facing = u8;

#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub struct GridIsomorphism {
  #[derivative (Default (value = "Vector::new(0,0)"))]
  pub translation: Vector,
  pub rotation: Facing,
  pub flip: bool,
}

pub trait TransformedBy {
  fn transformed_by (self, isomorphism: GridIsomorphism)->Self;
}
impl TransformedBy for Vector { 
  fn transformed_by (mut self, isomorphism: GridIsomorphism)->Self {
    if isomorphism.flip { self[0] *= -1; }
    self = self.rotate_90 (isomorphism.rotation);
    self + isomorphism.translation
  }
}
impl TransformedBy for Facing { 
  fn transformed_by (mut self, isomorphism: GridIsomorphism)->Self {
    if isomorphism.flip { self = (4-self) % 4 }
    self = self.rotate_90 (isomorphism.rotation);
    self
  }
}
impl <T: TransformedBy, U: TransformedBy> TransformedBy for (T, U) {
  fn transformed_by (self, isomorphism: GridIsomorphism)->Self {
    (self.0.transformed_by(isomorphism), self.1.transformed_by(isomorphism))
  }
}
impl <T: TransformedBy> TransformedBy for Option<T> {
  fn transformed_by (self, isomorphism: GridIsomorphism)->Self {
    self.map(|t| t.transformed_by(isomorphism))
  }
}
impl Mul<GridIsomorphism> for GridIsomorphism {
  type Output = GridIsomorphism; 
  fn mul (mut self, other: GridIsomorphism)->GridIsomorphism {
    if other.flip {
      self.translation[0] *= -1;
      self.rotation = (4-self.rotation) % 4;
      self.flip = !self.flip;
    }
    self.translation = self.translation.rotate_90 (other.rotation);
    self.rotation = self.rotation.rotate_90 (other.rotation);
    self.translation += other.translation;
    self
  }
}
impl Div<GridIsomorphism> for GridIsomorphism {
  type Output = GridIsomorphism; 
  fn div (self, other: GridIsomorphism)->GridIsomorphism {
    self * other.inverse()
  }
}
impl GridIsomorphism {
  pub fn inverse(mut self)->GridIsomorphism {
    self.translation = (-self.translation).rotate_90 ((4-self.rotation) % 4);
    if self.flip {
      self.translation[0] *= -1;
    }
    else {
      self.rotation = (4-self.rotation) % 4;
    }
    self
  }
}

#[cfg(test)]
mod tests {
use super::*;
use proptest::prelude::*;

#[test]
fn grid_isomorphism_unit_tests() {
  let isomorphism = GridIsomorphism {
    translation: Vector::new (5, 4),
    rotation: 1,
    flip: true
  };
  let vector = Vector::new (2, 1) ;
  
  assert_eq!(vector.transformed_by (isomorphism), Vector::new (4, 2));
  
}

fn arbitrary_vector() -> BoxedStrategy<Vector> {
  (
    -1000000i64..1000000i64,
    -1000000i64..1000000i64
  ).prop_map(|(x, y)| {
    Vector::new (x,y)
  }).boxed()
}
/*
prop_compose! {
  fn arbitrary_vector()(
    x in -1000000i64..1000000i64,
    y in -1000000i64..1000000i64
  )->Vector {
    Vector::new (x,y)
  }
}*/
prop_compose! {
  fn arbitrary_isomorphism()(
    translation in arbitrary_vector(),
    rotation in 0u8..4u8,
    flip in any::<bool>()
  )->GridIsomorphism {
    GridIsomorphism {
      translation, rotation, flip,
    }
  }
}

proptest! {
  
  #[test]
  fn randomly_test_grid_isomorphism_inverse_repetition (isomorphism in arbitrary_isomorphism()) {
    prop_assert_eq! (isomorphism, isomorphism.inverse().inverse());
  }
  #[test]
  fn randomly_test_grid_isomorphism_inverse_is_inverse (isomorphism in arbitrary_isomorphism()) {
    prop_assert_eq! (isomorphism * isomorphism.inverse(), GridIsomorphism::default());
    prop_assert_eq! (isomorphism.inverse() * isomorphism, GridIsomorphism::default());
  }
  #[test]
  fn randomly_test_grid_isomorphism_identity_is_identity (isomorphism in arbitrary_isomorphism()) {
    prop_assert_eq! (isomorphism * GridIsomorphism::default(), isomorphism);
    prop_assert_eq! (GridIsomorphism::default() * isomorphism, isomorphism);
  }
  #[test]
  fn randomly_test_grid_isomorphism_inverse_vector (isomorphism in arbitrary_isomorphism(), vector in arbitrary_vector()) {
    prop_assert_eq! (vector, vector.transformed_by(isomorphism).transformed_by(isomorphism.inverse()));
  }
  #[test]
  fn randomly_test_grid_isomorphism_inverse_facing (isomorphism in arbitrary_isomorphism(), facing in 0u8..4u8) {
    prop_assert_eq! (facing, facing.transformed_by(isomorphism).transformed_by(isomorphism.inverse()));
  }
  #[test]
  fn randomly_test_grid_isomorphism_is_flip_then_rotate_then_translate (isomorphism in arbitrary_isomorphism()) {
    prop_assert_eq! (isomorphism,
      GridIsomorphism {flip: isomorphism.flip, ..Default::default()}
      * GridIsomorphism {rotation: isomorphism.rotation, ..Default::default()}
      * GridIsomorphism {translation: isomorphism.translation, ..Default::default()}
    );
  }
  #[test]
  fn randomly_test_grid_isomorphism_associative (first in arbitrary_isomorphism(), second in arbitrary_isomorphism(), third in arbitrary_isomorphism()) {
    prop_assert_eq! ((first * second) * third, first * (second * third));
  }
  #[test]
  fn randomly_test_grid_isomorphism_transforms_translation_like_vector (isomorphism in arbitrary_isomorphism(), transformed_isomorphism in arbitrary_isomorphism()) {
    prop_assert_eq! ((transformed_isomorphism * isomorphism).translation, transformed_isomorphism.translation.transformed_by(isomorphism));
  }
}

}

pub struct DrawnMachine {
  pub icon: String,
  pub position: GridIsomorphism,
  pub size: Vector,
}

#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
#[derive (Derivative)]
#[derivative (Default)]
pub enum Material {
  IronOre,
  Iron,
  #[derivative (Default)]
  Garbage,
}
/*
#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct Material {
  material_type: MaterialType,
}*/

impl Material {
  pub fn icon (self)->& 'static str {
    match self {
      Material::IronOre => "ore",
      Material::Iron => "iron",
      Material::Garbage => "machine",
    }
  }
}

#[allow(unused)]
pub trait MachineTypeTrait {
  // basic information
  fn name (&self)->& str;
  fn cost (&self)->Vec<(Number, Material)> {vec![]}
  fn num_inputs (&self)->usize {0}
  fn num_outputs (&self)->usize {0}
  fn radius (&self)->Number {1}
  
  fn input_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)> {inputs![]}
  fn output_locations (&self, state: &MachineMapState)->Inputs <(Vector, Option<Facing>)> {inputs![]}
  fn input_materials (&self)->Inputs <Option <Material>> {inputs![]}
  
  fn displayed_storage (&self, map_state: & MachineMapState, materials_state: & MachineMaterialsState, input_patterns: & [(FlowPattern, Material)], time: Number)->Inputs <(Vector, (Number, Material))> {inputs![]}
  fn drawn_machine (&self, map_state: & MachineMapState)->DrawnMachine;
  
  // used to infer group input flow rates
  // property: with valid inputs, the returned values have the same length given by num_inputs/num_outputs
  // property: these are consistent with each other
  fn max_output_rates (&self, input_rates: & [(Number, Material)])->Inputs <(Number, Material)> {inputs![]}
  fn reduced_input_rates_that_can_still_produce (&self, input_rates: & [(Number, Material)], output_rates: & [(Number, Material)])->Inputs <(Number, Material)> {inputs![]}
  
  // property: if inputs don't change, current_output_rates doesn't change before next_output_change_time
  // property: when there is no next output change time, current_output_rates is equivalent to max_output_rates
  // maybe some property that limits the total amount of rate changes resulting from a single change by the player?
  fn with_inputs_changed (&self, old_state: &MachineMaterialsState, change_time: Number, old_input_patterns: & [(FlowPattern, Material)])->MachineMaterialsState {MachineMaterialsState {last_flow_change: change_time, ..old_state.clone()}}
  // property: next_change is not the same time twice in a row
  fn future_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [(FlowPattern, Material)])->Inputs <ArrayVec<[(Number, (FlowPattern, Material)); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>> {inputs![ArrayVec::new()]}

}

macro_rules! machine_type_enum {
  ($($Variant: ident,)*) => {
  

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum MachineType {
  $($Variant ($Variant),)*
}

impl MachineTypeTrait for MachineType {
  fn name (&self)->& str {match self {$(MachineType::$Variant (value) => value.name (),)*}}
  fn cost (&self)->Vec<(Number, Material)> {match self {$(MachineType::$Variant (value) => value.cost (),)*}}
  fn num_inputs (&self)->usize {match self {$(MachineType::$Variant (value) => value.num_inputs(),)*}}
  fn num_outputs (&self)->usize {match self {$(MachineType::$Variant (value) => value.num_outputs(),)*}}
  fn radius (&self)->Number {match self {$(MachineType::$Variant (value) => value.radius (),)*}}
  
  fn input_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)> {match self {$(MachineType::$Variant (value) => value.input_locations (state ),)*}}
  fn output_locations (&self, state: &MachineMapState)->Inputs <(Vector, Option<Facing>)> {match self {$(MachineType::$Variant (value) => value.output_locations (state ),)*}}
  fn input_materials (&self)->Inputs <Option <Material>> {match self {$(MachineType::$Variant (value) => value.input_materials (),)*}}
  
  fn displayed_storage (&self, map_state: & MachineMapState, materials_state: & MachineMaterialsState, input_patterns: & [(FlowPattern, Material)], time: Number)->Inputs <(Vector, (Number, Material))> {match self {$(MachineType::$Variant (value) => value.displayed_storage (map_state, materials_state, input_patterns, time ),)*}}
  fn drawn_machine (&self, map_state: & MachineMapState)->DrawnMachine {match self {$(MachineType::$Variant (value) => value.drawn_machine (map_state),)*}}
  
  fn max_output_rates (&self, input_rates: & [(Number, Material)])->Inputs <(Number, Material)> {match self {$(MachineType::$Variant (value) => value.max_output_rates (input_rates ),)*}}
  fn reduced_input_rates_that_can_still_produce (&self, input_rates: & [(Number, Material)], output_rates: & [(Number, Material)])->Inputs <(Number, Material)> {match self {$(MachineType::$Variant (value) => value.reduced_input_rates_that_can_still_produce (input_rates, output_rates ),)*}}
  
  fn with_inputs_changed (&self, old_state: &MachineMaterialsState, change_time: Number, old_input_patterns: & [(FlowPattern, Material)])->MachineMaterialsState {match self {$(MachineType::$Variant (value) => value.with_inputs_changed (old_state, change_time, old_input_patterns),)*}}
  fn future_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [(FlowPattern, Material)])->Inputs <ArrayVec<[(Number, (FlowPattern, Material)); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>> {match self {$(MachineType::$Variant (value) => value.future_output_patterns (state, input_patterns ),)*}}
}
  
  };
}

machine_type_enum! {
  StandardMachine, ModuleMachine, // Conveyor,
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
impl <T: Rotate90> Rotate90 for Option<T> {
  fn rotate_90 (self, facing: Facing)->Self {
    self.map(|t| t.rotate_90(facing))
  }
}


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct StandardMachineInput {
  pub material: Option <Material>,
  pub cost: Number,
  pub relative_location: (Vector, Facing),
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct StandardMachineOutput {
  pub material: Option <Material>,
  pub relative_location: (Vector, Option<Facing>),
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct StandardMachine {
  pub name: String,
  pub icon: String,
  pub radius: Number,
  pub cost: Vec<(Number, Material)>,
  pub inputs: Inputs <StandardMachineInput>,
  pub outputs: Inputs <StandardMachineOutput>,
  pub merge_inputs: bool,
  pub min_output_cycle_length: Number,
}


//#[derive (Clone, PartialEq, Eq, Hash, Debug)]
//pub struct Conveyor;

pub fn conveyor()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Conveyor".to_string(), icon: "conveyor".to_string(),
    cost: vec![(1, Material::Iron)],
    radius: 1,
    inputs: inputs! [
      StandardMachineInput {cost: 1, material: None, relative_location: (Vector::new (0, 0), 0)},
      StandardMachineInput {cost: 1, material: None, relative_location: (Vector::new (0, 0), 1)},
      StandardMachineInput {cost: 1, material: None, relative_location: (Vector::new (0, 0), 3)},
    ],
    outputs: inputs! [StandardMachineOutput {material: None, relative_location: (Vector::new (2,  0), Some(0))}],
    merge_inputs: true,
    min_output_cycle_length: TIME_TO_MOVE_MATERIAL,
  })
}

pub fn splitter()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Splitter".to_string(), icon: "splitter".to_string(),
    cost: vec![(1, Material::Iron)],
    radius: 1,
    inputs: inputs! [StandardMachineInput {cost: 2, material: None, relative_location: (Vector::new (0, 0), 0)}],
    outputs: inputs! [
      StandardMachineOutput {material: None, relative_location: (Vector::new (0,  2), Some(1))},
      StandardMachineOutput {material: None, relative_location: (Vector::new (0, -2), Some(3))},
    ],
    merge_inputs: true,
    min_output_cycle_length: TIME_TO_MOVE_MATERIAL,
  })
}

pub fn iron_smelter()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    cost: vec![(5, Material::Iron)],
    name: "Iron smelter".to_string(), icon: "machine".to_string(),
    radius: 3,
    inputs: inputs! [StandardMachineInput {cost: 1, material: Some(Material::IronOre), relative_location: (Vector::new (-2, 0), 0)}],
    outputs: inputs! [StandardMachineOutput {material: Some(Material::Iron), relative_location: (Vector::new (4, 0), Some(0))}],
    merge_inputs: false,
    min_output_cycle_length: 10*TIME_TO_MOVE_MATERIAL,
  })
}

pub fn material_generator()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Iron mine".to_string(), icon: "mine".to_string(),
    cost: vec![(50, Material::Iron)],
    radius: 3,
    inputs: inputs! [],
    outputs: inputs! [StandardMachineOutput {material: Some(Material::IronOre), relative_location: (Vector::new (4, 0), Some(0))}],
    merge_inputs: false,
    min_output_cycle_length: TIME_TO_MOVE_MATERIAL,
  })
}

pub fn consumer()->MachineType {
  MachineType::StandardMachine (StandardMachine {
    name: "Consumer".to_string(), icon: "chest".to_string(),
    cost: vec![(5, Material::Iron)],
    radius: 1,
    inputs: inputs! [
      StandardMachineInput {cost: 1, material: None, relative_location: (Vector::new (0, 0), 3)},
    ],
    outputs: inputs! [StandardMachineOutput {material: None, relative_location: (Vector::new (0,  0), None)}],
    merge_inputs: true,
    min_output_cycle_length: TIME_TO_MOVE_MATERIAL,
  })
}



#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct MachineMaterialsState {
  pub last_flow_change: Number,
  pub input_storage_before_last_flow_change: Inputs <(Number, Material)>,
  pub retained_output_pattern: FlowPattern,
}

impl MachineMaterialsState {
  pub fn empty <M: MachineTypeTrait> (machine: & M, time: Number)->MachineMaterialsState {
    MachineMaterialsState {
      retained_output_pattern: Default::default(),
      last_flow_change: time,
      input_storage_before_last_flow_change: ArrayVec::from_iter (iter::repeat ((0, Material::default())).take (machine.num_inputs())),
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


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct MachineMapState {
  pub position: GridIsomorphism,
}


pub fn only_value<I: Iterator>(mut iterator: I)->Option <I::Item> where I::Item: PartialEq<I::Item> { iterator.next().filter (|a| iterator.all (|b| *a==b)) }

impl StandardMachine {
  fn max_output_rate (&self)->Number {
    RATE_DIVISOR/self.min_output_cycle_length
  }
  fn max_output_rate_with_inputs <I: IntoIterator <Item = (Number, Material)>> (&self, input_rates: I)->Number {
    let mut ideal_rate = if self.merge_inputs {0} else {self.max_output_rate()};
    for ((rate, material), input) in input_rates.into_iter().zip(&self.inputs) {
      let allowed_material = input.material.unwrap_or (material) == material;
      let inferred_rate = if allowed_material {rate/input.cost} else {0};
      if self.merge_inputs {
        ideal_rate += inferred_rate;
      }
      else {
        ideal_rate = min (ideal_rate, inferred_rate);
      }
    }
    min(self.max_output_rate(), ideal_rate)
  }
  fn min_output_rate_to_produce <I: IntoIterator <Item = Number>> (&self, output_rates: I)->Number {
    output_rates.into_iter().max().unwrap_or_else(|| self.max_output_rate())
  }
  
  fn input_storage_before_impl (&self, input_patterns: & [(FlowPattern, Material)], output_pattern: FlowPattern, starting_storage: & [(Number, Material)], interval: [Number; 2])->Inputs <(Number, Material)> {
    let output_disbursements = output_pattern.num_disbursed_between (interval);
    if self.merge_inputs {
      let (storage, mut material) = starting_storage[0];
      let input_materials = input_patterns.iter().filter_map (| (pattern, material) | if pattern.rate() >0 {Some (material)} else {None});
      let only_input_material = only_value (input_materials);
      let mut used_storage = storage;
      let mut amount = 0;
      for (pattern, pattern_material) in input_patterns.iter() {
        let disbursed = pattern.num_disbursed_between (interval);
        amount += disbursed;
        if disbursed > 0 && *pattern_material != material {
          if only_input_material == Some(pattern_material) {
            used_storage = 0;
            material = *pattern_material;
          }
          else {
            material = Material::Garbage;
          }
        }
      }
      inputs! [(used_storage + amount - output_disbursements*self.inputs[0].cost, material)]
    }
    else {
      self.inputs.iter().zip (starting_storage).zip (input_patterns).map (| ((input, (storage_amount, storage_material)), (pattern, pattern_material)) | {
        let allowed_material = input.material.unwrap_or (*pattern_material) == *pattern_material;
        if allowed_material && pattern.rate() > 0 {
          if *storage_amount > 0 {
            assert_eq!(storage_material, pattern_material);
          }
          let accumulated = pattern.num_disbursed_between (interval);
          let spent = output_disbursements*input.cost;
          (storage_amount + accumulated - spent, *pattern_material)
        }
        else {
          (*storage_amount, *storage_material)
        }
      }).collect()
    }
  }
  
  pub fn input_storage_before (&self, state: &MachineMaterialsState, input_patterns: & [(FlowPattern, Material)], time: Number)->Inputs <(Number, Material)> {
    let (start_time, output_pattern, starting_storage) = self.future_internal_output_patterns (state, input_patterns).into_iter().rev().find (| (start_time, _, _) | *start_time < time).unwrap();
    
    self.input_storage_before_impl (input_patterns, output_pattern, &starting_storage, [max (start_time, state.last_flow_change), time])
  }
  
  fn update_last_flow_change (&self, state: &mut MachineMaterialsState, change_time: Number, old_input_patterns: & [(FlowPattern, Material)]) {
    let (start_time, output_pattern, starting_storage) = self.future_internal_output_patterns (state, old_input_patterns).into_iter().rev().find (| (time,_,_) | *time < change_time).unwrap();
    state.input_storage_before_last_flow_change = self.input_storage_before_impl (old_input_patterns, output_pattern, &starting_storage, [max (start_time, state.last_flow_change), change_time]);
    state.retained_output_pattern = output_pattern;
    state.last_flow_change = change_time;
  }
  
  fn push_output_pattern_impl (&self, result: &mut ArrayVec<[(Number, FlowPattern, Inputs <(Number, Material)>); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>, state: &MachineMaterialsState, input_patterns: & [(FlowPattern, Material)], time: Number, pattern: FlowPattern) {
    let (start_time, output_pattern, starting_storage) = result.last().cloned().unwrap();
    assert!(time >= start_time) ;
    if time == start_time {
      result.pop();
      result.push ((time, pattern, starting_storage));
    }
    else {
      let new_storage = self.input_storage_before_impl (input_patterns, output_pattern, &starting_storage, [max (start_time, state.last_flow_change), time]);
      result.push ((time, pattern, new_storage));
    }
  }
  
  fn future_internal_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [(FlowPattern, Material)])->ArrayVec<[(Number, FlowPattern, Inputs <(Number, Material)>); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]> {
    let mut result = ArrayVec::new();
    result.push ((Number::min_value(), state.retained_output_pattern, state.input_storage_before_last_flow_change.clone()));
    
    let time_to_switch_output = state.next_legal_output_change_time (self.min_output_cycle_length);
    self.push_output_pattern_impl (&mut result, state, input_patterns, time_to_switch_output, FlowPattern::default());
    
    let ideal_rate = self.max_output_rate_with_inputs (input_patterns.iter().map (| (pattern, material) | (pattern.rate(), *material)));
    if ideal_rate > 0 {
      let mut when_enough_inputs_to_begin_output = time_to_switch_output;
      let storage_before = result.last().unwrap().2.clone();
      if self.merge_inputs {
        when_enough_inputs_to_begin_output = max(when_enough_inputs_to_begin_output, time_from_which_patterns_will_always_disburse_at_least_amount_plus_ideal_rate_in_total (input_patterns.iter().map (| (pattern, _material) | *pattern), time_to_switch_output, (self.inputs[0].cost - 1) - storage_before[0].0).unwrap());
      }
      else {
        for (((pattern, _material), input), (storage_amount, _storage_material)) in input_patterns.iter().zip (self.inputs.iter()).zip (storage_before) {
          let min_start_time = pattern.time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (time_to_switch_output, (input.cost - 1) - storage_amount).unwrap();
          when_enough_inputs_to_begin_output = max (when_enough_inputs_to_begin_output, min_start_time);
        }
      }
      let ideal_output = FlowPattern::new (when_enough_inputs_to_begin_output, ideal_rate);
      self.push_output_pattern_impl (&mut result, state, input_patterns, when_enough_inputs_to_begin_output, ideal_output);
    }
    
    result
  }
}

impl MachineTypeTrait for StandardMachine {
  fn name (&self)->& str {&self.name}
  fn cost (&self)->Vec<(Number, Material)> {self.cost.clone()}
  fn num_inputs (&self)->usize {self.inputs.len()}
  fn num_outputs (&self)->usize {self.outputs.len()}
  fn radius (&self)->Number {self.radius}
  
  fn input_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)> {
    self.inputs.iter().map (| input | {
      input.relative_location.transformed_by (state.position)
    }).collect()
  }
  fn output_locations (&self, state: &MachineMapState)->Inputs <(Vector, Option<Facing>)> {
    self.outputs.iter().map (| output | {
      output.relative_location.transformed_by (state.position)
    }).collect()
  }
  
  fn input_materials (&self)->Inputs <Option <Material>> {
    self.inputs.iter().map (| input | input.material).collect()
  }
  
  fn displayed_storage (&self, map_state: & MachineMapState, materials_state: & MachineMaterialsState, input_patterns: & [(FlowPattern, Material)], time: Number)->Inputs <(Vector, (Number, Material))> {
    self.input_storage_before (materials_state, input_patterns, time).into_iter().zip (self.input_locations (map_state)).map (| ((amount, material), (position,_facing)) | (position, (amount, material))).collect()
  }
  fn drawn_machine (&self, map_state: & MachineMapState)->DrawnMachine {
    DrawnMachine {
      icon: self.icon.clone(),
      position: map_state.position,
      size: Vector::new (self.radius*2, self.radius*2),
    }
  }
  
  fn max_output_rates (&self, input_rates: & [(Number, Material)])->Inputs <(Number, Material)> {
    let input_materials = input_rates.iter().filter_map (| (rate, material) | if *rate > 0 {Some (*material)} else {None});
    let merged_material = only_value (input_materials).unwrap_or (Material::Garbage) ;
    let ideal_rate = self.max_output_rate_with_inputs (input_rates.iter().cloned());
    self.outputs.iter().map (| output | (ideal_rate, output.material.unwrap_or (merged_material))).collect()
  }
  fn reduced_input_rates_that_can_still_produce (&self, input_rates: & [(Number, Material)], output_rates: & [(Number, Material)])->Inputs <(Number, Material)> {
    let ideal_rate = self.min_output_rate_to_produce (output_rates.iter().map(|(rate, _material)| *rate));
    if self.merge_inputs {
      // TODO better
      //let total = input_rates().iter().sum();
      //let excess = max (0, total - RATE_DIVISOR);
      input_rates.iter().cloned().collect()
    }
    else {
      self.inputs.iter().zip(input_rates).map (| (input, (_rate, material)) | (ideal_rate*input.cost, *material)).collect()
    }
  }
  
  fn with_inputs_changed (&self, old_state: &MachineMaterialsState, change_time: Number, old_input_patterns: & [(FlowPattern, Material)])->MachineMaterialsState {
    let mut new_state = old_state.clone();
    self.update_last_flow_change (&mut new_state, change_time, old_input_patterns);    
    new_state
  }
  
  fn future_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [(FlowPattern, Material)])->Inputs <ArrayVec<[(Number, (FlowPattern, Material)); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>> {
    let internal = self.future_internal_output_patterns (state, input_patterns);
    let storage_after: Inputs<_> = internal.iter().map (| (start, pattern, storage) | {
      if *start < state.last_flow_change {
        state.input_storage_before_last_flow_change.clone()
      }
      else {
        self.input_storage_before_impl (input_patterns, *pattern, storage, [*start, start + 1])
      }
    }).collect();
    
    self.outputs.iter().map (| output | {
      internal.iter().zip(&storage_after).map (| ((start, pattern, _storage_before), storage_after) | {
        (max (start + TIME_TO_MOVE_MATERIAL, state.last_flow_change), (pattern.delayed_by (TIME_TO_MOVE_MATERIAL), output.material.unwrap_or_else(|| storage_after[0].1)))
      }).collect()
    }).collect()
  }
}




#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
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

#[derive (Serialize, Deserialize)]
pub struct Map {
  pub machines: ArrayVec <[StatefulMachine; MAX_COMPONENTS]>,
  pub last_change_time: Number,
  pub inventory_before_last_change: HashMap <Material, Number>,
}
