//! Macierze 4x4 i wektory 3D/4D dla grafiki (konwencja OpenGL).
//!
//! Macierze są przechowywane **kolumnami** (column-major), tak jak w OpenGL:
//! element w wierszu `i` i kolumnie `j` ma indeks `i + 4*j` (patrz [`ind`]).
//! Wektor translacji leży więc w `a[12]`, `a[13]`, `a[14]`.
//!
//! Port z C. Różnice względem oryginału:
//! * funkcje zwracają wynik zamiast zapisywać go przez wskaźnik
//!   (dzięki temu nie ma problemu aliasowania argumentów),
//! * porażka (macierz osobliwa itp.) jest zwracana jako `Option::None`
//!   zamiast `char`/`bool`,
//! * poprawiono błąd w macierzy odwrotnej w [`m4x4_ortho`] (znak `ai[14]`),
//! * w [`m4x4_skew_frustum`] bez macierzy widoku przesunięcie ma ten sam znak,
//!   co w przypadku z macierzą widoku (w C był odwrotny).
#![allow(clippy::needless_range_loop)]

/// Macierz 4x4, kolumnami.
pub type M4 = [f32; 16];
/// Wektor 3D.
pub type V3 = [f32; 3];
/// Wektor 4D (współrzędne jednorodne).
pub type V4 = [f32; 4];

/// Indeks elementu (wiersz `i`, kolumna `j`) w macierzy zapisanej kolumnami.
#[inline(always)]
pub const fn ind(i: usize, j: usize) -> usize {
    i + 4 * j
}

/// Macierz jednostkowa.
pub const IDENTITY: M4 = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0,
];

// ////////////////////////////////////////////////////////////////////////////
// Macierze elementarnych przekształceń

pub fn m4x4_ident() -> M4 {
    IDENTITY
}

pub fn m4x4_translate(x: f32, y: f32, z: f32) -> M4 {
    let mut a = IDENTITY;
    a[12] = x;
    a[13] = y;
    a[14] = z;
    a
}

pub fn m4x4_scale(sx: f32, sy: f32, sz: f32) -> M4 {
    let mut a = IDENTITY;
    a[0] = sx;
    a[5] = sy;
    a[10] = sz;
    a
}

/// Obrót o kąt `phi` (radiany) wokół osi X.
pub fn m4x4_rotate_x(phi: f32) -> M4 {
    let (s, c) = phi.sin_cos();
    let mut a = IDENTITY;
    a[5] = c;
    a[10] = c;
    a[6] = s;
    a[9] = -s;
    a
}

/// Obrót o kąt `phi` (radiany) wokół osi Y.
pub fn m4x4_rotate_y(phi: f32) -> M4 {
    let (s, c) = phi.sin_cos();
    let mut a = IDENTITY;
    a[0] = c;
    a[10] = c;
    a[8] = s;
    a[2] = -s;
    a
}

/// Obrót o kąt `phi` (radiany) wokół osi Z.
pub fn m4x4_rotate_z(phi: f32) -> M4 {
    let (s, c) = phi.sin_cos();
    let mut a = IDENTITY;
    a[0] = c;
    a[5] = c;
    a[1] = s;
    a[4] = -s;
    a
}

/// Obrót o kąt `phi` wokół osi o kierunku `(x, y, z)` przechodzącej przez
/// początek układu. Oś nie musi być unormowana, ale musi być niezerowa
/// (dla zerowej wynik zawiera NaN — tak samo jak w C).
pub fn m4x4_rotate_v(x: f32, y: f32, z: f32, phi: f32) -> M4 {
    let l = 1.0 / (x * x + y * y + z * z).sqrt();
    let (x, y, z) = (x * l, y * l, z * l);
    let (s, c) = phi.sin_cos();
    let c1 = 1.0 - c;
    let mut a = IDENTITY;
    a[0] = x * x * c1 + c;
    a[5] = y * y * c1 + c;
    a[10] = z * z * c1 + c;
    let (xy, xz, yz) = (x * y * c1, x * z * c1, y * z * c1);
    a[1] = xy + s * z;
    a[4] = xy - s * z;
    a[2] = xz - s * y;
    a[8] = xz + s * y;
    a[6] = yz + s * x;
    a[9] = yz - s * x;
    a
}

// ////////////////////////////////////////////////////////////////////////////
// Domnażanie przekształceń z prawej strony: a := a * B

pub fn m4x4_mtranslate(a: &mut M4, x: f32, y: f32, z: f32) {
    *a = m4x4_mult(a, &m4x4_translate(x, y, z));
}

