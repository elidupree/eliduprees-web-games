use super::*;


use arrayvec::ArrayVec;
use std::collections::HashMap;
use std::cmp::{min, max};
use std::rc::Rc;


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ModuleType {
  pub name: String,
  pub icon: String,
  pub radius: Number,
  pub inner_radius: Number,
  pub cost: Vec<(Number, Material)>,
  pub inputs: Inputs <(Vector, Facing)>,
  pub outputs: Inputs <(Vector, Facing)>,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ModuleInput {
  pub material: Option <Material>,
  pub ideal_rate: Number,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ModuleOutput {
  pub material: Option <Material>,
  pub ideal_rate: Number,
}


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct Module {
  pub module_type: ModuleType,
  pub cost: Vec<(Number, Material)>,
  pub inputs: Inputs <ModuleInput>,
  pub outputs: Inputs <ModuleOutput>,
  pub machines: ArrayVec <[StatefulMachine; MAX_COMPONENTS]>,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ModuleMachine {
  pub module: Rc<Module>,
}

pub fn basic_module()->MachineType {
  MachineType::ModuleMachine(ModuleMachine {
    module: Rc::new (Module {
      module_type: ModuleType {
        name: "Basic module".to_string(),
        icon: "rounded-rectangle-solid".to_string(),
        radius: 20,
        inner_radius: 18,
        cost: vec![(20, Material::Iron)],
        inputs: inputs![],
        outputs: inputs![],
      },
      cost: vec![(20, Material::Iron)],
      inputs: inputs![],
      outputs: inputs![],
      machines: ArrayVec::new(),
    })
  })
}

impl Module {
  fn max_operating_rate (&self)->Number {
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
  }
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
      facing: map_state.facing,
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


