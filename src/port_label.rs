use cable_management::model::*;
use eframe::egui;

pub struct PortLabel {
    pub port: PortRef,
    pub text: String,
    focus: bool,
}
pub enum Action {
    Save,
    Cancel,
}
impl PortLabel {
    pub fn has_changes(&self, project: &Project) -> bool {
        match project
            .connection_at(self.port)
            .map(|connection| connection.to)
        {
            Some(Target::Endpoint(id)) => project
                .endpoint(id)
                .is_some_and(|e| e.name != self.text.trim()),
            None => !self.text.trim().is_empty(),
            _ => false,
        }
    }
    pub fn new(project: &Project, port: PortRef) -> Option<Self> {
        project.port(port)?;
        if project.connected_peer(port).is_some() {
            return None;
        }
        let text = project
            .connection_at(port)
            .and_then(|connection| match connection.to {
                Target::Endpoint(id) => project.endpoint(id).map(|e| e.name.clone()),
                _ => None,
            })
            .unwrap_or_default();
        Some(Self {
            port,
            text,
            focus: true,
        })
    }
    pub fn show(&mut self, ctx: &egui::Context, project: &Project) -> Option<Action> {
        let port = project.port(self.port)?;
        let mut action = None;
        egui::Modal::new(egui::Id::new("port-label-editor")).show(ctx, |ui| {
            ui.set_min_width(370.);
            ui.heading("Connected equipment");
            ui.label(format!(
                "{} · {} · {}",
                port.name,
                port.kind.name(),
                port.face.name()
            ));
            ui.label("Enter an equipment name, such as gaming mouse or webcam.");
            let response = ui.add_sized(
                [370., 30.],
                egui::TextEdit::singleline(&mut self.text).hint_text("e.g. gaming mouse"),
            );
            if self.focus {
                response.request_focus();
                self.focus = false;
            }
            ui.small("The name appears beside this port. Clear it to remove the equipment label.");
            ui.horizontal(|ui| {
                if ui.button("Save label").clicked()
                    || ctx.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    action = Some(Action::Save);
                }
                if ui.button("Cancel").clicked() {
                    action = Some(Action::Cancel);
                }
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                action = Some(Action::Cancel);
            }
        });
        action
    }
}
