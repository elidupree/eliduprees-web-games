use crate::actions::{Action, BuildMechanism, SimpleAction};
use crate::mechanisms::{Conveyor, Mechanism, MechanismType, Tower};
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
  pub fn basic_conveyor() -> Self {
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
  pub fn basic_tower() -> Self {
    CardInstance {
      action: Action::BuildMechanism(BuildMechanism {
        mechanism: Mechanism {
          mechanism_type: MechanismType::Tower(Tower {maximum_volition:5.0,range:5.0,..Default::default()}),
          ..Default::default()
        },
        simple: SimpleAction::new(5, Some(40), "Defensive Tower", "", "You think *I* have a problem?! *You're* the monsters who are trying to kill me! Why won't you just shut up already?!"),
      }),
    }
  }
}
