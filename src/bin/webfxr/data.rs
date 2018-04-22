use std::rc::Rc;
use std::str::FromStr;
use std::fmt::Debug;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::{DeserializeOwned, Error};

use super::*;


pub const DISPLAY_SAMPLE_RATE: f64 = 50.0;
pub const MAX_RENDER_LENGTH: f64 = 10.0;


macro_rules! zero_information_number_type {
  ($Enum: ident, $Variant: ident, $Difference: ident, $name: expr, | $value: ident | $render: expr, | $rendered: ident | $from_rendered: expr) => {

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub enum $Enum {
  #[derivative (Default)]
  $Variant,
}

impl UserNumberType for $Enum {
  type DifferenceType = $Difference;
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

pub fn format_number (number: f64, minimum_significant_figures: i32)->String {
  let magnitude = number.abs();
  if magnitude < 0.0000001 { return format! ("0") }
  let biggest_figure = magnitude.log10().floor() as i32;
  let precision = ::std::cmp::max (0, minimum_significant_figures - 1 - biggest_figure);
  format! ("{:.*}", precision as usize, number)
}

zero_information_number_type!{
  FrequencyType, Frequency, IntervalType, "Hz",
  | value | {
    let value = value.log2();
    if value.is_finite() {Some (value)} else {None}
  },
  | rendered | format_number (rendered.exp2(), 3)
}
zero_information_number_type!{
  IntervalType, Ratio, Self, "ratio",
  | value | {
    let value = value.log2();
    if value.is_finite() {Some (value)} else {None}
  },
  | rendered | format_number (rendered.exp2(), 3)
}
zero_information_number_type!{
  TimeType, Seconds, Self, "s",
  | value | Some (value),
  | rendered | format!("{:.3}", rendered)
}
zero_information_number_type!{
  VolumeDifferenceType, Decibels, Self, "dB",
  | value | {
    let value = value/OCTAVES_TO_DECIBELS;
    if value.is_finite() {Some (value)} else {None}
  },
  | rendered | format!("{:.1}", rendered*OCTAVES_TO_DECIBELS)
}
zero_information_number_type!{
  DimensionlessType, Raw, Self, "dimensionless",
  | value | Some (value),
  | rendered | format!("{:.3}", rendered)
}

#[derive (Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub enum VolumeType {
  #[derivative (Default)]
  DecibelsAbove(#[derivative (Default (value = "DEFAULT_DECIBEL_BASE"))] f64),
}

pub trait UserNumberType: 'static + Clone + PartialEq + Serialize + DeserializeOwned + Debug + Default {
  type DifferenceType: UserNumberType;
  fn render (&self, value: & str)->Option<f64>;
  fn approximate_from_rendered (&self, rendered: f64)->String;
  fn unit_name (&self)->&'static str;
  fn currently_used (_state: & State)->Self {Self::default()}
}
#[derive (Clone, PartialEq)]
pub struct UserNumber <T: UserNumberType> {
  pub source: String,
  pub rendered: f64,
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

impl<T: UserNumberType> Serialize for UserNumber <T> {
  fn serialize <S: Serializer> (&self, serializer: S)->Result <S::Ok, S::Error> {
    (self.value_type.clone(), self.source.clone()).serialize (serializer)
  }
}

impl<'de, T: UserNumberType> Deserialize<'de> for UserNumber <T> {
  fn deserialize <D: Deserializer<'de>> (deserializer: D)->Result <Self, D::Error> {
    let (value_type, source): (T, String) = Deserialize::deserialize(deserializer)?;
    match Self::new(value_type.clone(), source.clone()) {
      Some (result) => Ok (result),
      None => Err (D::Error::custom (format!("{} isn't a valid value of type {:?}", source, value_type))),
    }
  }
}


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
  WhiteNoise,
}

js_serializable! (Waveform) ;
js_deserializable! (Waveform) ;


#[derive (Clone, PartialEq, Serialize, Deserialize)]
pub enum SignalEffect <T: UserNumberType> {
  Jump {time: UserTime, size: UserNumber<T::DifferenceType>},
  Slide {start: UserTime, duration: UserTime, size: UserNumber<T::DifferenceType>, smooth_start: bool, smooth_stop: bool},
  Oscillation {size: UserNumber<T::DifferenceType>, frequency: UserFrequency, waveform: Waveform},
}

#[derive (Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Signal <T: UserNumberType> {
  pub enabled: bool,
  #[serde(deserialize_with = "::serde::Deserialize::deserialize")]
  pub initial_value: UserNumber<T>,
  #[serde(deserialize_with = "::serde::Deserialize::deserialize")]
  pub effects: Vec<SignalEffect <T>>,
}

