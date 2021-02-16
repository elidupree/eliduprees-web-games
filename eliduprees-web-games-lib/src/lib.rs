#![recursion_limit = "256"]
#![feature(slice_patterns, specialization)]

#[macro_use]
extern crate stdweb;
extern crate arrayvec;
extern crate boolinator;
extern crate lyon;
extern crate nalgebra;
extern crate ordered_float;
extern crate rand;

use rand::Rng;
use stdweb::unstable::TryInto;
use stdweb::web;
use stdweb::JsSerialize;

use nalgebra::Vector2;

pub const TURN: f64 = ::std::f64::consts::PI * 2.0;

#[macro_export]
macro_rules! js_unwrap {
  ($($t: tt)*) => {
    {
      use stdweb::unstable::TryInto;
      let t = js! {$($t)*}.try_into().unwrap();
      t
    }
  };
}

pub fn now() -> f64 {
  let milliseconds: f64 = js! { return Date.now()}.try_into().unwrap();
  milliseconds * 0.001
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
pub fn auto_constant<T>(name: &str, default: T) -> T
where
  T: JsSerialize,
  stdweb::Value: TryInto<T>,
  <stdweb::Value as TryInto<T>>::Error: ::std::fmt::Debug,
{
  (js! {
    var value = window.auto_constants [@{name}];
    if (value === undefined) {
      return window.auto_constants [@{name}] = @{default};
    }
    return value;
  })
  .try_into()
  .unwrap()
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

pub struct FrameCallbackInputs {
  pub time: f64,
  pub window_dimensions: Vector2<f64>,

  pub resized_last_frame: bool,
  pub last_frame_duration: f64,
}

fn main_loop<F: 'static + FnMut(&FrameCallbackInputs)>(
  time: f64,
  mut inputs: FrameCallbackInputs,
  mut frame_callback: F,
) {
  let current_window_dimensions = Vector2::new(
    js! {return window.innerWidth;}.try_into().unwrap(),
    js! {return window.innerHeight;}.try_into().unwrap(),
  );
  inputs.resized_last_frame = current_window_dimensions != inputs.window_dimensions;
  inputs.window_dimensions = current_window_dimensions;

  inputs.last_frame_duration = time - inputs.time;
  inputs.time = time;

  (frame_callback)(&inputs);

  web::window().request_animation_frame(move |time| main_loop(time, inputs, frame_callback));
}

pub fn run<F: 'static + FnMut(&FrameCallbackInputs)>(frame_callback: F) {
  if js! {return window.eliduprees_web_games.cancel_starting;}
    .try_into()
    .unwrap()
  {
    return;
  }

  main_loop(
    0.0,
    FrameCallbackInputs {
      time: 0.0,
      window_dimensions: Vector2::new(-1.0, -1.0),
      resized_last_frame: true,
      last_frame_duration: 0.0,
    },
    frame_callback,
  );

  stdweb::event_loop();
}
