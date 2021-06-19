use crate::cards::HandCard;
use crate::game::{
  Game, InteractionIntent, PlayerActionState, PlayerActiveInteraction, Time, UPDATE_DURATION,
};
use crate::map::{FloatingVector, FloatingVectorExtension, Rotation, TILE_RADIUS, TILE_WIDTH};
use crate::mechanisms::Mechanism;
use crate::ui_glue::Draw;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::Debug;
use trait_enum::trait_enum;

pub enum ActionStatus {
  StillGoing,
  Completed,
}

pub struct ActionUpdateContext<'a> {
  pub game: &'a mut Game,
}

impl<'a> ActionUpdateContext<'a> {
  pub fn interaction_state(&self) -> &PlayerActiveInteraction {
    match &self.game.player.action_state {
      PlayerActionState::Interacting(i) => i,
      _ => unreachable!(),
    }
  }
  pub fn this_card(&self) -> &HandCard {
    match self.interaction_state().activating_intent {
      InteractionIntent::PlayCard(index) => self.game.cards.hand.get(index).unwrap(),
      _ => unreachable!(),
    }
  }
  pub fn this_card_mut(&mut self) -> &mut HandCard {
    match self.interaction_state().activating_intent {
      InteractionIntent::PlayCard(index) => self.game.cards.hand.get_mut(index).unwrap(),
      _ => unreachable!(),
    }
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum Cost {
  Fixed(i32),
  Variable,
  None,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct ActionDisplayInfo {
  pub name: String,
  pub health_cost: Cost,
  pub time_cost: Cost,
  pub rules_text: String,
  pub flavor_text: String,
}

pub trait ActionTrait {
  /** Perform a single time-step update on this action, possibly modifying the game state.

  Note that the action is removed from `game` before doing this, so that both mutable references can be held at the same time, so the action still stored in `game` is temporarily invalid.
  */
  fn update(&mut self, context: ActionUpdateContext) -> ActionStatus;

  fn display_info(&self) -> ActionDisplayInfo;

  fn draw(&self, game: &Game, draw: &mut dyn Draw);
}

macro_rules! action_enum {
  ($($Variant: ident,)*) => {
    trait_enum!{
      #[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
      pub enum Action: ActionTrait {
        $($Variant,)*
      }
    }

    $(
    impl<'a> TryFrom<&'a Action> for &'a $Variant {
      type Error = ();
      fn try_from(value: &'a Action) -> Result<&'a $Variant, Self::Error> {
        if let Action::$Variant(s) = value {
          Ok(s)
        }
        else {
          Err(())
        }
      }
    }

    impl<'a> TryFrom<&'a mut Action> for &'a mut $Variant {
      type Error = ();
      fn try_from(value: &'a mut Action) -> Result<&'a mut $Variant, Self::Error> {
        if let Action::$Variant(s) = value {
          Ok(s)
        }
        else {
          Err(())
        }
      }
    }
    )*
  }

}

action_enum! {
  Redraw,
  RotateMechanism,
  BuildMechanism,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct SimpleAction {
  display_info: ActionDisplayInfo,

  progress: Time,
  cancel_progress: Time,
}

impl Default for ActionDisplayInfo {
  fn default() -> Self {
    ActionDisplayInfo {
      name: "".to_string(),
      health_cost: Cost::None,
      time_cost: Cost::Fixed(2),
      rules_text: "".to_string(),
      flavor_text: "".to_string(),
    }
  }
}

fn smootherstep(a: f64, b: f64, x: f64) -> f64 {
  let x = ((x - a) / (b - a)).clamp(0.0, 1.0);
  x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

impl SimpleAction {
  pub fn new(
    time_cost: i32,
    health_cost: Option<i32>,
    name: &str,
    rules_text: &str,
    flavor_text: &str,
  ) -> SimpleAction {
    SimpleAction {
      display_info: ActionDisplayInfo {
        name: name.to_string(),
        health_cost: match health_cost {
          Some(c) => Cost::Fixed(c),
          None => Cost::None,
        },
        time_cost: Cost::Fixed(time_cost),
        rules_text: rules_text.to_string(),
        flavor_text: flavor_text.to_string(),
      },
      progress: 0.0,
      cancel_progress: 0.0,
    }
  }
  fn time_cost(&self) -> f64 {
    match self.display_info.time_cost {
      Cost::Fixed(cost) => cost as f64,
      _ => panic!(),
    }
  }
  fn health_cost(&self) -> f64 {
    match self.display_info.health_cost {
      Cost::Fixed(cost) => cost as f64,
      Cost::None => 0.0,
      _ => panic!(),
    }
  }
  fn cooldown_time(&self) -> f64 {
    self.time_cost() * 0.25
  }
  fn startup_time(&self) -> f64 {
    self.time_cost() * 0.25
  }
  fn finish_time(&self) -> f64 {
    self.time_cost() - self.cooldown_time()
  }
  fn finished(&self) -> bool {
    self.progress > self.finish_time()
  }
  fn health_to_pay_by(&self, progress: f64) -> f64 {
    smootherstep(self.startup_time(), self.finish_time(), progress) * self.health_cost()
  }
  fn update_noncard(
    &mut self,
    context: ActionUpdateContext,
    finish: impl FnOnce(ActionUpdateContext),
  ) -> ActionStatus {
    let canceled = context.interaction_state().canceled && !self.finished();
    if canceled {
      self.cancel_progress += UPDATE_DURATION;
    } else {
      let was_finished = self.finished();
      let health_paid_already = self.health_to_pay_by(self.progress);
      self.progress += UPDATE_DURATION;
      let health_payment = self.health_to_pay_by(self.progress) - health_paid_already;
      context.game.player.health -= health_payment;
      if self.finished() > was_finished {
        finish(context);
      }
    }

    if self.progress > self.time_cost() || self.cancel_progress > self.cooldown_time() {
      ActionStatus::Completed
    } else {
      ActionStatus::StillGoing
    }
  }
  fn update_card(
    &mut self,
    context: ActionUpdateContext,
    finish: impl FnOnce(ActionUpdateContext),
  ) -> ActionStatus {
    self.update_noncard(context, |context| {
      match context.interaction_state().activating_intent {
        InteractionIntent::PlayCard(index) => {
          context
            .game
            .cards
            .discard_pile
            .push(context.game.cards.hand.remove(index).card);
        }
        _ => unreachable!(),
      }
      finish(context)
    })
  }

  fn display_info(&self) -> ActionDisplayInfo {
    self.display_info.clone()
  }

  fn draw(&self, game: &Game, draw: &mut dyn Draw) {
    let a = game.player.position + FloatingVector::new(-TILE_RADIUS as f64 * 0.5, 0.0);
    draw.rectangle_on_map(
      70,
      a,
      FloatingVector::new(TILE_RADIUS as f64 * 0.25, TILE_WIDTH as f64),
      "#000",
    );
    draw.rectangle_on_map(
      71,
      a,
      FloatingVector::new(
        TILE_RADIUS as f64 * 0.25,
        TILE_WIDTH as f64 * self.progress / self.time_cost(),
      ),
      "#ff0",
    );
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct RotateMechanism {
  amount: Rotation,
  simple: SimpleAction,
}

impl RotateMechanism {
  pub fn new(amount: Rotation) -> RotateMechanism {
    RotateMechanism {
      amount,
      simple: SimpleAction::new(1, None, "Rotate", "", "You pivot, and pivot, and pivot, and yet you never feel like you've never found your direction in life."),
    }
  }
}

impl ActionTrait for RotateMechanism {
  fn update(&mut self, context: ActionUpdateContext) -> ActionStatus {
    let amount = self.amount;
    self.simple.update_noncard(context, |context| {
      context
        .game
        .map
        .tiles
        .get_mut(&context.game.player.position.containing_tile())
        .unwrap()
        .mechanism
        .as_mut()
        .unwrap()
        .facing += amount;
    })
  }

  fn display_info(&self) -> ActionDisplayInfo {
    self.simple.display_info()
  }

  fn draw(&self, game: &Game, draw: &mut dyn Draw) {
    self.simple.draw(game, draw)
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct BuildMechanism {
  pub mechanism: Mechanism,
  pub simple: SimpleAction,
}

impl ActionTrait for BuildMechanism {
  fn update(&mut self, context: ActionUpdateContext) -> ActionStatus {
    let mechanism = self.mechanism.clone();
    self.simple.update_card(context, |context| {
      let tile = context
        .game
        .map
        .tiles
        .entry(context.game.player.position.containing_tile())
        .or_insert_with(Default::default);
      tile.mechanism = Some(mechanism);
    })
  }

  fn display_info(&self) -> ActionDisplayInfo {
    self.simple.display_info()
  }

  fn draw(&self, game: &Game, draw: &mut dyn Draw) {
    self.simple.draw(game, draw)
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Redraw {
  pub simple: SimpleAction,
}

impl Redraw {
  pub fn new() -> Redraw {
    Redraw {
      simple: SimpleAction::new(5, Some(50), "Redraw", "", ""),
    }
  }
}

impl ActionTrait for Redraw {
  fn update(&mut self, context: ActionUpdateContext) -> ActionStatus {
    self.simple.update_noncard(context, |context| {
      let cards = &mut context.game.cards;
      cards
        .discard_pile
        .extend(cards.hand.drain(..).map(|c| c.card));
      if cards.draw_pile.is_empty() {
        cards.discard_pile.shuffle(&mut rand::thread_rng());
        std::mem::swap(&mut cards.draw_pile, &mut cards.discard_pile);
      }
      cards.hand.extend(
        cards
          .draw_pile
          .drain(cards.draw_pile.len().saturating_sub(5)..)
          .map(|card| HandCard { card }),
      );
    })
  }

  fn display_info(&self) -> ActionDisplayInfo {
    self.simple.display_info()
  }

  fn draw(&self, game: &Game, draw: &mut dyn Draw) {
    self.simple.draw(game, draw)
  }
}
