use crate::game::{Game, Time};
use crate::geometry::{
  Facing, FloatingVector, FloatingVectorExtension, GridVectorExtension, Rotation, TILE_RADIUS,
  TILE_SIZE, TILE_WIDTH,
};
use crate::geometry::{GridBounds, EPSILON};
use crate::mechanisms::{Conveyor, ConveyorSide, MechanismType};
use crate::ui_glue::Draw;
use crate::utils::Assume;
use derivative::Derivative;
use eliduprees_web_games_lib::auto_constant;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Range};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
#[derivative(Default)]
pub enum MoverType {
  #[derivative(Default)]
  Monster,
  Projectile,
  Material,
}
#[derive(
  Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug, Default,
)]
pub struct MoverId(pub usize);

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Mover {
  pub trajectory_base_time: Time,
  pub position_at_base_time: FloatingVector,
  pub velocity: FloatingVector,
  pub radius: f64,
  pub behavior: MoverBehavior,

  pub mover_type: MoverType,
}

impl Mover {
  pub fn position(&self, time: Time) -> FloatingVector {
    self.position_at_base_time + self.velocity * (time - self.trajectory_base_time)
  }

  pub fn bounds(&self, time: Time) -> [FloatingVector; 2] {
    let position = self.position(time);
    let half_size = FloatingVector::new(self.radius, self.radius);
    [position - half_size, position + half_size]
  }

  pub fn grid_bounds(&self, time: Time) -> GridBounds {
    GridBounds::containing(self.bounds(time))
  }

  pub fn rebase(&mut self, time: Time) {
    self.position_at_base_time += self.velocity * (time - self.trajectory_base_time);
    self.trajectory_base_time = time;
  }
}

#[derive(Debug)]
pub struct MoverView<'a> {
  mover: &'a mut Mover,
  now: Time,
}

#[derive(Debug)]
pub struct TypedMoverView<'a, T> {
  mover: MoverView<'a>,
  _marker: PhantomData<T>,
}

impl<'a, T> TypedMoverView<'a, T> {
  fn behavior<'b>(&'b self) -> &'b T
  where
    &'b T: TryFrom<&'b MoverBehavior>,
  {
    match (&self.mover.behavior).try_into() {
      Ok(b) => b,
      Err(_) => panic!("A TypedMoverView existed for the wrong type"),
    }
  }
  fn behavior_mut<'b>(&'b mut self) -> &'b mut T
  where
    &'b mut T: TryFrom<&'b mut MoverBehavior>,
  {
    match (&mut self.mover.behavior).try_into() {
      Ok(b) => b,
      Err(_) => panic!("A TypedMoverView existed for the wrong type"),
    }
  }
}

impl<'a> MoverView<'a> {
  fn position(&self) -> FloatingVector {
    self.mover.position(self.now)
  }
}

impl<'a> Deref for MoverView<'a> {
  type Target = Mover;

  fn deref(&self) -> &Mover {
    self.mover
  }
}

impl<'a> DerefMut for MoverView<'a> {
  fn deref_mut(&mut self) -> &mut Mover {
    self.mover
  }
}

impl<'a, T> Deref for TypedMoverView<'a, T> {
  type Target = MoverView<'a>;

  fn deref(&self) -> &MoverView<'a> {
    &self.mover
  }
}

impl<'a, T> DerefMut for TypedMoverView<'a, T> {
  fn deref_mut(&mut self) -> &mut MoverView<'a> {
    &mut self.mover
  }
}

#[derive(Debug)]
pub struct MoverUpdateContext<'a> {
  pub id: MoverId,
  pub game: &'a mut Game,
}

#[derive(Debug)]
pub struct MoverImmutableContext<'a> {
  pub id: MoverId,
  pub game: &'a Game,
}

#[derive(Debug)]
pub struct MoverCollideContext<'a> {
  pub update_context: MoverUpdateContext<'a>,
  pub other_id: MoverId,
}

impl<'a> MoverUpdateContext<'a> {
  pub fn this(&self) -> &Mover {
    self.game.mover(self.id).unwrap()
  }
  pub fn mutate_this<R, F: FnOnce(MoverView) -> R>(&mut self, f: F) -> R {
    let now = self.game.physics_time;
    self
      .game
      .mutate_mover(self.id, |mover| f(MoverView { mover, now }))
      .unwrap()
  }
  // take self by value unnecessarily, to protect from accidentally doing stuff after destroyed
  pub fn destroy_this(self) {
    self.game.remove_mover(self.id);
  }
}

impl<'a> MoverImmutableContext<'a> {
  pub fn this(&self) -> &Mover {
    self.game.mover(self.id).unwrap()
  }
}

impl<'a> Deref for MoverCollideContext<'a> {
  type Target = MoverUpdateContext<'a>;

  fn deref(&self) -> &MoverUpdateContext<'a> {
    &self.update_context
  }
}

impl<'a> DerefMut for MoverCollideContext<'a> {
  fn deref_mut(&mut self) -> &mut MoverUpdateContext<'a> {
    &mut self.update_context
  }
}

impl<'a> MoverCollideContext<'a> {
  pub fn other(&self) -> &Mover {
    self.game.mover(self.other_id).unwrap()
  }
  pub fn mutate_other<R, F: FnOnce(&mut Mover) -> R>(&mut self, f: F) -> R {
    self
      .update_context
      .game
      .mutate_mover(self.other_id, f)
      .unwrap()
  }
  // take self by value unnecessarily, to protect from accidentally doing stuff after destroyed
  pub fn destroy_this(self) {
    self.update_context.destroy_this();
  }
}

