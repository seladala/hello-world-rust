// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! Gibbons' unbounded spigot: streams digits of pi one at a time, forever.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::ToPrimitive;

pub struct Spigot {
    q: BigInt,
    r: BigInt,
    t: BigInt,
    k: BigInt,
    n: BigInt,
    l: BigInt,
}

pub fn stream() -> Spigot {
    let int = |v: i32| BigInt::from(v);
    Spigot { q: int(1), r: int(0), t: int(1), k: int(1), n: int(3), l: int(3) }
}

impl Iterator for Spigot {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        loop {
            let Spigot { q, r, t, k, n, l } = self;
            if &(&*q * 4u32) + &*r - &*t < &*n * &*t {
                let digit = n.to_u8().expect("pi digits are digits");
                let new_n = (((&*q * 3u32) + &*r) * 10u32).div_floor(t) - &*n * 10u32;
                let new_r = (&*r - &*n * &*t) * 10u32;
                *q *= 10u32;
                *r = new_r;
                *n = new_n;
                return Some(digit);
            }
            let new_q = &*q * &*k;
            let new_r = ((&*q * 2u32) + &*r) * &*l;
            let new_t = &*t * &*l;
            let new_n = ((&*q * (&*k * 7u32 + 2u32)) + &*r * &*l).div_floor(&(&*t * &*l));
            *k += 1u32;
            *l += 2u32;
            *q = new_q;
            *r = new_r;
            *t = new_t;
            *n = new_n;
        }
    }
}

pub fn pi_digits(n: usize) -> String {
    stream().take(n).map(|d| char::from(b'0' + d)).collect()
}
