use arrayvec::ArrayVec;
use derivative::Derivative;
use live_prop_test::{live_prop_test, lpt_assert, lpt_assert_eq};
use nalgebra::Vector2;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::cmp::max;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

use crate::flow_pattern::{Flow, MaterialFlow};
use crate::geometry::{
  Facing, GridIsomorphism, Number, Rotate, TransformedBy, Vector, VectorExtension,
};
use crate::modules::PlatonicModule;
use crate::primitive_machines::{Assembler, Distributor};
use crate::undo_history::AddRemoveMachines;
use std::ops::{Deref, DerefMut};

pub const MAX_COMPONENTS: usize = 256;
pub const MAX_MACHINE_INPUTS: usize = 8;
pub const TIME_TO_MOVE_MATERIAL: Number = 60;
pub const MAX_IMPLICIT_OUTPUT_FLOW_CHANGES: usize = 3;
pub type Inputs<T> = ArrayVec<[T; MAX_MACHINE_INPUTS]>;
macro_rules! inputs {
  ($($whatever:tt)*) => {::std::iter::FromIterator::from_iter ([$($whatever)*].iter().cloned())};
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum MachineTypeId {
  Preset(usize),
  Module(usize),
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct MachineState {
  pub position: GridIsomorphism,
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct PlatonicMachine {
  pub type_id: MachineTypeId,
  pub state: MachineState,
}

/// A simple wrapper around PlatonicMachine, indicating that this machine is positioned relative to the global map rather than its parent module. We add a wrapper to catch mistakes where you compare the two with each other.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct GlobalMachine(pub PlatonicMachine);
impl Deref for GlobalMachine {
  type Target = PlatonicMachine;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
impl DerefMut for GlobalMachine {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

// the existential impl stopped compiling at some Rust update
// pub type MachineIdWithinPlatonicRegion =
//   impl Copy + Clone + Ord + Hash + Debug + Default + Serialize + DeserializeOwned;
pub type MachineIdWithinPlatonicRegion = (Number, Number);

impl PlatonicMachine {
  /// An ID that is guaranteed to be unique within its region.
  ///
  /// note: this must NOT include a module index,
  /// because this is used for sorting BEFORE modules are canonicalized.
  /// currently, position is enough to enforce unique ids within legal game states.
  /// if we make bridge-like machines later, this will need to get more sophisticated.
  pub fn id_within_region(&self) -> MachineIdWithinPlatonicRegion {
    let position = self.state.position.translation;
    (position[0], position[1])
  }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Derivative)]
#[derivative(Default)]
pub enum Material {
  IronOre,
  Iron,
  #[derivative(Default)]
  Garbage,
}
/*
#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct Material {
  material_type: MaterialType,
}*/

impl Material {
  pub const fn icon(self) -> &'static str {
    match self {
      Material::IronOre => "ore",
      Material::Iron => "iron",
      Material::Garbage => "machine",
    }
  }
}

#[derive(Copy, Clone, Debug)]
pub struct MachineObservedInputs<'a> {
  pub input_flows: &'a [Option<MaterialFlow>],
  pub start_time: Number,
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Derivative)]
#[derivative(Default)]
pub enum MachineOperatingState {
  #[derivative(Default)]
  Operating,
  WaitingForInput,
  InputMissing,
  InputTooInfrequent,
  InputIncompatible,
  InCycle,
}

#[derive(Debug)]
pub struct MachineMomentaryVisuals {
  pub operating_state: MachineOperatingState,
  pub materials: Vec<(Vector2<f64>, Material)>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct InputLocation {
  pub position: Vector,
  pub facing: Facing,
}

impl InputLocation {
  pub fn input(x: Number, y: Number) -> InputLocation {
    let position = Vector::new(x, y);
    InputLocation {
      position,
      facing: position
        .closest_facing()
        .expect("there shouldn't be an input location at an exact diagonal")
        .rotate_90(2),
    }
  }
  pub fn output(x: Number, y: Number) -> InputLocation {
    let position = Vector::new(x, y);
    InputLocation {
      position,
      facing: position
        .closest_facing()
        .expect("there shouldn't be an output location at an exact diagonal"),
    }
  }
}

impl TransformedBy for InputLocation {
  fn transformed_by(self, isomorphism: GridIsomorphism) -> Self {
    InputLocation {
      position: self.position.transformed_by(isomorphism),
      facing: self.facing.transformed_by(isomorphism),
    }
  }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct StandardMachineInfo {
  pub name: String,
  pub icon: String,
  pub radius: Number,
  pub cost: Vec<(Number, Material)>,
}

impl StandardMachineInfo {
  pub fn new(
    name: impl Into<String>,
    icon: impl Into<String>,
    radius: Number,
    cost: Vec<(Number, Material)>,
  ) -> StandardMachineInfo {
    StandardMachineInfo {
      name: name.into(),
      icon: icon.into(),
      radius,
      cost,
    }
  }
}

#[allow(unused)]
#[live_prop_test]
pub trait MachineTypeTrait {
  // basic information
  fn name(&self) -> &str;
  fn cost(&self) -> &[(Number, Material)] {
    &[]
  }
  fn num_inputs(&self) -> usize {
    0
  }
  fn num_outputs(&self) -> usize {
    0
  }
  #[live_prop_test(postcondition = "result > 0")]
  fn radius(&self) -> Number {
    1
  }
  fn icon(&self) -> &str {
    ""
  }

  #[live_prop_test(
    postcondition = "result.len() == self.num_inputs()",
    postcondition = "check_input_output_locations(self.radius(), &result, &self.relative_output_locations())"
  )]
  fn relative_input_locations(&self) -> Inputs<InputLocation> {
    inputs![]
  }

  #[live_prop_test(
    postcondition = "result.len() == self.num_outputs()",
    postcondition = "check_input_output_locations(self.radius(), &self.relative_input_locations(), &result)"
  )]
  fn relative_output_locations(&self) -> Inputs<InputLocation> {
    inputs![]
  }

  #[live_prop_test(postcondition = "result.len() == self.num_inputs()")]
  fn input_materials(&self) -> Inputs<Option<Material>> {
    inputs![]
  }

  type Future: Clone + Eq + Hash + Serialize + DeserializeOwned + Debug;

  #[live_prop_test(
    postcondition = "result != Err(MachineOperatingState::Operating)", // That's not an error
    postcondition = "result != Err(MachineOperatingState::WaitingForInput)", // Futures are timeless and this state isn't
    postcondition = "result != Err(MachineOperatingState::InCycle)", // Individual machines can't detect this
  )]
  fn future(&self, inputs: MachineObservedInputs) -> Result<Self::Future, MachineOperatingState>;

  #[live_prop_test(
    precondition = "inputs.input_flows.len() == self.num_inputs()",
    postcondition = "result.len() == self.num_outputs()",
    postcondition = "check_output_flows(self, inputs, future, &result)"
  )]
  fn output_flows(
    &self,
    inputs: MachineObservedInputs,
    future: &Self::Future,
  ) -> Inputs<Option<MaterialFlow>> {
    inputs![]
  }

  // Note: at the moment when a piece of material is handed off from one machine to another, the SOURCE machine is responsible for drawing it, and the destination machine should not draw it.
  fn relative_momentary_visuals(
    &self,
    inputs: MachineObservedInputs,
    future: &Self::Future,
    time: Number,
  ) -> MachineMomentaryVisuals {
    MachineMomentaryVisuals {
      materials: Vec::new(),
      operating_state: MachineOperatingState::Operating,
    }
  }
}

