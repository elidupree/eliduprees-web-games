#![recursion_limit="256"]

extern crate eliduprees_web_games;

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
extern crate nalgebra;
extern crate ordered_float;

use std::rc::Rc;
use std::cell::RefCell;
use stdweb::unstable::TryInto;
use stdweb::web::TypedArray;
use stdweb::Value;
use ordered_float::OrderedFloat;


pub const TURN: f32 = ::std::f32::consts::PI*2.0;

pub const DISPLAY_SAMPLE_RATE: f32 = 50.0;

#[derive (Serialize, Deserialize)]
pub enum Waveform {
  Sine,
  Square,
  Triangle,
  Sawtooth,
}

js_serializable! (Waveform) ;
js_deserializable! (Waveform) ;

#[derive (Clone)]
pub struct ControlPoint {
  pub time: f32,
  pub value: f32,
  pub slope: f32,
  pub jump: bool,
  pub value_after_jump: f32,
}

pub struct Signal {
  pub constant: bool,
  pub control_points: Vec<ControlPoint>,
}
pub struct SignalSampler <'a> {
  signal: & 'a Signal,
  next_control_index: usize,
}

pub struct Envelope {
  pub attack: f32,
  pub sustain: f32,
  pub decay: f32,
}

pub struct SoundDefinition {
  pub waveform: Waveform,
  pub envelope: Envelope,
  pub log_frequency: Signal,
  
}

pub struct State {
  sound: SoundDefinition,
}


impl Waveform {
  pub fn sample (&self, phase: f32)->f32 {
    match *self {
      Waveform::Sine => (phase*TURN).sin(),
      Waveform::Square => if phase.fract() < 0.5 {0.5} else {-0.5},
      Waveform::Triangle => 1.0 - (phase.fract()-0.5).abs()*4.0,
      Waveform::Sawtooth => 1.0 - phase.fract()*2.0,
    }
  }
}

impl ControlPoint {
  fn value_after (&self)->f32 {if self.jump {self.value_after_jump} else {self.value}}
}

impl Signal {
  pub fn sampler (&self)->SignalSampler {
    SignalSampler {signal: self, next_control_index: 0}
  }
}

impl<'a> SignalSampler<'a> {
  pub fn sample (&mut self, time: f32)->f32 {
    if self.signal.constant {return self.signal.control_points [0].value;}
    
    while let Some(control) = self.signal.control_points.get (self.next_control_index) {
      if time >= control.time {
        self.next_control_index += 1;
      }
      else {break;}
    }
    
    let previous_control = self.signal.control_points.get (self.next_control_index.wrapping_sub (1));
    let next_control = self.signal.control_points.get (self.next_control_index);
    match (previous_control, next_control) {
      (None, None)=>0.0,
      (None, Some (control)) => {
        control.value + control.slope*(time - control.time)
      },
      (Some (control), None) => {
        control.value_after () + control.slope*(time - control.time)
      },
      (Some (first), Some (second)) => {
        let first_value = first.value_after() + first.slope*(time - first.time);
        let second_value = second.value + second.slope*(time - second.time);
        let fraction = (time - first.time)/(second.time - first.time);
        let adjusted_fraction = fraction*fraction*(3.0 - 2.0*fraction);
        //(((fraction - 0.5)*TURN/2.0).sin() + 1.0)/2.0;
        first_value * (1.0 - adjusted_fraction) + second_value * adjusted_fraction
      },
    }
  }
}

impl Envelope {
  pub fn duration (&self)->f32 {self.attack + self.sustain + self.decay}
  pub fn sample (&self, time: f32)->f32 {
    if time <self.attack {return time/self.attack;}
    if time <self.attack + self.sustain {return 1.0;}
    if time <self.attack + self.sustain + self.decay {return (self.attack + self.sustain + self.decay - time)/self.decay;}
    0.0
  }
}

impl SoundDefinition {
  pub fn duration(&self)->f32 {self.envelope.duration()}
  pub fn render (&self, sample_rate: u32)-> Vec<f32> {
    let num_frames = (self.duration()*sample_rate as f32).ceil() as u32;
    let mut wave_phase = 0.0;
    let mut log_frequency_sampler = self.log_frequency.sampler();
    let frame_duration = 1.0/sample_rate as f32;
    let mut frames = Vec::with_capacity (num_frames as usize);
    
    for index in 0..num_frames {
      let time = index as f32/sample_rate as f32;
      let frequency = log_frequency_sampler.sample(time).exp2();
      wave_phase += frequency*frame_duration;
      let sample = self.waveform.sample (wave_phase)*self.envelope.sample (time)/25.0;
      frames.push (sample) ;
    }
    
    frames
  }
}

  macro_rules! input_callback {
    ([$state: ident, $($args: tt)*] $contents: expr) => {{
      let $state = $state.clone();
      move |$($args)*| {
        { $contents }
        redraw (& $state);
      }
    }};
    ([$state: ident] $contents: expr) => {{
      let $state = $state.clone();
      move || {
        { $contents }
        redraw (& $state);
      }
    }}
  }


