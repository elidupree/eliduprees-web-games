//use std::hash::{Hash, Hasher};
use std::cmp::{max};

use num::Integer;
use geometry::Number;
use machine_data::Material;
pub const RATE_DIVISOR: Number = ::machine_data::TIME_TO_MOVE_MATERIAL * 2*2*2*2*2*2 * 3*3*3 * 5*5;


#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct FlowRate {
  rate: Number, //items per max cycle length
}

#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct FlowPattern {
  start_time: Number,
  rate: FlowRate,
}

#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct MaterialFlow {
  pub material: Material,
  pub flow: FlowPattern,
}


impl FlowRate {
  pub fn new (rate: Number) -> FlowRate {
    if rate <= 0 {panic! ("Tried to construct a FlowRate with non-positive rate")}
    FlowRate {
      rate
    }
  }
  pub fn rate (&self)->Number {self.rate}
  
  fn fractional_progress_before (&self, time: Number)->Number {
    time*self.rate + RATE_DIVISOR - 1
  }
  pub fn num_disbursed_at_time (&self, time: Number)->Number {
    self.num_disbursed_before (time + 1) - self.num_disbursed_before (time)
  }
  // calibrated to return 0 for time 0, but this could be a misleading name for this function, so not making it public
  fn num_disbursed_before (&self, time: Number)->Number {
    self.fractional_progress_before (time).div_floor (&RATE_DIVISOR)
  }
  pub fn num_disbursed_between (&self, range: [Number; 2])->Number {
    self.num_disbursed_before (range [1]) - self.num_disbursed_before (range [0])
  }
  pub fn last_disbursement_before (&self, time: Number)->Number {
    let fractional_part = self.fractional_progress_before (time).mod_floor(&RATE_DIVISOR);
    let time_not_disbursing = fractional_part.div_floor (&self.rate);
    time-1 - time_not_disbursing
  }
  pub fn nth_disbursement_time (&self, n: Number)->Number {
    (n*RATE_DIVISOR).div_floor (&self.rate)
  }
  pub fn time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (&self, start_time: Number, target_amount: Number)->Number {
    let already_disbursed = self.num_disbursed_before (start_time);
    let target_amount = target_amount + already_disbursed;
    (target_amount*RATE_DIVISOR + self.rate - 1).div_floor(&self.rate)
  }

}

impl FlowPattern {
  pub fn new (start_time: Number, rate: Number) -> FlowPattern {
    FlowPattern {
      start_time,
      rate: FlowRate::new (rate),
    }
  }
  pub fn start_time (&self)->Number {self.start_time}
  pub fn rate (&self)->Number {self.rate.rate()}
  pub fn delayed_by (&self, delay: Number)->FlowPattern {FlowPattern::new (self.start_time + delay, self.rate.rate())}
  
  pub fn num_disbursed_at_time (&self, time: Number)->Number {
    if time < self.start_time {return 0;}
    self.rate.num_disbursed_at_time (time - self.start_time)
  }
  pub fn num_disbursed_between (&self, range: [Number; 2])->Number {
    self.rate.num_disbursed_between ([max (0, range [0] - self.start_time), max (0, range [1] - self.start_time)])
  }
  
  pub fn num_disbursed_before (&self, time: Number)->Number {
    if time < self.start_time {return 0;}
    self.rate.num_disbursed_before (time - self.start_time)
  }
  pub fn last_disbursement_before (&self, time: Number)->Option <Number> {
    if time <= self.start_time {return None}
    Some (self.rate.last_disbursement_before (time - self.start_time) + self.start_time)
  }
  pub fn nth_disbursement_time (&self, n: Number)->Option <Number> {
    if n < 0 {return None;}
    Some (self.rate.nth_disbursement_time (n) + self.start_time)
  }
}

