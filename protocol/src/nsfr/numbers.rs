use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
    ShrAssign,
};

use std::hash::{Hash, Hasher};

use std::cmp::{Ord, Ordering, PartialOrd};

use std::fmt::{Binary, Display, Formatter, LowerHex, Octal, UpperHex};

pub const fn concat<T: Copy, const A: usize, const B: usize, const C: usize>(
    a: [T; A],
    b: [T; B],
) -> [T; C] {
    use std::mem::MaybeUninit;

    // Assert that `A + B == C`.
    // These overflow if that is not the case, which produces an error at compile-time.
    let _ = C - (A + B); // Assert that `A + B <= C`
    let _ = (A + B) - C; // Assert that `A + B >= C`

    // let result: [T; C] = std::array::from_fn(|i| if i < A { a[i] } else { b[i - A] });
    let mut result: [MaybeUninit<T>; C] = unsafe { MaybeUninit::uninit().assume_init() };

    let mut i = 0;
    loop {
        if i == C {
            break;
        }

        (result[i]).write(if i < A { a[i] } else { b[i - A] });

        i += 1;
    }

    unsafe { MaybeUninit::array_assume_init(result) }
}

macro_rules! define_unsigned {
    ($name:ident, $bits:literal, $type:ident) => {define_unsigned!(#[doc=""], $name, $bits, $type);};
    (#[$doc:meta], $name:ident, $bits:literal, $type:ident) => {

        #[$doc]
        #[allow(non_camel_case_types)]
        #[derive(Default, Clone, Copy, Debug)]
        pub struct $name($type);

        impl $name {
            pub const MAX: Self = $name(((1 as $type) << $bits) -1 );
            pub const MIN: Self = $name(0);

            fn mask(self) -> Self {
                $name(self.0 & ( ((1 as $type) << $bits).overflowing_sub(1).0))
            }
        }

        implement_common!($name, $bits, $type);
    }
}

macro_rules! define_signed {
    ($name:ident, $bits:literal, $type:ident) => {define_signed!(#[doc=""], $name, $bits, $type);};
    (#[$doc:meta], $name:ident, $bits:literal, $type:ident) => {
        #[$doc]
        #[allow(non_camel_case_types)]
        #[derive(Default, Clone, Copy, Debug)]
        pub struct $name($type);

        #[$doc]
        impl $name {
            pub const MAX: Self = $name(((1 as $type) << ($bits - 1)) - 1);
            pub const MIN: Self = $name(-((1 as $type) << ($bits - 1)));

            fn mask(self) -> Self {
                if ( self.0 & (1<<($bits-1)) ) == 0 {
                    $name(self.0 & ( ((1 as $type) << $bits).overflowing_sub(1).0))
                } else {
                    $name(self.0 | !( ((1 as $type) << $bits).overflowing_sub(1).0))
                }
            }
        }

        implement_common!($name, $bits, $type);
    }
}

macro_rules! implement_common {
    ($name:ident, $bits:literal, $type:ident) => {
        impl $name {
            /// Returns the smallest value that can be represented by this integer type.
            pub fn min_value() -> $name {
                $name::MIN
            }
            /// Returns the largest value that can be represented by this integer type.
            pub fn max_value() -> $name {
                $name::MAX
            }

            /// Creates [`Self`] from a value of the inner type.
            /// It is undefined behavior to specify an out of bounds value.
            pub unsafe fn new_unchecked(value: $type) -> $name {
                $name(value)
            }

            /// Crates a new variable
            ///
            /// This function mainly exists as there is currently not a better way to construct these types.
            /// May be deprecated or removed if a better way to construct these types becomes available.
            ///
            /// # Examples
            ///
            /// Basic usage:
            ///
            /// ```helpplease
            /// use oxcr_protocol::nsfr::*;
            ///
            /// assert_eq!(u31::new(64), u31::from(64u8));
            ///
            /// ```
            ///
            /// # Panic
            ///
            /// This function will panic if `value` is not representable by this type
            pub const fn new(value: $type) -> $name {
                assert!(value <= $name::MAX.0 && value >= $name::MIN.0);
                $name(value)
            }

            /// Wrapping (modular) subtraction. Computes `self - other`,
            /// wrapping around at the boundary of the type.
            ///
            /// # Examples
            ///
            /// Basic usage:
            ///
            /// ```helpplease
            /// use oxcr_protocol::nsfr::*;
            ///
            /// assert_eq!(i5::MIN.wrapping_sub(i5::new(1)), i5::MAX);
            ///
            /// assert_eq!(i5::new(-10).wrapping_sub(i5::new(5)), i5::new(-15));
            /// assert_eq!(i5::new(-15).wrapping_sub(i5::new(5)), i5::new(12));
            /// ```
            pub fn wrapping_sub(self, rhs: Self) -> Self {
                $name(self.0.wrapping_sub(rhs.0)).mask()
            }

            /// Wrapping (modular) addition. Computes `self + other`,
            /// wrapping around at the boundary of the type.
            ///
            /// # Examples
            ///
            /// Basic usage:
            ///
            /// ```helpplease
            /// use oxcr_protocol::nsfr::*;
            ///
            /// assert_eq!(i5::MAX.wrapping_add(i5::new(1)), i5::MIN);
            ///
            /// assert_eq!(i5::new(10).wrapping_add(i5::new(5)), i5::new(15));
            /// assert_eq!(i5::new(15).wrapping_add(i5::new(5)), i5::new(-12));
            /// ```
            pub fn wrapping_add(self, rhs: Self) -> Self {
                $name(self.0.wrapping_add(rhs.0)).mask()
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.mask().0 == other.mask().0
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &$name) -> Option<Ordering> {
                self.mask().0.partial_cmp(&other.mask().0)
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &$name) -> Ordering {
                self.mask().0.cmp(&other.mask().0)
            }
        }

        impl Hash for $name {
            fn hash<H: Hasher>(&self, h: &mut H) {
                self.mask().0.hash(h)
            }
        }

        // Implement formating functions
        impl Display for $name {
            fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as Display>::fmt(value, f)
            }
        }
        impl UpperHex for $name {
            fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as UpperHex>::fmt(value, f)
            }
        }
        impl LowerHex for $name {
            fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as LowerHex>::fmt(value, f)
            }
        }
        impl Octal for $name {
            fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as Octal>::fmt(value, f)
            }
        }
        impl Binary for $name {
            fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
                let &$name(ref value) = self;
                <$type as Binary>::fmt(value, f)
            }
        }

        impl<T> Shr<T> for $name
        where
            $type: Shr<T, Output = $type>,
        {
            type Output = $name;

            fn shr(self, rhs: T) -> $name {
                $name(self.mask().0.shr(rhs))
            }
        }

        impl<T> Shl<T> for $name
        where
            $type: Shl<T, Output = $type>,
        {
            type Output = $name;

            fn shl(self, rhs: T) -> $name {
                $name(self.mask().0.shl(rhs))
            }
        }

        impl<T> ShrAssign<T> for $name
        where
            $type: ShrAssign<T>,
        {
            fn shr_assign(&mut self, rhs: T) {
                *self = self.mask();
                self.0.shr_assign(rhs);
            }
        }

        impl<T> ShlAssign<T> for $name
        where
            $type: ShlAssign<T>,
        {
            fn shl_assign(&mut self, rhs: T) {
                *self = self.mask();
                self.0.shl_assign(rhs);
            }
        }

        impl BitOr<$name> for $name {
            type Output = $name;

            fn bitor(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitor(rhs.mask().0))
            }
        }

        impl<'a> BitOr<&'a $name> for $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitor(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitor(rhs.mask().0))
            }
        }

        impl<'a> BitOr<$name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitor(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitor(rhs.mask().0))
            }
        }

        impl<'a> BitOr<&'a $name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitor(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitor(rhs.mask().0))
            }
        }

        impl BitOrAssign<$name> for $name {
            fn bitor_assign(&mut self, other: $name) {
                *self = self.mask();
                self.0.bitor_assign(other.mask().0)
            }
        }

        impl BitXor<$name> for $name {
            type Output = $name;

            fn bitxor(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitxor(rhs.mask().0))
            }
        }

        impl<'a> BitXor<&'a $name> for $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitxor(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitxor(rhs.mask().0))
            }
        }

        impl<'a> BitXor<$name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitxor(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitxor(rhs.mask().0))
            }
        }

        impl<'a> BitXor<&'a $name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitxor(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitxor(rhs.mask().0))
            }
        }

        impl BitXorAssign<$name> for $name {
            fn bitxor_assign(&mut self, other: $name) {
                *self = self.mask();
                self.0.bitxor_assign(other.mask().0)
            }
        }

        impl Not for $name {
            type Output = $name;

            fn not(self) -> $name {
                $name(self.mask().0.not())
            }
        }

        impl<'a> Not for &'a $name {
            type Output = <$name as Not>::Output;

            fn not(self) -> $name {
                $name(self.mask().0.not())
            }
        }

        impl BitAnd<$name> for $name {
            type Output = $name;

            fn bitand(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitand(rhs.mask().0))
            }
        }

        impl<'a> BitAnd<&'a $name> for $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitand(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitand(rhs.mask().0))
            }
        }

        impl<'a> BitAnd<$name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitand(self, rhs: $name) -> Self::Output {
                $name(self.mask().0.bitand(rhs.mask().0))
            }
        }

        impl<'a> BitAnd<&'a $name> for &'a $name {
            type Output = <$name as BitOr<$name>>::Output;

            fn bitand(self, rhs: &'a $name) -> Self::Output {
                $name(self.mask().0.bitand(rhs.mask().0))
            }
        }

        impl BitAndAssign<$name> for $name {
            fn bitand_assign(&mut self, other: $name) {
                *self = self.mask();
                self.0.bitand_assign(other.mask().0)
            }
        }

        impl std::ops::Add<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn add(self, other: $name) -> $name {
                if self.0 > 0 && other.0 > 0 {
                    debug_assert!(Self::MAX.0 - other.0 >= self.0);
                } else if self.0 < 0 && other.0 < 0 {
                    debug_assert!(Self::MIN.0 - other.0 <= self.0);
                }
                self.wrapping_add(other)
            }
        }

        impl std::ops::Sub<$name> for $name {
            type Output = $name;
            #[allow(unused_comparisons)]
            fn sub(self, other: $name) -> $name {
                if self > other {
                    debug_assert!(Self::MAX.0 + other.0 >= self.0);
                } else if self < other {
                    debug_assert!(Self::MIN.0 + other.0 <= self.0);
                }
                self.wrapping_sub(other)
            }
        }
    };
}

