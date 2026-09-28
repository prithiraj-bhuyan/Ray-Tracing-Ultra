use std::sync::Arc;

use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::Vec3;

/// The result of a ray hitting an object: where, and what to do next.
pub struct HitRecord {
    pub t: f64,
    pub p: Vec3,
    /// Always points against the incoming ray (see `front_face`).
    pub normal: Vec3,
    /// True if the ray hit the outer surface (outward_normal opposed the
    /// ray); false if it hit from the inside (e.g. exiting a glass sphere).
    pub front_face: bool,
    pub material: Arc<dyn Material>,
}

impl HitRecord {
    /// Builds a `HitRecord`, orienting `normal` to always face against
    /// `ray`, and recording whether that required flipping `outward_normal`.
    pub fn new(t: f64, p: Vec3, ray: &Ray, outward_normal: Vec3, material: Arc<dyn Material>) -> Self {
        let front_face = ray.direction.dot(outward_normal) < 0.0;
        let normal = if front_face {
            outward_normal
        } else {
            -outward_normal
        };
        HitRecord {
            t,
            p,
            normal,
            front_face,
            material,
        }
    }
}

/// Anything a ray can hit. Implementations must be `Send + Sync` because
/// scenes are shared read-only across render threads.
pub trait Hittable: Send + Sync {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord>;
}
