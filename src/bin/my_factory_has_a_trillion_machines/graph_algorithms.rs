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
    region: &WorldRegionView<(BaseAspect,)>,
    fiat_inputs: &[(InputLocation, MaterialFlow)],
  ) -> RegionFuture {
    //debug!("{:?}", fiat_inputs);
    let mut result = RegionFuture {
      machines: region
        .machines()
        .map(|machine| MachineAndInputsFuture {
          inputs: self
            .machine_types
            .input_locations(machine.platonic())
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

    let (output_edges, topological_ordering) = match region.module_type_id() {
      Some(id) => self.module_geometries.get(&id).unwrap(),
      None => &self.global_region_geometry,
    };

    let machines: Vec<_> = region.machines().collect();
    for &machine_index in topological_ordering {
      let machine: &WorldMachineView<(BaseAspect,)> = &machines[machine_index];
      let inputs = MachineObservedInputs {
        input_flows: &result.machines[machine.index_within_region()].inputs,
        start_time: machine.last_disturbed_time().unwrap_or(0),
      };
      let future = machine.machine_type().future(inputs);

      let outputs = match (machine.as_module(), &future) {
        (Some(module), Ok(MachineFuture::Module(module_machine_future))) => {
          let inner_region = module.inner_region();

          let fiat_inputs: Vec<_> = module
            .platonic()
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
              .entry(machine.platonic().id_within_region())
              .or_insert(inner_future) // should always insert, but doing it this way to get a reference back
          } else {
            let platonic_module_futures = undisturbed_modules_futures
              .entry(machine.platonic().type_id)
              .or_default();

            match platonic_module_futures.get(&module_machine_future.canonical_inputs) {
              Some(e) => e,
              None => {
                let inner_future =
                  self.region_future(undisturbed_modules_futures, &inner_region, &fiat_inputs);

                match undisturbed_modules_futures.get_mut(&machine.platonic().type_id).unwrap().entry(module_machine_future.canonical_inputs.clone()) {
                  hash_map::Entry::Occupied(_) => unreachable!("A module's future was modified during calculation of its submodules' futures. Did a module get put inside itself somehow?"),
                  hash_map::Entry::Vacant(entry) => entry.insert(inner_future)
                }
              }
            }
          };
          module
            .platonic()
            .module_output_flows(inputs, module_machine_future, variation)
        }
        (_, Ok(future)) => machine.machine_type().output_flows(inputs, future),
        (_, Err(_)) => inputs![],
      };

      result.machines[machine_index].future = future;

      //println!("{:?}\n{:?}\n{:?}\n\n", machine, inputs , outputs);
      for ((flow, destination), location) in
        outputs.into_iter().zip(&output_edges[machine_index]).zip(
          machine
            .machine_type()
            .output_locations(machine.platonic().state.position),
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
      &GameView::new(self).global_region(),
      &[],
    );
    GameFuture {
      global_region,
      undisturbed_modules,
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

pub trait BaseAspectShared<'a>: WorldViewAspect<'a> {
  fn is_module(machine: &'a Self::Machine) -> bool;
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

impl<'a, T: WorldViewAspectGetMut<'a> + BaseAspectShared<'a>> WorldMachineView<'a, T> {
  pub fn as_module_mut(&'a mut self) -> Option<WorldModuleView<'a, T>> {
    if T::is_module(&self.aspects) {
      Some(WorldModuleView {
        aspects: T::as_module_mut(&mut self.aspects),
      })
    } else {
      None
    }
  }
}

impl<'a, T: WorldViewAspectGet<'a> + BaseAspectShared<'a>> WorldMachineView<'a, T> {
  pub fn as_module(&'a self) -> Option<WorldModuleView<'a, T>> {
    if T::is_module(&self.aspects) {
      Some(WorldModuleView {
        aspects: T::as_module(&self.aspects),
      })
    } else {
      None
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
   (($BaseAspect: ident, $($OtherAspect: ident,)*)) => {
    impl_world_views_for_aspect_tuple!($BaseAspect, ($BaseAspect, $($OtherAspect,)*), ($BaseAspect, $($OtherAspect,)*));
   };
   ($BaseAspect: ident, ($($Aspect: ident,)*), $Tuple: tt) => {

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

impl<'a> BaseAspectShared<'a> for $Tuple {
  fn is_module(machine: &'a Self::Machine) -> bool {
    $BaseAspect::is_module(&machine.0)
  }
}
)*

  };
  (&mut ($($Aspect: ident,)*)) => {
impl_world_views_for_aspect_tuple!(($($Aspect,)*));
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
impl_world_views_for_aspect_tuple!(($($Aspect,)*));
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

  #[derive(Copy, Clone, Debug)]
  pub struct GameView<'a> {
    game: &'a Game,
  }

  #[derive(Copy, Clone, Debug)]
  pub struct WorldRegionView<'a> {
    game: GameView<'a>,
    platonic: &'a PlatonicRegionContents,
    containing_module: Option<&'a WorldModuleView<'a>>,
    isomorphism: GridIsomorphism,
    last_disturbed_times: Option<&'a WorldMachinesMap<Number>>,
  }

  #[derive(Copy, Clone, Debug)]
  pub struct WorldMachineView<'a> {
    game: GameView<'a>,
    platonic: &'a PlatonicMachine,
    machine_type: MachineTypeRef<'a>,
    isomorphism: GridIsomorphism,
    parent: &'a WorldRegionView<'a>,
    index_within_parent: usize,
  }

  #[derive(Copy, Clone, Debug)]
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
        containing_module: None,
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
        containing_module: Some(module),
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
  impl<'a> BaseAspectShared<'a> for BaseAspect {
    fn is_module(machine: &'a Self::Machine) -> bool {
      match machine
        .game
        .game
        .machine_types
        .get(machine.platonic.type_id)
      {
        MachineTypeRef::Module(_) => true,
        _ => false,
      }
    }
  }

  impl<'a, T: GetSubaspect<'a, BaseAspect>> super::GameView<'a, T> {
    pub fn game(&'a self) -> &'a Game {
      T::get_game_aspect(&self.aspects).game
    }
  }

  impl<'a, T: GetSubaspect<'a, BaseAspect>> super::WorldRegionView<'a, T> {
    pub fn platonic(&'a self) -> &'a PlatonicRegionContents {
      T::get_region_aspect(&self.aspects).platonic
    }
  }

  impl<'a, T: GetSubaspect<'a, BaseAspect>> super::WorldRegionView<'a, T> {
    pub fn module_type_id(&'a self) -> Option<MachineTypeId> {
      T::get_region_aspect(&self.aspects)
        .containing_module
        .map(|module| module.as_machine.platonic.type_id)
    }
  }

  impl<'a, T: GetSubaspect<'a, BaseAspect>> super::WorldMachineView<'a, T> {
    pub fn platonic(&'a self) -> &'a PlatonicMachine {
      T::get_machine_aspect(&self.aspects).platonic
    }
  }

  impl<'a, T: GetSubaspect<'a, BaseAspect>> super::WorldMachineView<'a, T> {
    pub fn index_within_region(&'a self) -> usize {
      T::get_machine_aspect(&self.aspects).index_within_parent
    }
  }

  impl<'a, T: GetSubaspect<'a, BaseAspect>> super::WorldMachineView<'a, T> {
    pub fn machine_type(&'a self) -> MachineTypeRef<'a> {
      let aspect = T::get_machine_aspect(&self.aspects);
      aspect.game.game.machine_types.get(aspect.platonic.type_id)
    }
  }

  impl<'a, T: GetSubaspect<'a, BaseAspect>> super::WorldMachineView<'a, T> {
    pub fn last_disturbed_time(&'a self) -> Option<Number> {
      let aspect = T::get_machine_aspect(&self.aspects);
      aspect
        .parent
        .last_disturbed_times
        .and_then(|times| times.here.get(&aspect.platonic.id_within_region()).copied())
    }
  }

  impl<'a, T: GetSubaspect<'a, BaseAspect>> super::WorldModuleView<'a, T> {
    pub fn platonic(&'a self) -> &'a PlatonicModule {
      T::get_module_aspect(&self.aspects).platonic
    }
  }

  pub struct MachinesIter<'a, T: WorldViewAspect<'a>> {
    region: &'a <T as WorldViewAspect<'a>>::Region,
    ids: std::vec::IntoIter<ViewMachineIds>,
  }
  impl<'a, T: WorldViewAspect<'a> + WorldViewAspectGet<'a>> Iterator for MachinesIter<'a, T> {
    type Item = super::WorldMachineView<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
      self.ids.next().map(|ids| super::WorldMachineView {
        aspects: T::get_machine(self.region, ids),
      })
    }
  }
  impl<'a, T: GetSubaspect<'a, BaseAspect>> super::WorldRegionView<'a, T> {
    pub fn machines(&'a self) -> MachinesIter<'a, T> {
      let region_base = T::get_region_aspect(&self.aspects);
      let ids: Vec<_> = region_base
        .platonic
        .machines
        .iter()
        .enumerate()
        .map(|(index, machine)| ViewMachineIds {
          index,
          id_within_region: machine.id_within_region(),
          type_id: machine.type_id,
        })
        .collect();
      MachinesIter {
        ids: ids.into_iter(),
        region: &self.aspects,
      }
    }
  }
}

pub use self::base_mut_view_aspect::BaseMutAspect;
pub mod base_mut_view_aspect {
  use super::*;
  use machine_data::WorldMachinesMap;

  pub enum BaseMutAspect {}

  #[derive(Debug)]
  struct ViewGlobals {
    pub change_time: Number,
  }

  #[derive(Debug)]
  pub struct GameView<'a> {
    globals: ViewGlobals,
    game: &'a mut Game,
  }

  #[derive(Debug)]
  pub struct WorldRegionView<'a> {
    globals: &'a ViewGlobals,
    machine_types: &'a mut MachineTypes,
    platonic: &'a mut PlatonicRegionContents,
    output_edges: OutputEdges,
    isomorphism: GridIsomorphism,
    // Only None if an ancestor was disturbed at change_time
    last_disturbed_times: Option<&'a mut WorldMachinesMap<Number>>,
  }

  #[derive(Debug)]
  pub struct WorldMachineView<'a> {
    parent: &'a mut WorldRegionView<'a>,
    index_within_parent: usize,
  }

  #[derive(Debug)]
  pub struct WorldModuleView<'a> {
    as_machine: &'a mut WorldMachineView<'a>,
  }

  impl<'a> WorldViewAspect<'a> for BaseMutAspect {
    type Game = GameView<'a>;
    type Region = WorldRegionView<'a>;
    type Machine = WorldMachineView<'a>;
    type Module = WorldModuleView<'a>;
  }
  impl<'a> WorldViewAspectGetMut<'a> for BaseMutAspect {
    fn global_region_mut(game: &'a mut Self::Game) -> Self::Region {
      let output_edges = game
        .game
        .global_region
        .output_edges(&game.game.machine_types);
      WorldRegionView {
        globals: &game.globals,
        machine_types: &mut game.game.machine_types,
        platonic: &mut game.game.global_region,
        output_edges,
        isomorphism: GridIsomorphism::default(),
        last_disturbed_times: Some(&mut game.game.last_disturbed_times),
      }
    }
    fn get_machine_mut(region: &'a mut Self::Region, ids: ViewMachineIds) -> Self::Machine {
      let machine = &region.platonic.machines[ids.index];
      WorldMachineView {
        parent: region,
        index_within_parent: ids.index,
      }
    }
    fn as_module_mut(machine: &'a mut Self::Machine) -> Self::Module {
      WorldModuleView {
        as_machine: machine,
      }
    }
    fn inner_region_mut(module: &'a mut Self::Module) -> Self::Region {
      let parent = module.as_machine.parent;
      let platonic_machine = &mut parent.platonic.machines[module.as_machine.index_within_parent];
      let id_within_region = platonic_machine.id_within_region();
      let isomorphism = platonic_machine.state.position * parent.isomorphism;
      let old_platonic_module = match parent.machine_types.get(platonic_machine.type_id) {
        MachineTypeRef::Module(m) => m,
        _ => unreachable!(),
      };
      let mut new_platonic_module = old_platonic_module.clone();
      let new_module_index = parent.machine_types.custom_modules.len();
      platonic_machine.type_id = MachineTypeId::Module(new_module_index);
      let output_edges = new_platonic_module
        .region
        .output_edges(&parent.machine_types);
      parent
        .machine_types
        .custom_modules
        .push(new_platonic_module);
      WorldRegionView {
        globals: parent.globals,
        machine_types: parent.machine_types,
        platonic,
        output_edges,
        isomorphism,
        last_disturbed_times: parent
          .last_disturbed_times
          .and_then(|times| times.children.get_mut(&id_within_region)),
      } /*
        WorldRegionView {
          last_disturbed_times: module
            .as_machine
            .parent
            .last_disturbed_times
            .and_then(|times| {
              times
                .children
                .get(&module.as_machine.platonic.id_within_region())
            }),
        }*/
    }
  }
  impl<'a> BaseAspectShared<'a> for BaseMutAspect {
    fn is_module(machine: &'a Self::Machine) -> bool {
      match machine
        .parent
        .machine_types
        .get(machine.parent.platonic.machines[machine.index_within_parent].type_id)
      {
        MachineTypeRef::Module(_) => true,
        _ => false,
      }
    }
  }

  impl<'a> Drop for WorldRegionView<'a> {
    fn drop(&mut self) {
      if let Some(times) = &mut self.last_disturbed_times {
        times
          .children
          .retain(|key, value| !(value.here.is_empty() && value.children.is_empty()));
      }
    }
  }

  impl<'a> Drop for GameView<'a> {
    fn drop(&mut self) {
      self.game.canonicalize();
    }
  }
}

