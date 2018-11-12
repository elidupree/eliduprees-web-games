use super::*;

use std::iter;
use itertools::Itertools;

pub fn with_optional_next<I: Iterator> (iterator: I)->impl Iterator <Item = (I::Item, Option <I::Item>)> where I::Item: Clone {
  let (first, second) = iterator.tee();
  first.zip (second.map (Some).skip (1).chain (iter::once (None)))
}
