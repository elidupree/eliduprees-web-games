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
        let mut materials = Vec::with_capacity(self.inputs.len() - 1) ;
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
        
        let first_relevant_assembly_start_index = info.assembly_start_pattern.num_disbursed_between ([inputs.start_time, time - TIME_TO_MOVE_MATERIAL - self.assembly_duration);
        
        let mut materials = Vec::with_capacity(self.inputs.len() + self.outputs.len() - 1) ;
        //let mut operating_state = MachineOperatingState::WaitingForInput;
        for assembly_start_index in first_relevant_assembly_start_index.. {
          let assembly_start_time = info.assembly_start_pattern.nth_disbursement_time_since (inputs.start_time, assembly_start_index);
          let assembly_finish_time = assembly_start_time + self.assembly_duration;
          if assembly_start_time >= time {
            for (input, material_flow) in self.inputs.iter().zip (& inputs.input_flows) {
              let last_input_index = material_flow.flow.num_disbursed_between ([inputs.start_time, assembly_start_time - TIME_TO_MOVE_MATERIAL +1]) -1;
              for which_input in 0..input.cost {
                let input_index = last_input_index - which_input;
                
              }
            }
          }
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
