use crate::actions::{Action, BuildMechanism};
use crate::mechanisms::{Conveyor, Mechanism, MechanismType};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Cards {
  pub draw_pile: Vec<CardInstance>,
  pub discard_pile: Vec<CardInstance>,
  pub hand: Vec<HandCard>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct HandCard {
  pub card: CardInstance,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CardInstance {
  pub action: Action,
}

impl CardInstance {
  pub fn build_conveyor() -> Self {
    CardInstance {
      action: Action::BuildMechanism(BuildMechanism::new(Mechanism {
        mechanism_type: MechanismType::Conveyor(Conveyor {}),
        ..Default::default()
      })),
    }
  }
}
