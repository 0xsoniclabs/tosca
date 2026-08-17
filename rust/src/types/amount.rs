#[cfg(all(feature = "simd", target_endian = "little"))]
use std::simd::u8x32;
#[cfg(feature = "simd")]
use std::simd::{ToBytes, u64x4};
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

/// This represents a 256-bit integer in native endian.
#[expect(non_camel_case_types)]
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
        let (hi, lo) = self.0.into_words();
        if f.alternate() {
            write!(f, "0x")?;
        }
        write!(
            f,
            "{:016x}_{:016x}_{:016x}_{:016x}",
            (hi >> 64) as u64,
            hi as u64,
            (lo >> 64) as u64,
            lo as u64
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
        Self::from_be_bytes(value.bytes)
    }
}

impl From<u256> for Uint256 {
    fn from(value: u256) -> Self {
        Uint256 {
            bytes: value.to_be_bytes(),
        }
    }
}

impl From<bool> for u256 {
    fn from(value: bool) -> Self {
        Self::from(u64::from(value))
    }
}

impl From<u8> for u256 {
    fn from(value: u8) -> Self {
        Self::from(u64::from(value))
    }
}

impl From<u32> for u256 {
    fn from(value: u32) -> Self {
        Self::from(u64::from(value))
    }
}

impl From<u64> for u256 {
    fn from(value: u64) -> Self {
        std::cfg_select! {
            // With `simd` the value is zero-extended in a vector register rather than with scalar
            // stores that would straddle the slot, so that storing it takes a single store the
            // consuming instruction can forward from. Where it feeds arithmetic instead, the
            // vector never materializes.
            feature = "simd" => {
                let lanes = std::cfg_select! {
                    target_endian = "little" => [value, 0, 0, 0],
                    _ => [0, 0, 0, value],
                };
                Self(U256::from_ne_bytes(
                    u64x4::from_array(lanes).to_ne_bytes().to_array(),
                ))
            }
            _ => Self(U256::from(value)),
        }
    }
}

impl From<usize> for u256 {
    fn from(value: usize) -> Self {
        Self::from(value as u64)
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
        let bytes = value.to_be_bytes();
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
            (_, true) => {
                std::hint::cold_path();
                Err(U64Overflow)
            }
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
        std::cfg_select! {
            // Operating on lanes writes the result slot with a single store, see [`u256::lanes`].
            feature = "simd" => Self(U256::from_ne_bytes(
                (self.lanes() & rhs.lanes()).to_ne_bytes().to_array(),
            )),
            _ => Self(self.0.bitand(rhs.0)),
        }
    }
}

impl BitOr for u256 {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        std::cfg_select! {
            // Operating on lanes writes the result slot with a single store, see [`u256::lanes`].
            feature = "simd" => Self(U256::from_ne_bytes(
                (self.lanes() | rhs.lanes()).to_ne_bytes().to_array(),
            )),
            _ => Self(self.0.bitor(rhs.0)),
        }
    }
}

impl BitXor for u256 {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        std::cfg_select! {
            // Operating on lanes writes the result slot with a single store, see [`u256::lanes`].
            feature = "simd" => Self(U256::from_ne_bytes(
                (self.lanes() ^ rhs.lanes()).to_ne_bytes().to_array(),
            )),
            _ => Self(self.0.bitxor(rhs.0)),
        }
    }
}

impl Not for u256 {
    type Output = Self;

    fn not(self) -> Self::Output {
        std::cfg_select! {
            // Operating on lanes writes the result slot with a single store, see [`u256::lanes`].
            feature = "simd" => Self(U256::from_ne_bytes(
                (!self.lanes()).to_ne_bytes().to_array(),
            )),
            _ => Self(self.0.not()),
        }
    }
}

impl Shl for u256 {
    type Output = Self;

    fn shl(self, rhs: Self) -> Self::Output {
        let (hi, lo) = rhs.0.into_words();
        if hi != 0 || lo > 255 {
            return u256::ZERO;
        }
        Self(self.0.wrapping_shl(lo as u32))
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
        let (hi, lo) = rhs.0.into_words();
        if hi != 0 || lo > 255 {
            return u256::ZERO;
        }
        Self(self.0.wrapping_shr(lo as u32))
    }
}

