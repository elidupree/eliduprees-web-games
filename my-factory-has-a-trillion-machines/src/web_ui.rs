use super::*;

use nalgebra::Vector2;
use num::Integer;
use serde::Deserialize;
use std::cell::RefCell;
use std::cmp::max;
use std::collections::VecDeque;
use std::mem;
use wasm_bindgen::prelude::*;

use crate::geometry::{Facing, GridIsomorphism, Number, Rotate, Rotation, Vector, VectorExtension};
use crate::graph_algorithms::{
  BaseAspect, FutureAspect, GameFuture, GameView, SelectedAspect, WorldRegionView,
};
use crate::machine_data::{
  Game, MachineState, MachineType, MachineTypeId, MachineTypeTrait, MachineTypes, Material,
  PlatonicMachine, PlatonicRegionContents, WorldMachinesMap, TIME_TO_MOVE_MATERIAL,
};
use crate::undo_history::AddRemoveMachines;
//use misc;
//use modules::{self, Module};

mod js {
  use wasm_bindgen::prelude::*;

  #[wasm_bindgen]
  extern "C" {
    pub fn init_machine_type(machine_type_name: String);
    pub fn gather_dom_samples() -> JsValue;
    // this wants to return (), but that gets me "clear_canvas is not defined" for some reason
    pub fn clear_canvas() -> JsValue;
    pub fn draw_sprite(
      sprite: &str,
      cx: f32,
      cy: f32,
      sx: f32,
      sy: f32,
      quarter_turns_from_posx_towards_posy: u8,
    );
    pub fn update_inventory(inventory: JsValue);
  }
}

thread_local! {
  static STATE: RefCell<State> = {
    let mut game = Game {
      global_region: PlatonicRegionContents {
        machines: Vec::new(),
      },
      last_change_time: 0,
      inventory_before_last_change: Default::default(),
      undo_stack: Vec::new(),
      machine_types: MachineTypes {
        presets: machine_presets(),
        custom_modules: Vec::new(),
      },
      last_disturbed_times: WorldMachinesMap::default(),
      redo_stack: Vec::new(),
    };
    game
      .inventory_before_last_change
      .insert(Material::Iron, 1000);
    let future = game.future();
    RefCell::new(State {
      game,
      selected: WorldMachinesMap::default(),
      future,
      start_ui_time: now(),
      start_game_time: 0,
      current_game_time: 0,
      mouse: Default::default(),
      queued_mouse_moves: VecDeque::new(),
    })
  }
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
  STATE.with(|state| {
    let mut guard = state.borrow_mut();
    (f)(&mut *guard)
  })
}

#[derive(Copy, Clone)]
struct MouseGridPosition {
  tile_center: Vector,
  nearest_lines: Vector,
}

#[wasm_bindgen]
#[derive(Copy, Clone, Deserialize)]
pub struct MouseCssPositionOnMap {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
}

#[wasm_bindgen]
impl MouseCssPositionOnMap {
  #[wasm_bindgen(constructor)]
  pub fn from_js_value(v: JsValue) -> Self {
    v.into_serde().unwrap()
  }
}

#[wasm_bindgen]
#[derive(PartialEq, Eq, Clone, Deserialize)]
pub struct ClickType {
  pub buttons: u16,
  pub shift: bool,
  pub ctrl: bool,
}

#[wasm_bindgen]
impl ClickType {
  #[wasm_bindgen(constructor)]
  pub fn from_js_value(v: JsValue) -> Self {
    v.into_serde().unwrap()
  }
}

const REGULAR_CLICK: ClickType = ClickType {
  buttons: 1,
  shift: false,
  ctrl: false,
};

impl Default for ClickType {
  fn default() -> Self {
    REGULAR_CLICK
  }
}

#[derive(Clone)]
struct DragState {
  original_position: MouseGridPosition,
  click_type: ClickType,
  moved: bool,
}

#[derive(Default)]
struct MouseState {
  drag: Option<DragState>,
  position: Option<MouseGridPosition>,
  previous_position: Option<MouseGridPosition>,
}

struct State {
  game: Game,
  selected: WorldMachinesMap<()>,
  future: GameFuture,
  start_ui_time: f64,
  start_game_time: Number,
  current_game_time: Number,
  mouse: MouseState,
  queued_mouse_moves: VecDeque<MouseCssPositionOnMap>,
}

