use crate::flow_pattern::{
  CroppedFlow, Flow, FlowCollection, FlowPattern, MaterialFlow, RATE_DIVISOR,
};
use crate::geometry::{Number, VectorExtension};
use crate::machine_data::{
  InputLocation, Inputs, MachineMomentaryVisuals, MachineObservedInputs, MachineOperatingState,
  MachineType, MachineTypeTrait, Material, StandardMachineInfo, TIME_TO_MOVE_MATERIAL,
};
use live_prop_test::live_prop_test;
use nalgebra::Vector2;
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};
use std::convert::TryFrom;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct AssemblerInput {
  pub material: Material,
  pub cost: Number,
  pub location: InputLocation,
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct AssemblerOutput {
  pub material: Material,
  pub amount: Number,
  pub location: InputLocation,
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct Assembler {
  pub info: StandardMachineInfo,
  pub inputs: Inputs<AssemblerInput>,
  pub outputs: Inputs<AssemblerOutput>,
  pub assembly_duration: Number,
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct Distributor {
  pub info: StandardMachineInfo,
  pub inputs: Inputs<InputLocation>,
  pub outputs: Inputs<InputLocation>,
}

impl AssemblerInput {
  pub fn new(x: Number, y: Number, material: Material, cost: Number) -> AssemblerInput {
    AssemblerInput {
      location: InputLocation::input(x, y),
      material,
      cost,
    }
  }
}
impl AssemblerOutput {
  pub fn new(x: Number, y: Number, material: Material, amount: Number) -> AssemblerOutput {
    AssemblerOutput {
      location: InputLocation::output(x, y),
      material,
      amount,
    }
  }
}

pub fn conveyor() -> MachineType {
  MachineType::Distributor(Distributor {
    info: StandardMachineInfo::new("Conveyor", "conveyor", 1, vec![(1, Material::Iron)]),
    inputs: inputs![
      InputLocation::input(-1, 0),
      InputLocation::input(0, -1),
      InputLocation::input(0, 1),
    ],
    outputs: inputs![InputLocation::output(1, 0),],
  })
}

pub fn splitter() -> MachineType {
  MachineType::Distributor(Distributor {
    info: StandardMachineInfo::new("Splitter", "splitter", 1, vec![(1, Material::Iron)]),
    inputs: inputs![InputLocation::input(-1, 0),],
    outputs: inputs![InputLocation::output(0, 1), InputLocation::output(0, -1),],
  })
}

pub fn iron_smelter() -> MachineType {
  MachineType::Assembler(Assembler {
    info: StandardMachineInfo::new("Iron smelter", "machine", 3, vec![(5, Material::Iron)]),
    inputs: inputs![AssemblerInput::new(-3, 0, Material::IronOre, 3),],
    outputs: inputs![AssemblerOutput::new(3, 0, Material::Iron, 2),],
    assembly_duration: 10 * TIME_TO_MOVE_MATERIAL,
  })
}

pub fn iron_mine() -> MachineType {
  MachineType::Assembler(Assembler {
    info: StandardMachineInfo::new("Iron mine", "mine", 3, vec![(50, Material::Iron)]),
    inputs: inputs![],
    outputs: inputs![AssemblerOutput::new(3, 0, Material::IronOre, 1),],
    assembly_duration: TIME_TO_MOVE_MATERIAL,
  })
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct DistributorFuture {
  outputs: Inputs<FlowPattern>,
  output_availability_start: Number,
  material: Material,
}

#[live_prop_test(use_trait_tests)]
impl MachineTypeTrait for Distributor {
  fn name(&self) -> &str {
    &self.info.name
  }
  fn cost(&self) -> &[(Number, Material)] {
    &self.info.cost
  }
  fn num_inputs(&self) -> usize {
    self.inputs.len()
  }
  fn num_outputs(&self) -> usize {
    self.outputs.len()
  }
  fn radius(&self) -> Number {
    self.info.radius
  }
  fn icon(&self) -> &str {
    &self.info.icon
  }

  fn relative_input_locations(&self) -> Inputs<InputLocation> {
    self.inputs.clone()
  }
  fn relative_output_locations(&self) -> Inputs<InputLocation> {
    self.outputs.clone()
  }
  fn input_materials(&self) -> Inputs<Option<Material>> {
    self.inputs.iter().map(|_| None).collect()
  }

  type Future = DistributorFuture;

  fn future(&self, inputs: MachineObservedInputs) -> Result<Self::Future, MachineOperatingState> {
    let mut material_iterator = inputs
      .input_flows
      .iter()
      .flatten()
      .map(|material_flow| material_flow.material);
    let material = match material_iterator.next() {
      None => return Err(MachineOperatingState::InputMissing),
      Some(material) => {
        if material_iterator.all(|second| second == material) {
          material
        } else {
          return Err(MachineOperatingState::InputIncompatible);
        }
      }
    };

    let total_input_rate = inputs.input_flows.rate();

    let num_outputs = Number::try_from(self.outputs.len()).unwrap();
    let per_output_rate = min(
      RATE_DIVISOR / TIME_TO_MOVE_MATERIAL,
      total_input_rate / num_outputs,
    );
    if per_output_rate == 0 {
      return Err(MachineOperatingState::InputTooInfrequent);
    }
    let total_output_rate = per_output_rate * num_outputs;
    // the rounding here could theoretically be better, but this should be okay
    let latency_between_outputs = (RATE_DIVISOR + total_output_rate - 1) / total_output_rate;
    let output_availability_start = inputs
      .input_flows
      .iter()
      .flatten()
      .map(|material_flow| material_flow.first_disbursement_time_geq(inputs.start_time))
      .max()
      .unwrap();

    let first_output_start = output_availability_start + TIME_TO_MOVE_MATERIAL;

    let outputs = (0..self.outputs.len())
      .map(|index| {
        FlowPattern::new(
          first_output_start + Number::try_from(index).unwrap() * latency_between_outputs,
          per_output_rate,
        )
      })
      .collect();

    Ok(DistributorFuture {
      material,
      output_availability_start,
      outputs,
    })
  }

  fn output_flows(
    &self,
    _inputs: MachineObservedInputs,
    future: &Self::Future,
  ) -> Inputs<Option<MaterialFlow>> {
    let material = future.material;
    future
      .outputs
      .iter()
      .map(|&flow| Some(MaterialFlow { material, flow }))
      .collect()
  }

  fn relative_momentary_visuals(
    &self,
    inputs: MachineObservedInputs,
    future: &Self::Future,
    time: Number,
  ) -> MachineMomentaryVisuals {
    let output_disbursements_since_start = future
      .outputs
      .num_disbursed_between([inputs.start_time, time]);
    let mut materials = Vec::with_capacity(self.inputs.len() - 1);
    //let mut operating_state = MachineOperatingState::WaitingForInput;
    let output_rate = future.outputs.rate();
    let input_rate = inputs.input_flows.rate();
    let cropped_inputs: Inputs<_> = inputs
      .input_flows
      .iter()
      .map(|material_flow| {
        material_flow.map(|material_flow| CroppedFlow {
          flow: material_flow.flow,
          crop_start: material_flow
            .last_disbursement_time_leq(future.output_availability_start)
            .unwrap(),
        })
      })
      .collect();
    for output_index_since_start in output_disbursements_since_start.. {
      //input_rate may be greater than output_rate; if it is, we sometimes want to skip forward in the sequence. Note that if input_rate == output_rate, this uses the same index for both. Round down so as to use earlier inputs
      //TODO: wonder if there's a nice-looking way to make sure the deletions are distributed evenly over the inputs? (Right now when there is a simple 2-1 merge, everything from one side is deleted and everything from the other side goes through)
      let input_index_since_start = output_index_since_start * input_rate / output_rate;
      let (output_time, output_index) = future
        .outputs
        .nth_disbursement_geq_time(output_index_since_start, inputs.start_time)
        .unwrap();
      let (input_time, input_index) = cropped_inputs
        .nth_disbursement_geq_time(input_index_since_start, inputs.start_time)
        .unwrap();
      if input_time >= time {
        break;
      }
      //assert!(n <= previous_disbursements + self.inputs.len() + self.outputs.len() - 1);
      // TODO: smoother movement
      let input_location = self.inputs[input_index].position.to_f64();
      let output_location = self.outputs[output_index].position.to_f64();
      let output_fraction = (time - input_time) as f64 / (output_time - input_time) as f64;
      //println!("{:?}", (output_index_since_start, input_index_since_start, time, input_time, output_time, input_location, output_location, output_fraction));
      let location = input_location * (1.0 - output_fraction) + output_location * output_fraction;
      materials.push((location, future.material));
    }

    MachineMomentaryVisuals {
      operating_state: if output_disbursements_since_start > 0 {
        MachineOperatingState::Operating
      } else {
        MachineOperatingState::WaitingForInput
      },
      materials,
    }
  }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct AssemblerFuture {
  assembly_start_pattern: FlowPattern,
  outputs: Inputs<FlowPattern>,
}

#[live_prop_test(use_trait_tests)]
impl MachineTypeTrait for Assembler {
  // basic information
  fn name(&self) -> &str {
    &self.info.name
  }
  fn cost(&self) -> &[(Number, Material)] {
    &self.info.cost
  }
  fn num_inputs(&self) -> usize {
    self.inputs.len()
  }
  fn num_outputs(&self) -> usize {
    self.outputs.len()
  }
  fn radius(&self) -> Number {
    self.info.radius
  }
  fn icon(&self) -> &str {
    &self.info.icon
  }

  fn relative_input_locations(&self) -> Inputs<InputLocation> {
    self.inputs.iter().map(|a| a.location).collect()
  }
  fn relative_output_locations(&self) -> Inputs<InputLocation> {
    self.outputs.iter().map(|a| a.location).collect()
  }
  fn input_materials(&self) -> Inputs<Option<Material>> {
    self.inputs.iter().map(|a| Some(a.material)).collect()
  }

  type Future = AssemblerFuture;

  fn future(&self, inputs: MachineObservedInputs) -> Result<Self::Future, MachineOperatingState> {
    let mut assembly_rate = RATE_DIVISOR / self.assembly_duration;
    let mut assembly_start = inputs.start_time;
    for (input, material_flow) in self.inputs.iter().zip(inputs.input_flows) {
      // TODO: don't make the priority between the failure types be based on input order
      match material_flow {
        None => return Err(MachineOperatingState::InputMissing),
        Some(material_flow) => {
          if material_flow.material != input.material {
            return Err(MachineOperatingState::InputIncompatible);
          }
          assembly_rate = min(assembly_rate, material_flow.rate() / input.cost);
          assembly_start = max(
            assembly_start,
            material_flow
              .nth_disbursement_time_geq(input.cost - 1, inputs.start_time)
              .unwrap()
              + TIME_TO_MOVE_MATERIAL,
          );
        }
      }
    }

    if assembly_rate == 0 {
      return Err(MachineOperatingState::InputTooInfrequent);
    }

    let outputs = self
      .outputs
      .iter()
      .map(|output| {
        FlowPattern::new(
          assembly_start + self.assembly_duration + TIME_TO_MOVE_MATERIAL,
          assembly_rate * output.amount,
        )
      })
      .collect();

    Ok(AssemblerFuture {
      assembly_start_pattern: FlowPattern::new(assembly_start, assembly_rate),
      outputs,
    })
  }
  fn output_flows(
    &self,
    _inputs: MachineObservedInputs,
    future: &Self::Future,
  ) -> Inputs<Option<MaterialFlow>> {
    future
      .outputs
      .iter()
      .zip(&self.outputs)
      .map(|(&flow, output)| {
        Some(MaterialFlow {
          material: output.material,
          flow,
        })
      })
      .collect()
  }
  fn relative_momentary_visuals(
    &self,
    inputs: MachineObservedInputs,
    future: &Self::Future,
    time: Number,
  ) -> MachineMomentaryVisuals {
    // the output disbursement moment is the last moment we're responsible for displaying the material
    // materials can continue being outputted until the end of the NEXT assembly, plus TIME_TO_MOVE_MATERIAL
    // therefore,
    let first_relevant_assembly_start_index = max(
      0,
      future
        .assembly_start_pattern
        .num_disbursed_before(time - self.assembly_duration - TIME_TO_MOVE_MATERIAL)
        - 1,
    );

    let mut materials = Vec::with_capacity(self.inputs.len() + self.outputs.len() - 1);
    //let mut operating_state = MachineOperatingState::WaitingForInput;
    for assembly_start_index in first_relevant_assembly_start_index.. {
      let assembly_start_time = future
        .assembly_start_pattern
        .nth_disbursement_time(assembly_start_index)
        .unwrap();
      let assembly_finish_time = assembly_start_time + self.assembly_duration;
      let mut too_late = assembly_start_time >= time;

      if assembly_start_time >= time {
        for (input, material_flow) in self.inputs.iter().zip(inputs.input_flows) {
          let material_flow = material_flow.unwrap();
          let last_input_index = material_flow.num_disbursed_between([
            inputs.start_time,
            assembly_start_time - TIME_TO_MOVE_MATERIAL + 1,
          ]) - 1;
          for which_input in 0..input.cost {
            let input_index = last_input_index - which_input;
            let input_time = material_flow
              .nth_disbursement_time_geq(input_index, inputs.start_time)
              .unwrap();
            if input_time >= time {
              continue;
            }
            too_late = false;
            assert!(input_time < assembly_start_time);
            let input_location = input.location.position.to_f64();
            let assembly_location = Vector2::new(0.0, 0.0);
            let assembly_fraction =
              (time - input_time) as f64 / (assembly_start_time - input_time) as f64;
            let location =
              input_location * (1.0 - assembly_fraction) + assembly_location * assembly_fraction;
            materials.push((location, input.material));
          }
        }
      } else if assembly_finish_time <= time {
        for (output, flow) in self.outputs.iter().zip(&future.outputs) {
          let first_output_index = flow.num_disbursed_between([
            inputs.start_time,
            assembly_finish_time + TIME_TO_MOVE_MATERIAL,
          ]);
          for which_output in 0..output.amount {
            let output_index = first_output_index + which_output;
            let output_time = flow
              .nth_disbursement_time_geq(output_index, inputs.start_time)
              .unwrap();
            assert!(output_time >= assembly_finish_time + TIME_TO_MOVE_MATERIAL);
            if time <= output_time {
              let output_location = output.location.position.to_f64();
              let assembly_location = Vector2::new(0.0, 0.0);
              let assembly_fraction =
                (time - output_time) as f64 / (assembly_finish_time - output_time) as f64;
              let location =
                output_location * (1.0 - assembly_fraction) + assembly_location * assembly_fraction;
              materials.push((location, output.material));
            }
          }
        }
      } else {
        // hack, TODO better representation of the assembly being in progress
        materials.push((Vector2::new(0.0, 0.0), Material::Garbage));
      }

      if too_late {
        break;
      }
    }

    MachineMomentaryVisuals {
      operating_state: if time >= future.assembly_start_pattern.start_time() - TIME_TO_MOVE_MATERIAL
      {
        MachineOperatingState::Operating
      } else {
        MachineOperatingState::WaitingForInput
      },
      materials,
    }
  }
}
