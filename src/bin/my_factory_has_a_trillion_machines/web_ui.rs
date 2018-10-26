use super::*;

use stdweb;
use std::rc::Rc;
use std::cell::RefCell;
use glium::{Surface};
use arrayvec::ArrayVec;




struct State {
  glium_display: glium::Display,
  glium_program: glium::Program,
  map: Map,
}

#[derive(Copy, Clone)]
struct Vertex {
  center: [f32; 2],
  direction: [f32; 2],
  color: [f32; 3],
}
implement_vertex!(Vertex, center, direction, color);

pub fn run_game() {
  let vertex_shader_source = r#"
#version 100
attribute lowp vec2 center;
attribute lowp vec2 direction;
attribute lowp vec3 color;
varying lowp vec3 color_transfer;

void main() {
gl_Position = vec4 (center+direction, 0.0, 1.0);

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
      
  let state = Rc::new (RefCell::new (State {
    glium_display: display, glium_program: program,
    map: Map {machines: ArrayVec::new(),},
  }));
  
  let click_callback = {let state = state.clone(); move |x: f64,y: f64 | {
    let position = Vector::new ((x*30.0) as Number, (y*30.0) as Number);
    let machine_type = conveyor();
    let materials_state =MachineMaterialsState::empty (& machine_type);
    state.borrow_mut().map.machines.push (StatefulMachine {
      machine_type,
      map_state: MachineMapState {position, facing: 0},
      materials_state,
    });
  }};
  
  js!{
    $("#canvas").click (function(event) {
      var offset = canvas.getBoundingClientRect();
      var x = (event.clientX - offset.left)/offset.width;
      var y = (event.clientY - offset.top)/offset.height;
      @{click_callback}(x,y);
    })
  }
  
  run(move |inputs| {
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
    

          let vertex = |x,y| Vertex {
            direction: [0.4*x as f32, 0.3*y as f32],
            center: [0.5, 0.5],
            color: [0.0, 0.5, 1.0],
          };
          vertices.extend(&[
            vertex(-1,-1),vertex( 1,-1),vertex( 1, 1),
            vertex(-1,-1),vertex( 1, 1),vertex(-1, 1)
          ]);

    
    target.draw(&glium::VertexBuffer::new(& state.glium_display, &vertices)
                .expect("failed to generate glium Vertex buffer"),
              &indices,
              & state.glium_program,
              &glium::uniforms::EmptyUniforms,
              &parameters)
        .expect("failed target.draw");

    target.finish().expect("failed to finish drawing");
}
  