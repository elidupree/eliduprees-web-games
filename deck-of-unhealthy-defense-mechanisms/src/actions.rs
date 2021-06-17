use crate::cards::{CardInstance, HandCard};
use crate::game::{
  Game, InteractionIntent, PlayerActionState, PlayerActiveInteraction, Time, UPDATE_DURATION,
};
use crate::map::{FloatingVector, FloatingVectorExtension, Rotation, TILE_RADIUS, TILE_WIDTH};
use crate::mechanisms::Mechanism;
use crate::ui_glue::Draw;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

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

pub trait ActionTrait {
  /** Perform a single time-step update on this action, possibly modifying the game state.

  Note that the action is removed from `game` before doing this, so that both mutable references can be held at the same time, so the action still stored in `game` is temporarily invalid.
  */
  fn update(&mut self, context: ActionUpdateContext) -> ActionStatus;

  fn draw(&self, game: &Game, draw: &mut impl Draw);
}

macro_rules! action_enum {
  ($($Variant: ident,)*) => {
    #[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
    pub enum Action {
      $($Variant($Variant),)*
    }

    impl ActionTrait for Action {
      fn update(&mut self, context: ActionUpdateContext) -> ActionStatus {
        match self {
          $(Action::$Variant(s) => s.update(context),)*
        }
      }

      fn draw(&self, game: &Game, draw: &mut impl Draw) {
        match self {
          $(Action::$Variant(s) => s.draw(game, draw),)*
        }
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
  RotateMechanism,
  BuildMechanism,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
struct SimpleAction {
  time_cost: Time,
  startup_time: Time,
  cooldown_time: Time,
  health_cost: i32,

  progress: Time,
  cancel_progress: Time,
}

impl Default for SimpleAction {
  fn default() -> Self {
    SimpleAction {
      time_cost: 1.7,
      startup_time: 0.5,
      cooldown_time: 0.5,
      health_cost: 0,

      progress: 0.0,
      cancel_progress: 0.0,
    }
  }
}

fn smootherstep(a: f64, b: f64, x: f64) -> f64 {
  let x = ((x - a) / (b - a)).clamp(0.0, 1.0);
  x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

impl SimpleAction {
  fn finish_time(&self) -> f64 {
    self.time_cost - self.cooldown_time
  }
  fn finished(&self) -> bool {
    self.progress > self.finish_time()
  }
  fn health_to_pay_by(&self, progress: f64) -> i32 {
    (smootherstep(self.startup_time, self.finish_time(), progress) * self.health_cost as f64)
      .round() as i32
  }
  fn update(
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

    if self.progress > self.time_cost || self.cancel_progress > self.cooldown_time {
      ActionStatus::Completed
    } else {
      ActionStatus::StillGoing
    }
  }

  fn draw(&self, game: &Game, draw: &mut impl Draw) {
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
        TILE_WIDTH as f64 * self.progress / self.time_cost,
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
      simple: SimpleAction {
        time_cost: 1.1,
        ..Default::default()
      },
    }
  }
}

impl ActionTrait for RotateMechanism {
  fn update(&mut self, context: ActionUpdateContext) -> ActionStatus {
    let amount = self.amount;
    self.simple.update(context, |context| {
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

  fn draw(&self, game: &Game, draw: &mut impl Draw) {
    self.simple.draw(game, draw)
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct BuildMechanism {
  mechanism: Mechanism,
  simple: SimpleAction,
}

impl BuildMechanism {
  pub fn new(mechanism: Mechanism) -> BuildMechanism {
    BuildMechanism {
      mechanism,
      simple: SimpleAction {
        time_cost: 1.7,
        health_cost: 10,
        ..Default::default()
      },
    }
  }
}

impl ActionTrait for BuildMechanism {
  fn update(&mut self, context: ActionUpdateContext) -> ActionStatus {
    let mechanism = self.mechanism.clone();
    self.simple.update(context, |context| {
      let tile = context
        .game
        .map
        .tiles
        .entry(context.game.player.position.containing_tile())
        .or_insert_with(Default::default);
      tile.mechanism = Some(mechanism);
    })
  }

  fn draw(&self, game: &Game, draw: &mut impl Draw) {
    self.simple.draw(game, draw)
  }
}
