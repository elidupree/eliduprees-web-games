use super::*;
use std::collections::BTreeSet;
use ordered_float::OrderedFloat;

use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
type Generator = Xoshiro256StarStar;

pub fn logistic_curve (input: f64)->f64 {
  0.5+0.5*(input*0.5).tanh()
}

// BFXR uses first-order digital RC low/high pass filters.
// Personally, I always end up feeling like the rolloff isn't steep enough.
// So I chain multiples of them together.
const FILTER_ITERATIONS: usize = 3;

pub const CHORUS_OSCILLATOR_AMPLITUDE: f64 = 0.05;
pub const CHORUS_OSCILLATOR_MAX_LINGER_DURATION: f64 = CHORUS_OSCILLATOR_AMPLITUDE*2.0;

//#[derive (Default)]
pub type FirstOrderLowpassFilterState = f64;
#[derive (Default)]
pub struct LowpassFilterState(Vec<FirstOrderLowpassFilterState>);
//#[derive (Default)]
//pub struct FirstOrderHighpassFilterState {value: f64, previous: f64}
#[derive (Default)]
pub struct HighpassFilterState(Vec<FirstOrderLowpassFilterState>);

impl LowpassFilterState {
  pub fn new (order: usize)->Self {LowpassFilterState (vec![0.0; order])}
  pub fn apply (&mut self, mut sample: f64, cutoff_frequency: f64, duration: f64)->f64 {
    let decay_constant = TURN*cutoff_frequency/(2f64.powf (1.0/self.0.len() as f64) - 1.0).sqrt();
    let decay_factor = (-decay_constant*duration).exp();
    for value in self.0.iter_mut() {
      *value = *value + (1.0 - decay_factor) * (sample - *value);
      sample = *value;
    }
    sample    
  }
}
impl HighpassFilterState {
  pub fn new (order: usize)->Self {HighpassFilterState (vec![0.0; order])}
  pub fn apply (&mut self, mut sample: f64, cutoff_frequency: f64, duration: f64)->f64 {
    let decay_constant = TURN*cutoff_frequency*(2f64.powf (1.0/self.0.len() as f64) - 1.0).sqrt();
    let decay_factor = (-decay_constant*duration).exp();
    for value in self.0.iter_mut() {
      *value = *value + (1.0 - decay_factor) * (sample - *value);
      sample -= *value;
    }
    sample
  }
}

pub struct IllustrationLine {
  pub range: [f64; 2],
  pub clipping: bool,
}
#[derive (Derivative)]
#[derivative (Default)]
pub struct Illustration {
  #[derivative (Default (value = "[- 0.5, 0.5]"))]
  pub range: [f64; 2],
  pub lines: Vec<IllustrationLine>,
}
pub struct RenderedSamples {
  //pub serial_number: SerialNumber,
  pub samples: Vec<f32>,
  pub illustration: Illustration,
  pub audio_buffer: Value,
}
impl Default for RenderedSamples {
  fn default()->Self {
    RenderedSamples {
      //serial_number: Default::default(),
      samples: Vec::new(),
      illustration: Default::default(),
      audio_buffer: js!{
        if (window.webfxr_num_samples) {
          return audio.createBuffer (1, window.webfxr_num_samples, window.webfxr_sample_rate);
        }
        return undefined;
      },
    }
  }
}

#[derive (Derivative)]
#[derivative (Default)]
pub struct WaveformRenderingState {
  #[derivative (Default (value = "Generator::from_seed ([1; 32])"))]
  pub generator: Generator,
  
  pub last_phase: f64,
  pub value: f64,
  pub values: Vec<f64>,
  
  #[derivative (Default (value = "LowpassFilterState::new (16)"))]
  pub lowpass_state: LowpassFilterState,
  #[derivative (Default (value = "HighpassFilterState::new (16)"))]
  pub highpass_state: HighpassFilterState,
}

pub struct SignalEffectRenderingState {
  pub waveform: WaveformRenderingState,
}

#[derive (Default)]
pub struct SignalRenderingState {
  pub effects: Vec<SignalEffectRenderingState>,
  pub samples: Vec<f64>,
  pub illustration: Illustration,
  pub rendered_after: RenderedSamples,
}


