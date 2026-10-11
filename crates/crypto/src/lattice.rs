//! Lattice reduction with exact integer arithmetic.
//!
//! [`lll`] is the integral LLL algorithm (Cohen, *A Course in Computational
//! Algebraic Number Theory*, Algorithm 2.6.7, after de Weger). Instead of the
//! rational Gram–Schmidt coefficients mu_{i,j} it keeps the integers
//!   d_i       = det(Gram matrix of b_1..b_i)          (so |b_i*|^2 = d_i / d_{i-1})
//!   lam_{i,j} = d_j * mu_{i,j}
//! and every division it performs is exact. No floating point, no surprises.

use num_traits::{Signed, ToPrimitive, Zero};

use crate::bytes::{Big, big, log2_big, round_div};

pub type Vector = Vec<Big>;
pub type Basis = Vec<Vector>;

pub fn dot(u: &[Big], v: &[Big]) -> Big {
    u.iter().zip(v).fold(Big::zero(), |acc, (x, y)| acc + x * y)
}

/// u - q*v
pub fn sub(u: &[Big], v: &[Big], q: &Big) -> Vector {
    u.iter().zip(v).map(|(x, y)| x - q * y).collect()
}

pub fn norm2(u: &[Big]) -> Big {
    dot(u, u)
}

/// Gram determinants d_0..d_n of a basis (d_0 = 1), via the same exact recursion LLL uses.
pub fn gram_determinants(basis: &Basis) -> Vec<Big> {
    let n = basis.len();
    let mut d = vec![Big::zero(); n + 1];
    d[0] = big(1);
    let mut lam = vec![vec![Big::zero(); n + 1]; n + 1];
    for k in 1..=n {
        for j in 1..=k {
            let mut u = dot(&basis[k - 1], &basis[j - 1]);
            for i in 1..j {
                u = (&d[i] * u - &lam[k][i] * &lam[j][i]) / &d[i - 1];
            }
            if j < k {
                lam[k][j] = u;
            } else {
                d[k] = u;
            }
        }
    }
    d
}

/// log2 |b_i*| for i = 1..n: the "profile" of a basis. Reduction flattens it.
pub fn profile_from_dets(d: &[Big]) -> Vec<f64> {
    (1..d.len()).map(|i| (log2_big(&d[i]) - log2_big(&d[i - 1])) / 2.0).collect()
}

pub fn profile(basis: &Basis) -> Vec<f64> {
    profile_from_dets(&gram_determinants(basis))
}

