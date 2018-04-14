use super::*;

pub fn root_mean_square (samples: & [f32])->f32{
  (samples.iter().map (| sample | sample*sample).sum::<f32>()/samples.len() as f32).sqrt()
}

// BFXR uses first-order digital RC low/high pass filters.
// Personally, I always end up feeling like the rolloff isn't steep enough.
// So I chain multiples of them together.
const FILTER_ITERATIONS: usize = 3;

//#[derive (Default)]
pub type FirstOrderLowpassFilterState = f64;
#[derive (Default)]
pub struct LowpassFilterState([FirstOrderLowpassFilterState; FILTER_ITERATIONS]);
#[derive (Default)]
pub struct FirstOrderHighpassFilterState {value: f64, previous: f64}
#[derive (Default)]
pub struct HighpassFilterState([FirstOrderHighpassFilterState; FILTER_ITERATIONS]);

//note: the formulas for the filter cutoff are based on a first-order filter, so they are not exactly correct for this. TODO fix
impl LowpassFilterState {
  pub fn apply (&mut self, mut sample: f64, cutoff_frequency: f64, duration: f64)->f64 {
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
  pub fn apply (&mut self, mut sample: f64, cutoff_frequency: f64, duration: f64)->f64 {
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


pub struct RenderedSamples {
  pub unprocessed_supersamples: Vec<f64>,
  pub samples: Vec<f32>,
  pub illustration: Vec<f32>,
  pub canvas: Value,
  pub context: Value,
}
impl Default for RenderedSamples {
  fn default()->Self {
    let canvas = js!{ return $("<canvas>"); };
    let context = js!{ return @{&canvas}[0].getContext ("2d"); };
    RenderedSamples {
      unprocessed_supersamples: Vec::new(),
      samples: Vec::new(),
      illustration: Vec::new(),
      canvas: canvas, context: context
    }
  }
}
#[derive (Default)]
pub struct RenderingStateConstants {
  pub num_samples: usize,
  pub supersamples_per_sample: usize,
  pub num_supersamples: usize,
  pub supersample_duration: f64,
  pub samples_per_illustrated: usize,
}

#[derive (Default)]
pub struct RenderingState {
  pub next_supersample: usize,
  
  pub wave_phase: f64,
  pub after_frequency: RenderedSamples,
  
  pub after_volume: RenderedSamples,
  
  //pub after_flanger: RenderedSamples,
  
  pub lowpass_state: LowpassFilterState,
  pub after_lowpass: RenderedSamples,
  
  pub highpass_state: HighpassFilterState,
  pub after_highpass: RenderedSamples,
  
  pub bitcrush_phase: f64,
  pub bitcrush_last_used_sample: f64,
  pub after_bitcrush: RenderedSamples,
  
  pub after_envelope: RenderedSamples,
  
  pub constants: RenderingStateConstants,
}

impl RenderedSamples {
  pub fn push (&mut self, value: f64, constants: &RenderingStateConstants) {
    self.unprocessed_supersamples.push (value);
    if self.unprocessed_supersamples.len() == constants.supersamples_per_sample {
      self.samples.push ((self.unprocessed_supersamples.drain(..).sum::<f64>() / constants.supersamples_per_sample as f64) as f32);
      if self.samples.len() % constants.samples_per_illustrated == 0 {
        let value = root_mean_square (& self.samples [self.samples.len()-constants.samples_per_illustrated..]);
        js!{
          var canvas = @{&self.canvas}[0];
          var context = @{&self.context};
          context.fillStyle = "rgb(0,0,0)";
          // assume that root-mean-square only goes up to 0.5;
          // on the other hand, the radius should range from 0 to 0.5
          var radius = canvas.height*@{value};
          context.fillRect (@{self.illustration.len() as f64}, canvas.height*0.5 - radius, 1, radius*2);
        }
        self.illustration.push (value);
      }
    }
  }
}
impl RenderingState {
  pub fn final_samples (&self)->& RenderedSamples {& self.after_envelope}
  pub fn new (sound: & SoundDefinition)->RenderingState {
    let num_samples = (sound.duration()*sound.sample_rate() as f64).ceil() as usize;
    let supersamples_per_sample = 8;
    js! {
      window.webfxr_play_buffer = audio.createBuffer (1, @{num_samples as f64}, @{sound.sample_rate() as f64});
    }  
    RenderingState {
      constants: RenderingStateConstants {
        num_samples: num_samples,
        supersamples_per_sample: supersamples_per_sample,
        num_supersamples: num_samples*supersamples_per_sample,
        supersample_duration: 1.0/((sound.sample_rate()*supersamples_per_sample) as f64),
        samples_per_illustrated: (sound.sample_rate() as f64/DISPLAY_SAMPLE_RATE).ceil() as usize,
      },
      bitcrush_phase: 1.0,
      .. Default::default()
    }
  }
  fn superstep (&mut self, sound: & SoundDefinition) {
    let time = self.next_supersample as f64*self.constants.supersample_duration;
    
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
    self.next_supersample += 1;
  }
  pub fn step (&mut self, sound: & SoundDefinition)->bool {
    if self.next_supersample == self.constants.num_supersamples {return false;}
    
    let batch_samples = self.constants.samples_per_illustrated;
    let batch_supersamples = batch_samples*self.constants.supersamples_per_sample;
    for _ in 0..batch_supersamples {
      self.superstep(sound);
      if self.next_supersample == self.constants.num_supersamples {return false;}
    }
    
    
    
    let final_samples = &self.final_samples().samples;
    let batch_start = final_samples.len() - batch_samples;
    let rendered_slice = &final_samples[batch_start..];
    let rendered: TypedArray <f32> = rendered_slice.into();
    js! {
      const rendered = @{rendered};
      window.webfxr_play_buffer.copyToChannel (rendered, 0, @{batch_start as f64 });
    }  
    
    true
  }
}
