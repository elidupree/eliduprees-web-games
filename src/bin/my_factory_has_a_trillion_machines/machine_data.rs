use std::cmp::{min, max};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::hash::Hash;
use std::fmt::Debug;
use serde::{Serialize, de::DeserializeOwned};
use nalgebra::Vector2;

use arrayvec::ArrayVec;


use geometry::{Number, Vector, VectorExtension, Facing, GridIsomorphism, TransformedBy};
use flow_pattern::{FlowPattern, MaterialFlow, CroppedFlow, RATE_DIVISOR, Flow, FlowCollection};
//use modules::ModuleMachine;
use modules::{Module};

pub const MAX_COMPONENTS: usize = 256;
pub const MAX_MACHINE_INPUTS: usize = 8;
pub const TIME_TO_MOVE_MATERIAL: Number = 60;
pub const MAX_IMPLICIT_OUTPUT_FLOW_CHANGES: usize = 3;
pub type Inputs<T> = ArrayVec <[T; MAX_MACHINE_INPUTS]>;
macro_rules! inputs {
  ($($whatever:tt)*) => {::std::iter::FromIterator::from_iter ([$($whatever)*].iter().cloned())};
}

#[derive (Copy, Clone, Debug)]
pub struct MachineObservedInputs <'a> {
  pub input_flows: & 'a [Option<MaterialFlow>],
  pub start_time: Number,
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
  
  fn relative_input_locations (&self)->Inputs <InputLocation> {inputs![]}
  fn relative_output_locations (&self)->Inputs <InputLocation> {inputs![]}
  fn input_materials (&self)->Inputs <Option <Material>> {inputs![]}
  
  type Future: Clone + Eq + Hash + Serialize + DeserializeOwned + Debug;
  fn future (&self, inputs: MachineObservedInputs)->Result <Self::Future, MachineOperatingState>;
  fn output_flows(&self, inputs: MachineObservedInputs, future: &Self::Future)->Inputs <Option<MaterialFlow>> {inputs![]}
  
  // Note: at the moment when a piece of material is handed off from one machine to another, the SOURCE machine is responsible for drawing it, and the destination machine should not draw it.
  fn momentary_visuals(&self, inputs: MachineObservedInputs, future: &Self::Future, time: Number)->MachineMomentaryVisuals {MachineMomentaryVisuals {materials: Vec::new(), operating_state: MachineOperatingState::Operating}}
}

macro_rules! machine_type_enums {
  ($($Variant: ident,)*) => {
  

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum MachineType {
  $($Variant ($Variant),)*
}

#[derive (Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum MachineTypeRef<'a> {
  $($Variant (&'a $Variant),)*
}


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum MachineFuture {
  $($Variant (<$Variant as MachineTypeTrait>::Future),)*
}

impl MachineType {
  pub fn as_ref(&self) -> MachineTypeRef {
    match self {$(MachineType::$Variant (value) => MachineTypeRef::$Variant (value),)*}
  }
}


impl<'a> MachineTypeTrait for MachineTypeRef<'a> {
  fn name (&self)->& str {match self {$(MachineTypeRef::$Variant (value) => value.name(),)*}}
  fn cost (&self)->& [(Number, Material)] {match self {$(MachineTypeRef::$Variant (value) => value.cost (),)*}}
  fn num_inputs (&self)->usize {match self {$(MachineTypeRef::$Variant (value) => value.num_inputs (),)*}}
  fn num_outputs (&self)->usize {match self {$(MachineTypeRef::$Variant (value) => value.num_outputs (),)*}}
  fn radius (&self)->Number {match self {$(MachineTypeRef::$Variant (value) => value.radius (),)*}}
  fn icon(&self) ->& str {match self {$(MachineTypeRef::$Variant (value) => value.icon (),)*}}
  
  fn relative_input_locations (&self)->Inputs <InputLocation> {match self {$(MachineTypeRef::$Variant (value) => value.relative_input_locations (),)*}}
  fn relative_output_locations (&self)->Inputs <InputLocation> {match self {$(MachineTypeRef::$Variant (value) => value.relative_output_locations (),)*}}
  fn input_materials (&self)->Inputs <Option <Material>> {match self {$(MachineTypeRef::$Variant (value) => value.input_materials (),)*}}
  
  type Future = MachineFuture;
  
  fn future (&self, inputs: MachineObservedInputs)->Result <Self::Future, MachineOperatingState> {
    match self {
      $(MachineTypeRef::$Variant (value) => Ok(MachineFuture::$Variant (value.future (inputs)?)),)*
    }
  }
  
  fn output_flows(&self, inputs: MachineObservedInputs, future: &Self::Future)->Inputs <Option<MaterialFlow>> {
    match (self, future) {
      $((MachineTypeRef::$Variant (value), MachineFuture::$Variant (future)) => value.output_flows (inputs, future),)*
      _=> panic!("Passed wrong future type to MachineType::output_flows()"),
    }
  }
  
  fn momentary_visuals(&self, inputs: MachineObservedInputs, future: &Self::Future, time: Number)->MachineMomentaryVisuals {
    match (self, future) {
      $((MachineTypeRef::$Variant (value), MachineFuture::$Variant (future)) => value.momentary_visuals (inputs, future, time),)*
      _=> panic!("Passed wrong future type to MachineType::momentary_visuals()"),
    }
  }
}
    


  };
}

machine_type_enums! {
  Distributor, Assembler, Module, //Mine, ModuleMachine, // Conveyor,
}

#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum MachineTypeId {
  Preset (usize),
  Module (usize),
}


