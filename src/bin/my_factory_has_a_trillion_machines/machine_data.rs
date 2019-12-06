use std::cmp::{min, max};
use std::iter::{self, FromIterator};
use std::collections::HashMap;

use arrayvec::ArrayVec;


use geometry::{Number, Vector, Facing, GridIsomorphism, TransformedBy};
use flow_pattern::{self, FlowPattern, RATE_DIVISOR};
use modules::ModuleMachine;

pub const MAX_COMPONENTS: usize = 256;
pub const MAX_MACHINE_INPUTS: usize = 8;
pub const TIME_TO_MOVE_MATERIAL: Number = 60;
pub const MAX_IMPLICIT_OUTPUT_FLOW_CHANGES: usize = 3;
pub type Inputs<T> = ArrayVec <[T; MAX_MACHINE_INPUTS]>;
macro_rules! inputs {
  ($($whatever:tt)*) => {::std::iter::FromIterator::from_iter ([$($whatever)*].iter().cloned())};
}

pub struct MachineObservedInputs <'a> {
  input_flows: & 'a [Option<MaterialFlow>],
  start_time: Number,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub enum MachineOperatingState {
  #[derivative (Default)]
  Operating,
  WaitingForInput,
  InputMissing,
  InputTooInfrequent,
  InputIncompatible,
  InCycle,
}

pub struct MachineMomentaryVisuals {
  pub operating_state: MachineOperatingState,
  pub materials: Vec<(Vector2<f64>, Material)>,
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
  fn cost (&self)->& [(Number, Material)] {&[]}
  fn num_inputs (&self)->usize {0}
  fn num_outputs (&self)->usize {0}
  fn radius (&self)->Number {1}
  fn icon(&self) ->& str {""}
  
  fn input_locations (&self)->Inputs <InputLocation> {inputs![]}
  fn output_locations (&self)->Inputs <InputLocation> {inputs![]}
  fn input_materials (&self)->Inputs <Option <Material>> {inputs![]}
  
  fn output_flows(&self, inputs: MachineObservedInputs)->Inputs <Option<MaterialFlow>> {inputs![]}
  fn momentary_visuals(&self, inputs: MachineObservedInputs, time: Number)->MachineMomentaryVisuals {MachineMomentaryVisuals {materials: Vec::new(), operating_state: MachineOperatingState::Operating}}
}

macro_rules! machine_type_enum {
  ($($Variant: ident,)*) => {
  

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum MachineType {
  $($Variant ($Variant),)*
}

impl Deref for MachineType {
  type Target = dyn MachineTypeTrait;
  fn deref(&self)-> &dyn MachineTypeTrait {
    match self {
      $(MachineType::$Variant (value) => value,)*
    }
  }
}
  
  };
}

machine_type_enum! {
  Distributor, Assembler, Mine, ModuleMachine, // Conveyor,
}

