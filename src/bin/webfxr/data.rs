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

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub enum Waveform {
  #[derivative (Default)]
  Sine,
  Square,
  Triangle,
  Sawtooth,
  WhiteNoise,
  PinkNoise,
  BrownNoise,
  PitchedWhite,
  PitchedPink,
  Experimental,
}

js_serializable! (Waveform) ;
js_deserializable! (Waveform) ;

pub fn waveforms_list()->Vec<(Waveform, & 'static str)> {
  vec![
      (Waveform::Sine, "Sine"),
      (Waveform::Square, "Square"),
      (Waveform::Triangle, "Triangle"),
      (Waveform::Sawtooth, "Sawtooth"),
      (Waveform::WhiteNoise, "White noise"),
      (Waveform::PinkNoise, "Pink noise"),
      (Waveform::BrownNoise, "Brown noise"),
      (Waveform::PitchedWhite, "Pitched white"),
      (Waveform::PitchedPink, "Pitched pink"),
      (Waveform::Experimental, "Experimental"),
  ]
}


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

pub trait SignalIdentityGetters {
  type NumberType: UserNumberType;
  fn definition_getter()->Getter <Signals, Signal <Self::NumberType>>;
  fn rendering_getter()->Getter <SignalsRenderingState, SignalRenderingState>;
}
pub trait SignalIdentity: SignalIdentityGetters {
  fn info()->SignalInfo;
  fn applicable (_sound: & SoundDefinition)->bool {true}
}

#[derive (Clone, Derivative)]
#[derivative (Default)]
pub struct SignalInfo {
  pub id: & 'static str,
  pub name: & 'static str,
  pub slider_range: [f64; 2],
  pub differences_are_intervals: bool,
  pub default: f64,
  #[derivative (Default (value = "0.0"))]
  pub slider_step: f64,
  pub difference_slider_range: f64,
  #[derivative (Default (value = "0.7"))]
  pub average_effects: f64,
  #[derivative (Default (value = "true"))]
  pub can_disable: bool,
}

pub trait SignalVisitor {
  fn visit <T: SignalIdentity> (&mut self);
}


macro_rules! signals_definitions {
  ($([$Identity: ident, $field: ident, $NumberType: ident],)*) => {

#[derive (Clone, PartialEq, Serialize, Deserialize)]
pub struct Signals {
  $(pub $field: Signal <$NumberType>,)*
}
#[derive (Default)]
pub struct SignalsRenderingState {
  $(pub $field: SignalRenderingState,)*
}

impl Default for Signals {
  fn default()->Self {
    Signals {
      $($field: Signal::constant (UserNumber::from_rendered ($Identity::info().default)),)*
    }
  }
}

$(
  pub struct $Identity (!);
  impl SignalIdentityGetters for $Identity {
    type NumberType = $NumberType;
    fn definition_getter()->Getter <Signals, Signal <Self::NumberType>> {
      getter! (sound => sound.$field)
    }
    fn rendering_getter()->Getter <SignalsRenderingState, SignalRenderingState> {
      getter! (rendering => rendering.$field)
    }
  }
)*

pub fn visit_signals <Visitor: SignalVisitor> (visitor: &mut Visitor) {
  $(
    visitor.visit::<$Identity>();
  )*
}
  }
}

impl Signals {
  pub fn get<Identity: SignalIdentity> (&self)->& Signal <Identity::NumberType> {Identity::definition_getter().get (self)}
  pub fn get_mut<Identity: SignalIdentity> (&mut self)->&mut Signal <Identity::NumberType> {Identity::definition_getter().get_mut (self)}
}
impl SignalsRenderingState {
  pub fn get<Identity: SignalIdentity> (&self)->& SignalRenderingState{Identity::rendering_getter().get (self)}
  pub fn get_mut<Identity: SignalIdentity> (&mut self)->&mut SignalRenderingState {Identity::rendering_getter().get_mut (self)}
}