/*
impl MaterialFlow {
  pub fn new (material: Material, flow:FlowPattern) ->MaterialFlow {
    MaterialFlow {material, flow}
  }
  pub fn start_time (&self)->Number {self.flow.start_time()}
  pub fn rate (&self)->Number {self.flow.rate()}
  pub fn delayed_by (&self, delay: Number)->MaterialFlow {
    MaterialFlow {material: self.material, flow: self.flow.delayed_by (delay)}
  }
  
  pub fn num_disbursed_at_time (&self, time: Number)->Number {
    self.flow.num_disbursed_at_time (time)
  }
  pub fn num_disbursed_between (&self, range: [Number; 2])->Number {
    self.flow.num_disbursed_between (range)
  }
  
  pub fn num_disbursed_before (&self, time: Number)->Number {
    self.flow.num_disbursed_before (time)
  }
  pub fn last_disbursement_before (&self, time: Number)->Option <Number> {
    self.flow.last_disbursement_before (time)
  }
  pub fn nth_disbursement_time (&self, n: Number)->Option <Number> {
    if n < 0 {return None;}
    Some (self.rate.nth_disbursement_time (n) + self.start_time)
  }
}*/

/*

pub fn time_from_which_patterns_will_always_disburse_at_least_amount_plus_ideal_rate_in_total <I: IntoIterator <Item = FlowPattern> + Clone> (patterns: I, start_time: Number, target_amount: Number)->Option <Number> {
  let mut already_disbursed = 0;
  let mut total_rate = 0;
  let mut max_rounding_loss = 0;
  for pattern in patterns.clone().into_iter() {
    if pattern.rate > 0 {
      total_rate += pattern.rate;
      max_rounding_loss += RATE_DIVISOR - 1;
      already_disbursed += pattern.num_disbursed_before (start_time);
    }
  }
  let target_amount = target_amount + already_disbursed;
  if total_rate <= 0 && target_amount > 0 {return None;}
  let fractional_progress_before_start: Number = patterns.into_iter().filter (| pattern | pattern.rate > 0).map (| pattern | pattern.fractional_progress_before (start_time)).sum();
  Some (start_time + (
    target_amount*RATE_DIVISOR + max_rounding_loss - fractional_progress_before_start
    + total_rate - 1).div_floor(&total_rate)
  )
}

*/

#[cfg (test)]
mod tests {
  use super::*;
  
  use std::iter;
  
  fn assert_flow_pattern (rate: Number, prefix: & [Number]) {
    assert_eq! (
      prefix,
      (0..prefix.len()).map (| index | FlowPattern::new(0, rate).num_disbursed_at_time (index as Number)).collect::<Vec <_>>().as_slice()
    );
  }
  
  #[test]
  fn flow_pattern_unit_tests() {
    assert_flow_pattern (RATE_DIVISOR, &[1, 1, 1, 1]);
    assert_flow_pattern (RATE_DIVISOR/2, &[1, 0, 1, 0, 1, 0, 1, 0]);
    assert_flow_pattern (RATE_DIVISOR/3, &[1, 0, 0, 1, 0, 0, 1, 0]);
    assert_flow_pattern (RATE_DIVISOR*2/3, &[1, 1, 0, 1, 1, 0, 1, 1]);
  }
  
