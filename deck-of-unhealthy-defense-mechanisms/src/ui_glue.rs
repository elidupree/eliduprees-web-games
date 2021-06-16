use crate::game::{Game, Intent};
use crate::map::{FloatingVector, GridVectorExtension, TILE_SIZE};
use serde::Deserialize;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

mod js {
  use wasm_bindgen::prelude::*;

  #[wasm_bindgen]
  extern "C" {
    // this wants to return (), but that gets me "clear_canvas is not defined" for some reason
    pub fn clear_canvas() -> JsValue;
    pub fn draw_rect(cx: f32, cy: f32, sx: f32, sy: f32, color: &str);
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

  with_state(|state| {});
}

#[derive(Clone, Deserialize)]
pub struct StateFromJs {
  pub intent: Intent,
  pub canvas_physical_size: FloatingVector,
  pub canvas_css_size: FloatingVector,
}

#[wasm_bindgen]
pub fn rust_do_frame(frame_time: f64, state_from_js: JsValue) {
  let state_from_js = state_from_js.into_serde().unwrap();
  let StateFromJs {
    intent,
    canvas_physical_size,
    canvas_css_size,
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

    let canvas_position =
      |v| (canvas_physical_size * 0.5) + (v - state.game.player.position) * canvas_scale;
    let draw_rect = |pos: FloatingVector, size: FloatingVector, color| {
      let pos = canvas_position(pos);
      let size = size * canvas_scale;
      js::draw_rect(
        pos[0] as f32,
        pos[1] as f32,
        size[0] as f32,
        size[1] as f32,
        color,
      );
    };
    js::clear_canvas();

    for (&tile_position, tile) in &state.game.map.tiles {
      if let Some(mechanism) = &tile.mechanism {
        draw_rect(
          tile_position.to_floating(),
          TILE_SIZE.to_floating(),
          if mechanism.is_deck { "#f66" } else { "#888" },
        );
      }
    }

    for (&tile_position, tile) in &state.game.map.tiles {
      for material in &tile.materials {
        draw_rect(material.position, TILE_SIZE.to_floating() * 0.25, "#fff");
      }
    }

    draw_rect(state.game.player.position, TILE_SIZE.to_floating(), "#fff");
  })
}
