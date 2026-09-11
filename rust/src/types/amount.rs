#[cfg(all(feature = "simd", target_endian = "little"))]
use std::simd::u8x32;
#[cfg(feature = "simd")]
use std::simd::{ToBytes, u64x4};
use std::{
    fmt::{Debug, Display, LowerHex},
    ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Rem, Shl, Shr, Sub},
};

#[cfg(feature = "fuzzing")]
use arbitrary::Arbitrary;
use bnum::{cast::CastFrom, types::U512};
use ethnum::U256;
use evmc_vm::{Address, Uint256};

/// This represents a 256-bit integer in native endian.
#[expect(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

impl Sub for u256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

impl Mul for u256 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_mul(rhs.0))
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

impl Rem for u256 {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        if rhs == u256::ZERO {
            return u256::ZERO;
        }
        Self(self.0.wrapping_rem(rhs.0))
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

    pub fn least_significant_byte(&self) -> u8 {
        self.0.as_u8()
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
    use evmc_vm::{Address, Uint256};
    use rstest::rstest;

    use super::*;

    /// 2^128, the smallest value with a non-zero high word.
    const TWO_POW_128: u256 = u256(U256::from_words(1, 0));
    /// 2^255, the most negative value when interpreted as signed.
    const SIGNED_MIN: u256 = u256(U256::from_words(1 << 127, 0));

    #[rstest]
    #[case::zero(
        u256::ZERO,
        "0",
        "0000000000000000_0000000000000000_0000000000000000_0000000000000000"
    )]
    #[case::least_significant_byte(
        u256::from(0xfeu8),
        "254",
        "0000000000000000_0000000000000000_0000000000000000_00000000000000fe"
    )]
    #[case::most_significant_byte(
        u256::from(0xfeu8) << u256::from(8 * 31u8),
        "114887463540149662646824336688307533573166312910440247132899321632851308314624",
        "fe00000000000000_0000000000000000_0000000000000000_0000000000000000"
    )]
    #[case::every_byte_distinct(
        u256::from_be_bytes(std::array::from_fn(|i| i as u8)),
        "1780731860627700044960722568376592200742329637303199754547598369979440671",
        "0001020304050607_08090a0b0c0d0e0f_1011121314151617_18191a1b1c1d1e1f"
    )]
    fn display_and_lower_hex(#[case] value: u256, #[case] decimal: &str, #[case] hex: &str) {
        assert_eq!(format!("{value}"), decimal);
        assert_eq!(format!("{value:x}"), hex);
        assert_eq!(format!("{value:#x}"), format!("0x{hex}"));
    }

    #[test]
    fn conversions_to_u256() {
        assert_eq!(u256::from(false), u256::ZERO);
        assert_eq!(u256::from(true), u256::ONE);

        assert_eq!(u256::from(u8::MIN), u256(U256::from(u8::MIN)));
        assert_eq!(u256::from(1u8), u256(U256::from(1u8)));
        assert_eq!(u256::from(u8::MAX), u256(U256::from(u8::MAX)));

        assert_eq!(u256::from(u32::MIN), u256(U256::from(u32::MIN)));
        assert_eq!(u256::from(1u32), u256(U256::from(1u32)));
        assert_eq!(u256::from(u32::MAX), u256(U256::from(u32::MAX)));

        assert_eq!(u256::from(u64::MIN), u256(U256::from(u64::MIN)));
        assert_eq!(u256::from(1u64), u256(U256::from(1u64)));
        assert_eq!(u256::from(u64::MAX), u256(U256::from(u64::MAX)));

        assert_eq!(u256::from(usize::MIN), u256(U256::from(usize::MIN as u64)));
        assert_eq!(u256::from(1usize), u256(U256::from(1u64)));
        assert_eq!(u256::from(usize::MAX), u256(U256::from(usize::MAX as u64)));

        assert_eq!(u256::from(Uint256 { bytes: [0; 32] }), u256::ZERO);
        assert_eq!(
            u256::from(Uint256 {
                bytes: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 1
                ]
            }),
            u256::ONE
        );
        assert_eq!(u256::from(Uint256 { bytes: [0xff; 32] }), u256::MAX);

        assert_eq!(u256::from(Address { bytes: [0; 20] }), u256::ZERO);
        assert_eq!(
            u256::from(Address {
                bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
            }),
            u256::ONE
        );
        assert_eq!(
            u256::from(Address { bytes: [0xff; 20] }),
            u256::MAX >> u256::from(12 * 8u8)
        );
    }

    #[test]
    fn conversions_from_u256() {
        assert_eq!(Uint256::from(u256::ZERO), Uint256 { bytes: [0; 32] });
        assert_eq!(
            Uint256::from(u256::ONE),
            Uint256 {
                bytes: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 1
                ]
            }
        );
        assert_eq!(Uint256::from(u256::MAX), Uint256 { bytes: [0xff; 32] });

        assert_eq!(Address::from(u256::ZERO), Address { bytes: [0; 20] });
        assert_eq!(
            Address::from(u256::ONE),
            Address {
                bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
            }
        );
        assert_eq!(
            Address::from(u256::MAX >> u256::from(12 * 8u8)),
            Address { bytes: [0xff; 20] }
        );
        // the 12 most significant bytes do not fit into an address and are dropped
        assert_eq!(Address::from(u256::MAX), Address { bytes: [0xff; 20] });
    }

    #[rstest]
    #[case::zero(u256::ZERO, 0, false)]
    #[case::one(u256::ONE, 1, false)]
    #[case::u64_max(u256::from(u64::MAX), u64::MAX, false)]
    #[case::u64_max_plus_one(u256::from(u64::MAX) + u256::ONE, 0, true)]
    #[case::one_in_the_high_word(TWO_POW_128, 0, true)]
    #[case::max(u256::MAX, u64::MAX, true)]
    fn into_u64(#[case] value: u256, #[case] low: u64, #[case] overflows: bool) {
        assert_eq!(value.into_u64_with_overflow(), (low, overflows));
        assert_eq!(
            value.into_u64_saturating(),
            if overflows { u64::MAX } else { low }
        );
        assert_eq!(
            u64::try_from(value),
            if overflows { Err(U64Overflow) } else { Ok(low) }
        );
    }

    #[rstest]
    #[case::add_wraps_around(u256::MAX + u256::ONE, u256::ZERO)]
    #[case::sub_wraps_around(u256::ZERO - u256::ONE, u256::MAX)]
    #[case::mul_wraps_around(u256::MAX * u256::from(2u8), u256::MAX - u256::ONE)]
    #[case::div(u256::from(7u8) / u256::from(2u8), u256::from(3u8))]
    #[case::div_by_zero_is_zero(u256::from(7u8) / u256::ZERO, u256::ZERO)]
    #[case::rem(u256::from(7u8) % u256::from(2u8), u256::ONE)]
    #[case::rem_by_zero_is_zero(u256::from(7u8) % u256::ZERO, u256::ZERO)]
    fn arithmetic_operators(#[case] result: u256, #[case] expected: u256) {
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::and(u256::MAX & u256::from(0xf0u8), u256::from(0xf0u8))]
    #[case::or(u256::from(0x0fu8) | u256::from(0xf0u8), u256::from(0xffu8))]
    #[case::xor(u256::MAX ^ u256::from(0xf0u8), u256::MAX - u256::from(0xf0u8))]
    #[case::not(!(u256::from(0xffu8)), u256::MAX << u256::from(8u8))]
    fn bitwise_operators(#[case] result: u256, #[case] expected: u256) {
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::shl(u256::ONE << u256::from(8u8), u256::from(256u32))]
    #[case::shl_by_255(u256::ONE << u256::from(255u8), SIGNED_MIN)]
    #[case::shl_by_more_than_255_is_zero(u256::MAX << u256::from(256u32), u256::ZERO)]
    #[case::shl_by_amount_in_the_high_word_is_zero(u256::MAX << TWO_POW_128, u256::ZERO)]
    #[case::shr(u256::from(256u32) >> u256::from(8u8), u256::ONE)]
    #[case::shr_does_not_keep_the_sign(u256::MAX >> u256::from(255u8), u256::ONE)]
    #[case::shr_by_more_than_255_is_zero(u256::MAX >> u256::from(256u32), u256::ZERO)]
    #[case::shr_by_amount_in_the_high_word_is_zero(u256::MAX >> TWO_POW_128, u256::ZERO)]
    #[case::sar_of_positive_is_positive(u256::from(8u8).sar(u256::from(2u8)), u256::from(2u8))]
    #[case::sar_of_negative_is_negative(
        (u256::ZERO - u256::from(8u8)).sar(u256::from(2u8)),
        u256::ZERO - u256::from(2u8)
    )]
    #[case::sar_of_positive_by_255_keeps_the_sign(
        (u256::MAX >> u256::ONE).sar(u256::from(255u8)),
        u256::ZERO
    )]
    #[case::sar_of_negative_by_255_keeps_the_sign(u256::MAX.sar(u256::from(255u8)), u256::MAX)]
    #[case::sar_of_positive_by_more_than_255_keeps_the_sign(
        u256::from(8u8).sar(u256::from(256u32)),
        u256::ZERO
    )]
    #[case::sar_of_negative_by_more_than_255_keeps_the_sign(
        (u256::ZERO - u256::from(8u8)).sar(u256::from(256u32)),
        u256::MAX
    )]
    #[case::sar_of_positive_by_amount_in_the_high_word_keeps_the_sign(
        u256::from(8u8).sar(TWO_POW_128),
        u256::ZERO
    )]
    #[case::sar_of_negative_by_amount_in_the_high_word_keeps_the_sign(
        (u256::ZERO - u256::from(8u8)).sar(TWO_POW_128),
        u256::MAX
    )]
    fn shifts(#[case] result: u256, #[case] expected: u256) {
        assert_eq!(result, expected);
    }

    #[rstest]
    // sdiv sign is sign of quotient
    #[case::sdiv(u256::from(7u8).sdiv(u256::from(2u8)), u256::from(3u8))]
    #[case::sdiv_of_negative_dividend_truncates_towards_zero(
        (u256::ZERO - u256::from(7u8)).sdiv(u256::from(2u8)),
        u256::ZERO - u256::from(3u8)
    )]
    #[case::sdiv_of_negative_divisor_is_negative(
        u256::from(7u8).sdiv(u256::ZERO - u256::from(2u8)),
        u256::ZERO - u256::from(3u8)
    )]
    #[case::sdiv_of_two_negatives_is_positive(
        (u256::ZERO - u256::from(7u8)).sdiv(u256::ZERO - u256::from(2u8)),
        u256::from(3u8)
    )]
    #[case::sdiv_by_zero_is_zero(u256::from(7u8).sdiv(u256::ZERO), u256::ZERO)]
    #[case::sdiv_of_min_by_minus_one_wraps_to_min(SIGNED_MIN.sdiv(u256::MAX), SIGNED_MIN)]
    // srem sign is sign of dividend
    #[case::srem(u256::from(7u8).srem(u256::from(2u8)), u256::ONE)]
    #[case::srem_of_negative_dividend_is_negative(
        (u256::ZERO - u256::from(7u8)).srem(u256::from(2u8)),
        u256::ZERO - u256::ONE
    )]
    #[case::srem_of_negative_divisor_is_positive(
        u256::from(7u8).srem(u256::ZERO - u256::from(2u8)),
        u256::ONE
    )]
    #[case::srem_of_two_negatives_is_negative(
        (u256::ZERO - u256::from(7u8)).srem(u256::ZERO - u256::from(2u8)),
        u256::ZERO - u256::ONE
    )]
    #[case::srem_by_zero_is_zero(u256::from(7u8).srem(u256::ZERO), u256::ZERO)]
    #[case::srem_of_min_by_minus_one_is_zero(SIGNED_MIN.srem(u256::MAX), u256::ZERO)]
    fn sdiv_and_srem(#[case] result: u256, #[case] expected: u256) {
        assert_eq!(result, expected);
    }

    // 2 ^ 256 is congruent 2 modulo 7, so the intermediate result does not fit into 256 bits.
    #[rstest]
    #[case::addmod(
        u256::addmod(u256::from(3u8), u256::from(4u8), u256::from(5u8)),
        u256::from(2u8)
    )]
    #[case::addmod_carries_out_of_256_bits(
        u256::addmod(u256::MAX, u256::ONE, u256::from(7u8)),
        u256::from(2u8)
    )]
    #[case::addmod_by_zero_is_zero(u256::addmod(u256::ONE, u256::ONE, u256::ZERO), u256::ZERO)]
    #[case::mulmod(
        u256::mulmod(u256::from(3u8), u256::from(4u8), u256::from(5u8)),
        u256::from(2u8)
    )]
    #[case::mulmod_carries_out_of_256_bits(
        u256::mulmod(u256::MAX, u256::from(2u8), u256::from(7u8)),
        u256::from(2u8)
    )]
    #[case::mulmod_by_zero_is_zero(u256::mulmod(u256::ONE, u256::ONE, u256::ZERO), u256::ZERO)]
    fn addmod_and_mulmod(#[case] result: u256, #[case] expected: u256) {
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::zero_exponent_is_one(u256::from(3u8), u256::ZERO, u256::ONE)]
    #[case::zero_to_the_zero_is_one(u256::ZERO, u256::ZERO, u256::ONE)]
    #[case::one_exponent_is_base(u256::from(3u8), u256::ONE, u256::from(3u8))]
    #[case::even_exponent(u256::from(3u8), u256::from(2u8), u256::from(9u8))]
    #[case::odd_exponent(u256::from(3u8), u256::from(3u8), u256::from(27u8))]
    #[case::exponent_in_the_high_word(u256::MAX, TWO_POW_128, u256::ONE)]
    #[case::wraps_around(u256::from(2u8), u256::from(258u32), u256::ZERO)]
    fn pow(#[case] base: u256, #[case] exp: u256, #[case] expected: u256) {
        assert_eq!(base.pow(exp), expected);
    }

    #[rstest]
    #[case::positive_keeps_the_value(u256::ZERO, u256::from(0x7fu8), u256::from(0x7fu8))]
    #[case::positive_clears_the_bytes_above(u256::ZERO, u256::from(0x127fu32), u256::from(0x7fu8))]
    #[case::negative_sets_the_bytes_above(u256::ZERO, u256::from(0xffu8), u256::MAX)]
    #[case::negative_overwrites_the_bytes_above(u256::ZERO, u256::from(0x12ffu32), u256::MAX)]
    #[case::only_the_sign_byte_decides_the_sign(u256::ONE, u256::from(0xffu8), u256::from(0xffu8))]
    #[case::positive_keeps_two_bytes(u256::ONE, u256::from(0x7fffu32), u256::from(0x7fffu32))]
    #[case::negative_extends_two_bytes(u256::ONE, u256::from(0xffffu32), u256::MAX)]
    // sizes 15 and 16 are the two sides of the split between the low and the high word
    #[case::negative_in_the_last_byte_of_the_low_word(
        u256::from(15u8),
        u256::ONE << u256::from(127u8),
        u256::MAX << u256::from(127u8)
    )]
    #[case::negative_in_the_first_byte_of_the_high_word(
        u256::from(16u8),
        u256::ONE << u256::from(135u8),
        u256::MAX << u256::from(135u8)
    )]
    #[case::negative_in_the_high_word_keeps_the_low_word(
        u256::from(16u8),
        u256::MAX >> u256::from(120u8),
        u256::MAX
    )]
    #[case::positive_in_the_high_word_clears_the_bytes_above(
        u256::from(16u8),
        u256::MAX - (u256::from(0x80u8) << u256::from(128u8)),
        u256::MAX >> u256::from(121u8)
    )]
    #[case::negative_in_the_second_most_significant_byte(
        u256::from(30u8),
        u256::ONE << u256::from(247u8),
        u256::MAX << u256::from(247u8)
    )]
    #[case::size_31_keeps_a_positive_value(u256::from(31u8), u256::ONE, u256::ONE)]
    #[case::size_31_keeps_a_negative_value(u256::from(31u8), u256::MAX, u256::MAX)]
    #[case::size_above_31_keeps_the_value(u256::from(32u8), u256::from(0xffu8), u256::from(0xffu8))]
    #[case::size_in_the_high_word_keeps_the_value(
        TWO_POW_128,
        u256::from(0xffu8),
        u256::from(0xffu8)
    )]
    fn signextend(#[case] size: u256, #[case] value: u256, #[case] expected: u256) {
        assert_eq!(size.signextend(value), expected);
    }

    #[rstest]
    #[case::positive_less_than_positive(u256::ONE, u256::from(2u8), true)]
    #[case::positive_greater_than_positive(u256::from(2u8), u256::ONE, false)]
    #[case::positive_equal_positive(u256::ONE, u256::ONE, false)]
    #[case::negative_less_than_negative(u256::ZERO - u256::from(2u8), u256::ZERO - u256::ONE, true)]
    #[case::negative_greater_than_negative(
        u256::ZERO - u256::ONE,
        u256::ZERO - u256::from(2u8),
        false
    )]
    #[case::negative_equal_negative(u256::ZERO - u256::ONE, u256::ZERO - u256::ONE, false)]
    #[case::negative_less_than_positive(u256::ZERO - u256::ONE, u256::ONE, true)]
    #[case::positive_less_than_negative(u256::ONE, u256::ZERO - u256::ONE, false)]
    fn slt_and_sgt(#[case] lhs: u256, #[case] rhs: u256, #[case] expected: bool) {
        assert_eq!(lhs.slt(&rhs), expected);
        assert_eq!(rhs.sgt(&lhs), expected);
    }

    #[rstest]
    #[case::most_significant_byte(u256::MAX >> u256::from(4u8), u256::ZERO, u256::from(0x0fu8))]
    #[case::second_most_significant_byte(
        u256::MAX >> u256::from(12u8),
        u256::ONE,
        u256::from(0x0fu8)
    )]
    // indices 15 and 16 are the two sides of the split between the high and the low word
    #[case::least_significant_byte_of_the_high_word(
        u256(U256::from_words(0x12, 0x34 << 120)),
        u256::from(15u8),
        u256::from(0x12u8)
    )]
    #[case::most_significant_byte_of_the_low_word(
        u256(U256::from_words(0x12, 0x34 << 120)),
        u256::from(16u8),
        u256::from(0x34u8)
    )]
    #[case::second_least_significant_byte(
        u256::from(0x1234u32),
        u256::from(30u8),
        u256::from(0x12u8)
    )]
    #[case::least_significant_byte(u256::from(0x1234u32), u256::from(31u8), u256::from(0x34u8))]
    #[case::index_above_31_is_zero(u256::MAX, u256::from(32u8), u256::ZERO)]
    #[case::index_in_the_high_word_is_zero(u256::MAX, TWO_POW_128, u256::ZERO)]
    fn byte(#[case] value: u256, #[case] index: u256, #[case] expected: u256) {
        assert_eq!(value.byte(index), expected);
    }

    #[rstest]
    #[case::zero(u256::ZERO, 256, 0)]
    #[case::one(u256::ONE, 255, 1)]
    #[case::one_in_the_high_word(TWO_POW_128, 127, 129)]
    #[case::max(u256::MAX, 0, 256)]
    fn leading_zeros_and_bits(#[case] value: u256, #[case] leading_zeros: u32, #[case] bits: u32) {
        assert_eq!(value.leading_zeros(), leading_zeros);
        assert_eq!(value.bits(), bits);
    }

    #[test]
    fn least_significant_byte_returns_the_last_byte() {
        assert_eq!(u256::from(0x1234u32).least_significant_byte(), 0x34);
    }

    #[test]
    fn byte_array_conversions() {
        let bytes: [u8; 32] = std::array::from_fn(|i| i as u8);
        let be = u256(U256::from_words(
            0x0001020304050607_08090a0b0c0d0e0f,
            0x1011121314151617_18191a1b1c1d1e1f,
        ));
        let le = u256(U256::from_words(
            0x1f1e1d1c1b1a1918_1716151413121110,
            0x0f0e0d0c0b0a0908_0706050403020100,
        ));

        assert_eq!(u256::from_be_bytes(bytes), be);
        assert_eq!(u256::from_be_bytes_words(bytes), be);
        assert_eq!(u256::from_le_bytes(bytes), le);
        assert_eq!(be.to_be_bytes(), bytes);
    }
}