type StateViewAspects = (BaseAspect, SelectedAspect, FutureAspect);
impl State {
  fn view(&self) -> GameView<StateViewAspects> {
    GameView::<StateViewAspects>::new(&self.game, &self.selected, &self.future)
  }
}

#[derive(Clone, Deserialize)]
struct DomSamples {
  map_zoom: f64,
  map_css_scale: f64,
  map_world_center: Vector2<f64>,
  canvas_backing_size: Vector2<f64>,
  canvas_css_size: Vector2<f64>,
  device_pixel_ratio: f64,
  current_mode: String,
}

impl DomSamples {
  fn gather() -> Self {
    js::gather_dom_samples().into_serde().unwrap()
  }
  fn map_backing_scale(&self) -> f64 {
    self.map_css_scale * self.device_pixel_ratio
  }
}

fn machine_presets() -> Vec<MachineType> {
  vec![
    primitive_machines::conveyor(),
    primitive_machines::splitter(),
    primitive_machines::iron_smelter(),
    primitive_machines::iron_mine(),
    modules::basic_module(),
  ]
}

fn canvas_position(samples: &DomSamples, position: Vector) -> Vector2<f32> {
  canvas_position_from_f64(samples, position.to_f64())
}
fn canvas_position_from_f64(samples: &DomSamples, position: Vector2<f64>) -> Vector2<f32> {
  let scale = samples.map_backing_scale();
  let center = samples.map_world_center;
  let relative = (position - center) * scale;
  let canvas_center = samples.canvas_backing_size * 0.5;
  Vector2::new(
    (canvas_center[0] + relative[0]) as f32,
    (canvas_center[1] - relative[1]) as f32,
  )
}
fn tile_canvas_size(samples: &DomSamples) -> Vector2<f32> {
  let scale = samples.map_backing_scale() * 2.0;
  Vector2::new(scale as f32, scale as f32)
}
fn tile_position(samples: &DomSamples, css_position: Vector2<f64>) -> MouseGridPosition {
  let world = samples.map_world_center
    + (css_position - samples.canvas_css_size * 0.5) / samples.map_css_scale;
  MouseGridPosition {
    tile_center: Vector::new(
      (world[0] * 0.5).floor() as Number * 2 + 1,
      (world[1] * 0.5).floor() as Number * 2 + 1,
    ),
    nearest_lines: Vector::new(
      (world[0] * 0.5).round() as Number * 2,
      (world[1] * 0.5).round() as Number * 2,
    ),
  }
}

fn draw_rectangle(center: Vector2<f32>, size: Vector2<f32>, sprite: &str, rotation: Rotation) {
  js::draw_sprite(
    sprite,
    center[0],
    center[1],
    size[0],
    size[1],
    rotation.quarter_turns_from_posx_towards_posy(),
  );
}

fn inside_machine(
  machine_types: &MachineTypes,
  position: Vector,
  machine: &PlatonicMachine,
) -> bool {
  let machine_type = machine_types.get(machine.type_id);
  let radius = machine_type.radius();
  let offset = machine.state.position.translation - position;
  offset[0].abs() < radius && offset[1].abs() < radius
}

#[wasm_bindgen]
pub fn rust_init() {
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));
  live_prop_test::initialize();

  // let json_callback = {
  //   let state = state.clone();
  //   move |input: String| {
  //     println!("{}", &input);
  //     if let Ok(game) = serde_json::from_str::<Game>(&input) {
  //       let mut state = state.borrow_mut();
  //       state.start_ui_time = now();
  //       state.start_game_time = game.last_change_time;
  //       state.current_game_time = game.last_change_time;
  //       state.game = game;
  //       recalculate_future(&mut state);
  //     }
  //   }
  // };

  // js! {
  //   $("#json").click(function() {$(this).select();})
  //     .on ("input", function() {@{json_callback}($(this).val())});
  // }
  with_state(|state| {
    for name in state
      .game
      .machine_types
      .presets
      .iter()
      .map(|machine_type| machine_type.as_ref().name().to_owned())
    {
      js::init_machine_type(name);
    }
  });
}

#[wasm_bindgen]
pub fn rust_mousedown(position: MouseCssPositionOnMap, click_type: ClickType) {
  with_state(|state| {
    let samples = DomSamples::gather();
    mouse_move(
      state,
      &samples,
      tile_position(&samples, Vector2::new(position.x, position.y)),
    );
    mouse_down(state, &samples, click_type);
  })
}

