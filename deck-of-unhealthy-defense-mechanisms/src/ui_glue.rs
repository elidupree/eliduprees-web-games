use crate::game::{Game, Intent};
use crate::map::FloatingVector;
use serde::Deserialize;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

pub mod js {
  use wasm_bindgen::prelude::*;

  #[wasm_bindgen]
  extern "C" {
    // this wants to return (), but that gets me "clear_canvas is not defined" for some reason
    pub fn clear_canvas() -> JsValue;
    pub fn debug(message: &str);
    pub fn draw_rect(cx: f32, cy: f32, sx: f32, sy: f32, color: &str);
    pub fn draw_text(x: f32, y: f32, size: f32, color: &str, text: &str);
  }
}

struct State {
  game: Game,
  last_frame_time: Option<f64>,
  accumulated_game_time: f64,
}

thread_local! {
  static STATE: RefCell<State> = {
    RefCell::new(State {
      game : Game::new(),
      last_frame_time: None,
    accumulated_game_time:0.0,
    })
  }
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
  STATE.with(|state| {
    let mut guard = state.borrow_mut();
    (f)(&mut *guard)
  })
}

#[wasm_bindgen]
pub fn rust_init() {
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));
  live_prop_test::initialize();

  //with_state(|state| {});
}

#[derive(Clone, Deserialize)]
pub struct StateFromJs {
  pub intent: Intent,
  pub canvas_physical_size: FloatingVector,
  pub canvas_css_size: FloatingVector,
}

pub trait Draw {
  fn rectangle_on_map(
    &mut self,
    layer: i32,
    center: FloatingVector,
    size: FloatingVector,
    color: &str,
  );
  fn text(&mut self, position: FloatingVector, size: f64, color: &str, text: &str);
}
#[derive(Default)]
struct ProvisionalDraw {
  rectangles: Vec<(i32, FloatingVector, FloatingVector, String)>,
  text: Vec<(FloatingVector, f64, String, String)>,
}
impl Draw for ProvisionalDraw {
  fn rectangle_on_map(
    &mut self,
    layer: i32,
    center: FloatingVector,
    size: FloatingVector,
    color: &str,
  ) {
    self
      .rectangles
      .push((layer, center, size, color.to_string()));
  }
  fn text(&mut self, position: FloatingVector, size: f64, color: &str, text: &str) {
    self
      .text
      .push((position, size, color.to_string(), text.to_string()))
  }
}

#[wasm_bindgen]
pub fn rust_do_frame(frame_time: f64, state_from_js: JsValue) {
  let state_from_js = state_from_js.into_serde().unwrap();
  let StateFromJs {
    intent,
    canvas_physical_size,
    canvas_css_size: _,
  } = &state_from_js;

  let canvas_scale = f64::min(canvas_physical_size[0], canvas_physical_size[1]) / 40.0;

  with_state(|state| {
    if let Some(last_frame_time) = state.last_frame_time {
      let difference = (frame_time - last_frame_time).min(1.0 / 29.9);
      state.accumulated_game_time += difference;
    }
    state.last_frame_time = Some(frame_time);

    state
      .game
      .update_until(state.accumulated_game_time, *intent);

    let mut draw = ProvisionalDraw::default();
    state.game.draw(&mut draw);

    let canvas_position =
      |v| (canvas_physical_size * 0.5) + (v - state.game.player.position) * canvas_scale;
    draw.rectangles.sort_by_key(|&(layer, _, _, _)| layer);

    js::clear_canvas();
    for (_layer, center, size, color) in draw.rectangles {
      let center = canvas_position(center);
      let size = size * canvas_scale;
      js::draw_rect(
        center[0] as f32,
        center[1] as f32,
        size[0] as f32,
        size[1] as f32,
        &color,
      );
    }
    for (position, size, color, text) in draw.text {
      js::draw_text(
        (canvas_physical_size[0] * position[0]) as f32,
        (canvas_physical_size[1] * position[1]) as f32,
        size as f32,
        &color,
        &text,
      );
    }
  })
}