#[derive (Default)]
pub struct RenderingStateConstants {
  pub num_samples: usize,
  pub sample_rate: usize,
  pub sample_duration: f64,
  pub samples_per_illustrated: usize,
  pub samples_per_signal_sample: usize,
  pub started_rendering_at: f64,
}

#[derive (Derivative)]
#[derivative (Default)]
pub struct RenderingState {
  pub next_sample: usize,
  pub next_time: f64,
  
  #[derivative (Default (value = "Generator::from_seed ([0; 32])"))]
  pub generator: Generator,
  
  pub wave_phase: f64,
  pub cycle_starts: BTreeSet < OrderedFloat <f64>>,
  pub waveform_samples: Vec<f64>,
  
  pub harmonics: Vec<WaveformRenderingState>,
  pub signals: SignalsRenderingState,
  
  #[derivative (Default (value = "LowpassFilterState::new (FILTER_ITERATIONS)"))]
  pub lowpass_state: LowpassFilterState,
  #[derivative (Default (value = "HighpassFilterState::new (FILTER_ITERATIONS)"))]
  pub highpass_state: HighpassFilterState,
  
  pub bitcrush_phase: f64,
  pub bitcrush_last_used_sample: f64,
  
  pub final_samples: RenderedSamples,
  
  pub constants: RenderingStateConstants,
}

pub fn maybe_batch <T, F: FnMut(usize, &[T])> (samples: &[T], constants: &RenderingStateConstants, mut batch_function: F) {
  if samples.len() % constants.samples_per_illustrated == 0 || samples.len() == constants.num_samples {
    let batch_start = ((samples.len()-1) / constants.samples_per_illustrated) * constants.samples_per_illustrated;
    let rendered_slice = & samples [batch_start..];
    (batch_function)(batch_start, rendered_slice) ;
  }
}

pub fn resample <T: Clone + Into <f64>> (samples: & [T], position: f64)->f64 {
  // Linear interpolation because it doesn't matter that much anyway.
  let previous_index = position.floor() as isize as usize;
  let fraction = position.fract();
  let previous = match samples.get (previous_index) {
    Some (value) => value.clone().into(),
    None => 0.0,
  };
  let next = match samples.get (previous_index.wrapping_add (1)) {
    Some (value) => value.clone().into(),
    None => 0.0,
  };
  previous*(1.0 - fraction) + next*fraction
}

impl RenderedSamples {
  pub fn push (&mut self, value: f64, constants: &RenderingStateConstants) {
    self.samples.push (value as f32);
    let illustration = &mut self.illustration;
    let buffer = &self.audio_buffer;
    maybe_batch (& self.samples, constants, | batch_start, rendered_slice | {
    
        //let average = rendered_slice.iter().map (| sample | *sample as f64).sum::<f64>()/rendered_slice.len() as f64;
        let root_mean_square = (rendered_slice.iter().map (| sample | {
          let sample = *sample as f64;// - average;
          sample*sample
        }).sum::<f64>()/rendered_slice.len() as f64).sqrt();
                
        illustration.lines.push (IllustrationLine {range: [0.5-root_mean_square, 0.5+root_mean_square], clipping: rendered_slice.iter().any (| value | value.abs() > 1.0)});
        
        let rendered: TypedArray <f32> = rendered_slice.into();
        js! {
          const rendered = @{rendered};
          @{buffer}.copyToChannel (rendered, 0, @{batch_start as f64});
        }  
    });
  }
  
  pub fn resample (&self, time: f64, constants: &RenderingStateConstants)->f64 {
    let scaled = time*constants.sample_rate as f64;
    resample (& self.samples, scaled)
  }
}



/*pub fn generator_for_time (time: f64)->IsaacRng {
  let whatever = (time*1_000_000_000.0).floor() as i64 as u64;
  IsaacRng::from_seed (Array::from_fn (| index | (whatever >> (index*8)) as u8))
}*/

impl Waveform {
  pub fn sample_simple (&self, phase: f64)->f64 {
    let fraction = phase - phase.floor();
    match *self {
      Waveform::Sine => ((phase-0.25)*TURN).sin(),
      Waveform::Square => if fraction < 0.5 {0.5} else {-0.5},
      Waveform::Triangle => 1.0 - (fraction-0.5).abs()*4.0,
      Waveform::Sawtooth => 1.0 - fraction*2.0,
      _ => panic!("{:?} can't be sampled without a rendering state", self),
    }
  }
}

