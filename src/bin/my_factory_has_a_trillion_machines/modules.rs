#![allow(unused_imports)]
use arrayvec::ArrayVec;
//use std::collections::HashMap;
use std::cmp::{min, max};
use std::rc::Rc;

use geometry::{Number, Vector, Facing};
use flow_pattern::{FlowPattern, RATE_DIVISOR};
use machine_data::{Inputs, MachineType, Material, MachineTypeTrait, MachineMapState, MachineMaterialsState, StatefulMachine, Map, DrawnMachine, MAX_COMPONENTS};
use graph_algorithms::{MapFuture};


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
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


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct Module {
  pub module_type: ModuleType,
  pub cost: Vec<(Number, Material)>,
  pub map: Map,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ModuleFuture {
  map: MapFuture,
  outputs: Inputs <MaterialFlow>,
}




pub fn basic_module()->MachineType {
  fn input(index: Number, x: Number)->ModuleInput {
    ModuleInput {
      outer_location: InputLocation::new (x + x.signum(), -3 + index*2, 0),
      inner_location: InputLocation::new (x - x.signum(), -3 + index*2, 0),
    }
  }

  MachineType::ModuleMachine(ModuleMachine {
    module: Rc::new (Module {
      module_type: ModuleType {
        info: StandardMachineInfo::new ("Basic module", "rounded-rectangle-solid", 20, vec![(20, Material::Iron)])
        inner_radius: 18,
        inputs: (0..4).map(|i| input(i, -19)).collect(),
        outputs: (0..4).map(|i| input(i, 19)).collect(),
      },
      cost: vec![(20, Material::Iron)],
      map: Map{machines: ArrayVec::new()},
    })
  })
}



pub struct CanonicalModuleInputs {
  inputs: Inputs <Option <MaterialFlowRate>>,
}

pub fn canonical_module_input (input: MaterialFlow)->Option <MaterialFlowRate> {
  const STANDARD_RATE = RATE_DIVISOR / TIME_TO_MOVE_MATERIAL;
  
  // TODO: somehow decide on a more principled choice of rates
  const PERMITTED_RATES: & 'static Number = [
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
  
  MaterialFlowRate {material: input.material, rate: FlowRate::new (rounded_down)}
}

pub fn canonical_module_inputs (inputs: MachineObservedInputs)->(Number, CanonicalModuleInputs) {
  let output_availability_start = inputs.input_flows.iter().flatten().map (| material_flow | material_flow.first_disbursement_time_geq (inputs.start_time)).max ().unwrap() + TIME_TO_MOVE_MATERIAL;
  (
    output_availability_start,
    CanonicalModuleInputs {
      inputs: inputs.input_flows.iter().map (| material_flow | material_flow.and_then (canonical_module_input)).collect()
    }
  )
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
  fn input_materials (&self)->Inputs <Option <Material>> {self.inputs.iter().map (|_| None).collect()}
  
  
  type Future = ModuleFuture;
  
  fn future (&self, inputs: MachineObservedInputs, module_futures: &mut ModuleFutures)->Result <Self::Future, MachineOperatingState> {
    // note: the graph algorithms rely on this to be dependent only on the canonical inputs, so in THIS function, we shadow `inputs` (and discard the start time) to prevent accidentally relying on the noncanonical values
    let (_, inputs) = canonical_module_inputs (inputs);
    
    let output_edges = self.map.output_edges(& game.machine_types) ;
    let ordering = self.map.topological_ordering_of_noncyclic_machines (& output_edges);
    let fiat_inputs: Vec<_> = self.relative_input_locations().into_iter().zip (inputs.inputs).collect();
    let future = self.map.future (& game.machine_types, & output_edges, & ordering, module_futures, & fiat_inputs);
        
    Ok (ModuleFuture {
      map: future,
      outputs: self.relative_output_locations ().map (| output_location | {
        future.map.dumped.iter().find (| (location,_) | location == output_location).map (| (_, flow) | *flow)
      })
    })
  }
  
  fn output_flows(&self, inputs: MachineObservedInputs, future: &Self::Future)->Inputs <Option<MaterialFlow>> {
    // TODO: the fact that this line gets repeated in all 3 functions means that it should be handled the way we now handle Future for the other machine types, but in modules it is currently hacked to use the Future associated type for the shared module future
    let (module_start,_) = canonical_module_inputs (inputs);
    future.outputs.map (| output | output.delayed_by (module_start)).collect()
  }
  
  fn momentary_visuals(&self, inputs: MachineObservedInputs, future: &Self::Future, time: Number)->MachineMomentaryVisuals {
    let (module_start, canonical_inputs) = canonical_module_inputs (inputs);
    let inner_time = time - module_start;
    let mut materials = Vec::with_capacity(self.module_type.inputs.len() + self.module_type.outputs.len()) ;
    
    for (input_index, (exact_input, canonical_input)) in inputs.input_flows.iter().zip (canonical_inputs.inputs).enumerate() {
      let output_disbursements_since_start = canonical_input.num_disbursed_before (inner_time);
      
      // TODO: wait, surely each of these can only have one moving material at a time? So it shouldn't need to be a loop?
      for output_disbursement_index in output_disbursements_since_start .. {
        let output_time = canonical_input.nth_disbursement_time(output_disbursement_index) + module_start;
        let input_time = exact_input.last_disbursement_time_leq (input_time - TIME_TO_MOVE_MATERIAL).unwrap();
        if input_time > time {break}
        let output_fraction = (time - input_time) as f64/(output_time - input_time) as f64;
        let input_location = self.module_type.inputs [input_index].outer_location.to_f64 ();
        let output_location = self.module_type.inputs [input_index].inner_location.to_f64 ();
        let location = input_location*(1.0 - output_fraction) + output_location*output_fraction;
        materials.push ((location, future.material));
      }
    }
    
    for (output_index, canonical_output) in future.outputs.iter().enumerate() {
      let disbursements_since_start = canonical_output.num_disbursed_before (inner_time - TIME_TO_MOVE_MATERIAL);
      
      for disbursement_index in disbursements_since_start .. {
        let input_time = canonical_output.nth_disbursement_time(disbursement_index) + module_start;
        if input_time > time {break}
        let output_time = input_time + TIME_TO_MOVE_MATERIAL;
        let output_fraction = (time - input_time) as f64/(output_time - input_time) as f64;
        let input_location = self.module_type.outputs [output_index].inner_location.to_f64 ();
        let output_location = self.module_type.outputs [output_index].outer_location.to_f64 ();
        let location = input_location*(1.0 - output_fraction) + output_location*output_fraction;
        materials.push ((location, future.material));
      }
    }
    
    MachineMomentaryVisuals {
      operating_state: if output_disbursements_since_start > 0 {MachineOperatingState::Operating} else {MachineOperatingState::WaitingForInput},
      materials,
    }
  }
}



struct ModuleCollector <'a> {
  machine_types: & 'a MachineTypes,
  found_modules: HashMap <& 'a Module, MachineTypeId>,
  next_index: usize,
  map_queue: VecDeque<&'a Map>,
  new_ids: Vec<Option <MachineTypeId>>,
}