#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct InputLocation {
  pub position: Vector,
  pub facing: Facing,
}

impl TransformedBy for InputLocation {
  fn transformed_by (self, isomorphism: GridIsomorphism)->Self {
    InputLocation {
      position: self.position.transformed_by(isomorphism),
      facing: self.facing.transformed_by(isomorphism),
    }
  }
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

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct StandardMachineInfo {
  pub name: String,
  pub icon: String,
  pub radius: Number,
  pub cost: Vec<(Number, Material)>,
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
  pub fn new (x: Number, y: Number, facing: Facing)->InputLocation {InputLocation {position: Vector::new (x,y), facing}}
}
impl AssemblerInput {
  pub fn new (x: Number, y: Number, facing: Facing, material: Material, cost: Number)->AssemblerInput {
    AssemblerInput {
      location: InputLocation::new (x,y, facing), material, cost
    }
  }
}
impl AssemblerOutput {
  pub fn new (x: Number, y: Number, facing: Facing, material: Material, amount: Number)->AssemblerOutput {
    AssemblerOutput {
      location: InputLocation::new (x,y, facing), material, amount
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
      InputLocation::new (0, 1, 1),
      InputLocation::new (0, -1, 3),
    ],
  })
}

pub fn iron_smelter()->MachineType {
  MachineType::Assembler (Assembler {
    info: StandardMachineInfo::new ("Iron smelter", "machine", 3, vec![(5, Material::Iron)]),
    inputs: inputs! [
      AssemblerInput::new (-3, 0, 0, Material::IronOre, 3),
    ],
    outputs: inputs! [
      AssemblerOutput::new (3, 0, 0, Material::Iron, 2),
    ],
    assembly_duration: 10*TIME_TO_MOVE_MATERIAL,
  })
}

pub fn iron_mine()->MachineType {
  MachineType::Assembler (Assembler {
    info: StandardMachineInfo::new ("Iron mine", "mine", 3, vec![(50, Material::Iron)]),
    inputs: inputs! [],
    outputs: inputs! [
      AssemblerOutput::new (3, 0, 0, Material::IronOre, 1),
    ],
    assembly_duration: TIME_TO_MOVE_MATERIAL,
  })
}



#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct MachineState {
  pub position: GridIsomorphism,
  pub last_disturbed_time: Number,
}


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct DistributorFuture {
  outputs: Inputs <FlowPattern>,
  output_availability_start: Number,
  material: Material,
}



