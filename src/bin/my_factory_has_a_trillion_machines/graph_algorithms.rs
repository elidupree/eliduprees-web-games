//use std::cmp::{min, max};
use std::collections::{hash_map, HashMap};

use arrayvec::ArrayVec;

use flow_pattern::{FlowCollection, FlowPattern, MaterialFlow};
use geometry::{GridIsomorphism, Number};
use machine_data::{
  Game, InputLocation, Inputs, MachineFuture, MachineIdWithinPlatonicRegion,
  MachineMomentaryVisuals, MachineObservedInputs, MachineOperatingState, MachineTypeId,
  MachineTypeRef, MachineTypeTrait, MachineTypes, Material, PlatonicMachine,
  PlatonicRegionContents, WorldMachinesMap, MAX_COMPONENTS,
};
use modules::{CanonicalModuleInputs, ModuleMachineFuture, PlatonicModule};

pub type OutputEdges = ArrayVec<[Inputs<Option<(usize, usize)>>; MAX_COMPONENTS]>;

/// Not 100% sure whether this should be called PlatonicRegionFuture when
/// it will be determined by more than just the PlatonicRegionContents
/// (it'll also consider last_disturbed_times and module inputs)
/// So like, the ideal name would express PlatonicFutureOfPlatonicRegionContentsPlusDisturbedTimesAndFiatInputs
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RegionFuture {
  pub machines: Vec<MachineAndInputsFuture>,
  pub dumped: Vec<(InputLocation, MaterialFlow)>,
  pub disturbed_children: HashMap<MachineIdWithinPlatonicRegion, RegionFuture>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MachineAndInputsFuture {
  pub inputs: Inputs<Option<MaterialFlow>>,
  pub future: Result<MachineFuture, MachineOperatingState>,
}

pub type UndisturbedModuleFutures = HashMap<CanonicalModuleInputs, RegionFuture>;
pub type UndisturbedModulesFutures = HashMap<MachineTypeId, UndisturbedModuleFutures>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GameFuture {
  pub global_region: RegionFuture,
  pub undisturbed_modules: UndisturbedModulesFutures,
}

impl PlatonicRegionContents {
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
    machines: impl IntoIterator<Item = PlatonicMachine>,
    now: Number,
  ) {
    unimplemented!()
    /*let old_length = self.machines.len();
    self.machines.extend(machines);
    let mut disturbed = Vec::with_capacity(self.machines.len());
    disturbed.extend(old_length..self.machines.len());
    self.disturb_downstream(
      machine_types,
      &self.output_edges(machine_types),
      disturbed,
      now,
    );*/
  }

  pub fn remove_machines(
    &mut self,
    machine_types: &mut MachineTypes,
    machines: Vec<usize>,
    now: Number,
  ) {
    unimplemented!()
    /*let mut disturbed = Vec::with_capacity(self.machines.len());
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
    });*/
  }

  pub fn modify_machines(
    &mut self,
    machine_types: &mut MachineTypes,
    machines: Vec<usize>,
    now: Number,
    mut modify: impl FnMut(&mut PlatonicMachine),
  ) {
    unimplemented!()
    /*let mut disturbed = Vec::with_capacity(self.machines.len());
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
    }*/
  }

  /*pub fn disturb_downstream(
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
    if machine_types.custom_modules[module_index]
      .region
      .machines
      .iter()
      .any(|machine| machine.state.last_disturbed_time != 0)
    {
      let mut new_module = machine_types.custom_modules[module_index].clone();
      let new_module_index = machine_types.custom_modules.len();
      for machine in &mut new_module.region.machines {
        machine.state.last_disturbed_time = 0;
      }
      machine_types.custom_modules.push(new_module);

      self.machines[machine_index].type_id = MachineTypeId::Module(new_module_index);
    }
  }*/

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
}

struct GameFutureBuilder<'a> {
  machine_types: &'a MachineTypes,
  global_region_geometry: (OutputEdges, Vec<usize>),
  module_geometries: HashMap<MachineTypeId, (OutputEdges, Vec<usize>)>,
}

