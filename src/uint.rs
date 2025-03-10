//! Big unsigned integers.

#![allow(
    clippy::needless_range_loop,
    clippy::many_single_char_names,
    clippy::derive_hash_xor_eq
)]

#[macro_use]
mod concat;
#[macro_use]
mod split;

mod add;
mod add_mod;
mod bit_and;
mod bit_not;
mod bit_or;
mod bit_xor;
mod bits;
mod cmp;
mod div;
pub(crate) mod div_limb;
mod encoding;
mod from;
mod inv_mod;
mod mul;
mod mul_mod;
mod neg;
mod neg_mod;
mod resize;
mod shl;
mod shr;
mod sqrt;
mod sub;
mod sub_mod;

/// Implements modular arithmetic for constant moduli.
pub mod modular;

#[cfg(feature = "generic-array")]
mod array;

#[cfg(feature = "rand_core")]
mod rand;

use crate::{Bounded, Concat, Encoding, Integer, Limb, Split, Word, Zero};
use core::fmt;
use subtle::{Choice, ConditionallySelectable};

#[cfg(feature = "serde")]
use serdect::serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "zeroize")]
use zeroize::DefaultIsZeroes;

/// Big unsigned integer.
///
/// Generic over the given number of `LIMBS`
///
/// # Encoding support
/// This type supports many different types of encodings, either via the
/// [`Encoding`][`crate::Encoding`] trait or various `const fn` decoding and
/// encoding functions that can be used with [`Uint`] constants.
///
/// Optional crate features for encoding (off-by-default):
/// - `generic-array`: enables [`ArrayEncoding`][`crate::ArrayEncoding`] trait which can be used to
///   [`Uint`] as `GenericArray<u8, N>` and a [`ArrayDecoding`][`crate::ArrayDecoding`] trait which
///   can be used to `GenericArray<u8, N>` as [`Uint`].
/// - `rlp`: support for [Recursive Length Prefix (RLP)][RLP] encoding.
///
/// [RLP]: https://eth.wiki/fundamentals/rlp
// TODO(tarcieri): make generic around a specified number of bits.
#[derive(Copy, Clone, Debug, Hash)]
pub struct Uint<const LIMBS: usize> {
    /// Inner limb array. Stored from least significant to most significant.
    limbs: [Limb; LIMBS],
}

impl<const LIMBS: usize> Uint<LIMBS> {
    /// The value `0`.
    pub const ZERO: Self = Self::from_u8(0);

    /// The value `1`.
    pub const ONE: Self = Self::from_u8(1);

    /// Maximum value this [`Uint`] can express.
    pub const MAX: Self = Self {
        limbs: [Limb::MAX; LIMBS],
    };

    /// Total size of the represented integer in bits.
    pub const BITS: usize = LIMBS * Limb::BITS;

    /// Total size of the represented integer in bytes.
    pub const BYTES: usize = LIMBS * Limb::BYTES;

    /// The number of limbs used on this platform.
    pub const LIMBS: usize = LIMBS;

    /// Const-friendly [`Uint`] constructor.
    pub const fn new(limbs: [Limb; LIMBS]) -> Self {
        Self { limbs }
    }

    /// Create a [`Uint`] from an array of [`Word`]s (i.e. word-sized unsigned
    /// integers).
    #[inline]
    pub const fn from_words(arr: [Word; LIMBS]) -> Self {
        let mut limbs = [Limb::ZERO; LIMBS];
        let mut i = 0;

        while i < LIMBS {
            limbs[i] = Limb(arr[i]);
            i += 1;
        }

        Self { limbs }
    }

    /// Create an array of [`Word`]s (i.e. word-sized unsigned integers) from
    /// a [`Uint`].
    #[inline]
    pub const fn to_words(self) -> [Word; LIMBS] {
        let mut arr = [0; LIMBS];
        let mut i = 0;

        while i < LIMBS {
            arr[i] = self.limbs[i].0;
            i += 1;
        }

        arr
    }

    /// Borrow the inner limbs as an array of [`Word`]s.
    pub const fn as_words(&self) -> &[Word; LIMBS] {
        // SAFETY: `Limb` is a `repr(transparent)` newtype for `Word`
        #[allow(trivial_casts, unsafe_code)]
        unsafe {
            &*((&self.limbs as *const _) as *const [Word; LIMBS])
        }
    }

    /// Borrow the inner limbs as a mutable array of [`Word`]s.
    pub fn as_words_mut(&mut self) -> &mut [Word; LIMBS] {
        // SAFETY: `Limb` is a `repr(transparent)` newtype for `Word`
        #[allow(trivial_casts, unsafe_code)]
        unsafe {
            &mut *((&mut self.limbs as *mut _) as *mut [Word; LIMBS])
        }
    }

    /// Borrow the limbs of this [`Uint`].
    pub const fn as_limbs(&self) -> &[Limb; LIMBS] {
        &self.limbs
    }

