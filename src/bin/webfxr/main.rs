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

mod data;
#[macro_use]
mod ui;
pub use data::*;
pub use ui::*;


fn redraw(state: & Rc<RefCell<State>>) {
  {
  let guard = state.borrow();
  let sound = & guard.sound;
  
  let envelope_samples = display_samples (sound, | time | sound.envelope.sample (time));
  
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
$("#panels").append ($("<div>", {class: "panel"}).append (@{waveform_input}));

      const envelope_editor = $("<div>", {class: "panel"});
      $("#panels").append (envelope_editor);
      envelope_editor.append (@{canvas_of_samples (&envelope_samples)});
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

  
  }
  
  SignalEditorSpecification {
    state: & state,
    id: "frequency",
    name: "Frequency",
    slider_range: [20f32.log2(), 5000f32.log2()],
    difference_slider_range: [-2.0, 2.0],
    getter: getter! (state => state.sound.log_frequency),
  }.render();
  SignalEditorSpecification {
    state: & state,
    id: "volume",
    name: "Volume",
    slider_range: [0.0,1.0],
    difference_slider_range: [-1.0, 1.0],
    getter: getter! (state => state.sound.volume),
  }.render();
  SignalEditorSpecification {
    state: & state,
    id: "lowpass",
    name: "Low-pass filter cutoff",
    slider_range: [20f32.log2(), 48000f32.log2()],
    difference_slider_range: [-5.0, 5.0],
    getter: getter! (state => state.sound.log_lowpass_filter_cutoff),
  }.render();
  SignalEditorSpecification {
    state: & state,
    id: "highpass",
    name: "High-pass filter cutoff",
    slider_range: [20f32.log2(), 20000f32.log2()],
    difference_slider_range: [-5.0, 5.0],
    getter: getter! (state => state.sound.log_highpass_filter_cutoff),
  }.render();
  SignalEditorSpecification {
    state: & state,
    id: "bitcrush_frequency",
    name: "Bitcrush frequency",
    slider_range: [20f32.log2(), 48000f32.log2()],
    difference_slider_range: [-5.0, 5.0],
    getter: getter! (state => state.sound.log_bitcrush_frequency),
  }.render();
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
