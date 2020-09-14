//use std::cmp::{min, max};
use std::collections::{hash_map, HashMap};

use arrayvec::ArrayVec;

use flow_pattern::{FlowCollection, FlowPattern, MaterialFlow};
use geometry::Number;
use machine_data::{
  Game, InputLocation, Inputs, MachineFuture, MachineObservedInputs, MachineOperatingState,
  MachineTypeId, MachineTypeRef, MachineTypeTrait, MachineTypes, Map, Material, StatefulMachine,
  MAX_COMPONENTS,
};
use modules::CanonicalModuleInputs;

pub type OutputEdges = ArrayVec<[Inputs<Option<(usize, usize)>>; MAX_COMPONENTS]>;
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MapFuture {
  pub machines: Vec<MachineAndInputsFuture>,
  pub dumped: Vec<(InputLocation, MaterialFlow)>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MachineAndInputsFuture {
  pub inputs: Inputs<Option<MaterialFlow>>,
  // Hack: only non-module machines have their futures stored here; operating modules are listed as Err (Operating)
  pub future: Result<MachineFuture, MachineOperatingState>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ModuleFuture {
  pub output_edges: OutputEdges,
  pub topological_ordering: Vec<usize>,
  pub future_variations: HashMap<CanonicalModuleInputs, MapFuture>,
}

pub type ModuleFutures = HashMap<MachineTypeId, ModuleFuture>;

impl Map {
  pub fn output_edges(&self, machine_types: &MachineTypes) -> OutputEdges {
    self
      .machines
      .iter()
      .map(|machine| {
        machine_types
          .output_locations(machine)
          .map(|output_location| {
            self
              .machines
              .iter()
              .enumerate()
              .find_map(|(machine2_index, machine2)| {
                machine_types
                  .input_locations(machine2)
                  .enumerate()
                  .find_map(|(input_index, input_location)| {
                    if input_location == output_location {
                      Some((machine2_index, input_index))
                    } else {
                      None
                    }
                  })
              })
          })
          .collect()
      })
      .collect()
  }

  pub fn build_machines(
    &mut self,
    machine_types: &mut MachineTypes,
    machines: impl IntoIterator<Item = StatefulMachine>,
    now: Number,
  ) {
    let old_length = self.machines.len();
    self.machines.extend(machines);
    let mut disturbed = Vec::with_capacity(self.machines.len());
    disturbed.extend(old_length..self.machines.len());
    self.disturb_downstream(
      machine_types,
      &self.output_edges(machine_types),
      disturbed,
      now,
    );
  }

  pub fn remove_machines(
    &mut self,
    machine_types: &mut MachineTypes,
    machines: Vec<usize>,
    now: Number,
  ) {
    let mut disturbed = Vec::with_capacity(self.machines.len());
    disturbed.extend_from_slice(&machines);
    self.disturb_downstream(
      machine_types,
      &self.output_edges(machine_types),
      disturbed,
      now,
    );
    let mut index = 0;
    self.machines.retain(|_machine| {
      let result = !machines.contains(&index);
      index += 1;
      result
    });
  }

  pub fn modify_machines(
    &mut self,
    machine_types: &mut MachineTypes,
    machines: Vec<usize>,
    now: Number,
    mut modify: impl FnMut(&mut StatefulMachine),
  ) {
    let mut disturbed = Vec::with_capacity(self.machines.len());
    disturbed.extend_from_slice(&machines);
    self.disturb_downstream(
      machine_types,
      &self.output_edges(machine_types),
      disturbed,
      now,
    );
    for (index, machine) in self.machines.iter_mut().enumerate() {
      if machines.contains(&index) {
        (modify)(machine);
      }
    }
  }

  pub fn disturb_downstream(
    &mut self,
    machine_types: &mut MachineTypes,
    output_edges: &OutputEdges,
    starting_points: Vec<usize>,
    now: Number,
  ) {
    let mut stack = starting_points;
    let mut visited: Vec<bool> = vec![false; self.machines.len()];
    while let Some(index) = stack.pop() {
      let machine = &mut self.machines[index];
      machine.state.last_disturbed_time = now;
      if let MachineTypeId::Module(module_index) = machine.type_id {
        self.canonicalize_module(machine_types, index, module_index);
      }
      for &(destination_machine_index, _) in output_edges[index].iter().flatten() {
        if !visited[destination_machine_index] {
          visited[destination_machine_index] = true;
          stack.push(destination_machine_index);
        }
      }
    }
  }

  pub fn canonicalize_module(
    &mut self,
    machine_types: &mut MachineTypes,
    machine_index: usize,
    module_index: usize,
  ) {
    if machine_types.modules[module_index]
      .map
      .machines
      .iter()
      .any(|machine| machine.state.last_disturbed_time != 0)
    {
      let mut new_module = machine_types.modules[module_index].clone();
      let new_module_index = machine_types.modules.len();
      for machine in &mut new_module.map.machines {
        machine.state.last_disturbed_time = 0;
      }
      machine_types.modules.push(new_module);

      self.machines[machine_index].type_id = MachineTypeId::Module(new_module_index);
    }
  }

  pub fn topological_ordering_of_noncyclic_machines(
    &self,
    output_edges: &OutputEdges,
  ) -> Vec<usize> {
    let mut num_inputs: ArrayVec<[usize; MAX_COMPONENTS]> =
      self.machines.iter().map(|_| 0).collect();
    let mut result = Vec::with_capacity(MAX_COMPONENTS);
    let mut starting_points = Vec::with_capacity(MAX_COMPONENTS);
    for machine in output_edges {
      for output in machine {
        if let Some(output) = output {
          num_inputs[output.0] += 1
        }
      }
    }

    for (index, inputs) in num_inputs.iter().enumerate() {
      if *inputs == 0 {
        starting_points.push(index);
      }
    }

    while let Some(starting_point) = starting_points.pop() {
      result.push(starting_point);
      for destination in &output_edges[starting_point] {
        if let Some((machine, _input)) = *destination {
          num_inputs[machine] -= 1;
          if num_inputs[machine] == 0 {
            starting_points.push(machine);
          }
        }
      }
    }
    result
  }

  pub fn future(
    &self,
    machine_types: &MachineTypes,
    output_edges: &OutputEdges,
    topological_ordering: &[usize],
    module_futures: &mut ModuleFutures,
    fiat_inputs: &[(InputLocation, MaterialFlow)],
  ) -> MapFuture {
    //debug!("{:?}", fiat_inputs);
    let mut result = MapFuture {
      machines: self
        .machines
        .iter()
        .map(|machine| MachineAndInputsFuture {
          inputs: machine_types
            .input_locations(machine)
            .map(|input_location| {
              //debug!("{:?}", (input_location, fiat_inputs.iter().find (| (location,_) | *location == input_location)));
              fiat_inputs
                .iter()
                .find(|(location, _)| *location == input_location)
                .map(|(_, flow)| *flow)
            })
            .collect(),
          future: Err(MachineOperatingState::InCycle),
        })
        .collect(),
      dumped: Default::default(),
    };

    for &machine_index in topological_ordering {
      let machine = &self.machines[machine_index];
      let inputs = MachineObservedInputs {
        input_flows: &result.machines[machine_index].inputs,
        start_time: machine.state.last_disturbed_time,
      };
      let machine_type = machine_types.get(machine.type_id);
      let future = machine_type.future(inputs);

      let outputs = match (&machine_type, &future) {
        (MachineTypeRef::Module(module), Ok(MachineFuture::Module(module_machine_future))) => {
          let module_future = module_futures.entry(machine.type_id).or_insert_with(|| {
            let output_edges = module.map.output_edges(machine_types);
            let topological_ordering = module
              .map
              .topological_ordering_of_noncyclic_machines(&output_edges);
            ModuleFuture {
              output_edges,
              topological_ordering,
              future_variations: HashMap::new(),
            }
          });
          let variation = match module_future
            .future_variations
            .get(&module_machine_future.canonical_inputs)
          {
            Some(existing_future) => existing_future,
            None => {
              // note: we have to drop module_future here so we can use module_futures mutably in the recursive call
              // cloning is unfortunate; I could consider refactoring the data structure again to avoid this, but it's nowhere near a performance bottleneck
              let output_edges = module_future.output_edges.clone();
              let ordering = module_future.topological_ordering.clone();

              let fiat_inputs: Vec<_> = module
                .module_type
                .inputs
                .iter()
                .map(|input| input.inner_location)
                .zip(module_machine_future.canonical_inputs.iter())
                .filter_map(|(loc, flow)| {
                  flow.map(|f| {
                    (
                      loc,
                      MaterialFlow {
                        material: f.material,
                        flow: FlowPattern::new(0, f.rate()),
                      },
                    )
                  })
                })
                .collect();
              let future = module.map.future(
                machine_types,
                &output_edges,
                &ordering,
                module_futures,
                &fiat_inputs,
              );

              match module_futures.get_mut (&machine.type_id).unwrap().future_variations.entry (module_machine_future.canonical_inputs.clone()) {
                hash_map::Entry::Occupied (_) => unreachable!("A module's future was modified during calculation of its submodules' futures. Did a module get put inside itself somehow?"),
                hash_map::Entry::Vacant (entry) => entry.insert (future)
              }
            }
          };
          module.module_output_flows(inputs, module_machine_future, variation)
        }
        (_, Ok(future)) => machine_type.output_flows(inputs, future),
        (_, Err(_)) => inputs![],
      };

      result.machines[machine_index].future = future;

      //println!("{:?}\n{:?}\n{:?}\n\n", machine, inputs , outputs);
      for ((flow, destination), location) in outputs
        .into_iter()
        .zip(&output_edges[machine_index])
        .zip(machine_type.output_locations(machine.state.position))
      {
        match destination {
          None => {
            if let Some(flow) = flow {
              result.dumped.push((location, flow))
            }
          }
          Some((destination_machine, destination_input)) => {
            result.machines[*destination_machine].inputs[*destination_input] = flow
          }
        }
      }
    }

    result
  }
}

impl Game {
  pub fn inventory_at(&self, future: &MapFuture, time: Number) -> HashMap<Material, Number> {
    let mut inventory = self.inventory_before_last_change.clone();
    let interval = [self.last_change_time, time];
    for (_location, material_flow) in &future.dumped {
      *inventory.entry(material_flow.material).or_default() +=
        material_flow.flow.num_disbursed_between(interval);
    }
    inventory
  }
}