    /// Borrow the limbs of this [`Uint`] mutably.
    pub fn as_limbs_mut(&mut self) -> &mut [Limb; LIMBS] {
        &mut self.limbs
    }

    /// Convert this [`Uint`] into its inner limbs.
    pub const fn to_limbs(self) -> [Limb; LIMBS] {
        self.limbs
    }
}

impl<const LIMBS: usize> AsRef<[Word; LIMBS]> for Uint<LIMBS> {
    fn as_ref(&self) -> &[Word; LIMBS] {
        self.as_words()
    }
}

impl<const LIMBS: usize> AsMut<[Word; LIMBS]> for Uint<LIMBS> {
    fn as_mut(&mut self) -> &mut [Word; LIMBS] {
        self.as_words_mut()
    }
}

impl<const LIMBS: usize> AsRef<[Limb]> for Uint<LIMBS> {
    fn as_ref(&self) -> &[Limb] {
        self.as_limbs()
    }
}

impl<const LIMBS: usize> AsMut<[Limb]> for Uint<LIMBS> {
    fn as_mut(&mut self) -> &mut [Limb] {
        self.as_limbs_mut()
    }
}

impl<const LIMBS: usize> ConditionallySelectable for Uint<LIMBS> {
    fn conditional_select(a: &Self, b: &Self, choice: Choice) -> Self {
        let mut limbs = [Limb::ZERO; LIMBS];

        for i in 0..LIMBS {
            limbs[i] = Limb::conditional_select(&a.limbs[i], &b.limbs[i], choice);
        }

        Self { limbs }
    }
}

impl<const LIMBS: usize> Default for Uint<LIMBS> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<const LIMBS: usize> Integer for Uint<LIMBS> {
    const ONE: Self = Self::ONE;
    const MAX: Self = Self::MAX;
    const BITS: usize = Self::BITS;
    const BYTES: usize = Self::BYTES;
    const LIMBS: usize = Self::LIMBS;

    fn is_odd(&self) -> Choice {
        self.limbs
            .first()
            .map(|limb| limb.is_odd())
            .unwrap_or_else(|| Choice::from(0))
    }
}

impl<const LIMBS: usize> Zero for Uint<LIMBS> {
    const ZERO: Self = Self::ZERO;
}

impl<const LIMBS: usize> Bounded for Uint<LIMBS> {
    const BITS: usize = Self::BITS;
    const BYTES: usize = Self::BYTES;
}

impl<const LIMBS: usize> fmt::Display for Uint<LIMBS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(self, f)
    }
}

impl<const LIMBS: usize> fmt::LowerHex for Uint<LIMBS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for limb in self.limbs.iter().rev() {
            fmt::LowerHex::fmt(limb, f)?;
        }
        Ok(())
    }
}

impl<const LIMBS: usize> fmt::UpperHex for Uint<LIMBS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for limb in self.limbs.iter().rev() {
            fmt::UpperHex::fmt(limb, f)?;
        }
        Ok(())
    }
}

#[cfg(feature = "serde")]
impl<'de, const LIMBS: usize> Deserialize<'de> for Uint<LIMBS>
where
    Uint<LIMBS>: Encoding,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut buffer = Self::ZERO.to_le_bytes();
        serdect::array::deserialize_hex_or_bin(buffer.as_mut(), deserializer)?;

        Ok(Self::from_le_bytes(buffer))
    }
}

#[cfg(feature = "serde")]
impl<const LIMBS: usize> Serialize for Uint<LIMBS>
where
    Uint<LIMBS>: Encoding,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serdect::array::serialize_hex_lower_or_bin(&Encoding::to_le_bytes(self), serializer)
    }
}

#[cfg(feature = "zeroize")]
impl<const LIMBS: usize> DefaultIsZeroes for Uint<LIMBS> {}

// TODO(tarcieri): use `const_evaluatable_checked` when stable to make generic around bits.
macro_rules! impl_uint_aliases {
    ($(($name:ident, $bits:expr, $doc:expr)),+) => {
        $(
            #[doc = $doc]
            #[doc="unsigned big integer."]
            pub type $name = Uint<{nlimbs!($bits)}>;

            impl Encoding for $name {

                type Repr = [u8; $bits / 8];

                #[inline]
                fn from_be_bytes(bytes: Self::Repr) -> Self {
                    Self::from_be_slice(&bytes)
                }

                #[inline]
                fn from_le_bytes(bytes: Self::Repr) -> Self {
                    Self::from_le_slice(&bytes)
                }

                #[inline]
                fn to_be_bytes(&self) -> Self::Repr {
                    let mut result = [0u8; $bits / 8];
                    self.write_be_bytes(&mut result);
                    result
                }

                #[inline]
                fn to_le_bytes(&self) -> Self::Repr {
                    let mut result = [0u8; $bits / 8];
                    self.write_le_bytes(&mut result);
                    result
                }
            }
        )+
     };
}

