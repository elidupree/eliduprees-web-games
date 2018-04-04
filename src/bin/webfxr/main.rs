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

#[derive (PartialEq, Eq)]
pub enum FrequencyType {
  Frequency,
}
#[derive (PartialEq, Eq)]
pub enum TimeType {
  Seconds,
}
pub trait UserNumberType: Eq {
  fn render (&self, value: & str)->Option<f32>;
}
#[derive (Clone)]
pub struct UserNumber <T: UserNumberType> {
  pub source: String,
  pub rendered: f32,
  pub value_type: T,
}
impl UserNumberType for FrequencyType {
  fn render (&self, value: & str)->Option <f32> {
    match *self {
      FrequencyType::Frequency => f32::from_str (value).ok().map(| value | value.log2())
    }
  }
  fn approximate_from_rendered (&self, rendered: f32)->String {
    match *self {
      FrequencyType::Frequency => format!("{.1}", rendered)
    }
  }
}
impl UserNumberType for TimeType {
  fn render (&self, value: & str)->Option <f32> {
    match *self {
      TimeType::Seconds => f32::from_str (value).ok()
    }
  }
  fn approximate_from_rendered (&self, rendered: f32)->String {
    match *self {
      TimeType::Seconds => format!("{.3}", rendered)
    }
  }
}
impl<T: UserNumberType> UserNumber <T> {
  pub fn new (value_type: T, source: String)->Option <Self> {
    value_type.render (source).map (| rendered | UserNumber {
      source: source, rendered: rendered, value_type: value_type,
    });
  }
}

type UserFrequency = UserNumber <FrequencyType>;
type UserTime = UserNumber <TimeType>;

#[derive (Serialize, Deserialize)]
pub enum Waveform {
  Sine,
  Square,
  Triangle,
  Sawtooth,
}

js_serializable! (Waveform) ;
js_deserializable! (Waveform) ;


pub enum SignalEffect <T: UserNumberType> {
  Jump {time: UserTime, size: UserNumber<T>},
  Slide {start: UserTime, duration: UserTime, size: UserNumber<T>, smooth_start: bool, smooth_stop: bool},
  Oscillation {size: UserNumber<T>, frequency: UserFrequency, waveform: Waveform},
}

pub struct Signal <T: UserNumberType> {
  pub initial_value: UserNumber<T>,
  pub constant: bool,
  pub effects: Vec<SignalEffect <Value>>,
}

pub struct Envelope {
  pub attack: UserTime,
  pub sustain: UserTime,
  pub decay: UserTime,
}

pub struct SoundDefinition {
  pub waveform: Waveform,
  pub envelope: Envelope,
  pub log_frequency: Signal,
  pub log_bitcrush_frequency: Signal,
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

impl<T: UserNumberType> SignalEffect <T> {
  pub fn sample (&mut self, sample_time: f32)->f32 {
    match *self {
      SignalEffect::Jump {time, size} => if sample_time > time {size.rendered} else {0.0},
      SignalEffect::Slide {start, duration, size, smooth_start, smooth_stop} => {
        if sample_time <start.rendered {0.0}
        else if sample_time >start.rendered + duration.rendered {size.rendered}
        else {
          let fraction = (sample_time - start.rendered)/duration.rendered;
          let adjusted_fraction = match (smooth_start, smooth_stop) {
            (false, false) => fraction,
            (true, false) => fraction*fraction,
            (false, true) => fraction*(2.0-fraction),
            (true, true) => fraction*fraction*(3.0 - 2.0*fraction),
          };
          size.rendered*adjusted_fraction
        }
      },
      SignalEffect::Oscillation {size, frequency, waveform} => size.rendered*waveform.sample (sample_time*frequency.rendered),
    }
  }
}

impl<T: UserNumberType> Signal<T> {
  pub fn constant(value: UserNumber <T>)->Signal {
    Signal {
      constant: true,
      initial_value: value,
      effects: Vec::new(),
    }
  }

