use std::{
    fmt::{Debug, Display, LowerHex},
    ops::{
        Add, AddAssign, BitAnd, BitOr, BitXor, Div, DivAssign, Mul, MulAssign, Not, Rem, RemAssign,
        Shl, Shr, Sub, SubAssign,
    },
};

#[cfg(feature = "fuzzing")]
use arbitrary::Arbitrary;
use bnum::{cast::CastFrom, types::U512};
use ethnum::U256;
use evmc_vm::{Address, Uint256};
use zerocopy::transmute;

/// This represents a 256-bit integer in native endian.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
#[repr(align(16))] // 16 byte alignment is faster than 1, 8 or 32 byte alignment on x86-64.
pub struct u256(U256);

#[cfg(feature = "fuzzing")]
impl<'a> Arbitrary<'a> for u256 {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self(U256(Arbitrary::arbitrary(u)?)))
    }
}

impl LowerHex for u256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let digits: [u64; 4] = transmute!(self.0.0);
        if f.alternate() {
            write!(f, "0x")?;
        }
        write!(
            f,
            "{:016x}_{:016x}_{:016x}_{:016x}",
            digits[3], digits[2], digits[1], digits[0]
        )
    }
}

impl Display for u256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uint256> for u256 {
    fn from(value: Uint256) -> Self {
        Self(U256(transmute!(value.bytes)).to_be())
    }
}

impl From<u256> for Uint256 {
    fn from(value: u256) -> Self {
        Uint256 {
            bytes: transmute!(value.0.to_be().0),
        }
    }
}

impl From<bool> for u256 {
    fn from(value: bool) -> Self {
        Self(U256::from(value))
    }
}

impl From<u8> for u256 {
    fn from(value: u8) -> Self {
        Self(U256::from(value))
    }
}

impl From<u64> for u256 {
    fn from(value: u64) -> Self {
        Self(U256::from(value))
    }
}

impl From<usize> for u256 {
    fn from(value: usize) -> Self {
        Self(U256::from(value as u64))
    }
}

impl From<Address> for u256 {
    fn from(value: Address) -> Self {
        let mut bytes = [0; 32];
        bytes[32 - 20..].copy_from_slice(&value.bytes);
        Self::from_be_bytes(bytes)
    }
}

impl From<&Address> for u256 {
    fn from(value: &Address) -> Self {
        let mut bytes = [0; 32];
        bytes[32 - 20..].copy_from_slice(&value.bytes);
        Self::from_be_bytes(bytes)
    }
}

impl From<u256> for Address {
    fn from(value: u256) -> Self {
        let bytes: [u8; 32] = transmute!(value.0.to_be().0);
        let mut addr = Address { bytes: [0; 20] };
        addr.bytes.copy_from_slice(&bytes[32 - 20..]);
        addr
    }
}

#[derive(Debug, PartialEq)]
pub struct U64Overflow;

impl TryFrom<u256> for u64 {
    type Error = U64Overflow;

    fn try_from(value: u256) -> Result<Self, Self::Error> {
        match value.into_u64_with_overflow() {
            (_, true) => Err(U64Overflow),
            (value, false) => Ok(value),
        }
    }
}

impl Add for u256 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_add(rhs.0))
    }
}

impl AddAssign for u256 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for u256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

impl SubAssign for u256 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for u256 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_mul(rhs.0))
    }
}

impl MulAssign for u256 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Div for u256 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        if rhs == u256::ZERO {
            return u256::ZERO;
        }
        Self(self.0.wrapping_div(rhs.0))
    }
}

impl DivAssign for u256 {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Rem for u256 {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        if rhs == u256::ZERO {
            return u256::ZERO;
        }
        Self(self.0.wrapping_rem(rhs.0))
    }
}

impl RemAssign for u256 {
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

impl PartialEq for u256 {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for u256 {}

impl PartialOrd for u256 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for u256 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl BitAnd for u256 {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0.bitand(rhs.0))
    }
}

impl BitOr for u256 {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0.bitor(rhs.0))
    }
}

impl BitXor for u256 {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0.bitxor(rhs.0))
    }
}

impl Not for u256 {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0.not())
    }
}

impl Shl for u256 {
    type Output = Self;

    fn shl(self, rhs: Self) -> Self::Output {
        // rhs > 255
        let rhs = rhs.to_le_bytes();
        if rhs[1..] != [0; 31] {
            return u256::ZERO;
        }
        let shift = rhs[0] as u32;
        Self(self.0.wrapping_shl(shift))
    }
}