pub fn m4x4_mscale(a: &mut M4, sx: f32, sy: f32, sz: f32) {
    *a = m4x4_mult(a, &m4x4_scale(sx, sy, sz));
}

pub fn m4x4_mrotate_x(a: &mut M4, phi: f32) {
    *a = m4x4_mult(a, &m4x4_rotate_x(phi));
}

pub fn m4x4_mrotate_y(a: &mut M4, phi: f32) {
    *a = m4x4_mult(a, &m4x4_rotate_y(phi));
}

pub fn m4x4_mrotate_z(a: &mut M4, phi: f32) {
    *a = m4x4_mult(a, &m4x4_rotate_z(phi));
}

pub fn m4x4_mrotate_v(a: &mut M4, x: f32, y: f32, z: f32, phi: f32) {
    *a = m4x4_mult(a, &m4x4_rotate_v(x, y, z, phi));
}

// ////////////////////////////////////////////////////////////////////////////
// Algebra macierzy

/// Iloczyn `a * b`.
pub fn m4x4_mult(a: &M4, b: &M4) -> M4 {
    let mut ab = [0.0; 16];
    for i in 0..4 {
        for j in 0..4 {
            let mut s = a[ind(i, 0)] * b[ind(0, j)];
            for k in 1..4 {
                s += a[ind(i, k)] * b[ind(k, j)];
            }
            ab[ind(i, j)] = s;
        }
    }
    ab
}

/// Rozkład LU z częściowym wyborem elementu głównego: `P A = L U`.
///
/// Zwraca macierz `lu` (pod przekątną `L` bez jedynek na przekątnej,
/// na przekątnej i nad nią `U`) oraz tablicę przestawień `p`:
/// w kroku `j` zamieniono wiersze `j` i `p[j]`.
/// Zwraca `None`, gdy macierz jest osobliwa.
pub fn m4x4_lu_decomp(a: &M4) -> Option<(M4, [usize; 3])> {
    let mut lu = *a;
    let mut p = [0usize; 3];
    for j in 0..3 {
        let mut d = lu[ind(j, j)].abs();
        p[j] = j;
        for i in j + 1..4 {
            let t = lu[ind(i, j)].abs();
            if t > d {
                d = t;
                p[j] = i;
            }
        }
        if d == 0.0 {
            return None;
        }
        if p[j] != j {
            let i = p[j];
            for k in 0..4 {
                lu.swap(ind(i, k), ind(j, k));
            }
        }
        for i in j + 1..4 {
            lu[ind(i, j)] /= lu[ind(j, j)];
            let d = lu[ind(i, j)];
            for k in j + 1..4 {
                lu[ind(i, k)] -= d * lu[ind(j, k)];
            }
        }
    }
    if lu[15] != 0.0 { Some((lu, p)) } else { None }
}

/// Rozwiązuje `A x = v`, mając rozkład z [`m4x4_lu_decomp`].
pub fn m4x4_lu_solve(lu: &M4, p: &[usize; 3], v: &V4) -> V4 {
    let mut x = *v;
    for i in 0..3 {
        if p[i] != i {
            x.swap(i, p[i]);
        }
    }
    // L y = P v (L ma jedynki na przekątnej)
    for i in 1..4 {
        for j in 0..i {
            x[i] -= lu[ind(i, j)] * x[j];
        }
    }
    // U x = y
    for i in (0..4).rev() {
        for j in i + 1..4 {
            x[i] -= lu[ind(i, j)] * x[j];
        }
        x[i] /= lu[ind(i, i)];
    }
    x
}

/// Macierz odwrotna; `None`, gdy `a` jest osobliwa.
pub fn m4x4_invert(a: &M4) -> Option<M4> {
    let (lu, p) = m4x4_lu_decomp(a)?;
    let mut ai = [0.0; 16];
    for i in 0..4 {
        let mut e = [0.0; 4];
        e[i] = 1.0;
        let col = m4x4_lu_solve(&lu, &p, &e);
        ai[4 * i..4 * i + 4].copy_from_slice(&col);
    }
    Some(ai)
}

/// Rozwiązuje `Aᵀ x = v`, mając rozkład z [`m4x4_lu_decomp`]
/// (`Aᵀ = Uᵀ Lᵀ P`).
pub fn m4x4_ut_lt_solve(lu: &M4, p: &[usize; 3], v: &V4) -> V4 {
    let mut x = *v;
    // Uᵀ y = v
    for i in 0..4 {
        for j in 0..i {
            x[i] -= lu[ind(j, i)] * x[j];
        }
        x[i] /= lu[ind(i, i)];
    }
    // Lᵀ z = y
    for i in (0..3).rev() {
        for j in i + 1..4 {
            x[i] -= lu[ind(j, i)] * x[j];
        }
    }
    // x = Pᵀ z — przestawienia w odwrotnej kolejności
    for i in (0..3).rev() {
        if p[i] != i {
            x.swap(i, p[i]);
        }
    }
    x
}

