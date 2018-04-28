use super::*;
use rand::{Rng};
use rand::distributions::{self, Distribution};

pub fn random_time_linear <G: Rng>(generator: &mut G, min: f64, max: f64)->UserTime {
  UserNumber::from_rendered (generator.gen_range (min, max))
}
pub fn random_time_logarithmic <G: Rng>(generator: &mut G, min: f64, max: f64)->UserTime {
  UserNumber::from_rendered (generator.gen_range (min.log2(), max.log2()).exp2())
}

pub fn random_waveform <G: Rng>(generator: &mut G)->Waveform {
  generator.choose(&[
    Waveform::Sine,
    Waveform::Square,
    Waveform::Triangle,
    Waveform::Sawtooth,
    Waveform::WhiteNoise,
  ]).unwrap().clone()
}
pub fn random_envelope <G: Rng>(generator: &mut G)->Envelope {
  Envelope {
    attack: random_time_logarithmic (generator, 0.01, 1.0),
    sustain: random_time_logarithmic (generator, 0.05, 1.0),
    decay: random_time_logarithmic (generator, 0.05, 1.0),
  }
}
pub fn random_signal <G: Rng, T: UserNumberType>(generator: &mut G, duration: f64, info: & SignalInfo)->Signal <T> {
  let enabled = generator.gen() && info.can_disable;
  let mut effects = Vec::new() ;
  if enabled || !info.can_disable {
    let num_effects = distributions::poisson::Poisson::new(info.average_effects).sample (generator);
    for _ in 0..num_effects {
      effects.push (random_signal_effect (generator, duration, info));
    }
  }
  Signal {
    enabled: enabled,
    initial_value: UserNumber::from_rendered (generator.gen_range (info.slider_range [0], info.slider_range [1])),
    effects: effects,
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
  let buffer_duration = min(1.0,duration)*0.02;
  SignalEffect::Jump {
    time: random_time_linear(generator, buffer_duration, duration - buffer_duration),
    size: UserNumber::from_rendered (generator.gen_range (- info.difference_slider_range, info.difference_slider_range)),
  }
}
pub fn random_slide_effect <G: Rng, T: UserNumberType>(generator: &mut G, duration: f64, info: & SignalInfo)->SignalEffect <T> {
  let buffer_duration = min(1.0,duration)*0.2;
  SignalEffect::Slide {
    start: random_time_linear(generator, 0.0, duration - buffer_duration),
    duration: random_time_linear(generator, 0.01, 2.0),
    size: UserNumber::from_rendered (generator.gen_range (- info.difference_slider_range, info.difference_slider_range)),
    smooth_start: generator.gen(),
    smooth_stop: generator.gen(),
  }
}
pub fn random_oscillation_effect <G: Rng, T: UserNumberType>(generator: &mut G, _duration: f64, info: & SignalInfo)->SignalEffect <T> {
  SignalEffect::Oscillation {
    size: UserNumber::from_rendered (generator.gen_range (- info.difference_slider_range, info.difference_slider_range)),
    waveform: random_waveform (generator),
    frequency: UserNumber::from_rendered (generator.gen_range (1f64.log2(), 20f64.log2())),
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
      let duration = self.0.duration();
      *Identity::definition_getter() (self.0) = random_signal (self.1, duration, & Identity::info());
    }
  }
  
  visit_signals (&mut Visitor (&mut sound, generator));
  
  for _attempt in 0..90 {
    let log_frequency_range = sound.log_frequency.range();
    let info = SignalInfo::log_frequency();
    if log_frequency_range[0] < info.slider_range [0] || log_frequency_range[1] > info.slider_range [1] {
      sound.log_frequency = random_signal (generator, sound.duration(), &info);
    }
  }
  
  let max_attempts = 90;
  for attempt in 0..max_attempts {
    let last = attempt == max_attempts - 1;
    let volume_range = sound.volume.range();
    if volume_range[1] > -1.0 || volume_range[1] <= -2.0 {
      sound.volume = random_signal (generator, sound.duration(), &SignalInfo::volume());
    }
    let waveform_skew_range = sound.waveform_skew.range();
    if max (waveform_skew_range [1].abs(), waveform_skew_range [0].abs()) >5.0 {
      sound.waveform_skew = random_signal (generator, sound.duration(), &SignalInfo::waveform_skew());
    }
    if sound.log_lowpass_filter_cutoff.range() [0] < sound.log_frequency.range() [1] {
      let info = SignalInfo::log_lowpass_filter_cutoff();
      sound.log_lowpass_filter_cutoff = if last {Signal::constant (UserNumber::from_rendered (info.slider_range [1]))}
      else {random_signal (generator, sound.duration(), &info)};
    }
    if sound.log_highpass_filter_cutoff.range() [1] > sound.log_frequency.range() [0] {
      let info = SignalInfo::log_highpass_filter_cutoff();
      sound.log_highpass_filter_cutoff = if last {Signal::constant (UserNumber::from_rendered (info.slider_range [0]))}
      else {random_signal (generator, sound.duration(), &info)};
    }
    if sound.log_bitcrush_frequency.range() [0] < sound.log_frequency.range() [1] {
      sound.log_bitcrush_frequency = random_signal (generator, sound.duration(), &SignalInfo::log_bitcrush_frequency());
    }
  }
  
  sound
}
