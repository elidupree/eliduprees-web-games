use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;
use stdweb::unstable::TryInto;

//use super::*;

pub fn min(first: f64, second: f64) -> f64 {
  if first < second {
    first
  } else {
    second
  }
}
pub fn max(first: f64, second: f64) -> f64 {
  if first > second {
    first
  } else {
    second
  }
}

pub fn audio_now() -> f64 {
  let seconds: f64 = js! { return audio.currentTime}.try_into().unwrap();
  seconds
}

#[derive(Clone, PartialEq, Eq)]
pub struct SerialNumber(pub u64);

impl Default for SerialNumber {
  fn default() -> Self {
    thread_local! {static NEXT_SERIAL_NUMBER: Cell<u64> = Cell::new (0);}
    NEXT_SERIAL_NUMBER.with(|next| {
      let this = next.get();
      next.set(this + 1);
      SerialNumber(this)
    })
  }
}

pub trait GetterBase {
  type From;
  type To;
  fn get<'a, 'b>(&'a self, value: &'b Self::From) -> &'b Self::To
  where
    Self::To: 'b;
  fn get_mut<'a, 'b>(&'a self, value: &'b mut Self::From) -> &'b mut Self::To
  where
    Self::To: 'b;
}

#[derive(Clone)]
pub struct Getter<T>(pub T);
impl<T: GetterBase> GetterBase for Getter<T> {
  type From = T::From;
  type To = T::To;
  fn get<'a, 'b>(&'a self, value: &'b Self::From) -> &'b Self::To
  where
    Self::To: 'b,
  {
    self.0.get(value)
  }
  fn get_mut<'a, 'b>(&'a self, value: &'b mut Self::From) -> &'b mut Self::To
  where
    Self::To: 'b,
  {
    self.0.get_mut(value)
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DynamicGetterInner<From, To>(Rc<dyn GetterBase<From = From, To = To>>);
impl<From, To> GetterBase for DynamicGetterInner<From, To> {
  type From = From;
  type To = To;
  fn get<'a, 'b>(&'a self, value: &'b Self::From) -> &'b Self::To
  where
    Self::To: 'b,
  {
    self.0.get(value)
  }
  fn get_mut<'a, 'b>(&'a self, value: &'b mut Self::From) -> &'b mut Self::To
  where
    Self::To: 'b,
  {
    self.0.get_mut(value)
  }
}

pub type DynamicGetter<From, To> = Getter<DynamicGetterInner<From, To>>;

#[derive(Clone)]
pub struct GetterCompose<T, U>(pub T, pub U);

impl<T: GetterBase + Clone, U: GetterBase<From = T::To> + Clone> ::std::ops::Add<Getter<U>>
  for Getter<T>
{
  type Output = Getter<GetterCompose<T, U>>;
  fn add(self, other: Getter<U>) -> Self::Output {
    Getter(GetterCompose(self.0, other.0))
  }
}

impl<T: 'static + GetterBase> Getter<T> {
  pub fn dynamic(self) -> DynamicGetter<T::From, T::To> {
    Getter(DynamicGetterInner(Rc::new(self.0)))
  }
}

// Note: the condition of the middle type should theoretically be possible to weaken from 'static, but I couldn't figure out how to tell the compiler that
impl<T: GetterBase, U: GetterBase<From = T::To>> GetterBase for GetterCompose<T, U>
where
  T::To: 'static,
{
  type From = T::From;
  type To = U::To;
  fn get<'a, 'b>(&'a self, value: &'b Self::From) -> &'b Self::To
  where
    Self::To: 'b,
  {
    self.1.get(self.0.get(value))
  }
  fn get_mut<'a, 'b>(&'a self, value: &'b mut Self::From) -> &'b mut Self::To
  where
    Self::To: 'b,
  {
    self.1.get_mut(self.0.get_mut(value))
  }
}

#[derive(Clone)]
pub struct GetterClosures<From, To, T, U>(pub T, pub U, pub PhantomData<*const (From, To)>);

impl<From, To, T: Fn(&From) -> &To + Clone, U: Fn(&mut From) -> &mut To + Clone> GetterBase
  for GetterClosures<From, To, T, U>
{
  type From = From;
  type To = To;
  fn get<'a, 'b>(&'a self, value: &'b Self::From) -> &'b Self::To
  where
    Self::To: 'b,
  {
    (self.0)(value)
  }
  fn get_mut<'a, 'b>(&'a self, value: &'b mut Self::From) -> &'b mut Self::To
  where
    Self::To: 'b,
  {
    (self.1)(value)
  }
}

macro_rules! getter {
  /*($value: ident => $($path:tt)*) => {
    Getter(GetterClosures (
      move | $value | &    $($path)*,
      move | $value | &mut $($path)*,
      PhantomData,
    ))
  };*/
  ($value: ident: $Type: ty => $To: ty { $($path:tt)* }) => {
    {
      #[derive(Clone)]
      struct LocalGetter;
      impl GetterBase for LocalGetter {
        type From = $Type;
        type To = $To;
        fn get<'a, 'b> (&'a self, $value: &'b Self::From)->&'b Self::To where Self::To: 'b { &    $($path)* }
        fn get_mut<'a, 'b> (&'a self, $value: &'b mut Self::From)->&'b mut Self::To where Self::To: 'b { &mut $($path)* }
      }
      Getter(LocalGetter)
    }
  };
  ($self_hack: ident@ {$($varname:ident: $vartype: ty = $varval: expr,)*} => $value: ident: $Type: ty => $To: ty { $($path:tt)* }) => {
    {
      #[derive(Clone)]
      struct LocalGetter {$($varname: $vartype,)*}
      impl GetterBase for LocalGetter {
        type From = $Type;
        type To = $To;
        fn get<'a, 'b> (&'a $self_hack, $value: &'b Self::From)->&'b Self::To where Self::To: 'b { &    $($path)* }
        fn get_mut<'a, 'b> (&'a $self_hack, $value: &'b mut Self::From)->&'b mut Self::To where Self::To: 'b { &mut $($path)* }
      }
      Getter(LocalGetter{$($varname: $varval,)*})
    }
  };
  ($self_hack: ident@ <$([$Generic:ident $($Bounds:tt)*])*>{$($varname:ident: $vartype: ty = $varval: expr,)*} => $value: ident: $Type: ty => $To: ty { $($path:tt)* }) => {
    {
      #[derive(Clone)]
      struct LocalGetter<$($Generic $($Bounds)*,)*> {$($varname: $vartype,)*}
      impl<$($Generic $($Bounds)*,)*> GetterBase for LocalGetter<$($Generic,)*> {
        type From = $Type;
        type To = $To;
        fn get<'a, 'b> (&'a $self_hack, $value: &'b Self::From)->&'b Self::To where Self::To: 'b { &    $($path)* }
        fn get_mut<'a, 'b> (&'a $self_hack, $value: &'b mut Self::From)->&'b mut Self::To where Self::To: 'b { &mut $($path)* }
      }
      Getter(LocalGetter{$($varname: $varval,)*})
    }
  };
}
macro_rules! variant_field_getter {
  (<$([$Generic:ident $($Bounds:tt)*])*> $Enum: ident <$EnumArg:ident> =>::$Variant: ident => $field: ident: $Field: ty) => {
    {
      #[derive(Clone)]
      struct LocalGetter<$($Generic $($Bounds)*,)*>(PhantomData<*const ($($Generic,)*)>);
      impl<$($Generic $($Bounds)*,)*> GetterBase for LocalGetter<$($Generic,)*> {
        type From = $Enum<$EnumArg>;
        type To = $Field;
        fn get<'a, 'b> (&'a self, value: &'b Self::From)->&'b Self::To where Self::To: 'b { match value {
        &    $Enum::$Variant {ref     $field,..} => $field,
        _ => panic!("Variant field getter used with the incorrect variant"),
      }}
        fn get_mut<'a, 'b> (&'a self, value: &'b mut Self::From)->&'b mut Self::To where Self::To: 'b { match value {
        &mut $Enum::$Variant {ref mut $field,..} => $field,
        _ => panic!("Variant field getter used with the incorrect variant"),
      } }
      }
      Getter(LocalGetter(PhantomData))
    }
  }
}
