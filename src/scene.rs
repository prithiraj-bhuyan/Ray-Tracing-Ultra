use std::sync::Arc;

use crate::hittable::{HitRecord, Hittable};
use crate::ray::Ray;

/// A flat list of scene objects, itself `Hittable` (returns the closest
/// hit among all its objects).
#[derive(Default)]
pub struct Scene {
    pub objects: Vec<Arc<dyn Hittable>>,
}

impl Scene {
    pub fn new() -> Self {
        Scene { objects: Vec::new() }
    }

    pub fn add(&mut self, object: Arc<dyn Hittable>) {
        self.objects.push(object);
    }
}

impl Hittable for Scene {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let mut closest_so_far = t_max;
        let mut result = None;
        for object in &self.objects {
            if let Some(rec) = object.hit(ray, t_min, closest_so_far) {
                closest_so_far = rec.t;
                result = Some(rec);
            }
        }
        result
    }
}
