use cable_management::{geometry::*, model::Part};

/// Resize from a frozen gesture snapshot. The opposite edge stays fixed;
/// screen displacement is in millimetres, so zoom never changes dimensions.
pub fn resized(part: &Part, camera: &Camera, handle: [i8; 2], delta: [f32; 2]) -> Part {
    let mut next = part.clone();
    if delta.iter().any(|v| !v.is_finite()) {
        return next;
    }
    for (i, direction) in [camera.right(), mul(camera.up(), -1.)].iter().enumerate() {
        if handle[i] == 0 {
            continue;
        }
        let axis = (0..3)
            .max_by(|a, b| direction[*a].abs().total_cmp(&direction[*b].abs()))
            .unwrap();
        let side = f32::from(handle[i]);
        let size = (part.size[axis] + delta[i] * side).clamp(4., 5000.);
        let motion = (size - part.size[axis]) * side;
        next.size[axis] = size;
        next.offset[axis] += direction[axis].signum() * motion * 0.5;
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use cable_management::model::*;
    fn body() -> Part {
        Part {
            id: 1,
            name: "Body".into(),
            shape: Primitive::Box,
            size: [200., 120., 80.],
            offset: [10., 20., 30.],
        }
    }
    #[test]
    fn corners_resize_only_visible_dimensions_in_every_named_view() {
        for face in Face::ALL {
            let p = body();
            let camera = Camera::for_face(face);
            let next = resized(&p, &camera, [1, 1], [30., 20.]);
            let (a, b) = face.axes();
            assert!((next.size[a] - p.size[a] - 30.).abs() < 0.001, "{face:?}");
            assert!((next.size[b] - p.size[b] - 20.).abs() < 0.001, "{face:?}");
            let hidden = (0..3).find(|i| *i != a && *i != b).unwrap();
            assert_eq!(next.size[hidden], p.size[hidden]);
            let (shift, _) = camera.project(sub(next.offset, p.offset));
            assert!((shift[0] - 15.).abs() < 0.001);
            assert!((shift[1] - 10.).abs() < 0.001);
        }
    }
    #[test]
    fn edge_keeps_opposite_edge_fixed_and_limits_collapse() {
        let p = body();
        let camera = Camera::for_face(Face::Back);
        let next = resized(&p, &camera, [-1, 0], [-50., 90.]);
        assert_eq!(next.size, [250., 120., 80.]);
        assert_eq!(next.offset, [35., 20., 30.]);
        let collapsed = resized(&p, &camera, [1, 0], [-10000., 0.]);
        assert_eq!(collapsed.size[0], 4.);
        assert_eq!(collapsed.offset[0], 108.);
    }
    #[test]
    fn non_finite_pointer_delta_keeps_original_geometry() {
        let p = body();
        assert_eq!(
            resized(&p, &Camera::for_face(Face::Front), [1, 1], [f32::NAN, 0.]),
            p
        );
    }
}
