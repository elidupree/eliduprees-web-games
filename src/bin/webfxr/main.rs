#![feature (option_filter)]
#![recursion_limit="256"]

extern crate eliduprees_web_games;

#[macro_use]
extern crate stdweb;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate derivative;
extern crate nalgebra;
extern crate rand;
extern crate ordered_float;
extern crate array_ext;

use std::rc::Rc;
use std::cell::RefCell;
use stdweb::Value;
use stdweb::unstable::TryInto;
use stdweb::web::{self, TypedArray};
use std::time::{Instant, Duration};
pub use array_ext::Array;
pub use eliduprees_web_games::*;


#[macro_use]
mod misc;
#[macro_use]
mod data;
mod rendering;
mod ui;
mod randomization;
pub use misc::*;
pub use data::*;
pub use rendering::*;
pub use ui::*;
pub use randomization::*;


type SamplesGetter = Rc<Fn(& RenderingState)->& RenderedSamples>;
#[derive (Clone)]
pub struct Playback {
  start_audio_time: f64,
  samples_getter: SamplesGetter,
}

pub struct State {
  pub sound: SoundDefinition,
  pub rendering_state: RenderingState,
  pub playback_state: Option <Playback>,
  pub loop_playback: bool,
}


fn redraw(state: & Rc<RefCell<State>>) {
  {
    let mut guard = state.borrow_mut();
    let state = &mut*guard;
    state.rendering_state = RenderingState::new (& state.sound);
  }
  {
  let guard = state.borrow();
  let sound = & guard.sound;
  
  let mut rows = 1;
  pub fn assign_row (rows: u32, element: Value)->Value {
    js!{@{&element}.css("grid-row", @{rows}+" / span 1")};
    element
  }
  
  let sample_rate = 500.0;
  let envelope_samples = display_samples (sample_rate, sound.duration(), | time | sound.envelope.sample (time));
  
  macro_rules! add_envelope_input {
  ($variable: ident, $name: expr, $range: expr) => {
    let input = assign_row(rows, NumericalInputSpecification {
    state: state,
    id: stringify! ($variable),
    name: $name, 
    slider_range: $range,
    current_value: sound.envelope.$variable.clone(),
    input_callback: input_callback (state, move | state, value: UserTime | {
      if value.rendered >= 0.0 && value.rendered <= 30.0 {
        state.sound.envelope.$variable = value;
        return true
      }
      false
    }),
  }.render());
    
    let label = assign_row(rows, js!{ return @{&input}.children("label");});
    js!{@{&label}.append(":").addClass("toplevel_input_label")}
    js!{jQuery("#panels").append (@{label},@{input});}
    rows += 1;
    }
  }
    
  js!{$("#panels").empty();}  
  
      
  let play_button = assign_row (rows, button_input ("Play",
    { let state = state.clone(); move || {
      play (&mut state.borrow_mut(), Rc::new (| state | state.final_samples()));
      false
    }}
  ));
  js!{$("#panels").append (@{play_button});}
  rows += 1;
  
  let loop_button = assign_row (rows, checkbox_meta_input (state, "loop", "Loop", getter! (state => state.loop_playback)));
  js!{$("#panels").append (@{loop_button});}
  rows += 1;
      
  let randomize_button = assign_row (rows, button_input ("Randomize",
    input_callback_nullary (state, move | state | {
      state.sound = random_sound (&mut rand::thread_rng());
      true
    })
  ));
  js!{$("#panels").append (@{randomize_button});}
  rows += 1;
  
  let waveform_start = rows;
  let waveform_input = assign_row (rows, waveform_input (state, "waveform", "Waveform", getter! (state => state.sound.waveform)));
  let label = assign_row(rows, js!{ return @{&waveform_input}.children("label").first();});
  js!{@{&label}.addClass("toplevel_input_label")}
  js!{jQuery("#panels").append (@{label},@{waveform_input}.addClass("sound_waveform_input"));}
  rows += 1;
  
  js!{ $("#panels").prepend ($("<div>", {class:"input_region"}).css("grid-row", @{waveform_start}+" / "+@{rows})); }
  

  js!{$("#panels").append (
    @{canvas_of_samples (&envelope_samples, sample_rate, [0.0, 1.0], sound.duration())}
    .css("grid-row", @{rows}+" / span 3")
  );}
  js!{ $("#panels").prepend ($("<div>", {class:"input_region"}).css("grid-row", @{rows}+" / span 3")); }
  add_envelope_input!(attack, "Attack", [0.0, 1.0]);
  add_envelope_input!(sustain, "Sustain", [0.0, 3.0]);
  add_envelope_input!(decay, "Decay", [0.0, 3.0]);
  
  struct Visitor <'a> (& 'a Rc<RefCell<State>>, & 'a mut u32);
  impl<'a> SignalVisitor for Visitor<'a> {
    fn visit <T: UserNumberType> (&mut self, info: & TypedSignalInfo <T>, _signal: & Signal <T>) {
      SignalEditorSpecification {
    state: self.0,
    info: info,
    rows: self.1,
  }.render();
    }
  }
  
  let mut visitor = Visitor (state, &mut rows);
  for caller in sound.visit_callers::<Visitor>() {(caller)(&mut visitor, sound);}
  
  //js! {window.before_render = Date.now();}
  //let rendered: TypedArray <f64> = sound.render (44100).as_slice().into();
  
  //js! {console.log("rendering took this many milliseconds: " + (Date.now() - window.before_render));}
  
  }
  
  play (&mut state.borrow_mut(), Rc::new (| state | state.final_samples()));
}


