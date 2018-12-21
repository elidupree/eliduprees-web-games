use super::*;
use rand::{Rng};
use rand::distributions::{self, Distribution};
use rand::seq::SliceRandom;

pub const ATTACK_RANGE: [f64; 2] = [0.01, 1.0];
pub const SUSTAIN_RANGE: [f64; 2] = [0.01, 1.0];
pub const DECAY_RANGE: [f64; 2] = [0.01, 1.0];
pub const SLIDE_DURATION_RANGE: [f64; 2] = [0.01, 2.0];
pub fn oscillation_frequency_range()-> [f64; 2] {[1f64.log2(), 20f64.log2()]}
pub fn jump_time_range (duration: f64)->[f64; 2] {
  let buffer_duration = min(1.0,duration)*0.02;
  [buffer_duration, duration - buffer_duration]
}
pub fn slide_start_time_range (duration: f64)->[f64; 2] {
  let buffer_duration = min(1.0,duration)*0.2;
  [0.0, duration - buffer_duration]
}


pub fn random_number_linear <G: Rng, T: UserNumberType>(generator: &mut G, range: [f64; 2])->UserNumber <T> {
  UserNumber::from_rendered (generator.gen_range (range [0], range [1]))
}
pub fn random_number_logarithmic <G: Rng, T: UserNumberType>(generator: &mut G, range: [f64; 2])->UserNumber <T> {
  UserNumber::from_rendered (generator.gen_range (range [0].log2(), range [1].log2()).exp2())
}



pub fn random_waveform <G: Rng>(generator: &mut G)->Waveform {
  match generator.gen_range (0, 4) {
    0...2 => [
      Waveform::Sine,
      Waveform::Square,
      Waveform::Triangle,
      Waveform::Sawtooth,
    ].choose(generator).unwrap().clone(),
    _ => [
      Waveform::WhiteNoise,
      Waveform::PinkNoise,
      Waveform::BrownNoise,
      Waveform::PitchedWhite,
      Waveform::PitchedPink,
      Waveform::Experimental,
    ].choose(generator).unwrap().clone(),
  }
}
pub fn random_envelope <G: Rng>(generator: &mut G)->Envelope {
  Envelope {
    attack: random_number_logarithmic (generator, ATTACK_RANGE),
    sustain: random_number_logarithmic (generator, SUSTAIN_RANGE),
    decay: random_number_logarithmic (generator, DECAY_RANGE),
  }
}
pub fn random_signal <G: Rng, T: UserNumberType>(generator: &mut G, duration: f64, info: & SignalInfo)->Signal <T> {
  let enabled = generator.gen() && info.can_disable;
  let mut effects = Vec::new() ;
  if enabled || !info.can_disable {
    let num_effects = distributions::Poisson::new(info.average_effects).sample (generator);
    for _ in 0..num_effects {
      effects.push (random_signal_effect (generator, duration, info));
    }
  }
  Signal {
    enabled: enabled,
    initial_value: random_number_linear (generator, info.slider_range),
    effects: effects,
  }
}

pub fn random_difference <G: Rng, T: UserNumberType>(generator: &mut G, info: & SignalInfo)->UserNumber <T> {
  if !info.differences_are_intervals || (generator.gen_range (0, 3)==0) {
    random_number_linear (generator, [-info.difference_slider_range, info.difference_slider_range])
  }
  else {
    let intervals = [
      (2, 1), (2, 1), (2, 1), (2, 1),
      (3, 2), (3, 2), (3, 2), (3, 2), (3, 4), (3, 4),
      (5, 3), (5, 4), (5, 6), (5, 8),
      (7, 4), (7, 5), (7, 6), (7, 8), (7, 10), (7, 12),
      (9, 8), (15, 16),
    ];
    let (first, second) = intervals.choose(generator).unwrap().clone();
    let ratio = if generator.gen() {first as f64/second as f64} else {second as f64/first as f64};
    static_downcast (UserNumber::new (IntervalType::Ratio, format_number (ratio, 5)).unwrap())
  }
}

