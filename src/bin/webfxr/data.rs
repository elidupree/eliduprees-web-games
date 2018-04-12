use std::rc::Rc;
use std::str::FromStr;
use serde::{Serialize};
use serde::de::DeserializeOwned;

//use super::*;


pub const TURN: f32 = ::std::f32::consts::PI*2.0;

pub const DISPLAY_SAMPLE_RATE: f32 = 50.0;

pub fn min (first: f32, second: f32)->f32 {if first < second {first} else {second}}
pub fn max (first: f32, second: f32)->f32 {if first > second {first} else {second}}


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
const DEFAULT_DECIBEL_BASE: f32 = -40.0;
#[derive (Clone, PartialEq, Serialize, Deserialize, Derivative)]
#[derivative (Default)]
pub enum VolumeType {
  #[derivative (Default)]
  DecibelsAbove(#[derivative (Default (value = "DEFAULT_DECIBEL_BASE"))] f32),
}
#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative (Default)]
pub enum VolumeDifferenceType {
  #[derivative (Default)]
  Decibels,
}
#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative (Default)]
pub enum DimensionlessType {
  #[derivative (Default)]
  Raw,
}
pub trait UserNumberType: 'static + Clone + PartialEq + Serialize + DeserializeOwned + Default {
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
const OCTAVES_TO_DECIBELS: f32 = 3.0102999;
impl UserNumberType for VolumeType {
  type DifferenceType = VolumeDifferenceType;
  fn render (&self, value: & str)->Option <f32> {
    match *self {
      VolumeType::DecibelsAbove(base) => f32::from_str (value).ok().and_then(| value | {
        let value = (value + base)/OCTAVES_TO_DECIBELS;
        if value.is_finite() {Some (value)} else {None}
      })
    }
  }
  fn approximate_from_rendered (&self, rendered: f32)->String {
    match *self {
      VolumeType::DecibelsAbove(base) => format!("{:.1}", rendered*OCTAVES_TO_DECIBELS - base)
    }
  }
  fn unit_name (&self)->&'static str {
    match *self {
      VolumeType::DecibelsAbove(_) => "Decibels"
    }
  }
}
impl UserNumberType for VolumeDifferenceType{
  type DifferenceType = Self;
  fn render (&self, value: & str)->Option <f32> {
    match *self {
      VolumeDifferenceType::Decibels => f32::from_str (value).ok().and_then(| value | {
        let value = value/OCTAVES_TO_DECIBELS;
        if value.is_finite() {Some (value)} else {None}
      })
    }
  }
  fn approximate_from_rendered (&self, rendered: f32)->String {
    match *self {
      VolumeDifferenceType::Decibels => format!("{:.1}", rendered*OCTAVES_TO_DECIBELS)
    }
  }
  fn unit_name (&self)->&'static str {
    match *self {
      VolumeDifferenceType::Decibels => "Decibels"
    }
  }
}