macro_rules! machine_type_enums {
  ($($Type: ident as $Variant: ident,)*) => {


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum MachineType {
  $($Variant ($Type),)*
}

#[derive (Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum MachineTypeRef<'a> {
  $($Variant (&'a $Type),)*
}


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum MachineFuture {
  $($Variant (<$Type as MachineTypeTrait>::Future),)*
}

impl MachineType {
  pub fn as_ref(&self) -> MachineTypeRef {
    match self {$(MachineType::$Variant (value) => MachineTypeRef::$Variant (value),)*}
  }
}

#[live_prop_test(use_trait_tests)]
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

  fn relative_momentary_visuals(&self, inputs: MachineObservedInputs, future: &Self::Future, time: Number)->MachineMomentaryVisuals {
    match (self, future) {
      $((MachineTypeRef::$Variant (value), MachineFuture::$Variant (future)) => value.relative_momentary_visuals (inputs, future, time),)*
      _=> panic!("Passed wrong future type to MachineType::relative_momentary_visuals()"),
    }
  }
}



  };
}

machine_type_enums! {
  Distributor as Distributor, Assembler as Assembler, PlatonicModule as Module, //Mine, ModuleMachine, // Conveyor,
}

fn check_output_flows<T: MachineTypeTrait + ?Sized>(
  machine: &T,
  inputs: MachineObservedInputs,
  future: &T::Future,
  outputs: &[Option<MaterialFlow>],
) -> Result<(), String> {
  for output in outputs {
    if let Some(output) = output {
      lpt_assert!(
        output.flow.start_time() >= inputs.start_time,
        "output {:?} started before start time {}",
        output,
        inputs.start_time
      );
    }
  }

  fn has_near(
    visuals: &MachineMomentaryVisuals,
    material: Material,
    location: InputLocation,
    extra_leeway: f64,
  ) -> bool {
    let leeway = extra_leeway + 0.0000001;
    visuals.materials.iter().any(|&(l, m)| {
      m == material && {
        let rel = l - location.position.to_f64();
        rel[0].abs() <= leeway && rel[1].abs() <= leeway
      }
    })
  }

  let speed_leeway = machine.radius() as f64 * 2.0 / TIME_TO_MOVE_MATERIAL as f64;

  for (output, output_location) in outputs.iter().zip(machine.relative_output_locations()) {
    if let Some(output) = output {
      for i in 0..5 {
        let time = output.nth_disbursement_time(i).unwrap();
        let visuals = machine.relative_momentary_visuals(inputs, future, time);
        lpt_assert!(
          has_near(&visuals, output.material, output_location, 0.0),
          "Outputs must be displayed at exactly the output location at disbursement times: {:?}",
          (i, time, output, output_location, visuals)
        );
        let visuals = machine.relative_momentary_visuals(inputs, future, time + 1);
        lpt_assert!(
          !has_near(&visuals, output.material, output_location, speed_leeway),
          "There must be no materials near the output location right after disbursement times: {:?}",
          (i, time, output, output_location, visuals)
        );
      }
    }
  }
  for (input, input_location) in inputs
    .input_flows
    .iter()
    .zip(machine.relative_input_locations())
  {
    if let Some(input) = input {
      for i in 0..5 {
        let time = input.nth_disbursement_time(i).unwrap();
        let visuals = machine.relative_momentary_visuals(inputs, future, time);
        lpt_assert!(
          !has_near(&visuals, input.material, input_location, speed_leeway),
          "Inputs must not be displayed near the input location at disbursement times: {:?}",
          (i, time, input, input_location, visuals)
        );
        // can't always require displaying inputs, as some inputs may be deleted;
        // TODO come up with a way to selectively make the assertion for non-deleted inputs
        /*let visuals = machine.relative_momentary_visuals(inputs, future, time + 1);
        lpt_assert!(
          has_near(&visuals, input.material, input_location, speed_leeway),
          "Inputs must be displayed near the input location right after disbursement times: {:?}",
          (i, time, input, input_location, visuals)
        );*/
      }
    }
  }
  Ok(())
}

