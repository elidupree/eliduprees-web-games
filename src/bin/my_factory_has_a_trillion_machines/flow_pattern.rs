//use std::hash::{Hash, Hasher};
//use std::cmp::{max};

use num::Integer;
use geometry::Number;
pub const RATE_DIVISOR: Number = ::machine_data::TIME_TO_MOVE_MATERIAL * 2*2*2*2*2*2 * 3*3*3 * 5*5;


#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct FlowPattern {
  origin_time: Number, //when the first item was disbursed as part of this flow
  rate: Number, //items per max cycle length
}

impl FlowPattern {
  pub fn new (origin_time: Number, rate: Number) -> FlowPattern {
    FlowPattern {
      origin_time: if rate == 0 {0} else { origin_time.mod_floor (&RATE_DIVISOR)},
      rate
    }
  }
  pub fn rate (&self)->Number {self.rate}
  pub fn delayed_by (&self, delay: Number)->FlowPattern {FlowPattern::new (self.origin_time + delay, self.rate)}
  
  
  fn fractional_progress_before (&self, time: Number)->Number {
    ((time - self.origin_time)*self.rate + RATE_DIVISOR - 1)
  }
  pub fn num_disbursed_at_time (&self, time: Number)->Number {
    self.num_disbursed_before (time + 1) - self.num_disbursed_before (time)
  }
  fn num_disbursed_before (&self, time: Number)->Number {
    self.fractional_progress_before (time).div_floor (&RATE_DIVISOR)
  }
  pub fn num_disbursed_between (&self, range: [Number; 2])->Number {
    self.num_disbursed_before (range [1]) - self.num_disbursed_before (range [0])
  }
  pub fn last_disbursement_before (&self, time: Number)->Option <Number> {
    if self.rate <= 0 {return None;}
    let fractional_part = self.fractional_progress_before (time).mod_floor(&RATE_DIVISOR);
    let time_not_disbursing = fractional_part.div_floor (&self.rate);
    Some(time-1 - time_not_disbursing)
  }
  pub fn nth_disbursement (&self, n: Number)->Option <Number> {
    if self.rate <= 0 {return None;}
    Some (self.origin_time + (n*RATE_DIVISOR).div_floor (&self.rate))
  }
  pub fn time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (&self, start_time: Number, target_amount: Number)->Option <Number> {
    if self.rate <= 0 && target_amount > 0 {return None;}
    let already_disbursed = self.num_disbursed_before (start_time);
    let target_amount = target_amount + already_disbursed;
    Some (self.origin_time + (target_amount*RATE_DIVISOR + self.rate - 1).div_floor(&self.rate))
  }

}


/*
impl PartialEq for FlowPattern {
  fn eq (&self, other: & Self)->bool {
    self.rate == other.rate && (self.rate == 0 || self.origin_time == other.origin_time)
  }
}
impl Hash for FlowPattern {
  fn hash<H: Hasher> (&self, hasher: &mut H) {
    self.rate.hash (hasher);
    if self.rate != 0 {
      self.origin_time.hash (hasher);
    }
  }
}
*/

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
    fn randomly_test_flow_pattern_density_property(start in -1000000i64..1000000, rate in 0..=RATE_DIVISOR, initial_time in -1000000i64..1000000, duration in 0i64..1000000) {
      let ideal_rounded_down = rate*duration/RATE_DIVISOR;
      let ideal_rounded_up = (rate*duration + RATE_DIVISOR - 1)/RATE_DIVISOR;
      let observed = FlowPattern::new (start, rate).num_disbursed_between ([initial_time, initial_time + duration]);
      prop_assert!(observed >= ideal_rounded_down);
      prop_assert!(observed <= ideal_rounded_up);
    }
    
    #[test]
    fn randomly_test_flow_pattern_cycle_property(start in -1000000i64..1000000, rate in 0..=RATE_DIVISOR, initial_time in -1000000i64..1000000, test_time in -1000000i64..1000000) {
      let real_pattern = FlowPattern::new (start, rate);
      let noncanonical_pattern = FlowPattern {origin_time: start, rate};
      let interval = [initial_time, test_time];
      prop_assert_eq!(real_pattern.num_disbursed_between (interval), noncanonical_pattern.num_disbursed_between (interval));
    }
    
    #[test]
    fn randomly_test_flow_pattern_density_rounds_up_from_beginning (start in -1000000i64..1000000, rate in 0..=RATE_DIVISOR, duration in 0i64..1000000) {
      let ideal_rounded_up = (rate*duration + RATE_DIVISOR - 1)/RATE_DIVISOR;
      let observed = FlowPattern::new (start, rate).num_disbursed_between ([start, start + duration]);
      prop_assert_eq!(observed, ideal_rounded_up);
    }
    
    #[test]
    fn randomly_test_last_disbursement_before (start in -1000000i64..1000000, rate in 1..=RATE_DIVISOR, initial_time in -1000000i64..1000000) {
      let initial_time = initial_time + start;
      let pattern = FlowPattern::new (start, rate);
      let observed = pattern.last_disbursement_before (initial_time).unwrap();
      println!("{}", observed);
      prop_assert! (observed <initial_time) ;
      prop_assert_eq!(pattern.num_disbursed_between ([observed+1, initial_time]), 0);
      prop_assert_eq!(pattern.num_disbursed_between ([observed, initial_time]), 1);
    }
    
    #[test]
    fn randomly_test_nth_disbursement (start in -1000000i64..1000000, rate in 1..=RATE_DIVISOR, n in -1000000i64..1000000) {
      let pattern = FlowPattern::new (start, rate);
      let observed = pattern.nth_disbursement(n).unwrap();
      println!("{}, {}, {}", observed, pattern.num_disbursed_before (observed), pattern.num_disbursed_before (observed + 1));
      prop_assert_eq!(pattern.num_disbursed_before (observed), n);
      prop_assert_eq!(pattern.num_disbursed_before (observed + 1), n+1);
    }
    
    #[test]
    fn randomly_test_time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (start in -1000000i64..1000000, rate in 1..=RATE_DIVISOR, amount in -1000000i64..1000000, initial_time in  -1000000i64..1000000, duration in 0i64..1000000) {
      let pattern = FlowPattern::new (start, rate);
      let observed = pattern.time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (initial_time, amount).unwrap();
      let ideal_count_rounded_up = amount + (rate*(duration+1) + RATE_DIVISOR - 1)/RATE_DIVISOR;
      let observed_count = pattern.num_disbursed_between([initial_time, observed + duration + 1]);
      println!("{}, {}, {}", observed, ideal_count_rounded_up, observed_count);
      prop_assert!(observed_count >= ideal_count_rounded_up);
    }

    #[test]
    fn randomly_test_at_least_amount_plus_ideal_rate_functions_are_consistent (start in -1000000i64..1000000, rate in 0..=RATE_DIVISOR, amount in -100000i64..1000000, initial_time in  -1000000i64..1000000) {
      let pattern = FlowPattern::new (start, rate);
      let single = pattern.time_from_which_this_will_always_disburse_at_least_amount_plus_ideal_rate (initial_time, amount);
      let collection = time_from_which_patterns_will_always_disburse_at_least_amount_plus_ideal_rate_in_total (iter::once(pattern), initial_time, amount);
      prop_assert_eq!(single, collection);
    }
  }
}
