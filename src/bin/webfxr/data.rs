use std::rc::Rc;
use std::str::FromStr;
use serde::{Serialize};
use serde::de::DeserializeOwned;

use super::*;


pub const DISPLAY_SAMPLE_RATE: f64 = 50.0;

pub fn min (first: f64, second: f64)->f64 {if first < second {first} else {second}}
pub fn max (first: f64, second: f64)->f64 {if first > second {first} else {second}}

#[derive (Derivative)]
#[derivative (Clone (bound =""))]
pub struct Getter <T, U> {
  pub get: Rc <Fn(&T)->&U>,
  pub get_mut: Rc <Fn(&mut T)->&mut U>,
}
impl <T, U> Getter <T, U> {
  pub fn get<'a, 'b> (&'a self, value: &'b T)->&'b U {
    (self.get) (value)
  }
  pub fn get_mut<'a, 'b> (&'a self, value: &'b mut T)->&'b mut U {
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

macro_rules! zero_information_number_type {
  ($Enum: ident, $Variant: ident, $name: expr, | $value: ident | $render: expr, | $rendered: ident | $from_rendered: expr) => {

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative (Default)]
pub enum $Enum {
  #[derivative (Default)]
  $Variant,
}

impl UserNumberType for $Enum {
  type DifferenceType = IntervalType;
  fn render (&self, value: & str)->Option <f64> {
    let $value = match f64::from_str (value).ok().filter (| value | value.is_finite()) {
      None => return None,
      Some(a) => a,
    };
    match *self {
      $Enum::$Variant => $render
    }
  }
  fn approximate_from_rendered (&self, $rendered: f64)->String {
    match *self {
      $Enum::$Variant => $from_rendered
    }
  }
  fn unit_name (&self)->&'static str {
    match *self {
      $Enum::$Variant => $name
    }
  }
}

  }
}


pub const OCTAVES_TO_DECIBELS: f64 = 3.0102999;
pub const DEFAULT_DECIBEL_BASE: f64 = -40.0;

zero_information_number_type!{
  FrequencyType, Frequency, "Hz",
  | value | {
    let value = value.log2();
    if value.is_finite() {Some (value)} else {None}
  },
  | rendered | format!("{:.1}", rendered.exp2())
}
zero_information_number_type!{
  IntervalType, Ratio, "ratio",
  | value | {
    let value = value.log2();
    if value.is_finite() {Some (value)} else {None}
  },
  | rendered | format!("{:.2}", rendered.exp2())
}
zero_information_number_type!{
  TimeType, Seconds, "s",
  | value | Some (value),
  | rendered | format!("{:.3}", rendered)
}
zero_information_number_type!{
  VolumeDifferenceType, Decibels, "dB",
  | value | {
    let value = value/OCTAVES_TO_DECIBELS;
    if value.is_finite() {Some (value)} else {None}
  },
  | rendered | format!("{:.1}", rendered*OCTAVES_TO_DECIBELS)
}

#[derive (Clone, PartialEq, Serialize, Deserialize, Derivative)]
#[derivative (Default)]
pub enum VolumeType {
  #[derivative (Default)]
  DecibelsAbove(#[derivative (Default (value = "DEFAULT_DECIBEL_BASE"))] f64),
}

pub trait UserNumberType: 'static + Clone + PartialEq + Serialize + DeserializeOwned + Default {
  type DifferenceType: UserNumberType;
  fn render (&self, value: & str)->Option<f64>;
  fn approximate_from_rendered (&self, rendered: f64)->String;
  fn unit_name (&self)->&'static str;
  fn currently_used (_state: & State)->Self {Self::default()}
}
#[derive (Clone, Serialize, Deserialize)]
pub struct UserNumber <T: UserNumberType> {
  pub source: String,
  pub rendered: f64,
  // Hacky workaround for https://github.com/rust-lang/rust/issues/41617 (see https://github.com/serde-rs/serde/issues/943)
  #[serde(deserialize_with = "::serde::Deserialize::deserialize")]
  pub value_type: T,
}

