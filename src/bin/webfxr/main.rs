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
extern crate ordered_float;

use std::rc::Rc;
use std::cell::RefCell;
use std::str::FromStr;
use stdweb::unstable::{TryInto, TryFrom};
use stdweb::web::TypedArray;
use stdweb::{JsSerialize, Value};
use serde::{Serialize};
use serde::de::DeserializeOwned;
use ordered_float::OrderedFloat;


pub const TURN: f32 = ::std::f32::consts::PI*2.0;

pub const DISPLAY_SAMPLE_RATE: f32 = 50.0;

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative (Default)]
pub enum FrequencyType {
  #[derivative (Default)]
  Frequency,
}
#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative (Default)]
pub enum IntervalType {
  #[derivative (Default)]
  Ratio,
}
#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative (Default)]
pub enum TimeType {
  #[derivative (Default)]
  Seconds,
}
pub trait UserNumberType: 'static + Clone + Eq + Serialize + DeserializeOwned + Default {
  type DifferenceType: UserNumberType;
  fn render (&self, value: & str)->Option<f32>;
  fn approximate_from_rendered (&self, rendered: f32)->String;
  fn unit_name (&self)->&'static str;
  fn currently_used (_state: & State)->Self {Self::default()}
}
#[derive (Clone, Serialize, Deserialize)]
pub struct UserNumber <T: UserNumberType> {
  pub source: String,
  pub rendered: f32,
  // Hacky workaround for https://github.com/rust-lang/rust/issues/41617 (see https://github.com/serde-rs/serde/issues/943)
  #[serde(deserialize_with = "::serde::Deserialize::deserialize")]
  pub value_type: T,
}
impl UserNumberType for FrequencyType {
  type DifferenceType = IntervalType;
  fn render (&self, value: & str)->Option <f32> {
    match *self {
      FrequencyType::Frequency => f32::from_str (value).ok().and_then(| value | {
        let value = value.log2();
        if value.is_finite() {Some (value)} else {None}
      })
    }
  }
  fn approximate_from_rendered (&self, rendered: f32)->String {
    match *self {
      FrequencyType::Frequency => format!("{:.1}", rendered.exp2())
    }
  }
  fn unit_name (&self)->&'static str {
    match *self {
      FrequencyType::Frequency => "Hz"
    }
  }
}
impl UserNumberType for IntervalType {
  type DifferenceType = Self;
  fn render (&self, value: & str)->Option <f32> {
    match *self {
      IntervalType::Ratio => f32::from_str (value).ok().and_then(| value | {
        let value = value.log2();
        if value.is_finite() {Some (value)} else {None}
      })
    }
  }
  fn approximate_from_rendered (&self, rendered: f32)->String {
    match *self {
      IntervalType::Ratio => format!("{:.2}", rendered.exp2())
    }
  }
  fn unit_name (&self)->&'static str {
    match *self {
      IntervalType::Ratio => "ratio"
    }
  }
}
impl UserNumberType for TimeType {
  type DifferenceType = Self;
  fn render (&self, value: & str)->Option <f32> {
    match *self {
      TimeType::Seconds => f32::from_str (value).ok().filter (| value | value.is_finite())
    }
  }
  fn approximate_from_rendered (&self, rendered: f32)->String {
    match *self {
      TimeType::Seconds => format!("{:.3}", rendered)
    }
  }
  fn unit_name (&self)->&'static str {
    match *self {
      TimeType::Seconds => "s"
    }
  }
}
impl<T: UserNumberType> UserNumber <T> {
  pub fn new (value_type: T, source: String)->Option <Self> {
    value_type.render (&source).map (| rendered | UserNumber {
      source: source, rendered: rendered, value_type: value_type,
    })
  }
  pub fn from_rendered (rendered: f32)->Self {
    let value_type = T::default() ;
    UserNumber {
      source: value_type.approximate_from_rendered (rendered),
      rendered: rendered, value_type: value_type,
    }
  }
}


//js_serializable! (UserNumber) ;
//js_deserializable! (UserNumber) ;

type UserFrequency = UserNumber <FrequencyType>;
type UserTime = UserNumber <TimeType>;

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Waveform {
  Sine,
  Square,
  Triangle,
  Sawtooth,
}

js_serializable! (Waveform) ;
js_deserializable! (Waveform) ;


pub enum SignalEffect <T: UserNumberType> {
  Jump {time: UserTime, size: UserNumber<T::DifferenceType>},
  Slide {start: UserTime, duration: UserTime, size: UserNumber<T::DifferenceType>, smooth_start: bool, smooth_stop: bool},
  Oscillation {size: UserNumber<T::DifferenceType>, frequency: UserFrequency, waveform: Waveform},
}

