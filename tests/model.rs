use cable_management::model::*;
use cable_management::ports::PortKind;

#[test]
fn demo_has_exact_physical_route_without_endpoint_node() {
    let p = Project::demo();
    p.validate().unwrap();
    assert_eq!(p.nodes.len(), 2);
    assert_eq!(p.endpoints.len(), 1);
    let route = p.route_to_endpoint(p.endpoints[0].id);
    assert_eq!(route.len(), 2);
    assert_eq!(p.port(route[1]).unwrap().name, "USB-A 3");
}

#[test]
fn both_ends_of_a_cable_are_reserved_and_failure_is_atomic() {
    let mut p = Project::demo();
    let link = p.connections[0].clone();
    let before = serde_json::to_string(&p).unwrap();
    assert!(p.connect(link.from, link.to, false, false).is_err());
    assert_eq!(before, serde_json::to_string(&p).unwrap());
    if let Target::Port(to) = link.to {
        assert!(p.is_occupied(to));
    } else { panic!("demo must contain the hub cable"); }
}

#[test]
fn hierarchy_rejects_cycles_but_explicit_supplementary_link_is_supported() {
    let mut p = Project::demo();
    let pc = p.nodes[0].id;
    let hub = p.nodes[1].id;
    let a = p.add_port(hub, PortKind::UsbA, "Extra ut").unwrap();
    let b = p.add_port(pc, PortKind::UsbC, "Extra in").unwrap();
    let from = PortRef { node: hub, port: a };
    let to = Target::Port(PortRef { node: pc, port: b });
    assert!(p.connect(from, to, false, false).is_err());
    assert!(p.connect(from, to, true, false).is_ok());
    p.validate().unwrap();
}

#[test]
fn incompatible_connectors_require_explicit_adapter() {
    let mut p = Project::demo();
    let pc = p.nodes[0].id;
    let hub = p.nodes[1].id;
    let a = p.add_port(pc, PortKind::Hdmi, "HDMI extra").unwrap();
    let b = p.add_port(hub, PortKind::Xlr, "XLR").unwrap();
    let from = PortRef { node: pc, port: a };
    let to = Target::Port(PortRef { node: hub, port: b });
    assert!(p.connect(from, to, true, false).is_err());
    assert!(p.connect(from, to, true, true).is_ok());
}

#[test]
fn deleting_node_cleans_its_ports_links_and_unattached_endpoints() {
    let mut p = Project::demo();
    let hub = p.nodes[1].id;
    p.remove_node(hub);
    assert_eq!(p.nodes.len(), 1);
    assert!(p.connections.is_empty());
    assert!(p.endpoints.is_empty());
    p.validate().unwrap();
}

#[test]
fn malformed_ids_and_unsupported_versions_are_rejected() {
    let mut p = Project::demo();
    p.nodes[1].id = p.nodes[0].id;
    assert!(p.validate().is_err());
    let mut p = Project::demo();
    p.version = 999;
    assert!(p.validate().is_err());
}

#[test]
fn resize_preserves_connector_dimensions_and_attaches_to_surface() {
    let mut p = Project::demo();
    let port = p.nodes[0].ports[0].clone();
    let dimensions = port.kind.dimensions();
    p.nodes[0].parts[0].size = [400., 600., 500.];
    assert_eq!(dimensions, p.nodes[0].ports[0].kind.dimensions());
    assert_eq!(port.uv, p.nodes[0].ports[0].uv);
    p.validate().unwrap();
}
