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
            Self::Front => "Fram",
            Self::Back => "Bak",
            Self::Left => "Vänster",
            Self::Right => "Höger",
            Self::Top => "Topp",
            Self::Bottom => "Botten",
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
    pub metadata: String,
}
impl Port {
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
            name: "Mitt kabelprojekt".into(),
            next_id: 1,
            roots: vec![],
            nodes: vec![],
            endpoints: vec![],
            connections: vec![],
        }
    }
}
impl Project {
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
                name: "Hölje".into(),
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
            return Err("Enheten finns inte.".into());
        }
        let id = self.allocate();
        self.node_mut(node).unwrap().parts.push(Part {
            id,
            name: "Byggsten".into(),
            shape,
            size: [60., 40., 60.],
            offset: [0., 50., 0.],
        });
        Ok(id)
    }
    pub fn add_port(&mut self, node: Id, kind: PortKind, name: &str) -> Result<Id, String> {
        let n = self.node(node).ok_or("Enheten finns inte.")?;
        let part = n.parts.first().ok_or("Skapa ett hölje först.")?.id;
        let id = self.allocate();
        let mut p = Port {
            id,
            name: name.into(),
            kind,
            part,
            face: Face::Back,
            uv: [0.; 2],
            angle: 0.,
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
            return Err("Ange slutpunktens namn.".into());
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
        let n = self.node(node).ok_or("Enheten finns inte.")?;
        if n.parts.len() <= 1 {
            return Err("Enheten måste ha minst en byggsten.".into());
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
                "Projektversion {} stöds inte (appen använder {}).",
                self.version, FORMAT_VERSION
            ));
        }
        if self.name.trim().is_empty() {
            return Err("Projektet måste ha ett namn.".into());
        }
        if self.nodes.len() > 5000 || self.connections.len() > 100000 {
            return Err("Projektet är för stort (max 5 000 enheter / 100 000 kablar).".into());
        }
        let mut ids = HashSet::new();
        let mut take = |id| {
            if id == 0 || !ids.insert(id) {
                Err("Projektet innehåller ogiltiga eller duplicerade identiteter.".to_string())
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
                return Err("Varje enhet behöver namn och hölje.".into());
            }
            let mut parts = HashMap::new();
            for part in &n.parts {
                take(part.id)?;
                parts.insert(part.id, part);
                if part
                    .size
                    .iter()
                    .any(|x| !x.is_finite() || *x < 4. || *x > 5000.)
                    || part
                        .offset
                        .iter()
                        .any(|x| !x.is_finite() || x.abs() > 10000.)
                {
                    return Err(
                        "Höljets mått måste vara 4–5 000 mm och positionen inom ±10 000 mm.".into(),
                    );
                }
            }
            for p in &n.ports {
                take(p.id)?;
                if p.name.trim().is_empty() {
                    return Err("Porten måste ha ett namn.".into());
                }
                let part = parts.get(&p.part).ok_or("En port saknar sin byggsten.")?;
                if !p.angle.is_finite() || p.uv.iter().any(|v| !v.is_finite()) {
                    return Err("Portens position eller rotation är ogiltig.".into());
                }
                let size = part.surface_size(p.face, p.uv);
                let ext = p.half_extents();
                if p.uv[0].abs() * size[0] + ext[0] > size[0] * 0.5 + 0.01
                    || (p.uv[1] + 0.5) * size[1] < ext[1] - 0.01
                    || (0.5 - p.uv[1]) * size[1] < p.top_clearance(part) - 0.01
                {
                    return Err(format!(
                        "Porten '{}' ryms inte på ytan. Öka höljet eller flytta porten.",
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
                return Err("Slutpunkten saknar namn.".into());
            }
        }
        let mut used = HashSet::new();
        let mut used_ends = HashSet::new();
        let mut parents = HashMap::new();
        let mut edges: HashMap<Id, Vec<Id>> = HashMap::new();
        for c in &self.connections {
            take(c.id)?;
            let from = ports.get(&c.from).ok_or("En kabels startport saknas.")?;
            if !used.insert(c.from) {
                return Err("En fysisk port är redan upptagen. Koppla loss kabeln först.".into());
            }
            match c.to {
                Target::Endpoint(id) => {
                    if !endpoint_ids.contains(&id) {
                        return Err("Kabelns slutpunkt saknas.".into());
                    }
                    if !used_ends.insert(id) {
                        return Err("Slutpunkten är redan ansluten.".into());
                    }
                    if c.supplementary {
                        return Err("En slutpunkt behöver ingen extralänk.".into());
                    }
                }
                Target::Port(to) => {
                    let kind = ports.get(&to).ok_or("En kabels målport saknas.")?;
                    if !used.insert(to) {
                        return Err(
                            "En fysisk port är redan upptagen. Koppla loss kabeln först.".into(),
                        );
                    }
                    if c.from.node == to.node {
                        return Err("Välj en port på en annan enhet.".into());
                    }
                    if !c.adapter && !from.compatible(*kind) {
                        return Err("Porttyperna kräver en adapter. Markera adapter om kabeln faktiskt konverterar mellan dem.".into());
                    }
                    if !c.supplementary {
                        if parents.insert(to.node, c.from.node).is_some() {
                            return Err("Enheten har redan en överordnad anslutning. Använd en uttrycklig extralänk.".into());
                        }
                        edges.entry(c.from.node).or_default().push(to.node);
                    }
                }
            }
        }
        let mut roots = HashSet::new();
        for id in &self.roots {
            if !nodes.contains(id) || !roots.insert(*id) || parents.contains_key(id) {
                return Err("Projektets huvudnoder är ogiltiga.".into());
            }
        }
        if self
            .nodes
            .iter()
            .any(|n| !parents.contains_key(&n.id) && !roots.contains(&n.id))
        {
            return Err("En fristående enhet saknas bland huvudnoderna.".into());
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
            return Err("Anslutningen skapar en cykel i trädet. Välj extralänk om det är avsiktlig kabeldragning.".into());
        }
        if self.next_id <= ids.iter().copied().max().unwrap_or(0) || self.next_id == u64::MAX {
            return Err("Projektets identitetsräknare är ogiltig.".into());
        }
        Ok(())
    }

    pub fn demo() -> Self {
        let mut p = Self {
            name: "Skrivbord · exempel".into(),
            ..Self::default()
        };
        let pc = p.add_node("AMX-PC", [210., 420., 380.]);
        let hub = p.add_node("USB-hubb på skrivbordet", [105., 25., 45.]);
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
        let upstream = p.add_port(hub, PortKind::UsbC, "USB-C uppströms").unwrap();
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
            "Webbkamera",
        )
        .unwrap();
        p
    }
}
