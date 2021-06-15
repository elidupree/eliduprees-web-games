use crate::game::Game;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

mod js {
  use wasm_bindgen::prelude::*;

  #[wasm_bindgen]
  extern "C" {
    // this wants to return (), but that gets me "clear_canvas is not defined" for some reason
    pub fn clear_canvas() -> JsValue;
    pub fn draw_rect(cx: f32, cy: f32, sx: f32, sy: f32);
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
    accumulated_game_time:zero.zero,
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

  with_state(|state| {});
}

#[wasm_bindgen]
#[derive(Copy, Clone, Deserialize)]
pub struct StateFromJs {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
}

#[wasm_bindgen]
impl StateFromJs {
  #[wasm_bindgen(constructor)]
  pub fn from_js_value(v: JsValue) -> Self {
    v.into_serde().unwrap()
  }
}

#[wasm_bindgen]
pub fn rust_do_frame(frame_time: f64) {
  with_state(|state| {
    js::clear_canvas();
    if let Some(last_frame_time) = state.last_frame_time {
      let difference = (frame_time - last_frame_time).min(1.0 / 29.9);
      state.accumulated_game_time += difference;
      state.game.update_until(state.accumulated_game_time);
    }
    state.last_frame_time = Some(frame_time);
  })
}