impl MachineTypeTrait for Distributor {
  fn name (&self)->& str {& self.info.name}
  fn cost (&self)->& [(Number, Material)] {& self.info.cost}
  fn num_inputs (&self)->usize {self.inputs.len()}
  fn num_outputs (&self)->usize {self.outputs.len()}
  fn radius (&self)->Number {self.info.radius}
  fn icon(&self) ->& str {& self.info.icon}
  
  fn relative_input_locations (&self)->Inputs <InputLocation> {self.inputs.clone()}
  fn relative_output_locations (&self)->Inputs <InputLocation> {self.outputs.clone()}
  fn input_materials (&self)->Inputs <Option <Material>> {self.inputs.iter().map (|_| None).collect()}
  
  type Future = DistributorFuture;
  
  fn future (&self, inputs: MachineObservedInputs)->Result <Self::Future, MachineOperatingState> {
    let mut material_iterator = inputs.input_flows.iter().flatten().map (| material_flow | material_flow.material);
    let material = match material_iterator.next() {
      None => return Err(MachineOperatingState::InputMissing),
      Some (material) => if material_iterator.all(| second | second == material) {
        material
      } else {return Err(MachineOperatingState::InputIncompatible)}
    };
    
    
    let total_input_rate = inputs.input_flows.rate();
    
    let num_outputs = Number::try_from (self.outputs.len()).unwrap();
    let per_output_rate = min (RATE_DIVISOR/TIME_TO_MOVE_MATERIAL, total_input_rate/num_outputs);
    if per_output_rate == 0 {
      return Err(MachineOperatingState::InputTooInfrequent)
    }
    let total_output_rate = per_output_rate*num_outputs;
    // the rounding here could theoretically be better, but this should be okay
    let latency_between_outputs = (RATE_DIVISOR + total_output_rate - 1)/total_output_rate;
    let output_availability_start = inputs.input_flows.iter().flatten().map (| material_flow | material_flow.first_disbursement_time_geq (inputs.start_time)).max ().unwrap();
        
    let first_output_start = output_availability_start + TIME_TO_MOVE_MATERIAL;
        
    let outputs = (0..self.outputs.len()).map (| index | FlowPattern::new (first_output_start + Number::try_from (index).unwrap()*latency_between_outputs, per_output_rate)
    ).collect();
    
    Ok (DistributorFuture {
      material, output_availability_start, outputs
    })
  }
  
  fn output_flows(&self, _inputs: MachineObservedInputs, future: &Self::Future)->Inputs <Option<MaterialFlow>> {
        let material = future.material;
        future.outputs.iter().map (| & flow | Some (MaterialFlow {material, flow})).collect()
  }
  
  fn momentary_visuals(&self, inputs: MachineObservedInputs, future: &Self::Future, time: Number)->MachineMomentaryVisuals {
        let output_disbursements_since_start = future.outputs.num_disbursed_between ([inputs.start_time, time]);
        let mut materials = Vec::with_capacity(self.inputs.len() - 1) ;
        //let mut operating_state = MachineOperatingState::WaitingForInput;
        let output_rate = future.outputs.rate();
        let input_rate = inputs.input_flows.rate();
        let cropped_inputs: Inputs <_> = inputs.input_flows.iter().map (| material_flow | material_flow.map (| material_flow | CroppedFlow {flow: material_flow.flow, crop_start: material_flow.last_disbursement_time_leq (future.output_availability_start).unwrap()})).collect();
        for output_index_since_start in output_disbursements_since_start .. {
          //input_rate may be greater than output_rate; if it is, we sometimes want to skip forward in the sequence. Note that if input_rate == output_rate, this uses the same index for both. Round down so as to use earlier inputs
          //TODO: wonder if there's a nice-looking way to make sure the deletions are distributed evenly over the inputs? (Right now when there is a simple 2-1 merge, everything from one side is deleted and everything from the other side goes through)
          let input_index_since_start = output_index_since_start*input_rate/output_rate;
          let (output_time, output_index) = future.outputs.nth_disbursement_geq_time (output_index_since_start, inputs.start_time).unwrap();
          let (input_time, input_index) = cropped_inputs.nth_disbursement_geq_time (input_index_since_start, inputs.start_time).unwrap();
          if input_time > time {break}
          //assert!(n <= previous_disbursements + self.inputs.len() + self.outputs.len() - 1);
          // TODO: smoother movement
          let input_location = self.inputs [input_index].position.to_f64 ();
          let output_location = self.outputs [output_index].position.to_f64 ();
          let output_fraction = (time - input_time) as f64/(output_time - input_time) as f64;
          //println!("{:?}", (output_index_since_start, input_index_since_start, time, input_time, output_time, input_location, output_location, output_fraction));
          let location = input_location*(1.0 - output_fraction) + output_location*output_fraction;
          materials.push ((location, future.material));
        }
        
        MachineMomentaryVisuals {
          operating_state: if output_disbursements_since_start > 0 {MachineOperatingState::Operating} else {MachineOperatingState::WaitingForInput},
          materials,
        }
  }
}



