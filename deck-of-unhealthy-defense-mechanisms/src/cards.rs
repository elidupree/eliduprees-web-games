use serde::{Deserialize, Serialize};
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Cards {
  pub draw_pile: Vec<CardInstance>,
  pub discard_pile: Vec<CardInstance>,
  pub hand: Vec<CardInstance>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct CardInstance {}
