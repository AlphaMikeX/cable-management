use cable_management::{geometry::*, model::*, tree};

#[test]
fn fixed_views_face_their_named_surface() {
    for face in Face::ALL {
        let part = Part {
            id: 1,
            name: "box".into(),
            shape: Primitive::Box,
            size: [100., 80., 60.],
            offset: [0.; 3],
        };
        let frame = face_frame(&part, face, [0.; 2]);
        let camera = Camera::for_face(face);
        assert!(dot(frame.normal, camera.direction()) > 0.99, "{face:?}");
        let (point, depth) = camera.project(frame.center);
        assert!(point[0].abs() < 0.01 && point[1].abs() < 0.01);
        assert!(depth > 0.);
    }
}

#[test]
fn socket_keeps_physical_size_on_rotated_surface() {
    let p = Project::demo();
    let n = &p.nodes[0];
    let port = &n.ports[0];
    let frame = face_frame(&n.parts[0], port.face, port.uv);
    let corners = socket_points(port, &frame);
    assert!((length(sub(corners[1], corners[0])) - 12.).abs() < 0.001);
    assert!((length(sub(corners[2], corners[1])) - 4.5).abs() < 0.001);
}

#[test]
fn dragging_inverse_returns_same_face_coordinates() {
    let part = Part {
        id: 1,
        name: "box".into(),
        shape: Primitive::Wedge,
        size: [200., 80., 120.],
        offset: [20., -10., 30.],
    };
    for face in Face::ALL {
        let uv = [0.12, -0.1];
        let f = face_frame(&part, face, uv);
        let camera = Camera::for_face(face);
        let (screen, _) = camera.project(f.center);
        let recovered = face_uv_at(&part, face, &camera, screen).unwrap();
        assert!((uv[0] - recovered[0]).abs() < 0.001, "{face:?}");
        assert!((uv[1] - recovered[1]).abs() < 0.001, "{face:?}");
    }
}

#[test]
fn full_tree_shows_each_node_once_including_multiple_roots() {
    let mut p = Project::demo();
    p.add_node("Mixer", [200., 60., 160.]);
    let positions = tree::layout(&p);
    assert_eq!(positions.len(), 3);
    let pc = positions.iter().find(|x| x.id == p.nodes[0].id).unwrap();
    let hub = positions.iter().find(|x| x.id == p.nodes[1].id).unwrap();
    assert!(hub.position[1] > pc.position[1]);
}

#[test]
fn hidden_port_cannot_be_picked_through_box() {
    let p = Project::demo();
    let n = &p.nodes[0];
    let back = &n.ports[0];
    let camera = Camera::for_face(Face::Front);
    assert!(!port_visible(n, back, &camera));
    assert!(port_visible(n, back, &Camera::for_face(Face::Back)));
}

#[test]
fn ports_must_fit_the_actual_short_front_of_a_wedge() {
    let mut p = Project::default();
    let id = p.add_node("Kil", [100., 40., 80.]);
    p.node_mut(id).unwrap().parts[0].shape = Primitive::Wedge;
    let port = p
        .add_port(id, cable_management::ports::PortKind::Rj45, "Nätverk")
        .unwrap();
    let r = PortRef { node: id, port };
    p.port_mut(r).unwrap().face = Face::Front;
    p.port_mut(r).unwrap().uv = [0., 0.4];
    assert!(p.validate().is_err());
    let part = p.node(id).unwrap().parts[0].clone();
    p.port_mut(r).unwrap().clamp_to_part(&part);
    p.validate().unwrap();
    let port = p.port(r).unwrap();
    let f = face_frame(&part, port.face, port.uv);
    assert!(f.center[1] + port.kind.dimensions()[1] / 2. <= 0.01);
}

#[test]
fn a_wide_socket_cannot_extend_above_a_sloping_side() {
    let mut p = Project::default();
    let id = p.add_node("Kil", [100., 40., 80.]);
    p.node_mut(id).unwrap().parts[0].shape = Primitive::Wedge;
    let port = p
        .add_port(id, cable_management::ports::PortKind::UsbA, "USB")
        .unwrap();
    let r = PortRef { node: id, port };
    p.port_mut(r).unwrap().face = Face::Left;
    p.port_mut(r).unwrap().uv = [0.3, 0.40625];
    assert!(p.validate().is_err());
    let part = p.node(id).unwrap().parts[0].clone();
    p.port_mut(r).unwrap().clamp_to_part(&part);
    p.validate().unwrap();
    let port = p.port(r).unwrap();
    let frame = face_frame(&part, port.face, port.uv);
    for corner in socket_points(port, &frame) {
        assert!(corner[1] <= 10. - 0.25 * corner[2] + 0.01);
    }
}
