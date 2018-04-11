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
  
  let envelope_samples = display_samples (sound, | time | sound.envelope.sample (time));
  
  let randomize_button = button_input ("Randomize",
    input_callback_nullary (state, move | state | {
      state.sound = random_sound (&mut rand::thread_rng());
      true
    })
  );
  
  macro_rules! envelope_input {
  ($variable: ident, $name: expr, $range: expr) => {NumericalInputSpecification {
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
  }.render()
    }
  }
  
  let waveform_input = waveform_input (state, "waveform", "Waveform", getter! (state => state.sound.waveform));
  
  js! {
$("#panels").empty();
$("#panels").append (@{randomize_button}, $("<div>", {class: "panel"}).append (@{waveform_input}));

      const envelope_editor = $("<div>", {class: "panel"});
      $("#panels").append (envelope_editor);
      envelope_editor.append (@{canvas_of_samples (&envelope_samples, [0.0, 1.0])});
      envelope_editor.append (@{envelope_input!(attack, "Attack", [0.0, 1.0])});
      envelope_editor.append (@{envelope_input!(sustain, "Sustain", [0.0, 3.0])});
      envelope_editor.append (@{envelope_input!(decay, "Decay", [0.0, 3.0])});
  }

  let rendered: TypedArray <f32> = sound.render (44100).as_slice().into();
  
  js! {
  const rendered = @{rendered};
const sample_rate = 44100;
  const buffer = audio.createBuffer (1, rendered.length, sample_rate);
  buffer.copyToChannel (rendered, 0);
  play_buffer (buffer);
  }  

  
  struct Visitor <'a> (& 'a Rc<RefCell<State>>);
  impl<'a> SignalVisitor for Visitor<'a> {
    fn visit <T: UserNumberType> (&mut self, info: &SignalInfo, _signal: & Signal <T>, getter: Getter <State, Signal <T>>) {
      SignalEditorSpecification {
    state: self.0,
    info: info,
    getter: getter,
  }.render();
    }
  }
  
  for caller in sound.visit_callers::<Visitor>() {(caller)(&mut Visitor (state), sound);}
}


#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  
  let state = Rc::new (RefCell::new (State {
    sound: SoundDefinition {
      waveform: Waveform::Sine,
      envelope: Envelope {attack: UserNumber::from_rendered (0.1), sustain: UserNumber::from_rendered (0.5), decay: UserNumber::from_rendered (0.5)},
      log_frequency: Signal::constant (UserNumber::from_rendered (220.0_f32.log2())),
      volume: Signal::constant (UserNumber::from_rendered (0.04)),
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