// TODO(tarcieri): use `const_evaluatable_checked` when stable to make generic around bits.
impl_uint_aliases! {
    (U64, 64, "64-bit"),
    (U128, 128, "128-bit"),
    (U192, 192, "192-bit"),
    (U256, 256, "256-bit"),
    (U320, 320, "320-bit"),
    (U384, 384, "384-bit"),
    (U448, 448, "448-bit"),
    (U512, 512, "512-bit"),
    (U576, 576, "576-bit"),
    (U640, 640, "640-bit"),
    (U768, 768, "768-bit"),
    (U896, 896, "896-bit"),
    (U1024, 1024, "1024-bit"),
    (U1280, 1280, "1280-bit"),
    (U1536, 1536, "1536-bit"),
    (U1792, 1792, "1792-bit"),
    (U2048, 2048, "2048-bit"),
    (U3072, 3072, "3072-bit"),
    (U3584, 3584, "3584-bit"),
    (U4096, 4096, "4096-bit"),
    (U6144, 6144, "6144-bit"),
    (U8192, 8192, "8192-bit")
}

#[cfg(target_pointer_width = "32")]
impl_uint_aliases! {
    (U224, 224, "224-bit"), // For NIST P-224
    (U544, 544, "544-bit")  // For NIST P-521
}

// TODO(tarcieri): use `const_evaluatable_checked` when stable to make generic around bits.
impl_concat! {
    (U64, 64),
    (U128, 128),
    (U192, 192),
    (U256, 256),
    (U320, 320),
    (U384, 384),
    (U448, 448),
    (U512, 512),
    (U640, 640),
    (U768, 768),
    (U896, 896),
    (U1024, 1024),
    (U1536, 1536),
    (U1792, 1792),
    (U2048, 2048),
    (U3072, 3072),
    (U4096, 4096)
}

// TODO(tarcieri): use `const_evaluatable_checked` when stable to make generic around bits.
impl_split! {
    (U128, 128),
    (U256, 256),
    (U384, 384),
    (U512, 512),
    (U640, 640),
    (U768, 768),
    (U896, 896),
    (U1024, 1024),
    (U1280, 1280),
    (U1536, 1536),
    (U1792, 1792),
    (U2048, 2048),
    (U3072, 3072),
    (U3584, 3584),
    (U4096, 4096),
    (U6144, 6144),
    (U8192, 8192)
}

#[cfg(test)]
mod tests {
    use crate::{Encoding, U128};
    use subtle::ConditionallySelectable;

    #[cfg(feature = "serde")]
    use crate::U64;

    #[test]
    #[cfg(feature = "alloc")]
    fn display() {
        let hex = "AAAAAAAABBBBBBBBCCCCCCCCDDDDDDDD";
        let n = U128::from_be_hex(hex);

        use alloc::string::ToString;
        assert_eq!(hex, n.to_string());

        let hex = "AAAAAAAABBBBBBBB0000000000000000";
        let n = U128::from_be_hex(hex);
        assert_eq!(hex, n.to_string());

        let hex = "AAAAAAAABBBBBBBB00000000DDDDDDDD";
        let n = U128::from_be_hex(hex);
        assert_eq!(hex, n.to_string());

        let hex = "AAAAAAAABBBBBBBB0CCCCCCCDDDDDDDD";
        let n = U128::from_be_hex(hex);
        assert_eq!(hex, n.to_string());
    }

    #[test]
    fn from_bytes() {
        let a = U128::from_be_hex("AAAAAAAABBBBBBBB0CCCCCCCDDDDDDDD");

        let be_bytes = a.to_be_bytes();
        let le_bytes = a.to_le_bytes();
        for i in 0..16 {
            assert_eq!(le_bytes[i], be_bytes[15 - i]);
        }

        let a_from_be = U128::from_be_bytes(be_bytes);
        let a_from_le = U128::from_le_bytes(le_bytes);
        assert_eq!(a_from_be, a_from_le);
        assert_eq!(a_from_be, a);
    }

    #[test]
    fn conditional_select() {
        let a = U128::from_be_hex("00002222444466668888AAAACCCCEEEE");
        let b = U128::from_be_hex("11113333555577779999BBBBDDDDFFFF");

        let select_0 = U128::conditional_select(&a, &b, 0.into());
        assert_eq!(a, select_0);

        let select_1 = U128::conditional_select(&a, &b, 1.into());
        assert_eq!(b, select_1);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde() {
        const TEST: U64 = U64::from_u64(0x0011223344556677);

        let serialized = bincode::serialize(&TEST).unwrap();
        let deserialized: U64 = bincode::deserialize(&serialized).unwrap();

        assert_eq!(TEST, deserialized);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_owned() {
        const TEST: U64 = U64::from_u64(0x0011223344556677);

        let serialized = bincode::serialize(&TEST).unwrap();
        let deserialized: U64 = bincode::deserialize_from(serialized.as_slice()).unwrap();

        assert_eq!(TEST, deserialized);
    }
}
