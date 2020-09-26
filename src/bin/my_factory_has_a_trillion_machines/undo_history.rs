use crate::geometry::Number;
use crate::machine_data::Game;
use graph_algorithms::{GameFuture, GameView, WorldMachineView, WorldModuleView, WorldRegionView};
use live_prop_test::{live_prop_test, lpt_assert, lpt_assert_eq};
use machine_data::{MachineTypeId, TIME_TO_MOVE_MATERIAL};
use modules::CanonicalModuleInputs;
use std::collections::HashSet;

pub trait UndoModifyGame {
  fn undo(&self, game: &mut Game, future: &GameFuture, time: Number);
}

#[live_prop_test]
pub trait ModifyGame {
  #[live_prop_test(
    precondition = "game.is_canonical()",
    precondition = "*future == game.future()",
    postcondition = "check_modify_game(&old(game.clone()), game, future, time, &*result)"
  )]
  fn modify_game(
    &self,
    game: &mut Game,
    future: &GameFuture,
    time: Number,
  ) -> Box<dyn UndoModifyGame>;
}

fn check_modify_game<Undo: UndoModifyGame + ?Sized>(
  before: &Game,
  after: &Game,
  future: &GameFuture,
  time: Number,
  undo: &Undo,
) -> Result<(), String> {
  // Note: Null changes COULD be allowed to not change last_change_time...
  // but also maybe they shouldn't be a ModifyGame at all, because they probably shouldn't go in the undo history?
  lpt_assert_eq!(after.last_change_time, time);
  let before_view = GameView {
    game: before,
    future: Some(future),
    selected: None,
  };
  lpt_assert_eq!(
    after.inventory_before_last_change,
    before_view.inventory_at(time)
  );
  lpt_assert!(after.is_canonical());
  // we'd like to assert that every absolute disturbed time is either the same as before or is now...
  // except how do we tell which machines are the "same"?

  let after_future = after.future();
  let after_view = GameView {
    game: after,
    future: Some(&after_future),
    selected: None,
  };
  check_undo(before_view, after_view, undo, time)?;
  check_undo(
    before_view,
    after_view,
    undo,
    time + TIME_TO_MOVE_MATERIAL * 33 + 67,
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
  visited_module_pairs_without_explicit_world_data:
    HashSet<[(usize, Option<CanonicalModuleInputs>); 2]>,
}

fn check_undo<Undo: UndoModifyGame + ?Sized>(
  before: GameView,
  after: GameView,
  undo: &Undo,
  undo_time: Number,
) -> Result<(), String> {
  let mut undone = after.game.clone();
  undo.undo(&mut undone, &after.future.unwrap(), undo_time);
  lpt_assert!(undone.is_canonical());
  lpt_assert_eq!(undone.last_change_time, undo_time);
  lpt_assert_eq!(
    undone.inventory_before_last_change,
    after.inventory_at(undo_time)
  );
  let undone_future = undone.future();
  let undone = GameView {
    game: &undone,
    future: Some(&undone_future),
    selected: None,
  };
  CheckUndoneMap {
    //before,
    //modify_time,
    //undone,
    undo_time,
    visited_module_pairs_without_explicit_world_data: HashSet::new(),
  }
  .maps_undo_compatible(before.global_region(), undone.global_region(), false)
}

fn module_canonical_inputs(module: WorldModuleView) -> Option<CanonicalModuleInputs> {
  module.inner_start_time_and_module_future.map(
    |(_inner_start_time, module_machine_future, _module_map_future)| {
      module_machine_future.canonical_inputs.clone()
    },
  )
}

impl CheckUndoneMap {
  fn maps_undo_compatible(
    &mut self,
    before_map: WorldRegionView,
    undone_map: WorldRegionView,
    any_ancestor_disturbed: bool,
  ) -> Result<(), String> {
    let before_machines: Vec<_> = before_map.machines().collect();
    let undone_machines: Vec<_> = undone_map.machines().collect();
    lpt_assert_eq!(before_machines.len(), undone_machines.len());
    for (before_machine, undone_machine) in before_machines.into_iter().zip(undone_machines) {
      self.machines_undo_compatible(
        before_machine,
        undone_machine,
        any_ancestor_disturbed || undone_map.last_disturbed_times.is_none(),
      )?;
    }
    Ok(())
  }

  fn machines_undo_compatible(
    &mut self,
    before_machine: WorldMachineView,
    undone_machine: WorldMachineView,
    any_ancestor_disturbed: bool,
  ) -> Result<(), String> {
    lpt_assert_eq!(
      before_machine.platonic.state.position,
      undone_machine.platonic.state.position
    );
    if any_ancestor_disturbed {
      lpt_assert_eq!(undone_machine.last_disturbed_time, None);
    } else {
      lpt_assert!(
        undone_machine.last_disturbed_time == before_machine.last_disturbed_time
          || undone_machine.last_disturbed_time == Some(self.undo_time)
      );
    }

    match (
      before_machine.platonic.type_id,
      undone_machine.platonic.type_id,
    ) {
      (MachineTypeId::Preset(before_index), MachineTypeId::Preset(undone_index)) => {
        lpt_assert_eq!(before_index, undone_index)
      }
      (MachineTypeId::Module(before_index), MachineTypeId::Module(undone_index)) => {
        let before_module = before_machine.as_module().unwrap();
        let undone_module = undone_machine.as_module().unwrap();
        // short-circuit on repeated module pairings to avoid an exponential search.
        // theoretically, the two versions of the module, even with the same canonical inputs,
        // could still differ in their start_time.
        // However the start_times should only be different if the start_times are all
        // AFTER undo_time, which rules out any last_disturbed_time-related errors,
        // assuming last_disturbed_times inside modules are never negative.
        // TODO: actually assert that the start_times are all after undo_time in this case,
        // or prove that the other test already catch that.
        let before_region = before_module.region();
        let undone_region = undone_module.region();
        if before_region.last_disturbed_times.is_some()
          || undone_region.last_disturbed_times.is_some()
          || before_region.selected.is_some()
          || undone_region.selected.is_some()
          || self
            .visited_module_pairs_without_explicit_world_data
            .insert([
              (before_index, module_canonical_inputs(before_module)),
              (undone_index, module_canonical_inputs(undone_module)),
            ])
        {
          lpt_assert_eq!(
            before_module.platonic.module_type,
            undone_module.platonic.module_type
          );
          lpt_assert_eq!(before_module.platonic.cost, undone_module.platonic.cost);
          self.maps_undo_compatible(before_region, undone_region, any_ancestor_disturbed)?;
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
