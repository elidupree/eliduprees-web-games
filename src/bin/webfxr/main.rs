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
use stdweb::web::TypedArray;

#[macro_use]
mod data;
mod ui;
mod randomization;
pub use data::*;
pub use ui::*;
pub use randomization::*;


fn redraw(state: & Rc<RefCell<State>>) {
  let guard = state.borrow();
  let sound = & guard.sound;
  
  let mut rows = 1;
  pub fn assign_row (rows: u32, element: Value)->Value {
    js!{@{&element}.css("grid-row", @{rows}+" / span 1")};
    element
  }
  
  let envelope_samples = display_samples (sound, | time | sound.envelope.sample (time));
  
  macro_rules! envelope_input {
  ($variable: ident, $name: expr, $range: expr) => {assign_row(rows, NumericalInputSpecification {
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
  }.render())
    }
  }
    
  js!{$("#panels").empty();}
      
  let randomize_button = assign_row (rows, button_input ("Randomize",
    input_callback_nullary (state, move | state | {
      state.sound = random_sound (&mut rand::thread_rng());
      true
    })
  ));
  rows += 1;
  
  let waveform_input = assign_row (rows, waveform_input (state, "waveform", "Waveform", getter! (state => state.sound.waveform)));
  rows += 1;
  
  js!{$("#panels").append (@{randomize_button});}
  js!{$("#panels").append (@{waveform_input}.addClass("sound_waveform_input"));}
  js!{$("#panels").append (
    @{canvas_of_samples (&envelope_samples, [0.0, 1.0])}
    .css("grid-row", @{rows}+" / span 3")
  );}
  js!{$("#panels").append (@{envelope_input!(attack, "Attack", [0.0, 1.0])});}
  rows += 1;
  js!{$("#panels").append (@{envelope_input!(sustain, "Sustain", [0.0, 3.0])});}
  rows += 1;
  js!{$("#panels").append (@{envelope_input!(decay, "Decay", [0.0, 3.0])});}
  rows += 1;
  
  struct Visitor <'a> (& 'a Rc<RefCell<State>>, & 'a mut u32);
  impl<'a> SignalVisitor for Visitor<'a> {
    fn visit <T: UserNumberType> (&mut self, info: &SignalInfo, _signal: & Signal <T>, getter: Getter <State, Signal <T>>) {
      SignalEditorSpecification {
    state: self.0,
    info: info,
    getter: getter,
    rows: self.1,
  }.render();
    }
  }
  
  let mut visitor = Visitor (state, &mut rows);
  for caller in sound.visit_callers::<Visitor>() {(caller)(&mut visitor, sound);}
  
  
  
  let rendered: TypedArray <f32> = sound.render (44100).as_slice().into();
  
  js! {
  const rendered = @{rendered};
const sample_rate = 44100;
  const buffer = audio.createBuffer (1, rendered.length, sample_rate);
  buffer.copyToChannel (rendered, 0);
  play_buffer (buffer);
  }  
}


#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  
  let state = Rc::new (RefCell::new (State {
    sound: SoundDefinition {
      waveform: Waveform::Sine,
      envelope: Envelope {attack: UserNumber::from_rendered (0.1), sustain: UserNumber::from_rendered (0.5), decay: UserNumber::from_rendered (0.5)},
      log_frequency: Signal::constant (UserNumber::from_rendered (220.0_f32.log2())),
      volume: Signal::constant (UserNumber::from_rendered (-4.0)),
      log_bitcrush_frequency: Signal::constant (UserNumber::from_rendered (44100.0_f32.log2())),
      log_lowpass_filter_cutoff: Signal::constant (UserNumber::from_rendered (44100.0_f32.log2())),
      log_highpass_filter_cutoff: Signal::constant (UserNumber::from_rendered (20.0_f32.log2())),
    }
  }));
  

  
  redraw(&state);
    
  stdweb::event_loop();
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this natively");
}
