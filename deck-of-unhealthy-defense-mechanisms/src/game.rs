use crate::cards::Cards;
use crate::map::{FloatVectorExtension, FloatingVector, Map, TILE_WIDTH};
use eliduprees_web_games_lib::auto_constant;
use serde::{Deserialize, Serialize};

pub type Time = f64;
/// duration of each update in seconds:
const UPDATE_DURATION: f64 = 1.0 / 180.0;

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
  pub health: i32,
}
#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum PlayerActionState {
  Moving {
    velocity: FloatingVector,
  },
  Interacting {
    what: WhatInteraction,
    progress: Time,
  },
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub enum WhatInteraction {
  PlayCard(usize),
  InteractLeft,
  InteractRight,
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum Intent {
  Move(FloatingVector),
  Interact(WhatInteraction),
}

impl Game {
  pub fn new() -> Self {
    Game {
      map: Map {
        tiles: Default::default(),
      },
      player: Player {
        position: FloatingVector::zeros(),
        action_state: PlayerActionState::Moving {
          velocity: FloatingVector::zeros(),
        },
        health: 100,
      },
      cards: Cards {
        draw_pile: vec![],
        discard_pile: vec![],
        hand: vec![],
      },
      time: 0.0,
    }
  }
  fn update(&mut self, intent: Intent) {
    match intent {
      Intent::Move(movement_intent) => {
        if matches!(
          self.player.action_state,
          PlayerActionState::Interacting { .. }
        ) {
          self.player.action_state = PlayerActionState::Moving {
            velocity: FloatingVector::zeros(),
          };
        }
      }
      Intent::Interact(what) => {
        if matches!(self.player.action_state, PlayerActionState::Moving { velocity } if velocity == FloatingVector::zeros())
        {
          self.player.action_state = PlayerActionState::Interacting {
            what,
            progress: 0.0,
          };
        }
      }
    }

    match &mut self.player.action_state {
      PlayerActionState::Moving { velocity } => {
        let acceleration = auto_constant("player_acceleration", 4.0);
        match intent {
          Intent::Move(movement_intent) => {
            *velocity += acceleration * movement_intent;
          }
          Intent::Interact(_) => {
            velocity.apply_friction(acceleration * UPDATE_DURATION);
          }
        }
        velocity.limit_magnitude(auto_constant("player_max_speed", 1.4) * TILE_WIDTH);
        self.player.position += *velocity * UPDATE_DURATION;
      }
      PlayerActionState::Interacting { what, progress } => {
        *progress += UPDATE_DURATION;
        if *progress > 1.7 {}
      }
    }

    self.time += UPDATE_DURATION;
  }
  pub fn update_until(&mut self, new_time: Time, intent: Intent) {
    while self.time < new_time {
      self.update(intent);
    }
  }
}