pub use self::future_view_aspect::FutureAspect;
pub mod future_view_aspect {
  use super::*;
  use modules::ModuleMachineFuture;

  pub enum FutureAspect {}

  #[derive(Copy, Clone, Debug)]
  pub struct GameView<'a> {
    future: &'a GameFuture,
  }

  #[derive(Copy, Clone, Debug)]
  pub struct WorldRegionView<'a> {
    game: GameView<'a>,
    start_time_and_future: Option<(Number, &'a RegionFuture)>,
  }

  #[derive(Copy, Clone, Debug)]
  pub struct WorldMachineView<'a> {
    game: GameView<'a>,
    parent: &'a WorldRegionView<'a>,
    ids: ViewMachineIds,
    future: Option<&'a MachineAndInputsFuture>,
  }

  #[derive(Copy, Clone, Debug)]
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
        ids,
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
                machine.parent.start_time_and_future.unwrap().0 + module_machine_future.start_time,
                module_machine_future,
                machine.parent.start_time_and_future.unwrap().1.disturbed_children.get(&machine.ids.id_within_region).unwrap_or_else(||
                    machine
                        .game
                        .future
                        .undisturbed_modules
                        .get(& machine.ids.type_id)
                        .expect("there shouldn't be a ModuleMachineFuture if there isn't a corresponding ModuleFuture")
                        .get(&module_machine_future.canonical_inputs)
                        .expect("there shouldn't be a ModuleMachineFuture if there isn't a corresponding future-variation")),
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
}*/
