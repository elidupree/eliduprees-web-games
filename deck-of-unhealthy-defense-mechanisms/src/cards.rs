use crate::actions::{Action, BuildMechanism, SimpleAction};
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
      action: Action::BuildMechanism(BuildMechanism {
        mechanism: Mechanism {
          mechanism_type: MechanismType::Conveyor(Conveyor {}),
          ..Default::default()
        },
        simple: SimpleAction::new(2, Some(10), "Conveyor", "", "No matter how low you get, something keeps you moving forward. Is it hope for something better? Or is it just an endless grind, false hope leading you down the same corridor again and again and again?")
      }),
    }
  }
}
