/// Single-source token enum macro.
///
/// Defines a `#[repr(uN)]` enum where each variant's discriminant and
/// associated string are specified once. The macro generates:
///
/// - The enum with `Clone, Copy, Debug, PartialEq, Eq, Hash` derives
/// - An `as_u8()` or `as_u16()` method (based on repr type)
/// - A configurable string method (e.g. `css()`, `name()`, `selector()`)
/// - A `pub const MAPPINGS` array of `(repr_type, &str)` pairs
/// - `From<Enum>` for the repr type
/// - `TryFrom<repr_type>` for the enum (returns `Err(value)` if unknown)
///
/// # Example
///
/// ```ignore
/// define_token_enum! {
///     /// My doc comment.
///     pub enum Foo(u8) {
///         str_method = name;
///         mappings = FOO_MAPPINGS;
///
///         Bar = 0x00 => "bar",
///         Baz = 0x01 => "baz",
///     }
/// }
/// assert_eq!(u8::from(Foo::Bar), 0x00);
/// assert_eq!(Foo::try_from(0x01), Ok(Foo::Baz));
/// assert!(Foo::try_from(0xFF).is_err());
/// ```
macro_rules! define_token_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident ( u8 ) {
            str_method = $str_method:ident;
            mappings = $mappings:ident;

            $(
                $(#[$vmeta:meta])*
                $variant:ident = $code:expr => $str:expr
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum $name {
            $(
                $(#[$vmeta])*
                $variant = $code,
            )+
        }

        impl $name {
            /// Convert to wire protocol byte.
            pub fn as_u8(self) -> u8 {
                self as u8
            }

            pub fn $str_method(self) -> &'static str {
                match self {
                    $( Self::$variant => $str, )+
                }
            }
        }

        impl From<$name> for u8 {
            fn from(v: $name) -> u8 { v as u8 }
        }

        impl TryFrom<u8> for $name {
            type Error = u8;
            fn try_from(v: u8) -> Result<Self, u8> {
                match v {
                    $( $code => Ok(Self::$variant), )+
                    _ => Err(v),
                }
            }
        }

        pub const $mappings: &[(u8, &str)] = &[
            $( ($code, $str), )+
        ];
    };

    (
        $(#[$meta:meta])*
        pub enum $name:ident ( u16 ) {
            str_method = $str_method:ident;
            mappings = $mappings:ident;

            $(
                $(#[$vmeta:meta])*
                $variant:ident = $code:expr => $str:expr
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u16)]
        pub enum $name {
            $(
                $(#[$vmeta])*
                $variant = $code,
            )+
        }

        impl $name {
            /// Convert to wire protocol value.
            pub fn as_u16(self) -> u16 {
                self as u16
            }

            pub fn $str_method(self) -> &'static str {
                match self {
                    $( Self::$variant => $str, )+
                }
            }
        }

        impl From<$name> for u16 {
            fn from(v: $name) -> u16 { v as u16 }
        }

        impl TryFrom<u16> for $name {
            type Error = u16;
            fn try_from(v: u16) -> Result<Self, u16> {
                match v {
                    $( $code => Ok(Self::$variant), )+
                    _ => Err(v),
                }
            }
        }

        pub const $mappings: &[(u16, &str)] = &[
            $( ($code, $str), )+
        ];
    };
}