impl UserNumberType for DimensionlessType {
  type DifferenceType = Self;
  fn render (&self, value: & str)->Option <f32> {
    match *self {
      DimensionlessType::Raw => f32::from_str (value).ok().filter (| value | value.is_finite())
    }
  }
  fn approximate_from_rendered (&self, rendered: f32)->String {
    match *self {
      DimensionlessType::Raw => format!("{:.3}", rendered)
    }
  }
  fn unit_name (&self)->&'static str {
    match *self {
      DimensionlessType::Raw => ""
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
  pub constant: bool,
  pub effects: Vec<SignalEffect <T>>,
}

#[derive (Default)]
pub struct Envelope {
  pub attack: UserTime,
  pub sustain: UserTime,
  pub decay: UserTime,
}

pub struct SignalInfo {
  pub id: & 'static str,
  pub name: & 'static str,
  pub slider_range: [f32; 2],
  pub difference_slider_range: f32,
  pub average_effects: f32,
}

pub trait SignalVisitor {
  fn visit <T: UserNumberType> (&mut self, info: & SignalInfo, signal: & Signal <T>, getter: Getter <State, Signal <T>>);
}

pub trait SignalVisitorMut {
  fn visit_mut <T: UserNumberType> (&mut self, info: & SignalInfo, signal: &mut Signal <T>, getter: Getter <State, Signal <T>>);
}

macro_rules! signals_definitions {
  ($(($field: ident, $info: expr),)*) => {
    impl SoundDefinition {
      pub fn signals_static_info()->Vec<SignalInfo> {
        vec![
          $($info,)*
        ]
      }
      pub fn visit_callers <T: SignalVisitor> (&self)->Vec<Box<Fn(&mut T, &SoundDefinition)>> {
        vec![
          $(Box::new (| visitor, sound | visitor.visit (& $info, &sound.$field, getter! (state => state.sound.$field))),)*
        ]
      }
      pub fn visit_mut_callers <T: SignalVisitorMut> (&self)->Vec<Box<Fn(&mut T, &mut SoundDefinition)>> {
        vec![
          $(Box::new (| visitor, sound | visitor.visit_mut (& $info, &mut sound.$field, getter! (state => state.sound.$field))),)*
        ]
      }
    }
    
    impl SignalInfo {
      $(pub fn $field ()->Self {
        $info
      })*
    }
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
  (log_frequency, SignalInfo {
    id: "frequency",
    name: "Frequency",
    slider_range: [20f32.log2(), 5000f32.log2()],
    difference_slider_range: 2.0,
    average_effects: 2.0,
  }),
  (volume, SignalInfo {
    id: "volume",
    name: "Volume",
    slider_range: [DEFAULT_DECIBEL_BASE/OCTAVES_TO_DECIBELS,0.0],
    difference_slider_range: 2.0,
    average_effects: 0.7,
  }),
  (log_lowpass_filter_cutoff, SignalInfo {
    id: "lowpass",
    name: "Low-pass filter cutoff",
    slider_range: [20f32.log2(), 48000f32.log2()],
    difference_slider_range: 5.0,
    average_effects: 0.7,
  }),
  (log_highpass_filter_cutoff, SignalInfo {
    id: "highpass",
    name: "High-pass filter cutoff",
    slider_range: [10f32.log2(), 20000f32.log2()],
    difference_slider_range: 5.0,
    average_effects: 0.7,
  }),
  (log_bitcrush_frequency, SignalInfo {
    id: "bitcrush_frequency",
    name: "Bitcrush frequency",
    slider_range: [20f32.log2(), 48000f32.log2()],
    difference_slider_range: 5.0,
    average_effects: 0.7,
  }),
}

pub struct State {
  pub sound: SoundDefinition,
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
      SignalEffect::Oscillation {size, frequency, waveform} => size.rendered*waveform.sample (sample_time*frequency.rendered.exp2()),
    }
  }
  pub fn range (&self)->[f32;2] {
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
  
  pub fn range (&self)->[f32;2] {
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
    
    // BFXR uses first-order digital RC low/high pass filters.
    // Personally, I always end up feeling like the rolloff isn't steep enough.
    // So I chain multiples of them together.
    const FILTER_ITERATIONS: usize = 3;
    let mut lowpass_filter_state = [0.0; FILTER_ITERATIONS];
    let mut highpass_filter_state = [0.0; FILTER_ITERATIONS];
    let mut highpass_filter_prev_input = [0.0; FILTER_ITERATIONS];
    
    for index in 0..num_frames {
      let time = index as f32/sample_rate as f32;
      
      let mut sample = self.waveform.sample (wave_phase)*self.envelope.sample (time)*self.volume.sample (time).exp2();
      
      //note: the formulas for the filter cutoff are based on a first-order filter, so they are not exactly correct for this. TODO fix
      let lowpass_filter_frequency = self.log_lowpass_filter_cutoff.sample (time).exp2();
      let dt = 1.0/sample_rate as f32;
      let rc = 1.0/(TURN*lowpass_filter_frequency);
      let lowpass_filter_constant = dt/(dt + rc);
      for iteration in 0..FILTER_ITERATIONS {
        lowpass_filter_state [iteration] = lowpass_filter_state [iteration] + lowpass_filter_constant * (sample - lowpass_filter_state [iteration]);
        sample = lowpass_filter_state [iteration];
      }
      let highpass_filter_frequency = self.log_highpass_filter_cutoff.sample (time).exp2();
      let rc = 1.0/(TURN*highpass_filter_frequency);
      let highpass_filter_constant = rc/(rc + dt);
      for iteration in 0..FILTER_ITERATIONS {
        highpass_filter_state [iteration] = highpass_filter_constant * (
          highpass_filter_state [iteration] + (sample - highpass_filter_prev_input [iteration]));
        highpass_filter_prev_input [iteration] = sample;
        sample = highpass_filter_state [iteration];
      }
      
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
