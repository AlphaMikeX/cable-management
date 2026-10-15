use crate::scene::{self, ACCENT, MUTED, ORANGE, ViewState};
use crate::{
    palette,
    port_label::{self, PortLabel},
};
use cable_management::{model::*, ports::PortKind, storage, tree};
use eframe::egui::{
    self, Align2, Color32, FontId, Key, Modifiers, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Level,
    Tree,
}
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

pub struct CableApp {
    project: Project,
    saved: Project,
    path: Option<PathBuf>,
    selected: Option<Id>,
    selected_port: Option<Id>,
    selected_part: Option<Id>,
    draft: Option<Node>,
    cable_drafts: HashMap<Id, String>,
    endpoint_drafts: HashMap<Id, String>,
    mode: Mode,
    editing: bool,
    view: ViewState,
    tree_pan: Vec2,
    tree_zoom: f32,
    tree_fit: bool,
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
    endpoint_name: String,
    target_node: Option<Id>,
    target_port: Option<Id>,
    supplementary: bool,
    adapter: bool,
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
            selected_part: None,
            draft: None,
            cable_drafts: HashMap::new(),
            endpoint_drafts: HashMap::new(),
            mode: Mode::Level,
            editing: false,
            view: ViewState::default(),
            tree_pan: Vec2::ZERO,
            tree_zoom: 1.,
            tree_fit: true,
            undo: vec![],
            redo: vec![],
            status: "Lokalt projekt · inga konton behövs".into(),
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
            endpoint_name: String::new(),
            target_node: None,
            target_port: None,
            supplementary: false,
            adapter: false,
        }
    }
    fn dirty(&self) -> bool {
        self.project != self.saved || self.has_drafts()
    }
    fn has_drafts(&self) -> bool {
        self.port_label
            .as_ref()
            .is_some_and(|e| e.has_changes(&self.project))
            || self
                .draft
                .as_ref()
                .is_some_and(|n| self.project.node(n.id) != Some(n))
            || self.cable_drafts.iter().any(|(id, value)| {
                self.project
                    .connections
                    .iter()
                    .find(|c| c.id == *id)
                    .is_some_and(|c| &c.label != value)
            })
            || self
                .endpoint_drafts
                .iter()
                .any(|(id, value)| self.project.endpoint(*id).is_some_and(|e| &e.name != value))
    }
    fn with_drafts(&self) -> Project {
        let mut p = self.project.clone();
        if let Some(draft) = &self.draft
            && let Some(node) = p.node_mut(draft.id)
        {
            *node = draft.clone();
        }
        for c in &mut p.connections {
            if let Some(value) = self.cable_drafts.get(&c.id) {
                c.label = value.clone();
            }
        }
        for e in &mut p.endpoints {
            if let Some(value) = self.endpoint_drafts.get(&e.id) {
                e.name = value.clone();
            }
        }
        if let Some(editor) = &self.port_label
            && editor.has_changes(&p)
        {
            let _ = p.label_port(editor.port, &editor.text);
        }
        p
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
                    self.draft = None;
                    self.cable_drafts.clear();
                    self.endpoint_drafts.clear();
                    self.tree_fit = false;
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
            self.selected_part = None;
            self.draft = None;
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
        self.cable_drafts.clear();
        self.endpoint_drafts.clear();
        self.selected = self.project.roots.first().copied();
        self.selected_port = None;
        self.selected_part = None;
        self.draft = None;
        self.undo.clear();
        self.redo.clear();
        self.view = ViewState::default();
        self.tree_fit = true;
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
                    self.status = "Återställt utkast. Spara det som ett projekt.".into();
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
                self.status = "Projekt öppnat.".into();
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
                .set_file_name("Mitt projekt.cable.json")
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
                .unwrap_or_else(|| "Mitt projekt.cable.json".into());
            self.file_dialog = Some(FileDialog::Save);
        }
    }
    fn save_path(&mut self, path: PathBuf) {
        match storage::save(&path, &self.project) {
            Ok(()) => {
                self.path = Some(path);
                self.saved = self.project.clone();
                self.clear_recovery();
                self.status = "Projekt sparat. Föregående version finns som .bak.".into();
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
            self.mode = Mode::Level;
            self.editing = true;
        }
    }
    fn history(&mut self, redo: bool) {
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
            self.draft = None;
            self.cable_drafts.clear();
            self.endpoint_drafts.clear();
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
                if ui.button("Nytt").clicked() {
                    self.begin(Pending::New);
                }
                if ui.button("Öppna").clicked() {
                    self.begin(Pending::Open);
                }
                if ui.button("Spara").clicked() {
                    self.save(false);
                }
                if ui.button("Spara som…").clicked() {
                    self.save(true);
                }
                ui.separator();
                if ui
                    .add_enabled(!self.undo.is_empty(), egui::Button::new("Ångra"))
                    .on_hover_text("Ångra · Ctrl+Z")
                    .clicked()
                {
                    self.history(false);
                }
                if ui
                    .add_enabled(!self.redo.is_empty(), egui::Button::new("Gör om"))
                    .on_hover_text("Gör om · Ctrl+Y")
                    .clicked()
                {
                    self.history(true);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("?").clicked() {
                        self.help = true;
                    }
                    ui.label(
                        RichText::new(if self.dirty() {
                            "+ Osparat"
                        } else {
                            "+ Sparat"
                        })
                        .color(if self.dirty() { ORANGE } else { ACCENT }),
                    );
                });
            });
            ui.add_space(4.);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new(&self.project.name).color(MUTED));
                ui.add_space(18.);
                ui.selectable_value(&mut self.mode, Mode::Level, "NIVÅ FÖR NIVÅ");
                if ui
                    .selectable_value(&mut self.mode, Mode::Tree, "HELA TRÄDET")
                    .clicked()
                {
                    self.tree_fit = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.selectable_value(&mut self.editing, true, " · Redigera");
                    ui.selectable_value(&mut self.editing, false, "Visa");
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
                            "{} enheter · {} portar · {} kablar",
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
                section(ui, "ANSLUTNINGSNODER");
                ui.label(
                    RichText::new("Direkt till den fysiska enheten")
                        .size(11.)
                        .color(MUTED),
                );
                ui.add_space(10.);
                let nodes: Vec<_> = self
                    .project
                    .nodes
                    .iter()
                    .map(|n| {
                        (
                            n.id,
                            n.name.clone(),
                            n.ports.len(),
                            self.project.roots.contains(&n.id),
                        )
                    })
                    .collect();
                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - 200.).max(100.))
                    .show(ui, |ui| {
                        for (id, name, count, root) in nodes {
                            let text = format!(
                                "{} {}\n    {} portar",
                                if root { "Huvudnod ·" } else { "" },
                                name,
                                count
                            );
                            if ui
                                .add_sized(
                                    [ui.available_width(), 54.],
                                    egui::Button::new(text).selected(self.selected == Some(id)),
                                )
                                .clicked()
                            {
                                self.select(id, None);
                                self.mode = Mode::Level;
                            }
                        }
                    });
                ui.add_space(14.);
                ui.separator();
                ui.menu_button("+ Ny enhet", |ui| {
                    if ui.button("Dator / PC").clicked() {
                        self.add_node("Ny PC", [210., 420., 380.]);
                        ui.close();
                    }
                    if ui.button("USB-hubb").clicked() {
                        self.add_node("Ny USB-hubb", [105., 25., 45.]);
                        ui.close();
                    }
                    if ui.button("Ljudmixer").clicked() {
                        self.add_node("Ny ljudmixer", [280., 70., 220.]);
                        ui.close();
                    }
                    if ui.button("Egen enhet").clicked() {
                        self.add_node("Ny enhet", [150., 80., 100.]);
                        ui.close();
                    }
                });
                ui.add_space(12.);
                ui.label(
                    RichText::new(
                        "Skapa enheter bara när deras portar ska mappas, t.ex. en hubb. Dubbelklicka en port för att skriva gamingmus, kamera eller annan utrustning som en etikett.",
                    )
                    .size(11.)
                    .color(MUTED),
                );
                if self.recovery.is_some() {
                    ui.add_space(18.);
                    if ui.button("Återställ föregående utkast").clicked() {
                        self.begin(Pending::Recovery);
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.small(RichText::new("NATIVE RUST  /  OFFLINE\nv0.2.0").color(MUTED));
                });
            });
    }

    fn inspector(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector")
            .default_width(330.)
            .width_range(305.0..=480.0)
            .show(ctx, |ui| {
                if palette::show(ui, self.selected.is_some()) {
                    self.editing = true;
                    self.mode = Mode::Level;
                }
                egui::ScrollArea::vertical()
                    .id_salt("inspector-scroll")
                    .show(ui, |ui| {
                        ui.add_space(12.);
                        let Some(id) = self.selected else {
                            section(ui, "PROJEKT");
                            ui.label("Skapa en enhet för att börja placera fysiska portar.");
                            return;
                        };
                        let Some(original) = self.project.node(id).cloned() else {
                            return;
                        };
                        section(ui, "ENHET");
                        ui.heading(&original.name);
                        if self.project.roots.contains(&id) {
                            ui.label(
                                RichText::new("Huvudnod / fristående enhet")
                                    .size(11.)
                                    .color(ACCENT),
                            );
                        } else if let Some(c) = self.project.parent(id) {
                            ui.label(
                                RichText::new(format!(
                                    "Via {}",
                                    self.project.target_label(
                                        c,
                                        match c.to {
                                            Target::Port(r) => r,
                                            _ => c.from,
                                        }
                                    )
                                ))
                                .size(11.)
                                .color(MUTED),
                            );
                        }
                        if !self.editing {
                            ui.label(
                                RichText::new("Välj Redigera för att ändra modell och kablar.")
                                    .size(11.)
                                    .color(MUTED),
                            );
                        }
                        ui.add_space(8.);
                        let mut draft = self
                            .draft
                            .clone()
                            .filter(|n| n.id == id)
                            .unwrap_or_else(|| original.clone());
                        ui.add_enabled_ui(self.editing, |ui| {
                            ui.label("Namn");
                            ui.text_edit_singleline(&mut draft.name);
                            if self.selected_part.is_none() {
                                self.selected_part = draft.parts.first().map(|p| p.id);
                            }
                            egui::ComboBox::from_id_salt("part-select")
                                .selected_text(
                                    draft
                                        .parts
                                        .iter()
                                        .find(|p| Some(p.id) == self.selected_part)
                                        .map(|p| p.name.as_str())
                                        .unwrap_or("Byggsten"),
                                )
                                .show_ui(ui, |ui| {
                                    for p in &draft.parts {
                                        ui.selectable_value(
                                            &mut self.selected_part,
                                            Some(p.id),
                                            &p.name,
                                        );
                                    }
                                });
                            if let Some(part) = draft
                                .parts
                                .iter_mut()
                                .find(|p| Some(p.id) == self.selected_part)
                            {
                                ui.text_edit_singleline(&mut part.name);
                                egui::ComboBox::from_id_salt("shape")
                                    .selected_text(if part.shape == Primitive::Box {
                                        "Box"
                                    } else {
                                        "Kil"
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut part.shape, Primitive::Box, "Box");
                                        ui.selectable_value(
                                            &mut part.shape,
                                            Primitive::Wedge,
                                            "Kil / sluttande ovansida",
                                        );
                                    });
                                ui.label(
                                    RichText::new("MÅTT I MM · X / Y / Z")
                                        .size(10.)
                                        .color(MUTED),
                                );
                                ui.horizontal(|ui| {
                                    for value in &mut part.size {
                                        ui.add(
                                            egui::DragValue::new(value)
                                                .range(4.0..=5000.0)
                                                .speed(1.)
                                                .max_decimals(1),
                                        );
                                    }
                                });
                                ui.label(
                                    RichText::new("POSITION I MM · X / Y / Z")
                                        .size(10.)
                                        .color(MUTED),
                                );
                                ui.horizontal(|ui| {
                                    for value in &mut part.offset {
                                        ui.add(
                                            egui::DragValue::new(value)
                                                .range(-10000.0..=10000.0)
                                                .speed(1.)
                                                .max_decimals(1),
                                        );
                                    }
                                });
                            }
                            self.draft = Some(draft.clone());
                            ui.horizontal(|ui| {
                                if ui.small_button("+ Box").clicked()
                                    && self.change(|p| p.add_part(id, Primitive::Box).map(|_| ()))
                                {
                                    draft = self.project.node(id).unwrap().clone();
                                    self.selected_part = draft.parts.last().map(|p| p.id);
                                }
                                if ui.small_button("+ Kil").clicked()
                                    && self.change(|p| p.add_part(id, Primitive::Wedge).map(|_| ()))
                                {
                                    draft = self.project.node(id).unwrap().clone();
                                    self.selected_part = draft.parts.last().map(|p| p.id);
                                }
                                if ui
                                    .add_enabled(
                                        draft.parts.len() > 1,
                                        egui::Button::new("Ta bort byggsten").small(),
                                    )
                                    .clicked()
                                    && let Some(part) = self.selected_part
                                    && self.change(|p| p.remove_part(id, part))
                                {
                                    draft = self.project.node(id).unwrap().clone();
                                    self.selected_part = draft.parts.first().map(|p| p.id);
                                }
                            });
                            egui::ComboBox::from_id_salt("default-view")
                                .selected_text(format!("Standardvy: {}", draft.default_view.name()))
                                .show_ui(ui, |ui| {
                                    for face in Face::ALL {
                                        ui.selectable_value(
                                            &mut draft.default_view,
                                            face,
                                            face.name(),
                                        );
                                    }
                                });
                        });
                        ui.add_space(10.);
                        ui.separator();
                        section(ui, "PORTAR");
                        let mut select_port = None;
                        for port in &original.ports {
                            let connected = self.project.is_occupied(PortRef {
                                node: id,
                                port: port.id,
                            });
                            if ui
                                .selectable_label(
                                    self.selected_port == Some(port.id),
                                    RichText::new(format!(
                                        "{} {}  ·  {}",
                                        if connected { "+" } else { "o" },
                                        port.name,
                                        port.face.name()
                                    ))
                                    .color(if connected {
                                        ACCENT
                                    } else {
                                        MUTED
                                    }),
                                )
                                .clicked()
                            {
                                select_port = Some(port.id);
                            }
                        }
                        if let Some(port) = select_port {
                            self.select(id, Some(port));
                        }
                        ui.small(
                            "Lägg till portar genom att dra en ikon från portbiblioteket ovanför.",
                        );
                        if let Some(port_id) = self.selected_port {
                            let reference = PortRef {
                                node: id,
                                port: port_id,
                            };
                            if let Some(port) = draft.ports.iter_mut().find(|p| p.id == port_id) {
                                ui.add_space(8.);
                                section(ui, "VALD FYSISK PORT");
                                ui.add_enabled_ui(self.editing, |ui| {
                                    ui.text_edit_singleline(&mut port.name);
                                    egui::ComboBox::from_id_salt("port-kind")
                                        .selected_text(port.kind.name())
                                        .show_ui(ui, |ui| {
                                            for kind in PortKind::ALL {
                                                ui.selectable_value(
                                                    &mut port.kind,
                                                    kind,
                                                    kind.name(),
                                                );
                                            }
                                        });
                                    egui::ComboBox::from_id_salt("port-part")
                                        .selected_text(
                                            draft
                                                .parts
                                                .iter()
                                                .find(|p| p.id == port.part)
                                                .map(|p| p.name.as_str())
                                                .unwrap_or("?"),
                                        )
                                        .show_ui(ui, |ui| {
                                            for part in &draft.parts {
                                                ui.selectable_value(
                                                    &mut port.part,
                                                    part.id,
                                                    &part.name,
                                                );
                                            }
                                        });
                                    egui::ComboBox::from_id_salt("port-face")
                                        .selected_text(format!("Yta: {}", port.face.name()))
                                        .show_ui(ui, |ui| {
                                            for face in Face::ALL {
                                                ui.selectable_value(
                                                    &mut port.face,
                                                    face,
                                                    face.name(),
                                                );
                                            }
                                        });
                                    if let Some(part) =
                                        draft.parts.iter().find(|p| p.id == port.part)
                                    {
                                        let size = part.surface_size(port.face, port.uv);
                                        let mut u = port.uv[0] * size[0];
                                        let mut v = port.uv[1] * size[1];
                                        ui.label(
                                            RichText::new("PLACERING FRÅN YTANS MITT · MM")
                                                .size(10.)
                                                .color(MUTED),
                                        );
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut u)
                                                        .prefix("U ")
                                                        .speed(0.5)
                                                        .max_decimals(1),
                                                )
                                                .changed()
                                            {
                                                port.uv[0] = u / size[0];
                                            }
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut v)
                                                        .prefix("V ")
                                                        .speed(0.5)
                                                        .max_decimals(1),
                                                )
                                                .changed()
                                            {
                                                port.uv[1] = v / size[1];
                                            }
                                        });
                                    }
                                    ui.horizontal(|ui| {
                                        ui.label("Rotation");
                                        ui.add(
                                            egui::DragValue::new(&mut port.angle)
                                                .suffix("°")
                                                .range(-360.0..=360.0)
                                                .speed(1.),
                                        );
                                        if ui.small_button("90°").clicked() {
                                            port.angle = (port.angle + 90.).rem_euclid(360.);
                                        }
                                    });
                                    ui.text_edit_singleline(&mut port.metadata)
                                        .on_hover_text("Version / kapacitet / egna anteckningar");
                                });
                                let [w, h] = port.kind.dimensions();
                                ui.label(
                                    RichText::new(format!("Fysisk storlek: {w} × {h} mm"))
                                        .size(11.)
                                        .color(MUTED),
                                );
                            }
                            self.draft = Some(draft.clone());
                            if let Some(c) = self.project.connection_at(reference).cloned() {
                                ui.add_space(8.);
                                ui.label(RichText::new("ANSLUTEN TILL").size(10.).color(ACCENT));
                                ui.strong(self.project.target_label(&c, reference));
                                if c.supplementary {
                                    ui.small("Extralänk · ingår inte i hierarkin");
                                }
                                if c.adapter {
                                    ui.small("Adapter / konverterande kabel");
                                }
                                if !c.label.is_empty() {
                                    ui.label(&c.label);
                                }
                                let mut cable_label = self
                                    .cable_drafts
                                    .get(&c.id)
                                    .cloned()
                                    .unwrap_or_else(|| c.label.clone());
                                if self.editing {
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(&mut cable_label)
                                            .on_hover_text("Kabeletikett / färg");
                                        if ui.small_button("Sätt").clicked() {
                                            self.change(|p| {
                                                p.connections
                                                    .iter_mut()
                                                    .find(|x| x.id == c.id)
                                                    .unwrap()
                                                    .label = cable_label.clone();
                                                Ok(())
                                            });
                                        }
                                    });
                                    self.cable_drafts.insert(c.id, cable_label);
                                }
                                match c.to {
                                    Target::Port(to) => {
                                        let other = if c.from == reference { to } else { c.from };
                                        if ui.button("Gå till den andra porten >").clicked() {
                                            self.select(other.node, Some(other.port));
                                            self.mode = Mode::Level;
                                        }
                                    }
                                    Target::Endpoint(endpoint) => {
                                        let route = self.project.route_to_endpoint(endpoint);
                                        let text = route
                                            .iter()
                                            .map(|r| {
                                                format!(
                                                    "{} [{}]",
                                                    self.project.node(r.node).unwrap().name,
                                                    self.project.port(*r).unwrap().name
                                                )
                                            })
                                            .collect::<Vec<_>>()
                                            .join(" > ");
                                        ui.label(RichText::new(text).size(11.).color(MUTED));
                                        if self.editing {
                                            let mut name = self
                                                .endpoint_drafts
                                                .get(&endpoint)
                                                .cloned()
                                                .unwrap_or_else(|| {
                                                    self.project
                                                        .endpoint(endpoint)
                                                        .unwrap()
                                                        .name
                                                        .clone()
                                                });
                                            ui.horizontal(|ui| {
                                                ui.text_edit_singleline(&mut name);
                                                if ui.small_button("Byt namn").clicked() {
                                                    self.change(|p| {
                                                        p.endpoints
                                                            .iter_mut()
                                                            .find(|e| e.id == endpoint)
                                                            .unwrap()
                                                            .name = name.clone();
                                                        Ok(())
                                                    });
                                                }
                                            });
                                            self.endpoint_drafts.insert(endpoint, name);
                                        }
                                    }
                                }
                                if self.editing && ui.button("Koppla loss kabel").clicked() {
                                    self.change(|p| {
                                        p.disconnect(reference);
                                        Ok(())
                                    });
                                    draft = self.project.node(id).unwrap().clone();
                                }
                            } else if self.project.port(reference).is_some() {
                                ui.label(RichText::new("LEDIG PORT").color(MUTED));
                                if self.editing {
                                    self.connection_editor(ui, reference);
                                }
                            }
                            if self.editing
                                && ui
                                    .button(
                                        RichText::new("Ta bort port och dess kabel").color(ORANGE),
                                    )
                                    .clicked()
                                && self.change(|p| {
                                    p.remove_port(reference);
                                    Ok(())
                                })
                            {
                                self.selected_port = None;
                                draft = self.project.node(id).unwrap().clone();
                            }
                        }
                        ui.add_space(10.);
                        if self.editing {
                            if draft != original {
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(RichText::new("Tillämpa ändringar").color(ACCENT))
                                        .clicked()
                                    {
                                        let replacement = draft.clone();
                                        if self.change(|p| {
                                            *p.node_mut(id).unwrap() = replacement;
                                            Ok(())
                                        }) {
                                            draft = self.project.node(id).unwrap().clone();
                                            if let Some(port) =
                                                self.selected_port.and_then(|port| {
                                                    self.project.port(PortRef { node: id, port })
                                                })
                                            {
                                                self.view.set_face(port.face);
                                            }
                                        }
                                    }
                                    if ui.button("Återställ").clicked() {
                                        draft = self.project.node(id).unwrap().clone();
                                    }
                                });
                            }
                            ui.add_space(10.);
                            ui.separator();
                            if ui
                                .button(RichText::new("Ta bort enhet").color(ORANGE))
                                .on_hover_text(
                                    "Tar även bort enhetens portar och kablar. Kan ångras.",
                                )
                                .clicked()
                                && self.change(|p| {
                                    p.remove_node(id);
                                    Ok(())
                                })
                            {
                                self.selected = self.project.roots.first().copied();
                                self.selected_port = None;
                                self.draft = None;
                                return;
                            }
                        }
                        if self.selected == Some(id) {
                            self.draft = Some(draft);
                        }
                    });
            });
    }

    fn connection_editor(&mut self, ui: &mut egui::Ui, reference: PortRef) {
        section(ui, "KOPPLA EN KABEL");
        ui.label("Slutpunkt · bara en etikett");
        ui.text_edit_singleline(&mut self.endpoint_name)
            .on_hover_text("Exempel: Webbkamera, Stream Deck, mikrofon");
        if ui
            .add_enabled(
                !self.endpoint_name.trim().is_empty(),
                egui::Button::new("Anslut slutpunkt"),
            )
            .clicked()
        {
            let name = self.endpoint_name.clone();
            if self.change(|p| p.connect_endpoint(reference, &name).map(|_| ())) {
                self.endpoint_name.clear();
            }
        }
        ui.add_space(8.);
        ui.label("Port på en annan enhet");
        let nodes: Vec<_> = self
            .project
            .nodes
            .iter()
            .filter(|n| n.id != reference.node)
            .map(|n| (n.id, n.name.clone()))
            .collect();
        if !self
            .target_node
            .is_some_and(|id| nodes.iter().any(|n| n.0 == id))
        {
            self.target_node = nodes.first().map(|n| n.0);
            self.target_port = None;
        }
        let selected = self.target_node;
        egui::ComboBox::from_id_salt("target-node")
            .selected_text(
                nodes
                    .iter()
                    .find(|n| Some(n.0) == self.target_node)
                    .map(|n| n.1.as_str())
                    .unwrap_or("Skapa en annan enhet"),
            )
            .show_ui(ui, |ui| {
                for (id, name) in &nodes {
                    ui.selectable_value(&mut self.target_node, Some(*id), name);
                }
            });
        if self.target_node != selected {
            self.target_port = None;
        }
        let ports: Vec<_> = self
            .target_node
            .and_then(|id| self.project.node(id))
            .map(|n| {
                n.ports
                    .iter()
                    .map(|p| {
                        (
                            p.id,
                            p.name.clone(),
                            p.kind,
                            self.project.is_occupied(PortRef {
                                node: n.id,
                                port: p.id,
                            }),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        if !self
            .target_port
            .is_some_and(|id| ports.iter().any(|p| p.0 == id && !p.3))
        {
            self.target_port = ports.iter().find(|p| !p.3).map(|p| p.0);
        }
        egui::ComboBox::from_id_salt("target-port")
            .selected_text(
                ports
                    .iter()
                    .find(|p| Some(p.0) == self.target_port)
                    .map(|p| format!("{} · {}", p.1, p.2.name()))
                    .unwrap_or_else(|| "Ingen ledig port".into()),
            )
            .show_ui(ui, |ui| {
                for (id, name, kind, used) in &ports {
                    ui.add_enabled_ui(!used, |ui| {
                        ui.selectable_value(
                            &mut self.target_port,
                            Some(*id),
                            format!(
                                "{} · {}{}",
                                name,
                                kind.name(),
                                if *used { " (upptagen)" } else { "" }
                            ),
                        );
                    });
                }
            });
        ui.checkbox(&mut self.supplementary, "Extralänk utanför trädhierarkin");
        ui.checkbox(&mut self.adapter, "Kabeln har en adapter / konverterare");
        if ui
            .add_enabled(
                self.target_port.is_some(),
                egui::Button::new("Anslut enheterna"),
            )
            .clicked()
            && let (Some(node), Some(port)) = (self.target_node, self.target_port)
        {
            let supplementary = self.supplementary;
            let adapter = self.adapter;
            self.change(|p| {
                p.connect(
                    reference,
                    Target::Port(PortRef { node, port }),
                    supplementary,
                    adapter,
                )
                .map(|_| ())
            });
        }
    }

    fn apply_scene_event(&mut self, node: Id, event: scene::SceneEvent) {
        if let Some(port) = event.selected {
            self.selected_port = Some(port);
        }
        if event.drag_started && self.flush_draft() {
            self.remember();
        }
        if let Some((port, uv)) = event.moved {
            let mut candidate = self.with_drafts();
            if let Some(p) = candidate.port_mut(PortRef { node, port }) {
                p.uv = uv;
            }
            if candidate.validate().is_ok() {
                self.project = candidate;
                self.draft = None;
                self.cable_drafts.clear();
                self.endpoint_drafts.clear();
            }
        }
        if let Some(drop) = event.dropped {
            let mut id = 0;
            if self.change(|p| {
                id = p.place_port(node, drop.kind, drop.hit.part, drop.hit.face, drop.hit.uv)?;
                Ok(())
            }) {
                self.selected_port = Some(id);
                self.selected_part = Some(drop.hit.part);
                self.editing = true;
                self.status =
                    "Port placerad. Dubbelklicka för att ange ansluten utrustning.".into();
            }
        }
        if event.drop_rejected {
            self.status = "Släpp porten på en synlig yta där kontakten får plats.".into();
        }
        if let Some(port) = event.delete
            && self.change(|p| {
                p.remove_port(PortRef { node, port });
                Ok(())
            })
        {
            if self.selected_port == Some(port) {
                self.selected_port = None;
            }
            self.editing = true;
            self.status = "Porten och dess anslutning borttagna. Ctrl+Z ångrar.".into();
        }
        if let Some(port) = event.edit_label
            && self.flush_draft()
        {
            self.selected_port = Some(port);
            self.editing = true;
            self.port_label = PortLabel::new(&self.project, PortRef { node, port });
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
                if let Some(n) = self.project.node(id) {
                    let name = n.name.clone();
                    if ui.selectable_label(id == node.id, name).clicked() {
                        self.select(id, None);
                    }
                    if id != node.id {
                        ui.label(RichText::new("/").color(MUTED));
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.heading(&node.name);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(if self.editing {
                        "REDIGERINGSLÄGE"
                    } else {
                        "VISNINGSLÄGE"
                    })
                    .size(10.)
                    .color(if self.editing { ORANGE } else { ACCENT }),
                );
            });
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
            if ui.button("Standardvy").clicked() {
                self.view.set_face(node.default_view);
                self.view.reset();
            }
            if ui.small_button("−").clicked() {
                self.view.zoom = (self.view.zoom / 1.25).max(0.2);
            }
            if ui.small_button("+").clicked() {
                self.view.zoom = (self.view.zoom * 1.25).min(18.);
            }
            ui.checkbox(&mut self.view.labels, "Etiketter");
        });
        ui.label(
            RichText::new(if self.editing {
                "Dra portar från biblioteket · dubbelklicka för text · högerklicka för Delete"
            } else {
                "Välj sida med knapparna · dubbelklicka en port för text · scrolla för zoom"
            })
            .size(11.)
            .color(MUTED),
        );
        ui.add_space(5.);
        // The lower connection list provides accessible port selection even for hidden faces.
        let list_height = (node.ports.len() as f32 * 34. + 45.).clamp(60., 225.);
        let scene_height = (ui.available_height() - list_height - 10.).max(200.);
        let mut event = None;
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), scene_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                event = Some(scene::draw(
                    ui,
                    &self.project,
                    &node,
                    self.selected_port,
                    self.editing,
                    &mut self.view,
                ));
            },
        );
        if let Some(event) = event {
            self.apply_scene_event(node.id, event);
        }
        ui.add_space(4.);
        egui::ScrollArea::vertical()
            .id_salt("level-connections")
            .max_height(list_height)
            .show(ui, |ui| {
                if node.ports.is_empty() {
                    ui.label(
                        RichText::new(
                            "Inga portar ännu. Välj Redigera och lägg till från portbiblioteket.",
                        )
                        .color(MUTED),
                    );
                }
                for port in &node.ports {
                    let r = PortRef {
                        node: node.id,
                        port: port.id,
                    };
                    let connection = self.project.connection_at(r).cloned();
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                self.selected_port == Some(port.id),
                                format!(
                                    "{}  {}",
                                    if connection.is_some() { "+" } else { "o" },
                                    port.name
                                ),
                            )
                            .clicked()
                        {
                            self.select(node.id, Some(port.id));
                        }
                        ui.label(RichText::new(port.kind.name()).size(11.).color(MUTED));
                        if let Some(c) = connection {
                            let label = self.project.target_label(&c, r);
                            match c.to {
                                Target::Port(to) => {
                                    let other = if c.from == r { to } else { c.from };
                                    if ui
                                        .button(RichText::new(format!("> {label}")).color(ACCENT))
                                        .clicked()
                                    {
                                        self.select(other.node, Some(other.port));
                                    }
                                }
                                Target::Endpoint(_) => {
                                    ui.label(RichText::new(format!("> {label}")).color(ACCENT));
                                }
                            }
                        } else {
                            ui.label(RichText::new("Ledig").size(11.).color(MUTED));
                        }
                    });
                }
            });
    }

    fn welcome(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui|{
            ui.add_space((ui.available_height()*0.17).max(35.));ui.label(RichText::new("CM").size(72.).color(ACCENT));ui.add_space(15.);
            ui.label(RichText::new("Var sitter kabeln?").size(31.).strong());ui.add_space(10.);
            ui.label(RichText::new("Bygg en fysisk karta över ditt skrivbord.\nRätt enhet. Rätt port. Enkel återinkoppling.").size(16.).color(MUTED));ui.add_space(24.);
            if ui.add_sized([230.,44.],egui::Button::new(RichText::new("+ Skapa din första PC").color(ACCENT))).clicked(){self.add_node("Min PC",[210.,420.,380.]);}
            if ui.add_sized([230.,40.],egui::Button::new("Öppna fungerande exempel")).clicked(){self.begin(Pending::Demo);}
            ui.add_space(16.);ui.label(RichText::new("Exemplet visar PC > USB-hubb > webbkamera.\nSlutpunkter är etiketter, enheter har portar.").size(12.).color(MUTED));
        });
    }

    fn full_tree(&mut self, ui: &mut egui::Ui) {
        if self.project.nodes.is_empty() {
            self.welcome(ui);
            return;
        }
        ui.add_space(8.);
        ui.horizontal(|ui| {
            ui.heading("Hela anslutningsträdet");
            if ui.button("Anpassa till fönstret").clicked() {
                self.tree_fit = true;
            }
            if ui.small_button("−").clicked() {
                self.tree_zoom = (self.tree_zoom / 1.2).max(0.15);
            }
            if ui.small_button("+").clicked() {
                self.tree_zoom = (self.tree_zoom * 1.2).min(2.5);
            }
        });
        ui.label(RichText::new("Alla grenar visas · dra bakgrunden för panorering · scrolla för zoom · välj en port eller öppna 3D").size(11.).color(MUTED));
        ui.add_space(7.);
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0., Color32::from_rgb(18, 24, 31));
        let placements = tree::layout(&self.project);
        let mut heights: HashMap<usize, f32> = HashMap::new();
        for place in &placements {
            let depth = (place.position[1] / 310.).round() as usize;
            let height = self
                .project
                .node(place.id)
                .map(|n| 90. + n.ports.len() as f32 * 24.)
                .unwrap_or(90.);
            heights
                .entry(depth)
                .and_modify(|h| *h = h.max(height))
                .or_insert(height);
        }
        let mut ys = HashMap::new();
        let mut y = 0.;
        for depth in 0..=heights.keys().copied().max().unwrap_or(0) {
            ys.insert(depth, y);
            y += heights.get(&depth).copied().unwrap_or(90.) + 90.;
        }
        let positions: HashMap<_, _> = placements
            .iter()
            .map(|p| {
                (
                    p.id,
                    Vec2::new(
                        p.position[0],
                        ys[&((p.position[1] / 310.).round() as usize)],
                    ),
                )
            })
            .collect();
        let min_x = positions
            .values()
            .map(|p| p.x - 145.)
            .fold(f32::INFINITY, f32::min);
        let max_x = positions.values().map(|p| p.x + 145.).fold(0., f32::max);
        if self.tree_fit {
            self.tree_zoom = ((rect.width() - 50.) / (max_x - min_x).max(300.))
                .min((rect.height() - 50.) / y.max(150.))
                .clamp(0.15, 1.2);
            self.tree_pan = Vec2::new(
                rect.width() / 2. - (min_x + max_x) / 2. * self.tree_zoom,
                25.,
            );
            self.tree_fit = false;
        }
        let origin = rect.left_top() + self.tree_pan;
        let scale = self.tree_zoom;
        let node_rect = |id| {
            let n = self.project.node(id).unwrap();
            let p = positions[&id];
            Rect::from_min_size(
                origin + Vec2::new(p.x - 145., p.y) * scale,
                Vec2::new(290., 90. + n.ports.len() as f32 * 24.) * scale,
            )
        };
        let anchor = |r: PortRef, right: bool| {
            let n = self.project.node(r.node).unwrap();
            let index = n.ports.iter().position(|p| p.id == r.port).unwrap_or(0);
            let card = node_rect(r.node);
            Pos2::new(
                if right { card.right() } else { card.left() },
                card.top() + (73. + index as f32 * 24.) * scale,
            )
        };
        for c in &self.project.connections {
            if let Target::Port(to) = c.to {
                if !positions.contains_key(&c.from.node) || !positions.contains_key(&to.node) {
                    continue;
                }
                let start = anchor(c.from, true);
                let end = anchor(to, false);
                let bend = (end.y - start.y).abs().max(80. * scale) * 0.35;
                let color = if c.supplementary { ORANGE } else { ACCENT };
                painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                    [
                        start,
                        start + Vec2::new(65. * scale, bend),
                        end - Vec2::new(65. * scale, bend),
                        end,
                    ],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(if c.supplementary { 1. } else { 1.7_f32 }, color),
                ));
                if c.supplementary {
                    painter.text(
                        start.lerp(end, 0.5),
                        Align2::CENTER_CENTER,
                        "extralänk",
                        FontId::proportional(10. * scale.max(0.7)),
                        ORANGE,
                    );
                }
            }
        }
        let mut choice = None;
        let mut level = None;
        for place in &placements {
            let node = self.project.node(place.id).unwrap();
            let card = node_rect(node.id);
            if !card.intersects(rect) {
                continue;
            }
            painter.rect_filled(card, 7., Color32::from_rgb(29, 39, 48));
            painter.rect_stroke(
                card,
                7.,
                Stroke::new(
                    if self.selected == Some(node.id) {
                        1.8
                    } else {
                        1_f32
                    },
                    if self.selected == Some(node.id) {
                        ACCENT
                    } else {
                        Color32::from_rgb(61, 78, 91)
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let header = Rect::from_min_size(card.min, Vec2::new(card.width(), 54. * scale));
            let title = painter.layout(
                node.name.clone(),
                FontId::proportional(16. * scale),
                Color32::WHITE,
                220. * scale,
            );
            painter.galley(
                card.min + Vec2::new(15., 13.) * scale,
                title,
                Color32::WHITE,
            );
            painter.text(
                card.right_top() + Vec2::new(-13., 18.) * scale,
                Align2::RIGHT_TOP,
                "3D >",
                FontId::proportional(11. * scale),
                ACCENT,
            );
            if ui
                .interact(header, ui.id().with(("tree-node", node.id)), Sense::click())
                .on_hover_text("Öppna enhetens fysiska modell")
                .clicked()
            {
                level = Some(node.id);
            }
            for (index, port) in node.ports.iter().enumerate() {
                let r = PortRef {
                    node: node.id,
                    port: port.id,
                };
                let connected = self.project.connection_at(r);
                let row = Rect::from_min_size(
                    card.min + Vec2::new(8., 61. + index as f32 * 24.) * scale,
                    Vec2::new(274., 24.) * scale,
                );
                if self.selected == Some(node.id) && self.selected_port == Some(port.id) {
                    painter.rect_filled(row, 2., Color32::from_rgb(61, 67, 58));
                }
                painter.circle_filled(
                    row.left_center() + Vec2::new(8. * scale, 0.),
                    3. * scale,
                    if connected.is_some() { ACCENT } else { MUTED },
                );
                let text = if let Some(c) = connected {
                    format!("{}  >  {}", port.name, self.project.target_label(c, r))
                } else {
                    format!("{}  ·  ledig", port.name)
                };
                let mut job = egui::text::LayoutJob::simple(
                    text.clone(),
                    FontId::proportional(11. * scale),
                    if connected.is_some() { ACCENT } else { MUTED },
                    245. * scale,
                );
                job.wrap.max_rows = 1;
                job.wrap.overflow_character = Some('…');
                let galley = painter.layout_job(job);
                painter.galley(row.min + Vec2::new(17., 3.) * scale, galley, Color32::WHITE);
                if ui
                    .interact(row, ui.id().with(("tree-port", port.id)), Sense::click())
                    .on_hover_text(format!(
                        "{text}\n{} · {}",
                        port.kind.name(),
                        port.face.name()
                    ))
                    .clicked()
                {
                    choice = Some((node.id, port.id));
                }
            }
        }
        if let Some((node, port)) = choice {
            self.select(node, Some(port));
        }
        if let Some(node) = level {
            self.select(node, None);
            self.mode = Mode::Level;
        }
        if response.dragged() {
            self.tree_pan += ui.input(|i| i.pointer.delta());
        }
        if response.hovered() {
            let wheel = ui.input(|i| i.smooth_scroll_delta.y);
            if wheel != 0. {
                let old = self.tree_zoom;
                self.tree_zoom = (old * (wheel * 0.0025).exp()).clamp(0.15, 2.5);
                if let Some(pos) = response.hover_pos() {
                    let local = pos - rect.left_top();
                    self.tree_pan = local - (local - self.tree_pan) * (self.tree_zoom / old);
                }
            }
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.error.is_none() {
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
                        self.status = "Utrustningens namn sparat som etikett vid porten.".into();
                    }
                }
                Some(port_label::Action::Cancel) => self.port_label = None,
                Some(port_label::Action::GoTo(port)) => {
                    self.port_label = None;
                    self.select(port.node, Some(port.port));
                    self.mode = Mode::Level;
                }
                None => {}
            }
        }
        if let Some(error) = self.error.clone() {
            egui::Modal::new(egui::Id::new("error")).show(ctx, |ui| {
                ui.heading("Det gick inte att utföra ändringen");
                ui.label(error);
                if ui.button("Stäng").clicked() {
                    self.error = None;
                }
            });
        }
        if let Some(action) = self.pending.clone() {
            egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
                ui.heading("Spara ändringarna?");
                ui.label("Projektet har ändringar som inte är sparade.");
                ui.horizontal(|ui| {
                    if ui.button("Spara och fortsätt").clicked() {
                        self.pending = None;
                        self.after_save = Some(action.clone());
                        self.save(false);
                    }
                    if ui.button("Fortsätt utan att spara").clicked() {
                        self.pending = None;
                        self.perform(action.clone());
                    }
                    if ui.button("Avbryt").clicked() {
                        self.pending = None;
                    }
                });
            });
        }
        if self.file_dialog.is_some() {
            egui::Modal::new(egui::Id::new("path-dialog")).show(ctx, |ui| {
                let save = matches!(self.file_dialog, Some(FileDialog::Save));
                ui.heading(if save {
                    "Spara projekt"
                } else {
                    "Öppna projekt"
                });
                ui.label("Sökväg till projektfil (.cable.json)");
                ui.add_sized(
                    [520., 30.],
                    egui::TextEdit::singleline(&mut self.file_input),
                );
                ui.horizontal(|ui| {
                    if ui.button(if save { "Spara" } else { "Öppna" }).clicked() {
                        let path = PathBuf::from(self.file_input.trim());
                        self.file_dialog = None;
                        if save {
                            self.save_path(path);
                        } else {
                            self.load_path(path);
                        }
                    }
                    if ui.button("Avbryt").clicked() {
                        self.file_dialog = None;
                        self.after_save = None;
                    }
                });
            });
        }
        if self.help {
            egui::Window::new("Så använder du Cable Management").open(&mut self.help).default_width(550.).show(ctx,|ui|{
            ui.label("1. Skapa PC, hubb eller mixer med Ny enhet.");ui.label("2. Välj Redigera. Justera höljet och tillämpa ändringarna.");ui.label("3. Välj en sida. Dra en portikon från högerpaletten och släpp den på enheten.");ui.label("4. Dra portarna på ytan eller ange U/V och rotation i mm.");ui.label("5. Dubbelklicka porten och skriv t.ex. gamingmus. Skapa en separat enhet bara om den har portar som ska mappas.");ui.label("6. Växla mellan hela trädet och nivåvis navigering.");ui.separator();
            ui.label("Modellen byter sida enbart med sidknapparna. Scroll zoomar och mittenknappen panorerar. Dra på en port för att flytta den. Högerklicka på en port och välj Delete för att ta bort den.");ui.label("Ctrl+S spara · Ctrl+Skift+S spara som · Ctrl+O öppna\nCtrl+N nytt · Ctrl+Z ångra · Ctrl+Y gör om");ui.separator();
            ui.label("Porterna har fasta fysiska mått. Höljets storlek ändrar placeringen men förstorar aldrig kontakten. En adapter måste anges när kontaktfamiljerna skiljer sig.");
            ui.label("Sparade projekt är lokal, versionerad JSON. Föregående sparning ligger i .bak. Osparade ändringar får ett separat återställningsutkast var 15:e sekund.");
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
            || self.port_label.is_some();
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
        self.inspector(ctx);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(20, 27, 34))
                    .inner_margin(14.),
            )
            .show(ctx, |ui| match self.mode {
                Mode::Level => self.level(ui),
                Mode::Tree => self.full_tree(ui),
            });
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
                app.inspector(ctx);
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
        p.add_node("Min PC", [200., 300., 200.]);
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
            vec![egui::Event::Text("gamingmus".into())],
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
        assert_eq!(app.project.endpoints[0].name, "gamingmus");
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
    fn unapplied_editor_values_are_dirty_and_save_flushes_them() {
        let mut a = app();
        let mut draft = a.project.nodes[0].clone();
        draft.name = "Ändrat PC-namn".into();
        a.draft = Some(draft);
        assert!(a.dirty());
        let file = std::env::temp_dir().join(format!("cable-ui-{}.json", std::process::id()));
        a.path = Some(file.clone());
        a.save(false);
        assert_eq!(
            storage::load(&file).unwrap().nodes[0].name,
            "Ändrat PC-namn"
        );
        assert!(!a.dirty());
        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_file(storage::backup_path(&file));
    }
    #[test]
    fn switching_nodes_preserves_valid_edits_and_blocks_invalid_edits() {
        let mut a = app();
        let first = a.project.nodes[0].id;
        let second = a.project.nodes[1].id;
        let mut draft = a.project.nodes[0].clone();
        draft.name = "Min dator".into();
        a.draft = Some(draft);
        a.select(second, None);
        assert_eq!(a.project.node(first).unwrap().name, "Min dator");
        assert_eq!(a.selected, Some(second));
        let mut invalid = a.project.node(second).unwrap().clone();
        invalid.name.clear();
        a.draft = Some(invalid);
        a.select(first, None);
        assert_eq!(a.selected, Some(second));
        assert!(a.error.is_some());
        assert!(a.draft.is_some());
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
    fn adding_a_node_retains_pending_editor_changes() {
        let mut a = app();
        let first = a.project.nodes[0].id;
        let mut draft = a.project.nodes[0].clone();
        draft.name = "Bevarat namn".into();
        a.draft = Some(draft);
        a.add_node("Mixer", [200., 80., 150.]);
        assert_eq!(a.project.node(first).unwrap().name, "Bevarat namn");
    }
    #[test]
    fn endpoint_and_cable_text_buffers_are_saved() {
        let mut a = app();
        let endpoint = a.project.endpoints[0].id;
        let cable = a.project.connections[0].id;
        a.endpoint_drafts
            .insert(endpoint, "Kamera ovanför skärmen".into());
        a.cable_drafts.insert(cable, "Blå kabel".into());
        let file =
            std::env::temp_dir().join(format!("cable-ui-labels-{}.json", std::process::id()));
        a.path = Some(file.clone());
        a.save(false);
        let p = storage::load(&file).unwrap();
        assert_eq!(p.endpoint(endpoint).unwrap().name, "Kamera ovanför skärmen");
        assert_eq!(p.connections[0].label, "Blå kabel");
        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_file(storage::backup_path(&file));
    }
}