#[wasm_bindgen]
pub fn rust_mouseup(position: MouseCssPositionOnMap) {
  with_state(|state| {
    let samples = DomSamples::gather();
    mouse_move(
      state,
      &samples,
      tile_position(&samples, Vector2::new(position.x, position.y)),
    );
    mouse_up(state, &samples);
  })
}

#[wasm_bindgen]
pub fn rust_mousemove(position: MouseCssPositionOnMap) {
  with_state(|state| {
    queue_mouse_move(state, position);
  })
}

fn with_smallest_region_containing<F: FnOnce(WorldRegionView<StateViewAspects>) -> R, R>(
  state: &State,
  (position, radius): (Vector, Number),
  callback: F,
) -> R {
  fn recurse<F: FnOnce(WorldRegionView<StateViewAspects>) -> R, R>(
    region: WorldRegionView<StateViewAspects>,
    (position, radius): (Vector, Number),
    callback: F,
  ) -> R {
    for machine in region.machines() {
      if let Some(module) = machine.as_module() {
        let relative_position = position - machine.isomorphism().translation;
        let available_radius = module.platonic().module_type.inner_radius - radius;
        if max(relative_position[0].abs(), relative_position[1].abs()) <= available_radius {
          return recurse(module.inner_region(), (position, radius), callback);
        }
      }
    }
    callback(region)
  }
  recurse(state.view().global_region(), (position, radius), callback)
}

fn build_machine(state: &mut State, machine_type_id: MachineTypeId, position: GridIsomorphism) {
  let machine_type = state.game.machine_types.get(machine_type_id);

  let inventory = state.view().inventory_at(state.current_game_time);
  for (amount, material) in machine_type.cost() {
    if inventory
      .get(&material)
      .map_or(true, |storage| storage < amount)
    {
      // can't build – you can't afford it
      return;
    }
  }

  let obstructed = with_smallest_region_containing(
    state,
    (position.translation, machine_type.radius()),
    |region| {
      region.machines().any(|machine| {
        let radius = machine.machine_type().radius() + machine_type.radius();
        let offset = (position / machine.isomorphism()).translation;
        offset[0].abs() < radius && offset[1].abs() < radius
      })
    },
  );

  if obstructed {
    // can't build – something is in the way
    return;
  }

  //let machine_type = state.game.machine_types.get(machine_type_id);
  /*for (amount, material) in machine_type.cost() {
    *state
      .game
      .inventory_before_last_change
      .get_mut(&material)
      .unwrap() -= amount;
  }*/

  state.game.add_remove_machines(
    AddRemoveMachines {
      added: vec![PlatonicMachine {
        type_id: machine_type_id,
        state: MachineState { position },
      }],
      removed: vec![],
    },
    &mut state.selected,
    &state.future,
    state.current_game_time,
  );

  recalculate_future(state);
}

fn recalculate_future(state: &mut State) {
  state.game.inventory_before_last_change = state.view().inventory_at(state.current_game_time);
  state.game.last_change_time = state.current_game_time;

  state.game.canonicalize();

  state.future = state.game.future();

  /*js!{
    $("#json").val (@{serde_json::to_string_pretty (&state.game).unwrap()});
  }*/
}

fn hovering_area(
  state: &State,
  samples: &DomSamples,
  position: MouseGridPosition,
) -> (Vector, Number) {
  if let Some(machine_type) = state
    .game
    .machine_types
    .presets
    .iter()
    .find(|machine_type| machine_type.as_ref().name() == samples.current_mode)
  {
    (
      if machine_type.as_ref().radius().is_even() {
        position.nearest_lines
      } else {
        position.tile_center
      },
      machine_type.as_ref().radius(),
    )
  } else {
    (position.tile_center, 1)
  }
}

