use crate::vec3::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub const fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray { origin, direction }
    }

    /// Returns the point along this ray at parameter `t`, i.e. `origin + t * direction`.
    /// `t == 0.0` gives the origin; `t == 1.0` gives the origin offset by one full
    /// `direction` vector. `direction` need not be normalized, so `t` is not a distance
    /// unless `direction` is a unit vector.
    pub fn at(&self, t: f64) -> Vec3 {
        self.origin + self.direction * t
    }
}
