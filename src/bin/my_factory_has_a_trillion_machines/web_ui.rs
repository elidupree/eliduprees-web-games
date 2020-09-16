use super::*;

use eliduprees_web_games::js_unwrap;
use nalgebra::Vector2;
use num::Integer;
use siphasher::sip::SipHasher;
use std::cell::RefCell;
//use std::cmp::{max, min};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use stdweb;

use geometry::{Facing, GridIsomorphism, Number, Rotate90, TransformedBy, Vector, VectorExtension};
use graph_algorithms::{MachineAndInputsFuture, MapFuture, ModuleFutures};
use machine_data::{
  self, //Inputs,
  Game, //MAX_COMPONENTS,
  MachineFuture,
  MachineMomentaryVisuals,
  MachineObservedInputs,
  MachineState,
  MachineType,
  MachineTypeId, //MachineTypeTrait,
  MachineTypeRef,
  MachineTypeTrait,
  MachineTypes,
  Map,
  Material,
  StatefulMachine,
  TIME_TO_MOVE_MATERIAL,
};
use std::collections::VecDeque;
use std::mem;
//use misc;
//use modules::{self, Module};

#[derive(Copy, Clone)]
struct MousePosition {
  tile_center: Vector,
  nearest_lines: Vector,
}

#[derive(Copy, Clone)]
struct QueuedMouseMove {
  x: f64,
  y: f64,
  width: f64,
  height: f64,
}