impl u256 {
    pub const ZERO: Self = Self(U256::ZERO);
    pub const ONE: Self = Self(U256::ONE);
    pub const MAX: Self = Self(U256::MAX);

    pub fn into_u64_with_overflow(self) -> (u64, bool) {
        let (hi, lo) = self.0.into_words();
        (lo as u64, hi != 0 || lo > u64::MAX as u128)
    }

    pub fn into_u64_saturating(self) -> u64 {
        let (hi, lo) = self.0.into_words();
        if hi != 0 || lo > u64::MAX as u128 {
            u64::MAX
        } else {
            lo as u64
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
        let s1 = bnum::types::U256::from_le_bytes(s1.0.to_le_bytes());
        let s1 = U512::cast_from(s1);
        let s2 = bnum::types::U256::from_le_bytes(s2.0.to_le_bytes());
        let s2 = U512::cast_from(s2);
        let m = bnum::types::U256::from_le_bytes(m.0.to_le_bytes());
        let m = U512::cast_from(m);

        Self(U256::from_le_bytes(
            bnum::types::U256::cast_from((s1 + s2).rem(m)).to_le_bytes(),
        ))
    }

    // ethnum has no support for addmod and mulmod yet (see https://github.com/nlordell/ethnum-rs/issues/10)
    pub fn mulmod(s1: Self, s2: Self, m: Self) -> Self {
        if m == u256::ZERO {
            return u256::ZERO;
        }
        let s1 = bnum::types::U256::from_le_bytes(s1.0.to_le_bytes());
        let s1 = U512::cast_from(s1);
        let s2 = bnum::types::U256::from_le_bytes(s2.0.to_le_bytes());
        let s2 = U512::cast_from(s2);
        let m = bnum::types::U256::from_le_bytes(m.0.to_le_bytes());
        let m = U512::cast_from(m);

        Self(U256::from_le_bytes(
            bnum::types::U256::cast_from((s1 * s2).rem(m)).to_le_bytes(),
        ))
    }

    pub fn pow(self, exp: Self) -> Self {
        let mut exp = exp.0;
        let mut base = self.0;
        let mut acc = U256::ONE;

        while exp > U256::ONE {
            if (exp & U256::ONE) == U256::ONE {
                acc = acc.wrapping_mul(base);
            }
            exp >>= 1;
            base = base.wrapping_mul(base);
        }

        if exp == U256::ONE {
            acc = acc.wrapping_mul(base);
        }

        Self(acc)
    }

    pub fn signextend(self, rhs: Self) -> Self {
        let (size_hi, size) = self.0.into_words();
        // For 31 and higher the sign byte is already the last byte, so the result is the same as
        // rhs.
        if size_hi != 0 || size >= 31 {
            return rhs;
        }
        let size = size as u32;

        // Move the sign byte to the top of its word, then replicate its sign bit back down.
        let (hi, lo) = rhs.0.into_words();
        let (hi, lo) = if size < 16 {
            let shift = (15 - size) * 8;
            let lo = (((lo << shift) as i128) >> shift) as u128;
            (((lo as i128) >> 127) as u128, lo)
        } else {
            let shift = (31 - size) * 8;
            ((((hi << shift) as i128) >> shift) as u128, lo)
        };
        Self(U256::from_words(hi, lo))
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
        let (index_hi, index_lo) = index.0.into_words();
        if index_hi != 0 || index_lo >= 32 {
            return u256::ZERO;
        }
        let (hi, lo) = self.0.into_words();
        // Position of the requested byte, counted from the least significant one.
        let pos = 31 - index_lo as u32;
        let half = if pos < 16 { lo } else { hi };
        ((half >> ((pos % 16) * 8)) as u8).into()
    }

    pub fn sar(self, rhs: Self) -> Self {
        let lhs = self.0.as_i256();
        let (hi, lo) = rhs.0.into_words();
        if hi != 0 || lo > 255 {
            if lhs.is_negative() {
                return u256::MAX;
            } else {
                return u256::ZERO;
            }
        }
        Self(lhs.wrapping_shr(lo as u32).as_u256())
    }

    pub fn leading_zeros(&self) -> u32 {
        self.0.leading_zeros()
    }

    pub fn bits(&self) -> u32 {
        256 - self.0.leading_zeros()
    }

    pub fn from_le_bytes(bytes: [u8; 32]) -> Self {
        Self(U256::from_le_bytes(bytes))
    }

    /// A lane view of the value, for the bitwise operators. On the two `u128` halves the compiler
    /// has no single aligned 32 byte access available, because `u256` is only `align(16)`, so it
    /// splits the result slot into two stores and the next handler's single wide load of that
    /// slot forwards from neither. One lane access writes the slot once instead. Lane width and
    /// byte order do not matter to a bitwise operation, so this is a plain reinterpretation.
    #[cfg(feature = "simd")]
    fn lanes(self) -> u64x4 {
        u64x4::from_ne_bytes(self.0.to_ne_bytes().into())
    }

    pub fn from_be_bytes(bytes: [u8; 32]) -> Self {
        std::cfg_select! {
            // With `simd` the bytes are reversed in a vector register rather than word by word,
            // so that storing the result takes a single store, see [`From<u64>`]. On a big-endian
            // target the reversal is the identity, so there is nothing to vectorize.
            all(feature = "simd", target_endian = "little") => Self(U256::from_ne_bytes(
                u8x32::from_array(bytes).reverse().to_array(),
            )),
            _ => Self(U256::from_be_bytes(bytes)),
        }
    }

    /// Semantically equivalent to [`u256::from_be_bytes`] but always reads `bytes` word by word,
    /// for callers that have just written them at an offset the compiler does not know: no wide
    /// read of such a buffer can be served by store-to-load forwarding.
    pub fn from_be_bytes_words(bytes: [u8; 32]) -> Self {
        Self(U256::from_be_bytes(bytes))
    }

    pub fn least_significant_byte(&self) -> u8 {
        self.0.as_u8()
    }

    pub fn to_be_bytes(self) -> [u8; 32] {
        std::cfg_select! {
            // With `simd` the bytes are reversed in a vector register rather than word by word,
            // so that storing the result takes a single store, see [`u256::from_be_bytes`]. On a
            // big-endian target the reversal is the identity, so there is nothing to vectorize.
            all(feature = "simd", target_endian = "little") => {
                u8x32::from_array(self.0.to_ne_bytes()).reverse().to_array()
            }
            _ => self.0.to_be_bytes(),
        }
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

        assert_eq!(u256::from(0u32), u256::ZERO);
        assert_eq!(u256::from(1u32), u256::ONE);

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

    #[test]
    fn signextend() {
        let cases = [
            // sizes below 31 keep the bytes up to `size` and replicate the sign bit above them
            (u256::ZERO, u256::from(0x7fu64), u256::from(0x7fu64)),
            (u256::ZERO, u256::from(0x127fu64), u256::from(0x7fu64)),
            (u256::ZERO, u256::from(0xffu64), u256::MAX),
            (u256::ZERO, u256::from(0x12ffu64), u256::MAX),
            (u256::ONE, u256::from(0xffu64), u256::from(0xffu64)),
            (u256::ONE, u256::from(0x7fffu64), u256::from(0x7fffu64)),
            (u256::ONE, u256::from(0xffffu64), u256::MAX),
            (
                u256::from(30u64),
                u256::ONE << u256::from(247u64),
                u256::MAX << u256::from(247u64),
            ),
            // byte 31 is already the sign byte, so size 31 is the identity
            (
                u256::from(31u64),
                u256::ONE << u256::from(255u64),
                u256::ONE << u256::from(255u64),
            ),
            (u256::from(31u64), u256::MAX, u256::MAX),
            // sizes above 31 are the identity as well
            (u256::from(32u64), u256::from(0xffu64), u256::from(0xffu64)),
            (u256::MAX, u256::from(0xffu64), u256::from(0xffu64)),
        ];
        for (size, value, expected) in cases {
            assert_eq!(expected, u256::signextend(size, value));
        }
    }
}
