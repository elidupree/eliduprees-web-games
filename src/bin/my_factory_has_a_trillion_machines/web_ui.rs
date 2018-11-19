use super::*;

use stdweb;
use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::cmp::{min,max};
use std::iter;
use glium::{Surface};
use arrayvec::ArrayVec;
use stdweb::unstable::TryInto;
use stdweb::web::ArrayBuffer;
use siphasher::sip::SipHasher;
use nalgebra::Vector2;



#[derive (Deserialize)]
struct SpriteBounds {
  x: u32,y: u32, width: u32, height: u32,
}
js_deserializable! (SpriteBounds) ;

struct SpriteSheet {
  texture: glium::texture::CompressedSrgbTexture2d,
  size: [u32; 2],
  bounds_map: HashMap <String, SpriteBounds>
}

#[derive (Copy, Clone)]
struct MousePosition {
  tile_center: Vector,
  nearest_lines: Vector,
}

#[derive (Clone)]
struct DragState {
  original_position: MousePosition,
  moved: bool,
}

#[derive (Default)]
struct MouseState {
  drag: Option <DragState>,
  position: Option <MousePosition>,
  previous_position: Option <MousePosition>,
}

struct State {
  glium_display: glium::Display,
  glium_program: glium::Program,
  sprite_sheet: Option <SpriteSheet>,
  map: Map,
  future: MapFuture,
  start_ui_time: f64,
  start_game_time: Number,
  current_game_time: Number,
  mouse: MouseState,
}

#[derive(Copy, Clone)]
struct Vertex {
  position: [f32; 2],
  sprite_coordinates: [f32; 2],
  color: [f32; 3],
}
implement_vertex!(Vertex, position, sprite_coordinates, color);

fn machine_choices()->Vec<MachineType> { vec![conveyor(), splitter(), iron_smelter(), material_generator(), consumer()]}

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

fn draw_rectangle (vertices: &mut Vec<Vertex>, sprite_sheet: & SpriteSheet, center: Vector2<f32>, size: Vector2<f32>, color: [f32; 3], sprite: & str, facing: Facing) {
  let bounds = &sprite_sheet.bounds_map [sprite];
  let sprite_center = Vector2::new(
    bounds.x as f32 + bounds.width  as f32/2.0,
    bounds.y as f32 + bounds.height as f32/2.0,
  );
  let sprite_size = Vector2::new(
    (bounds.width -1) as f32,
    (bounds.height-1) as f32,
  );
    
  let vertex = |x,y| {
    let mut sprite_offset = Vector2::new(x*sprite_size[0], y*sprite_size[1]);
    sprite_offset = sprite_offset.rotate_90((4-facing)%4);
    sprite_offset[1] *= -1.0;
    let sprite_coordinates = sprite_center + sprite_offset;
    Vertex {
      position: [center [0] + size [0]*x, center [1] + size [1]*y],
      sprite_coordinates: [
        sprite_coordinates [0]/sprite_sheet.size [0] as f32,
        sprite_coordinates [1]/sprite_sheet.size [1] as f32,
      ], 
      color,
    }
  };
          vertices.extend(&[
            vertex(-0.5,-0.5),vertex( 0.5,-0.5),vertex( 0.5, 0.5),
            vertex(-0.5,-0.5),vertex( 0.5, 0.5),vertex(-0.5, 0.5)
          ]);
}

impl MousePosition {
  fn overlaps_machine (&self, machine: & StatefulMachine)->bool {
    let radius = machine.machine_type.radius();
    let offset = machine.map_state.position - self.tile_center;
    offset [0].abs() <radius && offset [1].abs() <radius
  }
}