pub struct Signal <T: UserNumberType> {
  pub initial_value: UserNumber<T>,
  pub constant: bool,
  pub effects: Vec<SignalEffect <T>>,
}

pub struct Envelope {
  pub attack: UserTime,
  pub sustain: UserTime,
  pub decay: UserTime,
}

pub struct SoundDefinition {
  pub waveform: Waveform,
  pub envelope: Envelope,
  pub log_frequency: Signal <FrequencyType>,
  pub log_bitcrush_frequency: Signal <FrequencyType>,
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
  pub fn sample (&self, sample_time: f32)->f32 {
    match self.clone() {
      SignalEffect::Jump {time, size} => if sample_time > time.rendered {size.rendered} else {0.0},
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
  pub fn constant(value: UserNumber <T>)->Self {
    Signal {
      constant: true,
      initial_value: value,
      effects: Vec::new(),
    }
  }

  pub fn sample (&self, time: f32)->f32 {
    if self.constant {
      self.initial_value.rendered
    }
    else {
      self.initial_value.rendered + self.effects.iter().map (| effect | effect.sample (time)).sum::<f32>()
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


#[derive (Derivative)]
#[derivative (Clone (bound =""))]
pub struct Getter <T, U> {
  get: Rc <Fn(&T)->&U>,
  get_mut: Rc <Fn(&mut T)->&mut U>,
}
impl <T, U> Getter <T, U> {
  fn get<'a, 'b> (&'a self, value: &'b T)->&'b U {
    (self.get) (value)
  }
  fn get_mut<'a, 'b> (&'a self, value: &'b mut T)->&'b mut U {
    (self.get_mut) (value)
  }
}

impl <T: 'static,U: 'static,V: 'static> ::std::ops::Add<Getter <U, V>> for Getter <T, U> {
  type Output = Getter <T, V>;
  fn add (self, other: Getter <U, V>)->Self::Output {
    let my_get = self.get;
    let my_get_mut = self.get_mut;
    let other_get = other.get;
    let other_get_mut = other.get_mut;
    Getter {
      get: Rc::new (move | value | (other_get) ((my_get) (value))),
      get_mut: Rc::new (move | value | (other_get_mut) ((my_get_mut) (value))),
    }
  }
}

macro_rules! getter {
  ($value: ident => $($path:tt)*) => {
    Getter {
      get    : Rc::new (move | $value | &    $($path)*),
      get_mut: Rc::new (move | $value | &mut $($path)*),
    }
  }
}
macro_rules! variant_field_getter {
  ($Enum: ident::$Variant: ident => $field: ident) => {
    Getter {
      get    : Rc::new (| value | match value {
        &    $Enum::$Variant {ref     $field,..} => $field,
        _ => unreachable!(),
      }),
      get_mut: Rc::new (| value | match value {
        &mut $Enum::$Variant {ref mut $field,..} => $field,
        _ => unreachable!(),
      }),
    }
  }
}


pub fn input_callback<T, F> (state: &Rc<RefCell<State>>, callback: F)->impl (Fn (T)->bool)
  where
    F: Fn(&mut State, T)->bool {
  let state = state.clone();
  move |arg: T| {
    let success = (callback)(&mut state.borrow_mut(), arg);
    if success {
      redraw (&state);
    }
    success
  }
}

pub fn input_callback_nullary<F> (state: &Rc<RefCell<State>>, callback: F)->impl (Fn ()->bool)
  where
    F: Fn(&mut State)->bool {
  let hack = input_callback (state, move | state,()| (callback)(state));
  move || {
    (hack)(())
  }
}


pub fn button_input<F: 'static + Fn()->bool> (id: & str, name: & str, callback: F)->Value {
  let result: Value = js!{
    $("<input>", {
      type: "button",
      id:@{id},
      value: @{name}
    }).click (function() {@{callback}();})
  };
  result
}

//fn round_step (input: f32, step: f32)->f32 {(input*step).round()/step}

pub struct RadioInputSpecification <'a, T: 'a, F> {
  state: & 'a Rc<RefCell<State>>,
  id: & 'a str,
  name: & 'a str,
  options: & 'a [(T, & 'a str)],
  current_value: T,
  input_callback: F,
}

impl <'a, F: 'static + Fn (T)->bool, T: Eq> RadioInputSpecification <'a, T, F>
  where
    T: JsSerialize,
    T: TryFrom<Value>,
    Value: TryInto<T>,
    <Value as TryInto<T>>::Error: ::std::fmt::Debug {
  pub fn render (self)->Value {
    let result = js!{return $("<div>", {class: "labeled_input"}).append (
      $("<label>", {text:@{self.name} + ":"})
    );};
    
    let update = js!{
      return function (value) {@{self.input_callback}(value)}
    };
    
    for &(ref value, name) in self.options.iter() {
      js!{
        function choice_overrides() {
          var value = $("input:radio[name="+@{self.id}+"_radios]:checked").val();
          @{&update}(value);
        }
        @{&result}.append (
          $("<input>", {type: "radio", id: @{self.id}+"_radios_" + @{value}, name: @{self.id}+"_radios", value: @{value}, checked: @{*value == self.current_value}}).click (choice_overrides),
          $("<label>", {"for": @{self.id}+"_radios_" + @{value}, text: @{name}})
        );
      }
    }
    
    result
  }
}