fn queue_mouse_move(state: &mut State, mouse_move: MouseCssPositionOnMap) {
  if state.queued_mouse_moves.len() >= 100 {
    state.queued_mouse_moves.pop_front();
  }
  state.queued_mouse_moves.push_back(mouse_move);
}
fn mouse_move(state: &mut State, samples: &DomSamples, position: MouseGridPosition) {
  let facing = match state.mouse.position {
    None => Facing::default(),
    Some(previous_position) => {
      let delta = hovering_area(state, samples, position).0
        - hovering_area(state, samples, previous_position).0;
      match delta.exact_facing() {
        Some(facing) => facing,
        _ => loop {
          let difference = hovering_area(state, samples, position).0
            - hovering_area(state, samples, state.mouse.position.unwrap()).0;
          if difference[0] != 0 {
            let offs = Vector::new(difference[0].signum() * 2, 0);
            let mut pos = state.mouse.position.unwrap();
            pos.tile_center += offs;
            pos.nearest_lines += offs;
            mouse_move(state, samples, pos);
          }
          let difference = hovering_area(state, samples, position).0
            - hovering_area(state, samples, state.mouse.position.unwrap()).0;
          if difference[1] != 0 {
            let offs = Vector::new(0, difference[1].signum() * 2);
            let mut pos = state.mouse.position.unwrap();
            pos.tile_center += offs;
            pos.nearest_lines += offs;
            mouse_move(state, samples, pos);
          }
          if hovering_area(state, samples, position).0
            == hovering_area(state, samples, state.mouse.position.unwrap()).0
          {
            return;
          }
        },
      }
    }
  };

  state.mouse.previous_position = state.mouse.position;
  state.mouse.position = Some(position);

  if let Some(ref mut drag) = state.mouse.drag {
    drag.moved = true;
    let drag = state.mouse.drag.clone().unwrap();
    if let Some(previous) = state.mouse.previous_position {
      if drag.click_type == REGULAR_CLICK
        && (hovering_area(state, samples, previous).0
          == hovering_area(state, samples, drag.original_position).0
          || samples.current_mode == "Conveyor")
      {
        /*
        let path = smallest_region_containing(state, (previous.tile_center, 1));
        let (region, isomorphism, start_time) = path.get_region(&state.game);
        let inner_position = previous.tile_center.transformed_by(isomorphism.inverse());
        let inner_now = start_time.map_or(0, |start_time| state.current_game_time - start_time);

        if let Some(rotated_index) = region
          .machines
          .iter()
          .position(|machine| inside_machine(&state.game.machine_types, inner_position, machine))
        {
          path.modify_region(&mut state.game, |machine_types, region| {
            region.modify_machines(machine_types, vec![rotated_index], inner_now, |machine| {
              machine.state.position = machine
                .state
                .position
                .with_rotation_changed_to_make_facing_transform_to(
                  Facing::default().transformed_by(isomorphism),
                  facing,
                )
            });
          });
          recalculate_future(state);
        }
        */
      }
    }
  }
  mouse_maybe_held(state, samples);
}

fn mouse_down(state: &mut State, samples: &DomSamples, click_type: ClickType) {
  state.mouse.drag = Some(DragState {
    original_position: state.mouse.position.unwrap(),
    click_type,
    moved: false,
  });
  mouse_maybe_held(state, samples);
}

fn mouse_maybe_held(state: &mut State, samples: &DomSamples) {
  let facing = match (state.mouse.previous_position, state.mouse.position) {
    (Some(first), Some(second)) => (hovering_area(state, samples, second).0
      - hovering_area(state, samples, first).0)
      .exact_facing()
      .unwrap_or_default(),
    _ => Facing::default(),
  };
  if let Some(drag) = state.mouse.drag.clone() {
    let position = state.mouse.position.unwrap();
    let hover = hovering_area(state, samples, position);
    if drag.click_type == REGULAR_CLICK && samples.current_mode == "Conveyor" {
      build_machine(
        state,
        MachineTypeId::Preset(
          state
            .game
            .machine_types
            .presets
            .iter()
            .position(|machine_type| machine_type == &primitive_machines::conveyor())
            .unwrap(),
        ),
        GridIsomorphism {
          translation: hover.0,
          rotation: facing - Facing::default(),
          ..Default::default()
        },
      );
    }

    if drag.click_type
      == (ClickType {
        buttons: 2,
        ..Default::default()
      })
    {
      /*
      let path = smallest_region_containing(state, hover);
      let (region, isomorphism, start_time) = path.get_region(&state.game);
      let inner_position = position.tile_center.transformed_by(isomorphism.inverse());
      let inner_now = start_time.map_or(0, |start_time| state.current_game_time - start_time);

      if let Some(deleted_index) = region
        .machines
        .iter()
        .position(|machine| inside_machine(&state.game.machine_types, inner_position, machine))
      {
        let machine_type = state
          .game
          .machine_types
          .get(region.machines[deleted_index].type_id);
        let cost = machine_type.cost();

        let inventory = &mut state.game.inventory_before_last_change;
        for (amount, material) in cost {
          *inventory.get_mut(&material).unwrap() += amount;
        }

        path.modify_region(&mut state.game, |machine_types, region| {
          region.remove_machines(machine_types, vec![deleted_index], inner_now);
        });
        recalculate_future(state);
      }
      */
    }
  }
}