/// Transpozycja macierzy odwrotnej, `(A⁻¹)ᵀ` (np. do przekształcania
/// wektorów normalnych); `None`, gdy `a` jest osobliwa.
pub fn m4x4_t_invert(a: &M4) -> Option<M4> {
    let (lu, p) = m4x4_lu_decomp(a)?;
    let mut ati = [0.0; 16];
    for i in 0..4 {
        let mut e = [0.0; 4];
        e[i] = 1.0;
        let col = m4x4_ut_lt_solve(&lu, &p, &e);
        ati[4 * i..4 * i + 4].copy_from_slice(&col);
    }
    Some(ati)
}

/// Odwrotność izometrii afinicznej `[R t; 0 1]` (R ortogonalna):
/// `[Rᵀ  -Rᵀt; 0 1]`. Funkcja nie sprawdza, czy `a` rzeczywiście jest izometrią.
pub fn m4x4_invert_affine_isometry(a: &M4) -> M4 {
    let mut ai = [0.0; 16];
    for i in 0..3 {
        for j in 0..3 {
            ai[ind(i, j)] = a[ind(j, i)];
        }
    }
    for i in 0..3 {
        ai[ind(i, 3)] = -(ai[ind(i, 0)] * a[ind(0, 3)]
            + ai[ind(i, 1)] * a[ind(1, 3)]
            + ai[ind(i, 2)] * a[ind(2, 3)]);
    }
    // ai[3], ai[7], ai[11] są już zerami
    ai[15] = 1.0;
    ai
}

pub fn m4x4_transpose(a: &M4) -> M4 {
    let mut at = [0.0; 16];
    for i in 0..4 {
        for j in 0..4 {
            at[ind(i, j)] = a[ind(j, i)];
        }
    }
    at
}

/// `A v`.
pub fn m4x4_mult_mv(a: &M4, v: &V4) -> V4 {
    let mut av = [0.0; 4];
    for i in 0..4 {
        let mut s = a[ind(i, 0)] * v[0];
        for j in 1..4 {
            s += a[ind(i, j)] * v[j];
        }
        av[i] = s;
    }
    av
}

/// `Aᵀ v`.
pub fn m4x4_mult_mtv(a: &M4, v: &V4) -> V4 {
    let mut av = [0.0; 4];
    for i in 0..4 {
        let mut s = a[ind(0, i)] * v[0];
        for j in 1..4 {
            s += a[ind(j, i)] * v[j];
        }
        av[i] = s;
    }
    av
}

// ////////////////////////////////////////////////////////////////////////////
// Rzutowania

/// Macierz rzutu perspektywicznego (jak `glFrustum`) i jej odwrotność.
/// Zwraca `(a, ai)`.
pub fn m4x4_frustum(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> (M4, M4) {
    let rl = right - left;
    let tb = top - bottom;
    let nf = near - far;
    let nn = near + near;

    let mut a = [0.0; 16];
    a[0] = nn / rl;
    a[8] = (right + left) / rl;
    a[5] = nn / tb;
    a[9] = (top + bottom) / tb;
    a[10] = (far + near) / nf;
    a[14] = far * nn / nf;
    a[11] = -1.0;

    let mut ai = [0.0; 16];
    ai[0] = rl / nn;
    ai[12] = (right + left) / nn;
    ai[5] = tb / nn;
    ai[13] = (top + bottom) / nn;
    ai[14] = -1.0;
    ai[11] = nf / (far * nn);
    ai[15] = (far + near) / (far * nn);

    (a, ai)
}

/// Macierz rzutu równoległego (jak `glOrtho`) i jej odwrotność.
/// Zwraca `(a, ai)`.
///
/// Uwaga: w oryginale w C było `ai[14] = 0.5*(far+near)`, co daje złą
/// odwrotność (`a * ai != I`); poprawna wartość to `-0.5*(far+near)`.
pub fn m4x4_ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> (M4, M4) {
    let rl = right - left;
    let tb = top - bottom;
    let nf = near - far;

    let mut a = [0.0; 16];
    a[0] = 2.0 / rl;
    a[12] = -(right + left) / rl;
    a[5] = 2.0 / tb;
    a[13] = -(top + bottom) / tb;
    a[10] = 2.0 / nf;
    a[14] = (far + near) / nf;
    a[15] = 1.0;

    let mut ai = [0.0; 16];
    ai[0] = 0.5 * rl;
    ai[12] = 0.5 * (right + left);
    ai[5] = 0.5 * tb;
    ai[13] = 0.5 * (top + bottom);
    ai[10] = 0.5 * nf;
    ai[14] = -0.5 * (far + near); // poprawione (w C: +0.5*(far+near))
    ai[15] = 1.0;

    (a, ai)
}

/// Wynik [`m4x4_skew_frustum`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkewFrustum {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    /// Macierz widoku z przesuniętym okiem: `T(-xv, -yv, 0) * vm`.
    pub shvm: M4,
    /// Położenie oka we współrzędnych świata (`shvm⁻¹ · (0,0,0,1)`);
    /// `None`, gdy `shvm` jest osobliwa.
    pub eyepos: Option<V4>,
    /// Macierz rzutu perspektywicznego.
    pub pm: M4,
    /// Macierz odwrotna do `pm`.
    pub pmi: M4,
}