pub struct NumericalInputSpecification <'a, T: UserNumberType, F> {
  state: & 'a Rc<RefCell<State>>,
  id: & 'a str,
  name: & 'a str,
  slider_range: [f32; 2],
  current_value: UserNumber <T>,
  input_callback: F,
}

impl <'a, F: 'static + Fn (UserNumber <T>)->bool, T: UserNumberType> NumericalInputSpecification<'a, T, F> {
  pub fn render (self)->Value {
    let value_type = T::currently_used (&self.state.borrow());
    let displayed_value = if value_type == self.current_value.value_type {self.current_value.source.clone()} else {value_type.approximate_from_rendered (self.current_value.rendered)};
    let slider_step = (self.slider_range [1] - self.slider_range [0])/1000.0;
    let input_callback = self.input_callback;
    let update_callback = { let value_type = value_type.clone(); move | value: String |{
          if let Some(value) = UserNumber::new (value_type.clone(), value) {
            if (input_callback)(value) {
              return true;
            }
          }
          false
        }};
    let update = js!{return _.debounce(function(value) {
        var success = @{update_callback} (value);
        if (!success) {
          // TODO display some sort of error message
        }
      }, 200);};
    let range_input = js!{return $("<input>", {type: "range", id: @{self.id}+"_numerical_range", value:@{self.current_value.rendered}, min:@{self.slider_range [0]}, max:@{self.slider_range [1]}, step:@{slider_step} });};
    let number_input = js!{return $("<input>", {type: "number", id: @{self.id}+"_numerical_number", value:@{displayed_value}});};
    
    let range_overrides = js!{return function () {
        var value = @{&range_input}[0].valueAsNumber;
        var source = @{{let value_type = value_type.clone(); move | value: f64 | value_type.approximate_from_rendered (value as f32)}} (value);
        // immediately update the number input with the range input, even though the actual data editing is debounced.
        @{&number_input}.val(source);
        @{&update}(source);
      }
;};
    let number_overrides = js!{return function () {
        @{&update}(@{&number_input}.val());
      }
;};
    
    
    let result: Value = js!{
      var result = $("<div>", {class: "labeled_input"}).append (
        @{&range_input}.on ("input", @{&range_overrides}),
        @{&number_input}.on ("input", @{&number_overrides}),
        $("<label>", {"for": @{self.id}+"_numerical_number", text:@{
          format! ("{} ({})", self.name, value_type.unit_name())
        }})
      );
      
      @{&range_input}.val(@{self.current_value.rendered});
      
      result.on("wheel", function (event) {
        var value = @{&range_input}[0].valueAsNumber;
        value += (-Math.sign(event.originalEvent.deltaY) || Math.sign(event.originalEvent.deltaX) || 0)*@{slider_step*50.0};
        @{&range_input}.val (value);
        @{&range_overrides}();
        event.preventDefault();
      });
      return result;
    };
    result
  }
}


pub struct SignalEditorSpecification <'a, T: UserNumberType> {
  state: & 'a Rc<RefCell<State>>,
  id: & 'a str,
  name: & 'a str,
  slider_range: [f32; 2],
  difference_slider_range: [f32; 2],
  getter: Getter <State, Signal <T>>
}

impl <'a, T: UserNumberType> SignalEditorSpecification <'a, T> {
  pub fn numeric_input <U: UserNumberType> (&self, id: & str, name: & str, slider_range: [f32; 2], getter: Getter <State, UserNumber <U>>)->Value {
    let current_value = getter.get (&self.state.borrow()).clone();
    NumericalInputSpecification {
      state: self.state,
      id: id,
      name: name, 
      slider_range: slider_range,
      current_value: current_value,
      input_callback: input_callback (self.state, move | state, value: UserNumber <U> | {
        *getter.get_mut (state) = value;
        true
      }),
    }.render()
  }

  pub fn time_input (&self, id: & str, name: & str, getter: Getter <State, UserTime>)->Value {
    self.numeric_input (id, name, [0.0, 3.0], getter)
  }
  
  pub fn value_input (&self, id: & str, name: & str, getter: Getter <State, UserNumber <T>>)->Value {
    self.numeric_input (id, name, self.slider_range, getter)
  }
  
