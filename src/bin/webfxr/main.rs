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

pub struct ControlPoint {
  pub time: f32,
  pub value: f32,
  pub slope: f32,
  pub jump: f32,
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
      (None, Some (control)) | (Some (control), None) => {
        control.value + control.slope*(time - control.time)
      },
      (Some (first), Some (second)) => {
        let first_value = first.value + first.slope*(time - first.time);
        let second_value = second.value + second.slope*(time - second.time);
        let fraction = (time - first.time)/(second.time - first.time);
        first_value * (1.0 - fraction) + second_value * fraction
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


fn frequency_editor <F: 'static + FnMut (f32)> (state: & State, id: & str, text: & str, current: f32, mut callback: F)->Value {
  let editor = js!{
  return numerical_input ({
    id: @{id},
    text: @{text} + " (Hz)",
    min: 20,
    max: 22050,
    current:@{(current.exp2()*100.0).round()/100.0},
    step: 1,
    logarithmic: true,
  }, @{move | value: f64 | callback (value.log2() as f32)}
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
      @{& control_editor}.append (numerical_input ({
  id: @{&id} + "time",
  text: "Time (seconds)",
  min: 0.0,
  max: 10.0,
  current:@{control_point.time},
  step: 0.01,
}, 
  @{control_input! ([control, value: f64] control.time = value as f32)}
      )) ;
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
@{& control_editor}.append (numerical_input ({
  id: @{&id} + "slope",
  text: "Slope (Octaves/second)",
  min: - 100.0,
  max: 100.0,
  current:@{control_point. slope},
  step: 1,
}, 
  @{control_input! ([control, value: f64] control.slope = value as f32)}
      )) ;
    }}
  }
  
  js!{
    var callback = @{signal_input! ([signal] signal.constant = !signal.constant)};
  @{& container}.append (
    $("<input>", {
      type: "button",
      id: @{&id} + "constant",
      value: @{signal.constant} ? "Complicate" : "Simplify"
    }).click (
      function() {callback()}
    )
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
      envelope_editor.append (numerical_input ({
  id: "attack",
  text: "Attack (s)",
  min: 0.0,
  max: 3.0,
  current:@{sound.envelope.attack},
  step: 0.01,
}, @{input_callback! ([state, value: f64] {
    state.borrow_mut().sound.envelope.attack = value as f32;
  })}));
  envelope_editor.append (numerical_input ({
  id: "sustain",
  text: "Sustain (s)",
  min: 0.0,
  max: 3.0,
  current:@{sound.envelope.sustain},
  step: 0.01,
}, @{input_callback! ([state, value: f64] {
    state.borrow_mut().sound.envelope.sustain= value as f32;
  })}));
  envelope_editor.append (numerical_input ({
  id: "decay",
  text: "Decay (s)",
  min: 0.0,
  max: 3.0,
  current:@{sound.envelope.decay},
  step: 0.01,
}, @{input_callback! ([state, value: f64] {
    state.borrow_mut().sound.envelope.decay= value as f32;
  })}));
  
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
        control_points: vec![ControlPoint {time: 0.0, value: 220.0_f32.log2(), slope: 0.0, jump: 0.0}]
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
