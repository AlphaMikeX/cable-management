use cable_management::{model::*, ports::PortKind, storage};

fn one_port(size: [f32; 3], kind: PortKind) -> (Project, PortRef, Id) {
    let mut project = Project::default();
    let node = project.add_node("Device", size);
    let part = project.node(node).unwrap().parts[0].id;
    let port = project.add_port(node, kind, "Socket").unwrap();
    (project, PortRef { node, port }, part)
}

#[test]
fn legacy_json_without_label_offsets_keeps_saved_names() {
    let mut project = Project::demo();
    project.name = "Mitt sparade projekt".into();
    project.nodes[0].name = "Min dator".into();
    let mut legacy = serde_json::to_value(&project).unwrap();
    for node in legacy["nodes"].as_array_mut().unwrap() {
        for port in node["ports"].as_array_mut().unwrap() {
            port.as_object_mut().unwrap().remove("label_offset");
        }
    }
    let restored: Project = serde_json::from_value(legacy).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, project);
    assert!(
        restored
            .nodes
            .iter()
            .flat_map(|n| &n.ports)
            .all(|p| p.label_offset.is_none())
    );
    assert!(
        !serde_json::to_string(&restored)
            .unwrap()
            .contains("label_offset")
    );
}

#[test]
fn label_offsets_and_normalized_rotation_roundtrip_through_storage() {
    let (mut project, port, _) = one_port([80., 50., 40.], PortKind::Hdmi);
    project.port_mut(port).unwrap().label_offset = Some([-32.5, 18.25]);
    project.rotate_port(port, -450.).unwrap();
    assert_eq!(project.port(port).unwrap().angle, 270.);
    let file = std::env::temp_dir().join(format!(
        "canvas-model-{}-roundtrip.json",
        std::process::id()
    ));
    storage::save(&file, &project).unwrap();
    assert_eq!(storage::load(&file).unwrap(), project);
    std::fs::remove_file(file).unwrap();
}

#[test]
fn invalid_label_offsets_are_rejected() {
    let (mut project, port, _) = one_port([80., 50., 40.], PortKind::Hdmi);
    for offset in [
        [f32::NAN, 0.],
        [0., f32::INFINITY],
        [10_001., 0.],
        [0., -10_001.],
    ] {
        project.port_mut(port).unwrap().label_offset = Some(offset);
        assert!(project.validate().is_err(), "accepted {offset:?}");
    }
    project.port_mut(port).unwrap().label_offset = Some([10_000., -10_000.]);
    project.validate().unwrap();
}

#[test]
fn rotation_clamps_to_surface_and_preserves_label_and_connection() {
    let (mut project, port, part) = one_port([40., 40., 40.], PortKind::Hdmi);
    project.port_mut(port).unwrap().uv = [0.30, 0.40];
    project.port_mut(port).unwrap().label_offset = Some([7., -8.]);
    project.connect_endpoint(port, "Monitor").unwrap();
    let connections = project.connections.clone();
    project.rotate_port(port, 450.).unwrap();
    let socket = project.port(port).unwrap();
    assert_eq!(socket.angle, 90.);
    assert!(socket.uv[1] < 0.40);
    assert!(
        socket.fits_on(
            project
                .node(port.node)
                .unwrap()
                .parts
                .iter()
                .find(|p| p.id == part)
                .unwrap()
        )
    );
    assert_eq!(socket.label_offset, Some([7., -8.]));
    assert_eq!(project.connections, connections);
    project.validate().unwrap();
}

#[test]
fn rejected_rotation_is_atomic_for_invalid_input_missing_port_or_no_room() {
    let (mut project, port, _) = one_port([20., 6., 20.], PortKind::Hdmi);
    project.validate().unwrap();
    let before = project.clone();
    for angle in [f32::NAN, f32::INFINITY, 90.] {
        assert!(project.rotate_port(port, angle).is_err());
        assert_eq!(project, before);
    }
    assert!(
        project
            .rotate_port(
                PortRef {
                    node: port.node,
                    port: u64::MAX
                },
                45.
            )
            .is_err()
    );
    assert_eq!(project, before);
}