#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct AssemblerFuture {
  assembly_start_pattern: FlowPattern,
  outputs: Inputs <FlowPattern>,
}




impl MachineTypeTrait for Assembler {
  // basic information
  fn name (&self)->& str {& self.info.name}
  fn cost (&self)->& [(Number, Material)] {& self.info.cost}
  fn num_inputs (&self)->usize {self.inputs.len()}
  fn num_outputs (&self)->usize {self.outputs.len()}
  fn radius (&self)->Number {self.info.radius}
  fn icon(&self) ->& str {& self.info.icon}
  
  fn relative_input_locations (&self)->Inputs <InputLocation> {self.inputs.iter().map (|a| a.location).collect()}
  fn relative_output_locations (&self)->Inputs <InputLocation> {self.outputs.iter().map (|a| a.location).collect()}
  fn input_materials (&self)->Inputs <Option <Material>> {self.inputs.iter().map (|a| Some(a.material)).collect()}
  
  type Future = AssemblerFuture;
  
  fn future (&self, inputs: MachineObservedInputs)->Result <Self::Future, MachineOperatingState> {
    let mut assembly_rate = RATE_DIVISOR/self.assembly_duration;
    let mut assembly_start = inputs.start_time;
    for (input, material_flow) in self.inputs.iter().zip (inputs.input_flows) {
      // TODO: don't make the priority between the failure types be based on input order
      match material_flow {
        None => return Err(MachineOperatingState::InputMissing),
        Some (material_flow) => {
          if material_flow.material != input.material {
            return Err(MachineOperatingState::InputIncompatible)
          }
          assembly_rate = min (assembly_rate, material_flow.rate()/input.cost);
          assembly_start = max (assembly_start, material_flow.nth_disbursement_time_geq (input.cost-1, inputs.start_time).unwrap() + TIME_TO_MOVE_MATERIAL);
        }
      }
    }
    
    if assembly_rate == 0 {
      return Err(MachineOperatingState::InputTooInfrequent)
    }
    
    let outputs = self.outputs.iter().map (| output | FlowPattern::new (assembly_start + self.assembly_duration + TIME_TO_MOVE_MATERIAL, assembly_rate*output.amount)).collect();
    
    Ok(AssemblerFuture {
      assembly_start_pattern: FlowPattern::new (assembly_start, assembly_rate),
      outputs
    })
  }
  fn output_flows(&self, _inputs: MachineObservedInputs, future: &Self::Future)->Inputs <Option<MaterialFlow>> {
        future.outputs.iter().zip (& self.outputs).map (| (& flow, output) | Some (MaterialFlow {material: output.material, flow})).collect()
  }
  fn momentary_visuals(&self, inputs: MachineObservedInputs, future: &Self::Future, time: Number)->MachineMomentaryVisuals {
        let first_relevant_assembly_start_index = max(0, future.assembly_start_pattern.num_disbursed_between ([inputs.start_time, time - self.assembly_duration]) - 1);
        
        let mut materials = Vec::with_capacity(self.inputs.len() + self.outputs.len() - 1) ;
        //let mut operating_state = MachineOperatingState::WaitingForInput;
        for assembly_start_index in first_relevant_assembly_start_index.. {
          let assembly_start_time = future.assembly_start_pattern.nth_disbursement_time_geq (assembly_start_index, inputs.start_time).unwrap();
          let assembly_finish_time = assembly_start_time + self.assembly_duration;
          let mut too_late = assembly_start_time >= time;

          if assembly_start_time >= time {
            for (input, material_flow) in self.inputs.iter().zip (inputs.input_flows) {
              let material_flow = material_flow.unwrap();
              let last_input_index = material_flow.num_disbursed_between ([inputs.start_time, assembly_start_time - TIME_TO_MOVE_MATERIAL +1]) -1;
              for which_input in 0..input.cost {
                let input_index = last_input_index - which_input;
                let input_time = material_flow.nth_disbursement_time_geq (input_index, inputs.start_time).unwrap();
                if input_time > time { continue;}
                too_late = false;
                assert!(input_time < assembly_start_time) ;
                let input_location = input.location.position.to_f64();
                let assembly_location = Vector2::new (0.0, 0.0);
                let assembly_fraction = (time - input_time) as f64/(assembly_start_time - input_time) as f64;
                let location = input_location*(1.0 - assembly_fraction) + assembly_location*assembly_fraction;
                materials.push ((location, input.material));
              }
            }
          }
          else if assembly_finish_time <= time {
            for (output, flow) in self.outputs.iter().zip (& future.outputs) {
              let first_output_index = flow.num_disbursed_between ([inputs.start_time, assembly_finish_time + TIME_TO_MOVE_MATERIAL]);
              for which_output in 0..output.amount {
                let output_index = first_output_index + which_output;
                let output_time = flow.nth_disbursement_time_geq (output_index, inputs.start_time).unwrap();
                assert!(output_time >assembly_finish_time) ;
                if time < output_time {
                let output_location = output.location.position.to_f64();
                let assembly_location = Vector2::new (0.0, 0.0);
                let assembly_fraction = (time - output_time) as f64/(assembly_finish_time - output_time) as f64;
                let location = output_location*(1.0 - assembly_fraction) + assembly_location*assembly_fraction;
                materials.push ((location, output.material));
                }
              }
            }
          }
          else {
            // hack, TODO better representation of the assembly being in progress
            materials.push ((Vector2::new (0.0, 0.0), Material::Garbage));
          }
          
          if too_late { break }
        }
        
        MachineMomentaryVisuals {
          operating_state: if time >= future.assembly_start_pattern.start_time() - TIME_TO_MOVE_MATERIAL {MachineOperatingState::Operating} else {MachineOperatingState::WaitingForInput},
          materials,
        }
  }
}


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct StatefulMachine {
  pub type_id: MachineTypeId,
  pub state: MachineState,
}


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct Map {
  pub machines: Vec <StatefulMachine>,
}

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct MachineTypes {
  pub presets: Vec<MachineType>,
  pub modules: Vec<Module>,
}

