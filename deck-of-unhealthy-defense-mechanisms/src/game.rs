use crate::actions::{
  Action, ActionStatus, ActionTrait, ActionUpdateContext, BuildMechanism, RotateMechanism,
};
use crate::cards::{CardInstance, Cards, HandCard};
use crate::map::{
  FloatingVector, FloatingVectorExtension, GridVector, GridVectorExtension, Map, Rotation, Tile,
  TILE_RADIUS, TILE_SIZE, TILE_WIDTH,
};
use crate::mechanisms::{
  Conveyor, Deck, Mechanism, MechanismImmutableContext, MechanismTrait, MechanismType,
};
use crate::ui_glue::Draw;
use eliduprees_web_games_lib::auto_constant;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Time = f64;
/// duration of each update in seconds:
pub const UPDATE_DURATION: Time = 1.0 / 180.0;

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Game {
  pub map: Map,
  pub player: Player,
  pub cards: Cards,
  pub time: Time,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Player {
  pub position: FloatingVector,
  pub action_state: PlayerActionState,
  pub already_begun_interaction_intent: Option<InteractionIntent>,
  pub maximum_health: i32,
  pub health: i32,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum PlayerInteractionCommitment {
  Performing { what: InteractionIntent },
  Canceled,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct PlayerActiveInteraction {
  pub activating_intent: InteractionIntent,
  pub action: Action,
  pub canceled: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum PlayerActionState {
  Moving { velocity: FloatingVector },
  Interacting(PlayerActiveInteraction),
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub enum InteractionIntent {
  PlayCard(usize),
  InteractLeft,
  InteractRight,
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum Intent {
  Move(FloatingVector),
  Interact(InteractionIntent),
}

impl Game {
  pub fn new() -> Self {
    let mut tiles: HashMap<GridVector, Tile> = HashMap::new();
    tiles
      .entry(GridVector::zeros())
      .or_insert_with(Default::default)
      .mechanism = Some(Mechanism {
      mechanism_type: MechanismType::Deck(Deck {}),
      ..Default::default()
    });
    Game {
      map: Map { tiles },
      player: Player {
        position: FloatingVector::zeros(),
        action_state: PlayerActionState::Moving {
          velocity: FloatingVector::zeros(),
        },
        already_begun_interaction_intent: None,
        maximum_health: 100,
        health: 100,
      },
      cards: Cards {
        draw_pile: vec![],
        discard_pile: vec![],
        hand: vec![HandCard {
          card: CardInstance::build_conveyor(),
        }],
      },
      time: 0.0,
    }
  }

  fn interactions(&self) -> [Option<Action>; 2] {
    let position = self.player.position.containing_tile();
    if let Some(tile) = self.map.tiles.get(&position) {
      if let Some(mechanism) = &tile.mechanism {
        return mechanism
          .mechanism_type
          .interactions(MechanismImmutableContext {
            position,
            map: &self.map,
          });
      }
    }
    [None, None]
  }

  fn update(&mut self, intent: Intent) {
    match intent {
      Intent::Move(movement_intent) => {}
      Intent::Interact(what) => {
        if self.player.already_begun_interaction_intent != Some(what)
          && matches!(self.player.action_state, PlayerActionState::Moving { velocity } if velocity == FloatingVector::zeros())
        {
          let [left, right] = self.interactions();
          let action = match what {
            InteractionIntent::InteractLeft => left,
            InteractionIntent::InteractRight => right,
            InteractionIntent::PlayCard(index) => self
              .cards
              .hand
              .get(index)
              .map(|card| card.card.action.clone()),
          };

          if let Some(action) = action {
            self.player.action_state = PlayerActionState::Interacting(PlayerActiveInteraction {
              activating_intent: what,
              action,
              canceled: false,
            });

            self.player.already_begun_interaction_intent = Some(what);
          }
        }
      }
    }

    if !matches!(self.player.already_begun_interaction_intent, Some(what) if intent == Intent::Interact(what))
    {
      self.player.already_begun_interaction_intent = None;
    }

    match &mut self.player.action_state {
      PlayerActionState::Moving { velocity } => {
        let acceleration = auto_constant("player_acceleration", 8.0);
        let max_speed = auto_constant("player_max_speed", 1.4) * TILE_WIDTH as f64;
        let target;
        if let Intent::Move(mut movement_intent) = intent {
          movement_intent.limit_magnitude(1.0);
          target = movement_intent * max_speed;
        } else {
          target = FloatingVector::zeros();
        }
        let mut bonus = 1.0;
        let epsilon = 0.00001;
        if let Some(acceleration_direction) = (target - *velocity).try_normalize(epsilon) {
          if let Some(velocity_direction) = velocity.try_normalize(epsilon) {
            bonus += (-acceleration_direction.dot(&velocity_direction)).max(0.0)
              * auto_constant("player_decelerate_bonus", 0.5);
          }
        }
        velocity.move_towards(target, acceleration * bonus * UPDATE_DURATION);
        velocity.limit_magnitude(max_speed);
        self.player.position += *velocity * UPDATE_DURATION;
      }
      PlayerActionState::Interacting(interaction_state) => {
        if intent != Intent::Interact(interaction_state.activating_intent) {
          interaction_state.canceled = true;
        }
        let mut interaction_state = interaction_state.clone();
        match interaction_state
          .action
          .update(ActionUpdateContext { game: self })
        {
          ActionStatus::StillGoing => {
            self.player.action_state = PlayerActionState::Interacting(interaction_state);
          }
          ActionStatus::Completed => {
            self.player.action_state = PlayerActionState::Moving {
              velocity: FloatingVector::zeros(),
            }
          }
        }
      }
    }

    self.map.update();

    self.time += UPDATE_DURATION;
  }
  pub fn update_until(&mut self, new_time: Time, intent: Intent) {
    while self.time < new_time {
      self.update(intent);
    }
  }

  pub fn draw(&mut self, draw: &mut impl Draw) {
    self.map.draw(draw);

    match &self.player.action_state {
      PlayerActionState::Moving { velocity } => {}
      PlayerActionState::Interacting(interaction_state) => {
        interaction_state.action.draw(self, draw);
      }
    }

    draw.rectangle_on_map(
      50,
      self.player.position,
      TILE_SIZE.to_floating() * 0.4,
      "#fff",
    );
    let a = self.player.position + FloatingVector::new(TILE_RADIUS as f64 * 0.5, 0.0);
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
        TILE_WIDTH as f64 * self.player.health as f64 / self.player.maximum_health as f64,
      ),
      "#f00",
    );
  }
}
