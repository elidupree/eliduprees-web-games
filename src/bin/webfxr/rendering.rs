use super::*;

use rand::{Rng, IsaacRng, SeedableRng};

pub fn root_mean_square (samples: & [f32])->f32{
  (samples.iter().map (| sample | sample*sample).sum::<f32>()/samples.len() as f32).sqrt()
}
pub fn logistic_curve (input: f64)->f64 {
  0.5+0.5*(input*0.5).tanh()
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

pub struct IllustrationLine {
  pub root_mean_square: f32,
  pub clipping: bool,
}
pub struct RenderedSamples {
  pub serial_number: SerialNumber,
  pub unprocessed_supersamples: Vec<f64>,
  pub samples: Vec<f32>,
  pub illustration: Vec<IllustrationLine>,
  pub canvas: Value,
  pub context: Value,
  pub audio_buffer: Value,
}
impl Default for RenderedSamples {
  fn default()->Self {
    let canvas = js!{ return $(new_canvas ()); };
    let context = js!{ return @{&canvas}[0].getContext ("2d"); };
    RenderedSamples {
      serial_number: Default::default(),
      unprocessed_supersamples: Vec::new(),
      samples: Vec::new(),
      illustration: Vec::new(),
      canvas: canvas, context: context,
      audio_buffer: js!{
        if (window.webfxr_num_samples) {
          return audio.createBuffer (1, window.webfxr_num_samples, window.webfxr_sample_rate);
        }
        return undefined;
      },
    }
  }
}
#[derive (Default)]
pub struct RenderingStateConstants {
  pub num_samples: usize,
  pub sample_rate: usize,
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
  
  pub after_flanger: RenderedSamples,
  
  pub lowpass_state: LowpassFilterState,
  pub after_lowpass: RenderedSamples,
  
  pub highpass_state: HighpassFilterState,
  pub after_highpass: RenderedSamples,
  
  pub bitcrush_phase: f64,
  pub bitcrush_last_used_sample: f64,
  pub after_bitcrush: RenderedSamples,
  
  pub final_samples: RenderedSamples,
  
  pub constants: RenderingStateConstants,
}

impl RenderedSamples {
  pub fn push (&mut self, value: f64, constants: &RenderingStateConstants) {
    self.unprocessed_supersamples.push (value);
    if self.unprocessed_supersamples.len() == constants.supersamples_per_sample {
      self.samples.push ((self.unprocessed_supersamples.drain(..).sum::<f64>() / constants.supersamples_per_sample as f64) as f32);
      if self.samples.len() % constants.samples_per_illustrated == 0 || self.samples.len() == constants.num_samples {
        let batch_start = ((self.samples.len()-1) / constants.samples_per_illustrated) * constants.samples_per_illustrated;
        let rendered_slice = & self.samples [batch_start..];
        let value = root_mean_square (rendered_slice);
        
        self.illustration.push (IllustrationLine {root_mean_square: value, clipping: rendered_slice.iter().any (| value | value.abs() > 1.0)});
        self.draw_line (self.illustration.len() - 1) ;
        
        let rendered: TypedArray <f32> = rendered_slice.into();
        js! {
          const rendered = @{rendered};
          @{&self.audio_buffer}.copyToChannel (rendered, 0, @{batch_start as f64});
        }  
      }
    }
  }
  
  pub fn draw_line (&self, index: usize) {
    let line = &self.illustration [index];
    // assume that root-mean-square only goes up to 0.5; the radius should also range from 0 to 0.5
    let radius = line.root_mean_square;
    
    js!{
      var canvas = @{&self.canvas}[0];
      var context = @{&self.context};
      context.fillStyle = @{line.clipping} ? "rgb(255,0,0)" : "rgb(0,0,0)";
      
      var radius = canvas.height*@{radius};
      context.fillRect (@{index as f64}, canvas.height*0.5 - radius, 1, radius*2);
    }
  }
  
  pub fn redraw (&self, playback_position: Option <f64>, constants: & RenderingStateConstants) {
    js!{
      var canvas = @{&self.canvas}[0];
      var context = @{&self.context};
      context.clearRect (0, 0, canvas.width, canvas.height);
    }
    for index in 0..self.illustration.len() {
      self.draw_line (index);
    }
    if let Some(playback_position) = playback_position {
    let index = (playback_position*constants.sample_rate as f64/constants.samples_per_illustrated as f64).floor();
    js!{
      var canvas = @{&self.canvas}[0];
      var context = @{&self.context};
      context.fillStyle = "rgb(255,255,0)";
      
      context.fillRect (@{index as f64}, 0, 1, canvas.height);
    }
    }
  }
  
  pub fn resample (&self, time: f64, constants: &RenderingStateConstants)->f64 {
    // note: it's technically worse to use samples instead of supersamples,
    // but storing the super samples would use more memory,
    // and that the time of this writing, I'm not using supersampling anyway.
    //
    // Linear interpolation because it doesn't matter that much anyway.
    
    let scaled = time*constants.sample_rate as f64;
    let previous_index = scaled.floor() as isize as usize;
    let fraction = scaled.fract();
    let previous = self.samples.get (previous_index).cloned().unwrap_or (0.0) as f64;
    let next = self.samples.get (previous_index.wrapping_add (1)).cloned().unwrap_or (0.0) as f64;
    previous*(1.0 - fraction) + next*fraction
  }
}



pub fn generator_for_time (time: f64)->IsaacRng {
  let whatever = (time*1_000_000_000.0).floor() as i64 as u64;
  IsaacRng::from_seed (Array::from_fn (| index | (whatever >> (index*8)) as u8))
}

impl Waveform {
  pub fn sample_simple (&self, phase: f64)->Option<f64> {
    let fraction = phase - phase.floor();
    Some(match *self {
      Waveform::Sine => ((phase-0.25)*TURN).sin(),
      Waveform::Square => if fraction < 0.5 {0.5} else {-0.5},
      Waveform::Triangle => 1.0 - (fraction-0.5).abs()*4.0,
      Waveform::Sawtooth => 1.0 - fraction*2.0,
      Waveform::WhiteNoise => /*generator_for_time (phase)*/rand::thread_rng().gen_range(-1.0, 1.0),
    })
  }
}

impl SoundDefinition {
  pub fn sample_waveform (&self, time: f64, phase: f64)->f64 {
    match self.waveform {
      Waveform::WhiteNoise => return self.waveform.sample_simple (phase).unwrap(),
      _=>(),
    }
    
    let mut result = 0.0;
    let harmonics = if self.harmonics.enabled {max (1.0, min (100.0, self.harmonics.sample (time)))} else {1.0};
    let skew = logistic_curve (self.waveform_skew.sample (time));
    for index in 0..harmonics.ceil() as usize {
      let mut harmonic = (index + 1) as f64;
      let fraction = if harmonic <= harmonics {1.0} else {harmonics + 1.0 - harmonic};
      if self.odd_harmonics {
        harmonic = (index*2 + 1) as f64;
      }
      let mut harmonic_phase = phase*harmonic;
      if self.waveform_skew.enabled {
        harmonic_phase = harmonic_phase - harmonic_phase.floor();
        harmonic_phase = if harmonic_phase < skew {harmonic_phase*0.5/skew} else {0.5 + (harmonic_phase - skew)*0.5/(1.0 - skew)};
      }
      result += self.waveform.sample_simple (harmonic_phase).unwrap()*fraction/harmonic;
    }
    result
  }
}

impl RenderingState {
  pub fn final_samples (&self)->& RenderedSamples {& self.final_samples}
  pub fn new (sound: & SoundDefinition)->RenderingState {
    let num_samples = (min(MAX_RENDER_LENGTH, sound.duration())*sound.sample_rate() as f64).ceil() as usize;
    js! { window.webfxr_num_samples = @{num_samples as f64}; window.webfxr_sample_rate = @{sound.sample_rate() as f64}; } 
    let supersamples_per_sample = 1;
    RenderingState {
      constants: RenderingStateConstants {
        num_samples: num_samples,
        sample_rate: sound.sample_rate(),
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
    
    let mut sample = sound.sample_waveform (time, self.wave_phase)*sound.envelope.sample (time);
    self.after_frequency.push (sample, &self.constants);
    
    sample *= sound.volume.sample (time).exp2();
    self.after_volume.push (sample, &self.constants);
    
    if sound.log_flanger_frequency.enabled {
      let flanger_frequency = sound.log_flanger_frequency.sample (time).exp2();
      let flanger_offset = 1.0/flanger_frequency;
      sample += self.after_volume.resample (time - flanger_offset, & self.constants);
      self.after_flanger.push (sample, &self.constants);
    }
    
    if sound.log_lowpass_filter_cutoff.enabled {
      let lowpass_filter_frequency = sound.log_lowpass_filter_cutoff.sample (time).exp2();
      sample = self.lowpass_state.apply (sample, lowpass_filter_frequency, self.constants.supersample_duration);
      self.after_lowpass.push (sample, &self.constants);
    }
    
    if sound.log_highpass_filter_cutoff.enabled {
      let highpass_filter_frequency = sound.log_highpass_filter_cutoff.sample (time).exp2();
      sample = self.highpass_state.apply (sample, highpass_filter_frequency, self.constants.supersample_duration);
      self.after_highpass.push (sample, &self.constants);
    }

    if sound.log_bitcrush_frequency.enabled {
      if self.bitcrush_phase >= 1.0 {
        self.bitcrush_phase -= 1.0;
        if self.bitcrush_phase >1.0 {self.bitcrush_phase = 1.0;}
        self.bitcrush_last_used_sample = sample; 
      }
      sample = self.bitcrush_last_used_sample;
      self.after_bitcrush.push (sample, &self.constants);
     
      let bitcrush_frequency = sound.log_bitcrush_frequency.sample(time).exp2();
      self.bitcrush_phase += bitcrush_frequency*self.constants.supersample_duration;
    }
    
    self.final_samples.push (sample, &self.constants) ;
    
    let frequency = sound.log_frequency.sample(time).exp2();
    self.wave_phase += frequency*self.constants.supersample_duration;
    
    self.next_supersample += 1;
  }
  pub fn step (&mut self, sound: & SoundDefinition) {
    if self.finished() {return;}
    
    let batch_samples = self.constants.samples_per_illustrated;
    let batch_supersamples = batch_samples*self.constants.supersamples_per_sample;
    for _ in 0..batch_supersamples {
      self.superstep(sound);
      if self.finished() {return;}
    }
  }
  pub fn finished (&self)->bool {
    self.next_supersample == self.constants.num_supersamples
  }
}
