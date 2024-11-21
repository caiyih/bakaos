use core::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Shl, ShlAssign,
    Shr, ShrAssign, Sub, SubAssign,
};

pub trait IUsizeAlias: Copy + Clone + PartialEq + PartialOrd + Eq + Ord {
    fn as_usize(&self) -> usize;

    fn from_usize(value: usize) -> Self;
}

pub trait IArithOps:
    IUsizeAlias
    + Add<usize>
    + Add<Self>
    + Sub<usize>
    + Sub<Self>
    + AddAssign<usize>
    + AddAssign<Self>
    + SubAssign<usize>
    + SubAssign<Self>
{
}

#[macro_export]
macro_rules! impl_arith_with_usize {
    ($type:ty) => {
        impl core::ops::Add<usize> for $type {
            type Output = Self;
            fn add(self, rhs: usize) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self) + rhs,
                )
            }
        }

        impl core::ops::Sub<usize> for $type {
            type Output = Self;
            fn sub(self, rhs: usize) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self) - rhs,
                )
            }
        }

        impl core::ops::AddAssign<usize> for $type {
            fn add_assign(&mut self, rhs: usize) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self) + rhs,
                );
            }
        }

        impl core::ops::SubAssign<usize> for $type {
            fn sub_assign(&mut self, rhs: usize) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self) - rhs,
                );
            }
        }
    };
}

#[macro_export]
macro_rules! impl_arith_with {
    ($type_this:ty, $type_them:ty) => {
        impl core::ops::Add<$type_them> for $type_this {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self)
                        + abstractions::IUsizeAlias::as_usize(&rhs),
                )
            }
        }

        impl core::ops::Sub<$type_them> for $type_this {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self)
                        - abstractions::IUsizeAlias::as_usize(&rhs),
                )
            }
        }

        impl core::ops::AddAssign<$type_them> for $type_this {
            fn add_assign(&mut self, rhs: Self) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self)
                        + abstractions::IUsizeAlias::as_usize(&rhs),
                );
            }
        }

        impl core::ops::SubAssign<$type_them> for $type_this {
            fn sub_assign(&mut self, rhs: Self) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self)
                        - abstractions::IUsizeAlias::as_usize(&rhs),
                );
            }
        }
    };
}

#[macro_export]
macro_rules! impl_arith_with_self {
    ($type:ty) => {
        abstractions::impl_arith_with!($type, Self);
    };
}

#[macro_export]
macro_rules! impl_arith_ops {
    ($type:ty) => {
        impl abstractions::IArithOps for $type {}

        abstractions::impl_arith_with_usize!($type);
        abstractions::impl_arith_with_self!($type);
    };
}

pub trait IBitwiseOps:
    IUsizeAlias
    + BitAnd<usize>
    + BitAnd<Self>
    + BitOr<usize>
    + BitOr<Self>
    + BitXor<usize>
    + BitXor<Self>
    + BitAndAssign<usize>
    + BitAndAssign<Self>
    + BitOrAssign<usize>
    + BitOrAssign<Self>
    + BitXorAssign<usize>
    + BitXorAssign<Self>
    + Shl<usize>
    + Shl<Self>
    + Shr<usize>
    + Shr<Self>
    + ShlAssign<usize>
    + ShlAssign<Self>
    + ShrAssign<usize>
    + ShrAssign<Self>
{
}