impl Shl<usize> for u256 {
    type Output = Self;

    fn shl(self, rhs: usize) -> Self::Output {
        Self(self.0.wrapping_shl(rhs as u32))
    }
}

impl Shr for u256 {
    type Output = Self;

    fn shr(self, rhs: Self) -> Self::Output {
        // rhs > 255
        let rhs = rhs.to_le_bytes();
        if rhs[1..] != [0; 31] {
            return u256::ZERO;
        }
        let shift = rhs[0] as u32;
        Self(self.0.wrapping_shr(shift))
    }
}

impl u256 {
    pub const ZERO: Self = Self(U256::ZERO);
    pub const ONE: Self = Self(U256::ONE);
    pub const MAX: Self = Self(U256::MAX);

    pub fn into_u64_with_overflow(self) -> (u64, bool) {
        let digits: [u64; 4] = transmute!(self.0.0);
        let overflow = digits[1..] != [0; 3];
        (digits[0], overflow)
    }

    pub fn into_u64_saturating(self) -> u64 {
        let digits: [u64; 4] = transmute!(self.0.0);
        if digits[1..] != [0; 3] {
            u64::MAX
        } else {
            digits[0]
        }
    }

    pub fn sdiv(self, rhs: Self) -> Self {
        if rhs == u256::ZERO {
            return u256::ZERO;
        }

        Self(self.0.as_i256().wrapping_div(rhs.0.as_i256()).as_u256())
    }

    pub fn srem(self, rhs: Self) -> Self {
        if rhs == u256::ZERO {
            return u256::ZERO;
        }
        Self(self.0.as_i256().wrapping_rem(rhs.0.as_i256()).as_u256())
    }

    // ethnum has no support for addmod and mulmod yet (see https://github.com/nlordell/ethnum-rs/issues/10)
    pub fn addmod(s1: Self, s2: Self, m: Self) -> Self {
        if m == u256::ZERO {
            return u256::ZERO;
        }
        let s1 = bnum::types::U256::from_digits(transmute!(s1.0.0));
        let s1 = U512::cast_from(s1);
        let s2 = bnum::types::U256::from_digits(transmute!(s2.0.0));
        let s2 = U512::cast_from(s2);
        let m = bnum::types::U256::from_digits(transmute!(m.0.0));
        let m = U512::cast_from(m);

        Self(U256(transmute!(
            *bnum::types::U256::cast_from((s1 + s2).rem(m)).digits()
        )))
    }

    // ethnum has no support for addmod and mulmod yet (see https://github.com/nlordell/ethnum-rs/issues/10)
    pub fn mulmod(s1: Self, s2: Self, m: Self) -> Self {
        if m == u256::ZERO {
            return u256::ZERO;
        }
        let s1 = bnum::types::U256::from_digits(transmute!(s1.0.0));
        let s1 = U512::cast_from(s1);
        let s2 = bnum::types::U256::from_digits(transmute!(s2.0.0));
        let s2 = U512::cast_from(s2);
        let m = bnum::types::U256::from_digits(transmute!(m.0.0));
        let m = U512::cast_from(m);

        Self(U256(transmute!(
            *bnum::types::U256::cast_from((s1 * s2).rem(m)).digits()
        )))
    }

    pub fn pow(self, exp: Self) -> Self {
        let mut exp = exp.0;
        let mut base = self.0;
        let mut acc = U256::ONE;

        while exp > U256::ONE {
            if (exp & U256::ONE) == U256::ONE {
                acc = acc.wrapping_mul(base);
            }
            exp /= U256::from(2u64);
            base = base.wrapping_mul(base);
        }

        if exp == U256::ONE {
            acc = acc.wrapping_mul(base);
        }

        Self(acc)
    }

    pub fn signextend(self, rhs: Self) -> Self {
        let (lhs, lhs_overflow) = self.into_u64_with_overflow();
        let lhs = lhs as usize;
        if lhs_overflow || lhs > 31 {
            return rhs;
        }

        let byte = 31 - lhs; // lhs <= 31 so this does not underflow
        let negative = (rhs.to_le_bytes()[lhs] & 0x80) > 0;

        let res = if negative {
            rhs.0 | (U256::MAX << ((32 - byte) * 8))
        } else {
            rhs.0 & (U256::MAX >> (byte * 8))
        };

        Self(res)
    }