impl UserNumberType for VolumeType {
  type DifferenceType = VolumeDifferenceType;
  fn render (&self, value: & str)->Option <f64> {
    match *self {
      VolumeType::DecibelsAbove(base) => f64::from_str (value).ok().and_then(| value | {
        let value = (value + base)/OCTAVES_TO_DECIBELS;
        if value.is_finite() {Some (value)} else {None}
      })
    }
  }
  fn approximate_from_rendered (&self, rendered: f64)->String {
    match *self {
      VolumeType::DecibelsAbove(base) => format!("{:.1}", rendered*OCTAVES_TO_DECIBELS - base)
    }
  }
  fn unit_name (&self)->&'static str {
    match *self {
      VolumeType::DecibelsAbove(_) => "dB"
    }
  }
}

impl<T: UserNumberType> UserNumber <T> {
  pub fn new (value_type: T, source: String)->Option <Self> {
    value_type.render (&source).map (| rendered | UserNumber {
      source: source, rendered: rendered, value_type: value_type,
    })
  }
  pub fn from_rendered (rendered: f64)->Self {
    let value_type = T::default() ;
    Self::new (value_type.clone(), value_type.approximate_from_rendered (rendered)).unwrap()
  }
}
impl<T: UserNumberType> Default for UserNumber <T> {
  fn default ()->Self {
    Self::from_rendered(1.0)
  }
}


//js_serializable! (UserNumber) ;
//js_deserializable! (UserNumber) ;

pub type UserFrequency = UserNumber <FrequencyType>;
pub type UserTime = UserNumber <TimeType>;

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative (Default)]
pub enum Waveform {
  #[derivative (Default)]
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

#[derive (Default)]
pub struct Signal <T: UserNumberType> {
  pub initial_value: UserNumber<T>,
  pub effects: Vec<SignalEffect <T>>,
}

#[derive (Default)]
pub struct Envelope {
  pub attack: UserTime,
  pub sustain: UserTime,
  pub decay: UserTime,
}

#[derive (Clone)]
pub struct SignalInfo {
  pub id: & 'static str,
  pub name: & 'static str,
  pub slider_range: [f64; 2],
  pub difference_slider_range: f64,
  pub average_effects: f64,
}

#[derive (Clone)]
pub struct TypedSignalInfo<T: UserNumberType> {
  pub untyped: SignalInfo,
  pub getter: Getter <State, Signal <T>>,
  pub rendered_getter: Rc<Fn(& State)->Option <& RenderedSamples>>,
}

pub trait SignalVisitor {
  fn visit <T: UserNumberType> (&mut self, info: & TypedSignalInfo <T>, signal: & Signal <T>);
}

pub trait SignalVisitorMut {
  fn visit_mut <T: UserNumberType> (&mut self, info: & TypedSignalInfo <T>, signal: &mut Signal <T>);
}

macro_rules! signals_definitions {
  ($(($field: ident, $NumberType: ident, $rendered_field: expr, $info: expr),)*) => {
    impl SoundDefinition {
      pub fn signals_static_info()->Vec<SignalInfo> {
        vec![
          $($info,)*
        ]
      }
      pub fn visit_callers <T: SignalVisitor> (&self)->Vec<Box<Fn(&mut T, &SoundDefinition)>> {
        vec![
          $(Box::new (| visitor, sound | visitor.visit (& TypedSignalInfo::$field(), &sound.$field)),)*
        ]
      }
      pub fn visit_mut_callers <T: SignalVisitorMut> (&self)->Vec<Box<Fn(&mut T, &mut SoundDefinition)>> {
        vec![
          $(Box::new (| visitor, sound | visitor.visit_mut (& TypedSignalInfo::$field(), &mut sound.$field)),)*
        ]
      }
    }
    
    impl SignalInfo {
      $(pub fn $field ()->Self {
        $info
      })*
    }
    $(impl TypedSignalInfo <$NumberType> {
      pub fn $field ()->Self {
        TypedSignalInfo {
          untyped: $info,
          getter: getter! (state => state.sound.$field),
          rendered_getter: Rc::new ($rendered_field),
        }
      }
    })*
  }
}

#[derive (Default)]
pub struct SoundDefinition {
  pub waveform: Waveform,
  pub envelope: Envelope,
  pub log_frequency: Signal <FrequencyType>,
  pub volume: Signal <VolumeType>,
  pub log_lowpass_filter_cutoff: Signal <FrequencyType>,
  pub log_highpass_filter_cutoff: Signal <FrequencyType>,
  pub log_bitcrush_frequency: Signal <FrequencyType>,
}

signals_definitions! {
  (log_frequency, FrequencyType, |state| Some(&state.rendering_state.after_frequency), SignalInfo {
    id: "frequency",
    name: "Frequency",
    slider_range: [20f64.log2(), 5000f64.log2()],
    difference_slider_range: 2.0,
    average_effects: 2.0,
  }),
  (volume, VolumeType, |state| Some(&state.rendering_state.after_volume), SignalInfo {
    id: "volume",
    name: "Volume",
    slider_range: [DEFAULT_DECIBEL_BASE/OCTAVES_TO_DECIBELS,0.0],
    difference_slider_range: 2.0,
    average_effects: 0.7,
  }),
  (log_lowpass_filter_cutoff, FrequencyType, |state| Some(&state.rendering_state.after_lowpass), SignalInfo {
    id: "lowpass",
    name: "Low-pass filter cutoff",
    slider_range: [20f64.log2(), 48000f64.log2()],
    difference_slider_range: 5.0,
    average_effects: 0.7,
  }),
  (log_highpass_filter_cutoff, FrequencyType, |state| Some(&state.rendering_state.after_highpass), SignalInfo {
    id: "highpass",
    name: "High-pass filter cutoff",
    slider_range: [10f64.log2(), 20000f64.log2()],
    difference_slider_range: 5.0,
    average_effects: 0.7,
  }),
  (log_bitcrush_frequency, FrequencyType, |state| Some(&state.rendering_state.after_bitcrush), SignalInfo {
    id: "bitcrush_frequency",
    name: "Bitcrush frequency",
    slider_range: [20f64.log2(), 48000f64.log2()],
    difference_slider_range: 5.0,
    average_effects: 0.7,
  }),
}


impl Waveform {
  pub fn sample (&self, phase: f64)->f64 {
    match *self {
      Waveform::Sine => (phase*TURN).sin(),
      Waveform::Square => if phase.fract() < 0.5 {0.5} else {-0.5},
      Waveform::Triangle => 1.0 - (phase.fract()-0.5).abs()*4.0,
      Waveform::Sawtooth => 1.0 - phase.fract()*2.0,
    }
  }
}

impl<T: UserNumberType> SignalEffect <T> {
  pub fn sample (&self, sample_time: f64)->f64 {
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
      SignalEffect::Oscillation {size, frequency, waveform} => size.rendered*waveform.sample (sample_time*frequency.rendered.exp2()),
    }
  }
  pub fn range (&self)->[f64;2] {
    match self.clone() {
      SignalEffect::Jump {size, ..} => [min (0.0, size.rendered), max (0.0, size.rendered)],
      SignalEffect::Slide {size, ..} => [min (0.0, size.rendered), max (0.0, size.rendered)],
      SignalEffect::Oscillation {size, ..} => [-size.rendered.abs(), size.rendered.abs()],
    }
  }
}