pub fn random_signal_effect <G: Rng, T: UserNumberType>(generator: &mut G, duration: f64, info: & SignalInfo)->SignalEffect <T> {
  match generator.gen_range (0, 3) {
    0 => random_jump_effect (generator, duration, info),
    1 => random_slide_effect (generator, duration, info),
    2 => random_oscillation_effect (generator, duration, info),
    _ => unreachable!(),
  }
}
pub fn random_jump_effect <G: Rng, T: UserNumberType>(generator: &mut G, duration: f64, info: & SignalInfo)->SignalEffect <T> {
  SignalEffect::Jump {
    time: random_number_linear(generator, jump_time_range (duration)),
    size: random_difference (generator, info),
  }
}
pub fn random_slide_effect <G: Rng, T: UserNumberType>(generator: &mut G, duration: f64, info: & SignalInfo)->SignalEffect <T> {
  SignalEffect::Slide {
    start: random_number_linear(generator, slide_start_time_range (duration)),
    duration: random_number_linear(generator, SLIDE_DURATION_RANGE),
    size: random_difference (generator, info),
    smooth_start: generator.gen(),
    smooth_stop: generator.gen(),
  }
}
pub fn random_oscillation_effect <G: Rng, T: UserNumberType>(generator: &mut G, _duration: f64, info: & SignalInfo)->SignalEffect <T> {
  SignalEffect::Oscillation {
    size: random_difference (generator, info),
    waveform: random_waveform (generator),
    frequency: random_number_linear (generator, oscillation_frequency_range()),
  }
}


pub fn random_sound <G: Rng>(generator: &mut G)->SoundDefinition {
  let mut sound = SoundDefinition {
    waveform: random_waveform(generator),
    envelope: random_envelope (generator),
    odd_harmonics: generator.gen(),
    soft_clipping: generator.gen(),
    ..Default::default()
  };
  struct Visitor <'a, G: 'a + Rng> (& 'a mut SoundDefinition, & 'a mut G);
  impl<'a, G: Rng> SignalVisitor for Visitor<'a, G> {
    fn visit <Identity: SignalIdentity> (&mut self) {
      let duration = self.0.envelope.duration();
      *Identity::definition_getter().get_mut (&mut self.0.signals) = random_signal (self.1, duration, & Identity::info());
    }
  }
  
  visit_signals (&mut Visitor (&mut sound, generator));
  
  for _attempt in 0..90 {
    let log_frequency_range = sound.signals.log_frequency.range();
    let info = LogFrequency::info();
    if log_frequency_range[0] < info.slider_range [0] || log_frequency_range[1] > info.slider_range [1] {
      sound.signals.log_frequency = random_signal (generator, sound.envelope.duration(), &info);
    }
  }
  
  let max_attempts = 90;
  for attempt in 0..max_attempts {
    let last = attempt == max_attempts - 1;
    let volume_range = sound.signals.volume.range();
    let volume_mid = (volume_range[1] + volume_range[0])/2.0;
    let target_mid = -1.0;
    let increase = target_mid - volume_mid;
    if increase.abs() > 0.001 {
      sound.signals.volume.initial_value = UserNumber::from_rendered (sound.signals.volume.initial_value.rendered + increase);
    }
    let waveform_skew_range = sound.signals.waveform_skew.range();
    if max (waveform_skew_range [1].abs(), waveform_skew_range [0].abs()) >5.0 {
      sound.signals.waveform_skew = random_signal (generator, sound.envelope.duration(), & WaveformSkew::info());
    }
    if sound.signals.log_lowpass_filter_cutoff.range() [0] < sound.signals.log_frequency.range() [1] {
      let info = LogLowpassFilterCutoff::info();
      sound.signals.log_lowpass_filter_cutoff = if last {Signal::constant (UserNumber::from_rendered (info.slider_range [1]))}
      else {random_signal (generator, sound.envelope.duration(), &info)};
    }
    if sound.signals.log_highpass_filter_cutoff.range() [1] > sound.signals.log_frequency.range() [0] {
      let info = LogHighpassFilterCutoff::info();
      sound.signals.log_highpass_filter_cutoff = if last {Signal::constant (UserNumber::from_rendered (info.slider_range [0]))}
      else {random_signal (generator, sound.envelope.duration(), &info)};
    }
    if sound.signals.log_bitcrush_frequency.range() [0] < sound.signals.log_frequency.range() [1] {
      sound.signals.log_bitcrush_frequency = random_signal (generator, sound.envelope.duration(), & LogBitcrushFrequency::info());
    }
  }
  
  sound
}