fn do_white_noise (value: &mut f64, fraction: f64, generator: &mut Generator)->f64 {
  let decay_factor = 1.0 - fraction;
  // normalize so that the power is equal to that of white noise
  // let infinite_sum = 1.0/(1.0 - decay_factor); (infinite sum of sample powers)
  // we want to reduce the power by that sum, so
  // let power_factor = (1.0/infinite_sum).sqrt();
  // but that can be done in fewer steps and without making an exception for 0.0
  let power_factor = fraction.sqrt();
  *value = generator.gen_range(-1.0, 1.0)*power_factor + *value*decay_factor;
  *value
}

fn do_pink_noise (values: &mut Vec<f64>, max_fraction: f64, min_fraction: f64, generator: &mut Generator)->f64 {
  // for this one, maintain constant power by always using the same number of components,
  // and just equally arranging them within the range
  let num_values = 24;
  while values.len() <num_values {values.push (0.0);}
  if max_fraction == 0.0 {return 0.0}
  let amplitude_adjust = 1.0/(num_values as f64).sqrt();
  let factor = (min_fraction/max_fraction).powf(1.0/(num_values-1) as f64);
  let mut fraction = max_fraction;
  let mut result = 0.0;
  for value in values.iter_mut() {
    do_white_noise (value, fraction, generator);
    fraction *= factor;
    result += *value*amplitude_adjust;
  }
  result
}

impl WaveformRenderingState {
  fn next_sample (&mut self, definition: & Waveform, _index: usize, _time: f64, frequency: f64, phase: f64, constants: &RenderingStateConstants)->f64 {
    let offset = phase - self.last_phase;
    let fraction = if offset > 1.0 {1.0} else {offset - offset.floor()};
    assert! (fraction.is_finite());
    assert! (fraction >= 0.0) ;
    let result = match definition.clone() {
      Waveform::WhiteNoise => self.generator.gen_range(-1.0, 1.0),
      Waveform::PinkNoise => do_pink_noise (&mut self.values, 1.0, 10.0/constants.sample_rate as f64, &mut self.generator),
      Waveform::BrownNoise => do_white_noise (&mut self.value, 20.0/constants.sample_rate as f64, &mut self.generator),
      Waveform::PitchedWhite => do_white_noise (&mut self.value, fraction, &mut self.generator),
      Waveform::PitchedPink => do_pink_noise (&mut self.values, fraction, min (fraction*0.25, 10.0/constants.sample_rate as f64), &mut self.generator),
      Waveform::Experimental => {
        let mut sample =do_pink_noise (&mut self.values, 1.0, 10.0/constants.sample_rate as f64, &mut self.generator);
        sample = self.lowpass_state.apply (sample, frequency, constants.sample_duration);
        sample = self.highpass_state.apply (sample, frequency, constants.sample_duration);
        sample*4.0
      },
      _ => definition.sample_simple (phase),
    };
    self.last_phase = phase;
    result
  }
}

const SMOOTH_TIME: f64 = 0.001;

impl SignalEffectRenderingState {
  fn next_sample <T: UserNumberType> (&mut self, definition: & SignalEffect <T>, index: usize, sample_time: f64, smooth: bool, constants: &RenderingStateConstants)->f64 {
    match definition.clone() {
      SignalEffect::Jump {time, size} => {
        if smooth && sample_time > time.rendered && sample_time < time.rendered + SMOOTH_TIME {
          size.rendered*(sample_time - time.rendered)/SMOOTH_TIME
        }
        else if sample_time > time.rendered {size.rendered} else {0.0}
      },
      SignalEffect::Slide {start, duration, size, smooth_start, smooth_stop} => {
        let mut duration = duration.rendered;
        if smooth {duration = max (duration, SMOOTH_TIME) ;}
        if sample_time <= start.rendered {0.0}
        else if sample_time >= start.rendered + duration {size.rendered}
        else {
          let fraction = (sample_time - start.rendered)/duration;
          let adjusted_fraction = match (smooth_start, smooth_stop) {
            (false, false) => fraction,
            (true, false) => fraction*fraction,
            (false, true) => fraction*(2.0-fraction),
            (true, true) => fraction*fraction*(3.0 - 2.0*fraction),
          };
          size.rendered*adjusted_fraction
        }
      },
      SignalEffect::Oscillation {size, frequency, waveform} => {
        let frequency = frequency.rendered.exp2();
        let phase = sample_time*frequency;
        if smooth && frequency < 100.0 {
          let smooth_phase = SMOOTH_TIME*frequency;
          let phase = phase - phase.floor();
          if waveform == Waveform::Square {
            if phase < smooth_phase {
              return size.rendered*(0.5 - phase/smooth_phase);
            }
            let phase_into_second_half = phase - 0.5;
            if phase_into_second_half >= 0.0 && phase_into_second_half < smooth_phase {
              return size.rendered*(phase_into_second_half/(smooth_phase*2.0) - 0.5);
            }
          }
          if waveform == Waveform::Sawtooth {
            if phase < smooth_phase {
              return size.rendered*(2.0*phase/smooth_phase - 1.0);
            } else {
              return size.rendered*(1.0 - 2.0*(phase-smooth_phase)/(1.0-smooth_phase));
            }
          }
        }
        size.rendered*self.waveform.next_sample (& waveform, index, sample_time, frequency, phase, constants)
      },
    }
  }
}

