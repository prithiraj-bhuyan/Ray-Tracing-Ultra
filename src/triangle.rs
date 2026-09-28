use std::sync::Arc;

use crate::hittable::{HitRecord, Hittable};
use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::Vec3;

/// A finite triangle with any vertex winding. Its geometric normal is
/// oriented against the incoming ray by `HitRecord::new`.
pub struct Triangle {
    pub p0: Vec3,
    pub p1: Vec3,
    pub p2: Vec3,
    pub material: Arc<dyn Material>,
}

impl Triangle {
    pub fn new(p0: Vec3, p1: Vec3, p2: Vec3, material: Arc<dyn Material>) -> Self {
        Triangle { p0, p1, p2, material }
    }
}

impl Hittable for Triangle {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        // Möller–Trumbore: solve for the ray distance and two barycentric
        // coordinates without first computing a plane intersection.
        let edge1 = self.p1 - self.p0;
        let edge2 = self.p2 - self.p0;
        let pvec = ray.direction.cross(edge2);
        let det = edge1.dot(pvec);
        
        // A near-zero determinant means the ray is parallel to the triangle.
        if det.abs() <= 1e-8 { 
            return None; 
        }

        let inv_det = 1.0 / det;
        let tvec = ray.origin - self.p0;
        let u = tvec.dot(pvec) * inv_det;

        // Inside the triangle requires u >= 0, v >= 0, and u + v <= 1.
        if u < 0f64 || u > 1f64 {
            return None;
        }

        let qvec = tvec.cross(edge1);
        let v = ray.direction.dot(qvec) * inv_det;

        if v < 0f64 || u+v > 1f64 {
            return None;
        }

        let t = edge2.dot(qvec) * inv_det;

        if t < t_min || t > t_max {
            return None;
        }

        let pos = ray.at(t);
        let normal = edge1.cross(edge2).unit_vector();

        let hit_record = HitRecord::new(t, pos, ray, normal, self.material.clone());

        Some(hit_record)
    }
}