#[derive (Clone, PartialEq, Serialize, Deserialize)]
#[serde (default)]
pub struct SoundDefinition {
  pub envelope: Envelope,
  pub waveform: Waveform,
  pub signals: Signals,
  pub odd_harmonics: bool,
  pub soft_clipping: bool,
  pub output_sample_rate: u32,
}

signals_definitions! {
  [LogFrequency, log_frequency, FrequencyType],
  [Harmonics, harmonics, DimensionlessType],
  [WaveformSkew, waveform_skew, DimensionlessType],
  [Volume, volume, VolumeType],
  [Chorus, chorus, DimensionlessType],
  [LogFlangerFrequency, log_flanger_frequency, FrequencyType],
  [LogLowpassFilterCutoff, log_lowpass_filter_cutoff, FrequencyType],
  [LogHighpassFilterCutoff, log_highpass_filter_cutoff, FrequencyType],
  [BitcrushResolutionBits, bitcrush_resolution_bits, DimensionlessType],
  [LogBitcrushFrequency, log_bitcrush_frequency, FrequencyType],
}

impl SignalIdentity for LogFrequency {
  fn info()->SignalInfo {SignalInfo {
    id: "frequency",
    name: "Frequency",
    slider_range: [20f64.log2(), 2000f64.log2()],
    differences_are_intervals: true,
    default: 220.0_f64.log2(),
    difference_slider_range: 2.0,
    average_effects: 2.0,
    can_disable: false,
    .. Default::default()
  }}
  fn applicable (sound: & SoundDefinition)->bool {match sound.waveform {
    Waveform::WhiteNoise | Waveform::PinkNoise | Waveform::BrownNoise => false,
    _ => true,
  }}
}
impl SignalIdentity for Harmonics {
  fn info()->SignalInfo {SignalInfo {
    id: "harmonics",
    name: "Harmonics",
    slider_range: [1.0, 13.0],
    default: 3.0,
    slider_step: 1.0,
    difference_slider_range: 5.0,
    average_effects: 0.5,
    .. Default::default()
  }}
  fn applicable (sound: & SoundDefinition)->bool {match sound.waveform {
    Waveform::WhiteNoise | Waveform::PinkNoise | Waveform::BrownNoise | Waveform::PitchedWhite | Waveform::PitchedPink => false,
    _ => true,
  }}
}
impl SignalIdentity for WaveformSkew {
  fn info()->SignalInfo {SignalInfo {
    id: "waveform_skew",
    name: "Waveform skew",
    slider_range: [-5.0, 5.0],
    default: -2.0,
    difference_slider_range: 5.0,
    .. Default::default()
  }}
  fn applicable (sound: & SoundDefinition)->bool {match sound.waveform {
    Waveform::WhiteNoise | Waveform::PinkNoise | Waveform::BrownNoise | Waveform::PitchedWhite | Waveform::PitchedPink | Waveform::Experimental => false,
    _ => true,
  }}
}
impl SignalIdentity for Volume {
  fn info()->SignalInfo {SignalInfo {
    id: "volume",
    name: "Volume",
    slider_range: [DEFAULT_DECIBEL_BASE/OCTAVES_TO_DECIBELS,0.0],
    default: -2.0,
    difference_slider_range: 2.0,
    can_disable: false,
    .. Default::default()
  }}
}
impl SignalIdentity for Chorus {
  fn info()->SignalInfo {SignalInfo {
    id: "chorus",
    name: "Chorus voices",
    slider_range: [1.0, 13.0],
    default: 3.0,
    slider_step: 1.0,
    difference_slider_range: 5.0,
    average_effects: 0.5,
    .. Default::default()
  }}
}
impl SignalIdentity for LogFlangerFrequency {
  fn info()->SignalInfo {SignalInfo {
    id: "flanger_frequency",
    name: "Flanger frequency",
    slider_range: [20f64.log2(), 20000f64.log2()],
    default: 1600.0_f64.log2(),
    difference_slider_range: 2.0,
    .. Default::default()
  }}
}
impl SignalIdentity for LogLowpassFilterCutoff {
  fn info()->SignalInfo {SignalInfo {
    id: "lowpass",
    name: "Low-pass filter cutoff",
    slider_range: [100f64.log2(), 20000f64.log2()],
    default: 2500.0_f64.log2(),
    difference_slider_range: 5.0,
    .. Default::default()
  }}
}
impl SignalIdentity for LogHighpassFilterCutoff {
  fn info()->SignalInfo {SignalInfo {
    id: "highpass",
    name: "High-pass filter cutoff",
    slider_range: [20f64.log2(), 10000f64.log2()],
    default: 600.0_f64.log2(),
    difference_slider_range: 5.0,
    .. Default::default()
  }}
}
impl SignalIdentity for BitcrushResolutionBits {
  fn info()->SignalInfo {SignalInfo {
    id: "bitcrush_resolution_bits",
    name: "Bitcrush resolution bits",
    slider_range: [1.0, 16.0],
    default: 6.0,
    slider_step: 1.0,
    difference_slider_range: 10.0,
    .. Default::default()
  }}
}
impl SignalIdentity for LogBitcrushFrequency {
  fn info()->SignalInfo {SignalInfo {
    id: "bitcrush_frequency",
    name: "Bitcrush frequency",
    slider_range: [100f64.log2(), 10000f64.log2()],
    default: 3600.0_f64.log2(),
    difference_slider_range: 5.0,
    .. Default::default()
  }}
}