fn round_step (input: f32, step: f32)->f32 {(input*step).round()/step}

fn frequency_editor <F: 'static + FnMut (f32)> (state: & State, id: & str, text: & str, current: f32, mut callback: F)->Value {
  let editor = js!{
  return numerical_input ({
    id: @{id},
    text: @{text} + " (Hz)",
    min: 20,
    max: 22050,
    current:@{round_step (current.exp2(), 100.0)},
    step: 1,
    logarithmic: true,
  }, @{move | value: f64 | callback (value.log2() as f32)}
  );
  };
  editor
}

fn time_editor <F: 'static + FnMut (f32)> (state: & State, id: & str, text: & str, current: f32, mut callback: F)->Value {
  let editor = js!{
  return numerical_input ({
    id: @{id},
    text: @{text} + " (s)",
    min: 0.0,
    max: 10.0,
    current:@{round_step (current, 1000.0)},
    step: 0.1,
  }, @{move | value: f64 | callback (value as f32)}
  );
  };
  editor
}

fn add_signal_editor <
  F: 'static + Fn (&State)->&Signal,
  FMut: 'static + Fn (&mut State)->&mut Signal
> (state: &Rc<RefCell<State>>, id: & 'static str, name: & 'static str, get_signal: F, get_signal_mut: FMut) {
  let get_signal = Rc::new (get_signal);
  let get_signal_mut = Rc::new (get_signal_mut);
  let guard = state.borrow();
  let sound = & guard.sound;
  let signal = get_signal (&guard) ;
  let container = js!{ return $("<div>", {id:@{id}, class: "panel"});};
  js!{ $("#panels").append (@{& container});}
  
  let mut sampler = signal.sampler();
  
  js!{@{& container}.append (@{name} + ": ");}
  if !signal.constant {js!{@{& container}.append (@{
    canvas_of_samples (& display_samples (sound, | time | sampler.sample (time)))
  });}}
  
   macro_rules! signal_input {
      ([$signal: ident $($args: tt)*] $effect: expr) => {{
      let get_signal_mut = get_signal_mut.clone();
  input_callback! ([state $($args)*] {
    let mut guard = state.borrow_mut();
    let $signal = get_signal_mut (&mut guard) ;
    $effect
  })
      }}
    }
  
  for (index, control_point) in signal.control_points.iter().enumerate() {
    if signal.constant && index >0 {break;}
    let id = format! ("{}_{}", id, index);
    macro_rules! control_input {
      ([$control: ident $($args: tt)*] $effect: expr) => {{
      let get_signal_mut = get_signal_mut.clone();
  input_callback! ([state $($args)*] {
    let mut guard = state.borrow_mut();
    let signal = get_signal_mut (&mut guard) ;
    let $control = &mut signal.control_points [index];
    $effect
  })
      }}
    }
    
    let control_editor = js!{
      const control_editor = $("<div>");
      @{& container}.append (control_editor);
      return control_editor;
    };
    
    if index >0 {js!{
      @{& control_editor}.append (@{
        time_editor (& guard, & format! ("{}_time", &id), "Time", control_point.time,
          control_input! ([control, value: f32] control.time = value)
        )
      })
    }}
    
    js!{
      var frequency_editor = @{
        frequency_editor (& guard, & format! ("{}_frequency", &id), "Frequency", control_point.value, control_input! ([control, value: f32] control.value = value))
      };
      @{& control_editor}.append (frequency_editor) ;
      if (@{signal.constant}) {
        @{& control_editor}.css ("display", "inline");
        frequency_editor.css ("display", "inline");
      }
    }
    if !signal.constant {js!{
      if (@{index >0}) {
        var jump_editor = @{
          frequency_editor (& guard, & format! ("{}_jump", &id), "Jump to", control_point.value_after_jump, control_input! ([control, value: f32] control.value_after_jump = value))
        };
        @{& control_editor}.append (jump_editor) ;
        var jump_toggle = @{
          control_input! ([control] control.jump = !control.jump)
        };
        jump_editor.prepend (
          $("<input>", {type: "checkbox", checked:@{control_point.jump}}).on ("input", function(event) {
            jump_toggle ();
          })
        );
      }
@{& control_editor}.append (numerical_input ({
  id: @{&id} + "slope",
  text: "Slope (Octaves/second)",
  min: - 100.0,
  max: 100.0,
  current:@{round_step (control_point.slope, 1000.0)},
  step: 1,
}, 
  @{control_input! ([control, value: f64] control.slope = value as f32)}
      )) ;
      
      if (@{index >0}) {
      var delete_callback = @{signal_input! ([signal] {
        signal.control_points.remove (index);
      })};
      @{& control_editor}.append ($("<input>", {
        type: "button",
        id: @{&id} + "delete_control",
        value: "Delete control point"
      }).click (function() {delete_callback()})
      );   
      }   

      var callback = @{signal_input! ([signal] {
        let mut new_point = signal.control_points [index].clone();
        let next = signal.control_points.get (index + 1).cloned();
        new_point.time = match next {
          None => new_point.time + 0.5,
          Some (thingy) => (new_point.time + thingy.time)/2.0,
        };
        signal.control_points.insert (index + 1, new_point);
      })};
      @{& container}.append ($("<input>", {
        type: "button",
        id: @{&id} + "add_control",
        value: "Add control point"
      }).click (function() {callback()})
      );
      
    }}
  }
  
  js!{
    var callback = @{signal_input! ([signal] signal.constant = !signal.constant)};
  @{& container}.append (
    $("<input>", {
      type: "button",
      id: @{&id} + "constant",
      value: @{signal.constant} ? "Complicate" : "Simplify"
    }).click (function() {callback()})
  );}
}