/// Skośny ostrosłup widzenia (np. do stereoskopii albo śledzenia głowy).
///
/// * `w`, `h` – rozmiar okna w pikselach,
/// * `aspect` – stosunek szerokości do wysokości piksela,
/// * `f` – ogniskowa w mm dla kliszy 36x24 mm (przekątna kliszy / `f`
///   wyznacza kąt widzenia po przekątnej),
/// * `dist` – odległość oka od płaszczyzny ekranu,
/// * `xv`, `yv` – przesunięcie oka (w jednostkach sceny),
/// * `xp`, `yp` – przesunięcie obrazu w pikselach,
/// * `vm` – macierz widoku; `None` oznacza jednostkową.
///
/// Uwaga: w C dla `vm == NULL` liczono `shvm = T(+xv, +yv, 0)`, a dla
/// `vm != NULL` – `T(-xv, -yv, 0) * vm`. To niespójne (`vm = I` dawało
/// inny wynik niż `vm == NULL`), więc tu w obu przypadkach jest
/// `T(-xv, -yv, 0) * vm`. Ponadto `eyepos` liczone jest zawsze.
#[allow(clippy::too_many_arguments)]
pub fn m4x4_skew_frustum(
    w: u32,
    h: u32,
    aspect: f32,
    f: f32,
    dist: f32,
    xv: f32,
    yv: f32,
    xp: f32,
    yp: f32,
    near: f32,
    far: f32,
    vm: Option<&M4>,
) -> SkewFrustum {
    let (w, h) = (w as f32, h as f32);
    let df = (36.0f64 * 36.0 + 24.0 * 24.0).sqrt() as f32 / f;
    let awh = aspect * w / h;
    let tb = df * near / (1.0 + awh * awh).sqrt();
    let rl = tb * awh;

    let xr = xv * near / dist - xp * rl / w;
    let left = -0.5 * rl - xr;
    let right = 0.5 * rl - xr;

    let yr = yv * near / dist - yp * tb / h;
    let bottom = -0.5 * tb - yr;
    let top = 0.5 * tb - yr;

    let shift = m4x4_translate(-xv, -yv, 0.0);
    let shvm = match vm {
        Some(vm) => m4x4_mult(&shift, vm),
        None => shift,
    };
    let eyepos = m4x4_lu_decomp(&shvm).map(|(lu, p)| m4x4_lu_solve(&lu, &p, &[0.0, 0.0, 0.0, 1.0]));

    let (pm, pmi) = m4x4_frustum(left, right, bottom, top, near, far);
    SkewFrustum {
        left,
        right,
        bottom,
        top,
        shvm,
        eyepos,
        pm,
        pmi,
    }
}

// ////////////////////////////////////////////////////////////////////////////
// Wektory 4D

pub fn v4_add(v1: &V4, v2: &V4) -> V4 {
    [v1[0] + v2[0], v1[1] + v2[1], v1[2] + v2[2], v1[3] + v2[3]]
}

pub fn v4_subtract(v1: &V4, v2: &V4) -> V4 {
    [v1[0] - v2[0], v1[1] - v2[1], v1[2] - v2[2], v1[3] - v2[3]]
}

pub fn v4_dot_product(v1: &V4, v2: &V4) -> f32 {
    v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2] + v1[3] * v2[3]
}

