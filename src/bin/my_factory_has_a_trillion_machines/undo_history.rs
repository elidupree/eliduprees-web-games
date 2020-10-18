use crate::geometry::Number;
use crate::machine_data::Game;
use graph_algorithms::{GameFuture, GameView, WorldMachineView, WorldModuleView, WorldRegionView};
use live_prop_test::{live_prop_test, lpt_assert, lpt_assert_eq};
use machine_data::{MachineTypeId, TIME_TO_MOVE_MATERIAL};
use modules::CanonicalModuleInputs;
use std::collections::HashSet;

pub trait ModifyGame {
  #[live_prop_test(
    precondition = "game.is_canonical()",
    postcondition = "check_modify_game(&old(game.clone()), game, time)"
  )]
  fn undo(&self, game: &mut Game, time: Number);
}

#[live_prop_test]
pub trait UndoableModifyGame {
  #[live_prop_test(
    precondition = "game.is_canonical()",
    postcondition = "check_undoable_modify_game(&old(game.clone()), game, time, &*result)"
  )]
  fn modify_game(&self, game: &mut Game, time: Number) -> Box<dyn ModifyGame>;
}

fn check_modify_game(before: &Game, after: &Game, modify_time: Number) -> Result<(), String> {
  lpt_assert!(after.is_canonical());
  lpt_assert_eq!(
    after.inventory_before_last_change,
    before_view.inventory_at(modify_time)
  );
  // Note: Null changes COULD be allowed to not change last_change_time...
  // but also maybe they shouldn't be a ModifyGame at all, because they probably shouldn't go in the undo history?
  lpt_assert_eq!(after.last_change_time, modify_time);
  Ok(())
}

type Aspects = (Platonic, LastDisturbed);

fn check_undoable_modify_game<Undo: ModifyGame + ?Sized>(
  before: &Game,
  after: &Game,
  modify_time: Number,
  undo: &Undo,
) -> Result<(), String> {
  check_modify_game(before, after, modify_time)?;
  let before_view = GameView {
    game: before,
    future,
  };
  lpt_assert_eq!(
    after.inventory_before_last_change,
    before_view.inventory_at(modify_time)
  );
  lpt_assert!(after.is_canonical());
  // we'd like to assert that every absolute disturbed time is either the same as before or is now...
  // except how do we tell which machines are the "same"?

  let after_future = after.future();
  let after_view = GameView {
    game: after,
    future: &after_future,
  };
  check_undo(before_view, after_view, undo, modify_time)?;
  check_undo(
    before_view,
    after_view,
    undo,
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
  verified_undisturbed_module_pairs: HashSet<[usize; 2]>,
}

fn check_undo<Undo: UndoModifyGame + ?Sized>(
  before: GameView<Aspects>,
  after: GameView<Aspects>,
  undo: &Undo,
  undo_time: Number,
) -> Result<(), String> {
  let mut undone = after.game.clone();
  undo.undo(&mut undone, &after.future, undo_time);
  lpt_assert!(undone.is_canonical());
  lpt_assert_eq!(undone.last_change_time, undo_time);
  lpt_assert_eq!(
    undone.inventory_before_last_change,
    after.inventory_at(undo_time)
  );
  let undone_future = undone.future();
  let undone = GameView {
    game: &undone,
    future: &undone_future,
  };
  CheckUndoneMap {
    //before,
    //modify_time,
    //undone,
    undo_time,
    verified_undisturbed_module_pairs: HashSet::new(),
  }
  .maps_undo_compatible(before.global_region(), undone.global_region())
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
    before_map: WorldRegionView<Aspects>,
    undone_map: WorldRegionView<Aspects>,
  ) -> Result<(), String> {
    let before_machines: Vec<_> = before_map.machines().collect();
    let undone_machines: Vec<_> = undone_map.machines().collect();
    lpt_assert_eq!(before_machines.len(), undone_machines.len());
    for (before_machine, undone_machine) in before_machines.into_iter().zip(undone_machines) {
      self.machines_undo_compatible(before_machine, undone_machine)?;
    }
    Ok(())
  }

  fn machines_undo_compatible(
    &mut self,
    before_machine: WorldMachineView<Aspects>,
    undone_machine: WorldMachineView<Aspects>,
  ) -> Result<(), String> {
    lpt_assert_eq!(
      before_machine.platonic().state.position,
      undone_machine.platonic().state.position
    );

    if let Some(undone_disturbed_time) = undone_machine.last_disturbed_time() {
      if undone_disturbed_time != self.undo_time {
        lpt_assert_eq!(undone_machine.last_disturbed_time(), self.undo_time);
      }
    }

    match (
      before_machine.platonic().type_id,
      undone_machine.platonic().type_id,
    ) {
      (MachineTypeId::Preset(before_index), MachineTypeId::Preset(undone_index)) => {
        lpt_assert_eq!(before_index, undone_index)
      }
      (MachineTypeId::Module(before_index), MachineTypeId::Module(undone_index)) => {
        let before_module = before_machine.module().unwrap();
        let undone_module = undone_machine.module().unwrap();
        // short-circuit on undisturbed module pairings to avoid an exponential search.
        if before_module.as_machine().last_disturbed_time().is_some()
          || before_module.as_machine().last_disturbed_time().is_some()
          || self
            .verified_undisturbed_module_pairs
            .insert([before_index, undone_index])
        {
          lpt_assert_eq!(
            before_module.platonic().module_type,
            undone_module.platonic().module_type
          );
          self.maps_undo_compatible(before_module.region(), undone_module.region())?;
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
