use crate::geometry::Number;
use crate::machine_data::Game;
use geometry::GridIsomorphism;
use graph_algorithms::{
  BaseAspect, BaseMutAspect, FutureAspect, GameFuture, GameView, SelectedAspect, SelectedMutAspect,
  WorldMachineView, WorldRegionView,
};
use live_prop_test::{live_prop_test, lpt_assert, lpt_assert_eq};
use machine_data::{
  MachineGlobalId, MachineTypeId, PlatonicMachine, WorldMachinesMap, TIME_TO_MOVE_MATERIAL,
};
use std::collections::HashSet;

//#[live_prop_test]
pub trait ModifyGame: Clone {
  /*#[live_prop_test(
    precondition = "game.is_canonical()",
    precondition = "future == &game.future()",
    postcondition = "check_modify_game(&old(game.clone()), game, &old(selected.clone()), selected, time)"
  )]*/
  fn modify_game(
    self,
    game: &mut Game,
    selected: &mut WorldMachinesMap<()>,
    future: &GameFuture,
    time: Number,
  );
}

#[live_prop_test]
pub trait ModifyGameUndoable: Clone {
  type Undo: ModifyGame;
  #[live_prop_test(
    precondition = "game.is_canonical()",
    precondition = "future == &game.future()",
    postcondition = "check_undoable_modify_game(&old(game.clone()), game, &old(selected.clone()), selected, time, &result)"
  )]
  fn modify_game_undoable(
    self,
    game: &mut Game,
    selected: &mut WorldMachinesMap<()>,
    future: &GameFuture,
    time: Number,
  ) -> Self::Undo;
}

impl<T: ModifyGameUndoable> ModifyGame for T {
  fn modify_game(
    self,
    game: &mut Game,
    selected: &mut WorldMachinesMap<()>,
    future: &GameFuture,
    time: Number,
  ) {
    self.modify_game_undoable(game, selected, future, time);
  }
}

fn check_modify_game(
  game_before: &Game,
  game_after: &Game,
  selected_before: &WorldMachinesMap<()>,
  selected_after: &WorldMachinesMap<()>,
  modify_time: Number,
) -> Result<(), String> {
  lpt_assert!(game_after.is_canonical());
  // TODO implement this check:
  /*
  let before_future = before.future();
  let before_material_totals = before.material_totals();
  let before_view = GameView::<Aspects>::new(before, &before_future);
  let after_material_totals = after.material_totals();
  lpt_assert_eq!(
    after.inventory_before_last_change + after_material_totals.global_region,
    before_view.inventory_at(modify_time) + before_material_totals.global_region
  );
  */
  // Note: Null changes COULD be allowed to not change last_change_time...
  // but also maybe they shouldn't be a ModifyGame at all, because they probably shouldn't go in the undo history?
  lpt_assert_eq!(game_after.last_change_time, modify_time);
  Ok(())
}

type AspectsForCheckModifyGame = (BaseAspect, SelectedAspect, FutureAspect);

fn check_undoable_modify_game<Undo: ModifyGame>(
  game_before: &Game,
  game_after: &Game,
  selected_before: &WorldMachinesMap<()>,
  selected_after: &WorldMachinesMap<()>,
  modify_time: Number,
  undo: &Undo,
) -> Result<(), String> {
  check_modify_game(
    game_before,
    game_after,
    selected_before,
    selected_after,
    modify_time,
  )?;
  let before_future = game_before.future();
  let before_view =
    GameView::<AspectsForCheckModifyGame>::new(game_before, selected_before, &before_future);
  // we'd like to assert that every absolute disturbed time is either the same as before or is now...
  // except how do we tell which machines are the "same"?

  let after_future = game_after.future();
  let after_view =
    GameView::<AspectsForCheckModifyGame>::new(game_after, selected_after, &after_future);
  check_undo(&before_view, &after_view, undo.clone(), modify_time)?;
  check_undo(
    &before_view,
    &after_view,
    undo.clone(),
    modify_time + TIME_TO_MOVE_MATERIAL * 33 + 67,
  )?;
  Ok(())
}

struct CheckUndoneMap
//<'a>
{
  //before: GameViewWithFuture<'a>,
  //modify_time: Number,
  //undone: GameViewWithFuture<'a>,
  undo_time: Number,
  visited_module_pairs_without_explicit_world_data: HashSet<[usize; 2]>,
}