  pub fn difference_input (&self, id: & str, name: & str, getter: Getter <State, UserNumber <T::DifferenceType>>)->Value {
    self.numeric_input (id, name, self.difference_slider_range, getter)
  }


  pub fn render (self) {
    
      let guard = self.state.borrow();
    let signal = self.getter.get (& guard);
      
  let container = js!{ return $("<div>", {id:@{self.id}, class: "panel"});};
  js!{ $("#panels").append (@{& container});}
  
  
  js!{@{& container}.append (@{self.name} + ": ");}
  if !signal.constant {js!{@{& container}.append (@{
    canvas_of_samples (& display_samples (& guard.sound, | time | signal.sample (time)))
  });}}
  
  let initial_value_input = self.value_input (
    & format! ("{}_initial", & self.id),
    if signal.constant {self.name} else {"Initial value"}, 
    self.getter.clone() + getter! (signal => signal.initial_value)
  );
  
  js!{@{& container}.append (@{initial_value_input})}
  
  
    
  for (index, effect) in signal.effects.iter().enumerate() {
    let effect_getter = self.getter.clone() + getter!(signal => signal.effects [index]);

    match *effect {
      SignalEffect::Jump {..} => {
        let time_input = self.time_input (
          & format! ("{}_{}_time", & self.id, index),
          "Time",
          effect_getter.clone() + variant_field_getter! (SignalEffect::Jump => time)
        );
        let size_input = self.difference_input (
          & format! ("{}_{}_size", & self.id, index),
          "Size",
          effect_getter.clone() + variant_field_getter! (SignalEffect::Jump => size)
        );
        js!{@{& container}.append (@{time_input},@{size_input})}
      }
      _=>(),
    }
  }
  
  let toggle_constant_button = button_input (
    & format! ("{}_toggle_constant", & self.id),
    if signal.constant {"Complicate"} else {"Simplify"},
    {
      let getter = self.getter.clone();
      input_callback_nullary (self.state, move | state | {
        let signal = getter.get_mut (state);
        signal.constant = !signal.constant;
        true
      })
    }
  );
  
  
  js!{ @{& container}.append (@{toggle_constant_button}); }
  
  if !signal.constant {
    let add_jump_button = button_input (
    & format! ("{}_add_jump", & self.id),
    "Add jump",
    {
      let getter = self.getter.clone();
      let range = self.difference_slider_range;
      input_callback_nullary (self.state, move | state | {
        getter.get_mut (state).effects.push (SignalEffect::Jump {time: UserTime::from_rendered (0.5), size: UserNumber::from_rendered (range [1])});
        true
      })
    }
  );
    js!{ @{& container}.append (@{add_jump_button}); }

  }
  
  }
}
  
  
  /*
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
  


      
      
      
    });
  }
}

fn add_signal_editor <T> () {
  let get_signal = Rc::new (get_signal);
  let get_signal_mut = Rc::new (get_signal_mut);
  let guard = state.borrow();
  let sound = & guard.sound;
  let signal = get_signal (&guard) ;}*/

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
  
  let waveform_input = RadioInputSpecification {
    state: state, id: "waveform", name: "Waveform",
    options: &[
      (Waveform::Sine, "Sine"),
      (Waveform::Square, "Square"),
      (Waveform::Triangle, "Triangle"),
      (Waveform::Sawtooth, "Sawtooth"), 
    ],
    current_value: sound.waveform.clone(),
    input_callback: input_callback (state, move | state, value: Waveform | {
      state.sound.waveform = value;
      true
    }),
  }.render();
  
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
    id: "bitcrush_frequency",
    name: "Bitcrush frequency",
    slider_range: [20f32.log2(), 48000f32.log2()],
    difference_slider_range: [-5.0, 5.0],
    getter: getter! (state => state.sound.log_bitcrush_frequency),
  }.render();

  
  //add_signal_editor (state, "frequency", "Frequency", |state| &state.sound.log_frequency, |state| &mut state.sound.log_frequency);
  //add_signal_editor (state, "bitcrush_frequency", "Bitcrush frequency", |state| &state.sound.log_bitcrush_frequency, |state| &mut state.sound.log_bitcrush_frequency);
}


#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  
  let state = Rc::new (RefCell::new (State {
    sound: SoundDefinition {
      waveform: Waveform::Sine,
      envelope: Envelope {attack: UserNumber::from_rendered (0.1), sustain: UserNumber::from_rendered (0.5), decay: UserNumber::from_rendered (0.5)},
      log_frequency: Signal::constant (UserNumber::from_rendered (220.0_f32.log2())),
      log_bitcrush_frequency: Signal::constant (UserNumber::from_rendered (44100.0_f32.log2())),
    }
  }));
  

  
  redraw(&state);
    
  stdweb::event_loop();
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this natively");
}
