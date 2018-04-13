
use super::*;

const SUPERSAMPLE_SHIFT: usize = 3;
const SUPERSAMPLE_RATIO: usize = 1 << SUPERSAMPLE_SHIFT;

// BFXR uses first-order digital RC low/high pass filters.
// Personally, I always end up feeling like the rolloff isn't steep enough.
// So I chain multiples of them together.
const FILTER_ITERATIONS: usize = 3;

//#[derive (Default)]
pub type FirstOrderLowpassFilterState = f32;
#[derive (Default)]
pub struct LowpassFilterState([FirstOrderLowpassFilterState; FILTER_ITERATIONS]);
#[derive (Default)]
pub struct FirstOrderHighpassFilterState {value: f32, previous: f32}
#[derive (Default)]
pub struct HighpassFilterState([FirstOrderHighpassFilterState; FILTER_ITERATIONS]);

//note: the formulas for the filter cutoff are based on a first-order filter, so they are not exactly correct for this. TODO fix
impl LowpassFilterState {
  pub fn apply (&mut self, mut sample: f32, cutoff_frequency: f32, duration: f32)->f32 {
    let dt = duration;
    let rc = 1.0/(TURN*cutoff_frequency);
    let lowpass_filter_constant = dt/(dt + rc);
    for iteration in 0..FILTER_ITERATIONS {
      self.0 [iteration] = self.0 [iteration] + lowpass_filter_constant * (sample - self.0 [iteration]);
      sample = self.0 [iteration];
    }
    sample    
  }
}
impl HighpassFilterState {
  pub fn apply (&mut self, mut sample: f32, cutoff_frequency: f32, duration: f32)->f32 {
    let dt = duration;
    let rc = 1.0/(TURN*cutoff_frequency);
    let highpass_filter_constant = rc/(rc + dt);
    for iteration in 0..FILTER_ITERATIONS {
      self.0 [iteration].value = highpass_filter_constant * (
        self.0 [iteration].value + (sample - self.0 [iteration].previous));
      self.0 [iteration].previous = sample;
      sample = self.0 [iteration].value;
    }
    sample
  }
}


#[derive (Default)]
pub struct RenderedSamples {
  pub samples: Vec<f32>,
  pub illustration: Vec<f32>,
}
#[derive (Default)]
pub struct RenderingStateConstants {
  num_samples: usize,
  supersample_duration: f32,
}

#[derive (Default)]
pub struct RenderingState {
  next_sample: usize,
  
  wave_phase: f32,
  after_frequency: RenderedSamples,
  
  after_volume: RenderedSamples,
  
  //after_flanger: RenderedSamples,
  
  lowpass_state: LowpassFilterState,
  after_lowpass: RenderedSamples,
  
  highpass_state: HighpassFilterState,
  after_highpass: RenderedSamples,
  
  bitcrush_phase: f32,
  bitcrush_last_used_sample: f32,
  after_bitcrush: RenderedSamples,
  
  after_envelope: RenderedSamples,
  
  constants: RenderingStateConstants,
}

impl RenderedSamples {
  pub fn push (&mut self, value: f32, constants: &RenderingStateConstants) {
    self.samples.push (value);
  }
}
impl RenderingState {
  pub fn final_samples (&self)->& RenderedSamples {& self.after_envelope}
  pub fn new (sound: & SoundDefinition)->RenderingState {
    RenderingState {
      constants: RenderingStateConstants {
        num_samples: (sound.duration()*sound.sample_rate() as f32).ceil() as usize,
        supersample_duration: 1.0/(sound.sample_rate()*SUPERSAMPLE_RATIO as f32),
      },
      bitcrush_phase: 1.0,
      .. Default::default()
    }
  }
  pub fn step (&mut self, sound: & SoundDefinition) {
    let time = self.next_sample as f32*self.constants.supersample_duration;
    
    let mut sample = sound.waveform.sample (self.wave_phase);
    self.after_frequency.push (sample, &self.constants);
    
    sample *= sound.volume.sample (time).exp2();
    self.after_volume.push (sample, &self.constants);
    
    let lowpass_filter_frequency = sound.log_lowpass_filter_cutoff.sample (time).exp2();
    sample = self.lowpass_state.apply (sample, lowpass_filter_frequency, self.constants.supersample_duration);
    self.after_lowpass.push (sample, &self.constants);
    
    let highpass_filter_frequency = sound.log_highpass_filter_cutoff.sample (time).exp2();
    sample = self.highpass_state.apply (sample, highpass_filter_frequency, self.constants.supersample_duration);
    self.after_highpass.push (sample, &self.constants);

    
    if self.bitcrush_phase >= 1.0 {
      self.bitcrush_phase -= 1.0;
      if self.bitcrush_phase >1.0 {self.bitcrush_phase = 1.0;}
      self.bitcrush_last_used_sample = sample; 
    }
    sample = self.bitcrush_last_used_sample;
    self.after_bitcrush.push (sample, &self.constants);
        
    sample *= sound.envelope.sample (time).exp2();
    self.after_envelope.push (sample, &self.constants);
    
      
    let frequency = sound.log_frequency.sample(time).exp2();
    let bitcrush_frequency = sound.log_bitcrush_frequency.sample(time).exp2();
    self.wave_phase += frequency*self.constants.supersample_duration;
    self.bitcrush_phase += bitcrush_frequency*self.constants.supersample_duration;
  }
}
