use crate::scene::{self, ACCENT, MUTED, ORANGE, ViewState};
use crate::{
    palette,
    port_label::{self, PortLabel},
};
use cable_management::{model::*, storage};
use eframe::egui::{self, Color32, Key, Modifiers, RichText, Sense, Stroke, Vec2};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone)]
enum Pending {
    New,
    Demo,
    Open,
    Quit,
    Recovery,
}
#[cfg_attr(windows, allow(dead_code))]
enum FileDialog {
    Save,
    Open,
}

struct RenameDevice {
    node: Id,
    text: String,
    focus: bool,
}
struct RotatePort {
    port: PortRef,
    degrees: f32,
}
struct ConnectPorts {
    source: PortRef,
    target_node: Option<Id>,
    target_port: Option<Id>,
    supplementary: bool,
    adapter: bool,
}

pub struct CableApp {
    project: Project,
    saved: Project,
    path: Option<PathBuf>,
    selected: Option<Id>,
    selected_port: Option<Id>,
    hovered_node: Option<Id>,
    editing: bool,
    view: ViewState,
    undo: Vec<Project>,
    redo: Vec<Project>,
    status: String,
    error: Option<String>,
    pending: Option<Pending>,
    after_save: Option<Pending>,
    file_dialog: Option<FileDialog>,
    file_input: String,
    recovery: Option<Project>,
    last_recovery: Instant,
    closing: bool,
    help: bool,
    port_label: Option<PortLabel>,
    rename_device: Option<RenameDevice>,
    rotate_port: Option<RotatePort>,
    connect_ports: Option<ConnectPorts>,
}