impl<'a> MachineTypeRef<'a> {
  pub fn input_locations (&self, position: GridIsomorphism)->impl Iterator <Item = InputLocation> {
    self.relative_input_locations().into_iter().map (move | location | location.transformed_by (position))
  }
  pub fn output_locations (&self, position: GridIsomorphism)->impl Iterator <Item = InputLocation> {
    self.relative_output_locations().into_iter().map (move | location | location.transformed_by (position))
  }

}

impl MachineTypes {
  pub fn get(&self, id: MachineTypeId)->MachineTypeRef {
    match id {
      MachineTypeId::Preset(index) => self.presets.get(index).unwrap().as_ref(),
      MachineTypeId::Module(index) => MachineTypeRef::Module(self.modules.get(index).unwrap()),
    }
  }
  
  pub fn input_locations(&self, machine: &StatefulMachine)->impl Iterator <Item = InputLocation> {
    self.get (machine.type_id).input_locations (machine.state.position)
  }
  pub fn output_locations(&self, machine: &StatefulMachine)->impl Iterator <Item = InputLocation> {
    self.get (machine.type_id).output_locations (machine.state.position)
  }

}

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Game {
  pub map: Map,
  pub machine_types: MachineTypes,
  pub last_change_time: Number,
  pub inventory_before_last_change: HashMap <Material, Number>,
}