fn display_samples <F: FnMut(f32)->f32> (sound: & SoundDefinition, mut sampler: F)->Vec<f32> {
  let num_samples = (sound.duration()*DISPLAY_SAMPLE_RATE).ceil() as usize + 1;
  (0..num_samples).map (| sample | sampler (sample as f32/DISPLAY_SAMPLE_RATE)).collect()
}

fn canvas_of_samples (samples: & [f32])->Value {
  let canvas = js!{ return document.createElement ("canvas") ;};
  let canvas_height = 100.0;
  let context = js!{
    var canvas = @{& canvas};
    canvas.width = @{samples.len() as f64};
    canvas.height = @{canvas_height};
    var context = canvas.getContext ("2d") ;
    return context;
  };
  
    let min = samples.iter().min_by_key (| value | OrderedFloat (**value)).unwrap() - 0.0001;
    let max = samples.iter().max_by_key (| value | OrderedFloat (**value)).unwrap() + 0.0001;
    let range = max - min;
    
    for (index, sample) in samples.iter().enumerate() {
      js!{
        var context =@{&context};
        var first = @{index as f32 + 0.5};
        var second = @{(max - sample)/range*canvas_height};
        if (@{index == 0}) {
          context.moveTo (first, second);
        } else {
          context.lineTo (first, second);
        }
      }
    }
    
  js!{
    var context =@{&context};
    context.stroke();
  }
  
  canvas
}

fn redraw(state: & Rc<RefCell<State>>) {
  {
  let guard = state.borrow();
  let sound = & guard.sound;
  
  let envelope_samples = display_samples (sound, | time | sound.envelope.sample (time));
  
  js! {
$("#panels").empty();


$("#panels").append ($("<div>", {class: "panel"}).append (radio_input ({
  id: "waveform",
  field: "waveform",
  text: "Waveform",
  current:@{&sound.waveform},
  options: [
    {value: "Sine", text: "Sine"},
    {value: "Square", text: "Square"},
    {value: "Triangle", text: "Triangle"},
    {value: "Sawtooth", text: "Sawtooth"},
  ]
}, 
  @{input_callback! ([state, value: Waveform] {
    state.borrow_mut().sound.waveform = value;
  })}
)));
  
      const envelope_editor = $("<div>", {class: "panel"});
      $("#panels").append (envelope_editor);
      envelope_editor.append (@{canvas_of_samples (&envelope_samples)});
      envelope_editor.append (@{
        time_editor (& guard, "attack", "Attack", sound.envelope.attack,
          input_callback! ([state, value: f32] {state.borrow_mut().sound.envelope.attack = value;})
        )
      });
      envelope_editor.append (@{
        time_editor (& guard, "sustain", "Sustain", sound.envelope.sustain,
          input_callback! ([state, value: f32] {state.borrow_mut().sound.envelope.sustain = value;})
        )
      });
      envelope_editor.append (@{
        time_editor (& guard, "decay", "Decay", sound.envelope.decay,
          input_callback! ([state, value: f32] {state.borrow_mut().sound.envelope.decay = value;})
        )
      });  
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
add_signal_editor (state, "frequency", "Frequency", |state| &state.sound.log_frequency, |state| &mut state.sound.log_frequency);
}


#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  
  let state = Rc::new (RefCell::new (State {
    sound: SoundDefinition {
      waveform: Waveform::Sine,
      envelope: Envelope {attack: 0.1, sustain: 0.5, decay: 0.5},
      log_frequency: Signal {
        constant: true,
        control_points: vec![ControlPoint {time: 0.0, value: 220.0_f32.log2(), slope: 0.0, jump: false, value_after_jump: 0.0}]
      }
    }
  }));
  

  
  redraw(&state);
    
  stdweb::event_loop();
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this natively");
}