impl ModuleCollector <'a> {
  fn find_machine (&mut self, id: MachineTypeId) {
    if let MachineTypeId::Module (module_index) = id {
      let new_id = *self.found_modules.entry (module).or_insert_with (|| {
        let result = MachineTypeId::Module (self.next_index);
        self.next_index += 1;
        self.map_queue.push_back (&module.map);
        result
      });
      self.new_ids [module_index] = new_id;
    }
  }
  
  fn new(game: &'a Game) -> ModuleCollector<'a> {
    let collector = ModuleCollector {
      machine_types: & game.machine_types,
      found_modules: HashMap::with_capacity (game.machine_types.modules.len()),
      next_index: 0,
      map_queue: VecDeque::with_capacity (game.machine_types.modules.len()),
      new_ids: vec![None; self.machine_types.modules.len()) 
    }
    collector.map_queue.push_back (& game.map);
    for (index, preset) in game.machine_types.presets.iter().enumerate() {
      if let MachineType::Module (module) = preset {
        self.found_modules.insert (module, MachineTypeId::Preset (index));
      }
    }
    collector
  }
  
  fn run(&mut self) {
    while let Some (map) = self.map_queue.pop () {
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
    
    let new_modules = (0..new_count).map (|_| Module::default());
    for (module, new_id) in std::mem::take (self.machine_types.modules).into_iter().zip (new_indices) {
      if let Some(MachineTypeId::Module (new_module_index)) = new_id {
        new_modules [new_module_index] = module;
      }
    }
    self.machine_types.modules = new_modules;
    
    for map in iter::once (&mut self.map).chain (self.machine_types.modules.iter_mut().map (| module | &mut module.map)) {
      for machine in map.machines {
        if let MachineTypeId::Module (module_index) = machine.type_id {
          machine.type_id = new_ids [module_index];
        }
      }
    }
  }
}





impl Module {
  /*fn max_operating_rate (&self)->Number {
    RATE_DIVISOR
  }
  fn max_operating_rate_with_inputs <I: IntoIterator <Item = (Number, Material)>> (&self, input_rates: I)->Number {
    let mut ideal_rate = self.max_operating_rate();
    for ((rate, material), input) in input_rates.into_iter().zip(&self.inputs) {
      let allowed_material = input.material.unwrap_or (material) == material;
      let inferred_rate = if allowed_material {rate} else {0};
      ideal_rate = min (ideal_rate, inferred_rate);
    }
    ideal_rate
  }*/
  /*fn min_operating_rate_to_produce <I: IntoIterator <Item = Number>> (&self, output_rates: I)->Number {
    output_rates.into_iter().zip (&self.outputs).map (| (rate, output) | {
      
    }).max().unwrap_or_else(|| self.max_output_rate())
  }*/
  
  /*fn input_storage_before_impl (&self, input_patterns: & [(FlowPattern, Material)], output_pattern: FlowPattern, starting_storage: & [(Number, Material)], interval: [Number; 2])->Inputs <(Number, Material)> {
    let output_disbursements = output_pattern.num_disbursed_between (interval);

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
  }*/
  
  /*pub fn input_storage_before (&self, state: &MachineMaterialsState, input_patterns: & [(FlowPattern, Material)], time: Number)->Inputs <(Number, Material)> {
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
  }*/
}

impl MachineTypeTrait for ModuleMachine {
  fn name (&self)->& str {&self.module.module_type.name}
  fn cost (&self)->Vec<(Number, Material)> {self.module.cost.clone()}
  //fn num_inputs (&self)->usize {self.module.module_type.inputs.len()}
  //fn num_outputs (&self)->usize {self.module.module_type.outputs.len()}
  fn radius (&self)->Number {self.module.module_type.radius}
  
  /*fn input_locations (&self, state: &MachineMapState)->Inputs <(Vector, Facing)> {
    self.module.module_type.inputs.iter().map (| input | {
      let (position, facing) = input.relative_location.rotate_90 (state.facing);
      (position + state.position, facing)
    }).collect()
  }
  fn output_locations (&self, state: &MachineMapState)->Inputs <(Vector, Option<Facing>)> {
    self.module.module_type.outputs.iter().map (| output | {
      let (position, facing) = output.relative_location.rotate_90 (state.facing);
      (position + state.position, facing)
    }).collect()
  }
  
  fn input_materials (&self)->Inputs <Option <Material>> {
    self.module.input_materials.clone()
  }
  
  fn displayed_storage (&self, map_state: & MachineMapState, materials_state: & MachineMaterialsState, input_patterns: & [(FlowPattern, Material)], time: Number)->Inputs <(Vector, (Number, Material))> {
    self.input_storage_before (materials_state, input_patterns, time).into_iter().zip (self.input_locations (map_state)).map (| ((amount, material), (position,_facing)) | (position, (amount, material))).collect()
  }*/
  fn drawn_machine (&self, map_state: & MachineMapState)->DrawnMachine {
    DrawnMachine {
      icon: self.module.module_type.icon.clone(),
      position: map_state.position,
      size: Vector::new (self.module.module_type.radius*2, self.module.module_type.radius*2),
    }
  }
  /*
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
  */
  /*fn future_output_patterns (&self, state: &MachineMaterialsState, input_patterns: & [(FlowPattern, Material)])->Inputs <ArrayVec<[(Number, (FlowPattern, Material)); MAX_IMPLICIT_OUTPUT_FLOW_CHANGES]>> {
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
  }*/
}