fn check_input_output_locations(
  radius: Number,
  inputs: &Inputs<InputLocation>,
  outputs: &Inputs<InputLocation>,
) -> Result<(), String> {
  for output in outputs {
    lpt_assert_eq!(output.position.closest_facing(), Some(output.facing));
  }
  for input in inputs {
    lpt_assert_eq!(
      input.position.closest_facing(),
      Some(input.facing.rotate_90(2))
    );
  }
  let mut discovered = HashSet::new();
  for output in outputs.iter().chain(inputs) {
    lpt_assert!(
      discovered.insert(output.position),
      "there are 2 inputs/outputs in the same position",
    );
    let distance_from_center = max(output.position[0].abs(), output.position[1].abs());
    lpt_assert_eq!(
      distance_from_center,
      radius,
      "input/output is not on the boundary of the machine",
    );
    lpt_assert!(
      output.position[0].abs() % 2 != output.position[1].abs() % 2,
      "input/output position {:?} is not properly aligned to the edge of a grid square (it's at a half position)",
      output.position,
    );
  }
  Ok(())
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct PlatonicRegionContents {
  pub machines: Vec<PlatonicMachine>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct MachineTypes {
  pub presets: Vec<MachineType>,
  pub custom_modules: Vec<PlatonicModule>,
}

impl<'a> MachineTypeRef<'a> {
  pub fn input_locations(&self, position: GridIsomorphism) -> impl Iterator<Item = InputLocation> {
    self
      .relative_input_locations()
      .into_iter()
      .map(move |location| location.transformed_by(position))
  }
  pub fn output_locations(&self, position: GridIsomorphism) -> impl Iterator<Item = InputLocation> {
    self
      .relative_output_locations()
      .into_iter()
      .map(move |location| location.transformed_by(position))
  }
}

impl MachineTypes {
  pub fn get(&self, id: MachineTypeId) -> MachineTypeRef {
    match id {
      MachineTypeId::Preset(index) => self.presets.get(index).unwrap().as_ref(),
      MachineTypeId::Module(index) => {
        MachineTypeRef::Module(self.custom_modules.get(index).unwrap())
      }
    }
  }

  /// kinda hacky function, deprecated
  pub fn get_module(&self, id: MachineTypeId) -> &PlatonicModule {
    match self.get(id) {
      MachineTypeRef::Module(module) => module,
      _ => panic!(
        "get_module() given an id of a non-module machine ({:?})",
        id
      ),
    }
  }

  pub fn input_locations(&self, machine: &PlatonicMachine) -> impl Iterator<Item = InputLocation> {
    self
      .get(machine.type_id)
      .input_locations(machine.state.position)
  }
  pub fn output_locations(&self, machine: &PlatonicMachine) -> impl Iterator<Item = InputLocation> {
    self
      .get(machine.type_id)
      .output_locations(machine.state.position)
  }

  pub fn modules(&self) -> impl Iterator<Item = (MachineTypeId, &PlatonicModule)> {
    self
      .presets
      .iter()
      .enumerate()
      .filter_map(|(index, preset)| match preset {
        MachineType::Module(module) => Some((MachineTypeId::Preset(index), module)),
        _ => None,
      })
      .chain(
        self
          .custom_modules
          .iter()
          .enumerate()
          .map(|(index, module)| (MachineTypeId::Module(index), module)),
      )
  }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Derivative)]
#[derivative(Default(bound = ""))]
pub struct WorldMachinesMap<T> {
  pub here: HashMap<MachineIdWithinPlatonicRegion, T>,
  pub children: HashMap<MachineIdWithinPlatonicRegion, WorldMachinesMap<T>>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Game {
  pub global_region: PlatonicRegionContents,
  pub machine_types: MachineTypes,
  pub last_disturbed_times: WorldMachinesMap<Number>,
  pub last_change_time: Number,
  pub inventory_before_last_change: HashMap<Material, Number>,
  pub undo_stack: Vec<AddRemoveMachines>,
  pub redo_stack: Vec<AddRemoveMachines>,
}

impl Game {
  pub fn platonic_regions(&self) -> impl Iterator<Item = &PlatonicRegionContents> {
    std::iter::once(&self.global_region).chain(
      self
        .machine_types
        .modules()
        .map(|(_id, module)| &module.region),
    )
  }
  pub fn platonic_machines_mut(&mut self) -> impl Iterator<Item = &'_ mut PlatonicMachine> + '_ {
    std::iter::once(&mut self.global_region)
      .chain(
        self
          .machine_types
          .custom_modules
          .iter_mut()
          .map(|module| &mut module.region),
      )
      .flat_map(|region| &mut region.machines)
  }
}