    pub fn slt(&self, rhs: &Self) -> bool {
        let lhs = self.0.as_i256();
        let rhs = rhs.0.as_i256();
        lhs < rhs
    }

    pub fn sgt(&self, rhs: &Self) -> bool {
        let lhs = self.0.as_i256();
        let rhs = rhs.0.as_i256();
        lhs > rhs
    }

    pub fn byte(&self, index: Self) -> Self {
        if index >= 32u8.into() {
            return u256::ZERO;
        }
        let idx = index.to_le_bytes()[0];
        self.to_le_bytes()[31 - idx as usize].into()
    }

    pub fn sar(self, rhs: Self) -> Self {
        let lhs = self.0.as_i256();
        let rhs = rhs.to_le_bytes();
        // rhs > 255
        if rhs[1..] != [0; 31] {
            if lhs.is_negative() {
                return u256::MAX;
            } else {
                return u256::ZERO;
            }
        }
        let shift = rhs[0] as u32;
        let mut shr = self.0.wrapping_shr(shift);
        if lhs.is_negative() {
            shr |= U256::MAX.wrapping_shl(255 - shift);
        }
        Self(shr)
    }

    pub fn bits(&self) -> u32 {
        256 - self.0.leading_zeros()
    }

    pub fn from_le_bytes(bytes: [u8; 32]) -> Self {
        Self(U256::from_le_bytes(bytes))
    }

    pub fn from_be_bytes(bytes: [u8; 32]) -> Self {
        Self(U256::from_be_bytes(bytes))
    }

    pub fn least_significant_byte(&self) -> u8 {
        self.0.0[0] as u8
    }

    pub fn to_le_bytes(self) -> [u8; 32] {
        self.0.to_le_bytes()
    }
}

#[cfg(test)]
mod tests {
    use evmc_vm::Address;

    use crate::types::amount::{U64Overflow, u256};

    #[test]
    fn display() {
        let x = [
            (
                u256::from(0u8),
                [
                    "0",
                    "0000000000000000_0000000000000000_0000000000000000_0000000000000000",
                    "0x0000000000000000_0000000000000000_0000000000000000_0000000000000000",
                ],
            ),
            (
                u256::from(0xfeu8),
                [
                    "254",
                    "0000000000000000_0000000000000000_0000000000000000_00000000000000fe",
                    "0x0000000000000000_0000000000000000_0000000000000000_00000000000000fe",
                ],
            ),
            (
                u256::from(0xfeu8) << u256::from(8 * 31u8),
                [
                    "114887463540149662646824336688307533573166312910440247132899321632851308314624",
                    "fe00000000000000_0000000000000000_0000000000000000_0000000000000000",
                    "0xfe00000000000000_0000000000000000_0000000000000000_0000000000000000",
                ],
            ),
        ];
        for (value, fmt_strings) in x {
            assert_eq!(format!("{value}",), fmt_strings[0]);
            assert_eq!(format!("{value:x}",), fmt_strings[1]);
            assert_eq!(format!("{value:#x}",), fmt_strings[2]);
        }
    }

    #[test]
    fn conversions() {
        assert_eq!(u256::from(false), u256::ZERO);
        assert_eq!(u256::from(true), u256::ONE);

        assert_eq!(u256::from(0u8), u256::ZERO);
        assert_eq!(u256::from(1u8), u256::ONE);

        assert_eq!(u256::from(0u64), u256::ZERO);
        assert_eq!(u256::from(1u64), u256::ONE);
        for num in [0, 1, u64::MAX - 1, u64::MAX] {
            assert_eq!(u256::from(num).try_into(), Ok(num));
        }
        for num in [0, 1, u64::MAX - 1, u64::MAX] {
            assert_eq!(u256::from(num).into_u64_with_overflow(), (num, false));
        }
        for num in [0, 1, u64::MAX - 1, u64::MAX] {
            assert_eq!(u256::from(num).into_u64_saturating(), num);
        }
        assert_eq!(u256::MAX.try_into(), Result::<u64, _>::Err(U64Overflow));
        assert_eq!(u256::MAX.into_u64_with_overflow(), (u64::MAX, true));
        assert_eq!(u256::MAX.into_u64_saturating(), u64::MAX);

        assert_eq!(
            Address::from(u256::ONE),
            Address {
                bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
            }
        );
        assert_eq!(
            u256::from(Address {
                bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
            }),
            u256::ONE
        );
    }
}
