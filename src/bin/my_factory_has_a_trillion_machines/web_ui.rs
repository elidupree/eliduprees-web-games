use super::*;

use stdweb;
use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
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
      let divisor = mask as f32 * 0.7;
      [
        ((hash      ) & mask) as f32/divisor,
        ((hash >> 20) & mask) as f32/divisor,
        ((hash >> 40) & mask) as f32/divisor,
      ]
}

fn tile_center (position: Vector)->Vector2 <f32> {
  Vector2::new (position [0] as f32/30.0, position [1] as f32/30.0)
}
fn tile_size()->Vector2 <f32> {
  Vector2::new (1.0/30.0, 1.0/30.0)
}

fn draw_rectangle (vertices: &mut Vec<Vertex>, sprite_sheet: & SpriteSheet, center: Vector2<f32>, size: Vector2<f32>, color: [f32; 3], sprite: & str) {
  let bounds = &sprite_sheet.bounds_map [sprite];
  let sprite_size = [
    bounds.width as f32/sprite_sheet.size [0] as f32,
    -(bounds.height as f32/sprite_sheet.size [1] as f32),
  ];
  let sprite_position = [
    bounds.x as f32/sprite_sheet.size [0] as f32,
    (bounds.y + bounds.height) as f32/sprite_sheet.size [1] as f32,
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
  }));
  
  let click_callback = {let state = state.clone(); move |x: f64,y: f64 | {
    let position = Vector::new ((x*30.0).round() as Number, (y*30.0).round() as Number);
    let choice: usize = js!{ return +$("input:radio[name=machine_choice]:checked").val()}.try_into().unwrap();
    let machine_type = machine_choices() [choice].clone();
    let mut state = state.borrow_mut();
    let materials_state =MachineMaterialsState::empty (& machine_type, state.current_game_time);
    state.map.machines.push (StatefulMachine {
      machine_type,
      map_state: MachineMapState {position, facing: 0},
      materials_state,
    });
    let output_edges = state.map.output_edges();
    let ordering = state.map.topological_ordering_of_noncyclic_machines(& output_edges);
    state.future = state.map.future (& output_edges, & ordering);
  }};
  
  js!{
    $("#canvas").attr ("width", 600).attr ("height", 600).click (function(event) {
      var offset = canvas.getBoundingClientRect();
      var x = (event.clientX - offset.left)/offset.width;
      var y = 1.0 - (event.clientY - offset.top)/offset.height;
      @{click_callback}(x,y);
    })
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
      let patterns = machine.machine_type.current_outputs_and_next_change (& machine.materials_state, & state.future [machine_index].inputs_at (state.current_game_time)).0;
      for ((output_location, output_facing), pattern) in machine.machine_type.output_locations (& machine.map_state).into_iter().zip (patterns) {
        if pattern.num_disbursed_at_time (state.current_game_time) > 0 {
          let offset = Vector2::new (tile_size() [0]*(fractional_time.fract() as f32 - 1.0), 0.0).rotate_90 (output_facing);
          draw_rectangle (&mut vertices, sprite_sheet,
            tile_center (output_location) + offset,
            tile_size()*0.6,
            [0.0,0.0,0.0], "iron"
          );
        }
      }
    }
    for (machine_index, machine) in state.map.machines.iter().enumerate() {
      for (storage_location, storage) in machine.machine_type.displayed_storage (& machine.map_state, & machine.materials_state,& state.future [machine_index].inputs_at (state.current_game_time), state.current_game_time) {
        let storage_fraction = storage as f32*0.1;
        let mut size = tile_size();
        if storage_fraction < 1.0 {size [1] *= storage_fraction;}
        draw_rectangle (&mut vertices, sprite_sheet,
          tile_center (storage_location),
          size,
          [0.0,0.0,0.0], "iron"
        );
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
  