#[macro_export]
macro_rules! impl_bitwise_ops_with_usize {
    ($type:ty) => {
        impl core::ops::BitAnd<usize> for $type {
            type Output = $type;
            fn bitand(self, rhs: usize) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self) & rhs,
                )
            }
        }

        impl core::ops::BitOr<usize> for $type {
            type Output = $type;
            fn bitor(self, rhs: usize) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self) | rhs,
                )
            }
        }

        impl core::ops::BitXor<usize> for $type {
            type Output = $type;
            fn bitxor(self, rhs: usize) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self) ^ rhs,
                )
            }
        }

        impl core::ops::BitAndAssign<usize> for $type {
            fn bitand_assign(&mut self, rhs: usize) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self) & rhs,
                );
            }
        }

        impl core::ops::BitOrAssign<usize> for $type {
            fn bitor_assign(&mut self, rhs: usize) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self) | rhs,
                );
            }
        }

        impl core::ops::BitXorAssign<usize> for $type {
            fn bitxor_assign(&mut self, rhs: usize) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self) ^ rhs,
                );
            }
        }

        impl core::ops::Shl<usize> for $type {
            type Output = $type;
            fn shl(self, rhs: usize) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self) << rhs,
                )
            }
        }

        impl core::ops::Shr<usize> for $type {
            type Output = $type;
            fn shr(self, rhs: usize) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self) >> rhs,
                )
            }
        }

        impl core::ops::ShlAssign<usize> for $type {
            fn shl_assign(&mut self, rhs: usize) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self) << rhs,
                );
            }
        }

        impl core::ops::ShrAssign<usize> for $type {
            fn shr_assign(&mut self, rhs: usize) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self) >> rhs,
                );
            }
        }
    };
}

#[macro_export]
macro_rules! impl_bitwise_ops_with {
    ($type_this:ty, $type_them:ty) => {
        impl core::ops::BitAnd<$type_them> for $type_this {
            type Output = $type_them;
            fn bitand(self, rhs: $type_them) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self)
                        & abstractions::IUsizeAlias::as_usize(&rhs),
                )
            }
        }

        impl core::ops::BitOr<$type_them> for $type_this {
            type Output = $type_them;
            fn bitor(self, rhs: $type_them) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self)
                        | abstractions::IUsizeAlias::as_usize(&rhs),
                )
            }
        }

        impl core::ops::BitXor<$type_them> for $type_this {
            type Output = $type_them;
            fn bitxor(self, rhs: $type_them) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self)
                        ^ abstractions::IUsizeAlias::as_usize(&rhs),
                )
            }
        }

        impl core::ops::BitAndAssign<$type_them> for $type_this {
            fn bitand_assign(&mut self, rhs: $type_them) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self)
                        & abstractions::IUsizeAlias::as_usize(&rhs),
                );
            }
        }

        impl core::ops::BitOrAssign<$type_them> for $type_this {
            fn bitor_assign(&mut self, rhs: $type_them) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self)
                        | abstractions::IUsizeAlias::as_usize(&rhs),
                );
            }
        }

        impl core::ops::BitXorAssign<$type_them> for $type_this {
            fn bitxor_assign(&mut self, rhs: $type_them) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self)
                        ^ abstractions::IUsizeAlias::as_usize(&rhs),
                );
            }
        }

        impl core::ops::Shl<$type_them> for $type_this {
            type Output = $type_them;
            fn shl(self, rhs: $type_them) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self)
                        << abstractions::IUsizeAlias::as_usize(&rhs),
                )
            }
        }

        impl core::ops::Shr<$type_them> for $type_this {
            type Output = $type_them;
            fn shr(self, rhs: $type_them) -> Self::Output {
                abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(&self)
                        >> abstractions::IUsizeAlias::as_usize(&rhs),
                )
            }
        }

        impl core::ops::ShlAssign<$type_them> for $type_this {
            fn shl_assign(&mut self, rhs: $type_them) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self)
                        << abstractions::IUsizeAlias::as_usize(&rhs),
                );
            }
        }

        impl core::ops::ShrAssign<$type_them> for $type_this {
            fn shr_assign(&mut self, rhs: $type_them) {
                *self = abstractions::IUsizeAlias::from_usize(
                    abstractions::IUsizeAlias::as_usize(self)
                        >> abstractions::IUsizeAlias::as_usize(&rhs),
                );
            }
        }
    };
}

#[macro_export]
macro_rules! impl_bitwise_ops_with_self {
    ($type:ty) => {
        abstractions::impl_bitwise_ops_with!($type, Self);
    };
}

#[macro_export]
macro_rules! impl_bitwise_ops {
    ($type:ty) => {
        impl abstractions::IBitwiseOps for $type {}

        abstractions::impl_bitwise_ops_with_usize!($type);
        abstractions::impl_bitwise_ops_with_self!($type);
    };
}

#[macro_export]
macro_rules! impl_usize_display {
    ($type:ty) => {
        impl core::fmt::Display for $type {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({:#x})", stringify!($type), self.as_usize())
            }
        }
    };
}