/// Uogólniony iloczyn wektorowy w R⁴: wektor prostopadły do `v1`, `v2`, `v3`.
pub fn v4_cross_product(v1: &V4, v2: &V4, v3: &V4) -> V4 {
    let a01 = v1[0] * v2[1] - v1[1] * v2[0];
    let a02 = v1[0] * v2[2] - v1[2] * v2[0];
    let a03 = v1[0] * v2[3] - v1[3] * v2[0];
    let a12 = v1[1] * v2[2] - v1[2] * v2[1];
    let a13 = v1[1] * v2[3] - v1[3] * v2[1];
    let a23 = v1[2] * v2[3] - v1[3] * v2[2];
    [
        -a23 * v3[1] + a13 * v3[2] - a12 * v3[3],
        a23 * v3[0] - a03 * v3[2] + a02 * v3[3],
        -a13 * v3[0] + a03 * v3[1] - a01 * v3[3],
        a12 * v3[0] - a02 * v3[1] + a01 * v3[2],
    ]
}

/// Normalizuje `v` w miejscu (o ile nie jest zerowy); zwraca jego
/// pierwotną długość.
pub fn v4_normalise(v: &mut V4) -> f32 {
    let s = v4_dot_product(v, v).sqrt();
    if s > 0.0 {
        for x in v.iter_mut() {
            *x /= s;
        }
    }
    s
}

// ////////////////////////////////////////////////////////////////////////////
// Wektory 3D

pub fn v3_add(v1: &V3, v2: &V3) -> V3 {
    [v1[0] + v2[0], v1[1] + v2[1], v1[2] + v2[2]]
}

pub fn v3_subtract(v1: &V3, v2: &V3) -> V3 {
    [v1[0] - v2[0], v1[1] - v2[1], v1[2] - v2[2]]
}

pub fn v3_dot_product(v1: &V3, v2: &V3) -> f32 {
    v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]
}

pub fn v3_cross_product(v1: &V3, v2: &V3) -> V3 {
    [
        v1[1] * v2[2] - v1[2] * v2[1],
        v1[2] * v2[0] - v1[0] * v2[2],
        v1[0] * v2[1] - v1[1] * v2[0],
    ]
}

/// Złożenie obrotów: najpierw o `phi1` wokół `v1`, potem o `phi2` wokół `v2`
/// (osie muszą być unormowane). Zwraca `(oś, kąt)` obrotu wypadkowego
/// (liczone przez mnożenie kwaternionów `q2 * q1`). Dla obrotu
/// tożsamościowego zwraca `([1, 0, 0], 0)`.
pub fn v3_comp_rotations(v2: &V3, phi2: f32, v1: &V3, phi1: f32) -> (V3, f32) {
    let (s2, c2) = (0.5 * phi2).sin_cos();
    let (s1, c1) = (0.5 * phi1).sin_cos();
    let v2v1 = v3_dot_product(v2, v1);
    let v2xv1 = v3_cross_product(v2, v1);
    let c = c2 * c1 - v2v1 * s2 * s1;
    let mut v = [0.0; 3];
    for i in 0..3 {
        v[i] = v2[i] * s2 * c1 + v1[i] * s1 * c2 + v2xv1[i] * s2 * s1;
    }
    let s = v3_dot_product(&v, &v).sqrt();
    if s > 0.0 {
        for x in v.iter_mut() {
            *x /= s;
        }
        (v, 2.0 * s.atan2(c))
    } else {
        ([1.0, 0.0, 0.0], 0.0)
    }
}

/// Normalizuje `v` w miejscu (o ile nie jest zerowy); zwraca jego
/// pierwotną długość.
pub fn v3_normalise(v: &mut V3) -> f32 {
    let s = v3_dot_product(v, v).sqrt();
    if s > 0.0 {
        for x in v.iter_mut() {
            *x /= s;
        }
    }
    s
}

// ////////////////////////////////////////////////////////////////////////////
// Przekształcenia afiniczne w 3D

/// Przekształca **wektor** 3D (bez translacji) – lewy górny blok 3x3 razy `v`.
pub fn m4x4_mult_mv3(a: &M4, v: &V3) -> V3 {
    let mut av = [0.0; 3];
    for i in 0..3 {
        let mut s = a[ind(i, 0)] * v[0];
        for j in 1..3 {
            s += a[ind(i, j)] * v[j];
        }
        av[i] = s;
    }
    av
}

/// Przekształca **punkt** 3D (z translacją, bez dzielenia perspektywicznego).
pub fn m4x4_mult_mp3(a: &M4, p: &V3) -> V3 {
    let mut ap = [0.0; 3];
    for i in 0..3 {
        let mut s = a[ind(i, 0)] * p[0] + a[ind(i, 3)];
        for j in 1..3 {
            s += a[ind(i, j)] * p[j];
        }
        ap[i] = s;
    }
    ap
}

