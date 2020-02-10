#![allow(unused_imports)]
use arrayvec::ArrayVec;
use std::collections::HashMap;
use std::cmp::{min, max};
use std::rc::Rc;
use std::collections::VecDeque;

use geometry::{Number, Vector, Facing, VectorExtension};
use flow_pattern::{FlowPattern, Flow, FlowRate, FlowCollection, MaterialFlow, MaterialFlowRate, RATE_DIVISOR};
use machine_data::{Inputs, MachineType, Material, StandardMachineInfo, MachineMomentaryVisuals, MachineObservedInputs, MachineOperatingState, InputLocation, MachineTypeId, MachineTypeTrait, MachineTypes, StatefulMachine, Map, MAX_COMPONENTS, Game, TIME_TO_MOVE_MATERIAL};
use graph_algorithms::{MapFuture};


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct ModuleType {
  pub info: StandardMachineInfo,
  pub inner_radius: Number,
  pub inputs: Inputs <ModuleInput>,
  pub outputs: Inputs <ModuleInput>,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ModuleInput {
  pub outer_location: InputLocation,
  pub inner_location: InputLocation,
}


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct Module {
  pub module_type: ModuleType,
  pub cost: Vec<(Number, Material)>,
  pub map: Map,
}


pub fn basic_module()->MachineType {
  fn input(index: Number, x: Number)->ModuleInput {
    ModuleInput {
      outer_location: InputLocation::new (x + x.signum(), -3 + index*2, 0),
      inner_location: InputLocation::new (x - x.signum(), -3 + index*2, 0),
    }
  }
  
  let cost = vec![(20, Material::Iron)];

  MachineType::Module(Module {
    module_type: ModuleType {
      info: StandardMachineInfo::new ("Basic module", "rounded-rectangle-solid", 20, cost.clone()),
      inner_radius: 18,
      inputs: (0..4).map(|i| input(i, -19)).collect(),
      outputs: (0..4).map(|i| input(i, 19)).collect(),
    },
    cost,
    map: Map{machines: Vec::new()},
  })
}



pub type CanonicalModuleInputs = Inputs <Option <MaterialFlowRate>>;
#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ModuleMachineFuture {
  pub canonical_inputs: CanonicalModuleInputs,
  pub start_time: Number,
}

pub fn canonical_module_input (input: MaterialFlow)->Option <MaterialFlowRate> {
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
  
  let rounded_down = match PERMITTED_RATES.binary_search(& input.rate()) {
    Ok (index) => PERMITTED_RATES [index],
    // something smaller than the minimum permitted rate can't flow at all and returns None
    Err (index) => PERMITTED_RATES [index.checked_sub (1)?],
  };
  
  Some(MaterialFlowRate {material: input.material, flow: FlowRate::new (rounded_down)})
}


impl Module {
  fn internal_outputs(&self, variation: &MapFuture)->Inputs <Option <MaterialFlow>> {
    self.relative_output_locations ().into_iter().map (| output_location | {
      variation.dumped.iter().find (| (location,_) | *location == output_location).map (| (_, flow) | *flow)
    }).collect()
  }
  
  pub fn module_output_flows(&self, _inputs: MachineObservedInputs, module_machine_future: & ModuleMachineFuture, variation: & MapFuture)->Inputs <Option<MaterialFlow>> {
    self.internal_outputs(variation).into_iter().map (| output | output.map(|output| output.delayed_by (module_machine_future.start_time))).collect()
  }
  
  pub fn module_momentary_visuals(&self, inputs: MachineObservedInputs, module_machine_future: & ModuleMachineFuture, time: Number, variation: & MapFuture)->MachineMomentaryVisuals {
    let inner_time = time - module_machine_future.start_time;
    let mut materials = Vec::with_capacity(self.module_type.inputs.len() + self.module_type.outputs.len());
    let mut operating_state = MachineOperatingState::WaitingForInput;
    
    for (input_index, (exact_input, canonical_input)) in inputs.input_flows.iter().zip (&module_machine_future.canonical_inputs).enumerate() {
      if let (Some (exact_input), Some(canonical_input)) = (exact_input, canonical_input) {
      
      let output_disbursements_since_start = canonical_input.num_disbursed_before (inner_time);
      if output_disbursements_since_start > 0 {operating_state = MachineOperatingState::Operating}
            
      // TODO: wait, surely each of these can only have one moving material at a time? So it shouldn't need to be a loop?
      for output_disbursement_index in output_disbursements_since_start .. {
        let output_time = canonical_input.nth_disbursement_time(output_disbursement_index).unwrap() + module_machine_future.start_time;
        assert!(output_time >= module_machine_future.start_time);
        let input_time = exact_input.last_disbursement_time_leq (output_time - TIME_TO_MOVE_MATERIAL).unwrap();
        if input_time > time {break}
        let output_fraction = (time - input_time) as f64/(output_time - input_time) as f64;
        let input_location = self.module_type.inputs [input_index].outer_location.position.to_f64 ();
        let output_location = self.module_type.inputs [input_index].inner_location.position.to_f64 ();
        let location = input_location*(1.0 - output_fraction) + output_location*output_fraction;
        materials.push ((location, canonical_input.material));
      }
      }
    }
    
    for (output_index, canonical_output) in self.internal_outputs (variation).iter().enumerate() {
      if let Some(canonical_output) = canonical_output {
      
      let disbursements_since_start = canonical_output.num_disbursed_before (inner_time - TIME_TO_MOVE_MATERIAL);
      
      for disbursement_index in disbursements_since_start .. {
        let input_time = canonical_output.nth_disbursement_time(disbursement_index).unwrap() + module_machine_future.start_time;
        if input_time > time {break}
        let output_time = input_time + TIME_TO_MOVE_MATERIAL;
        let output_fraction = (time - input_time) as f64/(output_time - input_time) as f64;
        let input_location = self.module_type.outputs [output_index].inner_location.position.to_f64 ();
        let output_location = self.module_type.outputs [output_index].outer_location.position.to_f64 ();
        let location = input_location*(1.0 - output_fraction) + output_location*output_fraction;
        materials.push ((location, canonical_output.material));
      }
      }
    }
    
    MachineMomentaryVisuals {
      operating_state,
      materials,
    }
  }
}