fn mouse_up(state: &mut State, samples: &DomSamples) {
  if let Some(drag) = state.mouse.drag.clone() {
    if drag.click_type == REGULAR_CLICK && !drag.moved {
      if let Some(preset_index) = state
        .game
        .machine_types
        .presets
        .iter()
        .position(|machine_type| machine_type.as_ref().name() == samples.current_mode)
      {
        build_machine(
          state,
          MachineTypeId::Preset(preset_index),
          GridIsomorphism {
            translation: hovering_area(state, samples, drag.original_position).0,
            ..Default::default()
          },
        );
      }
    }
  }
  state.mouse.drag = None;
}

fn draw_region(
  samples: &DomSamples,
  region: WorldRegionView<StateViewAspects>,
  absolute_time: Number,
) {
  for machine in region.machines() {
    let radius = machine.machine_type().radius();
    let size = Vector2::new(
      tile_canvas_size(samples)[0] * (radius * 2) as f32 / 2.0,
      tile_canvas_size(samples)[1] * (radius * 2) as f32 / 2.0,
    );
    draw_rectangle(
      canvas_position(samples, machine.isomorphism().translation),
      size,
      "rounded-rectangle-transparent",
      Rotation::default(),
    );
    draw_rectangle(
      canvas_position(samples, machine.isomorphism().translation),
      size,
      machine.machine_type().icon(),
      machine.isomorphism().rotation,
    );
  }
  for machine in region.machines() {
    if machine.machine_type().radius() > 1 {
      for (input_location, expected_material) in machine
        .input_locations()
        .into_iter()
        .zip(machine.machine_type().input_materials())
      {
        let pos = canvas_position(
          samples,
          input_location.position + input_location.facing.unit_vector(),
        );
        draw_rectangle(
          pos,
          tile_canvas_size(samples),
          "input",
          input_location.facing - Facing::default(),
        );
        if let Some(material) = expected_material {
          draw_rectangle(
            pos,
            tile_canvas_size(samples) * 0.8,
            material.icon(),
            Rotation::default(),
          );
        }
      }
    }
  }
  for machine in region.machines() {
    if machine.machine_type().radius() > 1 {
      for output_location in machine.output_locations() {
        draw_rectangle(
          canvas_position(
            samples,
            output_location.position - output_location.facing.unit_vector(),
          ),
          tile_canvas_size(samples),
          "input",
          output_location.facing.rotate_90(2) - Facing::default(),
        );
      }
    }
  }

  for machine in region.machines() {
    if let Some(visuals) = machine.momentary_visuals(absolute_time) {
      for (position, material) in visuals.materials {
        draw_rectangle(
          canvas_position_from_f64(samples, position),
          tile_canvas_size(samples) * 0.6,
          material.icon(),
          Rotation::default(),
        );
      }
    }
  }

  for machine in region.machines() {
    if let Some(module) = machine.as_module() {
      draw_region(samples, module.inner_region(), absolute_time);
    }
  }
}

#[wasm_bindgen]
pub fn do_frame() {
  let samples = DomSamples::gather();

  with_state(|state| {
    let mut tweaked_samples = samples.clone();
    for MouseCssPositionOnMap {
      x,
      y,
      width,
      height,
    } in mem::take(&mut state.queued_mouse_moves)
    {
      // just in case there was a queued mouse move followed by a redraw,
      // make sure to get the positioning correct based on the size at the time
      tweaked_samples.canvas_css_size = Vector2::new(width, height);
      mouse_move(
        state,
        &samples,
        tile_position(&tweaked_samples, Vector2::new(x, y)),
      );
    }

    let fractional_time = state.start_game_time as f64
      + (now() - state.start_ui_time) * TIME_TO_MOVE_MATERIAL as f64 * 2.0;
    state.current_game_time = fractional_time as Number;

    js::clear_canvas();

    //target.clear_color(1.0, 1.0, 1.0, 1.0);
    draw_region(
      &samples,
      state.view().global_region(),
      state.current_game_time,
    );

    js::update_inventory(
      JsValue::from_serde(&state.view().inventory_at(state.current_game_time)).unwrap(),
    );
  })
}