fn max_switch_chance (equilibrium_true_chance: f64, current_state: bool)->f64 {
  if current_state {1.0 - equilibrium_true_chance}
  else {equilibrium_true_chance}
}

fn poisson_chance (lambda: f64, outcome: usize)->f64 {
  (- lambda).exp()*lambda.powi (outcome as i32)/(1..outcome).map (|x| x as f64).product::<f64>()
}

fn max_absolute_poisson_exchange_chance (lambda: f64, lower_outcome: usize)->f64 {
  min (poisson_chance (lambda, lower_outcome), poisson_chance (lambda, lower_outcome + 1))*0.5
}
fn max_relative_poisson_exchange_chance (lambda: f64, lower_outcome: usize, from_outcome: usize)->f64 {
  max_absolute_poisson_exchange_chance (lambda, lower_outcome) / poisson_chance (lambda, from_outcome)
}

/// Tweaking value generated with a random distribution, maintaining that distribution
///
/// i.e. so that generate() and mutate_randomly_distributed(generate()) have the same distribution
fn mutate_uniformly_distributed <G: Rng>(generator: &mut G, range: [f64; 2], max_change_size: f64, value: f64)->f64 {
  let offset: f64 = generator.gen_range(0.0, max_change_size);
  let result_range = [
    value + offset - max_change_size,
    value + offset,
  ];
  // if we're already out of range, don't worry about it,
  // but still skew towards getting back in range
  if value < range [0] {
    generator.gen_range (result_range [0] + max_change_size*0.2, result_range [1])
  }
  else if value > range [1] {
    generator.gen_range (result_range [0], result_range [1] - max_change_size*0.2)
  }
  else {
    generator.gen_range (max (range [0], result_range [0]), min (range [1], result_range [1]))
  }
}

fn mutate_logarithmically_distributed <G: Rng>(generator: &mut G, range: [f64; 2], relative_max_change_size: f64, value: f64)->f64 {
  if value <= range [0] * 0.5 {
    mutate_uniformly_distributed (generator, range, relative_max_change_size*range[0]*0.5, value)
  }
  else {
    let range = [range [0].log2(), range [1].log2()];
    mutate_uniformly_distributed (generator, range, relative_max_change_size*(range [1] - range [0]), value.log2()).exp2()
  }
}

fn mutate_number_uniformly_distributed <G: Rng, T: UserNumberType>(generator: &mut G, range: [f64; 2], max_change_size: f64, value: &UserNumber <T>)->UserNumber <T> {
  UserNumber::from_rendered (mutate_uniformly_distributed (generator, range, max_change_size, value.rendered))
}
fn mutate_number_logarithmically_distributed <G: Rng, T: UserNumberType>(generator: &mut G, range: [f64; 2], max_change_size: f64, value: &UserNumber <T>)->UserNumber <T> {
  UserNumber::from_rendered (mutate_logarithmically_distributed (generator, range, max_change_size, value.rendered))
}

pub struct SoundMutator <'a, G> {
  pub generator: & 'a mut G,
  pub duration: f64,
  pub flop_chance: f64,
  pub tweak_chance: f64,
  pub tweak_size: f64,
}