impl SignalRenderingState {
  fn next_sample <T: UserNumberType> (&mut self, definition: & Signal <T>, index: usize, time: f64, smooth: bool, constants: &RenderingStateConstants)->f64 {
    definition.initial_value.rendered + self.effects.iter_mut().zip (definition.effects.iter()).map (| (rendering, definition) | rendering.next_sample(definition, index, time, smooth, constants)).sum::<f64>()
  }
}

impl RenderingState {
  pub fn sample_signal <Identity: SignalIdentity> (&mut self, definition: & SoundDefinition, smooth: bool)->f64 {
    let index = self.next_sample;
    let time = self.next_time;
    let rendering = Identity::rendering_getter().get_mut (&mut self.signals);
    let sample = rendering.next_sample (Identity::definition_getter().get (& definition.signals), index, time, smooth, &self.constants);
    rendering.samples.push (sample);
    let illustration = &mut rendering.illustration;
    maybe_batch (& rendering.samples, &self.constants, |_, sample_slice | {
        let mut range = [sample_slice.iter().map (|f|OrderedFloat(*f)).min().unwrap().0, sample_slice.iter().map (|f|OrderedFloat(*f)).max().unwrap().0];
        for value in &mut range {
          *value = (*value - illustration.range [0])/(illustration.range [1] - illustration.range [0]);
        }
        illustration.lines.push (IllustrationLine {range: range, clipping: false });
    });
    sample
  }