fn check_undo<Undo: ModifyGame>(
  before: &GameView<AspectsForCheckModifyGame>,
  after: &GameView<AspectsForCheckModifyGame>,
  undo: Undo,
  undo_time: Number,
) -> Result<(), String> {
  let mut undone_game = after.game().clone();
  let mut undone_selected = after.selected().clone();
  undo.modify_game(
    &mut undone_game,
    &mut undone_selected,
    &after.future(),
    undo_time,
  );
  check_modify_game(
    after.game(),
    &undone_game,
    after.selected(),
    &undone_selected,
    undo_time,
  )?;
  let undone_future = undone_game.future();
  let undone =
    GameView::<AspectsForCheckModifyGame>::new(&undone_game, &undone_selected, &undone_future);

  CheckUndoneMap {
    //before,
    //modify_time,
    //undone,
    undo_time,
    visited_module_pairs_without_explicit_world_data: HashSet::new(),
  }
  .maps_undo_compatible(before.global_region(), undone.global_region(), false)
}

impl CheckUndoneMap {
  fn maps_undo_compatible(
    &mut self,
    before_map: WorldRegionView<AspectsForCheckModifyGame>,
    undone_map: WorldRegionView<AspectsForCheckModifyGame>,
    any_ancestor_module_machine_disturbed_by_undo: bool,
  ) -> Result<(), String> {
    let before_machines: Vec<_> = before_map.machines().collect();
    let undone_machines: Vec<_> = undone_map.machines().collect();
    lpt_assert_eq!(before_machines.len(), undone_machines.len());
    for (before_machine, undone_machine) in before_machines.into_iter().zip(undone_machines) {
      self.machines_undo_compatible(
        before_machine,
        undone_machine,
        any_ancestor_module_machine_disturbed_by_undo,
      )?;
    }
    Ok(())
  }

  fn machines_undo_compatible(
    &mut self,
    before_machine: WorldMachineView<AspectsForCheckModifyGame>,
    undone_machine: WorldMachineView<AspectsForCheckModifyGame>,
    any_ancestor_module_machine_disturbed_by_undo: bool,
  ) -> Result<(), String> {
    lpt_assert_eq!(
      before_machine.platonic().state.position,
      undone_machine.platonic().state.position
    );
    if any_ancestor_module_machine_disturbed_by_undo {
      lpt_assert_eq!(undone_machine.last_disturbed_time(), None);
    } else {
      lpt_assert!(
        undone_machine.last_disturbed_time() == before_machine.last_disturbed_time()
          || undone_machine.last_disturbed_time() == Some(self.undo_time)
      );
    }

    match (
      before_machine.platonic().type_id,
      undone_machine.platonic().type_id,
    ) {
      (MachineTypeId::Preset(before_index), MachineTypeId::Preset(undone_index)) => {
        lpt_assert_eq!(before_index, undone_index)
      }
      (MachineTypeId::Module(before_index), MachineTypeId::Module(undone_index)) => {
        // short-circuit on undisturbed module pairings to avoid an exponential search.
        let module_machine_disturbed = undone_machine.last_disturbed_time() == Some(self.undo_time);
        let before_module = before_machine.as_module().unwrap();
        let undone_module = undone_machine.as_module().unwrap();
        let before_inner_region = before_module.inner_region();
        let undone_inner_region = undone_module.inner_region();
        if before_inner_region.last_disturbed_times().is_some()
          || undone_inner_region.last_disturbed_times().is_some()
          || before_inner_region.selected().is_some()
          || undone_inner_region.selected().is_some()
          || self
            .visited_module_pairs_without_explicit_world_data
            .insert([before_index, undone_index])
        {
          lpt_assert_eq!(
            before_module.platonic().module_type,
            undone_module.platonic().module_type
          );
          self.maps_undo_compatible(
            before_inner_region,
            undone_inner_region,
            any_ancestor_module_machine_disturbed_by_undo || module_machine_disturbed,
          )?;
        }
      }
      _ => {
        return Err(format!(
          "One machine was a module and the other wasn't: {:?}, {:?}",
          before_machine, undone_machine
        ))
      }
    }
    Ok(())
  }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct AddRemoveMachines {
  added: Vec<PlatonicMachine>,
  removed: Vec<MachineGlobalId>,
}

//impl_world_views_for_aspect_tuple!(&mut (BaseMutAspect, SelectedMutAspect,));

type AddRemoveMachinesAspects = (BaseMutAspect, SelectedMutAspect);
#[live_prop_test(use_trait_tests)]
impl ModifyGameUndoable for AddRemoveMachines {
  type Undo = AddRemoveMachines;