impl<T: UserNumberType> SignalEffect<T> {
  pub fn range (&self)->[f64;2] {
    match self.clone() {
      SignalEffect::Jump {size, ..} => [min (0.0, size.rendered), max (0.0, size.rendered)],
      SignalEffect::Slide {size, ..} => [min (0.0, size.rendered), max (0.0, size.rendered)],
      SignalEffect::Oscillation {size, ..} => [-size.rendered.abs(), size.rendered.abs()],
    }
  }
  pub fn draw_through_time (&self)->f64 {
    match self.clone() {
      SignalEffect::Jump {time, ..} => time.rendered + 0.1,
      SignalEffect::Slide {start, duration, ..} => start.rendered + duration.rendered + 0.1,
      SignalEffect::Oscillation {frequency, ..} => 1.1/frequency.rendered.exp2(),
    }
  }
}

impl<T: UserNumberType> Signal<T> {
  pub fn constant(value: UserNumber <T>)->Self {
    Signal {
      enabled: false,
      initial_value: value,
      effects: Vec::new(),
    }
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
  pub fn rendering_duration(&self)->f64 {
    let mut result = self.envelope.duration();
    if self.enabled::<Chorus>() {
      result += CHORUS_OSCILLATOR_MAX_LINGER_DURATION;
    }
    if self.enabled::<LogFlangerFrequency>() {
      result += 1.0/self.signals.log_flanger_frequency.range() [0].exp2();
    }
    if self.enabled::<LogBitcrushFrequency>() {
      result += 1.0/self.signals.log_bitcrush_frequency.range() [0].exp2();
    }
    result
  }
  pub fn sample_rate (&self)->usize {self.output_sample_rate as usize}
    
  pub fn enabled <Identity: SignalIdentity> (&self)->bool {
    Identity::applicable (self) && (Identity::definition_getter ().get (&self.signals).enabled || ! Identity::info().can_disable)
  }
}

impl Default for SoundDefinition {
  fn default()->Self {SoundDefinition {
      envelope: Envelope {attack: UserNumber::from_rendered (0.1), sustain: UserNumber::from_rendered (0.5), decay: UserNumber::from_rendered (0.5)},
      waveform: Waveform::Sine,
      signals: Default::default(),
      odd_harmonics: false,
      soft_clipping: false,
      output_sample_rate: 44100,
    }}
}
