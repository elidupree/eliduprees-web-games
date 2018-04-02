#![recursion_limit="256"]

extern crate eliduprees_web_games;

#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
extern crate nalgebra;
extern crate ordered_float;

use stdweb::unstable::TryInto;


pub const TURN: f32 = ::std::f32::consts::PI*2.0;

#[derive (Serialize, Deserialize)]
pub enum Waveform {
  Sine,
  Square,
  Triangle,
  Sawtooth,
}

pub struct ControlPoint {
  pub time: f32,
  pub value: f32,
  pub slope: f32,
  pub jump: f32,
}
pub struct Signal {
  pub control_points: Vec<ControlPoint>,
}
pub struct SignalSampler <'a> {
  signal: & 'a Signal,
  next_control_index: usize,
}
pub struct SoundDefinition {
  pub waveform: Waveform,
  pub frequency: Signal,
  
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

impl SoundDefinition {
  pub fn duration(&self)->f32 {2.0}
  pub fn render (&self, sample_rate: u32)-> Vec<f32> {
    let num_frames = (self.duration()*sample_rate as f32).ceil() as u32;
    let mut wave_phase = 0.0;
    let mut frequency_sampler = self.frequency.sampler();
    let frame_duration = 1.0/sample_rate as f32;
    let mut frames = Vec::with_capacity (num_frames as usize);
    
    for index in 0..num_frames {
      let time = index as f32/sample_rate as f32;
      let frequency = frequency_sampler.sample(time);
      wave_phase += frequency*frame_duration;
      let sample = self.waveform.sample (wave_phase);
      frames.push (sample) ;
    }
    
    frames
  }
}


#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  
  
  
  stdweb::event_loop();
}


#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this natively");
}
