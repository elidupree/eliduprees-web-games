use super::*;

use nalgebra::Vector2;
use arrayvec::ArrayVec;

type Number = i64;
const MAX_COMPONENTS: usize = 32;
const MAX_CYCLE_LENGTH: Number = 600;
const MAX_MACHINE_INPUTS: usize = 8;
type Inputs<T> = ArrayVec <[Number, MAX_MACHINE_INPUTS]>
type Position = Vector2 <Number>;


pub trait Machine {
  // basic information
  fn num_inputs (&self)->usize;
  fn num_outputs (&self)->usize;
  
  // used to infer group input flow rates
  // property: with valid inputs, the returned values have the same length given by num_inputs/num_outputs
  // property: these are consistent with each other
  fn max_output_rates (&self, input_rates: Inputs <Number>)->Inputs <Number>;
  // note: this API implies that mergers must have fixed ratios
  fn min_input_rates (&self, output_rates: Inputs <Number>)->Inputs <Number>;
  
  // property: if inputs don't change, current_output_rates doesn't change before next_output_change_time
  // property: an input changing to 0 causes at most one future change, ourselves changing to 0;
  //   and an input changing to non-0 causes at most 2 future changes, ourselves changing to 0 and then something else.
  //   This way, any manual change can only change each machine's output twice.
  // property: when there is no next output change time, current_output_rates is equivalent to max_output_rates
  // (Note, this property ALSO implies that mergers must have fixed ratios, or at least not tolerate an empty input)
  fn inputs_changed (&mut self, now: Number, input_patterns: Inputs <FlowPattern>);
  fn current_output_rates (&self, input_patterns: Inputs <FlowPattern>)->Inputs <FlowPattern>;
  fn next_output_change_time (&mut self, input_patterns: Inputs <FlowPattern>)->Option <Number>;
  fn next_output_change_time_reached (&mut self, input_patterns: Inputs <FlowPattern>);
}







pub struct FlowPattern {
  pub start_time: Number, //when the first item was disbursed as part of this flow
  pub rate: Number, //items per max cycle length
}

impl FlowPattern {
  pub fn num_disbursed_at_time (&self, time: Number)->bool {
    self.num_disbursed_before (time + 1) - self.num_disbursed_before (time)
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
  pub fn time_to_disburse_at_least (&self, collection_start_time: Number, amount: Number)->Number {
    
  }
}


pub fn entire_future (machines:, max_time: time) {
  for machine_info in machine_dag.forwards_iter_mut() {
    let mut simulation_time = now;
    loop {
      let change = first of machine_info.inputs.changes, machine_info.machine.next_output_change_time()
      match change {
        None => break,
        Some (time, change) => {
          if time > max_time {break;} 
          match change {
            OutputChange => {
              machine_info.machine.next_output_change_time_reached();
              machine_info.changes.push (machine_info.machine.current_output_rates());
            }
            InputChange => machine_info.machine.inputs_changed(),
          }
        }
      }
    }
  }
  changes map
}





enum MachineInputStorage {
  Unsettled {current_amount: Number},
  Equilibrium,
}

struct MachineInputState {
  input: FlowPattern,
  storage: MachineInputStorage,
}

struct MachineMaterialsState {
  output: FlowPattern,
  inputs: ArrayVec <[MachineInputState; MAX_MACHINE_INPUTS]>,
}

struct MachineWithState {
  map_state: &MachineMapState,
  materials: &mut MachineMaterialsState,
}

pub fn machine_step (now: Number, machine: MachineWithState, outputs: &mut [(MachineWithState, usize, usize)]) {
  inputs_are_equilibrium = machine.materials.inputs.all (| input | match input {Equilibrium {..} => true,_=> false});
  
  let mut can_start_ideal_rate = true;
  let mut ideal_rate = MAX_CYCLE_LENGTH/map_state.min_output_cycle_length;
  for (material_input, map_input) in machine.materials.inputs.iter().zip (machine.map_state.inputs.iter()) {
    ideal_rate = min (ideal_rate, material_input.input.rate/map_input.cost);
    match input.storage {
      Unsettled {current_amount} => {
        let capacity = map_input.capacity();
        current_amount += material_input.input.rate.num_disbursed_at_time (now);
        if current_amount < capacity {
          can_start_ideal_rate = false;
        }
      }
      _=>()
    }
  }
  
  if ideal_rate != machine.materials.output.rate && now >= machine.materials.output.last_disbursement_time() + map_state.min_output_cycle_length  {
    let actual_rate = if can_start_ideal_rate {ideal_rate} else {0};
    if actual_rate != machine.materials.output.rate {
      machine.materials.output = FlowPattern (start_time: now, rate: actual_rate};
      for (material_input, map_input) in machine.materials.inputs.iter().zip (machine.map_state.inputs.iter()) {
        material_input.storage.either_rate_changed(now);
      }  
    }
  }
  
  for (receiver_input, new_flow_rate) in outputs.iter_mut().zip (machine.map_state.output_patterns (machine.materials.output)) {
    if now >= new_flow_rate.start_time - 1 {
      receiver_input.set_rate (new_flow_rate);
    }
  }
  
  for (material_input, map_input) in machine.materials.inputs.iter().zip (machine.map_state.inputs.iter()) {
    match input.storage {
      Unsettled {current_amount} => {
        let last_rate_change_time = min (material_input.input.start_time, machine.materials.output.start_time);
        let capacity = map_input.capacity();
        current_amount -= map_input.cost*machine.materials.output.num_disbursed_at_time (now) ;
        if current_amount > capacity {
          current_amount = capacity;
        }
        let combined_cycle_length = num::integer::lcm (material_input.input.cycle_length(), machine.materials.output.cycle_length());
        if now == last_rate_change_time + combined_cycle_length {
          input.storage = MachineInputStorage::Equilibrium;
        }
      }
      _=>()
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