impl<T: UserNumberType> Signal<T> {
  pub fn constant(value: UserNumber <T>)->Self {
    Signal {
      initial_value: value,
      effects: Vec::new(),
    }
  }

  pub fn sample (&self, time: f64)->f64 {
    self.initial_value.rendered + self.effects.iter().map (| effect | effect.sample (time)).sum::<f64>()
  }
  
  pub fn range (&self)->[f64;2] {
    let mut result = [self.initial_value.rendered; 2];
    for effect in self.effects.iter() {
      let range = effect.range();
      result [0] += range [0];
      result [1] += range [1];
    }
    result
  }
}

impl Envelope {
  pub fn duration (&self)->f64 {self.attack.rendered + self.sustain.rendered + self.decay.rendered}
  pub fn sample (&self, time: f64)->f64 {
    if time <self.attack.rendered {return time/self.attack.rendered;}
    if time <self.attack.rendered + self.sustain.rendered {return 1.0;}
    if time <self.attack.rendered + self.sustain.rendered + self.decay.rendered {return (self.attack.rendered + self.sustain.rendered + self.decay.rendered - time)/self.decay.rendered;}
    0.0
  }
}

impl SoundDefinition {
  pub fn duration(&self)->f64 {self.envelope.duration()}
  pub fn sample_rate (&self)->usize {44100}
}