fn render_loop (state: Rc<RefCell<State>>) {
  {
    let mut guard = state.borrow_mut();
    let state = &mut*guard;
    let start = Instant::now();
    
    let already_finished = state.rendering_state.finished();
    
    while !state.rendering_state.finished() {
      { state.rendering_state.step(& state.sound); }
      let elapsed = start.elapsed();
      if elapsed.as_secs() > 0 || elapsed.subsec_nanos() > 5_000_000 {
        break;
      }
    }
    
    let rendered_duration = state.rendering_state.final_samples().samples.len() as f64/state.sound.sample_rate() as f64;
    if (state.rendering_state.finished() || rendered_duration > 0.2) && !already_finished {
      match state.playback_state {
        Some(ref playback) => {
          let now: f64 = js!{return audio.currentTime;}.try_into().unwrap();
          let tentative_offset = now - playback.start_audio_time;
          if tentative_offset < rendered_duration {
            js! {
              play_buffer (@{&(playback.samples_getter) (&state.rendering_state).audio_buffer},@{tentative_offset},@{rendered_duration - tentative_offset});
            }
          }
        }
        None => (),
      }
    }
    
    match state.playback_state.clone() {
      Some(playback) => {
        let now: f64 = js!{return audio.currentTime;}.try_into().unwrap();
        let offset = now - playback.start_audio_time;
        if offset > state.sound.duration() {
          if state.loop_playback {
            play (state, playback.samples_getter);
          } else {
            let samples = (playback.samples_getter) (&state.rendering_state);
            samples.redraw (None, & state.rendering_state.constants);
            state.playback_state = None;
          }
        } else {
          let samples = (playback.samples_getter) (&state.rendering_state);
          samples.redraw (Some(offset), & state.rendering_state.constants);
        }
      }
      None => (),
    }
  }
  
  web::window().request_animation_frame (move | _time | render_loop (state));
}

fn play (state: &mut State, getter: SamplesGetter) {
  let samples = (getter) (&state.rendering_state);
  if let Some(ref playback) = state.playback_state {
    let old_samples = (playback.samples_getter) (&state.rendering_state);
    if old_samples.serial_number != samples.serial_number {
      old_samples.redraw (None, & state.rendering_state.constants);
    }
  }

  let now: f64 = js!{return audio.currentTime;}.try_into().unwrap();
  state.playback_state = Some(Playback {
    start_audio_time: now,
    samples_getter: getter,
  });
  js! {
    play_buffer (@{&samples.audio_buffer});
  }  
}


#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  
  let state = Rc::new (RefCell::new (State {
    sound: SoundDefinition {
      waveform: Waveform::Sine,
      envelope: Envelope {attack: UserNumber::from_rendered (0.1), sustain: UserNumber::from_rendered (0.5), decay: UserNumber::from_rendered (0.5)},
      log_frequency: Signal::constant (UserNumber::from_rendered (220.0_f64.log2())),
      volume: Signal::constant (UserNumber::from_rendered (-4.0)),
      log_bitcrush_frequency: Signal::constant (UserNumber::from_rendered (44100.0_f64.log2())),
      log_lowpass_filter_cutoff: Signal::constant (UserNumber::from_rendered (44100.0_f64.log2())),
      log_highpass_filter_cutoff: Signal::constant (UserNumber::from_rendered (20.0_f64.log2())),
    },
    rendering_state: Default::default(),
    playback_state: None,
    loop_playback: false,
  }));
  

  
  redraw(&state);
  render_loop (state.clone());
    
  stdweb::event_loop();
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this natively");
}
