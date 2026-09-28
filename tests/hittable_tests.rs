//! Phase 1 correctness tests: exact, hand-derivable geometric test cases
//! for `Triangle::hit`, `refract`, and `reflectance`. These need no
//! rendering and no reference image -- the expected values are computed
//! analytically, the same way HW1/HW2's function tests are.

use hw3_raytracer::{reflectance, refract, Hittable, Material, Ray, Triangle, Vec3};
use std::sync::Arc;

const EPS: f64 = 1e-9;

fn approx_eq(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "expected {b}, got {a}");
}

fn approx_eq_vec3(a: Vec3, b: Vec3) {
    approx_eq(a.x, b.x);
    approx_eq(a.y, b.y);
    approx_eq(a.z, b.z);
}

struct DummyMaterial;
impl Material for DummyMaterial {
    fn scatter(&self, _ray_in: &Ray, _hit: &hw3_raytracer::HitRecord, _rng: &mut rand::rngs::SmallRng) -> Option<(Vec3, Ray)> {
        None
    }
}

fn test_triangle() -> Triangle {
    // Right triangle in the z = -1 plane: (0,0,-1), (1,0,-1), (0,1,-1).
    Triangle::new(
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, -1.0),
        Arc::new(DummyMaterial),
    )
}

#[test]
fn triangle_hit_at_centroid() {
    let triangle = test_triangle();
    // Centroid of the three vertices is (1/3, 1/3, -1).
    let ray = Ray::new(Vec3::new(1.0 / 3.0, 1.0 / 3.0, 0.0), Vec3::new(0.0, 0.0, -1.0));
    let hit = triangle.hit(&ray, 0.001, f64::INFINITY).expect("expected a hit at the centroid");
    approx_eq(hit.t, 1.0);
    approx_eq_vec3(hit.p, Vec3::new(1.0 / 3.0, 1.0 / 3.0, -1.0));
    approx_eq_vec3(hit.normal, Vec3::new(0.0, 0.0, 1.0));
    assert!(hit.front_face);
}

#[test]
fn triangle_miss_outside_bounds() {
    let triangle = test_triangle();
    // Aimed at (2,2,-1), well outside the triangle (max x = max y = 1).
    let ray = Ray::new(Vec3::new(2.0, 2.0, 0.0), Vec3::new(0.0, 0.0, -1.0));
    assert!(triangle.hit(&ray, 0.001, f64::INFINITY).is_none());
}

#[test]
fn triangle_miss_parallel_ray() {
    let triangle = test_triangle();
    // Direction has no z component, so it never reaches the z = -1 plane.
    let ray = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
    assert!(triangle.hit(&ray, 0.001, f64::INFINITY).is_none());
}

#[test]
fn triangle_hit_respects_t_max() {
    let triangle = test_triangle();
    let ray = Ray::new(Vec3::new(1.0 / 3.0, 1.0 / 3.0, 0.0), Vec3::new(0.0, 0.0, -1.0));
    // The hit is at t = 1.0; restricting t_max below that must miss.
    assert!(triangle.hit(&ray, 0.001, 0.5).is_none());
}

#[test]
fn refract_at_normal_incidence_is_unchanged() {
    let n = Vec3::new(0.0, 0.0, 1.0);
    let uv = -n;
    for etai_over_etat in [0.5, 1.0, 1.5, 2.0] {
        let result = refract(uv, n, etai_over_etat);
        approx_eq_vec3(result, uv);
    }
}

#[test]
fn reflectance_at_normal_incidence_matches_schlick_r0() {
    // Schlick's r0 at cosine = 1.0 is exactly ((1-ri)/(1+ri))^2.
    let value = reflectance(1.0, 1.5);
    approx_eq(value, 0.04);

    let value_vacuum = reflectance(1.0, 1.0);
    approx_eq(value_vacuum, 0.0);
}

#[test]
fn reflectance_increases_toward_grazing_angles() {
    let ri = 1.5;
    let near_normal = reflectance(0.99, ri);
    let near_grazing = reflectance(0.05, ri);
    assert!(
        near_grazing > near_normal,
        "reflectance should approach 1.0 at grazing angles: {near_grazing} vs {near_normal}"
    );
    assert!((0.0..=1.0 + EPS).contains(&near_grazing));
}