#[derive (Clone, PartialEq, Serialize, Deserialize, Default)]
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
  pub slider_step: f64,
  pub difference_slider_range: f64,
  pub average_effects: f64,
  pub can_disable: bool,
}

#[derive (Clone)]
pub struct TypedSignalInfo<T: UserNumberType> {
  pub untyped: SignalInfo,
  pub getter: Getter <State, Signal <T>>,
  pub rendered_getter: Option <Rc<Fn(& RenderingState)->& RenderedSamples>>,
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
          rendered_getter: $rendered_field,
        }
      }
    })*
  }
}

#[derive (Clone, PartialEq, Serialize, Deserialize)]
#[serde (default)]
pub struct SoundDefinition {
  pub envelope: Envelope,
  pub waveform: Waveform,
  pub harmonics: Signal <DimensionlessType>,
  pub odd_harmonics: bool,
  pub waveform_skew: Signal <DimensionlessType>,
  pub log_frequency: Signal <FrequencyType>,
  pub volume: Signal <VolumeType>,
  pub log_flanger_frequency: Signal <FrequencyType>,
  pub log_lowpass_filter_cutoff: Signal <FrequencyType>,
  pub log_highpass_filter_cutoff: Signal <FrequencyType>,
  pub bitcrush_resolution_bits: Signal <DimensionlessType>,
  pub log_bitcrush_frequency: Signal <FrequencyType>,
}

signals_definitions! {
  (log_frequency, FrequencyType, Some(Rc::new(|state| &state.after_frequency)), SignalInfo {
    id: "frequency",
    name: "Frequency",
    slider_range: [20f64.log2(), 2000f64.log2()],
    slider_step: 0.0,
    difference_slider_range: 2.0,
    average_effects: 2.0,
    can_disable: false,
  }),
  (harmonics, DimensionlessType, None, SignalInfo {
    id: "harmonics",
    name: "Harmonics",
    slider_range: [1.0, 13.0],
    slider_step: 1.0,
    difference_slider_range: 5.0,
    average_effects: 0.5,
    can_disable: true,
  }),
  (waveform_skew, DimensionlessType, None, SignalInfo {
    id: "waveform_skew",
    name: "Waveform skew",
    slider_range: [-5.0, 5.0],
    slider_step: 0.0,
    difference_slider_range: 5.0,
    average_effects: 0.7,
    can_disable: true,
  }),
  (volume, VolumeType, Some(Rc::new(|state| &state.after_volume)), SignalInfo {
    id: "volume",
    name: "Volume",
    slider_range: [DEFAULT_DECIBEL_BASE/OCTAVES_TO_DECIBELS,0.0],
    slider_step: 0.0,
    difference_slider_range: 2.0,
    average_effects: 0.7,
    can_disable: false,
  }),
  (log_flanger_frequency, FrequencyType, Some(Rc::new(|state| &state.after_flanger)), SignalInfo {
    id: "flanger_frequency",
    name: "Flanger frequency",
    slider_range: [20f64.log2(), 20000f64.log2()],
    slider_step: 0.0,
    difference_slider_range: 2.0,
    average_effects: 0.7,
    can_disable: true,
  }),
  (log_lowpass_filter_cutoff, FrequencyType, Some(Rc::new(|state| &state.after_lowpass)), SignalInfo {
    id: "lowpass",
    name: "Low-pass filter cutoff",
    slider_range: [100f64.log2(), 20000f64.log2()],
    slider_step: 0.0,
    difference_slider_range: 5.0,
    average_effects: 0.7,
    can_disable: true,
  }),
  (log_highpass_filter_cutoff, FrequencyType, Some(Rc::new(|state| &state.after_highpass)), SignalInfo {
    id: "highpass",
    name: "High-pass filter cutoff",
    slider_range: [20f64.log2(), 10000f64.log2()],
    slider_step: 0.0,
    difference_slider_range: 5.0,
    average_effects: 0.7,
    can_disable: true,
  }),
  (bitcrush_resolution_bits, DimensionlessType, Some(Rc::new(|state| &state.after_bitcrush_resolution)), SignalInfo {
    id: "bitcrush_resolution_bits",
    name: "Bitcrush resolution bits",
    slider_range: [1.0, 16.0],
    slider_step: 1.0,
    difference_slider_range: 10.0,
    average_effects: 0.7,
    can_disable: true,
  }),
  (log_bitcrush_frequency, FrequencyType, Some(Rc::new(|state| &state.after_bitcrush_frequency)), SignalInfo {
    id: "bitcrush_frequency",
    name: "Bitcrush frequency",
    slider_range: [100f64.log2(), 10000f64.log2()],
    slider_step: 0.0,
    difference_slider_range: 5.0,
    average_effects: 0.7,
    can_disable: true,
  }),
}