define_unsigned!(#[doc="The 1-bit unsigned integer type."], u1, 1, u8);
define_unsigned!(#[doc="The 2-bit unsigned integer type."], u2, 2, u8);
define_unsigned!(#[doc="The 3-bit unsigned integer type."], u3, 3, u8);
define_unsigned!(#[doc="The 4-bit unsigned integer type."], u4, 4, u8);
define_unsigned!(#[doc="The 5-bit unsigned integer type."], u5, 5, u8);
define_unsigned!(#[doc="The 6-bit unsigned integer type."], u6, 6, u8);
define_unsigned!(#[doc="The 7-bit unsigned integer type."], u7, 7, u8);

define_unsigned!(#[doc="The 9-bit unsigned integer type."], u9, 9, u16);
define_unsigned!(#[doc="The 10-bit unsigned integer type."], u10, 10, u16);
define_unsigned!(#[doc="The 11-bit unsigned integer type."], u11, 11, u16);
define_unsigned!(#[doc="The 12-bit unsigned integer type."], u12, 12, u16);
define_unsigned!(#[doc="The 13-bit unsigned integer type."], u13, 13, u16);
define_unsigned!(#[doc="The 14-bit unsigned integer type."], u14, 14, u16);
define_unsigned!(#[doc="The 15-bit unsigned integer type."], u15, 15, u16);

define_unsigned!(#[doc="The 17-bit unsigned integer type."], u17, 17, u32);
define_unsigned!(#[doc="The 18-bit unsigned integer type."], u18, 18, u32);
define_unsigned!(#[doc="The 19-bit unsigned integer type."], u19, 19, u32);
define_unsigned!(#[doc="The 20-bit unsigned integer type."], u20, 20, u32);
define_unsigned!(#[doc="The 21-bit unsigned integer type."], u21, 21, u32);
define_unsigned!(#[doc="The 22-bit unsigned integer type."], u22, 22, u32);
define_unsigned!(#[doc="The 23-bit unsigned integer type."], u23, 23, u32);
define_unsigned!(#[doc="The 24-bit unsigned integer type."], u24, 24, u32);

define_unsigned!(#[doc="The 25-bit unsigned integer type."], u25, 25, u32);
define_unsigned!(#[doc="The 26-bit unsigned integer type."], u26, 26, u32);
define_unsigned!(#[doc="The 27-bit unsigned integer type."], u27, 27, u32);
define_unsigned!(#[doc="The 28-bit unsigned integer type."], u28, 28, u32);
define_unsigned!(#[doc="The 29-bit unsigned integer type."], u29, 29, u32);
define_unsigned!(#[doc="The 30-bit unsigned integer type."], u30, 30, u32);
define_unsigned!(#[doc="The 31-bit unsigned integer type."], u31, 31, u32);

define_unsigned!(#[doc="The 33-bit unsigned integer type."], u33, 33, u64);
define_unsigned!(#[doc="The 34-bit unsigned integer type."], u34, 34, u64);
define_unsigned!(#[doc="The 35-bit unsigned integer type."], u35, 35, u64);
define_unsigned!(#[doc="The 36-bit unsigned integer type."], u36, 36, u64);
define_unsigned!(#[doc="The 37-bit unsigned integer type."], u37, 37, u64);
define_unsigned!(#[doc="The 38-bit unsigned integer type."], u38, 38, u64);
define_unsigned!(#[doc="The 39-bit unsigned integer type."], u39, 39, u64);
define_unsigned!(#[doc="The 40-bit unsigned integer type."], u40, 40, u64);

define_unsigned!(#[doc="The 41-bit unsigned integer type."], u41, 41, u64);
define_unsigned!(#[doc="The 42-bit unsigned integer type."], u42, 42, u64);
define_unsigned!(#[doc="The 43-bit unsigned integer type."], u43, 43, u64);
define_unsigned!(#[doc="The 44-bit unsigned integer type."], u44, 44, u64);
define_unsigned!(#[doc="The 45-bit unsigned integer type."], u45, 45, u64);
define_unsigned!(#[doc="The 46-bit unsigned integer type."], u46, 46, u64);
define_unsigned!(#[doc="The 47-bit unsigned integer type."], u47, 47, u64);
define_unsigned!(#[doc="The 48-bit unsigned integer type."], u48, 48, u64);

define_unsigned!(#[doc="The 49-bit unsigned integer type."], u49, 49, u64);
define_unsigned!(#[doc="The 50-bit unsigned integer type."], u50, 50, u64);
define_unsigned!(#[doc="The 51-bit unsigned integer type."], u51, 51, u64);
define_unsigned!(#[doc="The 52-bit unsigned integer type."], u52, 52, u64);
define_unsigned!(#[doc="The 53-bit unsigned integer type."], u53, 53, u64);
define_unsigned!(#[doc="The 54-bit unsigned integer type."], u54, 54, u64);
define_unsigned!(#[doc="The 55-bit unsigned integer type."], u55, 55, u64);
define_unsigned!(#[doc="The 56-bit unsigned integer type."], u56, 56, u64);

define_unsigned!(#[doc="The 57-bit unsigned integer type."], u57, 57, u64);
define_unsigned!(#[doc="The 58-bit unsigned integer type."], u58, 58, u64);
define_unsigned!(#[doc="The 59-bit unsigned integer type."], u59, 59, u64);
define_unsigned!(#[doc="The 60-bit unsigned integer type."], u60, 60, u64);
define_unsigned!(#[doc="The 61-bit unsigned integer type."], u61, 61, u64);
define_unsigned!(#[doc="The 62-bit unsigned integer type."], u62, 62, u64);
define_unsigned!(#[doc="The 63-bit unsigned integer type."], u63, 63, u64);

define_unsigned!(#[doc="The 65-bit unsigned integer type."], u65, 65, u128);
define_unsigned!(#[doc="The 66-bit unsigned integer type."], u66, 66, u128);
define_unsigned!(#[doc="The 67-bit unsigned integer type."], u67, 67, u128);
define_unsigned!(#[doc="The 68-bit unsigned integer type."], u68, 68, u128);
define_unsigned!(#[doc="The 69-bit unsigned integer type."], u69, 69, u128);
define_unsigned!(#[doc="The 70-bit unsigned integer type."], u70, 70, u128);
define_unsigned!(#[doc="The 71-bit unsigned integer type."], u71, 71, u128);
define_unsigned!(#[doc="The 72-bit unsigned integer type."], u72, 72, u128);

define_unsigned!(#[doc="The 73-bit unsigned integer type."], u73, 73, u128);
define_unsigned!(#[doc="The 74-bit unsigned integer type."], u74, 74, u128);
define_unsigned!(#[doc="The 75-bit unsigned integer type."], u75, 75, u128);
define_unsigned!(#[doc="The 76-bit unsigned integer type."], u76, 76, u128);
define_unsigned!(#[doc="The 77-bit unsigned integer type."], u77, 77, u128);
define_unsigned!(#[doc="The 78-bit unsigned integer type."], u78, 78, u128);
define_unsigned!(#[doc="The 79-bit unsigned integer type."], u79, 79, u128);
define_unsigned!(#[doc="The 80-bit unsigned integer type."], u80, 80, u128);

define_unsigned!(#[doc="The 81-bit unsigned integer type."], u81, 81, u128);
define_unsigned!(#[doc="The 82-bit unsigned integer type."], u82, 82, u128);
define_unsigned!(#[doc="The 83-bit unsigned integer type."], u83, 83, u128);
define_unsigned!(#[doc="The 84-bit unsigned integer type."], u84, 84, u128);
define_unsigned!(#[doc="The 85-bit unsigned integer type."], u85, 85, u128);
define_unsigned!(#[doc="The 86-bit unsigned integer type."], u86, 86, u128);
define_unsigned!(#[doc="The 87-bit unsigned integer type."], u87, 87, u128);
define_unsigned!(#[doc="The 88-bit unsigned integer type."], u88, 88, u128);

define_unsigned!(#[doc="The 89-bit unsigned integer type."], u89, 89, u128);
define_unsigned!(#[doc="The 90-bit unsigned integer type."], u90, 90, u128);
define_unsigned!(#[doc="The 91-bit unsigned integer type."], u91, 91, u128);
define_unsigned!(#[doc="The 92-bit unsigned integer type."], u92, 92, u128);
define_unsigned!(#[doc="The 93-bit unsigned integer type."], u93, 93, u128);
define_unsigned!(#[doc="The 94-bit unsigned integer type."], u94, 94, u128);
define_unsigned!(#[doc="The 95-bit unsigned integer type."], u95, 95, u128);
define_unsigned!(#[doc="The 96-bit unsigned integer type."], u96, 96, u128);

define_unsigned!(#[doc="The 97-bit unsigned integer type."], u97, 97, u128);
define_unsigned!(#[doc="The 98-bit unsigned integer type."], u98, 98, u128);
define_unsigned!(#[doc="The 99-bit unsigned integer type."], u99, 99, u128);
define_unsigned!(#[doc="The 100-bit unsigned integer type."], u100, 100, u128);
define_unsigned!(#[doc="The 101-bit unsigned integer type."], u101, 101, u128);
define_unsigned!(#[doc="The 102-bit unsigned integer type."], u102, 102, u128);
define_unsigned!(#[doc="The 103-bit unsigned integer type."], u103, 103, u128);
define_unsigned!(#[doc="The 104-bit unsigned integer type."], u104, 104, u128);

define_unsigned!(#[doc="The 105-bit unsigned integer type."], u105, 105, u128);
define_unsigned!(#[doc="The 106-bit unsigned integer type."], u106, 106, u128);
define_unsigned!(#[doc="The 107-bit unsigned integer type."], u107, 107, u128);
define_unsigned!(#[doc="The 108-bit unsigned integer type."], u108, 108, u128);
define_unsigned!(#[doc="The 109-bit unsigned integer type."], u109, 109, u128);
define_unsigned!(#[doc="The 110-bit unsigned integer type."], u110, 110, u128);
define_unsigned!(#[doc="The 111-bit unsigned integer type."], u111, 111, u128);
define_unsigned!(#[doc="The 112-bit unsigned integer type."], u112, 112, u128);

define_unsigned!(#[doc="The 113-bit unsigned integer type."], u113, 113, u128);
define_unsigned!(#[doc="The 114-bit unsigned integer type."], u114, 114, u128);
define_unsigned!(#[doc="The 115-bit unsigned integer type."], u115, 115, u128);
define_unsigned!(#[doc="The 116-bit unsigned integer type."], u116, 116, u128);
define_unsigned!(#[doc="The 117-bit unsigned integer type."], u117, 117, u128);
define_unsigned!(#[doc="The 118-bit unsigned integer type."], u118, 118, u128);
define_unsigned!(#[doc="The 119-bit unsigned integer type."], u119, 119, u128);
define_unsigned!(#[doc="The 120-bit unsigned integer type."], u120, 120, u128);

define_unsigned!(#[doc="The 121-bit unsigned integer type."], u121, 121, u128);
define_unsigned!(#[doc="The 122-bit unsigned integer type."], u122, 122, u128);
define_unsigned!(#[doc="The 123-bit unsigned integer type."], u123, 123, u128);
define_unsigned!(#[doc="The 124-bit unsigned integer type."], u124, 124, u128);
define_unsigned!(#[doc="The 125-bit unsigned integer type."], u125, 125, u128);
define_unsigned!(#[doc="The 126-bit unsigned integer type."], u126, 126, u128);
define_unsigned!(#[doc="The 127-bit unsigned integer type."], u127, 127, u128);

define_signed!(#[doc="The 1-bit signed integer type."], i1, 1, i8);
define_signed!(#[doc="The 2-bit signed integer type."], i2, 2, i8);
define_signed!(#[doc="The 3-bit signed integer type."], i3, 3, i8);
define_signed!(#[doc="The 4-bit signed integer type."], i4, 4, i8);
define_signed!(#[doc="The 5-bit signed integer type."], i5, 5, i8);
define_signed!(#[doc="The 6-bit signed integer type."], i6, 6, i8);
define_signed!(#[doc="The 7-bit signed integer type."], i7, 7, i8);

define_signed!(#[doc="The 9-bit signed integer type."], i9, 9, i16);
define_signed!(#[doc="The 10-bit signed integer type."], i10, 10, i16);
define_signed!(#[doc="The 11-bit signed integer type."], i11, 11, i16);
define_signed!(#[doc="The 12-bit signed integer type."], i12, 12, i16);
define_signed!(#[doc="The 13-bit signed integer type."], i13, 13, i16);
define_signed!(#[doc="The 14-bit signed integer type."], i14, 14, i16);
define_signed!(#[doc="The 15-bit signed integer type."], i15, 15, i16);

define_signed!(#[doc="The 17-bit signed integer type."], i17, 17, i32);
define_signed!(#[doc="The 18-bit signed integer type."], i18, 18, i32);
define_signed!(#[doc="The 19-bit signed integer type."], i19, 19, i32);
define_signed!(#[doc="The 20-bit signed integer type."], i20, 20, i32);
define_signed!(#[doc="The 21-bit signed integer type."], i21, 21, i32);
define_signed!(#[doc="The 22-bit signed integer type."], i22, 22, i32);
define_signed!(#[doc="The 23-bit signed integer type."], i23, 23, i32);
define_signed!(#[doc="The 24-bit signed integer type."], i24, 24, i32);

define_signed!(#[doc="The 25-bit signed integer type."], i25, 25, i32);
define_signed!(#[doc="The 26-bit signed integer type."], i26, 26, i32);
define_signed!(#[doc="The 27-bit signed integer type."], i27, 27, i32);
define_signed!(#[doc="The 28-bit signed integer type."], i28, 28, i32);
define_signed!(#[doc="The 29-bit signed integer type."], i29, 29, i32);
define_signed!(#[doc="The 30-bit signed integer type."], i30, 30, i32);
define_signed!(#[doc="The 31-bit signed integer type."], i31, 31, i32);

define_signed!(#[doc="The 33-bit signed integer type."], i33, 33, i64);
define_signed!(#[doc="The 34-bit signed integer type."], i34, 34, i64);
define_signed!(#[doc="The 35-bit signed integer type."], i35, 35, i64);
define_signed!(#[doc="The 36-bit signed integer type."], i36, 36, i64);
define_signed!(#[doc="The 37-bit signed integer type."], i37, 37, i64);
define_signed!(#[doc="The 38-bit signed integer type."], i38, 38, i64);
define_signed!(#[doc="The 39-bit signed integer type."], i39, 39, i64);
define_signed!(#[doc="The 40-bit signed integer type."], i40, 40, i64);

define_signed!(#[doc="The 41-bit signed integer type."], i41, 41, i64);
define_signed!(#[doc="The 42-bit signed integer type."], i42, 42, i64);
define_signed!(#[doc="The 43-bit signed integer type."], i43, 43, i64);
define_signed!(#[doc="The 44-bit signed integer type."], i44, 44, i64);
define_signed!(#[doc="The 45-bit signed integer type."], i45, 45, i64);
define_signed!(#[doc="The 46-bit signed integer type."], i46, 46, i64);
define_signed!(#[doc="The 47-bit signed integer type."], i47, 47, i64);
define_signed!(#[doc="The 48-bit signed integer type."], i48, 48, i64);

define_signed!(#[doc="The 49-bit signed integer type."], i49, 49, i64);
define_signed!(#[doc="The 50-bit signed integer type."], i50, 50, i64);
define_signed!(#[doc="The 51-bit signed integer type."], i51, 51, i64);
define_signed!(#[doc="The 52-bit signed integer type."], i52, 52, i64);
define_signed!(#[doc="The 53-bit signed integer type."], i53, 53, i64);
define_signed!(#[doc="The 54-bit signed integer type."], i54, 54, i64);
define_signed!(#[doc="The 55-bit signed integer type."], i55, 55, i64);
define_signed!(#[doc="The 56-bit signed integer type."], i56, 56, i64);

define_signed!(#[doc="The 57-bit signed integer type."], i57, 57, i64);
define_signed!(#[doc="The 58-bit signed integer type."], i58, 58, i64);
define_signed!(#[doc="The 59-bit signed integer type."], i59, 59, i64);
define_signed!(#[doc="The 60-bit signed integer type."], i60, 60, i64);
define_signed!(#[doc="The 61-bit signed integer type."], i61, 61, i64);
define_signed!(#[doc="The 62-bit signed integer type."], i62, 62, i64);
define_signed!(#[doc="The 63-bit signed integer type."], i63, 63, i64);

define_signed!(#[doc="The 65-bit signed integer type."], i65, 65, i128);
define_signed!(#[doc="The 66-bit signed integer type."], i66, 66, i128);
define_signed!(#[doc="The 67-bit signed integer type."], i67, 67, i128);
define_signed!(#[doc="The 68-bit signed integer type."], i68, 68, i128);
define_signed!(#[doc="The 69-bit signed integer type."], i69, 69, i128);
define_signed!(#[doc="The 70-bit signed integer type."], i70, 70, i128);
define_signed!(#[doc="The 71-bit signed integer type."], i71, 71, i128);
define_signed!(#[doc="The 72-bit signed integer type."], i72, 72, i128);

define_signed!(#[doc="The 73-bit signed integer type."], i73, 73, i128);
define_signed!(#[doc="The 74-bit signed integer type."], i74, 74, i128);
define_signed!(#[doc="The 75-bit signed integer type."], i75, 75, i128);
define_signed!(#[doc="The 76-bit signed integer type."], i76, 76, i128);
define_signed!(#[doc="The 77-bit signed integer type."], i77, 77, i128);
define_signed!(#[doc="The 78-bit signed integer type."], i78, 78, i128);
define_signed!(#[doc="The 79-bit signed integer type."], i79, 79, i128);
define_signed!(#[doc="The 80-bit signed integer type."], i80, 80, i128);

define_signed!(#[doc="The 81-bit signed integer type."], i81, 81, i128);
define_signed!(#[doc="The 82-bit signed integer type."], i82, 82, i128);
define_signed!(#[doc="The 83-bit signed integer type."], i83, 83, i128);
define_signed!(#[doc="The 84-bit signed integer type."], i84, 84, i128);
define_signed!(#[doc="The 85-bit signed integer type."], i85, 85, i128);
define_signed!(#[doc="The 86-bit signed integer type."], i86, 86, i128);
define_signed!(#[doc="The 87-bit signed integer type."], i87, 87, i128);
define_signed!(#[doc="The 88-bit signed integer type."], i88, 88, i128);

define_signed!(#[doc="The 89-bit signed integer type."], i89, 89, i128);
define_signed!(#[doc="The 90-bit signed integer type."], i90, 90, i128);
define_signed!(#[doc="The 91-bit signed integer type."], i91, 91, i128);
define_signed!(#[doc="The 92-bit signed integer type."], i92, 92, i128);
define_signed!(#[doc="The 93-bit signed integer type."], i93, 93, i128);
define_signed!(#[doc="The 94-bit signed integer type."], i94, 94, i128);
define_signed!(#[doc="The 95-bit signed integer type."], i95, 95, i128);
define_signed!(#[doc="The 96-bit signed integer type."], i96, 96, i128);

define_signed!(#[doc="The 97-bit signed integer type."], i97, 97, i128);
define_signed!(#[doc="The 98-bit signed integer type."], i98, 98, i128);
define_signed!(#[doc="The 99-bit signed integer type."], i99, 99, i128);
define_signed!(#[doc="The 100-bit signed integer type."], i100, 100, i128);
define_signed!(#[doc="The 101-bit signed integer type."], i101, 101, i128);
define_signed!(#[doc="The 102-bit signed integer type."], i102, 102, i128);
define_signed!(#[doc="The 103-bit signed integer type."], i103, 103, i128);
define_signed!(#[doc="The 104-bit signed integer type."], i104, 104, i128);

define_signed!(#[doc="The 105-bit signed integer type."], i105, 105, i128);
define_signed!(#[doc="The 106-bit signed integer type."], i106, 106, i128);
define_signed!(#[doc="The 107-bit signed integer type."], i107, 107, i128);
define_signed!(#[doc="The 108-bit signed integer type."], i108, 108, i128);
define_signed!(#[doc="The 109-bit signed integer type."], i109, 109, i128);
define_signed!(#[doc="The 110-bit signed integer type."], i110, 110, i128);
define_signed!(#[doc="The 111-bit signed integer type."], i111, 111, i128);
define_signed!(#[doc="The 112-bit signed integer type."], i112, 112, i128);

define_signed!(#[doc="The 113-bit signed integer type."], i113, 113, i128);
define_signed!(#[doc="The 114-bit signed integer type."], i114, 114, i128);
define_signed!(#[doc="The 115-bit signed integer type."], i115, 115, i128);
define_signed!(#[doc="The 116-bit signed integer type."], i116, 116, i128);
define_signed!(#[doc="The 117-bit signed integer type."], i117, 117, i128);
define_signed!(#[doc="The 118-bit signed integer type."], i118, 118, i128);
define_signed!(#[doc="The 119-bit signed integer type."], i119, 119, i128);
define_signed!(#[doc="The 120-bit signed integer type."], i120, 120, i128);

define_signed!(#[doc="The 121-bit signed integer type."], i121, 121, i128);
define_signed!(#[doc="The 122-bit signed integer type."], i122, 122, i128);
define_signed!(#[doc="The 123-bit signed integer type."], i123, 123, i128);
define_signed!(#[doc="The 124-bit signed integer type."], i124, 124, i128);
define_signed!(#[doc="The 125-bit signed integer type."], i125, 125, i128);
define_signed!(#[doc="The 126-bit signed integer type."], i126, 126, i128);
define_signed!(#[doc="The 127-bit signed integer type."], i127, 127, i128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masking() {
        assert_eq!(u4(0b11000110).mask().0, 0b00000110);
        assert_eq!(u4(0b00001000).mask().0, 0b00001000);
        assert_eq!(u4(0b00001110).mask().0, 0b00001110);

        assert_eq!(i4(0b11000110u8 as i8).mask().0, 0b00000110u8 as i8);
        assert_eq!(i4(0b00001000u8 as i8).mask().0, 0b11111000u8 as i8);
        assert_eq!(i4(0b00001110u8 as i8).mask().0, 0b11111110u8 as i8);
    }

    #[test]
    fn min_max_values() {
        assert_eq!(u1::MAX, u1(1));
        assert_eq!(u2::MAX, u2(3));
        assert_eq!(u3::MAX, u3(7));
        assert_eq!(u7::MAX, u7(127));
        assert_eq!(u9::MAX, u9(511));

        assert_eq!(i1::MAX, i1(0));
        assert_eq!(i2::MAX, i2(1));
        assert_eq!(i3::MAX, i3(3));
        assert_eq!(i7::MAX, i7(63));
        assert_eq!(i9::MAX, i9(255));

        assert_eq!(u1::MIN, u1(0));
        assert_eq!(u2::MIN, u2(0));
        assert_eq!(u3::MIN, u3(0));
        assert_eq!(u7::MIN, u7(0));
        assert_eq!(u9::MIN, u9(0));
        assert_eq!(u127::MIN, u127(0));

        assert_eq!(i1::MIN, i1(-1));
        assert_eq!(i2::MIN, i2(-2));
        assert_eq!(i3::MIN, i3(-4));
        assert_eq!(i7::MIN, i7(-64));
        assert_eq!(i9::MIN, i9(-256));
    }

    #[test]
    fn test_wrapping_add() {
        assert_eq!(u1::MAX.wrapping_add(u1(1)), u1(0));
        assert_eq!(u1::MAX.wrapping_add(u1(0)), u1(1));

        assert_eq!(u5::MAX.wrapping_add(u5(1)), u5(0));
        assert_eq!(u5::MAX.wrapping_add(u5(4)), u5(3));

        assert_eq!(u127::MAX.wrapping_add(u127(100)), u127(99));
        assert_eq!(u127::MAX.wrapping_add(u127(1)), u127(0));

        assert_eq!(i1::MAX.wrapping_add(i1(0)), i1(0));
        assert_eq!(i1::MAX.wrapping_add(i1(-1)), i1(-1));

        assert_eq!(i7::MAX.wrapping_add(i7(1)), i7::MIN);
        assert_eq!(i7::MAX.wrapping_add(i7(4)), i7(-61));
    }

    #[test]
    #[should_panic]
    fn test_add_overflow_u5() {
        let _s = u5::MAX + u5(1);
    }

    #[test]
    #[should_panic]
    fn test_add_overflow_u127() {
        let _s = u127::MAX + u127(1);
    }

    #[test]
    #[should_panic]
    fn test_add_overflow_i96() {
        let _s = i96::MAX + i96(100);
    }

    #[test]
    #[should_panic]
    fn test_add_underflow_i96() {
        let _s = i96::MIN + i96(-100);
    }

    #[test]
    #[should_panic]
    fn test_add_underflow_i17() {
        let _s = i17::MIN + i17(-1);
    }

    #[test]
    fn test_add() {
        assert_eq!(u5(1) + u5(2), u5(3));

        assert_eq!(i7::MAX + i7::MIN, i7(-1));
        assert_eq!(i7(4) + i7(-3), i7(1));
        assert_eq!(i7(-4) + i7(3), i7(-1));
        assert_eq!(i7(-3) + i7(-20), i7(-23));
    }

    #[test]
    #[should_panic]
    fn test_sub_overflow_i23() {
        let _s = i23::MIN - i23::MAX;
    }

    #[test]
    #[should_panic]
    fn test_sub_underflow_u5() {
        let _s = u5::MIN - u5(1);
    }

    #[test]
    #[should_panic]
    fn test_sub_underflow_i5() {
        let _s = i5::MIN - i5(1);
    }

    #[test]
    fn test_sub() {
        assert_eq!(u5(1) - u5(1), u5(0));
        assert_eq!(u5(3) - u5(2), u5(1));

        assert_eq!(i1(-1) - i1(-1), i1(0));
        assert_eq!(i7::MIN - i7::MIN, i7(0));
        assert_eq!(i7(4) - i7(-3), i7(7));
        assert_eq!(i7(-4) - i7(3), i7(-7));
        assert_eq!(i7(-3) - i7(-20), i7(17));
    }

    #[test]
    fn test_shr() {
        assert_eq!(u5(8) >> 1usize, u5(4));
        assert_eq!(u5(8) >> 1u8, u5(4));
        assert_eq!(u5(8) >> 1u16, u5(4));
        assert_eq!(u5(8) >> 1u32, u5(4));
        assert_eq!(u5(8) >> 1u64, u5(4));
        assert_eq!(u5(8) >> 1isize, u5(4));
        assert_eq!(u5(8) >> 1i8, u5(4));
        assert_eq!(u5(8) >> 1i16, u5(4));
        assert_eq!(u5(8) >> 1i32, u5(4));
        assert_eq!(u5(8) >> 1i64, u5(4));

        assert_eq!(u5::MAX >> 4, u5(1));

        assert_eq!(i7(-1) >> 5, i7(-1));
    }

    #[test]
    fn test_shl() {
        assert_eq!(u5(16) << 1usize, u5(32));
        assert_eq!(u5(16) << 1u8, u5(32));
        assert_eq!(u5(16) << 1u16, u5(32));
        assert_eq!(u5(16) << 1u32, u5(32));
        assert_eq!(u5(16) << 1u64, u5(32));
        assert_eq!(u5(16) << 1isize, u5(32));
        assert_eq!(u5(16) << 1i8, u5(32));
        assert_eq!(u5(16) << 1i16, u5(32));
        assert_eq!(u5(16) << 1i32, u5(32));
        assert_eq!(u5(16) << 1i64, u5(32));

        assert_eq!(u5::MAX << 4, u5(16));

        assert_eq!(i5(16) << 1, i5(0));
        assert_eq!(i7(1) << 3, i7(8));
    }

    #[test]
    fn test_shr_assign() {
        let mut x = u10(512);
        x >>= 1usize;
        assert_eq!(x, u10(256));
        x >>= 1isize;
        assert_eq!(x, u10(128));
        x >>= 1u8;
        assert_eq!(x, u10(64));
        x >>= 1i8;
        assert_eq!(x, u10(32));
        x >>= 2u64;
        assert_eq!(x, u10(8));
        x >>= 3i32;
        assert_eq!(x, u10(1));
    }

    #[test]
    fn test_shl_assign() {
        let mut x = u9(1);
        x <<= 3i32;
        assert_eq!(x, u9(8));
        x <<= 2u64;
        assert_eq!(x, u9(32));
        x <<= 1usize;
        assert_eq!(x, u9(64));
        x <<= 1isize;
        assert_eq!(x, u9(128));
        x <<= 1u8;
        assert_eq!(x, u9(256));
    }

    #[test]
    fn test_bitor() {
        assert_eq!(u9(1) | u9(8), u9(9));
        assert_eq!(&u9(1) | u9(8), u9(9));
        assert_eq!(u9(1) | &u9(8), u9(9));
        assert_eq!(&u9(1) | &u9(8), u9(9));
    }

    #[test]
    fn test_bitor_assign() {
        let mut x = u12(4);
        x |= u12(1);
        assert_eq!(x, u12(5));
        x |= u12(128);
        assert_eq!(x, u12(133));
        x = u12(1);
        x |= u12(127);
        assert_eq!(x, u12(127));
    }

    #[test]
    fn test_bitxor() {
        assert_eq!(u7(0x7F) ^ u7(42), u7(85));
        assert_eq!(&u7(0) ^ u7(42), u7(42));
        assert_eq!(u7(0x10) ^ &u7(0x1), u7(0x11));
        assert_eq!(&u7(11) ^ &u7(1), u7(10));
    }

    #[test]
    fn test_bitxor_assign() {
        let mut x = u12(4);
        x ^= u12(1);
        assert_eq!(x, u12(5));
        x ^= u12(128);
        assert_eq!(x, u12(133));
        x ^= u12(1);
        assert_eq!(x, u12(132));
        x ^= u12(127);
        assert_eq!(x, u12(251));
    }

    #[test]
    fn test_bitand() {
        assert_eq!(i9(-7) & i9(-9), i9::from(-7i8 & -9i8));
        assert_eq!(&i9(-7) & i9(-9), i9::from(&-7i8 & -9i8));
        assert_eq!(i9(-7) & &i9(-9), i9::from(-7i8 & &-9i8));
        assert_eq!(&i9(-7) & &i9(-9), i9::from(&-7i8 & &-9i8));

        assert_eq!(u9(8) & u9(9), u9(8));
        assert_eq!(&u9(8) & u9(9), u9(8));
        assert_eq!(u9(8) & &u9(9), u9(8));
        assert_eq!(&u9(8) & &u9(9), u9(8));
    }

    #[test]
    fn test_bitand_assign() {
        let mut x = u12(255);
        x &= u12(127);
        assert_eq!(x, u12(127));
        x &= u12(7);
        assert_eq!(x, u12(7));
        x &= u12(127);
        assert_eq!(x, u12(7));
        x &= u12(4);
        assert_eq!(x, u12(4));
    }

    #[test]
    fn test_not() {
        assert_eq!(!u7(42), u7(85));
        assert_eq!(!u7(0x7F), u7(0));
        assert_eq!(!u7(0), u7(0x7F));
        assert_eq!(!u7(56), u7(71));
    }
}

/// The error type returned when a checked integral type conversion fails.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromIntError(pub(crate) ());

impl From<std::num::TryFromIntError> for TryFromIntError {
    fn from(_: std::num::TryFromIntError) -> TryFromIntError {
        TryFromIntError(())
    }
}

impl From<std::convert::Infallible> for TryFromIntError {
    fn from(_: std::convert::Infallible) -> TryFromIntError {
        TryFromIntError(())
    }
}

impl Display for TryFromIntError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "out of range integral type conversion attempted")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TryFromIntError {}

// Only implement if $from can be converted into $name lossless
macro_rules! implement_from {
    {[$($name:ident),*], [$($from:ident),*] } => {$(implement_from!($name, $from);)*};
    {$name:ident, [$($from:ident),*] } => {$(implement_from!($name, $from);)*};
    {[$($name:ident),*], $from:ident } => {$(implement_from!($name, $from);)*};
    {$name:ident, $from:ty} => {
        impl From<$from> for $name {
            fn from(x: $from) -> $name {
                $name(x.into())
            }
        }
    };
}

macro_rules! implement_try_from {
    {[$($name:ident),*], [$($from:ident),*] } => {$(implement_try_from!($name, $from);)*};
    {$name:ident, [$($from:ident),*] } => {$(implement_try_from!($name, $from);)*};
    {[$($name:ident),*], $from:ident } => {$(implement_try_from!($name, $from);)*};
    {$name:ident, $from:ty} => {
        impl TryFrom<$from> for $name {
            type Error = TryFromIntError;

            fn try_from(x: $from) -> Result<$name, Self::Error> {
                // First get the value into the correct type
                let value = x.try_into()?;

                if value <= $name::MAX.into() && value >= $name::MIN.into() {
                    Ok($name(value))
                } else {
                    Err(TryFromIntError(()))
                }
            }
        }
    };
}

// Only implement if $type can be converted from $name lossless
macro_rules! implement_into {
    {[$($name:ident),*], $from:ident } => {$(implement_into!($name, $from);)*};
    {$name:ident, $into:ident} => {
        impl From<$name> for $into {
            fn from(x: $name) -> $into {
                $into::from(x.0)
            }
        }
    };
}

macro_rules! implement_try_into {
    {[$($name:ident),*], $from:ident } => {$(implement_try_into!($name, $from);)*};
    {$name:ident, $into:ident} => {

        impl TryFrom<$name> for $into {
            type Error = TryFromIntError;

            fn try_from(x: $name) -> Result<$into, Self::Error> {
                Ok($into::try_from(x.0)?)
            }
        }
    };
}

// Implement From for all unsigned integers

implement_try_from!([u1, u2, u3, u4, u5, u6, u7], u8);
implement_from!([u9, u10, u11, u12, u13, u14, u15], u8);
implement_from!([u17, u18, u19, u20, u21, u22, u23, u24], u8);
implement_from!([u25, u26, u27, u28, u29, u30, u31], u8);
implement_from!([u33, u34, u35, u36, u37, u38, u39, u40], u8);
implement_from!([u41, u42, u43, u44, u45, u46, u47, u48], u8);
implement_from!([u49, u50, u51, u52, u53, u54, u55, u56], u8);
implement_from!([u57, u58, u59, u60, u61, u62, u63], u8);

implement_into!([u1, u2, u3, u4, u5, u6, u7], u8);
implement_try_into!([u9, u10, u11, u12, u13, u14, u15], u8);
implement_try_into!([u17, u18, u19, u20, u21, u22, u23, u24], u8);
implement_try_into!([u25, u26, u27, u28, u29, u30, u31], u8);
implement_try_into!([u33, u34, u35, u36, u37, u38, u39, u40], u8);
implement_try_into!([u41, u42, u43, u44, u45, u46, u47, u48], u8);
implement_try_into!([u49, u50, u51, u52, u53, u54, u55, u56], u8);
implement_try_into!([u57, u58, u59, u60, u61, u62, u63], u8);

implement_try_from!([u1, u2, u3, u4, u5, u6, u7], u16);
implement_try_from!([u9, u10, u11, u12, u13, u14, u15], u16);
implement_from!([u17, u18, u19, u20, u21, u22, u23, u24], u16);
implement_from!([u25, u26, u27, u28, u29, u30, u31], u16);
implement_from!([u33, u34, u35, u36, u37, u38, u39, u40], u16);
implement_from!([u41, u42, u43, u44, u45, u46, u47, u48], u16);
implement_from!([u49, u50, u51, u52, u53, u54, u55, u56], u16);
implement_from!([u57, u58, u59, u60, u61, u62, u63], u16);

implement_into!([u1, u2, u3, u4, u5, u6, u7], u16);
implement_into!([u9, u10, u11, u12, u13, u14, u15], u16);
implement_try_into!([u17, u18, u19, u20, u21, u22, u23, u24], u16);
implement_try_into!([u25, u26, u27, u28, u29, u30, u31], u16);
implement_try_into!([u33, u34, u35, u36, u37, u38, u39, u40], u16);
implement_try_into!([u41, u42, u43, u44, u45, u46, u47, u48], u16);
implement_try_into!([u49, u50, u51, u52, u53, u54, u55, u56], u16);
implement_try_into!([u57, u58, u59, u60, u61, u62, u63], u16);

implement_try_from!([u1, u2, u3, u4, u5, u6, u7], u32);
implement_try_from!([u9, u10, u11, u12, u13, u14, u15], u32);
implement_try_from!([u17, u18, u19, u20, u21, u22, u23, u24], u32);
implement_try_from!([u25, u26, u27, u28, u29, u30, u31], u32);
implement_from!([u33, u34, u35, u36, u37, u38, u39, u40], u32);
implement_from!([u41, u42, u43, u44, u45, u46, u47, u48], u32);
implement_from!([u49, u50, u51, u52, u53, u54, u55, u56], u32);
implement_from!([u57, u58, u59, u60, u61, u62, u63], u32);

implement_into!([u1, u2, u3, u4, u5, u6, u7], u32);
implement_into!([u9, u10, u11, u12, u13, u14, u15], u32);
implement_into!([u17, u18, u19, u20, u21, u22, u23, u24], u32);
implement_into!([u25, u26, u27, u28, u29, u30, u31], u32);
implement_try_into!([u33, u34, u35, u36, u37, u38, u39, u40], u32);
implement_try_into!([u41, u42, u43, u44, u45, u46, u47, u48], u32);
implement_try_into!([u49, u50, u51, u52, u53, u54, u55, u56], u32);
implement_try_into!([u57, u58, u59, u60, u61, u62, u63], u32);

implement_try_from!([u1, u2, u3, u4, u5, u6, u7], u64);
implement_try_from!([u9, u10, u11, u12, u13, u14, u15], u64);
implement_try_from!([u17, u18, u19, u20, u21, u22, u23, u24], u64);
implement_try_from!([u25, u26, u27, u28, u29, u30, u31], u64);
implement_try_from!([u33, u34, u35, u36, u37, u38, u39, u40], u64);
implement_try_from!([u41, u42, u43, u44, u45, u46, u47, u48], u64);
implement_try_from!([u49, u50, u51, u52, u53, u54, u55, u56], u64);
implement_try_from!([u57, u58, u59, u60, u61, u62, u63], u64);

implement_into!([u1, u2, u3, u4, u5, u6, u7], u64);
implement_into!([u9, u10, u11, u12, u13, u14, u15], u64);
implement_into!([u17, u18, u19, u20, u21, u22, u23, u24], u64);
implement_into!([u25, u26, u27, u28, u29, u30, u31], u64);
implement_into!([u33, u34, u35, u36, u37, u38, u39, u40], u64);
implement_into!([u41, u42, u43, u44, u45, u46, u47, u48], u64);
implement_into!([u49, u50, u51, u52, u53, u54, u55, u56], u64);
implement_into!([u57, u58, u59, u60, u61, u62, u63], u64);

implement_try_into!([u1, u2, u3, u4, u5, u6, u7], usize);
implement_try_into!([u9, u10, u11, u12, u13, u14, u15], usize);
implement_try_into!([u17, u18, u19, u20, u21, u22, u23, u24], usize);
implement_try_into!([u25, u26, u27, u28, u29, u30, u31], usize);
implement_try_into!([u33, u34, u35, u36, u37, u38, u39, u40], usize);
implement_try_into!([u41, u42, u43, u44, u45, u46, u47, u48], usize);
implement_try_into!([u49, u50, u51, u52, u53, u54, u55, u56], usize);
implement_try_into!([u57, u58, u59, u60, u61, u62, u63], usize);

implement_try_from!(
    u1,
    [
        u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59,
        u60, u61, u62, u63, usize
    ]
);

implement_from!(u2, [u1]);
implement_try_from!(
    u2,
    [
        u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22, u23,
        u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42,
        u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60,
        u61, u62, u63, usize
    ]
);

implement_from!(u3, [u2, u1]);
implement_try_from!(
    u3,
    [
        u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22, u23, u24,
        u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43,
        u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61,
        u62, u63, usize
    ]
);

implement_from!(u4, [u1, u2, u3]);
implement_try_from!(
    u4,
    [
        u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22, u23, u24, u25,
        u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44,
        u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62,
        u63, usize
    ]
);

implement_from!(u5, [u1, u2, u3, u4]);
implement_try_from!(
    u5,
    [
        u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22, u23, u24, u25, u26,
        u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45,
        u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63,
        usize
    ]
);

implement_from!(u6, [u1, u2, u3, u4, u5]);
implement_try_from!(
    u6,
    [
        u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22, u23, u24, u25, u26,
        u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45,
        u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63,
        usize
    ]
);

implement_from!(u7, [u1, u2, u3, u4, u5, u6]);
implement_try_from!(
    u7,
    [
        u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22, u23, u24, u25, u26, u27,
        u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46,
        u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(u9, [u1, u2, u3, u4, u5, u6, u7]);
implement_try_from!(
    u9,
    [
        u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22, u23, u24, u25, u26, u27, u28,
        u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47,
        u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(u10, [u1, u2, u3, u4, u5, u6, u7, u9]);
implement_try_from!(
    u10,
    [
        u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22, u23, u24, u25, u26, u27, u28, u29,
        u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48,
        u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(u11, [u1, u2, u3, u4, u5, u6, u7, u9, u10]);
implement_try_from!(
    u11,
    [
        u12, u13, u14, u15, u17, u18, u19, u20, u21, u22, u23, u24, u25, u26, u27, u28, u29, u30,
        u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49,
        u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(u12, [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11]);
implement_try_from!(
    u12,
    [
        u13, u14, u15, u17, u18, u19, u20, u21, u22, u23, u24, u25, u26, u27, u28, u29, u30, u31,
        u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50,
        u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(u13, [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12]);
implement_try_from!(
    u13,
    [
        u14, u15, u17, u18, u19, u20, u21, u22, u23, u24, u25, u26, u27, u28, u29, u30, u31, u33,
        u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51,
        u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(u14, [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13]);
implement_try_from!(
    u14,
    [
        u15, u17, u18, u19, u20, u21, u22, u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34,
        u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52,
        u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u15,
    [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14]
);
implement_try_from!(
    u15,
    [
        u17, u18, u19, u20, u21, u22, u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35,
        u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53,
        u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u17,
    [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15]
);
implement_try_from!(
    u17,
    [
        u18, u19, u20, u21, u22, u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36,
        u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54,
        u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u18,
    [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17]
);
implement_try_from!(
    u18,
    [
        u19, u20, u21, u22, u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37,
        u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55,
        u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u19,
    [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18]
);
implement_try_from!(
    u19,
    [
        u20, u21, u22, u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38,
        u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56,
        u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u20,
    [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19]
);
implement_try_from!(
    u20,
    [
        u21, u22, u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39,
        u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57,
        u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u21,
    [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20]
);
implement_try_from!(
    u21,
    [
        u22, u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40,
        u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58,
        u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u22,
    [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21]
);
implement_try_from!(
    u22,
    [
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59,
        u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u23,
    [u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22]
);
implement_try_from!(
    u23,
    [
        u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42,
        u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60,
        u61, u62, u63, usize
    ]
);

implement_from!(
    u24,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23
    ]
);
implement_try_from!(
    u24,
    [
        u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43,
        u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61,
        u62, u63, usize
    ]
);

implement_from!(
    u25,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24
    ]
);
implement_try_from!(
    u25,
    [
        u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44,
        u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62,
        u63, usize
    ]
);

implement_from!(
    u26,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25
    ]
);
implement_try_from!(
    u26,
    [
        u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45,
        u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63,
        usize
    ]
);

implement_from!(
    u27,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26
    ]
);
implement_try_from!(
    u27,
    [
        u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46,
        u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u28,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27
    ]
);
implement_try_from!(
    u28,
    [
        u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47,
        u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u29,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28
    ]
);
implement_try_from!(
    u29,
    [
        u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48,
        u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u30,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29
    ]
);
implement_try_from!(
    u30,
    [
        u31, u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49,
        u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u31,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30
    ]
);
implement_try_from!(
    u31,
    [
        u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50,
        u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u33,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31
    ]
);
implement_try_from!(
    u33,
    [
        u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51,
        u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u34,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33
    ]
);
implement_try_from!(
    u34,
    [
        u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52,
        u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u35,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34
    ]
);
implement_try_from!(
    u35,
    [
        u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53,
        u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u36,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35
    ]
);
implement_try_from!(
    u36,
    [
        u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54,
        u55, u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u37,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36
    ]
);
implement_try_from!(
    u37,
    [
        u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55,
        u56, u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u38,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37
    ]
);
implement_try_from!(
    u38,
    [
        u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56,
        u57, u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u39,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38
    ]
);
implement_try_from!(
    u39,
    [
        u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57,
        u58, u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u40,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39
    ]
);
implement_try_from!(
    u40,
    [
        u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58,
        u59, u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u41,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40
    ]
);
implement_try_from!(
    u41,
    [
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59,
        u60, u61, u62, u63, usize
    ]
);

implement_from!(
    u42,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41
    ]
);
implement_try_from!(
    u42,
    [
        u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60,
        u61, u62, u63, usize
    ]
);

implement_from!(
    u43,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42
    ]
);
implement_try_from!(
    u43,
    [
        u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61,
        u62, u63, usize
    ]
);

implement_from!(
    u44,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43
    ]
);
implement_try_from!(
    u44,
    [
        u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62,
        u63, usize
    ]
);

implement_from!(
    u45,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44
    ]
);
implement_try_from!(
    u45,
    [
        u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63,
        usize
    ]
);

implement_from!(
    u46,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45
    ]
);
implement_try_from!(
    u46,
    [u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize]
);

implement_from!(
    u47,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46
    ]
);
implement_try_from!(
    u47,
    [u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize]
);

implement_from!(
    u48,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47
    ]
);
implement_try_from!(
    u48,
    [u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize]
);

implement_from!(
    u49,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48
    ]
);
implement_try_from!(
    u49,
    [u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize]
);

implement_from!(
    u50,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49
    ]
);
implement_try_from!(
    u50,
    [u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize]
);

implement_from!(
    u51,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50
    ]
);
implement_try_from!(
    u51,
    [u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize]
);

implement_from!(
    u52,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51
    ]
);
implement_try_from!(
    u52,
    [u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize]
);

implement_from!(
    u53,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52
    ]
);
implement_try_from!(
    u53,
    [u54, u55, u56, u57, u58, u59, u60, u61, u62, u63, usize]
);

implement_from!(
    u54,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53
    ]
);
implement_try_from!(u54, [u55, u56, u57, u58, u59, u60, u61, u62, u63, usize]);

implement_from!(
    u55,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54
    ]
);
implement_try_from!(u55, [u56, u57, u58, u59, u60, u61, u62, u63, usize]);

implement_from!(
    u56,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55
    ]
);
implement_try_from!(u56, [u57, u58, u59, u60, u61, u62, u63, usize]);

implement_from!(
    u57,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56
    ]
);
implement_try_from!(u57, [u58, u59, u60, u61, u62, u63, usize]);

implement_from!(
    u58,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57
    ]
);
implement_try_from!(u58, [u59, u60, u61, u62, u63, usize]);

implement_from!(
    u59,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58
    ]
);
implement_try_from!(u59, [u60, u61, u62, u63, usize]);

implement_from!(
    u60,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59
    ]
);
implement_try_from!(u60, [u61, u62, u63, usize]);

implement_from!(
    u61,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59,
        u60
    ]
);
implement_try_from!(u61, [u62, u63, usize]);

implement_from!(
    u62,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59,
        u60, u61
    ]
);
implement_from!(
    u63,
    [
        u1, u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22,
        u23, u24, u25, u26, u27, u28, u29, u30, u31, u33, u34, u35, u36, u37, u38, u39, u40, u41,
        u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59,
        u60, u61, u62
    ]
);

// Implement From for all signed integer

implement_try_from!([i2, i3, i4, i5, i6, i7], i8);
implement_from!([i9, i10, i11, i12, i13, i14, i15], i8);
implement_from!([i17, i18, i19, i20, i21, i22, i23, i24], i8);
implement_from!([i25, i26, i27, i28, i29, i30, i31], i8);
implement_from!([i33, i34, i35, i36, i37, i38, i39, i40], i8);
implement_from!([i41, i42, i43, i44, i45, i46, i47, i48], i8);
implement_from!([i49, i50, i51, i52, i53, i54, i55, i56], i8);
implement_from!([i57, i58, i59, i60, i61, i62, i63], i8);

implement_into!([i2, i3, i4, i5, i6, i7], i8);
implement_try_into!([i9, i10, i11, i12, i13, i14, i15], i8);
implement_try_into!([i17, i18, i19, i20, i21, i22, i23, i24], i8);
implement_try_into!([i25, i26, i27, i28, i29, i30, i31], i8);
implement_try_into!([i33, i34, i35, i36, i37, i38, i39, i40], i8);
implement_try_into!([i41, i42, i43, i44, i45, i46, i47, i48], i8);
implement_try_into!([i49, i50, i51, i52, i53, i54, i55, i56], i8);
implement_try_into!([i57, i58, i59, i60, i61, i62, i63], i8);

implement_try_from!([i2, i3, i4, i5, i6, i7], i16);
implement_try_from!([i9, i10, i11, i12, i13, i14, i15], i16);
implement_from!([i17, i18, i19, i20, i21, i22, i23, i24], i16);
implement_from!([i25, i26, i27, i28, i29, i30, i31], i16);
implement_from!([i33, i34, i35, i36, i37, i38, i39, i40], i16);
implement_from!([i41, i42, i43, i44, i45, i46, i47, i48], i16);
implement_from!([i49, i50, i51, i52, i53, i54, i55, i56], i16);
implement_from!([i57, i58, i59, i60, i61, i62, i63], i16);

implement_into!([i2, i3, i4, i5, i6, i7], i16);
implement_into!([i9, i10, i11, i12, i13, i14, i15], i16);
implement_try_into!([i17, i18, i19, i20, i21, i22, i23, i24], i16);
implement_try_into!([i25, i26, i27, i28, i29, i30, i31], i16);
implement_try_into!([i33, i34, i35, i36, i37, i38, i39, i40], i16);
implement_try_into!([i41, i42, i43, i44, i45, i46, i47, i48], i16);
implement_try_into!([i49, i50, i51, i52, i53, i54, i55, i56], i16);
implement_try_into!([i57, i58, i59, i60, i61, i62, i63], i16);

implement_try_from!([i2, i3, i4, i5, i6, i7], i32);
implement_try_from!([i9, i10, i11, i12, i13, i14, i15], i32);
implement_try_from!([i17, i18, i19, i20, i21, i22, i23, i24], i32);
implement_try_from!([i25, i26, i27, i28, i29, i30, i31], i32);
implement_from!([i33, i34, i35, i36, i37, i38, i39, i40], i32);
implement_from!([i41, i42, i43, i44, i45, i46, i47, i48], i32);
implement_from!([i49, i50, i51, i52, i53, i54, i55, i56], i32);
implement_from!([i57, i58, i59, i60, i61, i62, i63], i32);

implement_into!([i2, i3, i4, i5, i6, i7], i32);
implement_into!([i9, i10, i11, i12, i13, i14, i15], i32);
implement_into!([i17, i18, i19, i20, i21, i22, i23, i24], i32);
implement_into!([i25, i26, i27, i28, i29, i30, i31], i32);
implement_try_into!([i33, i34, i35, i36, i37, i38, i39, i40], i32);
implement_try_into!([i41, i42, i43, i44, i45, i46, i47, i48], i32);
implement_try_into!([i49, i50, i51, i52, i53, i54, i55, i56], i32);
implement_try_into!([i57, i58, i59, i60, i61, i62, i63], i32);

implement_try_from!([i2, i3, i4, i5, i6, i7], i64);
implement_try_from!([i9, i10, i11, i12, i13, i14, i15], i64);
implement_try_from!([i17, i18, i19, i20, i21, i22, i23, i24], i64);
implement_try_from!([i25, i26, i27, i28, i29, i30, i31], i64);
implement_try_from!([i33, i34, i35, i36, i37, i38, i39, i40], i64);
implement_try_from!([i41, i42, i43, i44, i45, i46, i47, i48], i64);
implement_try_from!([i49, i50, i51, i52, i53, i54, i55, i56], i64);
implement_try_from!([i57, i58, i59, i60, i61, i62, i63], i64);

implement_into!([i2, i3, i4, i5, i6, i7], i64);
implement_into!([i9, i10, i11, i12, i13, i14, i15], i64);
implement_into!([i17, i18, i19, i20, i21, i22, i23, i24], i64);
implement_into!([i25, i26, i27, i28, i29, i30, i31], i64);
implement_into!([i33, i34, i35, i36, i37, i38, i39, i40], i64);
implement_into!([i41, i42, i43, i44, i45, i46, i47, i48], i64);
implement_into!([i49, i50, i51, i52, i53, i54, i55, i56], i64);
implement_into!([i57, i58, i59, i60, i61, i62, i63], i64);

implement_try_from!(
    i2,
    [
        i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23,
        i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42,
        i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60,
        i61, i62, i63
    ]
);

implement_from!(i3, [i2]);
implement_try_from!(
    i3,
    [
        i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23, i24,
        i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43,
        i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61,
        i62, i63
    ]
);

implement_from!(i4, [i2, i3]);
implement_try_from!(
    i4,
    [
        i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23, i24, i25,
        i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44,
        i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62,
        i63
    ]
);

implement_from!(i5, [i2, i3, i4]);
implement_try_from!(
    i5,
    [
        i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23, i24, i25, i26,
        i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45,
        i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(i6, [i2, i3, i4, i5]);
implement_try_from!(
    i6,
    [
        i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23, i24, i25, i26,
        i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45,
        i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(i7, [i2, i3, i4, i5, i6]);
implement_try_from!(
    i7,
    [
        i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23, i24, i25, i26, i27,
        i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46,
        i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(i9, [i2, i3, i4, i5, i6, i7]);
implement_try_from!(
    i9,
    [
        i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23, i24, i25, i26, i27, i28,
        i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47,
        i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(i10, [i2, i3, i4, i5, i6, i7, i9]);
implement_try_from!(
    i10,
    [
        i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23, i24, i25, i26, i27, i28, i29,
        i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48,
        i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(i11, [i2, i3, i4, i5, i6, i7, i9, i10]);
implement_try_from!(
    i11,
    [
        i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23, i24, i25, i26, i27, i28, i29, i30,
        i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49,
        i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(i12, [i2, i3, i4, i5, i6, i7, i9, i10, i11]);
implement_try_from!(
    i12,
    [
        i13, i14, i15, i17, i18, i19, i20, i21, i22, i23, i24, i25, i26, i27, i28, i29, i30, i31,
        i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50,
        i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(i13, [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12]);
implement_try_from!(
    i13,
    [
        i14, i15, i17, i18, i19, i20, i21, i22, i23, i24, i25, i26, i27, i28, i29, i30, i31, i33,
        i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51,
        i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(i14, [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13]);
implement_try_from!(
    i14,
    [
        i15, i17, i18, i19, i20, i21, i22, i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34,
        i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52,
        i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(i15, [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14]);
implement_try_from!(
    i15,
    [
        i17, i18, i19, i20, i21, i22, i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35,
        i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53,
        i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i17,
    [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15]
);
implement_try_from!(
    i17,
    [
        i18, i19, i20, i21, i22, i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36,
        i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54,
        i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i18,
    [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17]
);
implement_try_from!(
    i18,
    [
        i19, i20, i21, i22, i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37,
        i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55,
        i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i19,
    [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18]
);
implement_try_from!(
    i19,
    [
        i20, i21, i22, i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38,
        i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56,
        i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i20,
    [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19]
);
implement_try_from!(
    i20,
    [
        i21, i22, i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39,
        i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57,
        i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i21,
    [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20]
);
implement_try_from!(
    i21,
    [
        i22, i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40,
        i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58,
        i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i22,
    [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21]
);
implement_try_from!(
    i22,
    [
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59,
        i60, i61, i62, i63
    ]
);

implement_from!(
    i23,
    [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22]
);
implement_try_from!(
    i23,
    [
        i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42,
        i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60,
        i61, i62, i63
    ]
);

implement_from!(
    i24,
    [i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23]
);
implement_try_from!(
    i24,
    [
        i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43,
        i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61,
        i62, i63
    ]
);

implement_from!(
    i25,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24
    ]
);
implement_try_from!(
    i25,
    [
        i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44,
        i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62,
        i63
    ]
);

implement_from!(
    i26,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25
    ]
);
implement_try_from!(
    i26,
    [
        i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45,
        i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i27,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26
    ]
);
implement_try_from!(
    i27,
    [
        i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46,
        i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i28,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27
    ]
);
implement_try_from!(
    i28,
    [
        i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47,
        i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i29,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28
    ]
);
implement_try_from!(
    i29,
    [
        i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48,
        i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i30,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29
    ]
);
implement_try_from!(
    i30,
    [
        i31, i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49,
        i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i31,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30
    ]
);
implement_try_from!(
    i31,
    [
        i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50,
        i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i33,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31
    ]
);
implement_try_from!(
    i33,
    [
        i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51,
        i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i34,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33
    ]
);
implement_try_from!(
    i34,
    [
        i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52,
        i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i35,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34
    ]
);
implement_try_from!(
    i35,
    [
        i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53,
        i54, i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i36,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35
    ]
);
implement_try_from!(
    i36,
    [
        i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54,
        i55, i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i37,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36
    ]
);
implement_try_from!(
    i37,
    [
        i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55,
        i56, i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i38,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37
    ]
);
implement_try_from!(
    i38,
    [
        i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56,
        i57, i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i39,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38
    ]
);
implement_try_from!(
    i39,
    [
        i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57,
        i58, i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i40,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39
    ]
);
implement_try_from!(
    i40,
    [
        i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58,
        i59, i60, i61, i62, i63
    ]
);

implement_from!(
    i41,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40
    ]
);
implement_try_from!(
    i41,
    [
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59,
        i60, i61, i62, i63
    ]
);

implement_from!(
    i42,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41
    ]
);
implement_try_from!(
    i42,
    [
        i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60,
        i61, i62, i63
    ]
);

implement_from!(
    i43,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42
    ]
);
implement_try_from!(
    i43,
    [
        i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61,
        i62, i63
    ]
);

implement_from!(
    i44,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43
    ]
);
implement_try_from!(
    i44,
    [
        i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62,
        i63
    ]
);

implement_from!(
    i45,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44
    ]
);
implement_try_from!(
    i45,
    [i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63]
);

implement_from!(
    i46,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45
    ]
);
implement_try_from!(
    i46,
    [i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63]
);

implement_from!(
    i47,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46
    ]
);
implement_try_from!(
    i47,
    [i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63]
);

implement_from!(
    i48,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47
    ]
);
implement_try_from!(
    i48,
    [i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63]
);

implement_from!(
    i49,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48
    ]
);
implement_try_from!(
    i49,
    [i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63]
);

implement_from!(
    i50,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49
    ]
);
implement_try_from!(
    i50,
    [i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63]
);

implement_from!(
    i51,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50
    ]
);
implement_try_from!(
    i51,
    [i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63]
);

implement_from!(
    i52,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51
    ]
);
implement_try_from!(i52, [i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63]);

implement_from!(
    i53,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52
    ]
);
implement_try_from!(i53, [i54, i55, i56, i57, i58, i59, i60, i61, i62, i63]);

implement_from!(
    i54,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53
    ]
);
implement_try_from!(i54, [i55, i56, i57, i58, i59, i60, i61, i62, i63]);

implement_from!(
    i55,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54
    ]
);
implement_try_from!(i55, [i56, i57, i58, i59, i60, i61, i62, i63]);

implement_from!(
    i56,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55
    ]
);
implement_try_from!(i56, [i57, i58, i59, i60, i61, i62, i63]);

implement_from!(
    i57,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56
    ]
);
implement_try_from!(i57, [i58, i59, i60, i61, i62, i63]);

implement_from!(
    i58,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57
    ]
);
implement_try_from!(i58, [i59, i60, i61, i62, i63]);

implement_from!(
    i59,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58
    ]
);
implement_try_from!(i59, [i60, i61, i62, i63]);

implement_from!(
    i60,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59
    ]
);
implement_try_from!(i60, [i61, i62, i63]);

implement_from!(
    i61,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59,
        i60
    ]
);
implement_try_from!(i61, [i62, i63]);

implement_from!(
    i62,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59,
        i60, i61
    ]
);
implement_from!(
    i63,
    [
        i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22,
        i23, i24, i25, i26, i27, i28, i29, i30, i31, i33, i34, i35, i36, i37, i38, i39, i40, i41,
        i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59,
        i60, i61, i62
    ]
);

impl From<bool> for u1 {
    fn from(b: bool) -> Self {
        match b {
            true => u1(1),
            false => u1(0),
        }
    }
}

impl From<u1> for bool {
    fn from(u1(x): u1) -> Self {
        match x {
            0 => false,
            1 => true,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;

    #[test]
    fn test_infallible_conversion_unsigned() {
        assert_eq!(u16::from(u9(12)), 12u16);
        assert_eq!(u32::from(u9(12)), 12u32);

        assert_eq!(u9(127), 127u8.into());

        assert_eq!(u7::from(u6(65)), u7(65));
    }

    #[test]
    fn test_infallible_conversion_signed() {
        assert_eq!(i16::from(i9(12)), 12i16);
        assert_eq!(i32::from(i9(12)), 12i32);

        assert_eq!(i16::from(i9(-12)), -12i16);
        assert_eq!(i32::from(i9(-12)), -12i32);

        assert_eq!(i9(127), 127i8.into());

        assert_eq!(i7::from(i6(65)), i7(65));
        assert_eq!(i7::from(i6(-65)), i7(-65));
    }

    #[test]
    fn test_fallible_conversion_unsigned() {
        assert_eq!(u16::try_from(u9(12)), Ok(12u16));
        assert_eq!(u32::try_from(u9(12)), Ok(12u32));

        assert_eq!(127u8.try_into(), Ok(u9(127)));

        assert_eq!(u7::try_from(u6(65)), Ok(u7(65)));

        assert!(u16::try_from(u19(0x1_ffff)).is_err());
        assert!(u32::try_from(u39(0x1_fffff_ffff)).is_err());

        assert!(u6::try_from(u7(127)).is_err());

        assert_eq!(u2::try_from(1usize), Ok(u2(1)));
        assert!(u2::try_from(4usize).is_err());
        assert_eq!(u2(1).try_into(), Ok(1usize));

        // Make sure that uX types behave the same as standard types with regards to usize
        // conversion.
        assert_eq!(
            usize::try_from(0x1_FFFF_FFFFu64).is_err(),
            usize::try_from(u33::MAX).is_err()
        );
    }

    #[test]
    fn test_fallible_conversion_signed() {
        assert_eq!(i16::try_from(i9(12)), Ok(12i16));
        assert_eq!(i32::try_from(i9(12)), Ok(12i32));

        assert_eq!(i16::try_from(i9(-12)), Ok(-12i16));
        assert_eq!(i32::try_from(i9(-12)), Ok(-12i32));

        assert_eq!(127i8.try_into(), Ok(i9(127)));

        assert_eq!(i7::try_from(i6(65)), Ok(i7(65)));
        assert_eq!(i7::try_from(i6(-65)), Ok(i7(-65)));

        assert!(i16::try_from(i19(0xffff)).is_err());
        assert!(i32::try_from(i39(0xffff_ffff)).is_err());

        assert!(i16::try_from(i19(-0xffff)).is_err());
        assert!(i32::try_from(i39(-0xffff_ffff)).is_err());

        assert!(i6::try_from(i7(64)).is_err());
        assert!(i6::try_from(i7(-64)).is_err());
    }

    #[test]
    #[cfg(feature = "std")]
    fn error_trait() {
        assert_eq!(
            (&TryFromIntError(()) as &dyn std::error::Error).to_string(),
            "out of range integral type conversion attempted"
        );
    }
}
