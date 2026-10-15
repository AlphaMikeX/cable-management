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
    GoTo(PortRef),
}
impl PortLabel {
    pub fn has_changes(&self, project: &Project) -> bool {
        match project.connection_at(self.port).map(|c| c.to) {
            Some(Target::Endpoint(id)) => project
                .endpoint(id)
                .is_some_and(|e| e.name != self.text.trim()),
            None => !self.text.trim().is_empty(),
            _ => false,
        }
    }
    pub fn new(project: &Project, port: PortRef) -> Option<Self> {
        project.port(port)?;
        let text = project
            .connection_at(port)
            .and_then(|c| match c.to {
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
            ui.heading("Ansluten utrustning");
            ui.label(format!(
                "{} · {} · {}",
                port.name,
                port.kind.name(),
                port.face.name()
            ));
            if let Some(c) = project.connection_at(self.port)
                && let Target::Port(to) = c.to
            {
                let peer = if c.from == self.port { to } else { c.from };
                ui.label(format!(
                    "Den här porten är kopplad till {}.",
                    project.target_label(c, self.port)
                ));
                ui.label("Enheten har egna portar som mappas separat.");
                if ui.button("Gå till den enheten").clicked() {
                    action = Some(Action::GoTo(peer));
                }
                if ui.button("Stäng").clicked() {
                    action = Some(Action::Cancel);
                }
            } else {
                ui.label("Skriv ett namn, till exempel gamingmus eller webbkamera.");
                let response = ui.add_sized(
                    [370., 30.],
                    egui::TextEdit::singleline(&mut self.text).hint_text("t.ex. gamingmus"),
                );
                if self.focus {
                    response.request_focus();
                    self.focus = false;
                }
                ui.small("Namnet visas vid porten. Ingen extra enhet behöver skapas.");
                ui.horizontal(|ui| {
                    if ui.button("Spara etikett").clicked()
                        || ctx.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        action = Some(Action::Save);
                    }
                    if ui.button("Avbryt").clicked() {
                        action = Some(Action::Cancel);
                    }
                });
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                action = Some(Action::Cancel);
            }
        });
        action
    }
}