#[allow(unused)]
pub trait MoverBehaviorTrait {
  /** Perform a single scheduled update on this mover, possibly modifying the game state.

  Note that when called, `self` is a *copy* of the actual MoverBehavior implementor;
  use `context.mutate_this()` to change this Mover.
  MoverBehavior implementors are currently expected to have no data; they use the shared data that is in Mover.
  */
  fn wake(&self, context: MoverUpdateContext) {}
  fn next_wake(&self, this: &Mover) -> Option<Time> {
    None
  }

  fn collide(&self, context: MoverCollideContext) {}
  fn escape_bounds(&self, context: MoverUpdateContext) {}

  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw);
}

trait_enum! {
  #[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
  #[derivative(Default)]
  pub enum MoverBehavior: MoverBehaviorTrait {
    Monster, Projectile, #[derivative(Default)] Material,
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Monster {
  pub home: FloatingVector,
  pub active_time: Range<f64>,
  pub next_wake: Time,
}

const MONSTER_WAKE_DELAY: Time = 0.1;

impl MoverBehaviorTrait for Monster {
  fn wake(&self, mut context: MoverUpdateContext) {
    let active = self.active_time.contains(&context.game.day_progress);
    let target;
    if active {
      target = FloatingVector::zeros();
    } else {
      target = self.home;
    }

    let relative_target = target - context.this().position(context.game.physics_time);

    let acceleration = auto_constant("monster_acceleration", 4.0) * TILE_WIDTH as f64;
    let max_speed = auto_constant("monster_max_speed", 1.6) * TILE_WIDTH as f64;
    // Account for stopping distance:
    // we might overshoot by an average speed of acceleration*MONSTER_WAKE_DELAY/2 due to not updating more frequently,
    // so saturating-subtract that much
    let target_speed = max_speed
      .min(
        (2.0 * relative_target.magnitude() * acceleration).sqrt()
          - acceleration * MONSTER_WAKE_DELAY * 0.5,
      )
      .max(0.0);
    //debug!("{:?}", relative_target);

    let now = context.game.physics_time;
    context.mutate_this(|mut this: MoverView| {
      let target_velocity = relative_target
        .try_normalize(EPSILON)
        .map_or(FloatingVector::zeros(), |target_direction| {
          target_direction * target_speed
        });
      this
        .velocity
        .move_towards(target_velocity, acceleration * MONSTER_WAKE_DELAY);

      this.behavior.assume::<Self>().next_wake = now + MONSTER_WAKE_DELAY;
    });
  }

  fn next_wake(&self, _this: &Mover) -> Option<Time> {
    Some(self.next_wake)
  }

  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      10,
      context.this().position(context.game.physics_time),
      TILE_SIZE.to_floating() * 0.8,
      "#000",
    );
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Projectile {
  pub disappear_time: Time,
}

impl MoverBehaviorTrait for Projectile {
  fn wake(&self, context: MoverUpdateContext) {
    context.destroy_this();
  }

  fn next_wake(&self, _this: &Mover) -> Option<Time> {
    Some(self.disappear_time)
  }

  fn collide(&self, mut context: MoverCollideContext) {
    if context.other().mover_type == MoverType::Monster {
      let impact = auto_constant("projectile_impact", 2.0) * TILE_WIDTH as f64;

      // Push the monster directly away from your deck. We COULD have the monster be propelled in the direction of the projectile (context.this().velocity) instead, but that causes the monster to be knocked around in a way that feels slippery, rather than feeling like a struggle of determination against determination. Having projectile directions matter would make this a physics game, and I actually DON'T want this to be a physics game.
      let now = context.game.physics_time;
      context.mutate_other(|target| {
        let direction = target.position(now);
        target.velocity += direction.normalize() * impact;
      });
      context.destroy_this();
    }
  }

  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      10,
      context.this().position(context.game.physics_time),
      TILE_SIZE.to_floating() * 0.2,
      "#fff",
    );
  }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Material {
  pub perpendicular_position: f64,
}

impl MoverBehaviorTrait for Material {
  fn draw(&self, context: MoverImmutableContext, draw: &mut dyn Draw) {
    draw.rectangle_on_map(
      20,
      context.this().position(context.game.physics_time),
      TILE_SIZE.to_floating() * 0.25,
      "#fff",
    );
  }

  fn escape_bounds(&self, mut context: MoverUpdateContext) {
    let position = context.this().position(context.game.physics_time);
    let tile_position = position.containing_tile();
    let mechanism = context.game.mechanism(tile_position);
    if let Some(mechanism) = mechanism {
      if let MechanismType::Conveyor(conveyor) = &mechanism.mechanism_type {
        let mut facings = Facing::ALL_FACINGS;
        facings.rotate_left(conveyor.last_sent.as_index() + 1);
        for &facing in &facings {
          let target_tile_position = tile_position + facing.unit_vector() * TILE_WIDTH;
          if !(conveyor.sides[facing.as_index()] == ConveyorSide::Output
            || context
              .game
              .mechanism(target_tile_position)
              .map_or(false, |mechanism| {
                mechanism.mechanism_type.wants_to_steal(self)
              }))
          {
            continue;
          }
          let target = tile_position.to_floating()
            + facing.unit_vector().to_floating() * (TILE_RADIUS as f64 * 1.01)
            + (facing + Rotation::CLOCKWISE).unit_vector().to_floating()
              * self.perpendicular_position;
          context.mutate_this(|mut this| {
            this.velocity = (target - position) * auto_constant("conveyor_speed", 0.8)
          });
          context.game.mutate_mechanism(tile_position, |m| {
            m.mechanism_type.assume::<Conveyor>().last_sent = facing
          });
          break;
        }
      }
    } else {
      context.destroy_this()
    }
  }
}
