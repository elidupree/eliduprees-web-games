use super::*;

use stdweb;
use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use glium::{Surface};
use arrayvec::ArrayVec;
use stdweb::unstable::TryInto;
use siphasher::sip::SipHasher;
use nalgebra::Vector2;



struct State {
  glium_display: glium::Display,
  glium_program: glium::Program,
  map: Map,
  future: MachinesFuture,
}

#[derive(Copy, Clone)]
struct Vertex {
  position: [f32; 2],
  color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

fn machine_choices()->Vec<StandardMachine> { vec![conveyor(), splitter(), merger(), slow_machine(), material_generator(), consumer()]}

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

fn draw_rectangle (vertices: &mut Vec<Vertex>, center: Vector2<f32>, size: Vector2<f32>, color: [f32; 3]) {
  let vertex = |x,y| Vertex {
            position: [center [0] + size [0]*x as f32, center [1] + size [1]*y as f32],
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
varying lowp vec3 color_transfer;

void main() {
gl_Position = vec4 (position*2.0 - 1.0, 0.0, 1.0);

color_transfer = color;
}

"#;

  let fragment_shader_source = r#"
#version 100
varying lowp vec3 color_transfer;

void main() {
gl_FragColor = vec4(color_transfer, 1.0);
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
    glium_display: display, glium_program: program,
    map, future,
  }));
  
  let click_callback = {let state = state.clone(); move |x: f64,y: f64 | {
    let position = Vector::new ((x*30.0).round() as Number, (y*30.0).round() as Number);
    let choice: usize = js!{ return +$("input:radio[name=machine_choice]:checked").val()}.try_into().unwrap();
    let machine_type = machine_choices() [choice].clone();
    let materials_state =MachineMaterialsState::empty (& machine_type);
    let mut state = state.borrow_mut();
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
      $("<label>", {for:@{& id}, text: @{choice.name}}).appendTo ($("#app"));
    }
  }
  
  run(move |_inputs| {
    do_frame (& state);
  });
  
  stdweb::event_loop();
}



fn do_frame(state: & Rc<RefCell<State>>) {
  let state = state.borrow_mut();
  let parameters = glium::DrawParameters {
    blend: glium::draw_parameters::Blend::alpha_blending(),
    ..Default::default()
  };
  let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let mut target = state.glium_display.draw();
    target.clear_color(1.0, 1.0, 1.0, 1.0);
    let mut vertices = Vec::<Vertex>::new();
    
    for machine in & state.map.machines {
      draw_rectangle (&mut vertices,
        tile_center (machine.map_state.position),
        tile_size(),
        machine_color (machine)
      );
    }
    for machine in & state.map.machines {
      for input_location in machine.machine_type.input_locations (& machine.map_state) {
        draw_rectangle (&mut vertices,
          tile_center (input_location),
          tile_size()* 0.8,
          machine_color (machine)
        );
      }
    }
    for machine in & state.map.machines {
      for output_location in machine.machine_type.output_locations (& machine.map_state) {
        draw_rectangle (&mut vertices,
          tile_center (output_location),
          tile_size()* 0.6,
          machine_color (machine)
        );
      }
    }


    target.draw(&glium::VertexBuffer::new(& state.glium_display, &vertices)
                .expect("failed to generate glium Vertex buffer"),
              &indices,
              & state.glium_program,
              &glium::uniforms::EmptyUniforms,
              &parameters)
        .expect("failed target.draw");

    target.finish().expect("failed to finish drawing");
}
  