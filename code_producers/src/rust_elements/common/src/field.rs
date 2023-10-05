use ff::{PrimeField, PrimeFieldBits};

// Construct field element from possibly canonical bytes
fn from_le_bytes_canonical<F: PrimeFieldBits>(bs: &[u8]) -> F {
    let mut res = F::ZERO;
    let mut bs = bs.iter().rev().peekable();
    while let Some(b) = bs.next() {
        let b: F = (*b as u64).into();
        if bs.peek().is_none() {
            res.add_assign(b)
        } else {
            res.add_assign(b);
            res.mul_assign(F::from(256u64));
        }
    }
    res
}

/// A field element is defined to be negative if it is odd after doubling.
pub fn copy<F: PrimeField>(dest: &mut F, src: &F) {
    *dest = *src;
}

pub fn shl<F: PrimeFieldBits>(x: F, n: F) -> F {
    let n = to_u64(n).unwrap() as usize;
    let mut x_bits = x.to_le_bits();
    x_bits.shift_right(n);
    let mut x = vec![0; (x_bits.len() + 1) / 8];
    for (n, b) in x_bits.into_iter().enumerate() {
        let (byte_i, bit_i) = (n / 8, n % 8);
        if b {
            x[byte_i] += 1u8 << bit_i;
        }
    }
    from_le_bytes_canonical(&x)
}

/// this is disturbing
pub fn shr<F: PrimeFieldBits>(x: F, n: F) -> F {
    let n = to_u64(n).unwrap() as usize;
    let mut x_bits = x.to_le_bits();
    x_bits.shift_left(n); // well this is just dumb
    let mut x = vec![0; (x_bits.len() + 1) / 8];
    for (n, b) in x_bits.into_iter().enumerate() {
        let (byte_i, bit_i) = (n / 8, n % 8);
        if b {
            x[byte_i] += 1u8 << bit_i;
        }
    }
    from_le_bytes_canonical(&x)
}

pub fn bit_and<F: PrimeFieldBits>(x: F, y: F) -> F {
    let x_bits = x.to_le_bits();
    let y_bits = y.to_le_bits();
    let bits = x_bits & y_bits;
    let mut res = vec![0; (bits.len() + 1) / 8];
    for (n, b) in bits.into_iter().enumerate() {
        let (byte_i, bit_i) = (n / 8, n % 8);
        if b {
            res[byte_i] += 1u8 << bit_i;
        }
    }
    from_le_bytes_canonical(&res)
}

/// A field element is defined to be negative if it is odd after doubling.
pub fn is_negative<F: PrimeField>(f: F) -> bool {
    f.double().is_odd().into()
}

pub fn lt<F: PrimeField>(a: F, b: F) -> bool {
    is_negative(a - b)
}

pub fn is_true<F: PrimeField>(f: F) -> bool {
    f != F::ZERO
}

pub fn from_bool<F: PrimeField>(x: bool) -> F {
    if x {
        F::ONE
    } else {
        F::ZERO
    }
}

/// Attempts to convert the field element to a u64
pub fn to_u64<F: PrimeField>(f: F) -> Option<u64> {
    for x in &f.to_repr().as_ref()[8..] {
        if *x != 0 {
            return None;
        }
    }
    let mut byte_array = [0u8; 8];
    byte_array.copy_from_slice(&f.to_repr().as_ref()[0..8]);
    Some(u64::from_le_bytes(byte_array))
}