  pub fn new (sound: & SoundDefinition)->RenderingState {
    let num_samples = (min(MAX_RENDER_LENGTH, sound.rendering_duration())*sound.sample_rate() as f64).ceil() as usize;
    js! { window.webfxr_num_samples = @{num_samples as f64}; window.webfxr_sample_rate = @{sound.sample_rate() as f64}; }
    let mut cycle_starts = BTreeSet::new();
    cycle_starts.insert (OrderedFloat (0.0));
    let mut result = RenderingState {
      constants: RenderingStateConstants {
        num_samples: num_samples,
        sample_rate: sound.sample_rate(),
        sample_duration: 1.0/(sound.sample_rate() as f64),
        samples_per_illustrated: (sound.sample_rate() as f64/DISPLAY_SAMPLE_RATE).ceil() as usize,
        samples_per_signal_sample: (sound.sample_rate() as f64/500.0).ceil() as usize,
        started_rendering_at: now(),
      },
      bitcrush_phase: 1.0,
      cycle_starts: cycle_starts,
      .. Default::default()
    };
    
    for _ in 0..(if sound.enabled::<Harmonics>() {max (1.0, min (100.0, sound.signals.harmonics.range() [1].ceil())) as usize} else {1}) {
      result.harmonics.push (Default::default());
    }
    
    struct Visitor <'a> (& 'a SoundDefinition, & 'a mut RenderingState);
    impl<'a> SignalVisitor for Visitor<'a> {
      fn visit <Identity: SignalIdentity> (&mut self) {
        // create the generator outside the if so the later generators are
        // still consistent regardless of whether this signal is enabled
        let mut generator = Generator::from_rng (&mut self.1.generator).unwrap();
        if self.0.enabled::<Identity>() {
          let signal = Identity::definition_getter().get (& self.0.signals);
          let rendering = Identity::rendering_getter().get_mut (&mut self.1.signals);
          let info = Identity::info();
          
          let observed_range = signal.range();
          rendering.illustration.range = [
            min (observed_range [0], info.slider_range [0]),
            max (observed_range [1], info.slider_range [1]),
          ];
          
          for _effect in signal.effects.iter() {
            rendering.effects.push (SignalEffectRenderingState {
              waveform: WaveformRenderingState {
                generator: Generator::from_rng (&mut generator).unwrap(),
                .. Default::default()
              },
            });
          }
        }
      }
    }
  
    visit_signals (&mut Visitor (sound, &mut result));
    
    result
  }

  
  pub fn next_waveform_sample (&mut self, sound: & SoundDefinition)->f64 {
    let time = self.next_time;
    let index = self.next_sample;
    let phase = self.wave_phase;
    let frequency = self.sample_signal::<LogFrequency> (sound, false).exp2();
    
    /*let sample = match sound.waveform {
      Waveform::WhiteNoise | Waveform::PinkNoise | Waveform::BrownNoise | Waveform::PitchedWhite | Waveform::PitchedPink | Waveform::Experimental => self.main_waveform.next_sample (& sound.waveform, index, time, frequency, phase, & self.constants),
      _=>{*/
      
    let mut result = 0.0;
    let harmonics = if sound.enabled::<Harmonics>() {max (1.0, self.sample_signal::<Harmonics> (sound, true))} else {1.0};
    let skew = logistic_curve (self.sample_signal::<WaveformSkew> (sound, true));
    let mut total = 0.0;
    for (harmonic_index, waveform) in self.harmonics.iter_mut().enumerate() {
      let mut harmonic = (harmonic_index + 1) as f64;
      if sound.odd_harmonics {
        harmonic = (harmonic_index*2 + 1) as f64;
      }
      let mut harmonic_phase = phase*harmonic;
      if sound.enabled::<WaveformSkew>() {
        harmonic_phase = harmonic_phase - harmonic_phase.floor();
        harmonic_phase = if harmonic_phase < skew {harmonic_phase*0.5/skew} else {0.5 + (harmonic_phase - skew)*0.5/(1.0 - skew)};
      }
      let this_sample = waveform.next_sample (& sound.waveform, index, time, frequency*harmonic, harmonic_phase, & self.constants);
      let leeway = harmonics - (harmonic_index as f64);
      if leeway > 0.0 {
        let fraction = min(1.0, leeway).sqrt();
        let amplitude = fraction/harmonic;
        total += amplitude*amplitude;
        result += this_sample*amplitude;
      }
    }
    
    let sample = result/total.sqrt();
    //  }
    //};
    self.waveform_samples.push (sample);
    
    let previous_floor = self.wave_phase.floor();
    let phase_increment = frequency*self.constants.sample_duration;
    self.wave_phase += phase_increment;
    let new_floor = self.wave_phase.floor();
    if new_floor != previous_floor {
      let time_fraction = 1.0 - (self.wave_phase.fract()/phase_increment);
      let switch_time = time + time_fraction*self.constants.sample_duration;
      self.cycle_starts.insert (OrderedFloat (switch_time)) ;
    }
    
    sample
  }
    