impl CableApp {
    pub fn new(cc: &eframe::CreationContext<'_>, path: Option<PathBuf>) -> Self {
        Self::from_context(&cc.egui_ctx, path)
    }
    fn from_context(ctx: &egui::Context, path: Option<PathBuf>) -> Self {
        ctx.set_visuals(egui::Visuals::dark());
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(9., 9.);
        style.spacing.button_padding = Vec2::new(12., 7.);
        style.visuals.panel_fill = Color32::from_rgb(24, 31, 39);
        style.visuals.window_fill = Color32::from_rgb(28, 36, 45);
        style.visuals.extreme_bg_color = Color32::from_rgb(15, 22, 29);
        style.visuals.selection.bg_fill = Color32::from_rgb(39, 85, 73);
        style.visuals.selection.stroke = Stroke::new(1_f32, ACCENT);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(44, 92, 80);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(48, 61, 72);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(35, 45, 55);
        style.visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(209, 219, 229);
        ctx.set_style(style);
        let mut project = Project::default();
        let mut error = None;
        let mut loaded_path = None;
        if let Some(file) = path {
            match storage::load(&file) {
                Ok(p) => {
                    project = p;
                    loaded_path = Some(file);
                }
                Err(e) => error = Some(e),
            }
        }
        let saved = project.clone();
        let selected = project.roots.first().copied();
        let recovery = storage::load(&storage::recovery_path()).ok();
        Self {
            project,
            saved,
            path: loaded_path,
            selected,
            selected_port: None,
            hovered_node: None,
            editing: false,
            view: ViewState::default(),
            undo: vec![],
            redo: vec![],
            status: "Local project · no account needed".into(),
            error,
            pending: None,
            after_save: None,
            file_dialog: None,
            file_input: String::new(),
            recovery,
            last_recovery: Instant::now(),
            closing: false,
            help: false,
            port_label: None,
            rename_device: None,
            rotate_port: None,
            connect_ports: None,
        }
    }
    fn dirty(&self) -> bool {
        self.project != self.saved || self.has_drafts()
    }
    fn has_drafts(&self) -> bool {
        self.port_label
            .as_ref()
            .is_some_and(|e| e.has_changes(&self.project))
            || self.rename_device.as_ref().is_some_and(|draft| {
                self.project
                    .node(draft.node)
                    .is_some_and(|n| n.name != draft.text.trim())
            })
    }
    fn with_drafts(&self) -> Project {
        let mut project = self.project.clone();
        if let Some(draft) = &self.rename_device
            && let Some(node) = project.node_mut(draft.node)
        {
            node.name = draft.text.trim().into();
        }
        if let Some(editor) = &self.port_label
            && editor.has_changes(&project)
        {
            let _ = project.label_port(editor.port, &editor.text);
        }
        project
    }
    fn flush_draft(&mut self) -> bool {
        !self.has_drafts() || self.change(|_| Ok(()))
    }
    fn remember(&mut self) {
        self.undo.push(self.project.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    fn change(&mut self, operation: impl FnOnce(&mut Project) -> Result<(), String>) -> bool {
        let mut p = self.with_drafts();
        match operation(&mut p).and_then(|_| p.validate()) {
            Ok(()) => {
                if p != self.project {
                    self.remember();
                    self.project = p;
                    self.rename_device = None;
                }
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
    fn select(&mut self, node: Id, port: Option<Id>) {
        if self.selected != Some(node) && !self.flush_draft() {
            return;
        }
        if self.selected != Some(node) {
            self.selected = Some(node);
            self.rename_device = None;
        }
        self.selected_port = port;
        if let Some(p) = port.and_then(|id| self.project.port(PortRef { node, port: id })) {
            if let Some(n) = self.project.node(node) {
                self.view.select(n);
            }
            self.view.set_face(p.face);
        }
    }
    fn reset_document(&mut self, p: Project, path: Option<PathBuf>, dirty: bool) {
        self.project = p;
        self.saved = if dirty {
            Project::default()
        } else {
            self.project.clone()
        };
        self.path = path;
        self.port_label = None;
        self.rename_device = None;
        self.rotate_port = None;
        self.connect_ports = None;
        self.hovered_node = None;
        self.selected = self.project.roots.first().copied();
        self.selected_port = None;
        self.undo.clear();
        self.redo.clear();
        self.view = ViewState::default();
        self.last_recovery = Instant::now();
    }
    fn begin(&mut self, action: Pending) {
        if self.dirty() {
            self.pending = Some(action);
        } else {
            self.perform(action);
        }
    }
    fn clear_recovery(&mut self) {
        let p = storage::recovery_path();
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(storage::backup_path(&p));
        self.recovery = None;
    }
    fn perform(&mut self, action: Pending) {
        match action {
            Pending::New => {
                self.clear_recovery();
                self.reset_document(Project::default(), None, false);
                self.editing = true;
            }
            Pending::Demo => {
                self.clear_recovery();
                self.reset_document(Project::demo(), None, true);
                self.editing = false;
            }
            Pending::Recovery => {
                if let Some(p) = self.recovery.take() {
                    self.reset_document(p, None, true);
                    self.status = "Draft recovered. Save it as a project.".into();
                }
            }
            Pending::Open => self.open_dialog(),
            Pending::Quit => {
                self.clear_recovery();
                self.closing = true;
            }
        }
    }
    fn open_dialog(&mut self) {
        #[cfg(windows)]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Cable Management", &["json"])
                .pick_file()
            {
                self.load_path(path);
            }
        }
        #[cfg(not(windows))]
        {
            self.file_input = self
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            self.file_dialog = Some(FileDialog::Open);
        }
    }
    fn load_path(&mut self, path: PathBuf) {
        match storage::load(&path) {
            Ok(p) => {
                self.clear_recovery();
                self.reset_document(p, Some(path), false);
                self.status = "Project opened.".into();
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn save(&mut self, save_as: bool) {
        if !self.flush_draft() {
            self.after_save = None;
            return;
        }
        if !save_as && let Some(path) = self.path.clone() {
            self.save_path(path);
            return;
        }
        #[cfg(windows)]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Cable Management", &["json"])
                .set_file_name("My project.cable.json")
                .save_file()
            {
                self.save_path(path);
            } else {
                self.after_save = None;
            }
        }
        #[cfg(not(windows))]
        {
            self.file_input = self
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "My project.cable.json".into());
            self.file_dialog = Some(FileDialog::Save);
        }
    }
    fn save_path(&mut self, path: PathBuf) {
        match storage::save(&path, &self.project) {
            Ok(()) => {
                self.path = Some(path);
                self.saved = self.project.clone();
                self.clear_recovery();
                self.status = "Project saved. The previous version is kept as .bak.".into();
                if let Some(action) = self.after_save.take() {
                    self.perform(action);
                }
            }
            Err(e) => {
                self.error = Some(e);
                self.after_save = None;
            }
        }
    }
    fn add_node(&mut self, name: &str, size: [f32; 3]) {
        let mut id = 0;
        if self.change(|p| {
            id = p.add_node(name, size);
            Ok(())
        }) {
            self.select(id, None);
            self.editing = true;
        }
    }
    fn history(&mut self, redo: bool) {
        self.view.cancel_gestures();
        let previous = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(p) = previous {
            if redo {
                self.undo.push(self.project.clone());
            } else {
                self.redo.push(self.project.clone());
            }
            self.project = p;
            self.port_label = None;
            self.rename_device = None;
            self.rotate_port = None;
            self.connect_ports = None;
            self.hovered_node = None;
            if !self
                .selected
                .is_some_and(|id| self.project.node(id).is_some())
            {
                self.selected = self.project.roots.first().copied();
                self.selected_port = None;
            }
        }
    }

    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(6.);
            ui.horizontal(|ui| {
                ui.label(RichText::new("CM").size(27.).color(ACCENT));
                ui.label(RichText::new("CABLE MANAGEMENT").size(18.).strong());
                ui.add_space(18.);
                if ui.button("New").clicked() {
                    self.begin(Pending::New);
                }
                if ui.button("Open").clicked() {
                    self.begin(Pending::Open);
                }
                if ui.button("Save").clicked() {
                    self.save(false);
                }
                if ui.button("Save as…").clicked() {
                    self.save(true);
                }
                ui.separator();
                if ui
                    .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                    .on_hover_text("Undo · Ctrl+Z")
                    .clicked()
                {
                    self.history(false);
                }
                if ui
                    .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo"))
                    .on_hover_text("Redo · Ctrl+Y")
                    .clicked()
                {
                    self.history(true);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("?").clicked() {
                        self.help = true;
                    }
                    ui.label(
                        RichText::new(if self.dirty() { "+ Unsaved" } else { "+ Saved" })
                            .color(if self.dirty() { ORANGE } else { ACCENT }),
                    );
                });
            });
            ui.add_space(4.);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new(&self.project.name).color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.selectable_value(&mut self.editing, true, "Edit");
                    ui.selectable_value(&mut self.editing, false, "View");
                });
            });
            ui.add_space(5.);
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&self.status).size(11.).color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} devices · {} ports · {} cables",
                            self.project.nodes.len(),
                            self.project
                                .nodes
                                .iter()
                                .map(|n| n.ports.len())
                                .sum::<usize>(),
                            self.project.connections.len()
                        ))
                        .size(11.)
                        .color(MUTED),
                    );
                });
            });
        });
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("nodes")
            .default_width(205.)
            .width_range(180.0..=300.0)
            .show(ctx, |ui| {
                ui.add_space(12.);
                section(ui, "DEVICES");
                ui.label(RichText::new("Map the physical device").size(11.).color(MUTED));
                ui.add_space(10.);
                let nodes: Vec<_> = self.project.nodes.iter().map(|node| {
                    (node.id, node.name.clone(), node.ports.len())
                }).collect();
                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - 225.).max(100.))
                    .show(ui, |ui| {
                        for (id, name, count) in nodes {
                            let response = ui.add_sized(
                                [ui.available_width(), 54.],
                                egui::Button::new(format!("{name}\n{count} ports"))
                                    .selected(self.selected == Some(id) || self.hovered_node == Some(id))
                                    .sense(Sense::click_and_drag()),
                            );
                            response.dnd_set_drag_payload(scene::NodeDrag { node: id });
                            if response.clicked() {
                                self.select(id, None);
                            }
                            response.context_menu(|ui| {
                                if ui.button("Rename device…").clicked() {
                                    self.start_rename(id);
                                    ui.close();
                                }
                                if ui.button(RichText::new("Delete device").color(ORANGE)).clicked() {
                                    self.delete_device(id);
                                    ui.close();
                                }
                            });
                        }
                    });
                ui.add_space(14.);
                ui.separator();
                ui.menu_button("+ New device", |ui| {
                    for (label, name, size) in [
                        ("Computer / PC", "New PC", [210., 420., 380.]),
                        ("USB hub", "New USB hub", [105., 25., 45.]),
                        ("Audio mixer", "New audio mixer", [280., 70., 220.]),
                        ("Custom device", "New device", [150., 80., 100.]),
                    ] {
                        if ui.button(label).clicked() {
                            self.add_node(name, size);
                            ui.close();
                        }
                    }
                });
                ui.add_space(12.);
                ui.label(RichText::new(
                    "Create a device when its ports need mapping. Double-click a port to name equipment. Drag a device onto a port to connect it."
                ).size(11.).color(MUTED));
                if self.recovery.is_some() {
                    ui.add_space(18.);
                    if ui.button("Recover previous draft").clicked() {
                        self.begin(Pending::Recovery);
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.small(RichText::new(concat!("NATIVE RUST  /  OFFLINE\nv", env!("CARGO_PKG_VERSION"))).color(MUTED));
                });
            });
    }

    fn palette_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("port-palette")
            .default_width(300.)
            .width_range(270.0..=400.0)
            .show(ctx, |ui| {
                ui.add_space(12.);
                if palette::show(ui, self.selected.is_some()) {
                    self.editing = true;
                }
            });
    }

    fn start_rename(&mut self, node: Id) {
        if !self.flush_draft() {
            return;
        }
        if let Some(node) = self.project.node(node) {
            self.rename_device = Some(RenameDevice {
                node: node.id,
                text: node.name.clone(),
                focus: true,
            });
        }
    }

    fn delete_device(&mut self, node: Id) {
        if self.change(|project| {
            project.remove_node(node);
            Ok(())
        }) {
            if self.selected == Some(node) {
                self.selected = self.project.roots.first().copied();
                self.selected_port = None;
            }
            self.hovered_node = None;
            self.status = "Device and its connections deleted. Ctrl+Z restores them.".into();
        }
    }

    fn open_port(&mut self, port: PortRef) {
        if !self.flush_draft() {
            return;
        }
        if let Some(peer) = self.project.connected_peer(port) {
            self.port_label = None;
            self.select(peer.node, Some(peer.port));
        } else {
            self.selected_port = Some(port.port);
            self.port_label = PortLabel::new(&self.project, port);
        }
    }

    fn open_connect(&mut self, source: PortRef, target_node: Option<Id>) {
        if self.flush_draft() && self.project.port(source).is_some() {
            self.connect_ports = Some(ConnectPorts {
                source,
                target_node: target_node.filter(|id| *id != source.node),
                target_port: None,
                supplementary: false,
                adapter: false,
            });
        }
    }

    fn connect_selected(&mut self) -> bool {
        let Some(dialog) = &self.connect_ports else {
            return false;
        };
        let (Some(node), Some(port)) = (dialog.target_node, dialog.target_port) else {
            return false;
        };
        let (source, supplementary, adapter) =
            (dialog.source, dialog.supplementary, dialog.adapter);
        if self.change(|project| {
            project
                .connect(
                    source,
                    Target::Port(PortRef { node, port }),
                    supplementary,
                    adapter,
                )
                .map(|_| ())
        }) {
            self.connect_ports = None;
            self.status =
                "Devices connected. Double-click either port to open the other device.".into();
            true
        } else {
            false
        }
    }

    fn apply_scene_event(&mut self, node: Id, event: scene::SceneEvent) {
        self.hovered_node = event.hovered_node;
        if let Some(port) = event.selected {
            self.selected_port = Some(port);
        }
        if event.drag_started && self.flush_draft() {
            self.remember();
        }
        // A gesture has one undo snapshot; its frames update only valid candidates.
        if event.moved.is_some() || event.label_moved.is_some() || event.resized.is_some() {
            let mut candidate = self.with_drafts();
            if let Some((port, uv)) = event.moved
                && let Some(port) = candidate.port_mut(PortRef { node, port })
            {
                port.uv = uv;
            }
            if let Some((port, offset)) = event.label_moved
                && let Some(port) = candidate.port_mut(PortRef { node, port })
            {
                port.label_offset = Some(offset);
            }
            let resized = event.resized.map_or(Ok(()), |(part, size, offset)| {
                candidate.resize_part(node, part, size, offset)
            });
            if resized.and_then(|_| candidate.validate()).is_ok() {
                self.project = candidate;
                self.rename_device = None;
            }
        }
        if let Some(drop) = event.dropped {
            let mut id = 0;
            if self.change(|project| {
                id = project.place_port(
                    node,
                    drop.kind,
                    drop.hit.part,
                    drop.hit.face,
                    drop.hit.uv,
                )?;
                Ok(())
            }) {
                self.selected_port = Some(id);
                self.status = "Port placed. Double-click it to name connected equipment.".into();
            }
        }
        if event.drop_rejected {
            self.status = "Drop the port on a visible surface with room for the connector.".into();
        }
        if let Some(port) = event.delete
            && self.change(|project| {
                project.remove_port(PortRef { node, port });
                Ok(())
            })
        {
            if self.selected_port == Some(port) {
                self.selected_port = None;
            }
            self.status = "Port and connection deleted. Ctrl+Z restores them.".into();
        }
        if let Some(port) = event.edit_label {
            self.open_port(PortRef { node, port });
        }
        if let Some(port) = event.rotate
            && self.flush_draft()
            && let Some(source) = self.project.port(PortRef { node, port })
        {
            self.rotate_port = Some(RotatePort {
                port: PortRef { node, port },
                degrees: source.angle,
            });
        }
        if let Some(port) = event.connect {
            self.open_connect(PortRef { node, port }, None);
        }
        if let Some((port, target_node)) = event.node_dropped {
            self.open_connect(PortRef { node, port }, Some(target_node));
        }
        if let Some(port) = event.disconnect {
            self.change(|project| {
                project.disconnect(PortRef { node, port });
                Ok(())
            });
        }
    }

    fn level(&mut self, ui: &mut egui::Ui) {
        let Some(node) = self.selected.and_then(|id| self.project.node(id)).cloned() else {
            self.welcome(ui);
            return;
        };
        ui.add_space(8.);
        ui.horizontal(|ui| {
            for id in self.project.ancestors(node.id) {
                if let Some(device) = self.project.node(id) {
                    let name = device.name.clone();
                    if ui.selectable_label(id == node.id, name).clicked() {
                        self.select(id, None);
                    }
                    if id != node.id {
                        ui.label(RichText::new("/").color(MUTED));
                    }
                }
            }
        });
        let title =
            ui.add(egui::Label::new(RichText::new(&node.name).heading()).sense(Sense::click()));
        title.context_menu(|ui| {
            if ui.button("Rename device…").clicked() {
                self.start_rename(node.id);
                ui.close();
            }
            if ui.button("Add box").clicked() {
                self.change(|project| project.add_part(node.id, Primitive::Box).map(|_| ()));
                ui.close();
            }
            if ui.button("Add wedge").clicked() {
                self.change(|project| project.add_part(node.id, Primitive::Wedge).map(|_| ()));
                ui.close();
            }
            if ui
                .button(RichText::new("Delete device").color(ORANGE))
                .clicked()
            {
                self.delete_device(node.id);
                ui.close();
            }
        });
        ui.add_space(4.);
        self.view.select(&node);
        ui.horizontal_wrapped(|ui| {
            for face in Face::ALL {
                if ui
                    .selectable_label(self.view.face == face, face.name())
                    .clicked()
                {
                    self.view.set_face(face);
                }
            }
            if ui.button("Reset view").clicked() {
                self.view.set_face(node.default_view);
                self.view.reset();
            }
            ui.checkbox(&mut self.view.labels, "Labels");
        });
        ui.label(RichText::new(
            "Drag ports from the palette · double-click to name equipment or open its device · right-click for actions"
        ).size(11.).color(MUTED));
        ui.add_space(5.);
        let previous_hover = self.hovered_node;
        let event = scene::draw(
            ui,
            &self.project,
            &node,
            self.selected_port,
            self.editing,
            &mut self.view,
        );
        self.apply_scene_event(node.id, event);
        if self.hovered_node != previous_hover {
            ui.ctx().request_repaint();
        }
    }

    fn welcome(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.17).max(35.));
            ui.label(RichText::new("CM").size(72.).color(ACCENT));
            ui.add_space(15.);
            ui.label(RichText::new("Where does this cable go?").size(31.).strong());
            ui.add_space(10.);
            ui.label(RichText::new("Build a physical map of your desk.\nThe right device. The right port. Easy reconnection.").size(16.).color(MUTED));
            ui.add_space(24.);
            if ui.add_sized([230., 44.], egui::Button::new(RichText::new("+ Create your first PC").color(ACCENT))).clicked() {
                self.add_node("My PC", [210., 420., 380.]);
            }
            if ui.add_sized([230., 40.], egui::Button::new("Open an example")).clicked() {
                self.begin(Pending::Demo);
            }
            ui.add_space(16.);
            ui.label(RichText::new("The example maps PC > USB hub > webcam.\nEquipment names are labels; mapped devices have ports.").size(12.).color(MUTED));
        });
    }

    fn action_dialogs(&mut self, ctx: &egui::Context) {
        // Read Enter before numeric/text widgets consume it while committing their value.
        let submit = ctx.input(|input| input.key_pressed(Key::Enter));
        if let Some(mut draft) = self.rename_device.take() {
            let mut apply = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("rename-device")).show(ctx, |ui| {
                ui.heading("Rename device");
                let response =
                    ui.add_sized([370., 30.], egui::TextEdit::singleline(&mut draft.text));
                if draft.focus {
                    response.request_focus();
                    draft.focus = false;
                }
                ui.horizontal(|ui| {
                    apply = ui.button("Save name").clicked() || submit;
                    cancel =
                        ui.button("Cancel").clicked() || ctx.input(|i| i.key_pressed(Key::Escape));
                });
            });
            if apply {
                let node = draft.node;
                let name = draft.text.trim().to_owned();
                if !self.change(|project| {
                    project
                        .node_mut(node)
                        .ok_or("The device no longer exists.")?
                        .name = name;
                    Ok(())
                }) {
                    self.rename_device = Some(draft);
                }
            } else if !cancel {
                self.rename_device = Some(draft);
            }
        }
        if let Some(mut editor) = self.rotate_port.take() {
            let mut apply = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("rotate-port")).show(ctx, |ui| {
                ui.heading("Rotate port");
                ui.label("Angle in degrees");
                ui.add(
                    egui::DragValue::new(&mut editor.degrees)
                        .speed(1.)
                        .suffix("°"),
                );
                ui.horizontal(|ui| {
                    apply = ui.button("Apply").clicked() || submit;
                    cancel =
                        ui.button("Cancel").clicked() || ctx.input(|i| i.key_pressed(Key::Escape));
                });
            });
            if apply {
                if !self.change(|project| project.rotate_port(editor.port, editor.degrees)) {
                    self.rotate_port = Some(editor);
                }
            } else if !cancel {
                self.rotate_port = Some(editor);
            }
        }
        if let Some(mut editor) = self.connect_ports.take() {
            let mut apply = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("connect-ports")).show(ctx, |ui| {
                ui.set_min_width(390.);
                ui.heading("Connect to a device");
                if self.project.is_occupied(editor.source) {
                    ui.label("This source port is occupied. Disconnect it first.");
                    cancel = ui.button("Close").clicked();
                } else {
                    if let Some(port) = self.project.port(editor.source) {
                        ui.label(format!("Source: {} · {}", port.name, port.kind.name()));
                    }
                    let nodes: Vec<_> = self
                        .project
                        .nodes
                        .iter()
                        .filter(|node| node.id != editor.source.node)
                        .map(|node| (node.id, node.name.clone()))
                        .collect();
                    if !editor
                        .target_node
                        .is_some_and(|id| nodes.iter().any(|node| node.0 == id))
                    {
                        editor.target_node = nodes.first().map(|node| node.0);
                        editor.target_port = None;
                    }
                    let before = editor.target_node;
                    egui::ComboBox::from_id_salt("connect-target-device")
                        .selected_text(
                            nodes
                                .iter()
                                .find(|node| Some(node.0) == editor.target_node)
                                .map(|node| node.1.as_str())
                                .unwrap_or("Create another device first"),
                        )
                        .show_ui(ui, |ui| {
                            for (id, name) in &nodes {
                                ui.selectable_value(&mut editor.target_node, Some(*id), name);
                            }
                        });
                    if editor.target_node != before {
                        editor.target_port = None;
                    }
                    let ports: Vec<_> = editor
                        .target_node
                        .and_then(|id| self.project.node(id))
                        .map(|node| {
                            node.ports
                                .iter()
                                .filter(|port| {
                                    !self.project.is_occupied(PortRef {
                                        node: node.id,
                                        port: port.id,
                                    })
                                })
                                .map(|port| (port.id, port.name.clone(), port.kind))
                                .collect()
                        })
                        .unwrap_or_default();
                    if !editor
                        .target_port
                        .is_some_and(|id| ports.iter().any(|port| port.0 == id))
                    {
                        editor.target_port = ports.first().map(|port| port.0);
                    }
                    egui::ComboBox::from_id_salt("connect-target-port")
                        .selected_text(
                            ports
                                .iter()
                                .find(|port| Some(port.0) == editor.target_port)
                                .map(|port| format!("{} · {}", port.1, port.2.name()))
                                .unwrap_or_else(|| "No free target port".into()),
                        )
                        .show_ui(ui, |ui| {
                            for (id, name, kind) in &ports {
                                ui.selectable_value(
                                    &mut editor.target_port,
                                    Some(*id),
                                    format!("{name} · {}", kind.name()),
                                );
                            }
                        });
                    ui.checkbox(&mut editor.adapter, "Adapter / converting cable");
                    ui.checkbox(
                        &mut editor.supplementary,
                        "Supplementary connection outside the hierarchy",
                    );
                    let compatible =
                        self.project
                            .port(editor.source)
                            .zip(
                                editor.target_node.zip(editor.target_port).and_then(
                                    |(node, port)| self.project.port(PortRef { node, port }),
                                ),
                            )
                            .is_some_and(|(source, target)| {
                                source.kind.compatible(target.kind) || editor.adapter
                            });
                    if editor.target_port.is_some() && !compatible {
                        ui.label(
                            RichText::new(
                                "Different connector families: select Adapter to connect them.",
                            )
                            .color(ORANGE),
                        );
                    }
                    ui.horizontal(|ui| {
                        apply = ui
                            .add_enabled(compatible, egui::Button::new("Connect devices"))
                            .clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                }
                cancel |= ctx.input(|i| i.key_pressed(Key::Escape));
            });
            if !cancel {
                self.connect_ports = Some(editor);
                if apply {
                    self.connect_selected();
                }
            }
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.error.is_none() && self.pending.is_none() && self.file_dialog.is_none() {
            let action = self
                .port_label
                .as_mut()
                .and_then(|editor| editor.show(ctx, &self.project));
            match action {
                Some(port_label::Action::Save) => {
                    let editor = self.port_label.as_ref().unwrap();
                    let port = editor.port;
                    let text = editor.text.clone();
                    if self.change(|p| p.label_port(port, &text)) {
                        self.port_label = None;
                        self.status = "Equipment name saved beside the port.".into();
                    }
                }
                Some(port_label::Action::Cancel) => self.port_label = None,
                None => {}
            }
        }
        if self.error.is_none() && self.pending.is_none() && self.file_dialog.is_none() {
            self.action_dialogs(ctx);
        }
        if let Some(error) = self.error.clone() {
            egui::Modal::new(egui::Id::new("error")).show(ctx, |ui| {
                ui.heading("Could not apply the change");
                ui.label(error);
                if ui.button("Close").clicked() {
                    self.error = None;
                }
            });
        }
        if let Some(action) = self.pending.clone() {
            egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
                ui.heading("Save changes?");
                ui.label("This project has unsaved changes.");
                ui.horizontal(|ui| {
                    if ui.button("Save and continue").clicked() {
                        self.pending = None;
                        self.after_save = Some(action.clone());
                        self.save(false);
                    }
                    if ui.button("Continue without saving").clicked() {
                        self.pending = None;
                        self.perform(action.clone());
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending = None;
                    }
                });
            });
        }
        if self.file_dialog.is_some() {
            egui::Modal::new(egui::Id::new("path-dialog")).show(ctx, |ui| {
                let save = matches!(self.file_dialog, Some(FileDialog::Save));
                ui.heading(if save { "Save project" } else { "Open project" });
                ui.label("Project file path (.cable.json)");
                ui.add_sized(
                    [520., 30.],
                    egui::TextEdit::singleline(&mut self.file_input),
                );
                ui.horizontal(|ui| {
                    if ui.button(if save { "Save" } else { "Open" }).clicked() {
                        let path = PathBuf::from(self.file_input.trim());
                        self.file_dialog = None;
                        if save {
                            self.save_path(path);
                        } else {
                            self.load_path(path);
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.file_dialog = None;
                        self.after_save = None;
                    }
                });
            });
        }
        if self.help {
            egui::Window::new("Using Cable Management")
                .open(&mut self.help)
                .default_width(550.)
                .show(ctx, |ui| {
                    ui.label("1. Create a computer, hub or mixer with New device.");
                    ui.label("2. Choose a side, then drag a port from the palette onto the device.");
                    ui.label("3. Choose Edit to move ports or resize the enclosure by dragging a corner. Drag labels to keep names readable.");
                    ui.label("4. Double-click a port to name equipment. A mapped port opens its connected device immediately.");
                    ui.label("5. Right-click a port to rotate, connect, disconnect or delete it. Connect to an existing device and choose a free port; drag a device from the list onto a port to preselect it.");
                    ui.label("6. Right-click a device name to rename or delete it. Right-click the canvas title to add a box or wedge.");
                    ui.separator();
                    ui.label("Change sides with the face buttons. Scroll to zoom; middle-drag to pan. Hover a mapped port or label to highlight its device in the list.");
                    ui.label("Ctrl+S save · Ctrl+Shift+S save as · Ctrl+O open\nCtrl+N new · Ctrl+Z undo · Ctrl+Y redo");
                    ui.separator();
                    ui.label("Connectors keep their physical size when you resize the enclosure. Choose Adapter when connector families differ. Supplementary connections do not change the device hierarchy.");
                    ui.label("Projects are local versioned JSON files. The previous save is kept as .bak. Unsaved changes receive a recovery draft every 15 seconds.");
                });
        }
    }
}

