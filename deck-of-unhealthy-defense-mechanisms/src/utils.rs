// Similar to the crate `trait_enum`, but with some improvements and adding From/TryFrom impls

use std::convert::{TryFrom, TryInto};
macro_rules! trait_enum {
    (
        $(#[$attr:meta])*
        $vis:vis enum $Enum:ident: $Trait:tt {
            $(
                $(#[$inner:meta])*
                $Variant:ident,
            )+
        }
    ) => {
        $(#[$attr])*
        $vis enum $Enum {
            $(
                $(#[$inner])*
                $Variant($Variant),
            )*
        }

        impl std::ops::Deref for $Enum {
            type Target = dyn $Trait;

            fn deref(&self) -> &(dyn $Trait + 'static) {
                match self {
                    $(
                        $Enum::$Variant(v) => v as &dyn $Trait,
                    )*
                }
            }
        }

        impl std::ops::DerefMut for $Enum {
            fn deref_mut(&mut self) -> &mut (dyn $Trait + 'static) {
                match self {
                    $(
                        $Enum::$Variant(v) => v as &mut dyn $Trait,
                    )*
                }
            }
        }

    $(
    impl std::convert::TryFrom<$Enum> for $Variant {
      type Error = ();
      fn try_from(value: $Enum) -> Result<$Variant, Self::Error> {
        if let $Enum::$Variant(s) = value {
          Ok(s)
        }
        else {
          Err(())
        }
      }
    }

    impl<'a> std::convert::TryFrom<&'a $Enum> for &'a $Variant {
      type Error = ();
      fn try_from(value: &'a $Enum) -> Result<&'a $Variant, Self::Error> {
        if let $Enum::$Variant(s) = value {
          Ok(s)
        }
        else {
          Err(())
        }
      }
    }

    impl<'a> std::convert::TryFrom<&'a mut $Enum> for &'a mut $Variant {
      type Error = ();
      fn try_from(value: &'a mut $Enum) -> Result<&'a mut $Variant, Self::Error> {
        if let $Enum::$Variant(s) = value {
          Ok(s)
        }
        else {
          Err(())
        }
      }
    }
    )*
    };
}

trait Assume {
  fn assume_ref<'b, T>(&'b self) -> &'b T
  where
    &'b T: TryFrom<&'b Self>,
  {
    match self.try_into() {
      Ok(b) => b,
      Err(_) => panic!("You assumed wrongly!"),
    }
  }
  fn assume<'b, T>(&'b mut self) -> &'b mut T
  where
    &'b mut T: TryFrom<&'b mut Self>,
  {
    match self.try_into() {
      Ok(b) => b,
      Err(_) => panic!("You assumed wrongly!"),
    }
  }
}

impl<T> Assume for T {}