  pub fn sample (&mut self, time: f32)->f32 {
    let mut result = self.initial_value.rendered;
    if self.signal.constant {
      self.initial_value.rendered
    }
    else {
      self.initial_value.rendered + self.effects.iter().map (| effect | effect.sample (time)).sum()
    }
  }
}

impl Envelope {
  pub fn duration (&self)->f32 {self.attack.rendered + self.sustain.rendered + self.decay.rendered}
  pub fn sample (&self, time: f32)->f32 {
    if time <self.attack.rendered {return time/self.attack.rendered;}
    if time <self.attack.rendered + self.sustain.rendered {return 1.0;}
    if time <self.attack.rendered + self.sustain.rendered + self.decay.rendered {return (self.attack.rendered + self.sustain.rendered + self.decay.rendered - time)/self.decay.rendered;}
    0.0
  }
}

impl SoundDefinition {
  pub fn duration(&self)->f32 {self.envelope.duration()}
  pub fn render (&self, sample_rate: u32)-> Vec<f32> {
    let num_frames = (self.duration()*sample_rate as f32).ceil() as u32;
    let mut wave_phase = 0.0;
    let mut bitcrush_phase = 1.0;
    let mut last_used_sample = 0.0;
    let frame_duration = 1.0/sample_rate as f32;
    let mut frames = Vec::with_capacity (num_frames as usize);
    
    for index in 0..num_frames {
      let time = index as f32/sample_rate as f32;
      
      let sample = self.waveform.sample (wave_phase)*self.envelope.sample (time)/25.0;
      if bitcrush_phase >= 1.0 {
        bitcrush_phase -= 1.0;
        if bitcrush_phase >1.0 {bitcrush_phase = 1.0;}
        last_used_sample = sample; 
      }
      frames.push (last_used_sample) ;
      
      let frequency = self.log_frequency.sample(time).exp2();
      let bitcrush_frequency = self.log_bitcrush_frequency.sample(time).exp2();
      wave_phase += frequency*frame_duration;
      bitcrush_phase += bitcrush_frequency*frame_duration;
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

pub struct NumericalInputSpecification <'a, T: UserNumberType> {
  id: & 'a str,
  name: & 'a str,
  slider_range: [f32; 2],
  hard_range: Option <[f32; 2]>,
  value_type: T,
  current_value: UserNumber <T>,
}

impl <'a, T: UserNumberType> NumericalInputSpecification<'a, T> {
  pub fn render (&self)->Value {
    let displayed_value = if self.value_type == current_value.value_type {current_value.source.clone()} else {self.value_type.approximate_from_rendered (current_value.rendered)};
    let (range_range, range_value, range_step) = if self.slider_logarithmic {
      ([0.0, 1.0], current_value.rendered
    }
    js!{
      
      range_input = $("<input>", {type: "range", id: @{self.id}+"_numerical_range", value:@{self.current_value.rendered}, min:@{self.slider_range [0]}, max:@{self.slider_range [1]}, step: data.step });
      number_input = $("<input>", {type: "number", id: data.id+"_numerical_number", value:@{displayed_value}, step: data.step });
      
      var result = $("<div>", {class: "labeled_input"}).append (
        range_input.on ("input", range_overrides),
        number_input.on ("input", number_overrides),
        $("<label>", {"for": data.id+"_numerical_number", text:@{}})
  ).on("wheel", function (event) {
    let value = number_input[0].valueAsNumber;
    value += (Math.sign(event.originalEvent.deltaY) || Math.sign(event.originalEvent.deltaX) || 0)*data.step;
    number_input.val (value);
    number_overrides ();
    event.preventDefault();
  });
      return result;
    }
  }
}

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
        jump_editor.prepend (
          $("<input>", {type: "checkbox", checked:@{control_point.jump}}).on ("input", function(event) {
            @{control_input! ([control] control.jump = !control.jump)}();
          })
        );
      }
@{& control_editor}.append (numerical_input ({
  id: @{&id} + "slope",
  text: "Slope (Octaves/second)",
  min: - 10.0,
  max: 10.0,
  current:@{round_step (control_point.slope, 1000.0)},
  step: 0.01,
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
        let previous = signal.control_points [index].clone();
        let next = signal.control_points.get (index + 1).cloned();
        let time = match next {
          None => previous.time + 0.5,
          Some (thingy) => (previous.time + thingy.time)/2.0,
        };
        let value = signal.sampler().sample(time);
        let offset = 0.000001;
        let offset_value = signal.sampler().sample (time + offset) ;
        let slope = (offset_value - value)/offset;
        signal.control_points.insert (index + 1, ControlPoint {
          time: time, value: value, slope: slope,
          jump: false, value_after_jump: value,
        });
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
  add_signal_editor (state, "bitcrush_frequency", "Bitcrush frequency", |state| &state.sound.log_bitcrush_frequency, |state| &mut state.sound.log_bitcrush_frequency);
}


#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  
  let state = Rc::new (RefCell::new (State {
    sound: SoundDefinition {
      waveform: Waveform::Sine,
      envelope: Envelope {attack: 0.1, sustain: 0.5, decay: 0.5},
      log_frequency: Signal::constant (220.0_f32.log2()),
      log_bitcrush_frequency: Signal::constant (44100.0_f32.log2()),
    }
  }));
  

  
  redraw(&state);
    
  stdweb::event_loop();
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this natively");
}