impl<'a> GameFutureBuilder<'a> {
  pub fn new(game: &'a Game) -> GameFutureBuilder<'a> {
    let module_geometries = game
      .machine_types
      .modules()
      .map(|(id, module)| {
        let output_edges = module.region.output_edges(&game.machine_types);
        let topological_ordering = module
          .region
          .topological_ordering_of_noncyclic_machines(&output_edges);
        (id, (output_edges, topological_ordering))
      })
      .collect();

    let output_edges = game.global_region.output_edges(&game.machine_types);
    let ordering = game
      .global_region
      .topological_ordering_of_noncyclic_machines(&output_edges);
    GameFutureBuilder {
      machine_types: &game.machine_types,
      global_region_geometry: (output_edges, ordering),
      module_geometries,
    }
  }
  pub fn region_future(
    &self,
    undisturbed_modules_futures: &'a mut UndisturbedModulesFutures,
    region: &WorldRegionView,
    fiat_inputs: &[(InputLocation, MaterialFlow)],
  ) -> RegionFuture {
    //debug!("{:?}", fiat_inputs);
    let mut result = RegionFuture {
      machines: region
        .machines()
        .map(|machine| MachineAndInputsFuture {
          inputs: self
            .machine_types
            .input_locations(machine.platonic)
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
      disturbed_children: Default::default(),
    };

    let (output_edges, topological_ordering) = match region.as_module {
      Some(module) => self
        .module_geometries
        .get(&module.as_machine.platonic.type_id)
        .unwrap(),
      None => &self.global_region_geometry,
    };

    let machines: Vec<_> = region.machines().collect();
    for &machine_index in topological_ordering {
      let machine: &WorldMachineView = &machines[machine_index];
      let inputs = MachineObservedInputs {
        input_flows: &result.machines[machine.index_within_region].inputs,
        start_time: machine.last_disturbed_time.unwrap_or(0),
      };
      let future = machine.machine_type.future(inputs);

      let outputs = match (machine.as_module(), &future) {
        (Some(module), Ok(MachineFuture::Module(module_machine_future))) => {
          let inner_region = module.region();

          let fiat_inputs: Vec<_> = module
            .platonic
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

          let variation = if inner_region.last_disturbed_times.is_some() {
            // Disturbed, and therefore unique enough that we don't need to deduplicate the future
            let inner_future =
              self.region_future(undisturbed_modules_futures, &inner_region, &fiat_inputs);
            result
              .disturbed_children
              .entry(machine.id_within_region)
              .or_insert(inner_future) // should always insert, but doing it this way to get a reference back
          } else {
            let platonic_module_futures = undisturbed_modules_futures
              .entry(machine.platonic.type_id)
              .or_default();

            match platonic_module_futures.get(&module_machine_future.canonical_inputs) {
              Some(e) => e,
              None => {
                let inner_future =
                  self.region_future(undisturbed_modules_futures, &inner_region, &fiat_inputs);

                match undisturbed_modules_futures.get_mut(&machine.platonic.type_id).unwrap().entry(module_machine_future.canonical_inputs.clone()) {
                  hash_map::Entry::Occupied(_) => unreachable!("A module's future was modified during calculation of its submodules' futures. Did a module get put inside itself somehow?"),
                  hash_map::Entry::Vacant(entry) => entry.insert(inner_future)
                }
              }
            }
          };
          module
            .platonic
            .module_output_flows(inputs, module_machine_future, variation)
        }
        (_, Ok(future)) => machine.machine_type.output_flows(inputs, future),
        (_, Err(_)) => inputs![],
      };

      result.machines[machine_index].future = future;

      //println!("{:?}\n{:?}\n{:?}\n\n", machine, inputs , outputs);
      for ((flow, destination), location) in
        outputs.into_iter().zip(&output_edges[machine_index]).zip(
          machine
            .machine_type
            .output_locations(machine.platonic.state.position),
        )
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
  pub fn future(&self) -> GameFuture {
    let builder = GameFutureBuilder::new(self);
    let mut undisturbed_modules = UndisturbedModulesFutures::default();
    let global_region = builder.region_future(
      &mut undisturbed_modules,
      &GameView {
        game: self,
        future: None,
        selected: None,
      }
      .global_region(),
      &[],
    );
    GameFuture {
      global_region,
      undisturbed_modules,
    }
  }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct GameView<'a> {
  pub game: &'a Game,
  pub future: Option<&'a GameFuture>,
  pub selected: Option<&'a WorldMachinesMap<()>>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WorldRegionView<'a> {
  pub game: GameView<'a>,
  pub as_module: Option<&'a WorldModuleView<'a>>,
  pub platonic: &'a PlatonicRegionContents,
  pub isomorphism: GridIsomorphism,
  pub start_time_and_future: Option<(Number, &'a RegionFuture)>,
  pub last_disturbed_times: Option<&'a WorldMachinesMap<Number>>,
  pub selected: Option<&'a WorldMachinesMap<()>>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WorldMachineView<'a> {
  pub game: GameView<'a>,
  pub platonic: &'a PlatonicMachine,
  pub machine_type: MachineTypeRef<'a>,
  pub isomorphism: GridIsomorphism,
  pub containing_region: &'a WorldRegionView<'a>,
  pub index_within_region: usize,
  pub id_within_region: MachineIdWithinPlatonicRegion,
  pub region_start_time_and_machine_future: Option<(Number, &'a MachineAndInputsFuture)>,
  pub last_disturbed_time: Option<Number>,
  pub selected: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WorldModuleView<'a> {
  pub game: GameView<'a>,
  pub as_machine: &'a WorldMachineView<'a>,
  pub platonic: &'a PlatonicModule,
  pub inner_start_time_and_module_future:
    Option<(Number, &'a ModuleMachineFuture, &'a RegionFuture)>,
}

impl<'a> GameView<'a> {
  pub fn global_region(&self) -> WorldRegionView {
    WorldRegionView {
      game: *self,
      as_module: None,
      platonic: &self.game.global_region,
      isomorphism: GridIsomorphism::default(),
      start_time_and_future: self.future.map(|future| (0, &future.global_region)),
      last_disturbed_times: Some(&self.game.last_disturbed_times),
      selected: self.selected,
    }
  }
  pub fn inventory_at(&self, time: Number) -> HashMap<Material, Number> {
    let future = self
      .future
      .expect("called inventory_at on a view with no future");
    let mut inventory = self.game.inventory_before_last_change.clone();
    let interval = [self.game.last_change_time, time];
    for (_location, material_flow) in &future.global_region.dumped {
      *inventory.entry(material_flow.material).or_default() +=
        material_flow.flow.num_disbursed_between(interval);
    }
    inventory
  }
}

impl<'a> WorldRegionView<'a> {
  pub fn machines<'b>(&'b self) -> impl Iterator<Item = WorldMachineView<'b>> + 'b {
    self
      .platonic
      .machines
      .iter()
      .enumerate()
      .map(move |(index, machine)| {
        let id_within_region = machine.id_within_region();
        WorldMachineView {
          game: self.game,
          platonic: machine,
          machine_type: self.game.game.machine_types.get(machine.type_id),
          isomorphism: machine.state.position * self.isomorphism,
          containing_region: self,
          index_within_region: index,
          id_within_region,
          region_start_time_and_machine_future: self
            .start_time_and_future
            .map(|(start_time, future)| (start_time, &future.machines[index])),
          last_disturbed_time: self
            .last_disturbed_times
            .and_then(|a| a.here.get(&id_within_region))
            .copied(),
          selected: self
            .selected
            .and_then(|a| a.here.get(&id_within_region))
            .is_some(),
        }
      })
  }
}

impl<'a> WorldMachineView<'a> {
  pub fn as_module(&self) -> Option<WorldModuleView> {
    match self.game.game.machine_types.get(self.platonic.type_id) {
      MachineTypeRef::Module(module) => Some(WorldModuleView {
        game: self.game,
        as_machine: self,
        platonic: module,
        inner_start_time_and_module_future: self.region_start_time_and_machine_future.and_then(
          |(start_time, machine_future)| match &machine_future.future {
            Ok(MachineFuture::Module(module_machine_future)) => Some((
              start_time + module_machine_future.start_time,
              module_machine_future,
              self.containing_region.start_time_and_future.unwrap().1.disturbed_children.get(&self.id_within_region).unwrap_or_else(||
              self
                .game
                .future
                .unwrap()
                .undisturbed_modules
                .get(& self.platonic.type_id)
                .expect("there shouldn't be a ModuleMachineFuture if there isn't a corresponding ModuleFuture")
                .get(&module_machine_future.canonical_inputs)
                .expect("there shouldn't be a ModuleMachineFuture if there isn't a corresponding future-variation")),
            )),
            _ => None,
          },
        ),
      }),
      _ => None,
    }
  }

  pub fn input_locations(&self) -> impl Iterator<Item = InputLocation> {
    self.machine_type.input_locations(self.isomorphism)
  }
  pub fn output_locations(&self) -> impl Iterator<Item = InputLocation> {
    self.machine_type.output_locations(self.isomorphism)
  }

  /// returns None if this machine doesn't have a future at all (i.e. is inside a non-operating module)
  pub fn momentary_visuals(&self, absolute_time: Number) -> Option<MachineMomentaryVisuals> {
    let (region_start_time, machine_future) = self.region_start_time_and_machine_future?;
    let local_time = absolute_time - region_start_time;
    let inputs = MachineObservedInputs {
      input_flows: &machine_future.inputs,
      start_time: self.last_disturbed_time.unwrap_or(0),
    };
    let visuals = match &machine_future.future {
      Err(operating_state) => MachineMomentaryVisuals {
        operating_state: operating_state.clone(),
        materials: Vec::new(),
      },
      Ok(future) => match self.as_module().and_then(|module| {
        module
          .inner_start_time_and_module_future
          .map(|stuff| (module, stuff))
      }) {
        Some((module, (_inner_start_time, module_machine_future, module_region_future))) => {
          module.platonic.module_momentary_visuals(
            inputs,
            module_machine_future,
            local_time,
            module_region_future,
          )
        }
        None => self
          .machine_type
          .momentary_visuals(inputs, future, local_time),
      },
    };
    Some(visuals)
  }
}

impl<'a> WorldModuleView<'a> {
  pub fn region(&self) -> WorldRegionView {
    WorldRegionView {
      game: self.game,
      as_module: Some(self),
      platonic: &self.platonic.region,
      isomorphism: self.as_machine.isomorphism,
      start_time_and_future: self
        .inner_start_time_and_module_future
        .map(|(a, _b, c)| (a, c)),
      last_disturbed_times: self
        .as_machine
        .containing_region
        .last_disturbed_times
        .and_then(|a| a.children.get(&self.as_machine.id_within_region)),
      selected: self
        .as_machine
        .containing_region
        .selected
        .and_then(|a| a.children.get(&self.as_machine.id_within_region)),
    }
  }
}