#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct InputLocation {
  pub position: Vector,
  pub facing: Facing,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct AssemblerInput {
  pub material: Material,
  pub cost: Number,
  pub location: InputLocation,
}


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct AssemblerOutput {
  pub material: Material,
  pub amount: Number,
  pub location: InputLocation,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct StandardMachineInfo {
  pub name: String,
  pub icon: String,
  pub radius: Number,
  pub cost: Vec<(Number, Material)>,
  pub min_output_cycle_length: Number,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct Assembler {
  pub info: StandardMachineInfo,
  pub inputs: Inputs <AssemblerInput>,
  pub outputs: Inputs <AssemblerOutput>,
  pub assembly_duration: Number,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct Distributor {
  pub info: StandardMachineInfo,
  pub inputs: Inputs <InputLocation>,
  pub outputs: Inputs <InputLocation>,
}


//#[derive (Clone, PartialEq, Eq, Hash, Debug)]
//pub struct Conveyor;
impl StandardMachineInfo {
  pub fn new (name: impl Into<String>, icon: impl Into<String>, radius: Number, cost: Vec<(Number, Material)>)->StandardMachineInfo {StandardMachineInfo {
    name: name.into(), icon: icon.into(), radius, cost
  }}
}

impl InputLocation {
  pub fn new (x: Number, y: Number, facing: Facing)->InputLocation {InputLocation {position: Vec::new (x,y), facing}}
}
impl AssemblerInput {
  pub fn new (x: Number, y: Number, facing: Facing, material: Material, cost: Number)->AssemblerInput {
    AssemblerInput {
      location: InputLocation (x,y, facing), material, cost
    }
  }
}
impl AssemblerOutput {
  pub fn new (x: Number, y: Number, facing: Facing, material: Material, amount: Number)->AssemblerOutput {
    AssemblerOutput {
      location: InputLocation (x,y, facing), material, amount
    }
  }
}

pub fn conveyor()->MachineType {
  MachineType::Distributor(Distributor{
    info: StandardMachineInfo::new ("Conveyor", "conveyor", 1, vec![(1, Material::Iron)]),
    inputs: inputs! [
      InputLocation::new (-1, 0, 0),
      InputLocation::new (0, -1, 1),
      InputLocation::new (0, 1, 3),
    ],
    outputs: inputs! [
      InputLocation::new (1, 0, 0),
    ],
  })
}

pub fn splitter()->MachineType {
  MachineType::Distributor(Distributor{
    info: StandardMachineInfo::new ("Splitter", "splitter", 1, vec![(1, Material::Iron)]),
    inputs: inputs! [
      InputLocation::new (-1, 0, 0),
    ],
    outputs: inputs! [
      InputLocation::new (1, 0, 1),
      InputLocation::new (-1, 0, 3),
    ],
  })
}

pub fn iron_smelter()->MachineType {
  MachineType::Assembler (Assembler {
    info: StandardMachineInfo::new ("Iron smelter", "machine", 3, vec![(5, Material::Iron)]),
    inputs: inputs! [
      AssemblerInput::new (-3, 0, 0, Material::IronOre, 10),
    ],
    outputs: inputs! [
      AssemblerInput::new (3, 0, 0, Material::Iron, 3),
    ],
    assembly_duration: 10*TIME_TO_MOVE_MATERIAL,
  })
}

pub fn iron_mine()->MachineType {
  MachineType::Assembler (Assembler {
    info: StandardMachineInfo::new ("Iron mine", "mine", 3, vec![(50, Material::Iron)]),
    inputs: inputs! [],
    outputs: inputs! [
      AssemblerInput::new (3, 0, 0, Material::IronOre, 1),
    ],
    assembly_duration: TIME_TO_MOVE_MATERIAL,
  })
}



#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct MachineState {
  pub position: GridIsomorphism,
  pub last_disturbed_time: Number,
}



struct DistributorFutureInfo {
  Failure (MachineOperatingState),
  Success (DistributorSuccessInfo),
}

struct DistributorSuccessInfo {
  outputs: Inputs <FlowPattern>,
  material: Material,
}



impl Distributor {
  fn future_info (&self, inputs: MachineObservedInputs)->DistributorFutureInfo {
    let material_iterator = inputs.input_flows().iter().flatten().map (| material_flow | material_flow.material);
    let material = match material_iter.next() {
      None => return DistributorFutureInfo::Failure (InputMissing),
      Some (material) => if material_iterator.all(| second | second == material) {
        material
      } else {return DistributorFutureInfo::Failure (InputIncompatible)}
    };
    
    
    let total_input_rate = inputs.input_flows.iter().flatten().map (| material_flow | material_flow.flow.rate()).sum();
    
    let per_output_rate = min (RATE_DIVISOR, total_input_rate/Number::from (self.outputs.len()));
    if per_output_rate == 0 {
      return DistributorFutureInfo::Failure (InputTooInfrequent)
    }
    let total_output_rate = per_output_rate*self.outputs.len();
    // the rounding here could theoretically be better, but this should be okay
    let denominator = total_output_rate*self.outputs.len();
    let per_output_latency = (RATE_DIVISOR + denominator - 1)/denominator;
    let output_availability_start = inputs.input_flows.iter().flatten().map (| material_flow | material_flow.flow.first_disbursement_time_geq (inputs.start_time)).max ();
        
    let first_output_start = output_availability_start + TIME_TO_MOVE_MATERIAL;
        
    let outputs = (0..self.outputs.len()).map (| index | Some (FlowPattern::new (first_output_start + Number::from (index)*per_output_latency, per_output_rate)
    )).collect();
    
    DistributorFutureInfo::Success (DistributorSuccessInfo {
      material, outputs
    })
  }
}

impl MachineTypeTrait for Distributor {
  // basic information
  fn name (&self)->& str {& self.info.name}
  fn cost (&self)->& [(Number, Material)] {& self.info.cost}
  fn num_inputs (&self)->usize {self.inputs.len()}
  fn num_outputs (&self)->usize {self.outputs.len()}
  fn radius (&self)->Number {self.info.radius}
  fn icon(&self) ->& str {& self.info.icon}
  
  fn input_locations (&self)->Inputs <InputLocation> {self.inputs.clone()}
  fn output_locations (&self)->Inputs <InputLocation> {self.outputs.clone()}
  fn input_materials (&self)->Inputs <Option <Material>> {self.inputs.iter().map (|_| None).collect()}
  
  fn output_flows(&self, inputs: MachineObservedInputs)->Inputs <Option<MaterialFlow>> {
    match self.future_info (inputs) {
      DistributorFutureInfo::Failure (_) => self.inputs.iter().map (|_| None).collect()
      DistributorFutureInfo::Success (info) => {
        let material = info.material;
        info.outputs.into_iter().map (| flow | Some (MaterialFlow {material, flow})).collect()
      }
    }
  }
  fn momentary_visuals(&self, inputs: MachineObservedInputs, time: Number)->MachineMomentaryVisuals {
    // TODO: something's wrong with the algorithm here…
    match self.future_info (inputs) {
      DistributorFutureInfo::Failure (failure) => MachineMomentaryVisuals {materials: Vec::new(), operating_state: failure},
      DistributorFutureInfo::Success (info) => {
        let output_disbursements_since_start = info.outputs.num_disbursed_between ([inputs.start_time, time + TIME_TO_MOVE_MATERIAL]);
        let mut materials = Vec::with_capacity(self.inputs.len() + self.outputs.len() - 1) ;
        //let mut operating_state = MachineOperatingState::WaitingForInput;
        let output_rate = info.outputs.rate();
        let input_rate = something.rate();
        for output_index_since_start in output_disbursements_since_start+1 .. {
          //input_rate may be greater than output_rate; if it is, we sometimes want to skip forward in the sequence. Note that if input_rate == output_rate, this uses the same index for both. Round down so as to use earlier inputs
          input_index_since_start = output_index_since_start*input_rate/output_rate;
          let (output_index, output_time) = info.outputs.nth_disbursement_since (info.start_time, output_index_since_start);
          let (input_index, input_time) = info.inputs.nth_disbursement_since (info.start_time, input_index_since_start);
          if input_time > time {break}
          //assert!(n <= previous_disbursements + self.inputs.len() + self.outputs.len() - 1);
          // TODO: smoother movement
          let input_location = vector_to_float (self.inputs [input_index]);
          let output_location = vector_to_float (self.outputs [output_index]);
          let output_fraction = (output_time - input_time) as f64/(time - input_time) as f64;
          let location = input_location*(1.0 - output_fraction) + output_location*output_fraction;
          materials.push ((location, info.material));
        }
        
        MachineMomentaryVisuals {
          operating_state: if disbursements_since_start > 0 {MachineOperatingState::Operating} else {MachineOperatingState::WaitingForInput},
          materials,
        }
      }
    }
  }
}






struct AssemblerFutureInfo {
  Failure (MachineOperatingState),
  Success (AssemblerSuccessInfo),
}

struct AssemblerSuccessInfo {
  assembly_start_pattern: FlowPattern,
  outputs: Inputs <FlowPattern>,
}



impl Assembler {
  fn future_info (&self, inputs: MachineObservedInputs)->AssemblerFutureInfo {
    let mut assembly_rate = RATE_DIVISOR/self.assembly_duration;
    let mut assembly_start = inputs.start_time;
    for (input, material_flow) in self.inputs.iter().zip (&inputs.input_flows) {
      // TODO: don't make the priority between the failure types be based on input order
      match material_flow {
        None => return AssemblerFutureInfo::Failure (InputMissing),
        Some (material_flow) => {
          if material_flow.material != input.material {
            return AssemblerFutureInfo::Failure (InputIncompatible)
          }
          assembly_rate = min (assembly_rate, material_flow.flow.rate()/input.cost);
          assembly_start = max (assembly_start, material_flow.flow.nth_disbursement_time_since (inputs.start_time, input.cost) + TIME_TO_MOVE_MATERIAL);
        }
      }
    }
    
    if assembly_rate == 0 {
      return AssemblerFutureInfo::Failure (InputTooInfrequent)
    }
    
    let outputs = self.outputs.iter().map (| output | FlowPattern::new (assembly_start + self.assembly_duration + TIME_TO_MOVE_MATERIAL, assembly_rate*output.amount)).collect();
    
    AssemblerFutureInfo::Success (AssemblerSuccessInfo {
      assembly_start_pattern: FlowPattern::new (assembly_start, assembly_rate),
      outputs
    })
  }
}



impl MachineTypeTrait for Assembler {
  // basic information
  fn name (&self)->& str {& self.info.name}
  fn cost (&self)->& [(Number, Material)] {& self.info.cost}
  fn num_inputs (&self)->usize {self.inputs.len()}
  fn num_outputs (&self)->usize {self.outputs.len()}
  fn radius (&self)->Number {self.info.radius}
  fn icon(&self) ->& str {& self.info.icon}
  
  fn input_locations (&self)->Inputs <InputLocation> {self.inputs.iter().map (|a| a.location).collect()}
  fn output_locations (&self)->Inputs <InputLocation> {self.outputs.iter().map (|a| a.location).collect()}
  fn input_materials (&self)->Inputs <Option <Material>> {self.inputs.iter().map (|a| a.material).collect()}
  
  fn output_flows(&self, inputs: MachineObservedInputs)->Inputs <Option<MaterialFlow>> {
    match self.future_info (inputs) {
      AssemblerFutureInfo::Failure (_) => self.inputs.iter().map (|_| None).collect()
      AssemblerFutureInfo::Success (info) => {
        info.outputs.into_iter().zip (& self.outputs).map (| (flow, output) | Some (MaterialFlow {material: output.material, flow})).collect()
      }
    }
  }
  fn momentary_visuals(&self, inputs: MachineObservedInputs, time: Number)->MachineMomentaryVisuals {
    match self.future_info (inputs) {
      AssemblerFutureInfo::Failure (failure) => MachineMomentaryVisuals {materials: Vec::new(), operating_state: failure},
      AssemblerFutureInfo::Success (info) => {
        
        let previous_disbursements = info.outputs.num_disbursed_before(time + TIME_TO_MOVE_MATERIAL);
        let disbursements_since_start = info.outputs.num_disbursed_between ([inputs.start_time, time + TIME_TO_MOVE_MATERIAL]);
        let mut materials = Vec::with_capacity(self.inputs.len() + self.outputs.len() - 1) ;
        //let mut operating_state = MachineOperatingState::WaitingForInput;
        for n in previous_disbursements+1 .. {
          let (output_index, output_time) = info.outputs.nth_disbursement (n);
          let (input_index, input_time) = info.inputs.nth_disbursement (n);
          if input_time > time {break}
          assert!(n <= previous_disbursements + self.inputs.len() + self.outputs.len() - 1);
          // TODO: smoother movement
          let input_location = vector_to_float (self.inputs [input_index]);
          let output_location = vector_to_float (self.outputs [output_index]);
          let output_fraction = (output_time - input_time) as f64/(time - input_time) as f64;
          let location = input_location*(1.0 - output_fraction) + output_location*output_fraction;
          materials.push ((location, info.material));
        }
        
        MachineMomentaryVisuals {
          operating_state: if disbursements_since_start > 0 {MachineOperatingState::Operating} else {MachineOperatingState::WaitingForInput},
          materials,
        }
      }
    }
  }
}


pub fn only_value<I: IntoIterator>(mut iterator: I)->Option <I::Item> where I::Item: PartialEq<I::Item> { let iterator = iterator.into_iter(); iterator.next().filter (|a| iterator.all (|b| *a==b)) }

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
        when_enough_inputs_to_begin_output = max(when_enough_inputs_to_begin_output, flow_pattern::time_from_which_patterns_will_always_disburse_at_least_amount_plus_ideal_rate_in_total (input_patterns.iter().map (| (pattern, _material) | *pattern), time_to_switch_output, (self.inputs[0].cost - 1) - storage_before[0].0).unwrap());
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
  pub state: MachineState,
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

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct Map {
  pub machines: ArrayVec <[StatefulMachine; MAX_COMPONENTS]>,
  pub last_change_time: Number,  
}

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Game {
  pub map: Map,
  pub inventory_before_last_change: HashMap <Material, Number>,
}
