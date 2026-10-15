use cable_management::{geometry::*, model::*, ports::PortKind};

#[test]
fn palette_drop_places_port_on_the_requested_part_and_surface() {
    let mut p = Project::default();
    let id = p.add_node("Hubb", [120., 30., 50.]);
    let part = p.node(id).unwrap().parts[0].id;
    let port = p
        .place_port(id, PortKind::UsbA, part, Face::Top, [0.2, -0.1])
        .unwrap();
    let placed = p.port(PortRef { node: id, port }).unwrap();
    assert_eq!(placed.part, part);
    assert_eq!(placed.face, Face::Top);
    assert_eq!(placed.uv, [0.2, -0.1]);
    assert_eq!(placed.kind.dimensions(), [12., 4.5]);
    p.validate().unwrap();
}
#[test]
fn invalid_drop_preserves_project_and_identity_counter() {
    let mut p = Project::default();
    let id = p.add_node("Liten", [10., 10., 10.]);
    let part = p.node(id).unwrap().parts[0].id;
    let before = p.clone();
    assert!(
        p.place_port(id, PortKind::Xlr, part, Face::Front, [0., 0.])
            .is_err()
    );
    assert_eq!(p, before);
    assert!(
        p.place_port(id, PortKind::UsbC, part, Face::Front, [3., 0.])
            .is_err()
    );
    assert_eq!(p, before);
}
#[test]
fn repeated_port_types_keep_distinct_names_after_deletion() {
    let mut p = Project::default();
    let id = p.add_node("PC", [200., 300., 200.]);
    let part = p.node(id).unwrap().parts[0].id;
    let first = p
        .place_port(id, PortKind::Hdmi, part, Face::Back, [0., 0.])
        .unwrap();
    let second = p
        .place_port(id, PortKind::Hdmi, part, Face::Back, [0.2, 0.])
        .unwrap();
    p.remove_port(PortRef {
        node: id,
        port: first,
    });
    let third = p
        .place_port(id, PortKind::Hdmi, part, Face::Back, [-0.2, 0.])
        .unwrap();
    assert_ne!(
        p.port(PortRef {
            node: id,
            port: second
        })
        .unwrap()
        .name,
        p.port(PortRef {
            node: id,
            port: third
        })
        .unwrap()
        .name
    );
}
#[test]
fn typing_gaming_mouse_creates_only_a_label_and_can_rename_or_clear_it() {
    let mut p = Project::demo();
    let r = PortRef {
        node: p.nodes[0].id,
        port: p.nodes[0].ports[1].id,
    };
    let node_count = p.nodes.len();
    p.label_port(r, "  gamingmus  ").unwrap();
    assert_eq!(p.nodes.len(), node_count);
    let Target::Endpoint(endpoint) = p.connection_at(r).unwrap().to else {
        panic!("must be a text endpoint")
    };
    assert_eq!(p.endpoint(endpoint).unwrap().name, "gamingmus");
    p.label_port(r, "Trådlös gamingmus").unwrap();
    assert_eq!(p.endpoint(endpoint).unwrap().name, "Trådlös gamingmus");
    let roundtrip: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(roundtrip, p);
    p.label_port(r, "").unwrap();
    assert!(p.connection_at(r).is_none());
    assert!(p.endpoint(endpoint).is_none());
    p.validate().unwrap();
}
#[test]
fn text_edit_cannot_replace_a_mapped_hub_connection() {
    let mut p = Project::demo();
    let r = p.connections[0].from;
    let before = p.clone();
    assert!(p.label_port(r, "gamingmus").is_err());
    assert_eq!(p, before);
}
#[test]
fn dropping_uses_visible_surface_in_each_of_the_six_fixed_views() {
    let mut p = Project::default();
    let id = p.add_node("PC", [200., 300., 200.]);
    let n = p.node(id).unwrap();
    for face in Face::ALL {
        let camera = Camera::for_face(face);
        let hit = surface_at(n, &camera, [0., 0.]).unwrap();
        assert_eq!(hit.face, face);
        assert_eq!(hit.part, n.parts[0].id);
        assert!(hit.uv.iter().all(|v| v.abs() < 0.001));
    }
    assert!(surface_at(n, &Camera::for_face(Face::Back), [900., 900.]).is_none());
}
#[test]
fn drop_selects_nearest_visible_building_block() {
    let mut p = Project::default();
    let id = p.add_node("PC", [200., 300., 200.]);
    let part = p.add_part(id, Primitive::Box).unwrap();
    p.node_mut(id).unwrap().parts[1].offset = [0., 0., -150.];
    assert_eq!(
        surface_at(p.node(id).unwrap(), &Camera::for_face(Face::Back), [0., 0.])
            .unwrap()
            .part,
        part
    );
}
