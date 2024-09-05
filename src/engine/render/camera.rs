use std::f32::consts::PI;

use nalgebra::{Matrix4, SimdComplexField, vector, Vector3, Vector4};
use winit::{dpi::PhysicalPosition, event::*};
use winit::keyboard::KeyCode;

const UP: Vector3<f32> = Vector3::<f32>::new(0.0, 0.0, 1.0);

#[allow(unused)]
#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub target: Vector3<f32>,
    pub eye: nalgebra::Point3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub z_near: f32,
    pub z_far: f32,
}

#[allow(unused)]
impl Camera {
    pub fn calc_target(&self, yaw: f32, pitch: f32) -> Vector3<f32> {
        let (sin, cos) = yaw.to_radians().simd_sin_cos();
        let target = Vector3::new(cos, sin * (1.0 - UP.y), sin * (1.0 - UP.z));
        let (sin, cos) = pitch.to_radians().simd_sin_cos();
        let target = (target * cos) + (UP * sin);
        target
    }

    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let proj = Matrix4::new_perspective(self.aspect, self.fovy, self.z_near, self.z_far);
        let view = Matrix4::<f32>::look_at_rh(&self.eye, &(self.eye + self.target), &UP);
        // v′=P⋅V⋅M⋅v
        proj * view
    }
    pub fn new(eye: nalgebra::Point3<f32>) -> Self {
        Self {
            target: vector![1.0, 0.0, 0.0],
            eye,
            aspect: 16.0 / 9.0,
            fovy: 80.0_f32.to_radians(),
            z_near: 0.0001,
            z_far: 1000.0,
        }
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_position: Vector4<f32>,
    pub view_proj: Matrix4<f32>,
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: Vector4::zeros(),
            view_proj: Matrix4::identity(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = camera.eye.to_homogeneous();
        self.view_proj = camera.build_view_projection_matrix();
    }
}
#[allow(unused)]
pub struct CameraController {

}

#[allow(unused)]
impl CameraController {


}

#[cfg(test)]
mod test {
    use nalgebra::{point, vector};

    use crate::engine::render::camera::{Camera, UP};

    #[test]
    fn test_coord() {
        assert_eq!(UP, vector![0.0, 0.0, 1.0]);
        let camera = Camera::new(point![0.0, 0.0, 0.0]);
        assert_eq!(camera.calc_target(0.0, 0.0), vector![1.0, 0.0, 0.0]);
    }
}