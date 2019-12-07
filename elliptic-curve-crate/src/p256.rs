//! A pure-Rust implementation of group operations on secp256r1.

mod field;
mod util;

#[cfg(test)]
mod test_vectors;

use std::convert::TryInto;
use subtle::{Choice, ConditionallySelectable, ConstantTimeEq, CtOption};

use crate::weierstrass::curve::nistp256::PublicKey;
use field::FieldElement;

/// a = -3
const CURVE_EQUATION_A: FieldElement = FieldElement::zero()
    .sub(&FieldElement::one())
    .sub(&FieldElement::one())
    .sub(&FieldElement::one());

/// b = 0x5AC635D8AA3A93E7B3EBBD55769886BC651D06B0CC53B0F63BCE3C3E27D2604B
const CURVE_EQUATION_B: FieldElement = FieldElement([
    0xd89c_df62_29c4_bddf,
    0xacf0_05cd_7884_3090,
    0xe5a2_20ab_f721_2ed6,
    0xdc30_061d_0487_4834,
]);

/// A point on the secp256r1 curve in affine coordinates.
#[derive(Clone, Copy, Debug)]
pub struct AffinePoint {
    x: FieldElement,
    y: FieldElement,
}

impl ConditionallySelectable for AffinePoint {
    fn conditional_select(a: &AffinePoint, b: &AffinePoint, choice: Choice) -> AffinePoint {
        AffinePoint {
            x: FieldElement::conditional_select(&a.x, &b.x, choice),
            y: FieldElement::conditional_select(&a.y, &b.y, choice),
        }
    }
}

impl ConstantTimeEq for AffinePoint {
    fn ct_eq(&self, other: &AffinePoint) -> Choice {
        self.x.ct_eq(&other.x) & self.y.ct_eq(&other.y)
    }
}

impl PartialEq for AffinePoint {
    fn eq(&self, other: &AffinePoint) -> bool {
        self.ct_eq(other).into()
    }
}

impl Eq for AffinePoint {}

impl AffinePoint {
    /// Attempts to parse the given [`PublicKey`] as an SEC-1-encoded `AffinePoint`.
    ///
    /// # Returns
    ///
    /// `None` value if `pubkey` is not on the secp256r1 curve.
    pub fn from_pubkey(pubkey: &PublicKey) -> CtOption<Self> {
        match pubkey {
            PublicKey::Compressed(_) => unimplemented!(),
            PublicKey::Uncompressed(point) => {
                let bytes = point.as_bytes();

                let x = FieldElement::from_bytes(bytes[1..33].try_into().unwrap());
                let y = FieldElement::from_bytes(bytes[33..65].try_into().unwrap());

                x.and_then(|x| {
                    y.and_then(|y| {
                        // Check that the point is on the curve
                        let lhs = y * &y;
                        let rhs = x * &x * &x + &(CURVE_EQUATION_A * &x) + &CURVE_EQUATION_B;
                        CtOption::new(AffinePoint { x, y }, lhs.ct_eq(&rhs))
                    })
                })
            }
        }
    }

    /// Returns the SEC-1 uncompressed encoding of this point, as a [`PublicKey`].
    pub fn to_uncompressed_pubkey(&self) -> PublicKey {
        let mut encoded = [0; 65];
        encoded[0] = 0x04;
        encoded[1..33].copy_from_slice(&self.x.to_bytes());
        encoded[33..65].copy_from_slice(&self.y.to_bytes());

        PublicKey::from_bytes(&encoded[..]).expect("we encoded it correctly")
    }
}

#[cfg(test)]
mod tests {
    use super::{AffinePoint, CURVE_EQUATION_A, CURVE_EQUATION_B};
    use crate::weierstrass::curve::nistp256::PublicKey;

    const CURVE_EQUATION_A_BYTES: &str =
        "FFFFFFFF00000001000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFC";
    const CURVE_EQUATION_B_BYTES: &str =
        "5AC635D8AA3A93E7B3EBBD55769886BC651D06B0CC53B0F63BCE3C3E27D2604B";

    const UNCOMPRESSED_BASEPOINT: &str =
        "046B17D1F2E12C4247F8BCE6E563A440F277037D812DEB33A0F4A13945D898C2964FE342E2FE1A7F9B8EE7EB4A7C0F9E162BCE33576B315ECECBB6406837BF51F5";

    #[test]
    fn verify_constants() {
        assert_eq!(
            hex::encode(CURVE_EQUATION_A.to_bytes()).to_uppercase(),
            CURVE_EQUATION_A_BYTES
        );
        assert_eq!(
            hex::encode(CURVE_EQUATION_B.to_bytes()).to_uppercase(),
            CURVE_EQUATION_B_BYTES
        );
    }

    #[test]
    fn uncompressed_round_trip() {
        let pubkey = PublicKey::from_bytes(&hex::decode(UNCOMPRESSED_BASEPOINT).unwrap()).unwrap();

        assert_eq!(
            AffinePoint::from_pubkey(&pubkey)
                .unwrap()
                .to_uncompressed_pubkey(),
            pubkey
        );
    }
}