impl eframe::App for CableApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.closing {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.pending.is_none() {
                self.begin(Pending::Quit);
            }
        }
        if self.closing {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let modal = self.pending.is_some()
            || self.file_dialog.is_some()
            || self.error.is_some()
            || self.port_label.is_some()
            || self.rename_device.is_some()
            || self.rotate_port.is_some()
            || self.connect_ports.is_some();
        if !modal {
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::S)) {
                self.save(true);
            } else if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::S)) {
                self.save(false);
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::O)) {
                self.begin(Pending::Open);
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::N)) {
                self.begin(Pending::New);
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Z)) {
                self.history(false);
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Y)) {
                self.history(true);
            }
        }
        self.toolbar(ctx);
        self.sidebar(ctx);
        self.palette_panel(ctx);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(20, 27, 34))
                    .inner_margin(14.),
            )
            .show(ctx, |ui| self.level(ui));
        self.dialogs(ctx);
        palette::drag_indicator(ctx);
        if self.dirty() && self.last_recovery.elapsed() > Duration::from_secs(15) {
            if let Err(e) = storage::save_recovery(&self.with_drafts()) {
                self.status = e;
            }
            self.last_recovery = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_secs(5));
    }
}

fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(5.);
    ui.label(RichText::new(text).size(10.).strong().color(MUTED));
}
#[cfg(test)]
mod tests {
    use super::*;
    use cable_management::ports::PortKind;
    use eframe::egui::{Pos2, Rect};
    #[test]
    fn close_confirmation_keeps_unfinished_equipment_text_when_escape_is_pressed() {
        let ctx = egui::Context::default();
        let mut app = CableApp::from_context(&ctx, None);
        app.reset_document(Project::demo(), None, false);
        let node = app.project.nodes[0].id;
        let port = app.project.nodes[0].ports[1].id;
        app.apply_scene_event(
            node,
            scene::SceneEvent {
                edit_label: Some(port),
                ..Default::default()
            },
        );
        app.port_label.as_mut().unwrap().text = "gaming mouse".into();
        app.begin(Pending::Quit);
        assert!(app.pending.is_some());
        interactive_frame(
            &ctx,
            &mut app,
            0.,
            vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert_eq!(
            app.port_label.as_ref().map(|e| e.text.as_str()),
            Some("gaming mouse")
        );
    }
    fn interactive_frame(
        ctx: &egui::Context,
        app: &mut CableApp,
        time: f64,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 850.))),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                app.toolbar(ctx);
                app.sidebar(ctx);
                app.palette_panel(ctx);
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.level(ui);
                });
                app.dialogs(ctx);
            },
        )
    }
    #[test]
    fn dragging_a_palette_icon_to_the_device_and_typing_a_name_needs_no_extra_node() {
        let ctx = egui::Context::default();
        let mut app = CableApp::from_context(&ctx, None);
        let mut p = Project::default();
        p.add_node("My PC", [200., 300., 200.]);
        app.reset_document(p, None, false);
        let output = interactive_frame(&ctx, &mut app, 0., vec![]);
        let start = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == "USB-A" => {
                    Some(t.pos + t.galley.size() / 2.)
                }
                _ => None,
            })
            .unwrap();
        let target = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Path(p) if p.fill == Color32::from_rgb(74, 82, 91) => {
                    Some(Rect::from_points(&p.points).center())
                }
                _ => None,
            })
            .unwrap();
        interactive_frame(&ctx, &mut app, 0.1, vec![egui::Event::PointerMoved(start)]);
        interactive_frame(
            &ctx,
            &mut app,
            0.2,
            vec![egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            }],
        );
        interactive_frame(&ctx, &mut app, 0.3, vec![egui::Event::PointerMoved(target)]);
        interactive_frame(&ctx, &mut app, 0.4, vec![egui::Event::PointerMoved(target)]);
        interactive_frame(
            &ctx,
            &mut app,
            0.5,
            vec![egui::Event::PointerButton {
                pos: target,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert_eq!(app.project.nodes[0].ports.len(), 1);
        let port = app.project.nodes[0].ports[0].id;
        let node = app.project.nodes[0].id;
        assert_eq!(app.project.nodes[0].ports[0].kind, PortKind::UsbA);
        app.apply_scene_event(
            node,
            scene::SceneEvent {
                edit_label: Some(port),
                ..Default::default()
            },
        );
        interactive_frame(&ctx, &mut app, 0.6, vec![]);
        interactive_frame(
            &ctx,
            &mut app,
            0.7,
            vec![egui::Event::Text("gaming mouse".into())],
        );
        interactive_frame(
            &ctx,
            &mut app,
            0.8,
            vec![egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(app.port_label.is_none());
        assert_eq!(app.project.nodes.len(), 1);
        assert_eq!(app.project.endpoints[0].name, "gaming mouse");
        app.project.validate().unwrap();
    }
    #[test]
    fn delete_from_context_menu_cleans_label_and_undo_restores_both() {
        let mut a = app();
        let before = a.project.clone();
        let node = a.project.nodes[1].id;
        let port = a.project.nodes[1].ports[3].id;
        a.apply_scene_event(
            node,
            scene::SceneEvent {
                delete: Some(port),
                ..Default::default()
            },
        );
        assert!(a.project.port(PortRef { node, port }).is_none());
        assert!(a.project.endpoints.is_empty());
        a.project.validate().unwrap();
        a.history(false);
        assert_eq!(a.project, before);
    }
    fn app() -> CableApp {
        let mut a = CableApp::from_context(&egui::Context::default(), None);
        a.reset_document(Project::demo(), None, false);
        a
    }
    #[test]
    fn palette_only_layout_has_no_properties_tree_or_lower_port_list() {
        let ctx = egui::Context::default();
        let mut a = CableApp::from_context(&ctx, None);
        let mut project = Project::default();
        let node = project.add_node("Desk PC", [200., 300., 200.]);
        project
            .add_port(node, PortKind::UsbA, "Unique port list sentinel")
            .unwrap();
        a.reset_document(project, None, false);
        let output = interactive_frame(&ctx, &mut a, 0., vec![]);
        let strings = painted_text(&output);
        assert!(strings.iter().any(|text| text == "PORT LIBRARY · 20 TYPES"));
        assert!(strings.iter().any(|text| text == "not connected"));
        for removed in [
            "PROPERTIES",
            "PORTS",
            "Rotation",
            "Dimensions",
            "Whole tree",
            "Unique port list sentinel",
        ] {
            assert!(
                !strings.iter().any(|text| text.contains(removed)),
                "obsolete UI: {removed}"
            );
        }
        let canvas = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect) if rect.fill == Color32::from_rgb(18, 24, 31) => {
                    Some(rect.rect)
                }
                _ => None,
            })
            .max_by(|a, b| a.area().total_cmp(&b.area()));
        let canvas = canvas.expect("canvas background");
        assert!(
            canvas.bottom() > 800.,
            "canvas should use the remaining height"
        );
    }

    #[test]
    fn double_click_event_opens_mapped_peer_immediately_in_both_directions() {
        let mut a = app();
        let connection = a
            .project
            .connections
            .iter()
            .find(|connection| matches!(connection.to, Target::Port(_)))
            .unwrap()
            .clone();
        let Target::Port(peer) = connection.to else {
            unreachable!()
        };
        a.apply_scene_event(
            connection.from.node,
            scene::SceneEvent {
                edit_label: Some(connection.from.port),
                ..Default::default()
            },
        );
        assert_eq!(a.selected, Some(peer.node));
        assert_eq!(a.selected_port, Some(peer.port));
        assert_eq!(a.view.face, a.project.port(peer).unwrap().face);
        assert!(a.port_label.is_none());
        a.apply_scene_event(
            peer.node,
            scene::SceneEvent {
                edit_label: Some(peer.port),
                ..Default::default()
            },
        );
        assert_eq!(a.selected, Some(connection.from.node));
        assert_eq!(a.selected_port, Some(connection.from.port));
        assert!(a.port_label.is_none());
    }

    #[test]
    fn scene_hover_updates_and_clears_sidebar_link_state() {
        let mut a = app();
        let node = a.project.nodes[0].id;
        let peer = a.project.nodes[1].id;
        a.apply_scene_event(
            node,
            scene::SceneEvent {
                hovered_node: Some(peer),
                ..Default::default()
            },
        );
        assert_eq!(a.hovered_node, Some(peer));
        a.apply_scene_event(node, scene::SceneEvent::default());
        assert_eq!(a.hovered_node, None);
    }

    #[test]
    fn rotate_modal_applies_absolute_angle_and_is_undoable() {
        let ctx = egui::Context::default();
        let mut a = app();
        let node = a.project.nodes[0].id;
        let port = a.project.nodes[0].ports[0].id;
        let reference = PortRef { node, port };
        let before = a.project.clone();
        a.apply_scene_event(
            node,
            scene::SceneEvent {
                rotate: Some(port),
                ..Default::default()
            },
        );
        assert_eq!(
            a.rotate_port.as_ref().unwrap().degrees,
            a.project.port(reference).unwrap().angle
        );
        interactive_frame(&ctx, &mut a, 0., vec![]);
        let output = interactive_frame(&ctx, &mut a, 0.1, vec![]);
        let angle = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text.ends_with('°') => {
                    Some(text.pos + text.galley.size() / 2.)
                }
                _ => None,
            })
            .expect("numeric angle field");
        interactive_frame(&ctx, &mut a, 0.2, vec![egui::Event::PointerMoved(angle)]);
        interactive_frame(&ctx, &mut a, 0.3, vec![pointer_button(angle, true)]);
        interactive_frame(&ctx, &mut a, 0.4, vec![pointer_button(angle, false)]);
        interactive_frame(&ctx, &mut a, 0.5, vec![]);
        interactive_frame(
            &ctx,
            &mut a,
            0.6,
            vec![egui::Event::Text("405".into()), key(Key::Enter)],
        );
        assert!(a.rotate_port.is_none());
        assert_eq!(a.project.port(reference).unwrap().angle, 45.);
        a.history(false);
        assert_eq!(a.project, before);
    }

    fn free_devices() -> (CableApp, PortRef, PortRef) {
        let mut project = Project::default();
        let source_node = project.add_node("PC", [200., 300., 200.]);
        let source_port = project
            .add_port(source_node, PortKind::UsbA, "USB-A 1")
            .unwrap();
        let target_node = project.add_node("Hub", [100., 40., 60.]);
        let occupied = project
            .add_port(target_node, PortKind::UsbA, "Occupied")
            .unwrap();
        let target_port = project
            .add_port(target_node, PortKind::Hdmi, "Free HDMI")
            .unwrap();
        project
            .connect_endpoint(
                PortRef {
                    node: target_node,
                    port: occupied,
                },
                "Mouse",
            )
            .unwrap();
        let mut a = CableApp::from_context(&egui::Context::default(), None);
        a.reset_document(project, None, false);
        (
            a,
            PortRef {
                node: source_node,
                port: source_port,
            },
            PortRef {
                node: target_node,
                port: target_port,
            },
        )
    }

    #[test]
    fn device_drop_connect_modal_uses_existing_device_and_only_free_ports() {
        let ctx = egui::Context::default();
        let (mut a, source, target) = free_devices();
        a.apply_scene_event(
            source.node,
            scene::SceneEvent {
                node_dropped: Some((source.port, target.node)),
                ..Default::default()
            },
        );
        interactive_frame(&ctx, &mut a, 0., vec![]);
        let output = interactive_frame(&ctx, &mut a, 0.1, vec![]);
        let editor = a.connect_ports.as_mut().unwrap();
        assert_eq!(editor.target_node, Some(target.node));
        assert_eq!(editor.target_port, Some(target.port));
        assert!(
            painted_text(&output)
                .iter()
                .any(|text| text.contains("Different connector families"))
        );
        editor.adapter = true;
        assert!(a.connect_selected());
        assert!(a.connect_ports.is_none());
        assert_eq!(a.project.nodes.len(), 2);
        assert_eq!(a.project.connected_peer(source), Some(target));
        assert!(a.project.connection_at(source).unwrap().adapter);
        a.project.validate().unwrap();
        a.history(false);
        assert_eq!(a.project.connected_peer(source), None);
    }

    #[test]
    fn occupied_source_connect_modal_requires_disconnect_without_mutation() {
        let ctx = egui::Context::default();
        let (mut a, source, target) = free_devices();
        a.project
            .connect(source, Target::Port(target), false, true)
            .unwrap();
        let before = a.project.clone();
        a.apply_scene_event(
            source.node,
            scene::SceneEvent {
                connect: Some(source.port),
                ..Default::default()
            },
        );
        interactive_frame(&ctx, &mut a, 0., vec![]);
        let output = interactive_frame(&ctx, &mut a, 0.1, vec![]);
        assert!(
            painted_text(&output)
                .iter()
                .any(|text| text.contains("Disconnect it first"))
        );
        assert_eq!(a.project, before);
        assert!(a.undo.is_empty());
    }

    #[test]
    fn label_and_resize_gestures_each_have_one_undo_and_persist_on_save() {
        let (mut a, reference, _) = free_devices();
        let original = a.project.clone();
        let part = a.project.node(reference.node).unwrap().parts[0].id;
        a.apply_scene_event(
            reference.node,
            scene::SceneEvent {
                drag_started: true,
                label_moved: Some((reference.port, [12., 18.])),
                ..Default::default()
            },
        );
        a.apply_scene_event(
            reference.node,
            scene::SceneEvent {
                label_moved: Some((reference.port, [24., 36.])),
                ..Default::default()
            },
        );
        assert_eq!(a.undo.len(), 1);
        assert_eq!(
            a.project.port(reference).unwrap().uv,
            original.port(reference).unwrap().uv
        );
        let labeled = a.project.clone();
        a.apply_scene_event(
            reference.node,
            scene::SceneEvent {
                drag_started: true,
                resized: Some((part, [250., 340., 200.], [25., 20., 0.])),
                ..Default::default()
            },
        );
        a.apply_scene_event(
            reference.node,
            scene::SceneEvent {
                resized: Some((part, [270., 360., 200.], [35., 30., 0.])),
                ..Default::default()
            },
        );
        assert_eq!(a.undo.len(), 2);
        let file =
            std::env::temp_dir().join(format!("cable-ui-gestures-{}.json", std::process::id()));
        a.path = Some(file.clone());
        a.save(false);
        assert!(!a.dirty());
        let saved = storage::load(&file).unwrap();
        assert_eq!(
            saved.port(reference).unwrap().label_offset,
            Some([24., 36.])
        );
        assert_eq!(
            saved.node(reference.node).unwrap().parts[0].size,
            [270., 360., 200.]
        );
        a.history(false);
        assert_eq!(a.project, labeled);
        a.history(false);
        assert_eq!(a.project, original);
        a.history(true);
        a.history(true);
        assert_eq!(a.project, saved);
        cleanup(&file);
    }

    #[test]
    fn rename_text_is_dirty_saved_and_kept_when_invalid() {
        let mut a = app();
        let node = a.project.nodes[0].id;
        a.start_rename(node);
        a.rename_device.as_mut().unwrap().text = "Renamed PC".into();
        assert!(a.dirty());
        let file =
            std::env::temp_dir().join(format!("cable-ui-rename-{}.json", std::process::id()));
        a.path = Some(file.clone());
        a.save(false);
        assert_eq!(
            storage::load(&file).unwrap().node(node).unwrap().name,
            "Renamed PC"
        );
        assert!(!a.dirty());
        a.start_rename(node);
        a.rename_device.as_mut().unwrap().text.clear();
        assert!(!a.flush_draft());
        assert!(a.error.is_some());
        assert!(a.rename_device.is_some());
        assert_eq!(a.project.node(node).unwrap().name, "Renamed PC");
        cleanup(&file);
    }

    #[test]
    fn invalid_load_preserves_current_document() {
        let mut a = app();
        let before = a.project.clone();
        a.load_path(PathBuf::from("/nonexistent-cable-project.json"));
        assert_eq!(a.project, before);
        assert!(a.error.is_some());
    }

    #[test]
    fn undo_ends_an_active_resize_even_while_pointer_is_held() {
        let ctx = egui::Context::default();
        let mut a = CableApp::from_context(&ctx, None);
        let mut p = Project::default();
        p.add_node("PC", [200., 300., 200.]);
        a.reset_document(p, None, false);
        a.editing = true;
        let original = a.project.clone();
        let out = interactive_frame(&ctx, &mut a, 0., vec![]);
        let bounds = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Path(path) if path.fill == Color32::from_rgb(74, 82, 91) => {
                    Some(Rect::from_points(&path.points))
                }
                _ => None,
            })
            .unwrap();
        let start = bounds.right_center();
        let end = start + Vec2::new(40., 0.);
        interactive_frame(&ctx, &mut a, 0.1, vec![egui::Event::PointerMoved(start)]);
        interactive_frame(&ctx, &mut a, 0.2, vec![pointer_button(start, true)]);
        interactive_frame(&ctx, &mut a, 0.3, vec![egui::Event::PointerMoved(end)]);
        assert_ne!(a.project, original);
        assert_eq!(a.undo.len(), 1);
        a.history(false);
        assert_eq!(a.project, original);
        interactive_frame(&ctx, &mut a, 0.4, vec![]);
        assert_eq!(a.project, original);
        assert!(a.undo.is_empty());
        assert_eq!(a.redo.len(), 1);
    }
    fn painted_text(output: &egui::FullOutput) -> Vec<String> {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect()
    }
    fn pointer_button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }
    fn key(key: Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }
    fn cleanup(file: &std::path::Path) {
        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_file(storage::backup_path(file));
    }
}
