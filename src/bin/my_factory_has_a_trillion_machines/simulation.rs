use super::*;

use nalgebra::Vector2;
use arrayvec::ArrayVec;

type Number = i64;
const MAX_COMPONENTS: usize = 32;
const MAX_CYCLE_LENGTH: Number = 600;
cost MAX_MACHINE_INPUTS: usize = MAX_COMPONENTS;


pub struct FlowPattern {
  pub start_time: Number, //when the first item was disbursed as part of this flow
  pub rate: Number, //items per max cycle length
}

impl FlowPattern {
  pub fn disburses_at_time (&self, time: Number)->bool {
    self.num_disbursed_before (time + 1) > self.num_disbursed_before (time)
  }
  pub fn num_disbursed_before (&self, time: Number)->Number {
    ((time - self.start_time)*self.rate + MAX_CYCLE_LENGTH)/MAX_CYCLE_LENGTH
  }
  pub fn num_disbursed_between (&self, range: [Number; 2])->Number {
    self.num_disbursed_before (range [1]) - self.num_disbursed_before (range [0])
  }
  pub fn cycle_length (&self)->Number {
    num::integer::gcd (self.rate, MAX_CYCLE_LENGTH)
  }
}

enum MachineOutputState {
  Equilibrium (FlowPattern),
  Unsettled {last_output_start_time: Option <Number>},
}

enum MachineInputStorageAmount {
  Unsettled {current_amount: Number},
  Equilibrium {amount_at_cycle_start: Number},
}

struct MachineInputStorageState {
  storage: MachineInputStorage,
  inputs_contributing: ArrayVec <[Option <FlowPattern>; MAX_MACHINE_INPUTS]>,
}

struct MachineMaterialsState {
  output: MachineOutputState,
  inputs: ArrayVec <[MachineInputState; MAX_MACHINE_INPUTS]>,
}

struct MachineWithState {
  map_state: &MachineMapState,
  materials: &mut MachineMaterialsState,
}

pub fn machine_step (now: Number, machine: MachineWithState, outputs: &mut [(MachineWithState, usize, usize)]) {
  inputs_are_equilibrium = machine.materials.inputs.all (| input | match input {Equilibrium {..} => true,_=> false});
  
  let mut inputs_are_equilibrium = true;
  let mut can_produce = match machine.materials.output {
    Unsettled {last_output_start_time} => {
      (now - last_output_start_time) >= machine.map_state.minimum_cycle_length()
    },
    
  };
  for (material_input, map_input) in machine.materials.inputs.iter().zip (machine.map_state.inputs.iter()) {
    if material_input.iter().any (Option::is_none) {inputs_are_equilibrium = false;}
    match input.storage {
      Unsettled {current_storage} => {
        let requirement = map_input.requirement() + material_input.inputs_contributing.len();
        if current_storage < requirement {
          can_produce = false;
        }
      }
      _=>()
    }
  }
  
  match machine.materials.output {
    MachineOutputState::Equilibrium (flow_pattern) {
      if !inputs_are_equilibrium {
        machine.materials.output = MachineOutputState::Unsettled {
          last_output_start_time: flow_pattern.most_recent_item()
        }
      }
    },
    MachineOutputState::Unsettled {last_output_start_time} => {
      if can_produce {
        if inputs_are_equilibrium {
          machine.materials.output = MachineOutputState::Equilibrium ();
        }
        else {
          machine.materials.output = MachineOutputState::Unsettled {
            last_output_start_time: now
          }
        }
      }
    }
  }
  
  let last_output_start_time = machine.materials.output.last_output_start_time();
  
  for ((output_machine, storage_index, my_index), output_material) in output.iter_mut().zip (machine.map_state.output_materials (now - last_output_start_time)) {
    match output_machine.materials.inputs [storage_index].inputs_contributing [my_index] {
      Some (pattern) => {
        if output_material != pattern.disburses_at_time (now) {
          output_machine.materials.inputs [storage_index].inputs_contributing [my_index] = None;
        }
      }
      None => {
        if
      }
    }
  }
}



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
  facing: u8,
  component_type: ComponentType,
}

pub struct Group {
  size: Vector2 <Length>,
  components: ArrayVec <[Component; MAX_COMPONENTS]>,
  average_color: [f64; 3],
}


pub struct Map {
  components: ArrayVec <[Component; MAX_COMPONENTS]>,
}



fn step_component (component:) {
  match component.component_type {
    ComponentType::Singular (SingularComponentType::Conveyor) => {
      destination.push_material (mem::replace (&mut component.carried, None));
    },
    
  }
}



fn step (component_graph: ?????) {
  for component in component_graph.iterate_from_lasts() {
    component.step();
  }
}