#[test]
fn resize_clamps_every_attached_port_and_keeps_connections_and_unrelated_part() {
    let mut project = Project::demo();
    let node = project.nodes[0].id;
    let part = project.nodes[0].parts[0].id;
    let other = project.add_part(node, Primitive::Box).unwrap();
    let other_before = project
        .node(node)
        .unwrap()
        .parts
        .iter()
        .find(|p| p.id == other)
        .unwrap()
        .clone();
    for (index, socket) in project.node_mut(node).unwrap().ports.iter_mut().enumerate() {
        socket.uv = [if index % 2 == 0 { -0.4 } else { 0.4 }, 0.4];
        socket.label_offset = Some([index as f32, -12.]);
    }
    let ports_before = project.node(node).unwrap().ports.clone();
    let connections = project.connections.clone();
    project
        .resize_part(node, part, [45., 20., 30.], [12., -15., 5.])
        .unwrap();
    let device = project.node(node).unwrap();
    let body = device.parts.iter().find(|p| p.id == part).unwrap();
    assert_eq!(body.size, [45., 20., 30.]);
    assert_eq!(body.offset, [12., -15., 5.]);
    assert!(device.ports.iter().all(|p| p.fits_on(body)));
    for (before, after) in ports_before.iter().zip(&device.ports) {
        assert_eq!(before.id, after.id);
        assert_eq!(before.kind, after.kind);
        assert_eq!(before.angle, after.angle);
        assert_eq!(before.label_offset, after.label_offset);
    }
    assert_eq!(
        device.parts.iter().find(|p| p.id == other).unwrap(),
        &other_before
    );
    assert_eq!(project.connections, connections);
    project.validate().unwrap();
}

#[test]
fn resize_keeps_ports_fitted_on_all_faces_of_boxes_and_wedges() {
    for shape in [Primitive::Box, Primitive::Wedge] {
        let mut project = Project::default();
        let node = project.add_node("Device", [80.; 3]);
        let part = project.node(node).unwrap().parts[0].id;
        project.node_mut(node).unwrap().parts[0].shape = shape;
        for (index, face) in Face::ALL.into_iter().enumerate() {
            let id = project
                .add_port(node, PortKind::MicroUsb, face.name())
                .unwrap();
            let port = project.port_mut(PortRef { node, port: id }).unwrap();
            port.face = face;
            port.uv = [0.4, 0.4];
            port.angle = index as f32 * 15.;
        }
        project.validate().unwrap();
        project.resize_part(node, part, [30.; 3], [5.; 3]).unwrap();
        let device = project.node(node).unwrap();
        assert!(device.ports.iter().all(|p| p.fits_on(&device.parts[0])));
        project.validate().unwrap();
    }
}

#[test]
fn resize_rejects_invalid_or_too_small_geometry_without_mutation() {
    let (mut project, port, part) = one_port([80., 50., 40.], PortKind::Hdmi);
    project.connect_endpoint(port, "Monitor").unwrap();
    let before = project.clone();
    for (size, offset) in [
        ([4., 4., 4.], [0.; 3]),
        ([f32::NAN, 50., 40.], [0.; 3]),
        ([80., 50., 40.], [0., f32::INFINITY, 0.]),
        ([80., 50., 40.], [10_001., 0., 0.]),
    ] {
        assert!(project.resize_part(port.node, part, size, offset).is_err());
        assert_eq!(project, before);
    }
    assert!(
        project
            .resize_part(port.node, u64::MAX, [80.; 3], [0.; 3])
            .is_err()
    );
    assert!(
        project
            .resize_part(u64::MAX, part, [80.; 3], [0.; 3])
            .is_err()
    );
    assert_eq!(project, before);
}

#[test]
fn labels_use_only_equipment_or_peer_node_name_and_peer_is_symmetric() {
    let mut project = Project::demo();
    let cable = project.connections[0].clone();
    let Target::Port(peer) = cable.to else {
        panic!("demo has a device cable")
    };
    assert_eq!(project.connected_peer(cable.from), Some(peer));
    assert_eq!(project.connected_peer(peer), Some(cable.from));
    assert_eq!(
        project.port_label(cable.from),
        project.node(peer.node).unwrap().name
    );
    assert_eq!(
        project.port_label(peer),
        project.node(cable.from.node).unwrap().name
    );
    assert!(
        project
            .target_label(&cable, cable.from)
            .contains(&project.port(peer).unwrap().name)
    );
    let equipment = project
        .connections
        .iter()
        .find(|c| matches!(c.to, Target::Endpoint(_)))
        .unwrap();
    let Target::Endpoint(id) = equipment.to else {
        unreachable!()
    };
    assert_eq!(
        project.port_label(equipment.from),
        project.endpoint(id).unwrap().name
    );
    assert_eq!(project.connected_peer(equipment.from), None);
    let free = PortRef {
        node: project.nodes[0].id,
        port: project.nodes[0].ports[1].id,
    };
    assert_eq!(project.port_label(free), "not connected");
    assert_eq!(project.connected_peer(free), None);
    project.disconnect(peer);
    assert_eq!(project.port_label(cable.from), "not connected");
    assert_eq!(project.connected_peer(peer), None);
}

#[test]
fn product_default_names_and_face_names_are_english() {
    let project = Project::demo();
    assert_eq!(Project::default().name, "My cable project");
    assert_eq!(project.name, "Desktop · example");
    assert_eq!(project.nodes[0].parts[0].name, "Enclosure");
    assert_eq!(Face::Front.name(), "Front");
    assert_eq!(Face::Left.name(), "Left");
    assert_eq!(PortKind::Jack35.name(), "3.5 mm jack");
    assert_eq!(PortKind::DcBarrel.name(), "DC barrel");
}
