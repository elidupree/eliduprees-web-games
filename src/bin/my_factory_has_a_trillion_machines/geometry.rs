use std::ops::Neg;
use std::ops::{Mul, Div};
use nalgebra::Vector2;

pub type Number = i64;


pub type Vector = Vector2 <Number>;
pub type Facing = u8;

pub trait VectorExtension {
  fn to_f64(&self)->Vector2 <f64>;
}
impl VectorExtension for Vector {
  fn to_f64(&self)->Vector2 <f64> {Vector2::new (self [0] as f64, self [1] as f64)}
}

#[derive (Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub struct GridIsomorphism {
  #[derivative (Default (value = "Vector::new(0,0)"))]
  pub translation: Vector,
  pub rotation: Facing,
  pub flip: bool,
}

pub trait TransformedBy {
  fn transformed_by (self, isomorphism: GridIsomorphism)->Self;
}
impl TransformedBy for Vector { 
  fn transformed_by (mut self, isomorphism: GridIsomorphism)->Self {
    if isomorphism.flip { self[0] *= -1; }
    self = self.rotate_90 (isomorphism.rotation);
    self + isomorphism.translation
  }
}
impl TransformedBy for Vector2 <f64> { 
  fn transformed_by (mut self, isomorphism: GridIsomorphism)->Self {
    if isomorphism.flip { self[0] *= -1.0; }
    self = self.rotate_90 (isomorphism.rotation);
    self + isomorphism.translation.to_f64()
  }
}
impl TransformedBy for Facing { 
  fn transformed_by (mut self, isomorphism: GridIsomorphism)->Self {
    if isomorphism.flip { self = (4-self) % 4 }
    self = self.rotate_90 (isomorphism.rotation);
    self
  }
}
impl <T: TransformedBy, U: TransformedBy> TransformedBy for (T, U) {
  fn transformed_by (self, isomorphism: GridIsomorphism)->Self {
    (self.0.transformed_by(isomorphism), self.1.transformed_by(isomorphism))
  }
}
impl <T: TransformedBy> TransformedBy for Option<T> {
  fn transformed_by (self, isomorphism: GridIsomorphism)->Self {
    self.map(|t| t.transformed_by(isomorphism))
  }
}
impl Mul<GridIsomorphism> for GridIsomorphism {
  type Output = GridIsomorphism; 
  fn mul (mut self, other: GridIsomorphism)->GridIsomorphism {
    if other.flip {
      self.translation[0] *= -1;
      self.rotation = (4-self.rotation) % 4;
      self.flip = !self.flip;
    }
    self.translation = self.translation.rotate_90 (other.rotation);
    self.rotation = self.rotation.rotate_90 (other.rotation);
    self.translation += other.translation;
    self
  }
}
impl Div<GridIsomorphism> for GridIsomorphism {
  type Output = GridIsomorphism; 
  #[allow (clippy::suspicious_arithmetic_impl)]
  fn div (self, other: GridIsomorphism)->GridIsomorphism {  
    self * other.inverse()
  }
}
impl GridIsomorphism {
  pub fn inverse(mut self)->GridIsomorphism {
    self.translation = (-self.translation).rotate_90 ((4-self.rotation) % 4);
    if self.flip {
      self.translation[0] *= -1;
    }
    else {
      self.rotation = (4-self.rotation) % 4;
    }
    self
  }
}

