#![recursion_limit = "256"]
#![feature(specialization)]

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
  thread_local! {
    static LATEST_AUTO_CONSTANTS: RefCell<HashMap<String, Box<dyn Any>>> = RefCell:: new (HashMap::new());
    static DEFAULT_AUTO_CONSTANTS: RefCell<HashMap<String, Box<dyn Any>>> = RefCell:: new (HashMap::new());
  }
  DEFAULT_AUTO_CONSTANTS.with(|defaults| {
    let insert_result = defaults.borrow_mut().insert(name.to_owned(), Box::new(default.clone()));
    if let Some(previous) = insert_result {
      let previous = previous.downcast_ref::< T >().unwrap();
      if !previous.eq(&default) {
        panic!("tried to use auto-constant {} with default {:?}, but it had already been used with default {:?}!", name, default, previous)
      }
    }
  });
  let from_js = js::auto_constant(name, default.clone().into());
  if let Some(t) = T::js_value_as(&from_js) {
    LATEST_AUTO_CONSTANTS.with(|latest| {
      latest
        .borrow_mut()
        .insert(name.to_owned(), Box::new(t.clone()));
    });
    t
  } else {
    LATEST_AUTO_CONSTANTS.with(|latest| {
      if let Some(t) = latest.borrow().get(name) {
        let t = t.downcast_ref::<T>().unwrap().clone();
        // debug!(
        //   "js gave invalid value {:?} for auto-constant `{}`; using the previous value of {:?}",
        //   from_js, t
        // );
        t
      } else {
        // debug!(
        //   "js gave invalid value {:?} for auto-constant `{}`; using the default value of {:?}",
        //   from_js, default
        // );
        default
      }
    })
  }
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