/// Odbicie punktu `q` względem płaszczyzny przechodzącej przez `p`
/// o wektorze normalnym `nv` (nie musi być unormowany, musi być niezerowy).
pub fn v3_reflect_point(p: &V3, nv: &V3, q: &V3) -> V3 {
    let qp = v3_subtract(q, p);
    let g = 2.0 * v3_dot_product(nv, &qp) / v3_dot_product(nv, nv);
    [q[0] - g * nv[0], q[1] - g * nv[1], q[2] - g * nv[2]]
}

/// Obrót o kąt `phi` wokół prostej przechodzącej przez punkt `p`
/// o kierunku `v`: `T(p) * R(v, phi) * T(-p)`.
pub fn m4x4_rotate_pv(p: &V3, v: &V3, phi: f32) -> M4 {
    let b = m4x4_rotate_v(v[0], v[1], v[2], phi);
    let c = m4x4_translate(-p[0], -p[1], -p[2]);
    let mut a = m4x4_mult(&b, &c);
    a[12] += p[0];
    a[13] += p[1];
    a[14] += p[2];
    a
}

/// Obrót przeprowadzający kierunek `v1` na kierunek `v2` (wokół osi
/// `v1 × v2`), opcjonalnie wokół punktu `p`.
///
/// Zwraca `None`, gdy `v1` i `v2` są równoległe (także przeciwnie
/// skierowane) lub któryś jest zerowy – wtedy oś obrotu jest nieokreślona.
pub fn m4x4_rotate_p2v(p: Option<&V3>, v1: &V3, v2: &V3) -> Option<M4> {
    let v = v3_cross_product(v1, v2);
    let sa = v3_dot_product(&v, &v).sqrt();
    if sa > 0.0 {
        let ca = v3_dot_product(v1, v2);
        let ang = sa.atan2(ca);
        let mut a = m4x4_rotate_v(v[0], v[1], v[2], ang);
        if let Some(p) = p {
            let c = m4x4_mult(&a, &m4x4_translate(-p[0], -p[1], -p[2]));
            a = m4x4_mult(&m4x4_translate(p[0], p[1], p[2]), &c);
        }
        Some(a)
    } else {
        None
    }
}