#[derive(Clone, Debug)]
pub struct LllTraceFrame {
    pub swaps: usize,
    pub reductions: usize,
    pub k: usize,
    pub profile: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct LllResult {
    pub basis: Basis,
    pub swaps: usize,
    pub reductions: usize,
    pub profile_before: Vec<f64>,
    pub profile_after: Vec<f64>,
    pub trace: Vec<LllTraceFrame>,
}

/// LLL-reduce the rows of `basis` with parameter delta = num/den (default 99/100).
/// Rows must be linearly independent.
pub fn lll(basis: &Basis, delta: (i64, i64), trace_every: usize) -> Result<LllResult, String> {
    let n = basis.len();
    let (dn, dd) = (big(delta.0), big(delta.1));
    let mut b: Vec<Vector> = std::iter::once(Vec::new()).chain(basis.iter().cloned()).collect();
    let mut d = vec![Big::zero(); n + 1];
    let mut lam = vec![vec![Big::zero(); n + 1]; n + 1];
    let profile_before = profile(basis);
    let mut trace = Vec::new();
    let mut swaps = 0usize;
    let mut reductions = 0usize;
    let trace_every = trace_every.max(1);

    let current_profile = |d: &[Big], kmax: usize| -> Vec<f64> {
        let mut p = profile_before.clone();
        for i in 1..=kmax {
            p[i - 1] = (log2_big(&d[i]) - log2_big(&d[i - 1])) / 2.0;
        }
        p
    };

    fn redi(b: &mut [Vector], d: &[Big], lam: &mut [Vec<Big>], k: usize, l: usize, reductions: &mut usize) {
        if big(2) * lam[k][l].abs() > d[l] {
            let q = round_div(&lam[k][l], &d[l]);
            b[k] = sub(&b[k], &b[l], &q);
            lam[k][l] = &lam[k][l] - &q * &d[l];
            for i in 1..l {
                let v = &lam[k][i] - &q * &lam[l][i];
                lam[k][i] = v;
            }
            *reductions += 1;
        }
    }

    d[0] = big(1);
    if n >= 1 {
        d[1] = dot(&b[1], &b[1]);
    }
    let mut kmax = 1usize;
    let mut k = 2usize;
    while k <= n {
        if k > kmax {
            kmax = k;
            for j in 1..=k {
                let mut u = dot(&b[k], &b[j]);
                for i in 1..j {
                    u = (&d[i] * u - &lam[k][i] * &lam[j][i]) / &d[i - 1];
                }
                if j < k {
                    lam[k][j] = u;
                } else {
                    if u.is_zero() {
                        return Err("basis vectors are linearly dependent".into());
                    }
                    d[k] = u;
                }
            }
        }
        loop {
            redi(&mut b, &d, &mut lam, k, k - 1, &mut reductions);
            let l = lam[k][k - 1].clone();
            if &dd * &d[k] * &d[k - 2] < &dn * &d[k - 1] * &d[k - 1] - &dd * &l * &l {
                // swap k and k-1
                b.swap(k, k - 1);
                for j in 1..k.saturating_sub(1) {
                    let t = lam[k][j].clone();
                    lam[k][j] = lam[k - 1][j].clone();
                    lam[k - 1][j] = t;
                }
                let bb = (&d[k - 2] * &d[k] + &l * &l) / &d[k - 1];
                for i in k + 1..=kmax {
                    let t = lam[i][k].clone();
                    lam[i][k] = (&d[k] * &lam[i][k - 1] - &l * &t) / &d[k - 1];
                    lam[i][k - 1] = (&bb * &t + &l * &lam[i][k]) / &d[k];
                }
                d[k - 1] = bb;
                swaps += 1;
                if swaps % trace_every == 0 {
                    trace.push(LllTraceFrame { swaps, reductions, k, profile: current_profile(&d, kmax) });
                }
                k = (k - 1).max(2);
                continue;
            }
            for l in (1..k - 1).rev() {
                redi(&mut b, &d, &mut lam, k, l, &mut reductions);
            }
            k += 1;
            break;
        }
    }
    let out: Basis = b.into_iter().skip(1).collect();
    let profile_after = profile_from_dets(&d);
    trace.push(LllTraceFrame { swaps, reductions, k: n, profile: profile_after.clone() });
    Ok(LllResult { basis: out, swaps, reductions, profile_before, profile_after, trace })
}

/// Check the two LLL conditions (size reduction and Lovász) exactly.
pub fn is_lll_reduced(basis: &Basis, delta: (i64, i64)) -> bool {
    let n = basis.len();
    let mut d = vec![Big::zero(); n + 1];
    d[0] = big(1);
    let mut lam = vec![vec![Big::zero(); n + 1]; n + 1];
    for k in 1..=n {
        for j in 1..=k {
            let mut u = dot(&basis[k - 1], &basis[j - 1]);
            for i in 1..j {
                u = (&d[i] * u - &lam[k][i] * &lam[j][i]) / &d[i - 1];
            }
            if j < k {
                lam[k][j] = u;
            } else {
                d[k] = u;
            }
        }
    }
    for k in 2..=n {
        for j in 1..k {
            if big(2) * lam[k][j].abs() > d[j] {
                return false;
            }
        }
        let l = &lam[k][k - 1];
        if big(delta.1) * &d[k] * &d[k - 2] < big(delta.0) * &d[k - 1] * &d[k - 1] - big(delta.1) * l * l {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Two dimensions: Lagrange–Gauss reduction, one move at a time.
// ---------------------------------------------------------------------------

pub type V2 = [i64; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gauss2 {
    pub b1: V2,
    pub b2: V2,
}

pub struct Mu2 {
    pub num: i64,
    pub den: i64,
    pub value: f64,
    pub nearest: i64,
}

pub fn n2(v: V2) -> i64 {
    v[0] * v[0] + v[1] * v[1]
}

impl Gauss2 {
    /// mu = <b1,b2>/<b1,b1> as an exact fraction and as a float for display.
    pub fn mu(&self) -> Mu2 {
        let num = self.b1[0] * self.b2[0] + self.b1[1] * self.b2[1];
        let den = n2(self.b1);
        let nearest = round_div(&big(num), &big(den)).to_i64().unwrap();
        Mu2 { num, den, value: num as f64 / den as f64, nearest }
    }

    pub fn step(&self, k: i64) -> Self {
        Self { b1: self.b1, b2: [self.b2[0] - k * self.b1[0], self.b2[1] - k * self.b1[1]] }
    }

    pub fn swap(&self) -> Self {
        Self { b1: self.b2, b2: self.b1 }
    }

    pub fn det(&self) -> i64 {
        self.b1[0] * self.b2[1] - self.b1[1] * self.b2[0]
    }

    /// Reduced: |mu| <= 1/2 and |b1| <= |b2|. Then b1 is a shortest nonzero vector.
    pub fn is_reduced(&self) -> bool {
        let m = self.mu();
        2 * m.num.abs() <= m.den && n2(self.b1) <= n2(self.b2)
    }

    /// Full Lagrange–Gauss reduction (used to check the player's answer).
    pub fn reduce(&self) -> (Self, usize) {
        let mut s = if n2(self.b1) <= n2(self.b2) { *self } else { self.swap() };
        let mut steps = 0;
        loop {
            s = s.step(s.mu().nearest);
            steps += 1;
            if n2(s.b2) >= n2(s.b1) {
                return (s, steps);
            }
            s = s.swap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bv(rows: &[&[i64]]) -> Basis {
        rows.iter().map(|r| r.iter().map(|&x| big(x)).collect()).collect()
    }

    #[test]
    fn lll_textbook_example() {
        // Cohen's / Wikipedia's example: reduces to small vectors and is LLL-reduced.
        let b = bv(&[&[1, 1, 1], &[-1, 0, 2], &[3, 5, 6]]);
        let r = lll(&b, (3, 4), 1).unwrap();
        assert!(is_lll_reduced(&r.basis, (3, 4)));
        assert_eq!(r.basis, bv(&[&[0, 1, 0], &[1, 0, 1], &[-1, 0, 2]]));
        assert!(!is_lll_reduced(&b, (3, 4)));
        let dets_before = gram_determinants(&b);
        let dets_after = gram_determinants(&r.basis);
        assert_eq!(dets_before[3], dets_after[3]); // same lattice volume
    }

    #[test]
    fn gauss_reduce_finds_short_basis() {
        let s = Gauss2 { b1: [90, 123], b2: [56, 76] };
        let (r, _) = s.reduce();
        assert!(r.is_reduced());
        assert_eq!(r.det().abs(), s.det().abs());
        // b1 is a shortest nonzero vector of the lattice: check by brute force
        let shortest = (-60i64..=60)
            .flat_map(|i| (-60i64..=60).map(move |j| (i, j)))
            .filter(|&ij| ij != (0, 0))
            .map(|(i, j)| n2([i * s.b1[0] + j * s.b2[0], i * s.b1[1] + j * s.b2[1]]))
            .min()
            .unwrap();
        assert_eq!(n2(r.b1), shortest);
    }
}
