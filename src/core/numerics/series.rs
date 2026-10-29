//! John Machin's 1706 formula: pi/4 = 4*arctan(1/5) - arctan(1/239).

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Pow, Zero};

fn arctan_inv(x: u32, scale: &BigInt) -> BigInt {
    let mut term = scale.div_floor(&BigInt::from(x));
    let mut total = term.clone();
    let x2 = BigInt::from(x) * x;
    let mut n: u64 = 1;
    let mut sign: i32 = -1;
    while !term.is_zero() {
        term = term.div_floor(&x2);
        total += term.div_floor(&BigInt::from(2 * n + 1)) * sign;
        sign = -sign;
        n += 1;
    }
    total
}

pub fn pi_digits(n: usize) -> String {
    let guard = 10;
    let scale = BigInt::from(10u32).pow((n + guard) as u32);
    let value: BigInt = (arctan_inv(5, &scale) * 4 - arctan_inv(239, &scale)) * 4;
    value.to_string().chars().take(n).collect()
}
