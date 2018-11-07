use super::*;

use stdweb;
use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::cmp::{min,max};
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

struct State {
  glium_display: glium::Display,
  glium_program: glium::Program,
  sprite_sheet: Option <SpriteSheet>,
  map: Map,
  future: MachinesFuture,
  start_ui_time: f64,
  current_game_time: Number,
  mouse_position: Vector,
  mouse_facing: Facing,
  mouse_pressed: bool,
}

#[derive(Copy, Clone)]
struct Vertex {
  position: [f32; 2],
  sprite_position: [f32; 2],
  sprite_size: [f32; 2],
  sprite_coordinates: [f32; 2],
  color: [f32; 3],
}
implement_vertex!(Vertex, position, sprite_position, sprite_size, sprite_coordinates, color);

fn machine_choices()->Vec<MachineType> { vec![conveyor(), splitter(), slow_machine(), material_generator(), consumer()]}

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

fn tile_center (position: Vector)->Vector2 <f32> {
  Vector2::new (position [0] as f32/30.0, position [1] as f32/30.0)
}
fn tile_size()->Vector2 <f32> {
  Vector2::new (1.0/30.0, 1.0/30.0)
}
fn tile_position (visual: Vector2 <f64>)->Vector {
  Vector::new ((visual [0]*30.0).round() as Number, (visual [1]*30.0).round() as Number)
}

fn draw_rectangle (vertices: &mut Vec<Vertex>, sprite_sheet: & SpriteSheet, center: Vector2<f32>, size: Vector2<f32>, color: [f32; 3], sprite: & str) {
  let bounds = &sprite_sheet.bounds_map [sprite];
  let sprite_size = [
    (bounds.width-1) as f32/sprite_sheet.size [0] as f32,
    -((bounds.height-1) as f32/sprite_sheet.size [1] as f32),
  ];
  let sprite_position = [
    (bounds.x as f32 + 0.5)/sprite_sheet.size [0] as f32,
    ((bounds.y + bounds.height) as f32 - 0.5)/sprite_sheet.size [1] as f32,
  ];
  let vertex = |x,y| Vertex {
            position: [center [0] + size [0]*x, center [1] + size [1]*y],
            sprite_position,
            sprite_size,
            sprite_coordinates: [(x+0.5), (y+0.5)], 
            color,
          };
          vertices.extend(&[
            vertex(-0.5,-0.5),vertex( 0.5,-0.5),vertex( 0.5, 0.5),
            vertex(-0.5,-0.5),vertex( 0.5, 0.5),vertex(-0.5, 0.5)
          ]);
}