impl <'a, G: 'a + Rng> SoundMutator <'a, G> {
  pub fn mutate_number <T: UserNumberType> (&mut self, number: &mut UserNumber <T>, range: [f64; 2]) {
    if self.generator.gen::<f64>() < self.tweak_chance {
      *number = mutate_number_uniformly_distributed (self.generator, range, self.tweak_size*(range [1] - range [0]), number);
    }
  }
  pub fn mutate_number_logarithmic <T: UserNumberType> (&mut self, number: &mut UserNumber <T>, range: [f64; 2]) {
    if self.generator.gen::<f64>() < self.tweak_chance {
      *number = mutate_number_logarithmically_distributed (self.generator, range, self.tweak_size, number);
    }
  }
  pub fn mutate_bool (&mut self, value: &mut bool) {
    if self.generator.gen::<f64>() < self.flop_chance*0.5 {
      *value = !*value;
    }
  }

  pub fn mutate_difference <T: UserNumberType> (&mut self, difference: &mut UserNumber <T>, info: & SignalInfo) {
    self.mutate_number (difference, [- info.difference_slider_range, info.difference_slider_range]);
  }
  
  pub fn mutate_signal_effect <T: UserNumberType> (&mut self, effect: &mut SignalEffect <T>, info: & SignalInfo) {
    match effect {
      SignalEffect::Jump {time, size} => {
        self.mutate_difference (size, info);
        self.mutate_number (time, jump_time_range (self.duration));
      },
      SignalEffect::Slide {start, duration, size, smooth_start, smooth_stop} => {
        self.mutate_difference (size, info);
        self.mutate_bool (smooth_start) ;
        self.mutate_bool (smooth_stop);
        self.mutate_number (start, slide_start_time_range (self.duration));
        self.mutate_number (duration, SLIDE_DURATION_RANGE);
      },
      SignalEffect::Oscillation {size, frequency, waveform} => {
        self.mutate_difference (size, info);
        if self.generator.gen::<f64>() < self.tweak_chance { *frequency = mutate_number_uniformly_distributed (self.generator, oscillation_frequency_range(), self.tweak_size, frequency); }
        if self.generator.gen::<f64>() < self.flop_chance*0.5 {
          *waveform = random_waveform(self.generator);
        }
      },
    }
  }
  pub fn mutate_signal <T: UserNumberType> (&mut self, signal: &mut Signal <T>, info: & SignalInfo) {
    if info.can_disable {
      let switch_chance = self.flop_chance*max_switch_chance (0.5, signal.enabled);
      if self.generator.gen::<f64>() < switch_chance {
        signal.enabled = !signal.enabled;
      }
    }
    if signal.enabled || !info.can_disable {
      let num_effects = signal.effects.len();
      let increase_chance = self.flop_chance*max_relative_poisson_exchange_chance (info.average_effects, num_effects, num_effects);
      let roll: f64 = self.generator.gen::<f64>() - increase_chance;
      if roll < 0.0 {
        signal.effects.push (random_signal_effect (self.generator, self.duration, info));
      }
      else if num_effects > 0 {
        let reduce_chance = self.flop_chance*max_relative_poisson_exchange_chance (info.average_effects, num_effects - 1, num_effects);
        if roll < reduce_chance {
          signal.effects.remove (self.generator.gen_range(0, num_effects));
        }
      }
      
      for effect in &mut signal.effects {
        self.mutate_signal_effect (effect, info);
      }
      
      if self.generator.gen::<f64>() < self.tweak_chance {
        signal.initial_value = mutate_number_uniformly_distributed(self.generator, info.slider_range, (info.slider_range [1] - info.slider_range [0])*self.tweak_size, &signal.initial_value);
      }
    }
  }
  pub fn mutate_sound (&mut self, sound: &mut SoundDefinition) {
    let old_sound = sound.clone();
    let old_volume_range = sound.signals.volume.range();
    let old_volume_mid = (old_volume_range[1] + old_volume_range[0])/2.0;
    let old_target_mid = -1.0;
    let old_lowpass_badness = if sound.enabled::<LogLowpassFilterCutoff>() { max(0.0, sound.signals.log_frequency.range() [1] - sound.signals.log_lowpass_filter_cutoff.range() [0]) } else {0.0};
    let old_highpass_badness = if sound.enabled::<LogHighpassFilterCutoff>() { max(0.0, sound.signals.log_highpass_filter_cutoff.range() [1] - sound.signals.log_frequency.range() [0]) } else {0.0};
    let old_bitcrush_badness = if sound.enabled::<LogBitcrushFrequency>() { max(0.0, sound.signals.log_frequency.range() [1] - sound.signals.log_bitcrush_frequency.range() [0]) } else {0.0};
    while *sound == old_sound {
    self.mutate_number_logarithmic (&mut sound.envelope.attack, ATTACK_RANGE);
    self.mutate_number_logarithmic (&mut sound.envelope.sustain, SUSTAIN_RANGE);
    self.mutate_number_logarithmic (&mut sound.envelope.decay, DECAY_RANGE);
    
    self.duration = sound.envelope.duration();
    
    struct Visitor <'b, 'a, G: 'a + Rng> (& 'b mut SoundMutator <'a, G>, & 'b mut SoundDefinition);
    impl<'b, 'a, G: Rng> SignalVisitor for Visitor<'b, 'a, G> {
      fn visit <Identity: SignalIdentity> (&mut self) {
        if Identity::applicable (self.1) {
          self.0.mutate_signal (self.1.signals.get_mut::<Identity>(), & Identity::info());
        }
      }
    }
    
    visit_signals (&mut Visitor (self, sound));
    
    self.mutate_bool (&mut sound.odd_harmonics);
    self.mutate_bool (&mut sound.soft_clipping);
    if self.generator.gen::<f64>() < self.flop_chance*0.5 {
      sound.waveform = random_waveform(self.generator);
    }
    }
    
    
    
    let volume_range = sound.signals.volume.range();
    let volume_mid = (volume_range[1] + volume_range[0])/2.0;
    let target_mid = -1.0;
    
    let target_change = target_mid - old_target_mid;
    let new_mid = old_volume_mid + target_change;
    let increase = new_mid - volume_mid;
    if increase.abs() > 0.01 {
      sound.signals.volume.initial_value = UserNumber::from_rendered (sound.signals.volume.initial_value.rendered + increase);
    }
    
    let lowpass_badness = if sound.enabled::<LogLowpassFilterCutoff>() { max(0.0, sound.signals.log_frequency.range() [1] - sound.signals.log_lowpass_filter_cutoff.range() [0]) } else {0.0};
    let highpass_badness = if sound.enabled::<LogHighpassFilterCutoff>() { max(0.0, sound.signals.log_highpass_filter_cutoff.range() [1] - sound.signals.log_frequency.range() [0]) } else {0.0};
    let bitcrush_badness = if sound.enabled::<LogBitcrushFrequency>() { max(0.0, sound.signals.log_frequency.range() [1] - sound.signals.log_bitcrush_frequency.range() [0]) } else {0.0};
    
    //println!("{:?}", ((old_lowpass_badness, lowpass_badness),(old_highpass_badness, highpass_badness),(old_bitcrush_badness, bitcrush_badness)));
    
    if old_lowpass_badness < 0.01 && lowpass_badness > 0.0 {
      sound.signals.log_lowpass_filter_cutoff.initial_value = UserNumber::from_rendered (sound.signals.log_lowpass_filter_cutoff.initial_value.rendered + lowpass_badness);
    }
    if old_highpass_badness < 0.01 && highpass_badness > 0.0 {
      sound.signals.log_highpass_filter_cutoff.initial_value = UserNumber::from_rendered (sound.signals.log_highpass_filter_cutoff.initial_value.rendered - highpass_badness);
    }
    if old_bitcrush_badness < 0.01 && bitcrush_badness > 0.0 {
      sound.signals.log_bitcrush_frequency.initial_value = UserNumber::from_rendered (sound.signals.log_bitcrush_frequency.initial_value.rendered + bitcrush_badness);
    }
  }
}

