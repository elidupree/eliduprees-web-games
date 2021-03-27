/**

An internal representation of the UI state.

This layer is agnostic about what frontend is used. In particular, it's aware of clicking and dragging, at fractional pointer positions within the world, but it's agnostic about how scrolling and zooming are accomplished. Our main (read: only) frontend is a web UI using leaflet, and leaflet has its own approach to scrolling, so this layer does not control scrolling (although it must be aware of the scroll position for some things – at the very least, to decide what machines are visible when drawing).

This layer's interface with the frontend: The frontend produces scrolling and pointer gestures and reports them to this layer. This layer reports back the visible machines/etc that must be drawn on-screen.

This layer's interface with the backend: This layer produces AddRemoveMachines instructions and applies them to the backend, then examines the resulting game states.

*/
use crate::geometry::{Number, Vector};
use crate::graph_algorithms::GameFuture;
use crate::machine_data::{
  Game, MachineMomentaryVisuals, Material, PlatonicMachine, WorldMachinesMap,
};
use live_prop_test::{live_prop_test, lpt_assert_eq};
use nalgebra::Vector2;
use std::collections::HashMap;

#[derive(Copy, Clone)]
struct MouseGridPosition {
  tile_center: Vector,
  nearest_lines: Vector,
}

#[derive(Clone, Debug)]
enum Selection {
  NormalMachines(WorldMachinesMap<()>),
  GhostMachinesMovedFrom {
    selected: WorldMachinesMap<()>,
    offset: Vector,
  },
  NovelGhostMachines(Vec<PlatonicMachine>),
}

#[derive(Clone, Debug)]
enum DragType {
  MoveMachines,
  RectangleSelect,
}

#[derive(Clone, Debug)]
struct DragState {
  original_position: Vector2<f64>,
  drag_type: DragType,
}

#[derive(Clone, Debug)]
enum Mode {
  Panning,
  Selection,
  PrimitiveMachine(usize),
}

#[derive(Clone, Debug)]
enum ImplicitMode {
  Normal(Mode),
  GhostMachines,
}

#[derive(Debug)]
pub struct UiState {
  game: Game,

  /// a cache; should always equal game.future()
  future: GameFuture,
  current_game_time: Number,

  // note that the ghost machine definitions may refer to machine types from `game`,
  // so the UI bits want to be a dependent type rather than separate
  mode: Mode,
  selected: Option<Selection>,
  drag: Option<DragState>,
}

struct DisplayedMachine {
  machine: PlatonicMachine,
  momentary_visuals: MachineMomentaryVisuals,
}

pub struct View {
  selection: Option<[Vector2<f64>; 2]>,
  machines: Vec<DisplayedMachine>,
  inventory: HashMap<Material, Number>,
}

impl UiState {
  fn implicit_mode(&self) -> ImplicitMode {
    match self.selected {
      Some(Selection::GhostMachinesMovedFrom { .. }) | Some(Selection::NovelGhostMachines(..)) => {
        ImplicitMode::GhostMachines
      }
      _ => ImplicitMode::Normal(self.mode.clone()),
    }
  }
}

#[live_prop_test]
impl UiState {
  pub fn check_invariants(&self) -> Result<(), String> {
    self.game.check_invariants()?;
    lpt_assert_eq!(self.future, self.game.future());
    Ok(())
  }

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn click_map(&mut self, position: Vector2<f64>) {
    match self.implicit_mode() {
      ImplicitMode::Normal(Mode::PrimitiveMachine(machine_type_id)) => todo!("build machine"),
      ImplicitMode::GhostMachines => self.discard_ghost_machines().unwrap(),
      _ => {}
    }
  }

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn click_rotate_selection(&mut self, clockwise: bool) {}

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn click_flip_selection(&mut self) {}

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn click_duplicate_selection(&mut self) {}

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn click_delete_selection(&mut self) {}

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn drag_map(&mut self, position: Vector2<f64>) {}

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn drag_duplicate_selection(&mut self, position: Vector2<f64>) {}

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn drag_primitive_machine(&mut self, position: Vector2<f64>) {}

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn continue_drag(&mut self, position: Vector2<f64>, is_over_map: bool) {}

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn release_drag(&mut self, position: Vector2<f64>, is_over_map: bool) {}

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn discard_ghost_machines(&mut self) -> Result<(), ()> {
    match self.selected.clone() {
      Some(Selection::GhostMachinesMovedFrom {
        selected,
        offset: _,
      }) => {
        self.selected = Some(Selection::NormalMachines(selected));
      }
      Some(Selection::NovelGhostMachines(machines)) => {
        //refund materials
        self.selected = None;
      }
      _ => return Err(()),
    }
    Ok(())
  }

  #[live_prop_test(
    precondition = "self.check_invariants()",
    postcondition = "self.check_invariants()"
  )]
  pub fn undo(&mut self) {}

  pub fn view(&self) -> View {
    todo!()
  }
}
