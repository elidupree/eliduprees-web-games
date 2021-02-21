#![recursion_limit = "256"]
#![feature(specialization)]

use log::warn;
use nalgebra::Vector2;
use rand::Rng;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use wasm_bindgen::prelude::*;

pub const TURN: f64 = ::std::f64::consts::PI * 2.0;

/// Statics representing web API objects, so you don't have to unwrap them each time.
pub mod web {
  thread_local! {
    pub static WINDOW: web_sys::Window = web_sys::window().expect("no global `window `exists");
    pub static PERFORMANCE: web_sys::Performance = WINDOW.with (| window | window.performance().expect ("window.performance is unavailable"));
  }
}

/// The current time relative to an arbitrary starting point, in seconds.
///
/// Currently implemented using `performance.now()`.
pub fn now() -> f64 {
  web::PERFORMANCE.with(web_sys::Performance::now) * 0.001
}

pub trait JsValueAs: Sized {
  fn js_value_as(value: &JsValue) -> Option<Self>;
}
impl JsValueAs for f64 {
  fn js_value_as(value: &JsValue) -> Option<Self> {
    value.as_f64()
  }
}
impl JsValueAs for String {
  fn js_value_as(value: &JsValue) -> Option<Self> {
    value.as_string()
  }
}
impl JsValueAs for bool {
  fn js_value_as(value: &JsValue) -> Option<Self> {
    value.as_bool()
  }
}

pub trait EqIncludingFloats {
  fn eq(&self, other: &Self) -> bool;
}
impl<T: PartialEq> EqIncludingFloats for T {
  default fn eq(&self, other: &Self) -> bool {
    self == other
  }
}
impl EqIncludingFloats for f64 {
  fn eq(&self, other: &Self) -> bool {
    self == other || (self.is_nan() && other.is_nan())
  }
}
impl EqIncludingFloats for f32 {
  fn eq(&self, other: &Self) -> bool {
    self == other || (self.is_nan() && other.is_nan())
  }
}

pub fn random_vector_exact_length<G: Rng>(generator: &mut G, length: f64) -> Vector2<f64> {
  loop {
    let vector = Vector2::new(
      generator.gen_range(-length, length),
      generator.gen_range(-length, length),
    );
    let test_length = vector.norm();
    if test_length <= length && test_length * 2.0 >= length {
      return vector * length / vector.norm();
    }
  }
}
pub fn random_vector_within_length<G: Rng>(generator: &mut G, length: f64) -> Vector2<f64> {
  loop {
    let vector = Vector2::new(
      generator.gen_range(-length, length),
      generator.gen_range(-length, length),
    );
    let test_length = vector.norm();
    if test_length <= length && test_length != 0.0 {
      return vector;
    }
  }
}

mod js {
  use wasm_bindgen::prelude::*;

  #[wasm_bindgen]
  extern "C" {
    pub fn auto_constant(name: &str, default: JsValue) -> JsValue;
  }
}

pub fn auto_constant<T>(name: &str, default: T) -> T
where
  T: JsValueAs + Into<JsValue> + Clone + EqIncludingFloats + Any + Debug,
{
  struct AutoConstantState<T> {
    known_default: Option<T>,
    latest_valid: Option<T>,
    latest_invalid: Option<JsValue>,
  }
  impl<T> Default for AutoConstantState<T> {
    fn default() -> Self {
      AutoConstantState {
        known_default: None,
        latest_valid: None,
        latest_invalid: None,
      }
    }
  }
  thread_local! {
    static AUTO_CONSTANTS: RefCell<HashMap<String, Box<dyn Any>>> = RefCell::new(HashMap::new());
  }
  AUTO_CONSTANTS.with(|auto_constants| {
    let mut auto_constants = auto_constants.borrow_mut();
    let state = auto_constants.entry(name.to_owned()).or_insert_with(|| Box::new(AutoConstantState::<T>::default()));
    let mut state = match state.downcast_mut::<AutoConstantState<T>>() {
      Some(state) => state,
      None => panic!("tried to use auto-constant {} with type {}, but it had already been used with a different type!", name, std::any::type_name::<T>()),
    };
    if let Some(known_default) = &state.known_default {
      if !known_default.eq(&default) {
        panic!("tried to use auto-constant {} with default {:?}, but it had already been used with default {:?}!", name, default, known_default)
      }
    }
    else {
      state.known_default = Some(default.clone());
    }

    let from_js = js::auto_constant(name, default.clone().into());

    if let Some(t) = T::js_value_as(&from_js) {
      state.latest_valid = Some(t.clone());
      t
    } else {
      let new_invalid = state.latest_invalid.as_ref() != Some(&from_js);
      let result = if let Some(t) = &state.latest_valid {
        if new_invalid {
          warn!(
            "js gave invalid value {:?} for auto-constant `{}`; using the previous value of {:?}",
            from_js, name, t
          );
        }
        t.clone()
      } else {
        if new_invalid {
          warn!(
            "js gave invalid value {:?} for auto-constant `{}`; using the default value of {:?}",
            from_js, name, default
          );
        }
        default
      };
      state.latest_invalid = Some(from_js);
      result
    }
  })
}

pub trait StaticDowncast<T> {
  fn static_downcast(self) -> T;
}
impl<T> StaticDowncast<T> for T {
  fn static_downcast(self) -> T {
    self
  }
}
impl<T, U> StaticDowncast<T> for U {
  default fn static_downcast(self) -> T {
    panic!("Tried to static_downcast between two different types")
  }
}
pub fn static_downcast<T, U>(input: T) -> U {
  StaticDowncast::<U>::static_downcast(input)
}

// pub struct FrameCallbackInputs {
//   pub time: f64,
//   pub window_dimensions: Vector2<f64>,
//
//   pub resized_last_frame: bool,
//   pub last_frame_duration: f64,
// }
//
// fn main_loop<F: 'static + FnMut(&FrameCallbackInputs)>(
//   time: f64,
//   mut inputs: FrameCallbackInputs,
//   mut frame_callback: F,
// ) {
//   let current_window_dimensions = Vector2::new(
//     js! {return window.innerWidth;}.try_into().unwrap(),
//     js! {return window.innerHeight;}.try_into().unwrap(),
//   );
//   inputs.resized_last_frame = current_window_dimensions != inputs.window_dimensions;
//   inputs.window_dimensions = current_window_dimensions;
//
//   inputs.last_frame_duration = time - inputs.time;
//   inputs.time = time;
//
//   (frame_callback)(&inputs);
//
//   web::window().request_animation_frame(move |time| main_loop(time, inputs, frame_callback));
// }
//
// pub fn run<F: 'static + FnMut(&FrameCallbackInputs)>(frame_callback: F) {
//   if js! {return window.eliduprees_web_games.cancel_starting;}
//     .try_into()
//     .unwrap()
//   {
//     return;
//   }
//
//   main_loop(
//     0.0,
//     FrameCallbackInputs {
//       time: 0.0,
//       window_dimensions: Vector2::new(-1.0, -1.0),
//       resized_last_frame: true,
//       last_frame_duration: 0.0,
//     },
//     frame_callback,
//   );
//
//   stdweb::event_loop();
// }
