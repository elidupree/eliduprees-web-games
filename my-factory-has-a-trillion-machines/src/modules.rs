#![allow(unused_imports)]
use arrayvec::ArrayVec;
use live_prop_test::{live_prop_test, lpt_assert};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::flow_pattern::{
  Flow, FlowCollection, FlowPattern, FlowRate, MaterialFlow, MaterialFlowRate, RATE_DIVISOR,
};
use crate::geometry::{Facing, Number, Vector, VectorExtension};
use crate::graph_algorithms::RegionFuture;
use crate::machine_data::{
  Game, InputLocation, Inputs, MachineMomentaryVisuals, MachineObservedInputs,
  MachineOperatingState, MachineType, MachineTypeId, MachineTypeRef, MachineTypeTrait,
  MachineTypes, Material, PlatonicMachine, PlatonicRegionContents, StandardMachineInfo,
  MAX_COMPONENTS, TIME_TO_MOVE_MATERIAL,
};

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct ModuleType {
  pub info: StandardMachineInfo,
  pub inner_radius: Number,
  pub inputs: Inputs<ModuleInput>,
  pub outputs: Inputs<ModuleInput>,
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ModuleInput {
  pub outer_location: InputLocation,
  pub inner_location: InputLocation,
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct PlatonicModule {
  pub module_type: ModuleType,
  pub cost: Vec<(Number, Material)>,
  pub region: PlatonicRegionContents,
}

pub fn basic_module() -> MachineType {
  fn y(index: Number) -> Number {
    -3 + index * 2
  }
  fn input(index: Number) -> ModuleInput {
    ModuleInput {
      outer_location: InputLocation::input(-20, y(index)),
      inner_location: InputLocation::input(-18, y(index)),
    }
  }
  fn output(index: Number) -> ModuleInput {
    ModuleInput {
      outer_location: InputLocation::output(20, y(index)),
      inner_location: InputLocation::output(18, y(index)),
    }
  }

  let cost = vec![(20, Material::Iron)];

  MachineType::Module(PlatonicModule {
    module_type: ModuleType {
      info: StandardMachineInfo::new("Basic module", "rounded-rectangle-solid", 20, cost.clone()),
      inner_radius: 18,
      inputs: (0..4).map(input).collect(),
      outputs: (0..4).map(output).collect(),
    },
    cost,
    region: PlatonicRegionContents {
      machines: Vec::new(),
    },
  })
}

pub type CanonicalModuleInputs = Inputs<Option<MaterialFlowRate>>;
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ModuleMachineFuture {
  pub canonical_inputs: CanonicalModuleInputs,
  pub start_time: Number,
}

pub fn canonical_module_input(input: MaterialFlow) -> Option<MaterialFlowRate> {
  const STANDARD_RATE: Number = RATE_DIVISOR / TIME_TO_MOVE_MATERIAL;

  // TODO: somehow decide on a more principled choice of rates
  const PERMITTED_RATES: &[Number] = &[
    STANDARD_RATE / 96,
    STANDARD_RATE / 64,
    STANDARD_RATE / 48,
    STANDARD_RATE / 36,
    STANDARD_RATE / 32,
    STANDARD_RATE / 24,
    STANDARD_RATE / 16,
    STANDARD_RATE / 12,
    STANDARD_RATE / 8,
    STANDARD_RATE / 6,
    STANDARD_RATE / 5,
    STANDARD_RATE / 4,
    STANDARD_RATE / 3,
    STANDARD_RATE * 2 / 5,
    STANDARD_RATE / 2,
    STANDARD_RATE * 2 / 3,
    STANDARD_RATE * 3 / 5,
    STANDARD_RATE * 3 / 4,
    STANDARD_RATE * 4 / 5,
    STANDARD_RATE,
  ];

  let rounded_down = match PERMITTED_RATES.binary_search(&input.rate()) {
    Ok(index) => PERMITTED_RATES[index],
    // something smaller than the minimum permitted rate can't flow at all and returns None
    Err(index) => PERMITTED_RATES[index.checked_sub(1)?],
  };

  Some(MaterialFlowRate {
    material: input.material,
    flow: FlowRate::new(rounded_down),
  })
}

#[live_prop_test]
impl PlatonicModule {
  fn internal_outputs(&self, variation: &RegionFuture) -> Inputs<Option<MaterialFlow>> {
    self
      .module_type
      .outputs
      .iter()
      .map(|output| {
        variation
          .dumped
          .iter()
          .find(|(location, _)| *location == output.inner_location)
          .map(|(_, flow)| *flow)
      })
      .collect()
  }

  #[live_prop_test(
    precondition = "_inputs.input_flows.len() == self.num_inputs()",
    postcondition = "result.len() == self.num_outputs()",
    postcondition = "check_module_output_flows(&self, _inputs, module_machine_future, variation, &result)"
  )]
  pub fn module_output_flows(
    &self,
    _inputs: MachineObservedInputs,
    module_machine_future: &ModuleMachineFuture,
    variation: &RegionFuture,
  ) -> Inputs<Option<MaterialFlow>> {
    self
      .internal_outputs(variation)
      .into_iter()
      .map(|output| {
        output
          .map(|output| output.delayed_by(module_machine_future.start_time + TIME_TO_MOVE_MATERIAL))
      })
      .collect()
  }

  pub fn module_relative_momentary_visuals(
    &self,
    inputs: MachineObservedInputs,
    module_machine_future: &ModuleMachineFuture,
    outer_time: Number,
    variation: &RegionFuture,
  ) -> MachineMomentaryVisuals {
    let inner_time = outer_time - module_machine_future.start_time;

    let mut materials =
      Vec::with_capacity(self.module_type.inputs.len() + self.module_type.outputs.len());
    let mut operating_state = MachineOperatingState::WaitingForInput;

    // Note: module_machine_future.start_time – the moment when inner_time is 0 – is the moment when they first set of materials arrives INSIDE the module, meaning that stuff is moving across the module boundary earlier than that.

    for (input_index, (outer_input, inner_input)) in inputs
      .input_flows
      .iter()
      .zip(&module_machine_future.canonical_inputs)
      .enumerate()
    {
      if let (Some(outer_input), Some(inner_input)) = (outer_input, inner_input) {
        let first_relevant_output_inner_index =
          inner_input.num_disbursed_before(max(0, inner_time));
        for index in first_relevant_output_inner_index..=first_relevant_output_inner_index + 1 {
          let material_output_inner_time = inner_input.nth_disbursement_time(index).unwrap();
          assert!(material_output_inner_time >= 0);
          let material_output_outer_time =
            material_output_inner_time + module_machine_future.start_time;
          let material_input_outer_time = outer_input
            .last_disbursement_time_leq(material_output_outer_time - TIME_TO_MOVE_MATERIAL)
            .unwrap();

          // >, not >=; don't draw at the moment of input, fitting the general rule that the source machine draws the material
          if outer_time > material_input_outer_time {
            operating_state = MachineOperatingState::Operating;
            let output_fraction = (outer_time - material_input_outer_time) as f64
              / (material_output_outer_time - material_input_outer_time) as f64;
            let input_location = self.module_type.inputs[input_index]
              .outer_location
              .position
              .to_f64();
            let output_location = self.module_type.inputs[input_index]
              .inner_location
              .position
              .to_f64();
            let location =
              input_location * (1.0 - output_fraction) + output_location * output_fraction;
            materials.push((location, inner_input.material));
          }
        }
      }
    }

    for (output_index, inner_output) in self.internal_outputs(variation).iter().enumerate() {
      if let Some(inner_output) = inner_output {
        if let Some(material_inner_input_time) = inner_output.last_disbursement_time_lt(inner_time)
        {
          let material_outer_input_time =
            material_inner_input_time + module_machine_future.start_time;
          let material_outer_output_time = material_outer_input_time + TIME_TO_MOVE_MATERIAL;
          if outer_time <= material_outer_output_time {
            let output_fraction = (outer_time - material_outer_input_time) as f64
              / (material_outer_output_time - material_outer_input_time) as f64;
            let input_location = self.module_type.outputs[output_index]
              .inner_location
              .position
              .to_f64();
            let output_location = self.module_type.outputs[output_index]
              .outer_location
              .position
              .to_f64();
            let location =
              input_location * (1.0 - output_fraction) + output_location * output_fraction;
            materials.push((location, inner_output.material));
          }
        }
      }
    }

    MachineMomentaryVisuals {
      operating_state,
      materials,
    }
  }
}

