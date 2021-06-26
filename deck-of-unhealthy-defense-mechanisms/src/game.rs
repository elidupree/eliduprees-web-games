use crate::actions::{Action, ActionStatus, ActionUpdateContext};
use crate::cards::{CardInstance, Cards};
use crate::geometry::{
  FloatingVector, FloatingVectorExtension, Grid, GridBounds, GridVector, GridVectorExtension,
  EPSILON, TILE_RADIUS, TILE_SIZE, TILE_WIDTH,
};
use crate::mechanisms::{
  Deck, Mechanism, MechanismImmutableContext, MechanismType, MechanismUpdateContext,
};
use crate::movers::{
  Monster, Mover, MoverBehavior, MoverId, MoverImmutableContext, MoverType, MoverUpdateContext,
};
use crate::ui_glue::Draw;
use derivative::Derivative;
use eliduprees_web_games_lib::auto_constant;
use live_prop_test::{live_prop_test, lpt_assert_eq};
use nalgebra::Vector2;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::mem;

pub type Time = f64;
/// duration of each update in seconds:
pub const UPDATE_DURATION: Time = 1.0 / 180.0;

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Game {
  pub player: Player,
  pub cards: Cards,
  pub ui_time: Time,
  pub physics_time: Time,
  pub day: i32,
  pub day_progress: f64,
  pub horizon: f64,
  pub grid: Grid<Tile>,
  movers: HashMap<MoverId, MoverAndScheduleStuff>,
  next_mover_id: usize,
  upcoming_events: BTreeSet<UpcomingEvent>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Player {
  pub position: FloatingVector,
  pub action_state: PlayerActionState,
  pub initiated_interaction: Option<WhichInteraction>,
  pub maximum_health: i32,
  pub health: f64,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct PlayerActiveInteraction {
  pub which: WhichInteraction,
  pub action: Action,
  pub canceled: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum PlayerActionState {
  Moving { velocity: FloatingVector },
  Interacting(PlayerActiveInteraction),
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub enum WhichInteraction {
  PlayCard,
  ActivateMechanism,
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum OngoingIntent {
  Move(FloatingVector),
  Interact(WhichInteraction),
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Tile {
  mechanism: Option<Mechanism>,
  mechanism_schedule: Option<Time>,
  movers: Vec<MoverId>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
struct MoverAndScheduleStuff {
  mover: Mover,
  stored_bounds: GridBounds,
  schedule: Option<MoverSchedule>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
struct MoverSchedule {
  time: OrderedFloat<Time>,
  event_type: MoverEventType,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
enum MoverEventType {
  Wake,
  EscapeTiles,
  CollideWith(MoverId),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
struct UpcomingEvent {
  time: OrderedFloat<Time>,
  event_type: UpcomingEventType,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Derivative)]
#[derivative(
  PartialOrd = "feature_allow_slow_enum",
  Ord = "feature_allow_slow_enum"
)]
enum UpcomingEventType {
  Mover(MoverId),
  Mechanism(
    #[derivative(
      PartialOrd(compare_with = "crate::geometry::vector_partial_cmp"),
      Ord(compare_with = "crate::geometry::vector_cmp")
    )]
    GridVector,
  ),
}

impl Game {
  pub fn check_invariants(&self) -> Result<(), String> {
    // for (id, mover) in &self.movers {
    //   guard!(let Some(tile) = self.tiles.get(mover.position.containing_tile()) else {return Err("mover is outside of grid".to_string())});
    //   if !tile.movers.contains(&id) {
    //     return Err(format!(
    //       "Mover does not have a record in its tile: {:?}",
    //       mover
    //     ));
    //   }
    // }
    // for (position, tile) in &self.grid {
    //   for &id in &tile.movers {
    //     guard!(let Some(mover) = self.movers.get(&id) else {return Err("tile retained reference to missing mover".to_string())});
    //     lpt_assert_eq!(mover.position.containing_tile(), position);
    //   }
    // }
    Ok(())
  }

  pub fn mover(&self, id: MoverId) -> Option<&Mover> {
    self.movers.get(&id).map(|m| &m.mover)
  }
  pub fn mover_ids(&self) -> impl Iterator<Item = MoverId> + '_ {
    self.movers.keys().copied()
  }
  pub fn movers(&self) -> impl Iterator<Item = (MoverId, &Mover)> + '_ {
    self.movers.iter().map(|(&id, m)| (id, &m.mover))
  }

  pub fn mechanism(&self, position: GridVector) -> Option<&Mechanism> {
    self.grid.get(position).and_then(|m| m.mechanism.as_ref())
  }

  pub fn movers_near(
    &self,
    position: FloatingVector,
    range: f64,
  ) -> impl Iterator<Item = (MoverId, &Mover)> + '_ {
    let range_squared = range * range;
    self
      .grid
      .tiles_near(position, range)
      .flat_map(move |(_pos, tile)| {
        tile.movers.iter().filter_map(move |&mover_id| {
          let mover = &self.movers.get(&mover_id).unwrap().mover;
          if (mover.position(self.physics_time) - position).magnitude_squared() <= range_squared {
            Some((mover_id, mover))
          } else {
            None
          }
        })
      })
  }

  pub fn create_mechanism(&mut self, position: GridVector, mechanism: Mechanism) {
    let id = MoverId(self.next_mover_id);
    self.grid.get_mut(position).unwrap().mechanism = Some(mechanism);
    self.update_mechanism_schedule(position);
  }
  pub fn mutate_mechanism<R, F: FnOnce(&mut Mechanism) -> R>(
    &mut self,
    position: GridVector,
    f: F,
  ) -> Option<R> {
    if let Some(mechanism) = self
      .grid
      .get_mut(position)
      .and_then(|tile| tile.mechanism.as_mut())
    {
      let result = f(mechanism);
      self.update_mechanism_schedule(position);
      Some(result)
    } else {
      None
    }
  }
  pub fn create_mover(&mut self, mover: Mover) -> MoverId {
    let id = MoverId(self.next_mover_id);
    self.movers.insert(
      id,
      MoverAndScheduleStuff {
        mover,
        stored_bounds: Default::default(),
        schedule: None,
      },
    );
    self.next_mover_id += 1;
    self.store_mover_bounds(id);
    self.update_mover_schedule(id);
    id
  }
  pub fn remove_mover(&mut self, id: MoverId) {
    self.forget_mover_bounds(id);
    let mover = self.movers.remove(&id).unwrap();
    if let Some(old_schedule) = mover.schedule {
      self.upcoming_events.remove(&UpcomingEvent {
        time: old_schedule.time,
        event_type: UpcomingEventType::Mover(id),
      });
    }
  }
  pub fn mutate_mover<R, F: FnOnce(&mut Mover) -> R>(&mut self, id: MoverId, f: F) -> Option<R> {
    if let Some(stuff) = self.movers.get_mut(&id) {
      stuff.mover.rebase(self.physics_time);
      let result = f(&mut stuff.mover);
      self.update_mover_bounds_and_schedule(id);
      Some(result)
    } else {
      None
    }
  }

  fn mechanism_schedule(&self, position: GridVector) -> Option<Time> {
    let mechanism = self.grid.get(position).unwrap().mechanism.as_ref().unwrap();
    mechanism.mechanism_type.next_wake(mechanism)
  }

  fn mover_schedule(&self, id: MoverId) -> Option<MoverSchedule> {
    let stuff = self.movers.get(&id).unwrap();
    let wake = stuff
      .mover
      .behavior
      .next_wake(&stuff.mover)
      .map(|time| MoverSchedule {
        time: OrderedFloat(time),
        event_type: MoverEventType::Wake,
      });
    let min = stuff.stored_bounds.min_tile_corner().to_floating();
    let max = stuff.stored_bounds.max_tile_corner().to_floating();
    let escapes = (0..2).flat_map(|dimension| {
      match stuff.mover.velocity[dimension].partial_cmp(&0.0).unwrap() {
        Ordering::Less => Some(MoverSchedule {
          time: OrderedFloat(
            stuff.mover.trajectory_base_time
              + ((min[dimension] - EPSILON)
                - (stuff.mover.position_at_base_time[dimension] - stuff.mover.radius))
                / stuff.mover.velocity[dimension],
          ),
          event_type: MoverEventType::EscapeTiles,
        }),
        Ordering::Greater => Some(MoverSchedule {
          time: OrderedFloat(
            stuff.mover.trajectory_base_time
              + ((max[dimension] + EPSILON)
                - (stuff.mover.position_at_base_time[dimension] + stuff.mover.radius))
                / stuff.mover.velocity[dimension],
          ),
          event_type: MoverEventType::EscapeTiles,
        }),
        Ordering::Equal => None,
      }
    });
    wake.into_iter().chain(escapes).min()
  }

  fn forget_mover_bounds(&mut self, id: MoverId) {
    let mover = self.movers.get(&id).unwrap();
    for position in mover.stored_bounds.tile_centers() {
      if let Some(tile) = self.grid.get_mut(position) {
        tile.movers.retain(|&id2| id2 != id);
      }
    }
  }

  fn store_mover_bounds(&mut self, id: MoverId) {
    let stuff = self.movers.get_mut(&id).unwrap();
    stuff.stored_bounds = stuff.mover.grid_bounds(self.physics_time);
    for position in stuff.stored_bounds.tile_centers() {
      if let Some(tile) = self.grid.get_mut(position) {
        tile.movers.push(id);
      }
    }
  }

  fn update_mover_bounds_and_schedule(&mut self, id: MoverId) {
    self.forget_mover_bounds(id);
    self.store_mover_bounds(id);
    self.update_mover_schedule(id);
  }

  fn update_mover_schedule(&mut self, id: MoverId) {
    let new_schedule = self.mover_schedule(id);
    let stuff = self.movers.get_mut(&id).unwrap();
    if stuff.schedule != new_schedule {
      if let Some(new_schedule) = &new_schedule {
        self.upcoming_events.insert(UpcomingEvent {
          time: new_schedule.time,
          event_type: UpcomingEventType::Mover(id),
        });
      }
      if let Some(old_schedule) = mem::replace(&mut stuff.schedule, new_schedule) {
        self.upcoming_events.remove(&UpcomingEvent {
          time: old_schedule.time,
          event_type: UpcomingEventType::Mover(id),
        });
      }
    }
  }

  fn update_mechanism_schedule(&mut self, position: GridVector) {
    let new_schedule = self.mechanism_schedule(position);
    let tile = self.grid.get_mut(position).unwrap();
    if tile.mechanism_schedule != new_schedule {
      if let Some(new_schedule) = new_schedule {
        self.upcoming_events.insert(UpcomingEvent {
          time: OrderedFloat(new_schedule),
          event_type: UpcomingEventType::Mechanism(position),
        });
      }
      if let Some(old_schedule) = mem::replace(&mut tile.mechanism_schedule, new_schedule) {
        self.upcoming_events.remove(&UpcomingEvent {
          time: OrderedFloat(old_schedule),
          event_type: UpcomingEventType::Mechanism(position),
        });
      }
    }
  }

  fn do_next_event(&mut self) {
    let event = self.upcoming_events.pop_first().unwrap();
    self.set_physics_time(event.time.0);
    match event.event_type {
      UpcomingEventType::Mover(id) => {
        let stuff = self.movers.get(&id).unwrap();
        let schedule = stuff.schedule.clone().unwrap();
        assert_eq!(event.time, schedule.time);
        match schedule.event_type {
          MoverEventType::Wake => {
            stuff
              .mover
              .behavior
              .clone()
              .wake(MoverUpdateContext { id, game: self });
          }
          MoverEventType::EscapeTiles => {
            self.update_mover_bounds_and_schedule(id);
          }
          MoverEventType::CollideWith(other_id) => {}
        }
      }
      UpcomingEventType::Mechanism(position) => {
        self
          .grid
          .get(position)
          .unwrap()
          .mechanism
          .as_ref()
          .unwrap()
          .mechanism_type
          .clone()
          .wake(MechanismUpdateContext {
            position,
            game: self,
          });
      }
    }
  }

  pub fn new() -> Self {
    let radius = 10;
    let grid = Grid::new(
      GridVector::new(-(radius as i32) * TILE_WIDTH, -(radius as i32) * TILE_WIDTH),
      Vector2::new(radius * 2 + 1, radius * 2 + 1),
    );
    let mut result = Game {
      player: Player {
        position: FloatingVector::zeros(),
        action_state: PlayerActionState::Moving {
          velocity: FloatingVector::zeros(),
        },
        initiated_interaction: None,
        maximum_health: 100,
        health: 100.0,
      },
      cards: Cards {
        deck: vec![
          CardInstance::basic_conveyor(),
          CardInstance::basic_conveyor(),
          CardInstance::basic_conveyor(),
          CardInstance::basic_tower(),
          CardInstance::basic_conveyor(),
          CardInstance::basic_conveyor(),
          CardInstance::basic_tower(),
          CardInstance::basic_conveyor(),
        ],
        selected_index: Some(0),
      },
      ui_time: 0.0,
      physics_time: 0.0,
      day: 1,
      day_progress: 0.0,
      horizon: 50.0,
      grid,
      movers: Default::default(),
      next_mover_id: 0,
      upcoming_events: Default::default(),
    };
    result.create_mechanism(
      GridVector::zeros(),
      Mechanism {
        mechanism_type: MechanismType::Deck(Deck {}),
        ..Default::default()
      },
    );
    result.create_mover(Mover {
      trajectory_base_time: 0.0,
      position_at_base_time: FloatingVector::new(4.0, 6.0),
      mover_type: MoverType::Monster,
      behavior: MoverBehavior::Monster(Monster {
        home: FloatingVector::new(8.0, 12.0),
        active_time: 0.5..0.9,
        next_wake: 0.0,
      }),

      ..Default::default()
    });
    result
  }

  pub fn current_mechanism_activation(&self) -> Option<Action> {
    let position = self.player.position.containing_tile();
    if let Some(tile) = self.grid.get(position) {
      if let Some(mechanism) = &tile.mechanism {
        return mechanism
          .mechanism_type
          .activation(MechanismImmutableContext {
            position,
            game: self,
          });
      }
    }
    None
  }

  pub fn initiate_interaction(&mut self, which: WhichInteraction) {
    self.player.initiated_interaction = Some(which);
  }

  fn update(&mut self, intent: OngoingIntent) {
    //if intent != OngoingIntent::Move(FloatingVector::zeros()) {
    self.update_physics(intent);

    self.ui_time += UPDATE_DURATION;
  }

  fn set_physics_time(&mut self, time: Time) {
    let change = time - self.physics_time;
    self.physics_time = time;
    let day_length = auto_constant("day_length", 60.0);
    self.day_progress += change / day_length;
    if self.day_progress >= 1.0 {
      self.day += 1;
      self.day_progress = 0.0;
    }
  }

  fn update_physics(&mut self, intent: OngoingIntent) {
    let former = self.clone();

    // cancel initiating interaction if you stopped holding it before it went off
    if matches!(self.player.initiated_interaction, Some(which) if intent != OngoingIntent::Interact(which))
    {
      self.player.initiated_interaction = None;
    }

    self.player.health += auto_constant("health_regeneration", 3.0) * UPDATE_DURATION;
    self.player.health = self.player.health.min(self.player.maximum_health as f64);

    match &mut self.player.action_state {
      PlayerActionState::Moving { velocity } => {
        let acceleration = auto_constant("player_acceleration", 4.0) * TILE_WIDTH as f64;
        let max_speed = auto_constant("player_max_speed", 1.4) * TILE_WIDTH as f64;
        let mut target;
        if let OngoingIntent::Move(mut movement_intent) = intent {
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
        if intent != OngoingIntent::Interact(interaction_state.which) {
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

    if matches!(self.player.action_state, PlayerActionState::Moving { velocity } if velocity == FloatingVector::zeros())
    {
      if let Some(which) = self.player.initiated_interaction.take() {
        let action = match which {
          WhichInteraction::ActivateMechanism => self.current_mechanism_activation(),
          WhichInteraction::PlayCard => self.cards.selected().map(|card| card.action.clone()),
        };

        if let Some(action) = action {
          if action.possible(self) {
            self.player.action_state = PlayerActionState::Interacting(PlayerActiveInteraction {
              which,
              action,
              canceled: false,
            });
          }
        }
      }
    }

    let end_time = self.physics_time + UPDATE_DURATION;

    while matches!(self.upcoming_events.first(), Some(event) if event.time.0 < end_time) {
      self.do_next_event();
    }
    self.set_physics_time(end_time);

    let closest_monster_distance = self
      .movers
      .iter()
      .map(|(_id, mover)| &mover.mover)
      .filter(|m| m.mover_type == MoverType::Monster)
      .map(|m| OrderedFloat(m.position(self.physics_time).magnitude()))
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

    self.physics_time = end_time;
  }
  pub fn update_until(&mut self, new_time: Time, intent: OngoingIntent) {
    while self.ui_time < new_time {
      self.update(intent);
    }
  }

  pub fn draw(&self, draw: &mut impl Draw) {
    for (tile_position, tile) in &self.grid {
      if let Some(mechanism) = &tile.mechanism {
        mechanism.mechanism_type.draw(
          MechanismImmutableContext {
            position: tile_position,
            game: self,
          },
          draw,
        );
      }
      for &mover_id in &tile.movers {
        self.movers.get(&mover_id).unwrap().mover.behavior.draw(
          MoverImmutableContext {
            id: mover_id,
            game: self,
          },
          draw,
        );
      }
    }

    draw.rectangle_on_map(
      0,
      FloatingVector::zeros(),
      FloatingVector::new(self.horizon * 2.0, self.horizon * 2.0),
      "#444",
    );

    match &self.player.action_state {
      PlayerActionState::Moving { velocity: _ } => {}
      PlayerActionState::Interacting(interaction_state) => {
        interaction_state.action.draw_progress(self, draw);
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

    self.cards.draw(self, draw);
  }
}
