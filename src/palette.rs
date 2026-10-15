use crate::scene::{ACCENT, MUTED};
use cable_management::ports::PortKind;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2};

/// All connector types are actual drag sources, with recognizable vector icons.
pub fn show(ui: &mut egui::Ui, enabled: bool) -> bool {
    ui.label(
        egui::RichText::new("PORT LIBRARY · 20 TYPES")
            .size(10.)
            .strong()
            .color(ACCENT),
    );
    ui.label(
        egui::RichText::new(if enabled {
            "Drag a port onto the visible device surface."
        } else {
            "Create or select a device to map its ports."
        })
        .size(11.)
        .color(MUTED),
    );
    let mut started = false;
    let height = ui.available_height();
    egui::ScrollArea::vertical().id_salt("port-palette-scroll").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible).max_height(height).show(ui,|ui|{
        ui.add_enabled_ui(enabled,|ui|{
            let width=(ui.available_width()-ui.spacing().item_spacing.x)/2.;
            for (row,kinds) in PortKind::ALL.chunks(2).enumerate(){
                ui.horizontal(|ui|{
                    for (column,kind) in kinds.iter().copied().enumerate(){
                        ui.push_id(("palette",row,column),|ui|{
                            let (rect,response)=ui.allocate_exact_size(Vec2::new(width,54.),Sense::drag());
                            response.dnd_set_drag_payload(kind);started|=response.drag_started();
                            let fill=if response.hovered(){Color32::from_rgb(44,62,67)}else{Color32::from_rgb(31,43,52)};
                            ui.painter().rect_filled(rect,5.,fill);
                            let color=if ui.is_enabled(){ACCENT}else{MUTED};
                            icon(ui.painter(),kind,Rect::from_min_size(rect.min+Vec2::new(9.,13.),Vec2::new(40.,28.)),color);
                            let text=ui.painter().layout(kind.name().into(),FontId::proportional(11.),color,(width-61.).max(35.));
                            ui.painter().galley(Pos2::new(rect.left()+56.,rect.center().y-text.size().y/2.),text,color);
                            let [w,h]=kind.dimensions();
                            response.on_hover_cursor(egui::CursorIcon::Grab).on_hover_text(format!("Drag {} onto the visible device surface.\nSocket opening: {w} × {h} mm",kind.name()));
                        });
                    }
                });
            }
        });
    });
    ui.separator();
    started
}

pub fn icon(painter: &egui::Painter, kind: PortKind, rect: Rect, color: Color32) {
    let [w, h] = kind.dimensions();
    let scale = (rect.width() / w).min(rect.height() / h);
    let size = Vec2::new(w, h) * scale;
    let center = rect.center();
    let point = |p: [f32; 2]| center + Vec2::new(p[0] * size.x, -p[1] * size.y);
    painter.add(Shape::convex_polygon(
        kind.outline().into_iter().map(point).collect(),
        Color32::from_rgb(11, 18, 24),
        Stroke::new(1.3_f32, color),
    ));
    let line = |a, b| painter.line_segment([point(a), point(b)], Stroke::new(1.2_f32, color));
    match kind {
        PortKind::Xlr => {
            for p in [[-0.17, 0.12], [0.17, 0.12], [0., -0.2]] {
                painter.circle_filled(point(p), size.x * 0.05, color);
            }
        }
        PortKind::UsbA
        | PortKind::UsbB
        | PortKind::UsbC
        | PortKind::MiniUsb
        | PortKind::MicroUsb => {
            line([-0.3, -0.15], [0.3, -0.15]);
        }
        PortKind::Vga | PortKind::Dvi => {
            for row in 0..3 {
                for col in 0..5 {
                    painter.circle_filled(
                        point([-0.32 + col as f32 * 0.16, -0.23 + row as f32 * 0.23]),
                        (size.y * 0.045).max(0.6),
                        color,
                    );
                }
            }
        }
        PortKind::Hdmi
        | PortKind::DisplayPort
        | PortKind::MiniDisplayPort
        | PortKind::Rj45
        | PortKind::Sata => {
            for i in 0..6 {
                let x = -0.32 + i as f32 * 0.128;
                line([x, -0.2], [x, 0.1]);
            }
        }
        PortKind::IecC14 => {
            for p in [[-0.2, -0.12], [0.2, -0.12], [0., 0.2]] {
                line([p[0], p[1] - 0.09], [p[0], p[1] + 0.09]);
            }
        }
        PortKind::Toslink => {
            line([-0.2, 0.2], [0.2, -0.2]);
            line([-0.2, -0.2], [0.2, 0.2]);
        }
        _ => {
            painter.circle_stroke(
                center,
                size.x.min(size.y) * 0.18,
                Stroke::new(1.1_f32, color),
            );
        }
    }
}

pub fn drag_indicator(ctx: &egui::Context) {
    if let (Some(kind), Some(pos)) = (
        egui::DragAndDrop::payload::<PortKind>(ctx),
        ctx.pointer_interact_pos(),
    ) {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("port-drag-indicator"),
        ));
        let label = painter.layout_no_wrap(kind.name().into(), FontId::proportional(12.), ACCENT);
        let rect = Rect::from_min_size(
            pos + Vec2::new(19., 20.),
            label.size() + Vec2::new(14., 10.),
        );
        painter.rect_filled(rect, 4., Color32::from_rgb(17, 34, 34));
        painter.galley(rect.min + Vec2::new(7., 5.), label, ACCENT);
        painter.text(
            rect.left_top() - Vec2::new(0., 4.),
            Align2::LEFT_BOTTOM,
            "Drop onto the device",
            FontId::proportional(10.),
            MUTED,
        );
    }
}
