use crate::actions::{Action, BuildConveyor, BuildMechanism, Cost, SimpleAction};
use crate::game::Game;
use crate::map::{FloatingVector, TILE_WIDTH};
use crate::mechanisms::{Mechanism, MechanismType, Tower};
use crate::ui_glue::Draw;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Cards {
  pub deck: Vec<CardInstance>,
  pub selected_index: Option<usize>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CardInstance {
  pub action: Action,
}

impl CardInstance {
  pub fn basic_conveyor() -> Self {
    CardInstance {
      action: Action::BuildConveyor(BuildConveyor {
        allow_splitting: false,
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
  pub fn selected(&self) -> Option<&CardInstance> {
    self
      .selected_index
      .map(|index| self.deck.get(index).unwrap())
  }
  pub fn selected_mut(&mut self) -> Option<&mut CardInstance> {
    self
      .selected_index
      .map(move |index| self.deck.get_mut(index).unwrap())
  }
  pub fn draw(&self, game: &Game, draw: &mut impl Draw) {
    let activation = game.current_mechanism_activation();
    fn draw_action(
      draw: &mut impl Draw,
      action: &Action,
      possible: bool,
      position: FloatingVector,
      size: f64,
    ) {
      let info = action.display_info();
      let color = if possible { "#cc0" } else { "#aaa" };
      draw.text(position, size, color, &info.name);
      if let Cost::Fixed(cost) = info.time_cost {
        draw.text(
          position + FloatingVector::new(0.0, 0.05 * (size / 28.0)),
          size,
          color,
          &format!("{} time", cost),
        );
      }
      if let Cost::Fixed(cost) = info.health_cost {
        draw.text(
          position + FloatingVector::new(0.0, 0.10 * (size / 28.0)),
          size,
          color,
          &format!("{} health", cost),
        );
      }
    }

    if let Some(action) = activation {
      draw_action(
        draw,
        &action,
        action.possible(game),
        FloatingVector::new(0.8, 0.4),
        30.0,
      );
    }

    if let Some(selected) = self.selected() {
      draw_action(
        draw,
        &selected.action,
        selected.action.possible(game),
        FloatingVector::new(0.05, 0.4),
        30.0,
      );
      selected.action.draw_preview(game, draw);
      for (index, upcoming) in self.deck[self.selected_index.unwrap() + 1..]
        .iter()
        .enumerate()
        .take(2)
      {
        draw_action(
          draw,
          &upcoming.action,
          false,
          FloatingVector::new(0.03, 0.6 + (index as f64 * 0.14)),
          18.0,
        );
      }
    }
  }
}
