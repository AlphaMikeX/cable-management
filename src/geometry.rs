use crate::model::{Face, Node, Part, Port, Primitive};

pub type Vec3 = [f32; 3];
pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn mul(a: Vec3, t: f32) -> Vec3 {
    [a[0] * t, a[1] * t, a[2] * t]
}
pub fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn length(a: Vec3) -> f32 {
    dot(a, a).sqrt()
}
pub fn unit(a: Vec3) -> Vec3 {
    mul(a, 1. / length(a).max(1e-6))
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub center: Vec3,
}
impl Camera {
    pub fn for_face(face: Face) -> Self {
        use std::f32::consts::{FRAC_PI_2, PI};
        let (yaw, pitch) = match face {
            Face::Front => (0., 0.),
            Face::Back => (PI, 0.),
            Face::Left => (-FRAC_PI_2, 0.),
            Face::Right => (FRAC_PI_2, 0.),
            Face::Top => (0., FRAC_PI_2),
            Face::Bottom => (0., -FRAC_PI_2),
        };
        Self {
            yaw,
            pitch,
            center: [0.; 3],
        }
    }
    pub fn direction(&self) -> Vec3 {
        [
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        ]
    }
    pub fn right(&self) -> Vec3 {
        [self.yaw.cos(), 0., -self.yaw.sin()]
    }
    pub fn up(&self) -> Vec3 {
        cross(self.direction(), self.right())
    }
    pub fn project(&self, point: Vec3) -> ([f32; 2], f32) {
        let p = sub(point, self.center);
        (
            [dot(p, self.right()), -dot(p, self.up())],
            dot(p, self.direction()),
        )
    }
    pub fn animate(&mut self, target: &Self, dt: f32) -> bool {
        let delta = (target.yaw - self.yaw + std::f32::consts::PI)
            .rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        let tilt = target.pitch - self.pitch;
        if delta.abs() + tilt.abs() < 0.001 {
            self.yaw = target.yaw;
            self.pitch = target.pitch;
            return false;
        }
        let t = 1. - (-dt.min(0.1) * 10.).exp();
        self.yaw += delta * t;
        self.pitch += tilt * t;
        true
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FaceFrame {
    pub center: Vec3,
    pub u: Vec3,
    pub v: Vec3,
    pub normal: Vec3,
}
pub fn face_frame(part: &Part, face: Face, uv: [f32; 2]) -> FaceFrame {
    let [w, h, d] = part.size;
    let mut frame = match face {
        Face::Front => FaceFrame {
            center: [uv[0] * w, uv[1] * h, d / 2.],
            u: [1., 0., 0.],
            v: [0., 1., 0.],
            normal: [0., 0., 1.],
        },
        Face::Back => FaceFrame {
            center: [-uv[0] * w, uv[1] * h, -d / 2.],
            u: [-1., 0., 0.],
            v: [0., 1., 0.],
            normal: [0., 0., -1.],
        },
        Face::Left => FaceFrame {
            center: [-w / 2., uv[1] * h, uv[0] * d],
            u: [0., 0., 1.],
            v: [0., 1., 0.],
            normal: [-1., 0., 0.],
        },
        Face::Right => FaceFrame {
            center: [w / 2., uv[1] * h, -uv[0] * d],
            u: [0., 0., -1.],
            v: [0., 1., 0.],
            normal: [1., 0., 0.],
        },
        Face::Top => FaceFrame {
            center: [uv[0] * w, h / 2., -uv[1] * d],
            u: [1., 0., 0.],
            v: [0., 0., -1.],
            normal: [0., 1., 0.],
        },
        Face::Bottom => FaceFrame {
            center: [uv[0] * w, -h / 2., uv[1] * d],
            u: [1., 0., 0.],
            v: [0., 0., 1.],
            normal: [0., -1., 0.],
        },
    };
    if part.shape == Primitive::Wedge {
        match face {
            Face::Front => frame.center[1] = -h / 4. + uv[1] * h / 2.,
            Face::Left | Face::Right => {
                let top = h / 4. - h / (2. * d) * frame.center[2];
                let local_height = top + h / 2.;
                frame.center[1] = -h / 2. + (uv[1] + 0.5) * local_height;
            }
            Face::Top => {
                let slope = h / (2. * d);
                frame.center[1] = h / 4. - slope * frame.center[2];
                frame.v = unit([0., slope, -1.]);
                frame.normal = unit([0., 1., slope]);
            }
            _ => {}
        }
    }
    frame.center = add(frame.center, part.offset);
    frame
}

pub fn socket_points(port: &Port, frame: &FaceFrame) -> Vec<Vec3> {
    let [w, h] = port.kind.dimensions();
    let (s, c) = port.angle.to_radians().sin_cos();
    port.kind
        .outline()
        .into_iter()
        .map(|[x, y]| {
            let u = x * w * c - y * h * s;
            let v = x * w * s + y * h * c;
            add(
                add(add(frame.center, mul(frame.u, u)), mul(frame.v, v)),
                mul(frame.normal, 0.15),
            )
        })
        .collect()
}

pub fn face_vertices(part: &Part, face: Face) -> Vec<Vec3> {
    let [w, h, d] = part.size.map(|x| x / 2.);
    let front_top = if part.shape == Primitive::Wedge {
        0.
    } else {
        h
    };
    let points = match face {
        Face::Front => vec![
            [-w, -h, d],
            [w, -h, d],
            [w, front_top, d],
            [-w, front_top, d],
        ],
        Face::Back => vec![[w, -h, -d], [-w, -h, -d], [-w, h, -d], [w, h, -d]],
        Face::Left => vec![[-w, -h, -d], [-w, -h, d], [-w, front_top, d], [-w, h, -d]],
        Face::Right => vec![[w, -h, d], [w, -h, -d], [w, h, -d], [w, front_top, d]],
        Face::Top => vec![
            [-w, front_top, d],
            [w, front_top, d],
            [w, h, -d],
            [-w, h, -d],
        ],
        Face::Bottom => vec![[-w, -h, -d], [w, -h, -d], [w, -h, d], [-w, -h, d]],
    };
    points.into_iter().map(|p| add(p, part.offset)).collect()
}

pub fn face_uv_at(part: &Part, face: Face, camera: &Camera, screen: [f32; 2]) -> Option<[f32; 2]> {
    let frame = face_frame(part, face, [0.; 2]);
    let origin = add(
        camera.center,
        add(mul(camera.right(), screen[0]), mul(camera.up(), -screen[1])),
    );
    let direction = camera.direction();
    let denominator = dot(direction, frame.normal);
    if denominator.abs() < 0.05 {
        return None;
    }
    let t = dot(sub(frame.center, origin), frame.normal) / denominator;
    let p = sub(add(origin, mul(direction, t)), part.offset);
    let [w, h, d] = part.size;
    let mut uv = match face {
        Face::Front => [p[0] / w, p[1] / h],
        Face::Back => [-p[0] / w, p[1] / h],
        Face::Left => [p[2] / d, p[1] / h],
        Face::Right => [-p[2] / d, p[1] / h],
        Face::Top => [p[0] / w, -p[2] / d],
        Face::Bottom => [p[0] / w, p[2] / d],
    };
    if part.shape == Primitive::Wedge {
        match face {
            Face::Front => uv[1] = (p[1] + h / 4.) / (h / 2.),
            Face::Left | Face::Right => {
                let top = h / 4. - h / (2. * d) * p[2];
                uv[1] = (p[1] + h / 2.) / (top + h / 2.) - 0.5;
            }
            _ => {}
        }
    }
    Some(uv)
}

pub fn node_bounds(node: &Node) -> (Vec3, Vec3) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &node.parts {
        for i in 0..3 {
            min[i] = min[i].min(p.offset[i] - p.size[i] / 2.);
            max[i] = max[i].max(p.offset[i] + p.size[i] / 2.);
        }
    }
    (min, max)
}

fn ray_triangle(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let p = cross(dir, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-6 {
        return None;
    }
    let inv = 1. / det;
    let s = sub(origin, a);
    let u = dot(s, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = dot(dir, q) * inv;
    if v < 0. || u + v > 1. {
        return None;
    }
    let t = dot(e2, q) * inv;
    if t > 0.001 { Some(t) } else { None }
}

/// Opaque parts block picking through a chassis, including overlapping building blocks.
pub fn port_visible(node: &Node, port: &Port, camera: &Camera) -> bool {
    let Some(part) = node.parts.iter().find(|p| p.id == port.part) else {
        return false;
    };
    let f = face_frame(part, port.face, port.uv);
    let direction = camera.direction();
    if dot(f.normal, direction) < 0.04 {
        return false;
    }
    let origin = add(f.center, mul(f.normal, 0.2));
    for part in &node.parts {
        for face in Face::ALL {
            let v = face_vertices(part, face);
            if ray_triangle(origin, direction, v[0], v[1], v[2]).is_some()
                || ray_triangle(origin, direction, v[0], v[2], v[3]).is_some()
            {
                return false;
            }
        }
    }
    true
}
