//! The maths of a corner pin: the matrix that takes the unit square to four corners.

pub type Matrix = [[f32; 3]; 3];

pub const IDENTITY: Matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Corners further out than this (as fractions of the screen) are pulled back in.
const LIMIT: (f32, f32) = (-1.0, 2.0);
/// A quad smaller than this (in screen areas) counts as collapsed.
const MIN_AREA: f64 = 1e-4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Degenerate {
    /// A coordinate is NaN or infinite.
    NotFinite,
    /// The four points collapse to a line or a point.
    Collapsed,
    /// An inner angle is 180 degrees or more, or the corners cross: a pin needs a convex quad.
    NotConvex,
}

/// Pulls every coordinate into a sane range; NaN passes through and is caught by `check`.
pub fn clamp_corners(corners: [[f32; 2]; 4]) -> [[f32; 2]; 4] {
    corners.map(|[x, y]| [x.clamp(LIMIT.0, LIMIT.1), y.clamp(LIMIT.0, LIMIT.1)])
}

/// Whether four corners make a quad a picture can be pinned to.
pub fn check(corners: [[f32; 2]; 4]) -> Result<(), Degenerate> {
    if corners.iter().flatten().any(|v| !v.is_finite()) {
        return Err(Degenerate::NotFinite);
    }
    let p = corners.map(|[x, y]| [x as f64, y as f64]);
    let mut sign = 0.0_f64;
    let mut area = 0.0_f64;
    for i in 0..4 {
        let (a, b, c) = (p[i], p[(i + 1) % 4], p[(i + 2) % 4]);
        let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
        area += a[0] * b[1] - b[0] * a[1];
        if cross == 0.0 {
            // A straight or doubled edge.
            return Err(Degenerate::Collapsed);
        }
        if sign != 0.0 && cross.signum() != sign {
            return Err(Degenerate::NotConvex);
        }
        sign = cross.signum();
    }
    if (area / 2.0).abs() < MIN_AREA { Err(Degenerate::Collapsed) } else { Ok(()) }
}

/// The matrix taking the unit square (0,0) (1,0) (1,1) (0,1) to `corners`: top left, top
/// right, bottom right, bottom left. A point `(u, v)` lands at `(x/w, y/w)` where
/// `[x, y, w] = M * [u, v, 1]`. Corners are clamped to a sane range first; corners that still
/// make no convex quad give the identity, so a bad setting leaves the picture alone.
pub fn homography(corners: [[f32; 2]; 4]) -> Matrix {
    try_homography(corners).unwrap_or(IDENTITY)
}

/// `homography`, saying why when the corners cannot be pinned.
pub fn try_homography(corners: [[f32; 2]; 4]) -> Result<Matrix, Degenerate> {
    let corners = clamp_corners(corners);
    check(corners)?;
    let [[x0, y0], [x1, y1], [x2, y2], [x3, y3]] = corners.map(|[x, y]| [x as f64, y as f64]);
    let (dx1, dx2, dx3) = (x1 - x2, x3 - x2, x0 - x1 + x2 - x3);
    let (dy1, dy2, dy3) = (y1 - y2, y3 - y2, y0 - y1 + y2 - y3);
    let (g, h) = if dx3.abs() < 1e-12 && dy3.abs() < 1e-12 {
        (0.0, 0.0)
    } else {
        let den = dx1 * dy2 - dx2 * dy1;
        if den.abs() < 1e-12 {
            return Err(Degenerate::Collapsed);
        }
        ((dx3 * dy2 - dx2 * dy3) / den, (dx1 * dy3 - dx3 * dy1) / den)
    };
    let m = [
        [x1 - x0 + g * x1, x3 - x0 + h * x3, x0],
        [y1 - y0 + g * y1, y3 - y0 + h * y3, y0],
        [g, h, 1.0],
    ];
    Ok(m.map(|row| row.map(|v| v as f32)))
}

/// Maps a unit-square point through `m`.
pub fn apply(m: &Matrix, u: f32, v: f32) -> [f32; 2] {
    let w = m[2][0] * u + m[2][1] * v + m[2][2];
    [(m[0][0] * u + m[0][1] * v + m[0][2]) / w, (m[1][0] * u + m[1][1] * v + m[1][2]) / w]
}