pub fn run_game() {
  let vertex_shader_source = r#"
#version 100
attribute highp vec2 position;
attribute lowp vec3 color;
attribute highp vec2 sprite_position;
attribute highp vec2 sprite_size;
attribute highp vec2 sprite_coordinates;
varying lowp vec3 color_transfer;
varying highp vec2 sprite_position_transfer;
varying highp vec2 sprite_size_transfer;
varying highp vec2 sprite_coordinates_transfer;

void main() {
gl_Position = vec4 (position*2.0 - 1.0, 0.0, 1.0);
sprite_position_transfer = sprite_position;
sprite_size_transfer = sprite_size;
sprite_coordinates_transfer = sprite_coordinates;
color_transfer = color;
}

"#;

  let fragment_shader_source = r#"
#version 100
varying lowp vec3 color_transfer;
varying highp vec2 sprite_position_transfer;
varying highp vec2 sprite_size_transfer;
varying highp vec2 sprite_coordinates_transfer;
uniform sampler2D sprite_sheet;

void main() {
lowp vec4 t = texture2D (sprite_sheet, sprite_position_transfer + sprite_size_transfer*sprite_coordinates_transfer);
gl_FragColor = vec4(color_transfer, t.a);
}

"#;
  let display = glium::Display::new (glium::glutin::WindowBuilder::new()
    .with_dimensions((600, 600).into()), glium::glutin::ContextBuilder::new(), & glium::glutin::EventsLoop::new()
    ).expect("failed to create window");
    
  let program =
    glium::Program::from_source(&display, vertex_shader_source, fragment_shader_source, None)
      .expect("glium program generation failed");
      
  let map =Map {machines: ArrayVec::new(),};
  let output_edges = map.output_edges();
  let ordering = map.topological_ordering_of_noncyclic_machines(& output_edges);
  let future = map.future (& output_edges, & ordering);
      
  let state = Rc::new (RefCell::new (State {
    glium_display: display, glium_program: program, sprite_sheet: None,
    map, future, start_ui_time: now(), current_game_time: 0,
    mouse_facing: 0, mouse_position: Vector::new (0, 0), mouse_pressed: false,
  }));
  
  let click_callback = {let state = state.clone(); move |x: f64,y: f64 | {
    click (&mut state.borrow_mut(), tile_position (Vector2::new (x,y)));
  }};
  let mousedown_callback = {let state = state.clone(); move |_x: f64,_y: f64 | {
    state.borrow_mut().mouse_pressed = true;
  }};
  let mouseup_callback = {let state = state.clone(); move |_x: f64,_y: f64 | {
    state.borrow_mut().mouse_pressed = false;
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
    $("#canvas").attr ("width", 600).attr ("height", 600)
      .click (mouse_callback (@{click_callback}));
    $("body")
      .on("mousedown", mouse_callback (@{mousedown_callback}))
      .on("mouseup", mouse_callback (@{mouseup_callback}))
      .on("mousemove", mouse_callback (@{mousemove_callback}));
  }
  
  for (index, choice) in machine_choices().into_iter().enumerate() {
    let id = format! ("Machine_choice_{}", index);
    js!{
      $("<input>", {type: "radio", id:@{& id}, name: "machine_choice", value: @{index as i32}, checked:@{index == 0}}).appendTo ($("#app"));
      $("<label>", {for:@{& id}, text: @{choice.name()}}).appendTo ($("#app"));
    }
  }
  
  run(move |_inputs| {
    do_frame (& state);
  });
  
  stdweb::event_loop();
}

fn click (state: &mut State, position: Vector) {
  let choice: usize = js!{ return +$("input:radio[name=machine_choice]:checked").val()}.try_into().unwrap();
  let machine_type = machine_choices() [choice].clone();
  build_machine (state, machine_type, MachineMapState {position, facing: 0});
}

fn build_machine (state: &mut State, machine_type: MachineType, map_state: MachineMapState) {
  let materials_state =MachineMaterialsState::empty (& machine_type, state.current_game_time);
  if state.map.machines.iter().any (| machine | machine.map_state.position == map_state.position) {
    return;
  }
  if state.map.machines.try_push (StatefulMachine {
    machine_type,
    map_state,
    materials_state,
  }).is_err() {
    return;
  }
  recalculate_future (state);
}

fn recalculate_future (state: &mut State) {
  let output_edges = state.map.output_edges();
  let ordering = state.map.topological_ordering_of_noncyclic_machines(& output_edges);
  state.future = state.map.future (& output_edges, & ordering);
}

fn mouse_move (state: &mut State, position: Vector) {
  let delta = position - state.mouse_position;
  let facing = match (delta [0], delta [1]) {
    (1, 0) => Some(0),
    (0, 1) => Some(1),
    (-1, 0) => Some(2),
    (0, -1) => Some(3),
    _=> None,
  };
  
  if let Some(facing) = facing {
    state.mouse_facing = facing;
    if state.mouse_pressed {
      let mut found_machine = false;
      for machine in &mut state.map.machines {
        if machine.map_state.position == state.mouse_position {
          found_machine = true;
          machine.map_state.facing = facing;
        }
      }
      if found_machine {
        recalculate_future (state) ;
      } else {
        build_machine (state, conveyor(), MachineMapState {position: state.mouse_position, facing: facing});
      }
    }
  }

  state.mouse_position = position;
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
  let fractional_time = (now() - state.start_ui_time)*2.0;
  state.current_game_time = fractional_time as Number;
  state.map.update_to (& state.future, state.current_game_time);
  
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
        tile_center (machine.map_state.position),
        tile_size(),
        machine_color (machine), drawn.icon
      );
    }
    /*for machine in & state.map.machines {
      for (input_location, input_facing) in machine.machine_type.input_locations (& machine.map_state) {
        draw_rectangle (&mut vertices, sprite_sheet,
          tile_center (input_location),
          tile_size()* 0.8,
          machine_color (machine)
        );
      }
    }
    for machine in & state.map.machines {
      for (output_location, output_facing) in machine.machine_type.output_locations (& machine.map_state) {
        draw_rectangle (&mut vertices, sprite_sheet,
          tile_center (output_location),
          tile_size()* 0.6,
          machine_color (machine)
        );
      }
    }*/
    for (machine_index, machine) in state.map.machines.iter().enumerate() {
      let future_output_patterns: Inputs <_> = machine.machine_type.future_output_patterns (& machine.materials_state, & state.future [machine_index].inputs_at (state.current_game_time));
      //let relevant_output_patterns = future_output_patterns.into_iter().map (| list | list.into_iter().rev().find(|(time,_pattern)| *time <= state.current_game_time).unwrap().1).collect();
      let start_time = state.current_game_time;
      let end_time = start_time + TIME_TO_MOVE_MATERIAL;
      for ((output_location, output_facing), patterns) in machine.machine_type.output_locations (& machine.map_state).into_iter().zip (future_output_patterns) {
        for (pattern_index, (pattern_start_time, pattern)) in patterns.iter().enumerate() {
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
            let offset = Vector2::new (tile_size() [0]*(progress - 1.0), 0.0).rotate_90 (output_facing);
            draw_rectangle (&mut vertices, sprite_sheet,
              tile_center (output_location) + offset,
              tile_size()*0.6,
              [0.0,0.0,0.0], "iron"
            );
          }
        }
      }
    }
    for (machine_index, machine) in state.map.machines.iter().enumerate() {
      for (storage_location, storage) in machine.machine_type.displayed_storage (& machine.map_state, & machine.materials_state,& state.future [machine_index].inputs_at (state.current_game_time), state.current_game_time) {
        for index in 0..min (3, storage) {
          let mut position = tile_center (storage_location);
          position [1] += tile_size() [1]*index as f32*0.1;
          draw_rectangle (&mut vertices, sprite_sheet,
            position,
            tile_size()*0.6,
            [0.0,0.0,0.0], "iron"
          );
        }
      }
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
  