  proptest! {
    #[test]
    fn randomly_test_flow_rate_density_property(rate in 1..=RATE_DIVISOR, initial_time in -1000000i64..1000000, duration in 0i64..1000000) {
      let ideal_rounded_down = rate*duration/RATE_DIVISOR;
      let ideal_rounded_up = (rate*duration + RATE_DIVISOR - 1)/RATE_DIVISOR;
      let observed = FlowRate::new (rate).num_disbursed_between ([initial_time, initial_time + duration]);
      prop_assert!(observed >= ideal_rounded_down);
      prop_assert!(observed <= ideal_rounded_up);
    }
    
    #[test]
    fn randomly_test_flow_rate_cycle_property(rate in 0..=RATE_DIVISOR, initial_time in -1000000i64..1000000, test_time in -1000000i64..1000000) {
      let rate = FlowRate::new (rate);
      prop_assert_eq!(
        rate.num_disbursed_between ([initial_time, test_time]),
        rate.num_disbursed_between ([initial_time + RATE_DIVISOR, test_time + RATE_DIVISOR])
      );
    }
    
    #[test]
    fn randomly_test_flow_rate_density_rounds_up_from_beginning (rate in 0..=RATE_DIVISOR, duration in 0i64..1000000) {
      let ideal_rounded_up = (rate*duration + RATE_DIVISOR - 1)/RATE_DIVISOR;
      let observed = FlowRate::new (rate).num_disbursed_between ([0, 0 + duration]);
      prop_assert_eq!(observed, ideal_rounded_up);
    }
    
    #[test]
    fn randomly_test_last_disbursement_before (start in -1000000i64..1000000, rate in 1..=RATE_DIVISOR, initial_time in -1000000i64..1000000) {
      let initial_time = initial_time + start;
      let pattern = FlowPattern::new (start, rate);
      let observed = pattern.last_disbursement_before (initial_time);
      match observed {
        None => {
          prop_assert_eq!(pattern.num_disbursed_before(initial_time), 0);
        }
        Some (observed) => {
          println!("{}", observed);
          prop_assert! (observed <initial_time) ;
          prop_assert_eq!(pattern.num_disbursed_between ([observed+1, initial_time]), 0);
          prop_assert_eq!(pattern.num_disbursed_between ([observed, initial_time]), 1);
        }
      }
    }
    
    #[test]
    fn randomly_test_nth_disbursement_time (start in -1000000i64..1000000, rate in 1..=RATE_DIVISOR, n in -1000000i64..1000000) {
      let flowrate = FlowRate::new (rate);
      let observed = flowrate.nth_disbursement_time (n).unwrap();
      println!("{}, {}, {}", observed, flowrate.num_disbursed_before (observed), flowrate.num_disbursed_before (observed + 1));
      prop_assert_eq!(flowrate.num_disbursed_before (observed), n);
      prop_assert_eq!(flowrate.num_disbursed_before (observed + 1), n+1);
      
      if n > 0 {
        let pattern = FlowPattern::new (start, rate);
        let observed = pattern.nth_disbursement_time (n).unwrap();
        println!("{}, {}, {}", observed, pattern.num_disbursed_before (observed), pattern.num_disbursed_before (observed + 1));
        prop_assert_eq!(pattern.num_disbursed_before (observed), n);
        prop_assert_eq!(pattern.num_disbursed_before (observed + 1), n+1);
      }
    }
    
    #[test]
    fn randomly_test_time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (rate in 1..=RATE_DIVISOR, amount in -1000000i64..1000000, initial_time in  -1000000i64..1000000, duration in 0i64..1000000) {
      let rate = FlowRate::new (rate);
      let observed = pattern.time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (initial_time, amount).unwrap();
      let ideal_count_rounded_up = amount + (rate*(duration+1) + RATE_DIVISOR - 1)/RATE_DIVISOR;
      let observed_count = pattern.num_disbursed_between([initial_time, observed + duration + 1]);
      println!("{}, {}, {}", observed, ideal_count_rounded_up, observed_count);
      prop_assert!(observed_count >= ideal_count_rounded_up);
    }

    /*#[test]
    fn randomly_test_at_least_amount_plus_ideal_rate_functions_are_consistent (start in -1000000i64..1000000, rate in 0..=RATE_DIVISOR, amount in -100000i64..1000000, initial_time in  -1000000i64..1000000) {
      let pattern = FlowPattern::new (start, rate);
      let single = pattern.time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (initial_time, amount);
      let collection = time_from_which_patterns_will_always_disburse_at_least_amount_plus_ideal_rate_in_total (iter::once(pattern), initial_time, amount);
      prop_assert_eq!(single, collection);
    }*/
  }
}
