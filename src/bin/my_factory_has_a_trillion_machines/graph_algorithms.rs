//use std::cmp::{min, max};
use std::collections::{hash_map, HashMap};

use arrayvec::ArrayVec;

use flow_pattern::{FlowCollection, FlowPattern, MaterialFlow};
use geometry::{GridIsomorphism, Number};
use machine_data::{
  Game, InputLocation, Inputs, MachineFuture, MachineIdWithinPlatonicRegion, MachineObservedInputs,
  MachineOperatingState, MachineTypeId, MachineTypeRef, MachineTypeTrait, MachineTypes, Material,
  PlatonicMachine, PlatonicRegionContents, MAX_COMPONENTS,
};
use modules::{CanonicalModuleInputs, PlatonicModule};

pub type OutputEdges = ArrayVec<[Inputs<Option<(usize, usize)>>; MAX_COMPONENTS]>;

/// Not 100% sure whether this should be called PlatonicRegionFuture when
/// it will be determined by more than just the PlatonicRegionContents
/// (it'll also consider last_disturbed_times and module inputs)
/// So like, the ideal name would express PlatonicFutureOfPlatonicRegionContentsPlusDisturbedTimesAndFiatInputs
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RegionFuture {
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
  pub future_variations: HashMap<CanonicalModuleInputs, RegionFuture>,
}

