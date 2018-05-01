use std::rc::Rc;
use std::cell::Cell;


//use super::*;


pub fn min (first: f64, second: f64)->f64 {if first < second {first} else {second}}
pub fn max (first: f64, second: f64)->f64 {if first > second {first} else {second}}


#[derive (Clone, PartialEq, Eq)]
pub struct SerialNumber (pub u64);

impl Default for SerialNumber {
  fn default()->Self {
    thread_local! {static NEXT_SERIAL_NUMBER: Cell<u64> = Cell::new (0);}
    NEXT_SERIAL_NUMBER.with (| next | {
      let this = next.get();
      next.set (this + 1);
      SerialNumber (this)
    })
  }
}


#[derive (Derivative)]
#[derivative (Clone (bound =""))]
pub struct Getter <T, U> {
  pub get: Rc <Fn(&T)->&U>,
  pub get_mut: Rc <Fn(&mut T)->&mut U>,
}
impl <T, U> Getter <T, U> {
  pub fn get<'a, 'b> (&'a self, value: &'b T)->&'b U {
    (self.get) (value)
  }
  pub fn get_mut<'a, 'b> (&'a self, value: &'b mut T)->&'b mut U {
    (self.get_mut) (value)
  }
}

impl <T: 'static,U: 'static,V: 'static> ::std::ops::Add<Getter <U, V>> for Getter <T, U> {
  type Output = Getter <T, V>;
  fn add (self, other: Getter <U, V>)->Self::Output {
    let my_get = self.get;
    let my_get_mut = self.get_mut;
    let other_get = other.get;
    let other_get_mut = other.get_mut;
    Getter {
      get: Rc::new (move | value | (other_get) ((my_get) (value))),
      get_mut: Rc::new (move | value | (other_get_mut) ((my_get_mut) (value))),
    }
  }
}

macro_rules! getter {
  ($value: ident => $($path:tt)*) => {
    Getter {
      get    : Rc::new (move | $value | &    $($path)*),
      get_mut: Rc::new (move | $value | &mut $($path)*),
    }
  };
  ($value: ident: $Type: ty => $($path:tt)*) => {
    Getter {
      get    : Rc::new (move | $value: &    $Type | &    $($path)*),
      get_mut: Rc::new (move | $value: &mut $Type | &mut $($path)*),
    }
  };
}
macro_rules! variant_field_getter {
  ($Enum: ident::$Variant: ident => $field: ident) => {
    Getter {
      get    : Rc::new (| value | match value {
        &    $Enum::$Variant {ref     $field,..} => $field,
        _ => unreachable!(),
      }),
      get_mut: Rc::new (| value | match value {
        &mut $Enum::$Variant {ref mut $field,..} => $field,
        _ => unreachable!(),
      }),
    }
  }
}

