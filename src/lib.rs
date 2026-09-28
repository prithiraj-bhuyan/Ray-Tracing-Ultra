//! Ray tracer library. The tile scheduler stays crate-private: it contains
//! the unsafe frame-buffer access, while `dynamic` uses only its safe API.

pub mod camera;
pub mod dynamic;
pub mod hittable;
pub mod material;
pub mod parallel;
pub mod ray;
pub mod render;
pub mod scene;
pub mod scenes;
pub(crate) mod scheduler;
pub mod sphere;
pub mod triangle;
pub mod vec3;

pub use camera::Camera;
pub use dynamic::render_dynamic;
pub use hittable::{HitRecord, Hittable};
pub use material::{reflectance, refract, Dielectric, Lambertian, Material, Metal};
pub use parallel::render_parallel;
pub use ray::Ray;
pub use render::{render_pixel, render_sequential, RenderOptions};
pub use scene::Scene;
pub use sphere::Sphere;
pub use triangle::Triangle;
pub use vec3::Vec3;

pub use image::RgbImage;
