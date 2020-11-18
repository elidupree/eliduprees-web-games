#![allow(unused_imports)]
use arrayvec::ArrayVec;
use live_prop_test::live_prop_test;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;

use flow_pattern::{
  Flow, FlowCollection, FlowPattern, FlowRate, MaterialFlow, MaterialFlowRate, RATE_DIVISOR,
};
use geometry::{Facing, Number, Vector, VectorExtension};
use graph_algorithms::RegionFuture;
use machine_data::{
  Game, InputLocation, Inputs, MachineMomentaryVisuals, MachineObservedInputs,
  MachineOperatingState, MachineType, MachineTypeId, MachineTypeTrait, MachineTypes, Material,
  PlatonicMachine, PlatonicRegionContents, StandardMachineInfo, MAX_COMPONENTS,
  TIME_TO_MOVE_MATERIAL,
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
        let material_inner_output_time =
          inner_input.first_disbursement_time_geq(max(0, inner_time));
        let material_outer_output_time =
          material_inner_output_time + module_machine_future.start_time;
        let material_outer_input_time = outer_input
          .last_disbursement_time_leq(material_outer_output_time - TIME_TO_MOVE_MATERIAL)
          .unwrap();

        // >, not >=; don't draw at the moment of input, fitting the general rule that the source machine draws the material
        if outer_time > material_outer_input_time {
          operating_state = MachineOperatingState::Operating;
          let output_fraction = (outer_time - material_outer_input_time) as f64
            / (material_outer_output_time - material_outer_input_time) as f64;
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
