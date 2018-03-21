use super::*;

use nalgebra::Vector2;


pub fn move_to (location: Vector2 <f64>) {
  js! {context.moveTo (@{location [0]},@{location [1]});}
}
pub fn line_to (location: Vector2 <f64>) {
  js! {context.lineTo (@{location [0]},@{location [1]});}
}
pub fn translate (location: Vector2 <f64>) {
  js! {context.translate (@{location [0]},@{location [1]});}
}
pub fn quadratic_curve (control: Vector2 <f64>, location: Vector2 <f64>) {
  js! {context.quadraticCurveTo (@{control [0]},@{control [1]},@{location [0]},@{location [1]});}
}
/*pub fn sigmoidneg11(input: f64)->f64 {
  (input*(TURN/4.0)).sin()
}
pub fn sigmoid01(input: f64)->f64 {
  (sigmoidneg11((input*2.0)-1.0)+1.0)/2.0
}*/

pub fn min (first: f64, second: f64)->f64 {if first < second {first} else {second}}
pub fn max (first: f64, second: f64)->f64 {if first > second {first} else {second}}
pub fn safe_normalize (vector: Vector2 <f64>)->Vector2 <f64>{
  let norm = vector.norm();
  if norm == 0.0 {vector} else {vector/norm}
}