fn check_module_output_flows(
  module: &PlatonicModule,
  inputs: MachineObservedInputs,
  module_machine_future: &ModuleMachineFuture,
  variation: &RegionFuture,
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

  let speed_leeway = 2.0 / TIME_TO_MOVE_MATERIAL as f64;

  for (output, output_location) in outputs.iter().zip(module.relative_output_locations()) {
    if let Some(output) = output {
      for i in 0..5 {
        let time = output.nth_disbursement_time(i).unwrap();
        let visuals =
          module.module_relative_momentary_visuals(inputs, module_machine_future, time, variation);
        lpt_assert!(
          has_near(&visuals, output.material, output_location, 0.0),
          "Outputs must be displayed at exactly the output location at disbursement times: {:?}",
          (i, time, output, output_location, visuals)
        );
        let visuals = module.module_relative_momentary_visuals(
          inputs,
          module_machine_future,
          time + 1,
          variation,
        );
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
    .zip(module.relative_input_locations())
  {
    if let Some(input) = input {
      for i in 0..5 {
        let time = input.nth_disbursement_time(i).unwrap();
        let visuals =
          module.module_relative_momentary_visuals(inputs, module_machine_future, time, variation);
        lpt_assert!(
          !has_near(&visuals, input.material, input_location, speed_leeway),
          "Inputs must not be displayed near the input location at disbursement times: {:?}",
          (i, time, input, input_location, visuals)
        );
        // can't always require displaying inputs, as some inputs may be deleted;
        // TODO come up with a way to selectively make the assertion for non-deleted inputs
        /*let visuals = module.module_relative_momentary_visuals(inputs, module_machine_future, time + 1, variation);
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

#[live_prop_test(use_trait_tests)]
impl MachineTypeTrait for PlatonicModule {
  // basic information
  fn name(&self) -> &str {
    &self.module_type.info.name
  }
  fn cost(&self) -> &[(Number, Material)] {
    &self.module_type.info.cost
  }
  fn num_inputs(&self) -> usize {
    self.module_type.inputs.len()
  }
  fn num_outputs(&self) -> usize {
    self.module_type.outputs.len()
  }
  fn radius(&self) -> Number {
    self.module_type.info.radius
  }
  fn icon(&self) -> &str {
    &self.module_type.info.icon
  }

  fn relative_input_locations(&self) -> Inputs<InputLocation> {
    self
      .module_type
      .inputs
      .iter()
      .map(|locations| locations.outer_location)
      .collect()
  }
  fn relative_output_locations(&self) -> Inputs<InputLocation> {
    self
      .module_type
      .outputs
      .iter()
      .map(|locations| locations.outer_location)
      .collect()
  }
  fn input_materials(&self) -> Inputs<Option<Material>> {
    self.module_type.inputs.iter().map(|_| None).collect()
  }

  type Future = ModuleMachineFuture;

  fn future(&self, inputs: MachineObservedInputs) -> Result<Self::Future, MachineOperatingState> {
    let output_availability_start = inputs
      .input_flows
      .iter()
      .flatten()
      .map(|material_flow| material_flow.first_disbursement_time_geq(inputs.start_time))
      .max()
      .unwrap_or(inputs.start_time)
      + TIME_TO_MOVE_MATERIAL;
    Ok(ModuleMachineFuture {
      canonical_inputs: inputs
        .input_flows
        .iter()
        .map(|material_flow| material_flow.and_then(canonical_module_input))
        .collect(),
      start_time: output_availability_start,
    })
  }

  fn output_flows(
    &self,
    _inputs: MachineObservedInputs,
    _future: &Self::Future,
  ) -> Inputs<Option<MaterialFlow>> {
    panic!("called Module::output_flows(); I'm using a hack where, for modules, you must use Module::module_output_flows instead");
  }

  fn relative_momentary_visuals(
    &self,
    _inputs: MachineObservedInputs,
    _future: &Self::Future,
    _time: Number,
  ) -> MachineMomentaryVisuals {
    panic!("called Module::momentary_visuals(); I'm using a hack where, for modules, you must use Module::module_momentary_visuals instead");
  }
}

struct ModuleCollector<'a> {
  machine_types: &'a MachineTypes,
  found_custom_modules: HashMap<usize, usize>,
}

impl<'a> ModuleCollector<'a> {
  fn visit_region(&mut self, region: &PlatonicRegionContents) {
    for machine in &region.machines {
      self.visit_machine(machine.type_id);
    }
  }

  fn visit_machine(&mut self, id: MachineTypeId) {
    if let MachineTypeId::Module(module_index) = id {
      let module = &self.machine_types.custom_modules[module_index];
      self.visit_region(&module.region);
      let num_found_modules = self.found_custom_modules.len();
      self
        .found_custom_modules
        .entry(module_index)
        .or_insert(num_found_modules);
    }
  }
}

impl PlatonicRegionContents {
  pub fn sort_canonically(&mut self) {
    self.machines.sort_by_key(PlatonicMachine::id_within_region)
  }
}

#[live_prop_test]
impl Game {
  /// Remove unused modules, and put them in a canonical ordering based on the order of machines in the regions.
  ///
  /// This is *required* after every Game change, for the purposes of the undo system.
  ///
  /// The ordering is guaranteed to go from contained to containing modules. Thus, if you iterate
  /// through the platonic modules in order, each one you visit contains only ones you've already
  /// visited.
  #[live_prop_test(postcondition = "self.is_canonical()")]
  pub fn canonicalize(&mut self) {
    self.global_region.sort_canonically();
    for module in &mut self.machine_types.custom_modules {
      module.region.sort_canonically();
    }

    let mut collector = ModuleCollector {
      machine_types: &self.machine_types,
      found_custom_modules: HashMap::with_capacity(self.machine_types.custom_modules.len()),
    };
    collector.visit_region(&self.global_region);
    let found_modules = collector.found_custom_modules;

    let mut new_modules: Vec<PlatonicModule> = (0..found_modules.len())
      .map(|_| PlatonicModule::default())
      .collect();
    for (&old_index, &new_index) in &found_modules {
      new_modules[new_index] = std::mem::take(&mut self.machine_types.custom_modules[old_index]);
    }
    self.machine_types.custom_modules = new_modules;

    for machine in self.platonic_machines_mut() {
      if let MachineTypeId::Module(module_index) = &mut machine.type_id {
        *module_index = found_modules[&module_index];
      }
    }
  }

  pub fn is_canonical(&self) -> bool {
    let mut canonicalized = self.clone();
    canonicalized.canonicalize();
    *self == canonicalized
  }
}