  fn modify_game_undoable(
    mut self,
    game: &mut Game,
    selected: &mut WorldMachinesMap<()>,
    future: &GameFuture,
    time: Number,
  ) -> AddRemoveMachines {
    let undo_removed: Vec<_> = self
      .added
      .iter()
      .map(|machine| machine.global_id(GridIsomorphism::default(), &game.machine_types))
      .collect();
    let mut undo_added = Vec::new();

    fn handle_region(
      mut region: WorldRegionView<AddRemoveMachinesAspects>,
      mut added: &mut [PlatonicMachine],
      mut removed: &mut [MachineGlobalId],
      undo_added: &mut Vec<PlatonicMachine>,
    ) {
      region.retain_machines(|machine| {
        if removed.contains(&machine.global_id()) {
          undo_added.push(machine.global_platonic().clone());
          false
        } else {
          if let Some(module) = machine.as_module() {
            let num_added_here = added
              .iter_mut()
              .partition_in_place(|machine| module.contains_global_id(machine));
            let (added_here, added_elsewhere) = added.split_at_mut(num_added_here);
            added = added_elsewhere;

            let num_removed_here = added
              .iter_mut()
              .partition_in_place(|id| module.contains_global_id(id));
            let (removed_here, removed_elsewhere) = removed.split_at_mut(num_added_here);
            removed = removed_elsewhere;

            if !(added_here.is_empty() && removed_here.is_empty()) {
              handle_region(module.inner_region(), added_here, removed_here, undo_added);
            }
          }
          true
        }
      });

      if !added.is_empty() {
        region.insert_machines(added.iter().cloned().map(|mut machine| {
          machine.state.position = machine.state.position / region.isomorphism();
          machine
        }));
      }
    }

    let mut game_view =
      GameView::<AddRemoveMachinesAspects>::new(BaseMutAspect::new(game, time, future), selected);
    handle_region(
      game_view.global_region_mut(),
      &mut self.added,
      &mut self.removed,
      &mut undo_added,
    );

    AddRemoveMachines {
      added: undo_added,
      removed: undo_removed,
    }
  }
}

/*
trait ModifyGameVisitor {
  type GameView: GameView;
  fn should_enter_module(module: &Self::GameView::MapView::ModuleView) -> bool;
  fn modify_game(&mut self) {}
}

struct AbstractModifyGame<V: ModifyGameVisitor> {
  game: V::GameView,
  visitor: V,
}

impl<V: ModifyGameVisitor> AbstractModifyGame<V> {
  pub fn modify_game(&mut self) {
    self.modify_map(self.game.map());
  }
  pub fn modify_map(&mut self, map: V::GameView::MapView) {
    for machine in map.machines() {
      if let Some(module) = machine.module() {
        if self.visitor.should_enter_module(&module) {
          self.modify_module(module);
        }
      }
    }
  }
  pub fn modify_module(&mut self, module: ModuleView) {}
}

fn delete_selected_machines_from_map(map: mutMapView_with_mut_selections_and_mut_last_disturbed) {
  for machine_id in selections.children.keys() {
    delete_selected_machines_from_map(map.get_machine(machine_id).module().unwrap().map())
  }
  if !selected.here.is_empty() {
    map
      .data
      .machines
      .retain(|machine| !map.selected.here.contains(machine.id()));
    map
      .last_disturbed
      .here
      .retain(|(machine_id, last_disturbed)| !map.selected.here.contains(machine_id));
    map.selected.here.clear();
  }
}

trait GameViewAugmentation {
  type Map;
  type Machine;
  type Module;
  fn map(&self) -> Self::Map;
  fn machine(
    &self,
    augmentation_map: &Self::Map,
    machine_data: &MachineView,
  ) -> Option<Self::Machine>;
  fn module(
    &self,
    augmentation_map: &Self::Map,
    machine_id: &MachineIdWithinMap,
    machine: &Self::Machine,
  ) -> Option<Self::Module>;
  fn module_map(&self);
}

pub struct WorldMachinesMap<T> {}
pub struct WorldMachinesMapNode<T> {
  here: HashMap<MachineIdWithinMap, T>,
  children: HashMap<MachineIdWithinMap, RepresentedMachinesMapNode<T>>,
}*/