#[cfg(test)]
mod tests {
use super::*;
use proptest::prelude::*;

#[test]
fn grid_isomorphism_unit_tests() {
  let isomorphism = GridIsomorphism {
    translation: Vector::new (5, 4),
    rotation: 1,
    flip: true
  };
  let vector = Vector::new (2, 1) ;
  
  assert_eq!(vector.transformed_by (isomorphism), Vector::new (4, 2));
  
}

fn arbitrary_vector() -> BoxedStrategy<Vector> {
  (
    -1000000i64..1000000i64,
    -1000000i64..1000000i64
  ).prop_map(|(x, y)| {
    Vector::new (x,y)
  }).boxed()
}
/*
prop_compose! {
  fn arbitrary_vector()(
    x in -1000000i64..1000000i64,
    y in -1000000i64..1000000i64
  )->Vector {
    Vector::new (x,y)
  }
}*/
prop_compose! {
  fn arbitrary_isomorphism()(
    translation in arbitrary_vector(),
    rotation in 0u8..4u8,
    flip in any::<bool>()
  )->GridIsomorphism {
    GridIsomorphism {
      translation, rotation, flip,
    }
  }
}

proptest! {
  
  #[test]
  fn randomly_test_grid_isomorphism_inverse_repetition (isomorphism in arbitrary_isomorphism()) {
    prop_assert_eq! (isomorphism, isomorphism.inverse().inverse());
  }
  #[test]
  fn randomly_test_grid_isomorphism_inverse_is_inverse (isomorphism in arbitrary_isomorphism()) {
    prop_assert_eq! (isomorphism * isomorphism.inverse(), GridIsomorphism::default());
    prop_assert_eq! (isomorphism.inverse() * isomorphism, GridIsomorphism::default());
  }
  #[test]
  fn randomly_test_grid_isomorphism_identity_is_identity (isomorphism in arbitrary_isomorphism()) {
    prop_assert_eq! (isomorphism * GridIsomorphism::default(), isomorphism);
    prop_assert_eq! (GridIsomorphism::default() * isomorphism, isomorphism);
  }
  #[test]
  fn randomly_test_grid_isomorphism_inverse_vector (isomorphism in arbitrary_isomorphism(), vector in arbitrary_vector()) {
    prop_assert_eq! (vector, vector.transformed_by(isomorphism).transformed_by(isomorphism.inverse()));
  }
  #[test]
  fn randomly_test_grid_isomorphism_inverse_facing (isomorphism in arbitrary_isomorphism(), facing in 0u8..4u8) {
    prop_assert_eq! (facing, facing.transformed_by(isomorphism).transformed_by(isomorphism.inverse()));
  }
  #[test]
  fn randomly_test_grid_isomorphism_is_flip_then_rotate_then_translate (isomorphism in arbitrary_isomorphism()) {
    prop_assert_eq! (isomorphism,
      GridIsomorphism {flip: isomorphism.flip, ..Default::default()}
      * GridIsomorphism {rotation: isomorphism.rotation, ..Default::default()}
      * GridIsomorphism {translation: isomorphism.translation, ..Default::default()}
    );
  }
  #[test]
  fn randomly_test_grid_isomorphism_associative (first in arbitrary_isomorphism(), second in arbitrary_isomorphism(), third in arbitrary_isomorphism()) {
    prop_assert_eq! ((first * second) * third, first * (second * third));
  }
  #[test]
  fn randomly_test_grid_isomorphism_transforms_translation_like_vector (isomorphism in arbitrary_isomorphism(), transformed_isomorphism in arbitrary_isomorphism()) {
    prop_assert_eq! ((transformed_isomorphism * isomorphism).translation, transformed_isomorphism.translation.transformed_by(isomorphism));
  }
}

}


pub trait Rotate90 {
  fn rotate_90 (self, facing: Facing)->Self;
}

impl <T: ::nalgebra::Scalar + Neg<Output=T>> Rotate90 for Vector2 <T> {
  fn rotate_90 (self, facing: Facing)->Self {
    match facing {
      0 => self,
      1 => Vector2::new (-self[1],  self[0]),
      2 => Vector2::new (-self[0], -self[1]),
      3 => Vector2::new ( self[1], -self[0]),
      _=> unreachable!()
    }
  }
}
impl Rotate90 for Facing {
  fn rotate_90 (self, facing: Facing)->Facing {
    (self + facing) % 4
  }
}
impl <T: Rotate90, U: Rotate90> Rotate90 for (T, U) {
  fn rotate_90 (self, facing: Facing)->Self {
    (self.0.rotate_90(facing), self.1.rotate_90(facing))
  }
}
impl <T: Rotate90> Rotate90 for Option<T> {
  fn rotate_90 (self, facing: Facing)->Self {
    self.map(|t| t.rotate_90(facing))
  }
}