pub fn run_game() {
  let vertex_shader_source = r#"
#version 100
attribute highp vec2 position;
attribute lowp vec3 color;
attribute highp vec2 sprite_coordinates;
varying lowp vec3 color_transfer;
varying highp vec2 sprite_coordinates_transfer;

void main() {
gl_Position = vec4 (position*2.0 - 1.0, 0.0, 1.0);
sprite_coordinates_transfer = sprite_coordinates;
color_transfer = color;
}

"#;

  let fragment_shader_source = r#"
#version 100
varying lowp vec3 color_transfer;
varying highp vec2 sprite_coordinates_transfer;
uniform sampler2D sprite_sheet;

void main() {
lowp vec4 t = texture2D (sprite_sheet, sprite_coordinates_transfer);
gl_FragColor = vec4(color_transfer, t.a);
}

"#;
  let display = glium::Display::new (glium::glutin::WindowBuilder::new()
    .with_dimensions((600, 600).into()), glium::glutin::ContextBuilder::new(), & glium::glutin::EventsLoop::new()
    ).expect("failed to create window");
    
  let program =
    glium::Program::from_source(&display, vertex_shader_source, fragment_shader_source, None)
      .expect("glium program generation failed");
      
  let mut map =Map {machines: ArrayVec::new(), last_change_time: 0, inventory_before_last_change: Default::default()};
  map.inventory_before_last_change.insert (Material::Iron, 100);
  let output_edges = map.output_edges();
  let ordering = map.topological_ordering_of_noncyclic_machines(& output_edges);
  let future = map.future (& output_edges, & ordering);
      
  let state = Rc::new (RefCell::new (State {
    glium_display: display, glium_program: program, sprite_sheet: None,
    map, future, start_ui_time: now(), start_game_time: 0, current_game_time: 0,
    mouse: Default::default(),
  }));
  
  let json_callback = {let state = state.clone(); move | input: String | {
    println!("{}", &input);
    if let Ok (map) = serde_json::from_str::<Map> (& input) {
      let mut state = state.borrow_mut();
      state.start_ui_time = now();
      state.start_game_time = map.last_change_time;
      state.current_game_time = map.last_change_time;
      state.map = map;
      recalculate_future(&mut state);
    }
  }};
  
  js!{
    $("#json").click(function() {$(this).select();})
      .on ("input", function() {@{json_callback}($(this).val())});
  }
  
 
  
  let mousedown_callback = {let state = state.clone(); move |x: f64,y: f64 | {
    mouse_move(&mut state.borrow_mut(), tile_position (Vector2::new (x,y)));
    mouse_down(&mut state.borrow_mut());
  }};
  let mouseup_callback = {let state = state.clone(); move |x: f64,y: f64 | {
    mouse_move(&mut state.borrow_mut(), tile_position (Vector2::new (x,y)));
    mouse_up(&mut state.borrow_mut());
  }};
  let mousemove_callback = {let state = state.clone(); move |x: f64,y: f64 | {
    mouse_move(&mut state.borrow_mut(), tile_position (Vector2::new (x,y)));
  }};
  
  js!{
    function mouse_callback (callback) {
      return function(event) {
        var offset = canvas.getBoundingClientRect();
        var x = (event.clientX - offset.left)/offset.width;
        var y = 1.0 - (event.clientY - offset.top)/offset.height;
        (callback)(x,y);
      }
    }
    var dpr = window.devicePixelRatio || 1.0;
    var width = 800;
    var height = 800;
    var physical_width = height*dpr;
    var physical_height = width*dpr;
    $("#canvas").css({width: width+"px", height:height+"px"})
      .attr ("width", physical_width).attr ("height", physical_height)
      .on("mousedown", mouse_callback (@{mousedown_callback}));
    $("body")
      .on("mouseup", mouse_callback (@{mouseup_callback}))
      .on("mousemove", mouse_callback (@{mousemove_callback}));
  }
  
  for name in machine_choices().iter().map(| machine_type | machine_type.name()).chain (vec!["erase"]) {
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
  let foo = js!{ return $("input:radio[name=machine_choice]:checked").val(); }.try_into().unwrap();
  foo
}

fn build_machine (state: &mut State, machine_type: MachineType, map_state: MachineMapState) {
  let materials_state =MachineMaterialsState::empty (& machine_type, state.current_game_time);
  if state.map.machines.iter().any (| machine | {
    let radius = machine.machine_type.radius() + machine_type.radius();
    let offset = machine.map_state.position - map_state.position;
    offset[0].abs() < radius && offset[1].abs() < radius
  }) {
    return;
  }
  if state.map.machines.len() == state.map.machines.capacity() {
    return;
  }
  let inventory = state.map.inventory_at (& state.future, state.current_game_time);
  for (amount, material) in machine_type.cost() {
    if inventory.get(&material).map_or (true, | storage | *storage <amount) {
      return;
    }
  }
  prepare_to_change_map (state);
  for (amount, material) in machine_type.cost() {
    *state.map.inventory_before_last_change.get_mut(&material).unwrap() -= amount;
  }
  state.map.machines.push (StatefulMachine {
    machine_type,
    map_state,
    materials_state,
  });
  recalculate_future (state) ;
}

fn prepare_to_change_map(state: &mut State) {
  state.map.inventory_before_last_change = state.map.inventory_at (& state.future, state.current_game_time) ;
  state.map.last_change_time = state.current_game_time;
  for (machine, future) in state.map.machines.iter_mut().zip (& state.future.machines) {
    machine.materials_state = machine.machine_type.with_inputs_changed(& future.materials_state_at (state.current_game_time, & machine.materials_state), state.current_game_time, &future.inputs_at(state.current_game_time));
  }
}

fn recalculate_future (state: &mut State) {
  let output_edges = state.map.output_edges();
  let ordering = state.map.topological_ordering_of_noncyclic_machines(& output_edges);
  state.future = state.map.future (& output_edges, & ordering);
     
  js!{
    $("#json").val (@{serde_json::to_string_pretty (&state.map).unwrap()});
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

fn mouse_move (state: &mut State, position: MousePosition) {
  let facing = match state.mouse.position {
    None => 0,
    Some(previous_position) => {
    let delta = position.tile_center - previous_position.tile_center;
     match exact_facing (delta) {
      Some (facing) => facing,
      _=> loop {
        let difference = position.tile_center - state.mouse.position.unwrap().tile_center;
        if difference[0] != 0 {
          let pos = state.mouse.position.unwrap().tile_center + Vector::new (difference[0].signum()*2, 0);
          mouse_move (state, MousePosition {tile_center: pos, nearest_lines: pos});
        }
        let difference = position.tile_center - state.mouse.position.unwrap().tile_center;
        if difference[1] != 0 {
          let pos = state.mouse.position.unwrap().tile_center + Vector::new (0, difference[1].signum()*2);
          mouse_move (state, MousePosition {tile_center: pos, nearest_lines: pos});
        }
        if position.tile_center == state.mouse.position.unwrap().tile_center {
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
      if previous.tile_center == drag.original_position.tile_center || current_mode() == "Conveyor" {
        if let Some(index) = state.map.machines.iter().position(|machine| previous.overlaps_machine (machine)) {
          prepare_to_change_map (state) ;
          state.map.machines[index].map_state.facing = facing;
          recalculate_future (state) ;
        }
      }
    }
  }
  mouse_maybe_held(state);
}

fn mouse_down(state: &mut State) {
  state.mouse.drag = Some (DragState {
    original_position: state.mouse.position.unwrap(),
    moved: false,
  });
  mouse_maybe_held(state);
}

fn mouse_maybe_held(state: &mut State) {
  let facing = match (state.mouse.previous_position, state.mouse.position) {
    (Some (first), Some (second)) => exact_facing (second.tile_center - first.tile_center).unwrap_or (0),
    _=> 0,
  };
  if let Some(_drag) = state.mouse.drag.clone() {
    let position = state.mouse.position.unwrap();
    if current_mode() == "Conveyor" {
      build_machine (state, conveyor(), MachineMapState {position: position.tile_center, facing});
    }
    if current_mode() == "erase" {
      if let Some(index) = state.map.machines.iter().position(|machine| position.overlaps_machine (machine)) {
        prepare_to_change_map (state) ;
        state.map.machines.remove(index);
        recalculate_future (state) ;
      }
    }
  }
}

fn mouse_up(state: &mut State) {
  if let Some(drag) = state.mouse.drag.clone() {
    if !drag.moved {
      if let Some(machine_type) = machine_choices().into_iter().find (| machine_type | machine_type.name() == current_mode()) {
        build_machine (state, machine_type, MachineMapState {position: drag.original_position.tile_center, facing: 0});
      }
    }
  }
  state.mouse.drag = None;
}

fn do_frame(state: & Rc<RefCell<State>>) {
  
  
  if state.borrow().sprite_sheet.is_none() {
    let state = state.clone();
    let load = move | data: ArrayBuffer, width: u32, height: u32, bounds_map: HashMap <String, SpriteBounds> | {
      
    
      let mut state = state.borrow_mut();
      let state = &mut*state;
      
      let data: Vec<u8> = data.into();
      let image = glium::texture::RawImage2d::from_raw_rgba (data, (width, height));
      
      state.sprite_sheet = Some (SpriteSheet {
        texture: glium::texture::CompressedSrgbTexture2d::new (& state.glium_display, image).unwrap(),
        size: [width, height],
        bounds_map
      });
    };
    js!{
      if (window.loaded_sprites) {
        @{load} (loaded_sprites.rgba.buffer, loaded_sprites.width, loaded_sprites.height, loaded_sprites.coords);
      }
    }
  }
  
  let mut state = state.borrow_mut();
  let state = &mut *state;
  let fractional_time = state.start_game_time as f64 + (now() - state.start_ui_time)*TIME_TO_MOVE_MATERIAL as f64*2.0;
  state.current_game_time = fractional_time as Number;
  //state.map.update_to (& state.future, state.current_game_time);
  
  let sprite_sheet = match state.sprite_sheet {Some (ref value) => value, None => return};
  
  let parameters = glium::DrawParameters {
    blend: glium::draw_parameters::Blend::alpha_blending(),
    ..Default::default()
  };
  let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let mut target = state.glium_display.draw();
    target.clear_color(1.0, 1.0, 1.0, 1.0);
    let mut vertices = Vec::<Vertex>::new();
    
    for machine in & state.map.machines {
      let drawn = machine.machine_type.drawn_machine(& machine.map_state);
      draw_rectangle (&mut vertices, sprite_sheet,
        canvas_position (machine.map_state.position),
        Vector2::new(tile_size()[0] * drawn.size[0] as f32/2.0, tile_size()[1] * drawn.size[1] as f32/2.0),
        machine_color (machine), &drawn.icon, drawn.facing
      );
    }
    /*for machine in & state.map.machines {
      for (input_location, input_facing) in machine.machine_type.input_locations (& machine.map_state) {
        draw_rectangle (&mut vertices, sprite_sheet,
          canvas_position (input_location),
          tile_size()* 0.8,
          machine_color (machine)
        );
      }
    }
    for machine in & state.map.machines {
      for (output_location, output_facing) in machine.machine_type.output_locations (& machine.map_state) {
        draw_rectangle (&mut vertices, sprite_sheet,
          canvas_position (output_location),
          tile_size()* 0.6,
          machine_color (machine)
        );
      }
    }*/
    for (machine, future) in state.map.machines.iter().zip (&state.future.machines) {
      let materials_states = iter::once (&machine.materials_state).chain (future.changes.iter().map (| (time, state) | {
        assert!(*time == state.last_flow_change);
        state
      }));
      let mut future_output_patterns: Inputs <Vec<_>> = (0..machine.machine_type.num_outputs()).map(|_| Vec::new()).collect();
      for (materials, next) in with_optional_next (materials_states) {
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
            draw_rectangle (&mut vertices, sprite_sheet,
              canvas_position (output_location) + offset,
              tile_size()*0.6,
              [0.0,0.0,0.0], pattern_material.icon(), Facing::default()
            );
          }
        }
      }
    }
    for (machine, future) in state.map.machines.iter().zip (&state.future.machines) {
      for (storage_location, (storage_amount, storage_material)) in machine.machine_type.displayed_storage (& machine.map_state, & future.materials_state_at(state.current_game_time, & machine.materials_state),& future.inputs_at (state.current_game_time), state.current_game_time) {
        for index in 0..min (3, storage_amount) {
          let mut position = canvas_position (storage_location);
          position [1] += tile_size() [1]*index as f32*0.1;
          draw_rectangle (&mut vertices, sprite_sheet,
            position,
            tile_size()*0.6,
            [0.0,0.0,0.0], storage_material.icon(), Facing::default()
          );
        }
      }
    }
    js!{ $("#inventory").empty();}
    for (material, amount) in state.map.inventory_at (& state.future, state.current_game_time) {
      js!{ $("#inventory").append(@{format!("{:?}: {}", material, amount)});}
    }


    target.draw(&glium::VertexBuffer::new(& state.glium_display, &vertices)
                .expect("failed to generate glium Vertex buffer"),
              &indices,
              & state.glium_program,
              & uniform! {sprite_sheet: & sprite_sheet.texture},
              &parameters)
        .expect("failed target.draw");

    target.finish().expect("failed to finish drawing");
}
  