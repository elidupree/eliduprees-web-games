use super::*;

use stdweb;
use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::cmp::{min,max};
use std::iter;
use arrayvec::ArrayVec;
use stdweb::unstable::TryInto;
use stdweb::web::ArrayBuffer;
use siphasher::sip::SipHasher;
use nalgebra::Vector2;
use num::Integer;

use geometry::{Number, Vector, Facing, GridIsomorphism, Rotate90, TransformedBy};
use machine_data::{self, Inputs, Material, MachineType, MachineTypeTrait, MachineState, StatefulMachine, Map, Game, MAX_COMPONENTS, TIME_TO_MOVE_MATERIAL};
use graph_algorithms::MapFuture;
use misc;
//use modules::{self, Module};


#[derive (Copy, Clone)]
struct MousePosition {
  tile_center: Vector,
  nearest_lines: Vector,
}

#[derive (PartialEq, Eq, Clone, Deserialize)]
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
  fn default ()->Self {REGULAR_CLICK}
}

js_deserializable! (ClickType);

#[derive (Clone)]
struct DragState {
  original_position: MousePosition,
  click_type: ClickType,
  moved: bool,
}

#[derive (Default)]
struct MouseState {
  drag: Option <DragState>,
  position: Option <MousePosition>,
  previous_position: Option <MousePosition>,
}

struct State {
  game: Game,
  future: MapFuture,
  start_ui_time: f64,
  start_game_time: Number,
  current_game_time: Number,
  mouse: MouseState,
}

fn machine_choices()->Vec<MachineType> { vec![machine_data::conveyor(), machine_data::splitter(), machine_data::iron_smelter(), machine_data::iron_mine(), //modules::basic_module()
]}

fn machine_color(machine: & StatefulMachine)->[f32; 3] {
  let mut hasher = SipHasher::new() ;
      machine.hash (&mut hasher);
      let hash = hasher.finish();
      let mask = (1u64 << 20)-1;
      let factor = 0.8 / (mask as f32);
      [
        0.1+ ((hash      ) & mask) as f32*factor,
        0.1+ ((hash >> 20) & mask) as f32*factor,
        0.1+ ((hash >> 40) & mask) as f32*factor,
      ]
}

fn canvas_position (position: Vector)->Vector2 <f32> {
  Vector2::new (position [0] as f32/60.0, position [1] as f32/60.0)
}
fn tile_size()->Vector2 <f32> {
  Vector2::new (1.0/30.0, 1.0/30.0)
}
fn tile_position (visual: Vector2 <f64>)->MousePosition {
  MousePosition {
    tile_center: Vector::new (
      (visual [0]*30.0).floor() as Number * 2 + 1,
      (visual [1]*30.0).floor() as Number * 2 + 1,
    ),
    nearest_lines: Vector::new (
      (visual [0]*30.0).round() as Number * 2,
      (visual [1]*30.0).round() as Number * 2,
    ),
  }
}

