/**

An internal representation of the UI state.

This layer is agnostic about what frontend is used. In particular, it's aware of clicking and dragging, at fractional pointer positions within the world, but it's agnostic about how scrolling and zooming are accomplished. Our main (read: only) frontend is a web UI using leaflet, and leaflet has its own approach to scrolling, so this layer does not control scrolling (although it must be aware of the scroll position for some things – at the very least, to decide what machines are visible when drawing).

This layer's interface with the frontend: The frontend produces scrolling and pointer gestures and reports them to this layer. This layer reports back the visible machines/etc that must be drawn on-screen.

This layer's interface with the backend: This layer produces AddRemoveMachines instructions and applies them to the backend, then examines the resulting game states.

*/
use crate::geometry::{Number, Vector};
use crate::graph_algorithms::{
  BaseAspect, FutureAspect, GameFuture, GameView, WorldMachineView, WorldRegionView,
};
use crate::machine_data::{Game, GlobalMachine, MachineMomentaryVisuals, Material};
use live_prop_test::{live_prop_test, lpt_assert_eq};
use nalgebra::Vector2;
use std::collections::{HashMap, HashSet};

#[derive(Copy, Clone)]
struct MouseGridPosition {
  tile_center: Vector,
  nearest_lines: Vector,
}

#[derive(Clone, Debug)]
enum Selection {
  NormalMachines(HashSet<GlobalMachine>),
  HoveringMachinesMovedFrom {
    source_machines: HashSet<GlobalMachine>,
    offset: Vector,
  },
  NovelHoveringMachines(Vec<GlobalMachine>),
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
  HoveringMachines,
}

#[derive(Debug)]
pub struct UiState {
  game: Game,

  /// a cache; should always equal game.future()
  future: GameFuture,
  current_game_time: Number,

  // note that the hovering machine definitions may refer to machine types from `game`,
  // so the UI bits want to be a dependent type rather than separate
  mode: Mode,
  selected: Selection,
  drag: Option<DragState>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum MachineRealness {
  Normal,
  Hovering,
  Hypothetical,
}

struct DisplayedMachine {
  machine: GlobalMachine,
  momentary_visuals: Option<MachineMomentaryVisuals>,
  realness: MachineRealness,
  selected: bool,
}

pub struct DisplayedStuff {
  selection_rectangle: Option<[Vector2<f64>; 2]>,
  machines: Vec<DisplayedMachine>,
  inventory: HashMap<Material, Number>,
}

impl UiState {
  fn implicit_mode(&self) -> ImplicitMode {
    match self.selected {
      Selection::HoveringMachinesMovedFrom { .. } | Selection::NovelHoveringMachines(..) => {
        ImplicitMode::HoveringMachines
      }
      _ => ImplicitMode::Normal(self.mode.clone()),
    }
  }
}

type StateViewAspects = (BaseAspect, FutureAspect);

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
      ImplicitMode::HoveringMachines => self.discard_hovering_machines().unwrap(),
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
  pub fn discard_hovering_machines(&mut self) -> Result<(), ()> {
    match self.selected.clone() {
      Selection::HoveringMachinesMovedFrom {
        source_machines, ..
      } => {
        self.selected = Selection::NormalMachines(source_machines);
      }
      Selection::NovelHoveringMachines(machines) => {
        // TODO: refund materials
        self.selected = Selection::NormalMachines(HashSet::new());
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

  pub fn set_current_game_time(&mut self, time: Number) {
    self.current_game_time = time;
  }

  pub fn displayed_stuff(&self, display_filter: impl Fn(&GlobalMachine) -> bool) -> DisplayedStuff {
    let collector = DisplayedStuffCollector::new(self, display_filter);
    collector.collect_all()
  }
}

struct DisplayedStuffCollector<'a, F> {
  state: &'a UiState,
  display_filter: F,
  result: DisplayedStuff,
}

impl<'a, F: Fn(&GlobalMachine) -> bool> DisplayedStuffCollector<'a, F> {
  fn new(state: &'a UiState, display_filter: F) -> Self {
    DisplayedStuffCollector {
      state,
      display_filter,
      result: DisplayedStuff {
        selection_rectangle: None,
        machines: Vec::new(),
        inventory: Default::default(),
      },
    }
  }

  fn collect_machine(
    &mut self,
    machine: WorldMachineView<StateViewAspects>,
    parent_realness: MachineRealness,
  ) {
    let global = machine.global();
    if !(self.display_filter)(&global) {
      return;
    }

    let mut realness = parent_realness;
    let mut selected = realness == MachineRealness::Hovering;
    match self.state.selected {
      Selection::HoveringMachinesMovedFrom {
        source_machines, ..
      } => {
        if realness == MachineRealness::Normal && source_machines.contains(&global) {
          // If we got here, we're looking at the version of the hovering machine at its *original* location
          realness = MachineRealness::Hypothetical;
        }
      }
      Selection::NormalMachines(machines) => {
        if machines.contains(&global) {
          selected = true;
        }
      }
      _ => {}
    }

    self.result.machines.push(DisplayedMachine {
      machine: global,
      momentary_visuals: machine.momentary_visuals(self.state.current_game_time),
      realness,
      selected,
    });

    if let Some(module) = machine.as_module() {
      self.collect_region(module.inner_region(), realness);
    }
  }

  fn collect_region(
    &mut self,
    region: WorldRegionView<StateViewAspects>,
    parent_realness: MachineRealness,
  ) {
    for machine in region.machines() {
      self.collect_machine(machine, parent_realness);
    }
  }

  fn collect_all(self) -> DisplayedStuff {
    let view = GameView::<StateViewAspects>::new(&self.state.game, &self.state.future);

    self.collect_region(view.global_region(), MachineRealness::Normal);

    match self.state.selected.clone() {
      Selection::HoveringMachinesMovedFrom {
        source_machines,
        offset,
      } => {
        for machine in source_machines {
          machine.state.position.translation += offset;
          self.collect_machine(machine, MachineRealness::Hovering)
        }
      }
      Selection::NovelHoveringMachines(machines) => {
        for machine in machines {
          self.collect_machine(machine, MachineRealness::Hovering)
        }
      }
      _ => {}
    }

    // TODO: hypothetical conveyor belts arising from drag state

    self.result
  }
}
