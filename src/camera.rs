use crate::ray::Ray;
use crate::vec3::Vec3;

/// A simple pinhole camera (no depth-of-field).
pub struct Camera {
    origin: Vec3,
    lower_left_corner: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
}

impl Camera {
    /// `vfov_degrees` is the vertical field of view. `aspect_ratio` should
    /// match the output image's `width as f64 / height as f64`.
    pub fn new(lookfrom: Vec3, lookat: Vec3, vup: Vec3, vfov_degrees: f64, aspect_ratio: f64) -> Self {
        let theta = vfov_degrees.to_radians();
        let h = (theta / 2.0).tan();
        let viewport_height = 2.0 * h;
        let viewport_width = aspect_ratio * viewport_height;

        let w = (lookfrom - lookat).unit_vector();
        let u = vup.cross(w).unit_vector();
        let v = w.cross(u);

        let origin = lookfrom;
        let horizontal = u * viewport_width;
        let vertical = v * viewport_height;
        let lower_left_corner = origin - horizontal / 2.0 - vertical / 2.0 - w;

        Camera {
            origin,
            lower_left_corner,
            horizontal,
            vertical,
        }
    }

    /// `s` and `t` are normalized viewport coordinates in `[0, 1]`
    /// (0,0 = bottom-left of the image, 1,1 = top-right).
    pub fn get_ray(&self, s: f64, t: f64) -> Ray {
        Ray::new(
            self.origin,
            self.lower_left_corner + self.horizontal * s + self.vertical * t - self.origin,
        )
    }
}
