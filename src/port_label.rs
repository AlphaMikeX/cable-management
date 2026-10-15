use cable_management::model::*;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionType {
    Equipment,
    Mapped,
}

/// One draft for either a named piece of equipment or a mapped device connection.
pub struct PortLabel {
    pub port: PortRef,
    pub text: String,
    pub kind: ConnectionType,
    pub target_node: Option<Id>,
    pub target_port: Option<Id>,
    pub supplementary: bool,
    pub adapter: bool,
    focus: bool,
}
pub enum Action {
    Save,
    Cancel,
    Open(PortRef),
}
impl PortLabel {
    pub fn has_changes(&self, project: &Project) -> bool {
        match self.kind {
            ConnectionType::Equipment => match project.connection_at(self.port).map(|c| c.to) {
                Some(Target::Endpoint(id)) => project
                    .endpoint(id)
                    .is_some_and(|e| e.name != self.text.trim()),
                Some(Target::Port(_)) => true,
                None => !self.text.trim().is_empty(),
            },
            ConnectionType::Mapped => {
                let target = self
                    .target_node
                    .zip(self.target_port)
                    .map(|(node, port)| PortRef { node, port });
                let connection = project.connection_at(self.port);
                target != project.connected_peer(self.port)
                    || target.is_none()
                    || connection.is_none_or(|c| {
                        c.adapter != self.adapter || c.supplementary != self.supplementary
                    })
            }
        }
    }
    pub fn new(project: &Project, port: PortRef) -> Option<Self> {
        project.port(port)?;
        let connection = project.connection_at(port);
        let peer = project.connected_peer(port);
        let text = connection
            .and_then(|connection| match connection.to {
                Target::Endpoint(id) => project.endpoint(id).map(|e| e.name.clone()),
                _ => None,
            })
            .unwrap_or_default();
        Some(Self {
            port,
            text,
            kind: if peer.is_some() {
                ConnectionType::Mapped
            } else {
                ConnectionType::Equipment
            },
            target_node: peer.map(|p| p.node),
            target_port: peer.map(|p| p.port),
            supplementary: connection.is_some_and(|c| c.supplementary),
            adapter: connection.is_some_and(|c| c.adapter),
            focus: true,
        })
    }
    /// Apply the draft as one transaction. Failure preserves the project and all IDs.
    pub fn apply(&self, project: &mut Project) -> Result<(), String> {
        if project.port(self.port).is_none() {
            return Err("The port no longer exists.".into());
        }
        if !self.has_changes(project) {
            return Ok(());
        }
        let mut candidate = project.clone();
        match self.kind {
            ConnectionType::Equipment => {
                if candidate.connected_peer(self.port).is_some() {
                    candidate.disconnect(self.port);
                }
                candidate.label_port(self.port, &self.text)?;
            }
            ConnectionType::Mapped => {
                let (Some(node), Some(port)) = (self.target_node, self.target_port) else {
                    return Err("Choose a mapped device and a free target port, or cancel the connection draft.".into());
                };
                let target = PortRef { node, port };
                if node == self.port.node || candidate.port(target).is_none() {
                    return Err("Choose a port on another mapped device.".into());
                }
                let peer = candidate.connected_peer(self.port);
                if candidate.is_occupied(target) && peer != Some(target) {
                    return Err("The target port is occupied. Choose a free target port.".into());
                }
                if peer == Some(target) {
                    // A cable is the same cable from either end. Preserve its direction,
                    // identity and label while updating options from the receiving port.
                    let id = candidate.connection_at(self.port).unwrap().id;
                    let connection = candidate
                        .connections
                        .iter_mut()
                        .find(|c| c.id == id)
                        .unwrap();
                    connection.supplementary = self.supplementary;
                    connection.adapter = self.adapter;
                    candidate.refresh_roots();
                } else {
                    let incoming = candidate
                        .connection_at(self.port)
                        .is_some_and(|c| c.from != self.port);
                    candidate.disconnect(self.port);
                    let (from, to) = if incoming {
                        (target, self.port)
                    } else {
                        (self.port, target)
                    };
                    candidate.connect(from, Target::Port(to), self.supplementary, self.adapter)?;
                }
            }
        }
        candidate.validate()?;
        *project = candidate;
        Ok(())
    }
    pub fn preselect_device(&mut self, project: &Project, node: Id) {
        self.kind = ConnectionType::Mapped;
        self.target_node = (node != self.port.node && project.node(node).is_some()).then_some(node);
        self.target_port = None;
        self.select_available_port(project);
    }
    fn available_ports<'a>(&self, project: &'a Project) -> Vec<&'a Port> {
        self.target_node
            .and_then(|node| project.node(node))
            .map(|node| {
                node.ports
                    .iter()
                    .filter(|port| {
                        let reference = PortRef {
                            node: node.id,
                            port: port.id,
                        };
                        !project.is_occupied(reference)
                            || project.connected_peer(self.port) == Some(reference)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    fn select_available_port(&mut self, project: &Project) {
        let ports = self.available_ports(project);
        if self.target_port.is_none() {
            self.target_port = ports.first().map(|p| p.id);
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, project: &Project) -> Option<Action> {
        let port = project.port(self.port)?;
        let mut action = None;
        // Escape closes a dropdown during rendering, so remember its prior state.
        let popup_was_open = egui::Popup::is_any_open(ctx);
        let submit = !popup_was_open
            && self.kind == ConnectionType::Equipment
            && ctx.input(|i| i.key_pressed(egui::Key::Enter));
        egui::Modal::new(egui::Id::new("connection-editor")).show(ctx, |ui| {
            ui.set_min_width(410.);
            ui.heading("Connection");
            ui.label(format!(
                "{} · {} · {}",
                port.name,
                port.kind.name(),
                port.face.name()
            ));
            if let Some(peer) = project.connected_peer(self.port)
                && let Some(node) = project.node(peer.node)
            {
                ui.horizontal(|ui| {
                    ui.label(format!("Connected to {}", node.name));
                    if ui
                        .add_enabled(!self.has_changes(project), egui::Button::new("Open device"))
                        .on_disabled_hover_text(
                            "Save or cancel the connection draft before opening the device.",
                        )
                        .clicked()
                    {
                        action = Some(Action::Open(peer));
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui
                    .selectable_value(&mut self.kind, ConnectionType::Equipment, "Equipment name")
                    .changed()
                {
                    self.focus = true;
                }
                ui.selectable_value(&mut self.kind, ConnectionType::Mapped, "Mapped device");
            });
            let mut can_save = true;
            match self.kind {
                ConnectionType::Equipment => {
                    ui.label("Enter an equipment name, such as gaming mouse or webcam.");
                    let response = ui.add_sized(
                        [410., 30.],
                        egui::TextEdit::singleline(&mut self.text).hint_text("e.g. gaming mouse"),
                    );
                    if self.focus {
                        response.request_focus();
                        self.focus = false;
                    }
                    ui.small(
                        "The name appears beside this port. Clear it to remove the connection.",
                    );
                }
                ConnectionType::Mapped => {
                    let nodes: Vec<_> = project
                        .nodes
                        .iter()
                        .filter(|n| n.id != self.port.node)
                        .collect();
                    if self.target_node.is_none() {
                        self.target_node = nodes.first().map(|n| n.id);
                        self.target_port = None;
                    }
                    ui.label("Device");
                    let before = self.target_node;
                    egui::ComboBox::from_id_salt("connection-target-device")
                        .selected_text(
                            nodes
                                .iter()
                                .find(|n| Some(n.id) == self.target_node)
                                .map(|n| n.name.as_str())
                                .unwrap_or("Create another device first"),
                        )
                        .show_ui(ui, |ui| {
                            for node in nodes {
                                ui.selectable_value(
                                    &mut self.target_node,
                                    Some(node.id),
                                    &node.name,
                                );
                            }
                        });
                    if self.target_node != before {
                        self.target_port = None;
                    }
                    self.select_available_port(project);
                    ui.label("Free port");
                    egui::ComboBox::from_id_salt("connection-target-port")
                        .selected_text(
                            self.target_node
                                .zip(self.target_port)
                                .and_then(|(node, port)| project.port(PortRef { node, port }))
                                .map(|p| format!("{} · {}", p.name, p.kind.name()))
                                .unwrap_or_else(|| "No free target port".into()),
                        )
                        .show_ui(ui, |ui| {
                            for port in self.available_ports(project) {
                                ui.selectable_value(
                                    &mut self.target_port,
                                    Some(port.id),
                                    format!("{} · {}", port.name, port.kind.name()),
                                );
                            }
                        });
                    ui.checkbox(&mut self.adapter, "Adapter / converting cable");
                    ui.checkbox(
                        &mut self.supplementary,
                        "Supplementary connection outside the hierarchy",
                    );
                    let target = self
                        .target_node
                        .zip(self.target_port)
                        .map(|(node, port)| PortRef { node, port });
                    let occupied = target.is_some_and(|target| {
                        project.is_occupied(target)
                            && project.connected_peer(self.port) != Some(target)
                    });
                    can_save = !occupied
                        && target
                            .filter(|target| target.node != self.port.node)
                            .and_then(|target| project.port(target))
                            .is_some_and(|target| {
                                port.kind.compatible(target.kind) || self.adapter
                            });
                    if occupied {
                        ui.colored_label(
                            egui::Color32::from_rgb(236, 174, 100),
                            "The target port is occupied. Choose a free target port.",
                        );
                    } else if self.target_port.is_some() && !can_save {
                        ui.colored_label(
                            egui::Color32::from_rgb(236, 174, 100),
                            "Different connector families: select Adapter to connect them.",
                        );
                    }
                }
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_save, egui::Button::new("Save"))
                    .clicked()
                    || (submit
                        && can_save
                        && self.kind == ConnectionType::Equipment
                        && !egui::Popup::is_any_open(ctx))
                {
                    action = Some(Action::Save);
                }
                if ui.button("Cancel").clicked() {
                    action = Some(Action::Cancel);
                }
            });
            if !popup_was_open
                && !egui::Popup::is_any_open(ctx)
                && ctx.input(|i| i.key_pressed(egui::Key::Escape))
            {
                action = Some(Action::Cancel);
            }
        });
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cable_management::ports::PortKind;
    use egui::{Pos2, Rect, Vec2};
    fn frame(
        ctx: &egui::Context,
        editor: &mut PortLabel,
        project: &Project,
        time: f64,
        events: Vec<egui::Event>,
    ) -> (Option<Action>, egui::FullOutput) {
        let mut action = None;
        let out = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                action = editor.show(ctx, project);
            },
        );
        (action, out)
    }
    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }
    #[test]
    fn dropdown_enter_and_escape_do_not_submit_or_discard_the_connection_draft() {
        let mut project = Project::default();
        let source = project.add_node("Source", [100.; 3]);
        let target = project.add_node("Target device", [100.; 3]);
        let port = project
            .add_port(source, PortKind::UsbA, "Source port")
            .unwrap();
        project
            .add_port(target, PortKind::UsbA, "Target port")
            .unwrap();
        for label in ["Target device", "Target port · USB-A"] {
            for shortcut in [egui::Key::Escape, egui::Key::Enter] {
                let ctx = egui::Context::default();
                let mut editor = PortLabel::new(&project, PortRef { node: source, port }).unwrap();
                editor.preselect_device(&project, target);
                frame(&ctx, &mut editor, &project, 0., vec![]);
                let (_, out) = frame(&ctx, &mut editor, &project, 0.1, vec![]);
                let pos = out
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t) if t.galley.job.text == label => {
                            Some(t.pos + t.galley.size() / 2.)
                        }
                        _ => None,
                    })
                    .unwrap();
                frame(
                    &ctx,
                    &mut editor,
                    &project,
                    0.2,
                    vec![egui::Event::PointerMoved(pos)],
                );
                frame(&ctx, &mut editor, &project, 0.3, vec![button(pos, true)]);
                frame(&ctx, &mut editor, &project, 0.4, vec![button(pos, false)]);
                frame(&ctx, &mut editor, &project, 0.45, vec![]);
                assert!(egui::Popup::is_any_open(&ctx));
                let (action, _) = frame(&ctx, &mut editor, &project, 0.5, vec![key(shortcut)]);
                assert!(
                    action.is_none(),
                    "{label}: {shortcut:?} belongs to the dropdown"
                );
                assert!(editor.has_changes(&project));
                assert!(project.connections.is_empty());
            }
        }
    }
}
