use crate::geometry::Number;
use crate::graph_algorithms::MapFuture;
use crate::machine_data::{Game, Map};
use live_prop_test::{live_prop_test, lpt_assert_eq};
use machine_data::{MachineTypeId, StatefulMachine};
use std::collections::HashSet;

pub trait UndoModifyGame {
  fn undo(&self, game: &mut Game, future: &MapFuture, time: Number);
}

#[live_prop_test]
pub trait ModifyGame {
  #[live_prop_test(
    postcondition = "check_modify_game(&old(game.clone()), game, future, time, &*result)"
  )]
  fn modify_game(
    &self,
    game: &mut Game,
    future: &MapFuture,
    time: Number,
  ) -> Box<dyn UndoModifyGame>;
}

fn check_modify_game<Undo: UndoModifyGame + ?Sized>(
  before: &Game,
  after: &Game,
  future: &MapFuture,
  time: Number,
  undo: &Undo,
) -> Result<(), String> {
  // Note: Null changes COULD be allowed to not change last_change_time...
  // but also maybe they shouldn't be a ModifyGame at all, because they probably shouldn't go in the undo history?
  lpt_assert_eq!(after.last_change_time, time);
  lpt_assert_eq!(
    after.inventory_before_last_change,
    before.inventory_at(future, time)
  );

  let future_after = unimplemented!();
  check_undo(before, after, &future_after, undo, time)?;
  check_undo(before, after, &future_after, undo, time + 67)?;
  Ok(())
}

struct CheckUndoneMap<'a> {
  before: &'a Game,
  //modify_time: Number,
  undone: &'a Game,
  undo_time: Number,
  verified_module_pairs: HashSet<(usize, usize)>,
}

fn check_undo<Undo: UndoModifyGame + ?Sized>(
  before: &Game,
  after: &Game,
  future_after: &MapFuture,
  undo: &Undo,
  undo_time: Number,
) -> Result<(), String> {
  let mut undone = after.clone();
  undo.undo(&mut undone, future_after, undo_time);
  lpt_assert_eq!(undone.last_change_time, undo_time);
  lpt_assert_eq!(
    undone.inventory_before_last_change,
    after.inventory_at(future_after, undo_time)
  );
  CheckUndoneMap {
    before,
    //modify_time,
    undone: &undone,
    undo_time,
    verified_module_pairs: HashSet::new(),
  }
  .maps_undo_compatible(&before.map, 0, &undone.map, 0)
}

impl<'a> CheckUndoneMap<'a> {
  fn maps_undo_compatible(
    &mut self,
    before_map: &Map,
    before_containing_module_start_time: Number,
    undone_map: &Map,
    undone_containing_module_start_time: Number,
  ) -> Result<(), String> {
    lpt_assert_eq!(before_map.machines.len(), undone_map.machines.len());
    for (before_machine, undone_machine) in before_map.machines.iter().zip(&undone_map.machines) {
      self.machines_undo_compatible(
        before_machine,
        before_containing_module_start_time,
        undone_machine,
        undone_containing_module_start_time,
      )?;
    }
    Ok(())
  }

  fn machines_undo_compatible(
    &mut self,
    before_machine: &StatefulMachine,
    before_containing_module_start_time: Number,
    undone_machine: &StatefulMachine,
    undone_containing_module_start_time: Number,
  ) -> Result<(), String> {
    lpt_assert_eq!(before_machine.state.position, undone_machine.state.position);
    let before_absolute_disturbed_time =
      before_containing_module_start_time + before_machine.state.last_disturbed_time;
    let undone_absolute_disturbed_time =
      undone_containing_module_start_time + undone_machine.state.last_disturbed_time;
    if undone_absolute_disturbed_time != before_absolute_disturbed_time {
      lpt_assert_eq!(undone_absolute_disturbed_time, self.undo_time);
    }
    match (before_machine.type_id, undone_machine.type_id) {
      (MachineTypeId::Preset(before_index), MachineTypeId::Preset(undone_index)) => {
        lpt_assert_eq!(before_index, undone_index)
      }
      (MachineTypeId::Module(before_index), MachineTypeId::Module(undone_index)) => {
        if self
          .verified_module_pairs
          .insert((before_index, undone_index))
        {
          let before_module = self.before.machine_types.get_module(before_machine.type_id);
          let undone_module = self.undone.machine_types.get_module(undone_machine.type_id);
          lpt_assert_eq!(before_module.module_type, undone_module.module_type);
          lpt_assert_eq!(before_module.cost, undone_module.cost);
          self.maps_undo_compatible(
            &before_module.map,
            before_containing_module_start_time + 0,
            &undone_module.map,
            undone_containing_module_start_time + 0,
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