#[derive(PartialEq, Eq, Clone, Deserialize)]
struct ClickType {
  buttons: u16,
  shift: bool,
  ctrl: bool,
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

js_deserializable!(ClickType);

#[derive(Clone)]
struct DragState {
  original_position: MousePosition,
  click_type: ClickType,
  moved: bool,
}

#[derive(Default)]
struct MouseState {
  drag: Option<DragState>,
  position: Option<MousePosition>,
  previous_position: Option<MousePosition>,
}

struct State {
  game: Game,
  map_future: MapFuture,
  module_futures: ModuleFutures,
  start_ui_time: f64,
  start_game_time: Number,
  current_game_time: Number,
  mouse: MouseState,
  queued_mouse_moves: VecDeque<QueuedMouseMove>,
}

fn machine_presets() -> Vec<MachineType> {
  vec![
    machine_data::conveyor(),
    machine_data::splitter(),
    machine_data::iron_smelter(),
    machine_data::iron_mine(),
    modules::basic_module(),
  ]
}

fn machine_color(machine: &StatefulMachine) -> [f32; 3] {
  let mut hasher = SipHasher::new();
  machine.hash(&mut hasher);
  let hash = hasher.finish();
  let mask = (1u64 << 20) - 1;
  let factor = 0.8 / (mask as f32);
  [
    0.1 + ((hash) & mask) as f32 * factor,
    0.1 + ((hash >> 20) & mask) as f32 * factor,
    0.1 + ((hash >> 40) & mask) as f32 * factor,
  ]
}

fn map_canvas_scale() -> f64 {
  js_unwrap! { return leaflet_map.getZoomScale(leaflet_map.getZoom(), 0) * (window.devicePixelRatio || 1.0); }
}
fn map_css_scale() -> f64 {
  js_unwrap! { return leaflet_map.getZoomScale(leaflet_map.getZoom(), 0); }
}
fn map_center() -> Vector2<f64> {
  let center_x = js_unwrap! { return leaflet_map.getCenter().lng; };
  let center_y = js_unwrap! { return leaflet_map.getCenter().lat; };
  Vector2::new(center_x, center_y)
}
fn canvas_size() -> Vector2<f64> {
  let x = js_unwrap! { return context.canvas.width; };
  let y = js_unwrap! { return context.canvas.height; };
  Vector2::new(x, y)
}
fn canvas_position(position: Vector) -> Vector2<f32> {
  canvas_position_from_f64(position.to_f64())
}
fn canvas_position_from_f64(position: Vector2<f64>) -> Vector2<f32> {
  let scale = map_canvas_scale();
  let center = map_center();
  let relative = (position - center) * scale;
  let canvas_center = canvas_size() * 0.5;
  Vector2::new(
    (canvas_center[0] + relative[0]) as f32,
    (canvas_center[1] - relative[1]) as f32,
  )
}
fn tile_canvas_size() -> Vector2<f32> {
  let scale = map_canvas_scale() * 2.0;
  Vector2::new(scale as f32, scale as f32)
}
fn tile_position(css_position: Vector2<f64>, canvas_css_size: Vector2<f64>) -> MousePosition {
  let world = map_center() + (css_position - canvas_css_size * 0.5) / map_css_scale();
  MousePosition {
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

fn draw_rectangle(
  center: Vector2<f32>,
  size: Vector2<f32>,
  color: [f32; 3],
  sprite: &str,
  facing: Facing,
) {
  //let mut center = center;
  //center[1] = 1.0-center[1];
  let corner = -size / 2.0;
  debug!("{:?}", (center, size));
  js! {
    context.save();
    //context.scale(context.canvas.width, context.canvas.height);
    context.translate (@{center [0]},@{center [1]});
    context.rotate (-Math.PI * @{facing} / 2);

    var sprite = loaded_sprites[@{sprite}];

    context.drawImage (sprite, @{corner[0]},@{corner[1]}, @{size [0]},@{size [1]});
    /*context.globalCompositeOperation = "lighter";
    var r = @{color[0]*255.0};
    var g = @{color[1]*255.0};
    var b = @{color[2]*255.0};
    context.fillStyle = "rgb("+r+","+g+","+b+")";
    context.fillRect (@{corner[0]},@{corner[1]}, @{size [0]},@{size [1]});*/

    context.restore();
  };
  /*sprite_offset.rotate_90((4-facing)%4);*/
}

fn inside_machine(
  machine_types: &MachineTypes,
  position: Vector,
  machine: &StatefulMachine,
) -> bool {
  let machine_type = machine_types.get(machine.type_id);
  let radius = machine_type.radius();
  let offset = machine.state.position.translation - position;
  offset[0].abs() < radius && offset[1].abs() < radius
}

pub fn run_game() {
  let mut game = Game {
    map: Map {
      machines: Vec::new(),
    },
    last_change_time: 0,
    inventory_before_last_change: Default::default(),
    machine_types: MachineTypes {
      presets: machine_presets(),
      modules: Vec::new(),
    },
  };
  game
    .inventory_before_last_change
    .insert(Material::Iron, 1000);
  let output_edges = game.map.output_edges(&game.machine_types);
  let ordering = game
    .map
    .topological_ordering_of_noncyclic_machines(&output_edges);
  let mut module_futures = ModuleFutures::default();
  let map_future = game.map.future(
    &game.machine_types,
    &output_edges,
    &ordering,
    &mut module_futures,
    &[],
  );

  let state = Rc::new(RefCell::new(State {
    game,
    map_future,
    module_futures,
    start_ui_time: now(),
    start_game_time: 0,
    current_game_time: 0,
    mouse: Default::default(),
    queued_mouse_moves: VecDeque::new(),
  }));

  let json_callback = {
    let state = state.clone();
    move |input: String| {
      println!("{}", &input);
      if let Ok(game) = serde_json::from_str::<Game>(&input) {
        let mut state = state.borrow_mut();
        state.start_ui_time = now();
        state.start_game_time = game.last_change_time;
        state.current_game_time = game.last_change_time;
        state.game = game;
        recalculate_future(&mut state);
      }
    }
  };

  js! {
    $("#json").click(function() {$(this).select();})
      .on ("input", function() {@{json_callback}($(this).val())});
  }

  let mousedown_callback = {
    let state = state.clone();
    move |x: f64, y: f64, width: f64, height: f64, click_type: ClickType| {
      mouse_move(
        &mut state.borrow_mut(),
        tile_position(Vector2::new(x, y), Vector2::new(width, height)),
      );
      mouse_down(&mut state.borrow_mut(), click_type);
    }
  };
  let mouseup_callback = {
    let state = state.clone();
    move |x: f64, y: f64, width: f64, height: f64| {
      mouse_move(
        &mut state.borrow_mut(),
        tile_position(Vector2::new(x, y), Vector2::new(width, height)),
      );
      mouse_up(&mut state.borrow_mut());
    }
  };
  let mousemove_callback = {
    let state = state.clone();
    move |x: f64, y: f64, width: f64, height: f64| {
      queue_mouse_move(
        &mut state.borrow_mut(),
        QueuedMouseMove {
          x,
          y,
          width,
          height,
        },
      );
    }
  };

  js! {
    window.mousedown_callback = function(event) {
      var xywh = mouse_coords(event);
      (@{mousedown_callback})(xywh[0],xywh[1],xywh[2],xywh[3], {buttons: event.buttons, shift: event.shiftKey, ctrl: event.ctrlKey});
    };
  }
  js! {
    var dpr = window.devicePixelRatio || 1.0;
    var width = 800;
    var height = 800;
    var physical_width = height*dpr;
    var physical_height = width*dpr;
    $("#canvas").css({width: width+"px", height:height+"px"})
      .attr ("width", physical_width).attr ("height", physical_height);
    leaflet_map.on("mousedown", function(event) { mousedown_callback(event.originalEvent); });
    //window.leaflet_map.on("contextmenu", function(e) {e.preventDefault()});
    $("body")
      .on("mouseup", mouse_callback (@{mouseup_callback}))
      .on("mousemove", mouse_callback (@{mousemove_callback}));
  }

  for name in state
    .borrow()
    .game
    .machine_types
    .presets
    .iter()
    .map(|machine_type| machine_type.as_ref().name().to_owned())
    .chain(vec![])
  {
    let id = format!("Machine_choice_{}", &name);
    js! {
      $("<input>", {type: "radio", id:@{& id}, name: "machine_choice", value: @{&name}, checked:@{name == "Iron mine"}})
        .on("click", function(e) {
          if (@{&name} === "Conveyor") {
            leaflet_map.dragging.disable();
          } else {
            leaflet_map.dragging.enable();
          }
        })
        .appendTo ($("#app"));
      $("<label>", {for:@{& id}, text: @{&name}}).appendTo ($("#app"));
    }
  }

  run(move |_inputs| {
    do_frame(&state);
  });

  stdweb::event_loop();
}

fn current_mode() -> String {
  js_unwrap! { return ($("input:radio[name=machine_choice]:checked").val()); }
}

//as the output of a convenience function, it's intentional that a bunch of this data is redundant
struct ModuleInstancePathNode {
  machine_index_in_parent: usize,
  module_id: MachineTypeId,
  isomorphism: GridIsomorphism,
  start_time: Number,
}
struct ModuleInstancePath {
  nodes: Vec<ModuleInstancePathNode>,
}

fn smallest_module_containing(
  state: &State,
  (position, radius): (Vector, Number),
) -> ModuleInstancePath {
  let mut nodes = Vec::new();
  let mut map = &state.game.map;
  let mut isomorphism = GridIsomorphism::default();
  let mut start_time = 0;
  let mut map_future = Some(&state.map_future);

  'outer: loop {
    for (machine_index, machine) in map.machines.iter().enumerate() {
      if let MachineTypeRef::Module(module) = state.game.machine_types.get(machine.type_id) {
        let machine_isomorphism = machine.state.position * isomorphism;
        let relative_position = position - machine_isomorphism.translation;
        let available_radius = module.module_type.inner_radius - radius;
        if relative_position[0].abs() <= available_radius
          && relative_position[1].abs() <= available_radius
        {
          if let Some((new_map_future, new_start_time)) = map_future.and_then(|map_future| {
            if let MachineAndInputsFuture {
              future: Ok(MachineFuture::Module(module_machine_future)),
              ..
            } = &map_future.machines[machine_index]
            {
              Some((
                &state
                  .module_futures
                  .get(&machine.type_id)
                  .unwrap()
                  .future_variations[&module_machine_future.canonical_inputs],
                start_time + module_machine_future.start_time,
              ))
            } else {
              None
            }
          }) {
            map_future = Some(new_map_future);
            start_time = new_start_time;
          } else {
            map_future = None;
            start_time = state.current_game_time;
          }
          isomorphism = machine_isomorphism;
          map = &module.map;
          nodes.push(ModuleInstancePathNode {
            isomorphism,
            machine_index_in_parent: machine_index,
            module_id: machine.type_id,
            start_time,
          });
          continue 'outer;
        }
      }
    }

    return ModuleInstancePath { nodes };
  }
}

impl ModuleInstancePath {
  fn get_map<'a>(&self, game: &'a Game) -> (&'a Map, GridIsomorphism, Number) {
    match self.nodes.last() {
      None => (&game.map, GridIsomorphism::default(), 0),
      Some(ModuleInstancePathNode {
        module_id,
        isomorphism,
        start_time,
        ..
      }) => (
        &game.machine_types.get_module(*module_id).map,
        *isomorphism,
        *start_time,
      ),
    }
  }

  fn modify_map(
    mut self,
    game: &mut Game,
    modify: impl FnOnce(&mut MachineTypes, &mut Map, GridIsomorphism, Number),
  ) {
    let node = match self.nodes.pop() {
      Some(node) => node,
      None => {
        (modify)(
          &mut game.machine_types,
          &mut game.map,
          GridIsomorphism::default(),
          0,
        );
        return;
      }
    };

    let mut module = game.machine_types.get_module(node.module_id).clone();
    (modify)(
      &mut game.machine_types,
      &mut module.map,
      node.isomorphism,
      node.start_time,
    );
    let mut new_module_index = game.machine_types.modules.len();
    game.machine_types.modules.push(module);

    while let Some(parent_node) = self.nodes.pop() {
      let mut parent_module = game.machine_types.get_module(parent_node.module_id).clone();
      parent_module.map.machines[node.machine_index_in_parent].type_id =
        MachineTypeId::Module(new_module_index);
      let new_parent_module_index = game.machine_types.modules.len();
      game.machine_types.modules.push(parent_module);

      new_module_index = new_parent_module_index;
    }

    game.map.machines[node.machine_index_in_parent].type_id =
      MachineTypeId::Module(new_module_index);
  }
}

fn build_machine(state: &mut State, machine_type_id: MachineTypeId, position: GridIsomorphism) {
  let machine_type = state.game.machine_types.get(machine_type_id);
  let path = smallest_module_containing(state, (position.translation, machine_type.radius()));
  let (map, isomorphism, start_time) = path.get_map(&state.game);

  if map.machines.iter().any(|machine| {
    let radius = state.game.machine_types.get(machine.type_id).radius() + machine_type.radius();
    let offset = (position / (machine.state.position * isomorphism)).translation;
    offset[0].abs() < radius && offset[1].abs() < radius
  }) {
    // can't build – something is in the way
    return;
  }

  let inventory = state
    .game
    .inventory_at(&state.map_future, state.current_game_time);
  for (amount, material) in machine_type.cost() {
    if inventory
      .get(&material)
      .map_or(true, |storage| storage < amount)
    {
      // can't build – you can't afford it
      return;
    }
  }

  let machine_type = state.game.machine_types.get(machine_type_id);
  for (amount, material) in machine_type.cost() {
    *state
      .game
      .inventory_before_last_change
      .get_mut(&material)
      .unwrap() -= amount;
  }
  let inner_now = state.current_game_time - start_time;

  path.modify_map(&mut state.game, |machine_types, map, _, _| {
    map.build_machines(
      machine_types,
      vec![StatefulMachine {
        type_id: machine_type_id,
        state: MachineState {
          position: position / isomorphism,
          last_disturbed_time: inner_now,
        },
      }],
      inner_now,
    );
  });
  recalculate_future(state);
}

fn recalculate_future(state: &mut State) {
  state.game.inventory_before_last_change = state
    .game
    .inventory_at(&state.map_future, state.current_game_time);
  state.game.last_change_time = state.current_game_time;

  state.game.cleanup_modules();

  let output_edges = state.game.map.output_edges(&state.game.machine_types);
  let ordering = state
    .game
    .map
    .topological_ordering_of_noncyclic_machines(&output_edges);
  state.module_futures = ModuleFutures::default();
  state.map_future = state.game.map.future(
    &state.game.machine_types,
    &output_edges,
    &ordering,
    &mut state.module_futures,
    &[],
  );

  /*js!{
    $("#json").val (@{serde_json::to_string_pretty (&state.game).unwrap()});
  }*/
}

fn exact_facing(vector: Vector) -> Option<Facing> {
  match (vector[0].signum(), vector[1].signum()) {
    (1, 0) => Some(0),
    (0, 1) => Some(1),
    (-1, 0) => Some(2),
    (0, -1) => Some(3),
    _ => None,
  }
}

fn hovering_area(state: &State, position: MousePosition) -> (Vector, Number) {
  if let Some(machine_type) = state
    .game
    .machine_types
    .presets
    .iter()
    .find(|machine_type| machine_type.as_ref().name() == current_mode())
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

fn queue_mouse_move(state: &mut State, mouse_move: QueuedMouseMove) {
  if state.queued_mouse_moves.len() >= 100 {
    state.queued_mouse_moves.pop_front();
  }
  state.queued_mouse_moves.push_back(mouse_move);
}
fn mouse_move(state: &mut State, position: MousePosition) {
  let facing = match state.mouse.position {
    None => 0,
    Some(previous_position) => {
      let delta = hovering_area(state, position).0 - hovering_area(state, previous_position).0;
      match exact_facing(delta) {
        Some(facing) => facing,
        _ => loop {
          let difference = hovering_area(state, position).0
            - hovering_area(state, state.mouse.position.unwrap()).0;
          if difference[0] != 0 {
            let offs = Vector::new(difference[0].signum() * 2, 0);
            let mut pos = state.mouse.position.unwrap();
            pos.tile_center += offs;
            pos.nearest_lines += offs;
            mouse_move(state, pos);
          }
          let difference = hovering_area(state, position).0
            - hovering_area(state, state.mouse.position.unwrap()).0;
          if difference[1] != 0 {
            let offs = Vector::new(0, difference[1].signum() * 2);
            let mut pos = state.mouse.position.unwrap();
            pos.tile_center += offs;
            pos.nearest_lines += offs;
            mouse_move(state, pos);
          }
          if hovering_area(state, position).0
            == hovering_area(state, state.mouse.position.unwrap()).0
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
        && (hovering_area(state, previous).0 == hovering_area(state, drag.original_position).0
          || current_mode() == "Conveyor")
      {
        let path = smallest_module_containing(state, (previous.tile_center, 1));
        let (map, isomorphism, start_time) = path.get_map(&state.game);
        let inner_position = previous.tile_center.transformed_by(isomorphism.inverse());
        let inner_now = state.current_game_time - start_time;

        if let Some(rotated_index) = map
          .machines
          .iter()
          .position(|machine| inside_machine(&state.game.machine_types, inner_position, machine))
        {
          path.modify_map(&mut state.game, |machine_types, map, _, _| {
            map.modify_machines(machine_types, vec![rotated_index], inner_now, |machine| {
              machine.state.position.rotation = facing
            });
          });
          recalculate_future(state);
        }
      }
    }
  }
  mouse_maybe_held(state);
}

fn mouse_down(state: &mut State, click_type: ClickType) {
  state.mouse.drag = Some(DragState {
    original_position: state.mouse.position.unwrap(),
    click_type,
    moved: false,
  });
  mouse_maybe_held(state);
}

fn mouse_maybe_held(state: &mut State) {
  let facing = match (state.mouse.previous_position, state.mouse.position) {
    (Some(first), Some(second)) => {
      exact_facing(hovering_area(state, second).0 - hovering_area(state, first).0).unwrap_or(0)
    }
    _ => 0,
  };
  if let Some(drag) = state.mouse.drag.clone() {
    let position = state.mouse.position.unwrap();
    let hover = hovering_area(state, position);
    if drag.click_type == REGULAR_CLICK && current_mode() == "Conveyor" {
      build_machine(
        state,
        MachineTypeId::Preset(
          state
            .game
            .machine_types
            .presets
            .iter()
            .position(|machine_type| machine_type == &machine_data::conveyor())
            .unwrap(),
        ),
        GridIsomorphism {
          translation: hover.0,
          rotation: facing,
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
      let path = smallest_module_containing(state, hover);
      let (map, isomorphism, start_time) = path.get_map(&state.game);
      let inner_position = position.tile_center.transformed_by(isomorphism.inverse());
      let inner_now = state.current_game_time - start_time;

      if let Some(deleted_index) = map
        .machines
        .iter()
        .position(|machine| inside_machine(&state.game.machine_types, inner_position, machine))
      {
        let machine_type = state
          .game
          .machine_types
          .get(map.machines[deleted_index].type_id);
        let cost = machine_type.cost();

        let inventory = &mut state.game.inventory_before_last_change;
        for (amount, material) in cost {
          *inventory.get_mut(&material).unwrap() += amount;
        }

        path.modify_map(&mut state.game, |machine_types, map, _, _| {
          map.remove_machines(machine_types, vec![deleted_index], inner_now);
        });
        recalculate_future(state);
      }
    }
  }
}

fn mouse_up(state: &mut State) {
  if let Some(drag) = state.mouse.drag.clone() {
    if drag.click_type == REGULAR_CLICK && !drag.moved {
      if let Some(preset_index) = state
        .game
        .machine_types
        .presets
        .iter()
        .position(|machine_type| machine_type.as_ref().name() == current_mode())
      {
        build_machine(
          state,
          MachineTypeId::Preset(preset_index),
          GridIsomorphism {
            translation: hovering_area(state, drag.original_position).0,
            ..Default::default()
          },
        );
      }
    }
  }
  state.mouse.drag = None;
}

fn draw_map(
  state: &State,
  map: &Map,
  map_future: &MapFuture,
  isomorphism: GridIsomorphism,
  time: Number,
) {
  for machine in &map.machines {
    let machine_type = state.game.machine_types.get(machine.type_id);
    let radius = machine_type.radius();
    let size = Vector2::new(
      tile_canvas_size()[0] * (radius * 2) as f32 / 2.0,
      tile_canvas_size()[1] * (radius * 2) as f32 / 2.0,
    );
    let machine_isomorphism = machine.state.position * isomorphism;
    draw_rectangle(
      canvas_position(machine_isomorphism.translation),
      size,
      machine_color(machine),
      "rounded-rectangle-transparent",
      0,
    );
    draw_rectangle(
      canvas_position(machine_isomorphism.translation),
      size,
      machine_color(machine),
      machine_type.icon(),
      machine_isomorphism.rotation,
    );
  }
  for machine in &map.machines {
    let machine_type = state.game.machine_types.get(machine.type_id);
    if machine_type.radius() > 1 {
      for (input_location, expected_material) in machine_type
        .input_locations(machine.state.position)
        .into_iter()
        .zip(machine_type.input_materials())
      {
        let pos = canvas_position(
          (input_location.position + Vector::new(1, 0).rotate_90(input_location.facing))
            .transformed_by(isomorphism),
        );
        draw_rectangle(
          pos,
          tile_canvas_size(),
          machine_color(machine),
          "input",
          input_location.facing.transformed_by(isomorphism),
        );
        if let Some(material) = expected_material {
          draw_rectangle(
            pos,
            tile_canvas_size() * 0.8,
            machine_color(machine),
            material.icon(),
            0,
          );
        }
      }
    }
  }
  for machine in &map.machines {
    let machine_type = state.game.machine_types.get(machine.type_id);
    if machine_type.radius() > 1 {
      for output_location in machine_type.output_locations(machine.state.position) {
        draw_rectangle(
          canvas_position(
            (output_location.position - Vector::new(1, 0).rotate_90(output_location.facing))
              .transformed_by(isomorphism),
          ),
          tile_canvas_size(),
          machine_color(machine),
          "input",
          output_location
            .facing
            .rotate_90(2)
            .transformed_by(isomorphism),
        );
      }
    }
  }

  for (machine, machine_future) in map.machines.iter().zip(&map_future.machines) {
    let inputs = MachineObservedInputs {
      input_flows: &machine_future.inputs,
      start_time: machine.state.last_disturbed_time,
    };
    let machine_isomorphism = machine.state.position * isomorphism;
    let visuals = match &machine_future.future {
      Ok(future) => {
        let machine_type = state.game.machine_types.get(machine.type_id);
        match (machine_type, &machine_future.future) {
          (MachineTypeRef::Module(module), Ok(MachineFuture::Module(module_machine_future))) => {
            let variation = state
              .module_futures
              .get(&machine.type_id)
              .unwrap()
              .future_variations
              .get(&module_machine_future.canonical_inputs)
              .unwrap();
            module.module_momentary_visuals(inputs, module_machine_future, time, variation)
          }
          _ => machine_type.momentary_visuals(inputs, future, time),
        }
      }
      Err(operating_state) => MachineMomentaryVisuals {
        operating_state: operating_state.clone(),
        materials: Vec::new(),
      },
    };
    for (location, material) in &visuals.materials {
      draw_rectangle(
        canvas_position_from_f64(location.transformed_by(machine_isomorphism)),
        tile_canvas_size() * 0.6,
        [0.0, 0.0, 0.0],
        material.icon(),
        Facing::default(),
      );
    }
  }

  for (machine, machine_future) in map.machines.iter().zip(&map_future.machines) {
    let machine_type = state.game.machine_types.get(machine.type_id);
    match (machine_type, &machine_future.future) {
      (MachineTypeRef::Module(module), Ok(MachineFuture::Module(module_machine_future))) => {
        let machine_isomorphism = machine.state.position * isomorphism;
        let variation = state
          .module_futures
          .get(&machine.type_id)
          .unwrap()
          .future_variations
          .get(&module_machine_future.canonical_inputs)
          .unwrap();
        draw_map(
          state,
          &module.map,
          &variation,
          machine_isomorphism,
          time - module_machine_future.start_time,
        );
      }
      _ => (),
    }
  }
}

fn do_frame(state: &Rc<RefCell<State>>) {
  if js_unwrap! {return window.loaded_sprites === undefined;} {
    return;
  }

  let mut state = state.borrow_mut();
  let state = &mut *state;

  for QueuedMouseMove {
    x,
    y,
    width,
    height,
  } in mem::take(&mut state.queued_mouse_moves)
  {
    mouse_move(
      state,
      tile_position(Vector2::new(x, y), Vector2::new(width, height)),
    );
  }

  let fractional_time = state.start_game_time as f64
    + (now() - state.start_ui_time) * TIME_TO_MOVE_MATERIAL as f64 * 2.0;
  state.current_game_time = fractional_time as Number;

  js! {
    context.fillStyle = "white";
    context.fillRect(0, 0, context.canvas.width, context.canvas.height);
  }

  //target.clear_color(1.0, 1.0, 1.0, 1.0);

  draw_map(
    state,
    &state.game.map,
    &state.map_future,
    Default::default(),
    state.current_game_time,
  );

  js! { $("#inventory").empty();}
  for (material, amount) in state
    .game
    .inventory_at(&state.map_future, state.current_game_time)
  {
    js! { $("#inventory").append(@{format!("{:?}: {}", material, amount)});}
  }
}