/// The inverse (screen fraction back to the unit square), scaled so `w` is positive over the
/// quad: past the vanishing line `w` flips sign and the picture would mirror.
pub fn inverse(m: &Matrix) -> Matrix {
    let a = m.map(|row| row.map(|v| v as f64));
    let cof = |r0: usize, r1: usize, c0: usize, c1: usize| a[r0][c0] * a[r1][c1] - a[r0][c1] * a[r1][c0];
    // Adjugate of a 3x3: the transposed cofactors.
    let adj = [
        [cof(1, 2, 1, 2), -cof(0, 2, 1, 2), cof(0, 1, 1, 2)],
        [-cof(1, 2, 0, 2), cof(0, 2, 0, 2), -cof(0, 1, 0, 2)],
        [cof(1, 2, 0, 1), -cof(0, 2, 0, 1), cof(0, 1, 0, 1)],
    ];
    // The quad's centre, as the screen sees it.
    let c = apply(m, 0.5, 0.5);
    let z = adj[2][0] * c[0] as f64 + adj[2][1] * c[1] as f64 + adj[2][2];
    let flip = if z < 0.0 { -1.0 } else { 1.0 };
    adj.map(|row| row.map(|v| (v * flip) as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIT: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5
    }

    #[test]
    fn identity_corners_give_identity_matrix() {
        let m = homography(UNIT);
        for (r, row) in m.iter().enumerate() {
            for (c, v) in row.iter().enumerate() {
                assert!((v - IDENTITY[r][c]).abs() < 1e-6, "m[{r}][{c}] = {v}");
            }
        }
    }

    #[test]
    fn a_shift_moves_every_point() {
        let shifted = UNIT.map(|[x, y]| [x + 0.1, y + 0.2]);
        let m = homography(shifted);
        assert!(near(apply(&m, 0.5, 0.5), [0.6, 0.7]));
        assert!(near(apply(&m, 0.0, 0.0), [0.1, 0.2]));
        assert!(near(apply(&m, 1.0, 1.0), [1.1, 1.2]));
    }

    #[test]
    fn the_four_corners_land_on_the_four_corners() {
        let corners = [[0.05, 0.02], [0.97, 0.0], [1.0, 1.0], [0.0, 0.96]];
        let m = homography(corners);
        for (i, [u, v]) in UNIT.iter().enumerate() {
            assert!(near(apply(&m, *u, *v), corners[i]), "corner {i}");
        }
    }

    #[test]
    fn a_keystone_centre_is_where_the_diagonals_cross() {
        // Top edge narrowed to 0.1..0.9: the diagonals cross at x = 0.5, y = 4/9.
        let m = homography([[0.1, 0.0], [0.9, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        assert!(near(apply(&m, 0.5, 0.5), [0.5, 4.0 / 9.0]));
        assert!(near(apply(&m, 0.5, 0.0), [0.5, 0.0]));
    }

    #[test]
    fn the_inverse_undoes_the_matrix() {
        let m = homography([[0.1, 0.05], [0.9, 0.0], [1.0, 0.95], [0.0, 1.0]]);
        let inv = inverse(&m);
        for (u, v) in [(0.0, 0.0), (0.3, 0.8), (1.0, 0.5), (0.5, 0.5)] {
            let [x, y] = apply(&m, u, v);
            assert!(near(apply(&inv, x, y), [u, v]), "({u}, {v})");
        }
    }

    #[test]
    fn the_inverse_keeps_w_positive_over_the_quad() {
        let m = homography([[0.2, 0.0], [0.8, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        let inv = inverse(&m);
        for (u, v) in [(0.0, 0.0), (1.0, 0.0), (0.5, 0.5), (1.0, 1.0)] {
            let [x, y] = apply(&m, u, v);
            assert!(inv[2][0] * x + inv[2][1] * y + inv[2][2] > 0.0);
        }
    }

    #[test]
    fn collapsed_corners_are_rejected() {
        let point = [[0.5, 0.5]; 4];
        assert_eq!(try_homography(point), Err(Degenerate::Collapsed));
        let line = [[0.0, 0.5], [0.3, 0.5], [0.7, 0.5], [1.0, 0.5]];
        assert_eq!(try_homography(line), Err(Degenerate::Collapsed));
        assert_eq!(homography(point), IDENTITY);
    }

    #[test]
    fn crossed_and_concave_corners_are_rejected() {
        let bowtie = [[0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 1.0]];
        assert_eq!(try_homography(bowtie), Err(Degenerate::NotConvex));
        let dart = [[0.0, 0.0], [1.0, 0.0], [0.4, 0.4], [0.0, 1.0]];
        assert_eq!(try_homography(dart), Err(Degenerate::NotConvex));
    }

    #[test]
    fn nan_is_rejected_and_wild_values_are_clamped() {
        let mut bad = UNIT;
        bad[2][0] = f32::NAN;
        assert_eq!(try_homography(bad), Err(Degenerate::NotFinite));
        let wild = [[-50.0, -50.0], [60.0, -50.0], [60.0, 60.0], [-50.0, 60.0]];
        assert!(try_homography(wild).is_ok());
        assert!(near(apply(&homography(wild), 0.0, 0.0), [-1.0, -1.0]));
    }
}
