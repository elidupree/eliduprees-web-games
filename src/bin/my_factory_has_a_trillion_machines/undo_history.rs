use crate::geometry::Number;
use crate::machine_data::Game;
use graph_algorithms::{GameFuture, GameViewWithFuture, MachineViewWithFuture, MapViewWithFuture};
use live_prop_test::{live_prop_test, lpt_assert, lpt_assert_eq};
use machine_data::{MachineTypeId, TIME_TO_MOVE_MATERIAL};
use std::collections::HashSet;

pub trait UndoModifyGame {
  fn undo(&self, game: &mut Game, future: &GameFuture, time: Number);
}

#[live_prop_test]
pub trait ModifyGame {
  #[live_prop_test(
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
  let before_view = GameViewWithFuture {
    game: before,
    future,
  };
  lpt_assert_eq!(
    after.inventory_before_last_change,
    before_view.inventory_at(time)
  );
  lpt_assert!(before.is_canonical());
  lpt_assert!(after.is_canonical());
  // todo: every absolute disturbed time is either the same as before or is now

  let after_future = after.future();
  let after_view = GameViewWithFuture {
    game: after,
    future: &after_future,
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

struct CheckUndoneMap<'a> {
  before: GameViewWithFuture<'a>,
  //modify_time: Number,
  undone: GameViewWithFuture<'a>,
  undo_time: Number,
  verified_module_pairs: HashSet<(usize, usize)>,
}

fn check_undo<Undo: UndoModifyGame + ?Sized>(
  before: GameViewWithFuture,
  after: GameViewWithFuture,
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
  let undone = GameViewWithFuture {
    game: &undone,
    future: &undone_future,
  };
  CheckUndoneMap {
    before,
    //modify_time,
    undone,
    undo_time,
    verified_module_pairs: HashSet::new(),
  }
  .maps_undo_compatible(before.map(), undone.map())
}

impl<'a> CheckUndoneMap<'a> {
  fn maps_undo_compatible(
    &mut self,
    before_map: MapViewWithFuture,
    undone_map: MapViewWithFuture,
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
    before_machine: MachineViewWithFuture,
    undone_machine: MachineViewWithFuture,
  ) -> Result<(), String> {
    lpt_assert_eq!(
      before_machine.machine.state.position,
      undone_machine.machine.state.position
    );
    match (
      &before_machine.map_start_time_and_machine_future,
      &undone_machine.map_start_time_and_machine_future,
    ) {
      (Some((before_start, _before_future)), Some((undone_start, _undone_future))) => {
        let before_absolute_disturbed_time =
          before_start + before_machine.machine.state.last_disturbed_time;
        let undone_absolute_disturbed_time =
          undone_start + undone_machine.machine.state.last_disturbed_time;
        if undone_absolute_disturbed_time != before_absolute_disturbed_time {
          lpt_assert_eq!(undone_absolute_disturbed_time, self.undo_time);
        }
      }
      (None, None) => {
        // right now, last_disturbed_time doesn't matter inside of modules that aren't operating.
        // we might eventually require it to be 0 in that case, but currently, we don't.
      }
      _ => {
        return Err(format!(
          "One machine had a future and the other didn't: {:?}, {:?}",
          before_machine, undone_machine
        ))
      }
    }

    match (
      before_machine.machine.type_id,
      undone_machine.machine.type_id,
    ) {
      (MachineTypeId::Preset(before_index), MachineTypeId::Preset(undone_index)) => {
        lpt_assert_eq!(before_index, undone_index)
      }
      (MachineTypeId::Module(before_index), MachineTypeId::Module(undone_index)) => {
        if self
          .verified_module_pairs
          .insert((before_index, undone_index))
        {
          let before_module = self
            .before
            .game
            .machine_types
            .get_module(before_machine.machine.type_id);
          let undone_module = self
            .undone
            .game
            .machine_types
            .get_module(undone_machine.machine.type_id);
          lpt_assert_eq!(before_module.module_type, undone_module.module_type);
          lpt_assert_eq!(before_module.cost, undone_module.cost);
          self.maps_undo_compatible(
            before_machine.module_map().unwrap(),
            undone_machine.module_map().unwrap(),
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
