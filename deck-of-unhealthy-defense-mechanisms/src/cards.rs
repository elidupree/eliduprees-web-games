use crate::actions::{Action, BuildMechanism, Cost, SimpleAction};
use crate::game::Game;
use crate::map::{FloatingVector, TILE_WIDTH};
use crate::mechanisms::{Conveyor, Mechanism, MechanismType, Tower};
use crate::ui_glue::Draw;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Cards {
  pub draw_pile: Vec<CardInstance>,
  pub discard_pile: Vec<CardInstance>,
  pub hand: Vec<HandCard>,
  pub selected: Option<usize>,
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
          mechanism_type: MechanismType::Tower(Tower {maximum_volition:5.0,range:5.0 * TILE_WIDTH as f64,..Default::default()}),
          ..Default::default()
        },
        simple: SimpleAction::new(5, Some(40), "Defensive Tower", "", "You think *I* have a problem?! *You're* the monsters who are trying to kill me! Why won't you just shut up already?!"),
      }),
    }
  }
}

impl Cards {
  pub fn draw(&self, game: &Game, draw: &mut impl Draw) {
    let [left, right] = game.interactions();
    let actions: Vec<_> = std::iter::once(left.as_ref().map(|a| (a, false)))
      .chain((0..5).map(|index| {
        self
          .hand
          .get(index)
          .map(|card| (&card.card.action, self.selected == Some(index)))
      }))
      .chain(std::iter::once(right.as_ref().map(|a| (a, false))))
      .collect();
    for (index, &action) in actions.iter().enumerate() {
      if let Some((action, selected)) = action {
        let info = action.display_info();
        let horizontal = (index as f64 + 0.1) / actions.len() as f64;
        let possible = action.possible(game);
        let color = if possible { "#cc0" } else { "#aaa" };
        let size = if selected { 28.0 } else { 24.0 };
        let offset = if selected { 0.02 } else { 0.0 };
        draw.text(
          FloatingVector::new(horizontal, 0.8 - offset),
          size,
          color,
          &info.name,
        );
        if let Cost::Fixed(cost) = info.time_cost {
          draw.text(
            FloatingVector::new(horizontal, 0.85 - offset),
            size,
            color,
            &format!("{} time", cost),
          );
        }
        if let Cost::Fixed(cost) = info.health_cost {
          draw.text(
            FloatingVector::new(horizontal, 0.9 - offset),
            size,
            color,
            &format!("{} health", cost),
          );
        }
      }
    }

    if let Some(index) = self.selected {
      self
        .hand
        .get(index)
        .unwrap()
        .card
        .action
        .draw_preview(game, draw);
    }
  }
}