impl MachineTypeTrait for Module {
  // basic information
  fn name (&self)->& str {& self.module_type.info.name}
  fn cost (&self)->& [(Number, Material)] {& self.module_type.info.cost}
  fn num_inputs (&self)->usize {self.module_type.inputs.len()}
  fn num_outputs (&self)->usize {self.module_type.outputs.len()}
  fn radius (&self)->Number {self.module_type.info.radius}
  fn icon(&self) ->& str {& self.module_type.info.icon}
  
  fn relative_input_locations (&self)->Inputs <InputLocation> {self.module_type.inputs.iter().map (| locations | locations.outer_location).collect()}
  fn relative_output_locations (&self)->Inputs <InputLocation> {self.module_type.outputs.iter().map (| locations | locations.outer_location).collect()}
  fn input_materials (&self)->Inputs <Option <Material>> {self.module_type.inputs.iter().map (|_| None).collect()}
  
  
  type Future = ModuleMachineFuture;
  
  fn future (&self, inputs: MachineObservedInputs)->Result <Self::Future, MachineOperatingState> {
    let output_availability_start = inputs.input_flows.iter().flatten().map (| material_flow | material_flow.first_disbursement_time_geq (inputs.start_time)).max ().unwrap_or(inputs.start_time) + TIME_TO_MOVE_MATERIAL;
    Ok(ModuleMachineFuture {
      canonical_inputs: inputs.input_flows.iter().map (| material_flow | material_flow.and_then (canonical_module_input)).collect(),
      start_time: output_availability_start,
    })
  }
  
  fn output_flows(&self, _inputs: MachineObservedInputs, _future: &Self::Future)->Inputs <Option<MaterialFlow>> {
    panic!("called Module::output_flows(); I'm using a hack where, for modules, you must use Module::module_output_flows instead");
  }
  
  fn momentary_visuals(&self, _inputs: MachineObservedInputs, _future: &Self::Future, _time: Number)->MachineMomentaryVisuals {
    panic!("called Module::momentary_visuals(); I'm using a hack where, for modules, you must use Module::module_momentary_visuals instead");
  }
}



struct ModuleCollector <'a> {
  machine_types: & 'a MachineTypes,
  found_modules: HashMap <& 'a Module, MachineTypeId>,
  next_index: usize,
  map_queue: VecDeque<&'a Map>,
  new_ids: Vec<Option <MachineTypeId>>,
}

impl<'a> ModuleCollector <'a> {
  fn find_machine (&mut self, id: MachineTypeId) {
    if let MachineTypeId::Module (module_index) = id {
      let module = & self.machine_types.modules [module_index];
      let next_index = &mut self.next_index;
      let map_queue = &mut self.map_queue;
      let new_id = *self.found_modules.entry (module).or_insert_with (|| {
        let result = MachineTypeId::Module (*next_index);
        *next_index += 1;
        map_queue.push_back (&module.map);
        result
      });
      self.new_ids [module_index] = Some (new_id);
    }
  }
  
  fn new(game: &'a Game) -> ModuleCollector<'a> {
    let mut collector = ModuleCollector {
      machine_types: & game.machine_types,
      found_modules: HashMap::with_capacity (game.machine_types.modules.len()),
      next_index: 0,
      map_queue: VecDeque::with_capacity (game.machine_types.modules.len()),
      new_ids: vec![None; game.machine_types.modules.len()],
    };
    collector.map_queue.push_back (& game.map);
    for (index, preset) in game.machine_types.presets.iter().enumerate() {
      if let MachineType::Module (module) = preset {
        collector.found_modules.insert (module, MachineTypeId::Preset (index));
      }
    }
    collector
  }
  
  fn run(&mut self) {
    while let Some (map) = self.map_queue.pop_front () {
      for machine in & map.machines {
        self.find_machine (machine.type_id);
      }
    }
  }
}

impl Game {
  /// Deduplicate modules, remove unused modules, and put them in a canonical ordering based on the order of machines on the maps.
  pub fn cleanup_modules (&mut self) {
    let mut collector = ModuleCollector::new (self);
    collector.run();
    let new_ids = collector.new_ids;
    let new_count = collector.next_index;
    
    let mut new_modules: Vec<Module> = (0..new_count).map (|_| Module::default()).collect();
    for (module, new_id) in std::mem::replace(&mut self.machine_types.modules, Default::default()).into_iter().zip (&new_ids) {
      if let Some(MachineTypeId::Module (new_module_index)) = new_id {
        new_modules [*new_module_index] = module;
      }
    }
    self.machine_types.modules = new_modules;
    
    for map in std::iter::once (&mut self.map).chain (self.machine_types.modules.iter_mut().map (| module | &mut module.map)) {
      for machine in &mut map.machines {
        if let MachineTypeId::Module (module_index) = machine.type_id {
          machine.type_id = new_ids [module_index].unwrap();
        }
      }
    }
  }
}