// ////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn close(a: &[f32], b: &[f32], eps: f32) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= eps)
    }

    #[track_caller]
    fn assert_close(a: &[f32], b: &[f32]) {
        assert!(close(a, b, EPS), "\n left: {a:?}\nright: {b:?}");
    }

    fn sample() -> M4 {
        let mut a = m4x4_rotate_v(1.0, 2.0, -0.5, 0.7);
        m4x4_mtranslate(&mut a, 1.0, -2.0, 3.0);
        m4x4_mscale(&mut a, 2.0, 0.5, 1.5);
        a[3] = 0.3; // żeby nie była afiniczna
        a[7] = -0.2;
        a
    }

    #[test]
    fn rotate_v_matches_axis_rotations() {
        let phi = 0.83;
        assert_close(&m4x4_rotate_v(1.0, 0.0, 0.0, phi), &m4x4_rotate_x(phi));
        assert_close(&m4x4_rotate_v(0.0, 2.0, 0.0, phi), &m4x4_rotate_y(phi));
        assert_close(&m4x4_rotate_v(0.0, 0.0, 3.0, phi), &m4x4_rotate_z(phi));
    }

    #[test]
    fn rotation_directions_right_handed() {
        let h = std::f32::consts::FRAC_PI_2;
        // Rz(90°): x -> y
        assert_close(
            &m4x4_mult_mv3(&m4x4_rotate_z(h), &[1.0, 0.0, 0.0]),
            &[0.0, 1.0, 0.0],
        );
        // Rx(90°): y -> z
        assert_close(
            &m4x4_mult_mv3(&m4x4_rotate_x(h), &[0.0, 1.0, 0.0]),
            &[0.0, 0.0, 1.0],
        );
        // Ry(90°): z -> x
        assert_close(
            &m4x4_mult_mv3(&m4x4_rotate_y(h), &[0.0, 0.0, 1.0]),
            &[1.0, 0.0, 0.0],
        );
    }

    #[test]
    fn translate_column_major() {
        let t = m4x4_translate(1.0, 2.0, 3.0);
        assert_close(&m4x4_mult_mp3(&t, &[0.0, 0.0, 0.0]), &[1.0, 2.0, 3.0]);
        assert_close(&m4x4_mult_mv3(&t, &[1.0, 1.0, 1.0]), &[1.0, 1.0, 1.0]);
    }

    #[test]
    fn invert_and_t_invert() {
        let a = sample();
        let ai = m4x4_invert(&a).unwrap();
        assert_close(&m4x4_mult(&a, &ai), &IDENTITY);
        assert_close(&m4x4_mult(&ai, &a), &IDENTITY);
        let ati = m4x4_t_invert(&a).unwrap();
        assert_close(&ati, &m4x4_transpose(&ai));
    }

    #[test]
    fn invert_needs_pivoting() {
        // a[0][0] = 0 — bez wyboru elementu głównego się nie da
        let a: M4 = [
            0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let ai = m4x4_invert(&a).unwrap();
        assert_close(&m4x4_mult(&a, &ai), &IDENTITY);
        let ati = m4x4_t_invert(&a).unwrap();
        assert_close(&ati, &m4x4_transpose(&ai));
    }

    #[test]
    fn singular_returns_none() {
        let mut a = sample();
        for i in 0..4 {
            a[ind(i, 3)] = a[ind(i, 1)]; // dwie równe kolumny
        }
        assert!(m4x4_invert(&a).is_none());
        assert!(m4x4_t_invert(&a).is_none());
        assert!(m4x4_invert(&[0.0; 16]).is_none());
    }

    #[test]
    fn lu_solve_and_transposed_solve() {
        let a = sample();
        let v = [1.0, -2.0, 0.5, 3.0];
        let (lu, p) = m4x4_lu_decomp(&a).unwrap();
        let x = m4x4_lu_solve(&lu, &p, &v);
        assert_close(&m4x4_mult_mv(&a, &x), &v);
        let y = m4x4_ut_lt_solve(&lu, &p, &v);
        assert_close(&m4x4_mult_mtv(&a, &y), &v);
    }

    #[test]
    fn affine_isometry_inverse() {
        let mut a = m4x4_rotate_v(-1.0, 0.3, 2.0, 1.1);
        m4x4_mtranslate(&mut a, 4.0, -1.0, 2.5);
        let ai = m4x4_invert_affine_isometry(&a);
        assert_close(&ai, &m4x4_invert(&a).unwrap());
    }

    #[test]
    fn mult_mtv_is_transpose() {
        let a = sample();
        let v = [0.3, 1.0, -4.0, 2.0];
        assert_close(
            &m4x4_mult_mtv(&a, &v),
            &m4x4_mult_mv(&m4x4_transpose(&a), &v),
        );
    }

    #[test]
    fn frustum_inverse() {
        let (a, ai) = m4x4_frustum(-1.0, 2.0, -0.5, 1.5, 0.5, 20.0);
        assert_close(&m4x4_mult(&a, &ai), &IDENTITY);
        // punkt na bliskiej płaszczyźnie -> z_ndc = -1
        let p = m4x4_mult_mv(&a, &[0.0, 0.0, -0.5, 1.0]);
        assert!((p[2] / p[3] + 1.0).abs() < EPS);
    }

    #[test]
    fn ortho_inverse() {
        let (a, ai) = m4x4_ortho(-1.0, 2.0, -0.5, 1.5, 0.5, 20.0);
        assert_close(&m4x4_mult(&a, &ai), &IDENTITY);
        let p = m4x4_mult_mv(&a, &[0.0, 0.0, -20.0, 1.0]);
        assert!((p[2] - 1.0).abs() < EPS);
    }

    #[test]
    fn skew_frustum() {
        let vm = m4x4_mult(&m4x4_rotate_y(0.4), &m4x4_translate(1.0, 2.0, -3.0));
        let sf = m4x4_skew_frustum(
            800,
            600,
            1.0,
            50.0,
            2.0,
            0.1,
            -0.05,
            3.0,
            4.0,
            0.5,
            100.0,
            Some(&vm),
        );
        assert_close(&m4x4_mult(&sf.pm, &sf.pmi), &IDENTITY);
        // oko jest w początku układu po przekształceniu shvm
        let e = sf.eyepos.unwrap();
        assert_close(&m4x4_mult_mv(&sf.shvm, &e), &[0.0, 0.0, 0.0, 1.0]);
        // vm = None to to samo co vm = I
        let s1 = m4x4_skew_frustum(
            800, 600, 1.0, 50.0, 2.0, 0.1, -0.05, 3.0, 4.0, 0.5, 100.0, None,
        );
        let s2 = m4x4_skew_frustum(
            800,
            600,
            1.0,
            50.0,
            2.0,
            0.1,
            -0.05,
            3.0,
            4.0,
            0.5,
            100.0,
            Some(&IDENTITY),
        );
        assert_eq!(s1, s2);
        assert_close(&s1.eyepos.unwrap(), &[0.1, -0.05, 0.0, 1.0]);
    }

    #[test]
    fn cross_products() {
        let a = [1.0, 2.0, 3.0];
        let b = [-1.0, 0.5, 2.0];
        let c = v3_cross_product(&a, &b);
        assert!(v3_dot_product(&a, &c).abs() < EPS && v3_dot_product(&b, &c).abs() < EPS);
        assert_close(
            &v3_cross_product(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]),
            &[0.0, 0.0, 1.0],
        );

        let v1 = [1.0, 2.0, 0.0, -1.0];
        let v2 = [0.0, 1.0, 3.0, 1.0];
        let v3 = [2.0, -1.0, 1.0, 0.5];
        let w = v4_cross_product(&v1, &v2, &v3);
        for v in [&v1, &v2, &v3] {
            assert!(v4_dot_product(v, &w).abs() < EPS);
        }
        assert!(v4_dot_product(&w, &w) > 1.0);
    }

    #[test]
    fn normalise() {
        let mut v = [3.0, 0.0, 4.0];
        assert!((v3_normalise(&mut v) - 5.0).abs() < EPS);
        assert_close(&v, &[0.6, 0.0, 0.8]);
        let mut z = [0.0; 4];
        assert_eq!(v4_normalise(&mut z), 0.0);
        assert_eq!(z, [0.0; 4]);
    }

    #[test]
    fn comp_rotations_matches_matrix_product() {
        let mut v1 = [1.0, 2.0, -1.0];
        let mut v2 = [0.0, -1.0, 3.0];
        v3_normalise(&mut v1);
        v3_normalise(&mut v2);
        let (phi1, phi2) = (0.9, -1.3);
        let (v, phi) = v3_comp_rotations(&v2, phi2, &v1, phi1);
        let r = m4x4_mult(
            &m4x4_rotate_v(v2[0], v2[1], v2[2], phi2),
            &m4x4_rotate_v(v1[0], v1[1], v1[2], phi1),
        );
        assert_close(&m4x4_rotate_v(v[0], v[1], v[2], phi), &r);
        // obrót i obrót odwrotny -> tożsamość
        let (v, phi) = v3_comp_rotations(&v1, -phi1, &v1, phi1);
        assert_eq!((v, phi), ([1.0, 0.0, 0.0], 0.0));
    }

    #[test]
    fn reflect_point() {
        let r = v3_reflect_point(&[0.0, 1.0, 0.0], &[0.0, 2.0, 0.0], &[3.0, 4.0, 5.0]);
        assert_close(&r, &[3.0, -2.0, 5.0]);
    }

    #[test]
    fn rotate_about_point() {
        let p = [1.0, 2.0, 3.0];
        let a = m4x4_rotate_pv(&p, &[0.3, -1.0, 0.7], 1.2);
        assert_close(&m4x4_mult_mp3(&a, &p), &p); // punkt na osi jest stały
        let expected = m4x4_mult(
            &m4x4_translate(1.0, 2.0, 3.0),
            &m4x4_mult(
                &m4x4_rotate_v(0.3, -1.0, 0.7, 1.2),
                &m4x4_translate(-1.0, -2.0, -3.0),
            ),
        );
        assert_close(&a, &expected);
    }

    #[test]
    fn rotate_p2v() {
        let v1 = [1.0, 2.0, 0.5];
        let v2 = [-2.0, 0.0, 4.0];
        let p = [0.5, -1.0, 2.0];
        let a = m4x4_rotate_p2v(Some(&p), &v1, &v2).unwrap();
        // p + v1 przechodzi na p + t*v2, t = |v1|/|v2|
        let t = (v3_dot_product(&v1, &v1) / v3_dot_product(&v2, &v2)).sqrt();
        let q = m4x4_mult_mp3(&a, &v3_add(&p, &v1));
        assert_close(&q, &[p[0] + t * v2[0], p[1] + t * v2[1], p[2] + t * v2[2]]);
        assert_close(&m4x4_mult_mp3(&a, &p), &p);
        // równoległe -> None
        assert!(m4x4_rotate_p2v(None, &v1, &[2.0, 4.0, 1.0]).is_none());
        assert!(m4x4_rotate_p2v(None, &v1, &[-1.0, -2.0, -0.5]).is_none());
    }
}
