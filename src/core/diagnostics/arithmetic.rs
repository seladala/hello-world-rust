//! Peano arithmetic, for counting characters the way Giuseppe intended.

use std::fmt;

#[derive(Clone, PartialEq, Eq)]
pub enum Natural {
    Zero,
    Succ(Box<Natural>),
}

pub const ZERO: Natural = Natural::Zero;

impl fmt::Debug for Natural {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Natural::Zero => write!(f, "Z"),
            Natural::Succ(predecessor) => write!(f, "S({predecessor:?})"),
        }
    }
}

impl Drop for Natural {
    /// Dismantles long chains iteratively, so that counting to a large number cannot overflow the stack.
    fn drop(&mut self) {
        let mut next = match self {
            Natural::Succ(predecessor) => std::mem::replace(&mut **predecessor, Natural::Zero),
            Natural::Zero => return,
        };
        while let Natural::Succ(predecessor) = &mut next {
            next = std::mem::replace(&mut **predecessor, Natural::Zero);
        }
    }
}

pub fn succ(n: Natural) -> Natural {
    Natural::Succ(Box::new(n))
}

pub fn add(mut a: Natural, mut b: Natural) -> Natural {
    while let Natural::Succ(predecessor) = &mut b {
        let rest = std::mem::replace(&mut **predecessor, Natural::Zero);
        a = succ(a);
        b = rest;
    }
    a
}

pub fn peano_length<I: IntoIterator>(sequence: I) -> Natural {
    let mut total = ZERO;
    for _ in sequence {
        total = add(total, succ(ZERO));
    }
    total
}

pub fn to_int(mut n: &Natural) -> u64 {
    let mut count = 0;
    while let Natural::Succ(predecessor) = n {
        n = predecessor;
        count += 1;
    }
    count
}