fn draw_rectangle (center: Vector2<f32>, size: Vector2<f32>, color: [f32; 3], sprite: & str, facing: Facing) {
  let mut center = center;
  center[1] = 1.0-center[1];
  let corner = -size / 2.0;
  js! {
    context.save();
    context.scale(context.canvas.width, context.canvas.height);
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

impl MousePosition {
  fn overlaps_machine (&self, machine: & StatefulMachine)->bool {
    let radius = machine.machine_type.radius();
    let offset = machine.state.position.translation - self.tile_center;
    offset [0].abs() <radius && offset [1].abs() <radius
  }
}

fn inside_machine (position: Vector, machine: & StatefulMachine)->bool {
  let radius = machine.machine_type.radius();
  let offset = machine.state.position.translation - position;
  offset [0].abs() <radius && offset [1].abs() <radius
}

pub fn run_game() {

      
  let mut game=Game{map:Map {machines: Vec::new(), },last_change_time: 0, inventory_before_last_change: Default::default()};
  game.inventory_before_last_change.insert (Material::Iron, 100);
  let output_edges = game.map.output_edges();
  let ordering = game.map.topological_ordering_of_noncyclic_machines(& output_edges);
  let future = game.map.future (& output_edges, & ordering);
      
  let state = Rc::new (RefCell::new (State {
    game, future, start_ui_time: now(), start_game_time: 0, current_game_time: 0,
    mouse: Default::default(),
  }));
  
  let json_callback = {let state = state.clone(); move | input: String | {
    println!("{}", &input);
    if let Ok (game) = serde_json::from_str::<Game> (& input) {
      let mut state = state.borrow_mut();
      state.start_ui_time = now();
      state.start_game_time = game.last_change_time;
      state.current_game_time = game.last_change_time;
      state.game = game;
      recalculate_future(&mut state);
    }
  }};
  
  js!{
    $("#json").click(function() {$(this).select();})
      .on ("input", function() {@{json_callback}($(this).val())});
  }
  
 
  
  let mousedown_callback = {let state = state.clone(); move |x: f64,y: f64, click_type: ClickType | {
    mouse_move(&mut state.borrow_mut(), tile_position (Vector2::new (x,y)));
    mouse_down(&mut state.borrow_mut(), click_type);
  }};
  let mouseup_callback = {let state = state.clone(); move |x: f64,y: f64 | {
    mouse_move(&mut state.borrow_mut(), tile_position (Vector2::new (x,y)));
    mouse_up(&mut state.borrow_mut());
  }};
  let mousemove_callback = {let state = state.clone(); move |x: f64,y: f64 | {
    mouse_move(&mut state.borrow_mut(), tile_position (Vector2::new (x,y)));
  }};
  
  js!{
    window.mouse_coords = function (event) {
      var offset = canvas.getBoundingClientRect();
      var x = (event.clientX - offset.left)/offset.width;
      var y = 1.0 - (event.clientY - offset.top)/offset.height;
      return [x,y];
    };
    window.mouse_callback = function (callback) {
      return function(event) {
        var xy = mouse_coords(event);
        (callback)(xy[0],xy[1]);
      }
    };
    window.mousedown_callback = function(event) {
      var xy = mouse_coords(event);
      (@{mousedown_callback})(xy[0],xy[1], {buttons: event.buttons, shift: event.shiftKey, ctrl: event.ctrlKey});
    };
  }
  js!{
    var dpr = window.devicePixelRatio || 1.0;
    var width = 800;
    var height = 800;
    var physical_width = height*dpr;
    var physical_height = width*dpr;
    $("#canvas").css({width: width+"px", height:height+"px"})
      .attr ("width", physical_width).attr ("height", physical_height)
      .on("mousedown", mousedown_callback)
      .on("contextmenu", function(e) {e.preventDefault()});
    $("body")
      .on("mouseup", mouse_callback (@{mouseup_callback}))
      .on("mousemove", mouse_callback (@{mousemove_callback}));
  }
  
  for name in machine_choices().iter().map(| machine_type | machine_type.name()).chain (vec![]) {
    let id = format! ("Machine_choice_{}", name);
    js!{
      $("<input>", {type: "radio", id:@{& id}, name: "machine_choice", value: @{name}, checked:@{name == "Iron mine"}}).appendTo ($("#app"));
      $("<label>", {for:@{& id}, text: @{name}}).appendTo ($("#app"));
    }
  }
  
  run(move |_inputs| {
    do_frame (& state);
  });
  
  stdweb::event_loop();
}

fn current_mode ()->String {
  let foo = js!{ return ($("input:radio[name=machine_choice]:checked").val()); }.try_into().unwrap();
  foo
}

// TODO reduce duplicate code id 394342002
fn in_smallest_module<F: FnOnce(GridIsomorphism, &[StatefulMachine])->R, R> (machines: &[StatefulMachine], isomorphism: GridIsomorphism, (position, radius): (Vector, Number), callback: F)->R {
  for machine in machines.iter() {
    /*if let MachineType::ModuleMachine(module_machine) = &machine.machine_type {
      let machine_isomorphism = machine.state.position*isomorphism;
      let relative_position = position - machine_isomorphism.translation;
      let available_radius = module_machine.module.module_type.inner_radius - radius;
      if relative_position[0].abs() <= available_radius && relative_position[1].abs() <= available_radius {
        return in_smallest_module(&module_machine.module.map.machines, machine_isomorphism, (position, radius), callback);
      }
    }*/
  }
  callback (isomorphism, machines)
}
// TODO reduce duplicate code id 394342002
fn edit_in_smallest_module<F: FnOnce(GridIsomorphism, &mut Vec<StatefulMachine>)->R, R> (machines: &mut Vec <StatefulMachine>, isomorphism: GridIsomorphism, (position, radius): (Vector, Number), callback: F)->R {
  for machine in machines.iter_mut() {
    /*if let MachineType::ModuleMachine(module_machine) = &mut machine.machine_type {
      let machine_isomorphism = machine.state.position*isomorphism;
      let relative_position = position - machine_isomorphism.translation;
      let available_radius = module_machine.module.module_type.inner_radius - radius;
      if relative_position[0].abs() <= available_radius && relative_position[1].abs() <= available_radius {
        let mut edited: Module = (*module_machine.module).clone();
        let result = edit_in_smallest_module(&mut edited.map.machines, machine_isomorphism, (position, radius), callback);
        module_machine.module = Rc::new(edited);
        return result;
      }
    }*/
  }
  callback (isomorphism, machines)
}

fn build_machine (state: &mut State, machine_type: MachineType, position: GridIsomorphism) {
  let machine_state =MachineState {position, last_disturbed_time: state.current_game_time};
  if in_smallest_module (&state.game.map.machines, Default::default(), (machine_state.position.translation, machine_type.radius()), | isomorphism, machines| {
    if machines.iter().any (| machine | {
      let radius = machine.machine_type.radius() + machine_type.radius();
      let offset = (machine_state.position / (isomorphism*machine.state.position)).translation;
      offset[0].abs() < radius && offset[1].abs() < radius
    }) {
      return true;
    }
    /*if machines.len() == machines.capacity() {
      return true;
    }*/
    false
  }) {
    return;
  }
  let inventory = state.game.inventory_at (& state.future, state.current_game_time);
  for (amount, material) in machine_type.cost() {
    if inventory.get(&material).map_or (true, | storage | storage <amount) {
      return;
    }
  }
  prepare_to_change_map (state);
  for (amount, material) in machine_type.cost() {
    *state.game.inventory_before_last_change.get_mut(&material).unwrap() -= amount;
  }
  edit_in_smallest_module(&mut state.game.map.machines, Default::default(), (machine_state.position.translation, machine_type.radius()), | isomorphism, machines| machines.push (StatefulMachine {
    machine_type,
    state: MachineState{position: machine_state.position/isomorphism, .. machine_state},
  }));
  recalculate_future (state) ;
}

fn prepare_to_change_map(state: &mut State) {
  state.game.inventory_before_last_change = state.game.inventory_at (& state.future, state.current_game_time) ;
  state.game.last_change_time = state.current_game_time;
  for (machine, future) in state.game.map.machines.iter_mut().zip (& state.future.machines) {
    // TODO: only the downstream machines
    machine.state.last_disturbed_time = state.current_game_time;
  }
}

fn recalculate_future (state: &mut State) {
  let output_edges = state.game.map.output_edges();
  let ordering = state.game.map.topological_ordering_of_noncyclic_machines(& output_edges);
  state.future = state.game.map.future (& output_edges, & ordering);
     
  js!{
    $("#json").val (@{serde_json::to_string_pretty (&state.game).unwrap()});
  }
}

fn exact_facing (vector: Vector)->Option <Facing> {
  match (vector[0].signum(), vector[1].signum()) {
      (1, 0) => Some(0),
      (0, 1) => Some(1),
      (-1, 0) => Some(2),
      (0, -1) => Some(3),
      _=>None,
  }
}

fn hovering_area (position: MousePosition)->(Vector, Number) {
  if let Some(machine_type) = machine_choices().into_iter().find (| machine_type | machine_type.name() == current_mode()) {
    (
      if machine_type.radius().is_even() { position.nearest_lines } else {position.tile_center},
      machine_type.radius(),
    )
  }
  else {
    (position.tile_center, 1)
  }
}

fn mouse_move (state: &mut State, position: MousePosition) {
  let facing = match state.mouse.position {
    None => 0,
    Some(previous_position) => {
    let delta = hovering_area (position).0 - hovering_area (previous_position).0;
     match exact_facing (delta) {
      Some (facing) => facing,
      _=> loop {
        let difference = hovering_area (position).0 - hovering_area (state.mouse.position.unwrap()).0;
        if difference[0] != 0 {
          let offs = Vector::new (difference[0].signum()*2, 0);
          let mut pos = state.mouse.position.unwrap();
          pos.tile_center += offs;
          pos.nearest_lines += offs;
          mouse_move (state, pos);
        }
        let difference = hovering_area (position).0 - hovering_area (state.mouse.position.unwrap()).0;
        if difference[1] != 0 {
          let offs = Vector::new (0, difference[1].signum()*2);
          let mut pos = state.mouse.position.unwrap();
          pos.tile_center += offs;
          pos.nearest_lines += offs;
          mouse_move (state, pos);
        }
        if hovering_area (position).0 == hovering_area (state.mouse.position.unwrap()).0 {
          return;
        }
      },
     }
    },
  };
  
  state.mouse.previous_position = state.mouse.position;
  state.mouse.position = Some(position);
  
  if let Some(ref mut drag) = state.mouse.drag {
    drag.moved = true;
    if let Some(previous) = state.mouse.previous_position {
      if drag.click_type == REGULAR_CLICK && (hovering_area (previous).0 == hovering_area (drag.original_position).0 || current_mode() == "Conveyor") {
        if let Some(index) = state.game.map.machines.iter().position(|machine| previous.overlaps_machine (machine)) {
          prepare_to_change_map (state) ;
          state.game.map.machines[index].state.position.rotation = facing;
          recalculate_future (state) ;
        }
      }
    }
  }
  mouse_maybe_held(state);
}

fn mouse_down(state: &mut State, click_type: ClickType) {
  state.mouse.drag = Some (DragState {
    original_position: state.mouse.position.unwrap(),
    click_type,
    moved: false,
  });
  mouse_maybe_held(state);
}

fn mouse_maybe_held(state: &mut State) {
  let facing = match (state.mouse.previous_position, state.mouse.position) {
    (Some (first), Some (second)) => exact_facing (hovering_area (second).0 - hovering_area (first).0).unwrap_or (0),
    _=> 0,
  };
  if let Some(drag) = state.mouse.drag.clone() {
    let position = state.mouse.position.unwrap();
    if drag.click_type == REGULAR_CLICK && current_mode() == "Conveyor" {
      build_machine (state, machine_data::conveyor(), GridIsomorphism { translation: hovering_area (position).0, rotation: facing, ..Default::default()});
    }
    
    if drag.click_type == (ClickType {buttons: 2,..Default::default()}) {
      if in_smallest_module (&state.game.map.machines, Default::default(), hovering_area(position), | isomorphism, machines| machines.iter().position(|machine| inside_machine (position.tile_center.transformed_by(isomorphism.inverse()), machine)).is_some()) {
        prepare_to_change_map (state);
        let cost = edit_in_smallest_module (&mut state.game.map.machines, Default::default(), hovering_area(position), | isomorphism, machines| {
          let index = machines.iter().position(|machine| inside_machine (position.tile_center.transformed_by(isomorphism.inverse()), machine)).unwrap();
          let cost = machines[index].machine_type.cost().to_owned();
          machines.remove(index);
          cost
        });
        for (amount, material) in cost {
          *state.game.inventory_before_last_change.get_mut(&material).unwrap() += amount;
        }
        recalculate_future (state) ;
      }
    }
  }
}

fn mouse_up(state: &mut State) {
  if let Some(drag) = state.mouse.drag.clone() {
    if drag.click_type == REGULAR_CLICK && !drag.moved {
      if let Some(machine_type) = machine_choices().into_iter().find (| machine_type | machine_type.name() == current_mode()) {
        build_machine (state, machine_type, GridIsomorphism { translation: hovering_area (drag.original_position).0, ..Default::default()});
      }
    }
  }
  state.mouse.drag = None;
}




fn draw_machines (state: & State, machines: & [StatefulMachine], isomorphism: GridIsomorphism) {
    for machine in machines {
      let drawn = machine.machine_type.drawn_machine(& machine.map_state);
      let size = Vector2::new(tile_size()[0] * drawn.size[0] as f32/2.0, tile_size()[1] * drawn.size[1] as f32/2.0);
      let machine_isomorphism = machine.state.position*isomorphism;
      draw_rectangle (
        canvas_position (machine_isomorphism.translation),
        size,
        machine_color (machine), "rounded-rectangle-transparent", drawn.position.rotation
      );
      draw_rectangle (
        canvas_position (machine_isomorphism.translation),
        size,
        machine_color (machine), &drawn.icon, drawn.position.rotation.transformed_by(isomorphism)
      );
    }
    for machine in machines {
      if machine.machine_type.radius() > 1 {
        for (input_location, expected_material) in machine.input_locations ().into_iter().zip (machine.machine_type. input_materials()) {
          draw_rectangle (
            canvas_position (input_location.position.transformed_by(isomorphism)),
            tile_size(),
            machine_color (machine), "input", input_location.facing.transformed_by(isomorphism)
          );
          if let Some(material) = expected_material {
            draw_rectangle (
              canvas_position (input_location.position.transformed_by(isomorphism)),
              tile_size()*0.8,
              machine_color (machine), material.icon(), 0
            );
          }
        }
      }
    }
    for machine in machines {
      if machine.machine_type.radius() > 1 {
        for output_location in machine.output_locations () {
          draw_rectangle (
            canvas_position ((output_location.position - Vector::new(2, 0).rotate_90(output_location.facing)).transformed_by(isomorphism)),
            tile_size(),
            machine_color (machine), "input", output_location.facing.rotate_90(2).transformed_by(isomorphism)
          );
        }
      }
    }
    
    for machine in machines {
      /*if let MachineType::ModuleMachine(module_machine) = &machine.machine_type {
        let machine_isomorphism = machine.state.position*isomorphism;
        draw_machines (state, &module_machine.module.map.machines, machine_isomorphism);
      }*/
    }
}

fn do_frame(state: & Rc<RefCell<State>>) {  
  let mut state = state.borrow_mut();
  let state = &mut *state;
  let fractional_time = state.start_game_time as f64 + (now() - state.start_ui_time)*TIME_TO_MOVE_MATERIAL as f64*2.0;
  state.current_game_time = fractional_time as Number;
  
  if (js! {return window.loaded_sprites === undefined;}.try_into().unwrap()) {
    return;
  }
  
  js! {
    context.fillStyle = "white";
    context.fillRect(0, 0, context.canvas.width, context.canvas.height);
  }
  
    //target.clear_color(1.0, 1.0, 1.0, 1.0);
    
    draw_machines (state, &state.game.map.machines, Default::default());
    for (machine, future) in state.game.map.machines.iter().zip (&state.future.machines) {
      let materials_states = iter::once (&machine.materials_state).chain (future.changes.iter().map (| (time, state) | {
        assert!(*time == state.last_flow_change);
        state
      }));
      let mut future_output_patterns: Inputs <Vec<_>> = (0..machine.machine_type.num_outputs()).map(|_| Vec::new()).collect();
      for (materials, next) in misc::with_optional_next (materials_states) {
        //if state.mouse_pressed {println!(" {:?} ", (&materials, &next));}
        let end_time = match next {None => Number::max_value(), Some (state) => state.last_flow_change};
        assert!(end_time >= materials.last_flow_change, "{:?} > {:?}", materials, next) ;
        if end_time == materials.last_flow_change {continue;}
        assert_eq!(materials, &future.materials_state_at(materials.last_flow_change, & machine.materials_state));
        assert_eq!(materials, &future.materials_state_at(end_time-1, & machine.materials_state));
        for (collector, patterns) in future_output_patterns.iter_mut().zip (machine.machine_type.future_output_patterns (& materials, & future.inputs_at (materials.last_flow_change))) {
          for (time, pattern) in patterns {
            if time < end_time {
              collector.push ((time, pattern)) ;
            }
          }
        }
      }
      //if state.mouse_pressed {println!(" {:?} ", future_output_patterns);}
      //let relevant_output_patterns = future_output_patterns.into_iter().map (| list | list.into_iter().rev().find(|(time,_pattern)| *time <= state.current_game_time).unwrap().1).collect();
      let start_time = state.current_game_time;
      let end_time = start_time + TIME_TO_MOVE_MATERIAL;
      for ((output_location, output_facing), patterns) in machine.machine_type.output_locations (& machine.map_state).into_iter().zip (future_output_patterns) {
        for (pattern_index, (pattern_start_time, (pattern, pattern_material))) in patterns.iter().enumerate() {
          let pattern_end_time = patterns.get (pattern_index + 1).map_or_else (Number::max_value, | (time,_pattern) | *time) ;
          if *pattern_start_time >= end_time || pattern_end_time <= start_time { continue; }
          
          let soon_disbursements = pattern.num_disbursed_between ([max (*pattern_start_time, start_time), min(pattern_end_time, end_time)]);
          if soon_disbursements > 1 {
            eprintln!(" Warning: things released more frequently than permitted {:?} ", soon_disbursements);
          }
          if soon_disbursements > 0 {
            let time = pattern.last_disbursement_before (state.current_game_time + TIME_TO_MOVE_MATERIAL).unwrap();
            assert!(time >= start_time) ;
            let progress = ((TIME_TO_MOVE_MATERIAL - 1 - (time - start_time)) as f32 + fractional_time.fract() as f32) / TIME_TO_MOVE_MATERIAL as f32;
            let offset = if let Some(output_facing) = output_facing {
              Vector2::new (tile_size() [0]*(progress - 1.0), 0.0).rotate_90 (output_facing)
            }
            else {
              Vector2::new (tile_size() [0]*progress*1.2, -tile_size() [1]*progress*1.6)
            };
            draw_rectangle (
              canvas_position (output_location) + offset,
              tile_size()*0.6,
              [0.0,0.0,0.0], pattern_material.icon(), Facing::default()
            );
          }
        }
      }
    }
    for (machine, future) in state.game.map.machines.iter().zip (&state.future.machines) {
      for (storage_location, (storage_amount, storage_material)) in machine.machine_type.displayed_storage (& machine.map_state, & future.materials_state_at(state.current_game_time, & machine.materials_state),& future.inputs_at (state.current_game_time), state.current_game_time) {
        for index in 0..min (3, storage_amount) {
          let mut position = canvas_position (storage_location);
          position [1] += tile_size() [1]*index as f32*0.1;
          draw_rectangle (
            position,
            tile_size()*0.6,
            [0.0,0.0,0.0], storage_material.icon(), Facing::default()
          );
        }
      }
    }
    js!{ $("#inventory").empty();}
    for (material, amount) in state.game.inventory_at (& state.future, state.current_game_time) {
      js!{ $("#inventory").append(@{format!("{:?}: {}", material, amount)});}
    }
}
  