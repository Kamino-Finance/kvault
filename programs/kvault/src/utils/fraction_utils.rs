use kamino_lending::{fraction::Fraction, utils::U256};

#[inline]
pub fn full_mul_fraction_ratio_ceil(
    input_fraction: Fraction,
    numerator: Fraction,
    denominator: Fraction,
) -> Fraction {
    let big_sf = U256::from(input_fraction.to_bits());
    let big_numerator = U256::from(numerator.to_bits());
    let big_denominator = U256::from(denominator.to_bits());

    // For ceiling behavior: (a * b + c - 1) / c
    // where c is the denominator
    let big_sf_res = (big_sf * big_numerator + big_denominator - 1) / big_denominator;

    let sf_res: u128 = big_sf_res
        .try_into()
        .expect("Result doesn't fit in a Fraction.");
    Fraction::from_bits(sf_res)
}