impl<T: UserNumberType> Signal<T> {
  pub fn constant(value: UserNumber <T>)->Self {
    Signal {
      enabled: false,
      initial_value: value,
      effects: Vec::new(),
    }
  }

  pub fn sample (&self, time: f64, smooth: bool)->f64 {
    self.initial_value.rendered + self.effects.iter().map (| effect | effect.sample (time, smooth)).sum::<f64>()
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
  
  pub fn draw_through_time (&self)->f64 {
    let mut result = 0.0;
    for effect in self.effects.iter() {
      result = max (result, effect.draw_through_time());
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
  pub fn duration(&self)->f64 {
    let mut result = self.envelope.duration();
    if self.log_flanger_frequency.enabled {
      result += 1.0/self.log_flanger_frequency.range() [0].exp2();
    }
    if self.log_bitcrush_frequency.enabled {
      result += 1.0/self.log_bitcrush_frequency.range() [0].exp2();
    }
    result
  }
  pub fn sample_rate (&self)->usize {44100}
}

impl Default for SoundDefinition {
  fn default()->Self {SoundDefinition {
      envelope: Envelope {attack: UserNumber::from_rendered (0.1), sustain: UserNumber::from_rendered (0.5), decay: UserNumber::from_rendered (0.5)},
      waveform: Waveform::Sine,
      harmonics: Signal::constant (UserNumber::from_rendered (3.0)),
      odd_harmonics: false,
      waveform_skew: Signal::constant (UserNumber::from_rendered (-2.0)),
      log_frequency: Signal::constant (UserNumber::from_rendered (220.0_f64.log2())),
      volume: Signal::constant (UserNumber::from_rendered (-2.0)),
      log_flanger_frequency: Signal::constant (UserNumber::from_rendered (1600.0_f64.log2())),
      log_bitcrush_frequency: Signal::constant (UserNumber::from_rendered (3600.0_f64.log2())),
      log_lowpass_filter_cutoff: Signal::constant (UserNumber::from_rendered (2500.0_f64.log2())),
      log_highpass_filter_cutoff: Signal::constant (UserNumber::from_rendered (600.0_f64.log2())),
      bitcrush_resolution_bits: Signal::constant (UserNumber::from_rendered (6.0)),
    }}
}