  /*fn handle_signal <Identity: SignalIdentity> (&mut self, sound: & SoundDefinition, smooth: bool,) {
    let sample = self.sample_signal::<Identity>
  }*/
  
  
  fn single_step (&mut self, sound: & SoundDefinition) {
    let time = self.next_time;
    
    let mut sample = self.next_waveform_sample (sound)*sound.envelope.sample (time);
    self.signals.log_frequency.rendered_after.push (sample, &self.constants);
    
    sample *= self.sample_signal::<Volume> (sound, true).exp2();
    self.signals.volume.rendered_after.push (sample, &self.constants);
    
    let mut previous_getter = Volume::rendering_getter().dynamic();
    
    if sound.enabled::<Chorus>() {
      let voices = min(100.0, self.sample_signal::<Chorus> (sound, true));
      if voices > 0.0 {for voice in 0..voices.ceil() as usize {
        let fraction = if voices >= (voice + 1) as f64 {1.0} else {(voices - voice as f64).sqrt()};
        let oscillator_amplitude = CHORUS_OSCILLATOR_AMPLITUDE;
        let oscillator_max_derivative = 0.006;
        let oscillator_max_speed = oscillator_max_derivative/oscillator_amplitude;
        let oscillator_speed = oscillator_max_speed*5.0/(5.0 + voice as f64);
        let initial_phase = TURN*5.0/(5.0 + voice as f64);
        let offset = ((time*oscillator_speed + initial_phase).sin() - 1.0)*oscillator_amplitude;
        assert!(offset >= -(0.000001+CHORUS_OSCILLATOR_MAX_LINGER_DURATION), "oscillator offset outside of allowed range; the max range is relied upon in Sound::duration() to make sure there is no pop at the end of the sound");
        sample += self.signals.volume.rendered_after.resample (time + offset, & self.constants)*fraction;
      }
        sample /= voices + 1.0;
      }
      self.signals.chorus.rendered_after.push (sample, &self.constants);
      previous_getter = Chorus::rendering_getter().dynamic();
    }
    
    if sound.enabled::<LogFlangerFrequency>() {
      let flanger_frequency = self.sample_signal::<LogFlangerFrequency> (sound, true).exp2();
      let flanger_offset = 1.0/flanger_frequency;
      sample += previous_getter.get (& self.signals).rendered_after.resample (time - flanger_offset, & self.constants);
      sample += self.signals.get::<LogFlangerFrequency>().rendered_after.resample (time - flanger_offset, & self.constants);
      sample /= 2.0;
      self.signals.log_flanger_frequency.rendered_after.push (sample, &self.constants);
    }
    
    if sound.enabled::<LogLowpassFilterCutoff>() {
      let lowpass_filter_frequency = self.sample_signal::<LogLowpassFilterCutoff> (sound, false).exp2();
      sample = self.lowpass_state.apply (sample, lowpass_filter_frequency, self.constants.sample_duration);
      self.signals.log_lowpass_filter_cutoff.rendered_after.push (sample, &self.constants);
    }
    
    if sound.enabled::<LogHighpassFilterCutoff>() {
      let highpass_filter_frequency = self.sample_signal::<LogHighpassFilterCutoff> (sound, false).exp2();
      sample = self.highpass_state.apply (sample, highpass_filter_frequency, self.constants.sample_duration);
      self.signals.log_highpass_filter_cutoff.rendered_after.push (sample, &self.constants);
    }
    
    if sound.enabled::<BitcrushResolutionBits>() {
      let bits = max (1.0, self.sample_signal::<BitcrushResolutionBits> (sound, false));
      let floor_bits = bits.floor();
      let bits_fraction = bits - floor_bits;
      let increment = 4.0/floor_bits.exp2();
      let sample_increments = (sample+1.0)/increment;
      let sample_increments_rounded = sample_increments.round();
      let sample_fraction = sample_increments - sample_increments_rounded;
      sample = if sample_fraction.abs() > 0.25*(2.0 - bits_fraction) {sample_increments_rounded + sample_fraction.signum()*0.5} else {sample_increments_rounded}*increment - 1.0;
      self.signals.bitcrush_resolution_bits.rendered_after.push (sample, &self.constants);
    }

    if sound.enabled::<LogBitcrushFrequency>() {
      if self.bitcrush_phase >= 1.0 {
        self.bitcrush_phase -= 1.0;
        if self.bitcrush_phase >1.0 {self.bitcrush_phase = 1.0;}
        self.bitcrush_last_used_sample = sample; 
      }
      sample = self.bitcrush_last_used_sample;
      self.signals.log_bitcrush_frequency.rendered_after.push (sample, &self.constants);
     
      let bitcrush_frequency = self.sample_signal::<LogBitcrushFrequency> (sound, false).exp2();
      self.bitcrush_phase += bitcrush_frequency*self.constants.sample_duration;
    }
    
    if sound.soft_clipping {
      sample = sample/(1.0 + sample.abs());
    }
    
    self.final_samples.push (sample, &self.constants) ;
    
    self.next_sample += 1;
    self.next_time = self.next_sample as f64*self.constants.sample_duration;
  }
  pub fn step (&mut self, sound: & SoundDefinition) {
    if self.finished() {return;}
    
    let batch_samples = self.constants.samples_per_illustrated;
    for _ in 0..batch_samples {
      self.single_step(sound);
      if self.finished() {return;}
    }
  }
  pub fn finished (&self)->bool {
    self.next_sample == self.constants.num_samples
  }
}
