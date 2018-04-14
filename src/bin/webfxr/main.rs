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

use std::rc::Rc;
use std::cell::RefCell;
use stdweb::Value;
use stdweb::unstable::TryInto;
use stdweb::web::{self, TypedArray};
use std::time::{Instant, Duration};
pub use eliduprees_web_games::*;

#[macro_use]
mod data;
mod rendering;
mod ui;
mod randomization;
pub use data::*;
pub use rendering::*;
pub use ui::*;
pub use randomization::*;


pub struct Playback {
  start_audio_time: f64,
}

pub struct State {
  pub sound: SoundDefinition,
  pub rendering_state: RenderingState,
  pub playback_state: Option <Playback>,
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
  
  render_loop (state.clone());
}


fn render_loop (state: Rc<RefCell<State>>) {
  let mut unfinished;
  
  {
    let mut guard = state.borrow_mut();
    let start = Instant::now();
    
    loop {
      { let state = &mut*guard; unfinished = state.rendering_state.step(& state.sound); }
      //if !unfinished {play (&mut guard);}
      let elapsed = start.elapsed();
      if elapsed.as_secs() > 0 || elapsed.subsec_nanos() > 5_000_000 {
        break;
      }
    }
    play (&mut guard);
  }
  
  if unfinished {
    web::window().request_animation_frame (move | _time | render_loop (state));
  }
}

fn play (state: &mut State) {
  let rendered_duration = state.rendering_state.final_samples().samples.len() as f64/state.sound.sample_rate() as f64;
  let now = js!{return audio.currentTime;}.try_into().unwrap();
  let (offset, duration) = match state.playback_state {
    None => {
      state.playback_state = Some(Playback {start_audio_time: now});
      (0.0, rendered_duration)
    },
    Some (ref mut playback) => {
      let tentative_offset = now - playback.start_audio_time;
      if tentative_offset < rendered_duration {
        (tentative_offset, rendered_duration - tentative_offset)
      } else {
        playback.start_audio_time = now;
        (0.0, rendered_duration)
      }
    }
  };
  js! {
    play_buffer (window.webfxr_play_buffer,@{offset},@{duration});
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
  }));
  

  
  redraw(&state);
    
  stdweb::event_loop();
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this natively");
}
