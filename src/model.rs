use crate::ports::PortKind;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

pub type Id = u64;
pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Face {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}
impl Face {
    pub const ALL: [Self; 6] = [
        Self::Front,
        Self::Back,
        Self::Left,
        Self::Right,
        Self::Top,
        Self::Bottom,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Front => "Front",
            Self::Back => "Back",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Top => "Top",
            Self::Bottom => "Bottom",
        }
    }
    pub fn axes(self) -> (usize, usize) {
        match self {
            Self::Front | Self::Back => (0, 1),
            Self::Left | Self::Right => (2, 1),
            Self::Top | Self::Bottom => (0, 2),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Primitive {
    Box,
    Wedge,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub id: Id,
    pub name: String,
    pub shape: Primitive,
    pub size: [f32; 3],
    pub offset: [f32; 3],
}
impl Part {
    fn validate_geometry(&self) -> Result<(), String> {
        if self
            .size
            .iter()
            .any(|x| !x.is_finite() || *x < 4. || *x > 5000.)
            || self
                .offset
                .iter()
                .any(|x| !x.is_finite() || x.abs() > 10000.)
        {
            return Err(
                "Enclosure dimensions must be 4–5,000 mm and its position within ±10,000 mm."
                    .into(),
            );
        }
        Ok(())
    }

    pub fn surface_size(&self, face: Face, uv: [f32; 2]) -> [f32; 2] {
        let (a, b) = face.axes();
        let mut size = [self.size[a], self.size[b]];
        if self.shape == Primitive::Wedge {
            match face {
                Face::Front => size[1] *= 0.5,
                Face::Left => size[1] *= 0.75 - uv[0] * 0.5,
                Face::Right => size[1] *= 0.75 + uv[0] * 0.5,
                Face::Top => size[1] *= (1. + (self.size[1] / (2. * self.size[2])).powi(2)).sqrt(),
                _ => {}
            }
        }
        size
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Port {
    pub id: Id,
    pub name: String,
    pub kind: PortKind,
    pub part: Id,
    pub face: Face,
    pub uv: [f32; 2],
    pub angle: f32,
    /// Screen-plane displacement in millimetres from the socket; each coordinate is within ±10,000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_offset: Option<[f32; 2]>,
    pub metadata: String,
}
impl Port {
    pub fn fits_on(&self, part: &Part) -> bool {
        if !self.angle.is_finite() || self.uv.iter().any(|v| !v.is_finite()) {
            return false;
        }
        let size = part.surface_size(self.face, self.uv);
        let ext = self.half_extents();
        self.uv[0].abs() * size[0] + ext[0] <= size[0] * 0.5 + 0.01
            && (self.uv[1] + 0.5) * size[1] >= ext[1] - 0.01
            && (0.5 - self.uv[1]) * size[1] >= self.top_clearance(part) - 0.01
    }
    fn top_clearance(&self, part: &Part) -> f32 {
        if part.shape == Primitive::Wedge && matches!(self.face, Face::Left | Face::Right) {
            let slope =
                part.size[1] / (2. * part.size[2]) * if self.face == Face::Left { 1. } else { -1. };
            let [w, h] = self.kind.dimensions();
            let (s, c) = self.angle.to_radians().sin_cos();
            (s + slope * c).abs() * w / 2. + (c - slope * s).abs() * h / 2.
        } else {
            self.half_extents()[1]
        }
    }
    pub fn half_extents(&self) -> [f32; 2] {
        let [w, h] = self.kind.dimensions();
        let (s, c) = self.angle.to_radians().sin_cos();
        [
            (w * c.abs() + h * s.abs()) / 2.,
            (w * s.abs() + h * c.abs()) / 2.,
        ]
    }
    pub fn clamp_to_part(&mut self, part: &Part) {
        let ext = self.half_extents();
        let width = part.surface_size(self.face, self.uv)[0];
        let max_u = (0.5 - ext[0] / width).max(0.);
        self.uv[0] = self.uv[0].clamp(-max_u, max_u);
        let height = part.surface_size(self.face, self.uv)[1];
        let min_v = -0.5 + ext[1] / height;
        let max_v = 0.5 - self.top_clearance(part) / height;
        self.uv[1] = if min_v <= max_v {
            self.uv[1].clamp(min_v, max_v)
        } else {
            0.
        };
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub name: String,
    pub parts: Vec<Part>,
    pub ports: Vec<Port>,
    pub default_view: Face,
    pub notes: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: Id,
    pub name: String,
    pub notes: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortRef {
    pub node: Id,
    pub port: Id,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Port(PortRef),
    Endpoint(Id),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub id: Id,
    pub from: PortRef,
    pub to: Target,
    pub supplementary: bool,
    pub adapter: bool,
    pub label: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub next_id: Id,
    pub roots: Vec<Id>,
    pub nodes: Vec<Node>,
    pub endpoints: Vec<Endpoint>,
    pub connections: Vec<Connection>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            name: "My cable project".into(),
            next_id: 1,
            roots: vec![],
            nodes: vec![],
            endpoints: vec![],
            connections: vec![],
        }
    }
}
impl Project {
    pub fn place_port(
        &mut self,
        node: Id,
        kind: PortKind,
        part: Id,
        face: Face,
        uv: [f32; 2],
    ) -> Result<Id, String> {
        self.place_port_rotated(node, kind, part, face, uv, 0.)
    }
    /// Place a library connector at its chosen angle, validating the rotated fit atomically.
    pub fn place_port_rotated(
        &mut self,
        node: Id,
        kind: PortKind,
        part: Id,
        face: Face,
        uv: [f32; 2],
        angle: f32,
    ) -> Result<Id, String> {
        if !angle.is_finite() {
            return Err("Port rotation must be finite.".into());
        }
        if uv.iter().any(|v| !v.is_finite() || v.abs() > 0.5) {
            return Err("Drop the port on a visible device surface.".into());
        }
        let n = self.node(node).ok_or("The device does not exist.")?;
        let body = n
            .parts
            .iter()
            .find(|p| p.id == part)
            .ok_or("The part does not exist.")?
            .clone();
        let mut index = 1;
        let name = loop {
            let name = format!("{} {}", kind.name(), index);
            if !n.ports.iter().any(|p| p.name == name) {
                break name;
            }
            index += 1;
        };
        let mut candidate = self.clone();
        let id = candidate.add_port(node, kind, &name)?;
        let port = candidate.port_mut(PortRef { node, port: id }).unwrap();
        port.part = part;
        port.face = face;
        port.uv = uv;
        port.angle = angle.rem_euclid(360.);
        port.clamp_to_part(&body);
        candidate.validate()?;
        *self = candidate;
        Ok(id)
    }
    pub fn label_port(&mut self, port: PortRef, text: &str) -> Result<(), String> {
        if self.port(port).is_none() {
            return Err("The port no longer exists.".into());
        }
        let text = text.trim();
        let mut candidate = self.clone();
        if let Some(connection) = candidate.connection_at(port).cloned() {
            let Target::Endpoint(id) = connection.to else {
                return Err("The port is connected to a mapped device. Disconnect the cable before replacing it with an equipment label.".into());
            };
            if text.is_empty() {
                candidate.disconnect(port);
            } else {
                candidate
                    .endpoints
                    .iter_mut()
                    .find(|e| e.id == id)
                    .unwrap()
                    .name = text.into();
            }
        } else if !text.is_empty() {
            candidate.connect_endpoint(port, text)?;
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub fn allocate(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    pub fn node(&self, id: Id) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
    pub fn node_mut(&mut self, id: Id) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }
    pub fn port(&self, r: PortRef) -> Option<&Port> {
        self.node(r.node)?.ports.iter().find(|p| p.id == r.port)
    }
    pub fn port_mut(&mut self, r: PortRef) -> Option<&mut Port> {
        self.node_mut(r.node)?
            .ports
            .iter_mut()
            .find(|p| p.id == r.port)
    }
    pub fn endpoint(&self, id: Id) -> Option<&Endpoint> {
        self.endpoints.iter().find(|e| e.id == id)
    }

    pub fn add_node(&mut self, name: &str, size: [f32; 3]) -> Id {
        let id = self.allocate();
        let part = self.allocate();
        self.nodes.push(Node {
            id,
            name: name.into(),
            parts: vec![Part {
                id: part,
                name: "Enclosure".into(),
                shape: Primitive::Box,
                size,
                offset: [0.; 3],
            }],
            ports: vec![],
            default_view: Face::Back,
            notes: String::new(),
        });
        self.roots.push(id);
        id
    }
    pub fn add_part(&mut self, node: Id, shape: Primitive) -> Result<Id, String> {
        if self.node(node).is_none() {
            return Err("The device does not exist.".into());
        }
        let id = self.allocate();
        self.node_mut(node).unwrap().parts.push(Part {
            id,
            name: "Part".into(),
            shape,
            size: [60., 40., 60.],
            offset: [0., 50., 0.],
        });
        Ok(id)
    }
    pub fn add_port(&mut self, node: Id, kind: PortKind, name: &str) -> Result<Id, String> {
        let n = self.node(node).ok_or("The device does not exist.")?;
        let part = n.parts.first().ok_or("Create an enclosure first.")?.id;
        let id = self.allocate();
        let mut p = Port {
            id,
            name: name.into(),
            kind,
            part,
            face: Face::Back,
            uv: [0.; 2],
            angle: 0.,
            label_offset: None,
            metadata: String::new(),
        };
        p.clamp_to_part(&self.node(node).unwrap().parts[0]);
        self.node_mut(node).unwrap().ports.push(p);
        Ok(id)
    }

    pub fn connection_at(&self, r: PortRef) -> Option<&Connection> {
        self.connections
            .iter()
            .find(|c| c.from == r || c.to == Target::Port(r))
    }
    pub fn is_occupied(&self, r: PortRef) -> bool {
        self.connection_at(r).is_some()
    }

    pub fn connected_peer(&self, r: PortRef) -> Option<PortRef> {
        let connection = self.connection_at(r)?;
        let Target::Port(to) = connection.to else {
            return None;
        };
        Some(if connection.from == r {
            to
        } else {
            connection.from
        })
    }

    /// The connected equipment or device name shown next to a physical socket.
    pub fn port_label(&self, r: PortRef) -> String {
        let name = self
            .connection_at(r)
            .and_then(|connection| match connection.to {
                Target::Endpoint(id) => self.endpoint(id).map(|endpoint| endpoint.name.as_str()),
                Target::Port(to) => {
                    let peer = if connection.from == r {
                        to
                    } else {
                        connection.from
                    };
                    self.node(peer.node).map(|node| node.name.as_str())
                }
            });
        name.unwrap_or("not connected").into()
    }

    /// Set an absolute angle in degrees, keeping the socket on its supporting surface.
    pub fn rotate_port(&mut self, r: PortRef, degrees: f32) -> Result<(), String> {
        if !degrees.is_finite() {
            return Err("Port rotation must be finite.".into());
        }
        let port = self.port(r).ok_or("The port no longer exists.")?;
        let part = self
            .node(r.node)
            .and_then(|node| node.parts.iter().find(|part| part.id == port.part))
            .ok_or("The port's supporting part is missing.")?
            .clone();
        let mut candidate = self.clone();
        let port = candidate.port_mut(r).unwrap();
        port.angle = degrees.rem_euclid(360.);
        port.clamp_to_part(&part);
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    /// Resize and move a part, clamping every socket attached to it as one atomic edit.
    pub fn resize_part(
        &mut self,
        node: Id,
        part: Id,
        size: [f32; 3],
        offset: [f32; 3],
    ) -> Result<(), String> {
        let mut candidate = self.clone();
        let device = candidate
            .node_mut(node)
            .ok_or("The device does not exist.")?;
        let body = device
            .parts
            .iter_mut()
            .find(|body| body.id == part)
            .ok_or("The part does not exist.")?;
        body.size = size;
        body.offset = offset;
        body.validate_geometry()?;
        for port in device.ports.iter_mut().filter(|port| port.part == part) {
            port.clamp_to_part(body);
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    pub fn connect(
        &mut self,
        from: PortRef,
        to: Target,
        supplementary: bool,
        adapter: bool,
    ) -> Result<Id, String> {
        // Validate a candidate so every failure leaves identities and data unchanged.
        let mut candidate = self.clone();
        let id = candidate.allocate();
        candidate.connections.push(Connection {
            id,
            from,
            to,
            supplementary,
            adapter,
            label: String::new(),
        });
        candidate.refresh_roots();
        candidate.validate()?;
        *self = candidate;
        Ok(id)
    }
    pub fn connect_endpoint(&mut self, from: PortRef, name: &str) -> Result<Id, String> {
        if name.trim().is_empty() {
            return Err("Enter the equipment name.".into());
        }
        let mut p = self.clone();
        let endpoint = p.allocate();
        p.endpoints.push(Endpoint {
            id: endpoint,
            name: name.trim().into(),
            notes: String::new(),
        });
        p.connect(from, Target::Endpoint(endpoint), false, false)?;
        *self = p;
        Ok(endpoint)
    }
    pub fn disconnect(&mut self, r: PortRef) {
        self.connections
            .retain(|c| c.from != r && c.to != Target::Port(r));
        self.clean_endpoints();
        self.refresh_roots();
    }
    fn clean_endpoints(&mut self) {
        self.endpoints.retain(|e| {
            self.connections
                .iter()
                .any(|c| c.to == Target::Endpoint(e.id))
        });
    }
    pub fn refresh_roots(&mut self) {
        let children: HashSet<_> = self
            .connections
            .iter()
            .filter(|c| !c.supplementary)
            .filter_map(|c| match c.to {
                Target::Port(r) => Some(r.node),
                _ => None,
            })
            .collect();
        self.roots = self
            .nodes
            .iter()
            .filter(|n| !children.contains(&n.id))
            .map(|n| n.id)
            .collect();
    }
    pub fn remove_port(&mut self, r: PortRef) {
        self.disconnect(r);
        if let Some(n) = self.node_mut(r.node) {
            n.ports.retain(|p| p.id != r.port);
        }
    }
    pub fn remove_part(&mut self, node: Id, part: Id) -> Result<(), String> {
        let n = self.node(node).ok_or("The device does not exist.")?;
        if n.parts.len() <= 1 {
            return Err("The device must have at least one part.".into());
        }
        let ports: Vec<_> = n
            .ports
            .iter()
            .filter(|p| p.part == part)
            .map(|p| p.id)
            .collect();
        for port in ports {
            self.remove_port(PortRef { node, port });
        }
        self.node_mut(node).unwrap().parts.retain(|p| p.id != part);
        Ok(())
    }
    pub fn remove_node(&mut self, id: Id) {
        self.connections
            .retain(|c| c.from.node != id && !matches!(c.to,Target::Port(r) if r.node==id));
        self.nodes.retain(|n| n.id != id);
        self.clean_endpoints();
        self.refresh_roots();
    }
    pub fn parent(&self, id: Id) -> Option<&Connection> {
        self.connections
            .iter()
            .find(|c| !c.supplementary && matches!(c.to,Target::Port(r) if r.node==id))
    }
    pub fn ancestors(&self, id: Id) -> Vec<Id> {
        let mut path = vec![id];
        let mut seen = HashSet::from([id]);
        let mut at = id;
        while let Some(c) = self.parent(at) {
            at = c.from.node;
            if !seen.insert(at) {
                break;
            }
            path.push(at);
        }
        path.reverse();
        path
    }
    pub fn route_to_endpoint(&self, id: Id) -> Vec<PortRef> {
        let Some(c) = self
            .connections
            .iter()
            .find(|c| c.to == Target::Endpoint(id))
        else {
            return vec![];
        };
        let mut route = vec![c.from];
        let mut at = c.from.node;
        let mut seen = HashSet::from([at]);
        while let Some(parent) = self.parent(at) {
            route.push(parent.from);
            at = parent.from.node;
            if !seen.insert(at) {
                break;
            }
        }
        route.reverse();
        route
    }
    pub fn target_label(&self, c: &Connection, from: PortRef) -> String {
        match c.to {
            Target::Endpoint(id) => self
                .endpoint(id)
                .map(|e| e.name.clone())
                .unwrap_or_default(),
            Target::Port(to) => {
                let r = if c.from == from { to } else { c.from };
                format!(
                    "{} · {}",
                    self.node(r.node).map(|n| n.name.as_str()).unwrap_or("?"),
                    self.port(r).map(|p| p.name.as_str()).unwrap_or("?")
                )
            }
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != FORMAT_VERSION {
            return Err(format!(
                "Project version {} is unsupported (this app uses {}).",
                self.version, FORMAT_VERSION
            ));
        }
        if self.name.trim().is_empty() {
            return Err("The project must have a name.".into());
        }
        if self.nodes.len() > 5000 || self.connections.len() > 100000 {
            return Err(
                "The project is too large (maximum 5,000 devices / 100,000 cables).".into(),
            );
        }
        let mut ids = HashSet::new();
        let mut take = |id| {
            if id == 0 || !ids.insert(id) {
                Err("The project contains invalid or duplicate IDs.".to_string())
            } else {
                Ok(())
            }
        };
        let mut ports = HashMap::new();
        let mut nodes = HashSet::new();
        let mut endpoint_ids = HashSet::new();
        for n in &self.nodes {
            take(n.id)?;
            nodes.insert(n.id);
            if n.name.trim().is_empty() || n.parts.is_empty() {
                return Err("Every device needs a name and an enclosure.".into());
            }
            let mut parts = HashMap::new();
            for part in &n.parts {
                take(part.id)?;
                parts.insert(part.id, part);
                part.validate_geometry()?;
            }
            for p in &n.ports {
                take(p.id)?;
                if p.name.trim().is_empty() {
                    return Err("The port must have a name.".into());
                }
                let part = parts
                    .get(&p.part)
                    .ok_or("A port is missing its supporting part.")?;
                if !p.angle.is_finite() || p.uv.iter().any(|v| !v.is_finite()) {
                    return Err("The port position or rotation is invalid.".into());
                }
                if p.label_offset
                    .is_some_and(|offset| offset.iter().any(|v| !v.is_finite() || v.abs() > 10000.))
                {
                    return Err(
                        "Port label coordinates must be finite and within ±10,000 mm.".into(),
                    );
                }
                if !p.fits_on(part) {
                    return Err(format!(
                        "Port '{}' does not fit on the surface. Enlarge the enclosure or move the port.",
                        p.name
                    ));
                }
                ports.insert(
                    PortRef {
                        node: n.id,
                        port: p.id,
                    },
                    p.kind,
                );
            }
        }
        for e in &self.endpoints {
            take(e.id)?;
            endpoint_ids.insert(e.id);
            if e.name.trim().is_empty() {
                return Err("The equipment name is missing.".into());
            }
        }
        let mut used = HashSet::new();
        let mut used_ends = HashSet::new();
        let mut parents = HashMap::new();
        let mut edges: HashMap<Id, Vec<Id>> = HashMap::new();
        for c in &self.connections {
            take(c.id)?;
            let from = ports
                .get(&c.from)
                .ok_or("A cable source port is missing.")?;
            if !used.insert(c.from) {
                return Err(
                    "A physical port is already occupied. Disconnect the cable first.".into(),
                );
            }
            match c.to {
                Target::Endpoint(id) => {
                    if !endpoint_ids.contains(&id) {
                        return Err("The cable equipment endpoint is missing.".into());
                    }
                    if !used_ends.insert(id) {
                        return Err("The equipment is already connected.".into());
                    }
                    if c.supplementary {
                        return Err("Equipment does not need a supplementary link.".into());
                    }
                }
                Target::Port(to) => {
                    let kind = ports
                        .get(&to)
                        .ok_or("A cable destination port is missing.")?;
                    if !used.insert(to) {
                        return Err(
                            "A physical port is already occupied. Disconnect the cable first."
                                .into(),
                        );
                    }
                    if c.from.node == to.node {
                        return Err("Choose a port on another device.".into());
                    }
                    if !c.adapter && !from.compatible(*kind) {
                        return Err("These port types require an adapter. Enable the adapter option if the cable converts between them.".into());
                    }
                    if !c.supplementary {
                        if parents.insert(to.node, c.from.node).is_some() {
                            return Err("The device already has a parent connection. Use an explicit supplementary link.".into());
                        }
                        edges.entry(c.from.node).or_default().push(to.node);
                    }
                }
            }
        }
        let mut roots = HashSet::new();
        for id in &self.roots {
            if !nodes.contains(id) || !roots.insert(*id) || parents.contains_key(id) {
                return Err("The project root devices are invalid.".into());
            }
        }
        if self
            .nodes
            .iter()
            .any(|n| !parents.contains_key(&n.id) && !roots.contains(&n.id))
        {
            return Err("A standalone device is missing from the project roots.".into());
        }
        let mut degree: HashMap<_, usize> = nodes
            .iter()
            .map(|id| (*id, usize::from(parents.contains_key(id))))
            .collect();
        let mut queue: VecDeque<_> = degree
            .iter()
            .filter(|(_, v)| **v == 0)
            .map(|(k, _)| *k)
            .collect();
        let mut count = 0;
        while let Some(id) = queue.pop_front() {
            count += 1;
            if let Some(children) = edges.get(&id) {
                for child in children {
                    let d = degree.get_mut(child).unwrap();
                    *d -= 1;
                    if *d == 0 {
                        queue.push_back(*child);
                    }
                }
            }
        }
        if count != nodes.len() {
            return Err("The connection creates a cycle in the tree. Choose a supplementary link if this wiring is intentional.".into());
        }
        if self.next_id <= ids.iter().copied().max().unwrap_or(0) || self.next_id == u64::MAX {
            return Err("The project ID counter is invalid.".into());
        }
        Ok(())
    }

    pub fn demo() -> Self {
        let mut p = Self {
            name: "Desktop · example".into(),
            ..Self::default()
        };
        let pc = p.add_node("AMX-PC", [210., 420., 380.]);
        let hub = p.add_node("Desktop USB hub", [105., 25., 45.]);
        for (name, kind, uv) in [
            ("USB-A 1", PortKind::UsbA, [-0.22, 0.2]),
            ("USB-A 2", PortKind::UsbA, [-0.12, 0.2]),
            ("HDMI 1", PortKind::Hdmi, [-0.2, -0.18]),
            ("HDMI 2", PortKind::Hdmi, [-0.08, -0.18]),
            ("Ethernet", PortKind::Rj45, [0.14, 0.2]),
        ] {
            let id = p.add_port(pc, kind, name).unwrap();
            p.port_mut(PortRef { node: pc, port: id }).unwrap().uv = uv;
        }
        let upstream = p.add_port(hub, PortKind::UsbC, "USB-C upstream").unwrap();
        for i in 0..4 {
            let id = p
                .add_port(hub, PortKind::UsbA, &format!("USB-A {}", i + 1))
                .unwrap();
            let port = p
                .port_mut(PortRef {
                    node: hub,
                    port: id,
                })
                .unwrap();
            port.face = Face::Front;
            port.uv = [-0.34 + i as f32 * 0.225, 0.];
        }
        let pc_port = p.nodes[0].ports[0].id;
        p.connect(
            PortRef {
                node: pc,
                port: pc_port,
            },
            Target::Port(PortRef {
                node: hub,
                port: upstream,
            }),
            false,
            false,
        )
        .unwrap();
        let camera = p.nodes[1].ports[3].id;
        p.connect_endpoint(
            PortRef {
                node: hub,
                port: camera,
            },
            "Webcam",
        )
        .unwrap();
        p
    }
}