pub type ModuleFutures = HashMap<MachineTypeId, ModuleFuture>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GameFuture {
  pub global_region: RegionFuture,
  pub modules: ModuleFutures,
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
    mut modify: impl FnMut(&mut PlatonicMachine),
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
      .region
      .machines
      .iter()
      .any(|machine| machine.state.last_disturbed_time != 0)
    {
      let mut new_module = machine_types.modules[module_index].clone();
      let new_module_index = machine_types.modules.len();
      for machine in &mut new_module.region.machines {
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
  ) -> RegionFuture {
    //debug!("{:?}", fiat_inputs);
    let mut result = RegionFuture {
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
            let output_edges = module.region.output_edges(machine_types);
            let topological_ordering = module
              .region
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
              let future = module.region.future(
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
  pub fn future(&self) -> GameFuture {
    let output_edges = self.global_region.output_edges(&self.machine_types);
    let ordering = self
      .global_region
      .topological_ordering_of_noncyclic_machines(&output_edges);
    let mut module_futures = ModuleFutures::default();
    let region_future = self.global_region.future(
      &self.machine_types,
      &output_edges,
      &ordering,
      &mut module_futures,
      &[],
    );
    GameFuture {
      global_region: region_future,
      modules: module_futures,
    }
  }
}

#[derive(Copy, Clone, Debug)]
pub struct ViewMachineIds {
  index: usize,
  id_within_region: MachineIdWithinPlatonicRegion,
  type_id: MachineTypeId,
}

pub trait WorldViewAspect<'a> {
  type Game: 'a;
  type Region: 'a;
  type Machine: 'a;
  type Module: 'a;
}
pub trait WorldViewAspectGetMut<'a>: WorldViewAspect<'a> {
  fn global_region_mut(game: &'a mut Self::Game) -> Self::Region;
  fn get_machine_mut(region: &'a mut Self::Region, ids: ViewMachineIds) -> Self::Machine;
  // One might think this should be Option<Self::Module>,
  // but some aspect types can't tell whether the module exists or not
  fn as_module_mut(machine: &'a mut Self::Machine) -> Self::Module;
  fn inner_region_mut(module: &'a mut Self::Module) -> Self::Region;
}
pub trait WorldViewAspectGet<'a>: WorldViewAspectGetMut<'a> {
  fn global_region(game: &'a Self::Game) -> Self::Region;
  fn get_machine(region: &'a Self::Region, ids: ViewMachineIds) -> Self::Machine;
  fn as_module(machine: &'a Self::Machine) -> Self::Module;
  fn inner_region(module: &'a Self::Module) -> Self::Region;
}

impl<'a, T: WorldViewAspectGet<'a>> WorldViewAspectGetMut<'a> for T {
  fn global_region_mut(game: &'a mut Self::Game) -> Self::Region {
    Self::global_region(game)
  }
  fn get_machine_mut(region: &'a mut Self::Region, ids: ViewMachineIds) -> Self::Machine {
    Self::get_machine(region, ids)
  }
  fn as_module_mut(machine: &'a mut Self::Machine) -> Self::Module {
    Self::as_module(machine)
  }
  fn inner_region_mut(module: &'a mut Self::Module) -> Self::Region {
    Self::inner_region(module)
  }
}

pub trait GetSubaspect<'a, T: WorldViewAspect<'a>>: WorldViewAspect<'a> {
  fn get_game_aspect(game: &'a Self::Game) -> &'a T::Game;
  fn get_region_aspect(region: &'a Self::Region) -> &'a T::Region;
  fn get_machine_aspect(machine: &'a Self::Machine) -> &'a T::Machine;
  fn get_module_aspect(module: &'a Self::Module) -> &'a T::Module;
}
pub trait GetSubaspectMut<'a, T: WorldViewAspect<'a>>: WorldViewAspect<'a> {
  fn get_game_aspect_mut(game: &'a mut Self::Game) -> &'a mut T::Game;
  fn get_region_aspect_mut(region: &'a mut Self::Region) -> &'a mut T::Region;
  fn get_machine_aspect_mut(machine: &'a mut Self::Machine) -> &'a mut T::Machine;
  fn get_module_aspect_mut(module: &'a mut Self::Module) -> &'a mut T::Module;
}

pub trait RegionViewListMachines {
  fn view_machine_ids(&self) -> Vec<ViewMachineIds>;
}

#[derive(Debug)]
pub struct GameView<'a, T: WorldViewAspect<'a>> {
  pub aspects: T::Game,
}

#[derive(Debug)]
pub struct WorldRegionView<'a, T: WorldViewAspect<'a>> {
  pub aspects: T::Region,
}

#[derive(Debug)]
pub struct WorldMachineView<'a, T: WorldViewAspect<'a>> {
  pub aspects: T::Machine,
}

#[derive(Debug)]
pub struct WorldModuleView<'a, T: WorldViewAspect<'a>> {
  pub aspects: T::Module,
}

impl<'a, T: WorldViewAspectGetMut<'a>> GameView<'a, T> {
  pub fn global_region_mut(&'a mut self) -> WorldRegionView<'a, T> {
    WorldRegionView {
      aspects: T::global_region_mut(&mut self.aspects),
    }
  }
}

impl<'a, T: WorldViewAspectGet<'a>> GameView<'a, T> {
  pub fn global_region(&'a self) -> WorldRegionView<'a, T> {
    WorldRegionView {
      aspects: T::global_region(&self.aspects),
    }
  }
}

impl<'a, T: WorldViewAspectGetMut<'a>> WorldRegionView<'a, T> {
  pub fn get_machine_mut(&'a mut self, ids: ViewMachineIds) -> WorldMachineView<'a, T> {
    WorldMachineView {
      aspects: T::get_machine_mut(&mut self.aspects, ids),
    }
  }
}

impl<'a, T: WorldViewAspectGet<'a>> WorldRegionView<'a, T> {
  pub fn get_machine(&'a self, ids: ViewMachineIds) -> WorldMachineView<'a, T> {
    WorldMachineView {
      aspects: T::get_machine(&self.aspects, ids),
    }
  }
}

impl<'a, T: WorldViewAspectGetMut<'a>> WorldMachineView<'a, T> {
  pub fn as_module_mut(&'a mut self) -> WorldModuleView<'a, T> {
    WorldModuleView {
      aspects: T::as_module_mut(&mut self.aspects),
    }
  }
}

impl<'a, T: WorldViewAspectGet<'a>> WorldMachineView<'a, T> {
  pub fn as_module(&'a self) -> WorldModuleView<'a, T> {
    WorldModuleView {
      aspects: T::as_module(&self.aspects),
    }
  }
}

impl<'a, T: WorldViewAspectGetMut<'a>> WorldModuleView<'a, T> {
  pub fn inner_region_mut(&'a mut self) -> WorldRegionView<'a, T> {
    WorldRegionView {
      aspects: T::inner_region_mut(&mut self.aspects),
    }
  }
}

impl<'a, T: WorldViewAspectGet<'a>> WorldModuleView<'a, T> {
  pub fn inner_region(&'a self) -> WorldRegionView<'a, T> {
    WorldRegionView {
      aspects: T::inner_region(&self.aspects),
    }
  }
}

macro_rules! impl_world_views_for_aspect_tuple {
   (($($Aspect: ident,)*), $Tuple: tt) => {

impl<'a> WorldViewAspect<'a> for $Tuple {
  type Game = ($(<$Aspect as WorldViewAspect<'a>>::Game,)*);
  type Region = ($(<$Aspect as WorldViewAspect<'a>>::Region,)*);
  type Machine = ($(<$Aspect as WorldViewAspect<'a>>::Machine,)*);
  type Module = ($(<$Aspect as WorldViewAspect<'a>>::Module,)*);
}

$(
#[allow(non_snake_case)]
impl<'a> GetSubaspect<'a, $Aspect> for $Tuple {
  fn get_game_aspect( game:&'a Self::Game) -> &'a <$Aspect as WorldViewAspect<'a>>::Game {
    let $Tuple = game;
    $Aspect
  }
  fn get_region_aspect( region:&'a Self::Region) -> &'a <$Aspect as WorldViewAspect<'a>>::Region{
    let $Tuple =  region;
    $Aspect
  }
  fn get_machine_aspect( machine:&'a Self::Machine) -> &'a <$Aspect as WorldViewAspect<'a>>::Machine{
    let $Tuple =  machine;
    $Aspect
  }
  fn get_module_aspect( module:&'a Self::Module) -> &'a <$Aspect as WorldViewAspect<'a>>::Module{
    let $Tuple =  module;
    $Aspect
  }
}
#[allow(non_snake_case)]
impl<'a> GetSubaspectMut<'a, $Aspect> for $Tuple {
  fn get_game_aspect_mut( game:&'a mut Self::Game) -> &'a mut <$Aspect as WorldViewAspect<'a>>::Game{
    let $Tuple = game;
    $Aspect
  }
  fn get_region_aspect_mut( region:&'a mut Self::Region) -> &'a mut <$Aspect as WorldViewAspect<'a>>::Region{
    let $Tuple =  region;
    $Aspect
  }
  fn get_machine_aspect_mut( machine:&'a mut Self::Machine) -> &'a mut <$Aspect as WorldViewAspect<'a>>::Machine{
    let $Tuple =  machine;
    $Aspect
  }
  fn get_module_aspect_mut( module:&'a mut Self::Module) -> &'a mut <$Aspect as WorldViewAspect<'a>>::Module{
    let $Tuple =  module;
    $Aspect
  }
}
)*

  };
  (&mut ($($Aspect: ident,)*)) => {
impl_world_views_for_aspect_tuple!(($($Aspect,)*), ($($Aspect,)*));
#[allow(non_snake_case)]
impl<'a> WorldViewAspectGetMut<'a> for ($($Aspect,)*) {
  fn global_region_mut(game: &'a mut Self::Game) -> Self::Region {
    let ($($Aspect,)*) = game;
    ($(
      $Aspect::global_region_mut($Aspect),
    )*)
  }
  fn get_machine_mut(region: &'a mut Self::Region, ids: ViewMachineIds) -> Self::Machine {
    let ($($Aspect,)*) = region;
    ($(
      $Aspect::get_machine_mut($Aspect, ids),
    )*)
  }
  fn as_module_mut(machine: &'a mut Self::Machine) -> Self::Module {
    let ($($Aspect,)*) = machine;
    ($(
      $Aspect::as_module_mut($Aspect),
    )*)
  }
  fn inner_region_mut(module: &'a mut Self::Module) -> Self::Region {
    let ($($Aspect,)*) = module;
    ($(
      $Aspect::inner_region_mut($Aspect),
    )*)
  }
}
  };

  (& ($($Aspect: ident,)*)) => {
impl_world_views_for_aspect_tuple!(($($Aspect,)*), ($($Aspect,)*));
#[allow(non_snake_case)]
impl<'a> WorldViewAspectGet<'a> for ($($Aspect,)*) {
  fn global_region(game: &'a Self::Game) -> Self::Region {
    let ($($Aspect,)*) = game;
    ($(
      $Aspect::global_region($Aspect),
    )*)
  }
  fn get_machine(region: &'a Self::Region, ids: ViewMachineIds) -> Self::Machine {
    let ($($Aspect,)*) = region;
    ($(
      $Aspect::get_machine($Aspect, ids),
    )*)
  }
  fn as_module(machine: &'a  Self::Machine) -> Self::Module {
    let ($($Aspect,)*) = machine;
    ($(
      $Aspect::as_module($Aspect),
    )*)
  }
  fn inner_region(module: &'a Self::Module) -> Self::Region {
    let ($($Aspect,)*) = module;
    ($(
      $Aspect::inner_region($Aspect),
    )*)
  }
}
  };
}

pub use self::base_view_aspect::BaseAspect;
pub mod base_view_aspect {
  use super::*;
  use machine_data::WorldMachinesMap;

  pub enum BaseAspect {}

  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub struct GameView<'a> {
    game: &'a Game,
  }

  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub struct WorldRegionView<'a> {
    game: GameView<'a>,
    platonic: &'a PlatonicRegionContents,
    isomorphism: GridIsomorphism,
    last_disturbed_times: Option<&'a WorldMachinesMap<Number>>,
  }

  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub struct WorldMachineView<'a> {
    game: GameView<'a>,
    platonic: &'a PlatonicMachine,
    machine_type: MachineTypeRef<'a>,
    isomorphism: GridIsomorphism,
    parent: &'a WorldRegionView<'a>,
    index_within_parent: usize,
  }

  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub struct WorldModuleView<'a> {
    game: GameView<'a>,
    as_machine: &'a WorldMachineView<'a>,
    platonic: &'a PlatonicModule,
  }

  impl<'a> WorldViewAspect<'a> for BaseAspect {
    type Game = GameView<'a>;
    type Region = WorldRegionView<'a>;
    type Machine = WorldMachineView<'a>;
    type Module = WorldModuleView<'a>;
  }
  impl<'a> WorldViewAspectGet<'a> for BaseAspect {
    fn global_region(game: &'a Self::Game) -> Self::Region {
      WorldRegionView {
        game: *game,
        platonic: &game.game.global_region,
        isomorphism: GridIsomorphism::default(),
        last_disturbed_times: Some(&game.game.last_disturbed_times),
      }
    }
    fn get_machine(region: &'a Self::Region, ids: ViewMachineIds) -> Self::Machine {
      let machine = &region.platonic.machines[ids.index];
      WorldMachineView {
        game: region.game,
        platonic: machine,
        machine_type: region.game.game.machine_types.get(machine.type_id),
        isomorphism: machine.state.position * region.isomorphism,
        parent: region,
        index_within_parent: ids.index,
      }
    }
    fn as_module(machine: &'a Self::Machine) -> Self::Module {
      match machine
        .game
        .game
        .machine_types
        .get(machine.platonic.type_id)
      {
        MachineTypeRef::Module(module) => WorldModuleView {
          game: machine.game,
          as_machine: machine,
          platonic: module,
        },
        _ => panic!("can't call as_module unless the machine is actually a module"),
      }
    }
    fn inner_region(module: &'a Self::Module) -> Self::Region {
      WorldRegionView {
        game: module.game,
        platonic: &module.platonic.region,
        isomorphism: module.as_machine.isomorphism,
        last_disturbed_times: module
          .as_machine
          .parent
          .last_disturbed_times
          .and_then(|times| {
            times
              .children
              .get(&module.as_machine.platonic.id_within_region())
          }),
      }
    }
  }

  impl<'a, T: GetSubaspect<'a, BaseAspect>> super::GameView<'a, T> {
    pub fn game(&'a self) -> &'a Game {
      T::get_game_aspect(&self.aspects).game
    }
  }
}

pub use self::future_view_aspect::FutureAspect;
pub mod future_view_aspect {
  use super::*;
  use modules::ModuleMachineFuture;

  pub enum FutureAspect {}

  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub struct GameView<'a> {
    future: &'a GameFuture,
  }

  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub struct WorldRegionView<'a> {
    game: GameView<'a>,
    start_time_and_future: Option<(Number, &'a RegionFuture)>,
  }

  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub struct WorldMachineView<'a> {
    game: GameView<'a>,
    parent: &'a WorldRegionView<'a>,
    type_id: MachineTypeId,
    future: Option<&'a MachineAndInputsFuture>,
  }

  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub struct WorldModuleView<'a> {
    game: GameView<'a>,
    as_machine: &'a WorldMachineView<'a>,
    inner_start_time_and_module_future: Option<(Number, &'a ModuleMachineFuture, &'a RegionFuture)>,
  }

  impl<'a> WorldViewAspect<'a> for FutureAspect {
    type Game = GameView<'a>;
    type Region = WorldRegionView<'a>;
    type Machine = WorldMachineView<'a>;
    type Module = WorldModuleView<'a>;
  }
  impl<'a> WorldViewAspectGet<'a> for FutureAspect {
    fn global_region(game: &'a Self::Game) -> Self::Region {
      WorldRegionView {
        game: *game,
        start_time_and_future: Some((0, &game.future.global_region)),
      }
    }
    fn get_machine(region: &'a Self::Region, ids: ViewMachineIds) -> Self::Machine {
      WorldMachineView {
        game: region.game,
        parent: region,
        type_id: ids.type_id,
        future: region
          .start_time_and_future
          .map(|(_start_time, future)| &future.machines[ids.index]),
      }
    }
    fn as_module(machine: &'a Self::Machine) -> Self::Module {
      WorldModuleView {
          game: machine.game,
          as_machine: machine,
          inner_start_time_and_module_future: machine.future.and_then(
            |machine_future| match &machine_future.future {
              Ok(MachineFuture::Module(module_machine_future)) => Some((
                machine.parent.start_time_and_future.as_ref().unwrap().0 + module_machine_future.start_time,
                module_machine_future,
                machine
                    .game
                    .future
                    .modules
                    .get(& machine.type_id)
                    .expect("there shouldn't be a ModuleMachineFuture if there isn't a corresponding ModuleFuture")
                    .future_variations
                    .get(&module_machine_future.canonical_inputs)
                    .expect("there shouldn't be a ModuleMachineFuture if there isn't a corresponding future-variation"),
              )),
              _ => None,
            },
          ),
        }
    }
    fn inner_region(module: &'a Self::Module) -> Self::Region {
      WorldRegionView {
        game: module.game,
        start_time_and_future: module
          .inner_start_time_and_module_future
          .map(|(a, _b, c)| (a, c)),
      }
    }
  }

  impl<'a, T: GetSubaspect<'a, FutureAspect>> super::GameView<'a, T> {
    pub fn future(&'a self) -> &'a GameFuture {
      T::get_game_aspect(&self.aspects).future
    }
  }
}

impl_world_views_for_aspect_tuple!(&(BaseAspect,));

impl<'a, T: GetSubaspect<'a, BaseAspect> + GetSubaspect<'a, FutureAspect>> GameView<'a, T> {
  pub fn inventory_at(&'a self, time: Number) -> HashMap<Material, Number> {
    let mut inventory = self.game().inventory_before_last_change.clone();
    let interval = [self.game().last_change_time, time];
    for (_location, material_flow) in &self.future().global_region.dumped {
      *inventory.entry(material_flow.material).or_default() +=
        material_flow.flow.num_disbursed_between(interval);
    }
    inventory
  }
}

/*
impl<$($Field,)*> WorldView<$($Field,)*> where Platonic: RegionViewListMachines {
  fn machines<'a>(&'a self, ids: ViewMachineIds) -> impl Iterator<Item = WorldView<$(<$Field as RegionViewAspect<'a>>::Machine,)*>> + 'a where $($Field: RegionViewAspect<'a>,)* {
    self.platonic.machine_ids().into_iter().map(move |ids| self.get_machine(ids))
  }
}
//unfortunately, the lifetimes don't work for the mutable version

impl<'a,$($Field: MachineViewAspect<'a>,)*> WorldView<$($Field,)*> {
  fn as_module(&'a self) -> WorldView<$($Field::Module,)*> {
    WorldView {
      $($field: self.$field.as_module(),)*
    }
  }
}

impl<'a,$($Field: MachineViewAspectMut<'a>,)*> WorldView<$($Field,)*> {
  fn as_module_mut(&'a mut self) -> WorldView<$($Field::ModuleMut,)*> {
    WorldView {
      $($field: self.$field.as_module_mut(),)*
    }
  }
}

impl<'a,$($Field: ModuleViewAspect<'a>,)*> WorldView<$($Field,)*> {
  fn inner_region(&'a self) -> WorldView<$($Field::Region,)*> {
    WorldView {
      $($field: self.$field.inner_region(),)*
    }
  }
}

impl<'a,$($Field: ModuleViewAspectMut<'a>,)*> WorldView<$($Field,)*> {
  fn inner_region_mut(&'a mut self) -> WorldView<$($Field::RegionMut,)*> {
    WorldView {
      $($field: self.$field.inner_region_mut(),)*
    }
  }
}

  }
}


views! {
  platonic: Platonic,
  last_disturbed: LastDisturbed,
  selected: Selected,
  future: Future,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct GameView<'a> {
  pub game: &'a Game,
  pub future: &'a GameFuture,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WorldRegionView<'a> {
  pub game: GameView<'a>,
  pub region: &'a PlatonicRegionContents,
  pub isomorphism: GridIsomorphism,
  pub start_time_and_future: Option<(Number, &'a RegionFuture)>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WorldMachineView<'a> {
  pub game: GameView<'a>,
  pub machine: &'a PlatonicMachine,
  pub machine_type: MachineTypeRef<'a>,
  pub isomorphism: GridIsomorphism,
  pub parent: &'a WorldRegionView<'a>,
  pub index_within_parent: usize,
  pub region_start_time_and_machine_future: Option<(Number, &'a MachineAndInputsFuture)>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WorldModuleView<'a> {
  pub game: GameView<'a>,
  pub machine: &'a WorldMachineView<'a>,
  pub module: &'a PlatonicModule,
  pub inner_start_time_and_module_future:
    Option<(Number, &'a ModuleMachineFuture, &'a RegionFuture)>,
}

impl<'a> GameView<'a> {
  pub fn global_region(&self) -> WorldRegionView {
    WorldRegionView {
      game: *self,
      region: &self.game.global_region,
      isomorphism: GridIsomorphism::default(),
      start_time_and_future: Some((0, &self.future.global_region)),
    }
  }
  pub fn inventory_at(&self, time: Number) -> HashMap<Material, Number> {
    let mut inventory = self.game.inventory_before_last_change.clone();
    let interval = [self.game.last_change_time, time];
    for (_location, material_flow) in &self.future.global_region.dumped {
      *inventory.entry(material_flow.material).or_default() +=
        material_flow.flow.num_disbursed_between(interval);
    }
    inventory
  }
}

impl<'a> WorldRegionView<'a> {
  pub fn machines<'b>(&'b self) -> impl Iterator<Item = WorldMachineView<'b>> + 'b {
    self
      .region
      .machines
      .iter()
      .enumerate()
      .map(move |(index, machine)| WorldMachineView {
        game: self.game,
        machine,
        machine_type: self.game.game.machine_types.get(machine.type_id),
        isomorphism: machine.state.position * self.isomorphism,
        parent: self,
        index_within_parent: index,
        region_start_time_and_machine_future: self
          .start_time_and_future
          .map(|(start_time, future)| (start_time, &future.machines[index])),
      })
  }
}

impl<'a> WorldMachineView<'a> {
  pub fn module(&self) -> Option<WorldModuleView> {
    match self.game.game.machine_types.get(self.machine.type_id) {
      MachineTypeRef::Module(module) => Some(WorldModuleView {
        game: self.game,
        machine: self,
        module,
        inner_start_time_and_module_future: self.region_start_time_and_machine_future.and_then(
          |(start_time, machine_future)| match &machine_future.future {
            Ok(MachineFuture::Module(module_machine_future)) => Some((
              start_time + module_machine_future.start_time,
              module_machine_future,
              self
                .game
                .future
                .modules
                .get(& self.machine.type_id)
                .expect("there shouldn't be a ModuleMachineFuture if there isn't a corresponding ModuleFuture")
                .future_variations
                .get(&module_machine_future.canonical_inputs)
                .expect("there shouldn't be a ModuleMachineFuture if there isn't a corresponding future-variation"),
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
      start_time: self.machine.state.last_disturbed_time,
    };
    let visuals = match &machine_future.future {
      Err(operating_state) => MachineMomentaryVisuals {
        operating_state: operating_state.clone(),
        materials: Vec::new(),
      },
      Ok(future) => match self.module().and_then(|module| {
        module
          .inner_start_time_and_module_future
          .map(|stuff| (module, stuff))
      }) {
        Some((module, (_inner_start_time, module_machine_future, module_region_future))) => {
          module.module.module_momentary_visuals(
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
      region: &self.module.region,
      isomorphism: self.machine.isomorphism,
      start_time_and_future: self
        .inner_start_time_and_module_future
        .map(|(a, _b, c)| (a, c)),
    }
  }
}*/
