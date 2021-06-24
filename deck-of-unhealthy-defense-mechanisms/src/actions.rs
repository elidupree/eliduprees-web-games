use crate::cards::CardInstance;
use crate::game::{Game, PlayerActionState, PlayerActiveInteraction, Time, UPDATE_DURATION};
use crate::map::{
  Facing, FloatingVector, FloatingVectorExtension, GridVector, GridVectorExtension, Rotation,
  TILE_RADIUS, TILE_SIZE, TILE_WIDTH,
};
use crate::mechanisms::{Conveyor, ConveyorSide, Mechanism, MechanismType};
use crate::ui_glue::Draw;
use guard::guard;
use ordered_float::OrderedFloat;
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
  pub fn this_card(&self) -> &CardInstance {
    self.game.cards.selected().unwrap()
  }
  pub fn this_card_mut(&mut self) -> &mut CardInstance {
    self.game.cards.selected_mut().unwrap()
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

#[allow(unused)]
pub trait ActionTrait {
  /** Perform a single time-step update on this action, possibly modifying the game state.

  Note that the action is removed from `game` before doing this, so that both mutable references can be held at the same time, so the action still stored in `game` is temporarily invalid.
  */
  fn update(&mut self, context: ActionUpdateContext) -> ActionStatus;

  fn display_info(&self) -> ActionDisplayInfo;
  fn possible(&self, game: &Game) -> bool {
    true
  }

  fn draw_progress(&self, game: &Game, draw: &mut dyn Draw);
  fn draw_preview(&self, game: &Game, draw: &mut dyn Draw) {}
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
  Reshuffle,
  BuildConveyor,
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
      match context.game.cards.selected_index {
        Some(index) => {
          if index + 1 == context.game.cards.deck.len() {
            context.game.cards.selected_index = None;
          } else {
            context.game.cards.selected_index = Some(index + 1);
          }
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
        .get_mut(context.game.player.position.containing_tile())
        .unwrap();
      tile.mechanism = Some(mechanism);
    })
  }

  fn display_info(&self) -> ActionDisplayInfo {
    self.simple.display_info()
  }

  fn possible(&self, game: &Game) -> bool {
    game
      .map
      .tiles
      .get(game.player.position.containing_tile())
      .map_or(false, |tile| tile.mechanism.is_none())
  }

  fn draw_progress(&self, game: &Game, draw: &mut dyn Draw) {
    self.simple.draw(game, draw)
  }
  fn draw_preview(&self, game: &Game, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      5,
      game.player.position.containing_tile().to_floating(),
      TILE_SIZE.to_floating(),
      "#666",
    );
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct BuildConveyor {
  pub allow_splitting: bool,
  pub simple: SimpleAction,
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
struct BuildConveyorCandidate {
  position: GridVector,
  input_side: Facing,
}

impl BuildConveyorCandidate {
  fn input_position(&self) -> GridVector {
    self.position + self.input_side.unit_vector() * TILE_WIDTH
  }
  fn output_side(&self) -> Facing {
    self.input_side + Rotation::U_TURN
  }
}

impl BuildConveyor {
  fn candidate_valid(
    game: &Game,
    candidate: BuildConveyorCandidate,
    allow_splitting: bool,
  ) -> bool {
    let input_mechanism = game
      .map
      .tiles
      .get(candidate.input_position())
      .and_then(|here| here.mechanism.as_ref());
    guard!(let Some(output_tile) = game.map.tiles.get(candidate.position) else { return false });
    let output_mechanism = output_tile.mechanism.as_ref();

    //debug!("{:?}", (candidate, input_mechanism, output_mechanism));

    guard!(let Some(input_mechanism) = input_mechanism else { return false });
    if !input_mechanism
      .mechanism_type
      .can_be_material_source(candidate.output_side())
    {
      return false;
    }
    if !allow_splitting {
      if matches!(&input_mechanism.mechanism_type, MechanismType::Conveyor(conveyor) if conveyor.sides.iter().filter(|&&side| side == ConveyorSide::Output).count() > 0)
      {
        return false;
      }
    }

    if let Some(output_mechanism) = output_mechanism {
      guard!(let Mechanism { mechanism_type: MechanismType::Conveyor(conveyor), .. } = output_mechanism else { return false });
      if conveyor.sides[candidate.input_side.as_index()] != ConveyorSide::Disconnected {
        return false;
      }
    }

    true
  }
  /// the returned facing is the input side of the new conveyor
  fn current_target(game: &Game, allow_splitting: bool) -> Option<BuildConveyorCandidate> {
    let player_position = game.player.position.containing_tile();
    let player_offset = game.player.position - player_position.to_floating();
    let mut candidates = Vec::new();
    let mut consider = |candidate, score| {
      if Self::candidate_valid(game, candidate, allow_splitting) {
        candidates.push((candidate, score))
      }
    };

    for facing in Facing::ALL_FACINGS {
      consider(
        BuildConveyorCandidate {
          position: player_position,
          input_side: facing,
        },
        (player_offset - facing.unit_vector().to_floating()).magnitude_squared(),
      );
      consider(
        BuildConveyorCandidate {
          position: player_position - facing.unit_vector() * TILE_WIDTH,
          input_side: facing,
        },
        (player_offset - -facing.unit_vector().to_floating()).magnitude_squared(),
      );
    }
    candidates
      .into_iter()
      .min_by_key(|&(_, score)| OrderedFloat(score))
      .map(|(c, _)| c)
  }
}

impl ActionTrait for BuildConveyor {
  fn update(&mut self, context: ActionUpdateContext) -> ActionStatus {
    let allow_splitting = self.allow_splitting;
    self.simple.update_card(context, |context| {
      let candidate = Self::current_target(context.game, allow_splitting).unwrap();
      let tile = context.game.map.tiles.get_mut(candidate.position).unwrap();
      let mut sides = [ConveyorSide::Disconnected; 4];
      sides[candidate.input_side.as_index()] = ConveyorSide::Input;
      tile.mechanism = Some(Mechanism {
        mechanism_type: MechanismType::Conveyor(Conveyor {
          sides,
          last_sent: Facing::from_index(0),
        }),
      });

      let input_tile = context
        .game
        .map
        .tiles
        .get_mut(candidate.input_position())
        .unwrap();
      if let Some(Mechanism {
        mechanism_type: MechanismType::Conveyor(Conveyor { sides, .. }),
        ..
      }) = &mut input_tile.mechanism
      {
        sides[candidate.output_side().as_index()] = ConveyorSide::Output;
      }
    })
  }

  fn display_info(&self) -> ActionDisplayInfo {
    self.simple.display_info()
  }

  fn possible(&self, game: &Game) -> bool {
    Self::current_target(game, self.allow_splitting).is_some()
  }

  fn draw_progress(&self, game: &Game, draw: &mut dyn Draw) {
    self.simple.draw(game, draw)
  }
  fn draw_preview(&self, game: &Game, draw: &mut dyn Draw) {
    if let Some(candidate) = Self::current_target(game, self.allow_splitting) {
      draw.rectangle_on_map(
        5,
        candidate.position.to_floating(),
        TILE_SIZE.to_floating(),
        "#666",
      );
      draw.rectangle_on_map(
        5,
        candidate.input_position().to_floating(),
        TILE_SIZE.to_floating(),
        "#555",
      );
    } else {
      draw.rectangle_on_map(
        5,
        game.player.position.containing_tile().to_floating(),
        TILE_SIZE.to_floating(),
        "#555",
      );
    }
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Reshuffle {
  pub simple: SimpleAction,
}

impl Reshuffle {
  pub fn new() -> Reshuffle {
    Reshuffle {
      simple: SimpleAction::new(5, Some(50), "Reshuffle", "", ""),
    }
  }
}

impl ActionTrait for Reshuffle {
  fn update(&mut self, context: ActionUpdateContext) -> ActionStatus {
    self.simple.update_noncard(context, |context| {
      let cards = &mut context.game.cards;
      cards.deck.shuffle(&mut rand::thread_rng());
      cards.selected_index = Some(0);
    })
  }

  fn display_info(&self) -> ActionDisplayInfo {
    self.simple.display_info()
  }

  fn draw_progress(&self, game: &Game, draw: &mut dyn Draw) {
    self.simple.draw(game, draw)
  }
}
