use core::f64;

use rand::rngs::SmallRng;
use rand::RngExt;

use crate::hittable::HitRecord;
use crate::ray::Ray;
use crate::vec3::Vec3;

/// A surface's scattering behavior. Implementations must be `Send + Sync`
/// because scenes (and their materials) are shared read-only across render
/// threads.
///
/// Returns `None` if the ray is fully absorbed; otherwise the attenuation
/// (color multiplier) and the scattered ray.
pub trait Material: Send + Sync {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord, rng: &mut SmallRng) -> Option<(Vec3, Ray)>;
}

fn random_in_unit_sphere(rng: &mut SmallRng) -> Vec3 {
    loop {
        let p = Vec3::new(
            rng.random_range(-1.0..1.0),
            rng.random_range(-1.0..1.0),
            rng.random_range(-1.0..1.0),
        );
        if p.length_squared() < 1.0 {
            return p;
        }
    }
}

fn random_unit_vector(rng: &mut SmallRng) -> Vec3 {
    random_in_unit_sphere(rng).unit_vector()
}

/// A matte (diffuse) material: scatters uniformly around the surface
/// normal.
pub struct Lambertian {
    pub albedo: Vec3,
}

impl Lambertian {
    pub fn new(albedo: Vec3) -> Self {
        Lambertian { albedo }
    }
}

impl Material for Lambertian {
    fn scatter(&self, _ray_in: &Ray, hit: &HitRecord, rng: &mut SmallRng) -> Option<(Vec3, Ray)> {
        let mut scatter_direction = hit.normal + random_unit_vector(rng);
        if scatter_direction.near_zero() {
            scatter_direction = hit.normal;
        }
        Some((self.albedo, Ray::new(hit.p, scatter_direction)))
    }
}

/// A reflective (mirror-like) material. `fuzz` (0.0-1.0) randomly perturbs
/// the reflected ray to simulate a brushed/rough metal.
pub struct Metal {
    pub albedo: Vec3,
    pub fuzz: f64,
}

impl Metal {
    pub fn new(albedo: Vec3, fuzz: f64) -> Self {
        Metal {
            albedo,
            fuzz: fuzz.clamp(0.0, 1.0),
        }
    }
}

impl Material for Metal {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord, rng: &mut SmallRng) -> Option<(Vec3, Ray)> {
        let reflected = ray_in.direction.unit_vector().reflect(hit.normal);
        let scattered = Ray::new(hit.p, reflected + random_unit_vector(rng) * self.fuzz);
        if scattered.direction.dot(hit.normal) > 0.0 {
            Some((self.albedo, scattered))
        } else {
            None
        }
    }
}

/// A transparent material that reflects or refracts an incoming ray.
pub struct Dielectric {
    pub refraction_index: f64,
}

impl Dielectric {
    pub fn new(refraction_index: f64) -> Self {
        Dielectric { refraction_index }
    }
}

/// Refracts a unit incoming direction `uv` through a surface with (unit)
/// outward normal `n`, where `etai_over_etat` is the ratio of the
/// refractive index of the incident medium to that of the transmitted
/// medium.
pub fn refract(uv: Vec3, n: Vec3, etai_over_etat: f64) -> Vec3 {
    let cos_theta = (-uv).dot(n).min(1.0);
    // Snell's law scales the component along the surface. The remaining
    // component points through the surface, opposite its normal.
    let perpendicular = (uv + n * cos_theta) * etai_over_etat;
    let parallel = -n * (1.0 - perpendicular.length_squared()).abs().sqrt();
    perpendicular + parallel
}

/// Schlick's approximation for the probability that a ray reflects off a
/// dielectric boundary rather than refracting through it, given the cosine
/// of the angle of incidence and the relative refraction index.
pub fn reflectance(cosine: f64, refraction_index: f64) -> f64 {
    let ratio = (1.0 - refraction_index) / (1.0 + refraction_index);
    let r0 = ratio.powi(2);
    let result = r0 + (1.0 - r0) * (1.0 - cosine).powi(5);
    result
}

impl Material for Dielectric {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord, rng: &mut SmallRng) -> Option<(Vec3, Ray)> {
        let attenuation = Vec3::new(1.0, 1.0, 1.0);
        let refraction_ratio = if hit.front_face {
            1.0 / self.refraction_index
        }
        else {
            self.refraction_index
        };
        let unit_direction = ray_in.direction / ray_in.direction.length();
        let cos_theta = (-unit_direction).dot(hit.normal).min(1.0);
        let sin_theta = (1.0 - cos_theta.powi(2)).sqrt();
        // Beyond the critical angle, refraction is impossible. Otherwise
        // Schlick's estimate chooses reflection probabilistically.
        let cannot_refract = refraction_ratio * sin_theta > 1.0;
        let direction = if cannot_refract || reflectance(cos_theta, self.refraction_index) > rng.random::<f64>() {
            unit_direction.reflect(hit.normal)
        }
        else {
            refract(unit_direction, hit.normal, refraction_ratio)
        };
        let scattered = Ray::new(hit.p, direction);
        Some((attenuation, scattered))
    }
}
