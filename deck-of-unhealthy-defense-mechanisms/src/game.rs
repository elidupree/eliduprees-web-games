use crate::actions::{Action, ActionStatus, ActionUpdateContext, Cost};
use crate::cards::{CardInstance, Cards, HandCard};
use crate::map::{
  FloatingVector, FloatingVectorExtension, GridVector, GridVectorExtension, Map, Tile, TILE_RADIUS,
  TILE_SIZE, TILE_WIDTH,
};
use crate::mechanisms::{Deck, Mechanism, MechanismImmutableContext, MechanismType};
use crate::movers::{Monster, Mover, MoverBehavior, MoverType};
use crate::ui_glue::Draw;
use eliduprees_web_games_lib::auto_constant;
use ordered_float::OrderedFloat;
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
  pub day: i32,
  pub day_progress: f64,
  pub horizon: f64,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Player {
  pub position: FloatingVector,
  pub action_state: PlayerActionState,
  pub already_begun_interaction_intent: Option<InteractionIntent>,
  pub maximum_health: i32,
  pub health: f64,
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
    let tile = tiles
      .entry(GridVector::zeros())
      .or_insert_with(Default::default);
    tile.mechanism = Some(Mechanism {
      mechanism_type: MechanismType::Deck(Deck {}),
      ..Default::default()
    });
    tile.movers.push(Mover {
      position: FloatingVector::new(4.0, 6.0),
      mover_type: MoverType::Monster,
      behavior: MoverBehavior::Monster(Monster),
      home: FloatingVector::new(8.0, 12.0),
      active_time: 0.2..0.9,
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
        health: 100.0,
      },
      cards: Cards {
        draw_pile: vec![
          CardInstance::basic_conveyor(),
          CardInstance::basic_conveyor(),
          CardInstance::basic_conveyor(),
        ],
        discard_pile: vec![],
        hand: vec![
          HandCard {
            card: CardInstance::basic_conveyor(),
          },
          HandCard {
            card: CardInstance::basic_tower(),
          },
          HandCard {
            card: CardInstance::basic_conveyor(),
          },
          HandCard {
            card: CardInstance::basic_conveyor(),
          },
          HandCard {
            card: CardInstance::basic_tower(),
          },
        ],
      },
      time: 0.0,
      day: 1,
      day_progress: 0.0,
      horizon: 50.0,
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
            game: self,
          });
      }
    }
    [None, None]
  }

  fn update(&mut self, intent: Intent) {
    let former = self.clone();
    match intent {
      Intent::Move(_movement_intent) => {}
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

    self.player.health += auto_constant("health_regeneration", 3.0) * UPDATE_DURATION;
    self.player.health = self.player.health.min(self.player.maximum_health as f64);

    match &mut self.player.action_state {
      PlayerActionState::Moving { velocity } => {
        let acceleration = auto_constant("player_acceleration", 4.0) * TILE_WIDTH as f64;
        let max_speed = auto_constant("player_max_speed", 1.4) * TILE_WIDTH as f64;
        let mut target;
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
        let horizon_violation = self.player.position.magnitude() / self.horizon;
        if horizon_violation > 1.0 {
          let squash_direction = -self.player.position.normalize();
          target -=
            squash_direction * target.dot(&squash_direction) * (1.0 - 1.0 / horizon_violation);
          target += squash_direction * (horizon_violation - 1.0) * max_speed * 2.0;
        }
        velocity.move_towards(target, acceleration * bonus * UPDATE_DURATION);
        //velocity.limit_magnitude(max_speed);
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

    self.map.update(&former);

    let closest_monster_distance = self
      .map
      .tiles
      .iter()
      .flat_map(|(_, t)| &t.movers)
      .map(|m| OrderedFloat(m.position.magnitude()))
      .min()
      .unwrap_or(OrderedFloat(100000.0))
      .0;
    let horizon_leeway_width = auto_constant("horizon_leeway_width", 1.0) * TILE_WIDTH as f64;
    let decay_size_needed = (self.horizon - closest_monster_distance) - horizon_leeway_width;
    if decay_size_needed > 0.0 {
      self.horizon -= decay_size_needed
        * (1.0 - auto_constant("horizon_contract_decay", 0.5).powf(UPDATE_DURATION));
    }
    let expand_size_needed = closest_monster_distance - self.horizon;
    if expand_size_needed > 0.0 {
      self.horizon += expand_size_needed
        * (1.0 - auto_constant("horizon_expand_decay", 0.8).powf(UPDATE_DURATION));
    }

    self.time += UPDATE_DURATION;
    let day_length = auto_constant("day_length", 60.0);
    self.day_progress += UPDATE_DURATION / day_length;
    if self.day_progress >= 1.0 {
      self.day += 1;
      self.day_progress = 0.0;
    }
  }
  pub fn update_until(&mut self, new_time: Time, intent: Intent) {
    while self.time < new_time {
      self.update(intent);
    }
  }

  pub fn draw(&self, draw: &mut impl Draw) {
    self.map.draw(self, draw);

    draw.rectangle_on_map(
      0,
      FloatingVector::zeros(),
      FloatingVector::new(self.horizon * 2.0, self.horizon * 2.0),
      "#444",
    );

    match &self.player.action_state {
      PlayerActionState::Moving { velocity: _ } => {}
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
        TILE_WIDTH as f64 * self.player.health / self.player.maximum_health as f64,
      ),
      "#f00",
    );

    let [left, right] = self.interactions();
    let actions: Vec<_> = std::iter::once(left.as_ref())
      .chain((0..5).map(|index| self.cards.hand.get(index).map(|card| &card.card.action)))
      .chain(std::iter::once(right.as_ref()))
      .collect();
    for (index, &action) in actions.iter().enumerate() {
      if let Some(action) = action {
        let info = action.display_info();
        let horizontal = (index as f64 + 0.1) / actions.len() as f64;
        draw.text(FloatingVector::new(horizontal, 0.8), &info.name);
        if let Cost::Fixed(cost) = info.time_cost {
          draw.text(
            FloatingVector::new(horizontal, 0.85),
            &format!("{} time", cost),
          );
        }
        if let Cost::Fixed(cost) = info.health_cost {
          draw.text(
            FloatingVector::new(horizontal, 0.9),
            &format!("{} health", cost),
          );
        }
      }
    }
  }
}
