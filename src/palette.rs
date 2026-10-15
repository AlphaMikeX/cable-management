use crate::scene::{ACCENT, MUTED};
use cable_management::ports::PortKind;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2};

/// The angle travels with the drag, independent of subsequent library changes.
#[derive(Clone, Copy, Debug)]
pub struct PortDrag {
    pub kind: PortKind,
    pub angle: f32,
}
impl PortDrag {
    pub fn new(kind: PortKind, clockwise_turns: u8) -> Self {
        Self {
            kind,
            angle: (-90. * f32::from(clockwise_turns % 4)).rem_euclid(360.),
        }
    }
}

/// Library orientation is a session view preference, never a project edit.
pub fn show(ui: &mut egui::Ui, enabled: bool) -> bool {
    let id = ui.id().with("palette-clockwise-turns");
    let mut turns = ui
        .ctx()
        .data_mut(|data| data.get_temp::<u8>(id).unwrap_or(0));
    let started = show_rotated(ui, enabled, &mut turns);
    ui.ctx().data_mut(|data| data.insert_temp(id, turns));
    started
}

fn show_rotated(ui: &mut egui::Ui, enabled: bool, turns: &mut u8) -> bool {
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
    if ui
        .button("Rotate 90° clockwise")
        .on_hover_text("Rotate all library icons clockwise. Newly placed ports use this angle.")
        .clicked()
    {
        *turns = (*turns % 4 + 1) % 4;
    }
    ui.label(
        egui::RichText::new(format!("Next drop: {}° clockwise", u16::from(*turns) * 90))
            .size(11.)
            .color(MUTED),
    );
    let angle = PortDrag::new(PortKind::UsbA, *turns).angle;
    let mut started = false;
    egui::ScrollArea::vertical().id_salt("port-palette-scroll")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .max_height(ui.available_height()).show(ui,|ui|{
        ui.add_enabled_ui(enabled,|ui|{
            let width=(ui.available_width()-ui.spacing().item_spacing.x)/2.;
            for (row,kinds) in PortKind::ALL.chunks(2).enumerate() {
                ui.horizontal(|ui|{
                    for (column,kind) in kinds.iter().copied().enumerate() {
                        ui.push_id(("palette",row,column),|ui|{
                            let (rect,response)=ui.allocate_exact_size(Vec2::new(width,54.),Sense::drag());
                            response.dnd_set_drag_payload(PortDrag {kind,angle});
                            started|=response.drag_started();
                            let fill=if response.hovered(){Color32::from_rgb(44,62,67)}else{Color32::from_rgb(31,43,52)};
                            ui.painter().rect_filled(rect,5.,fill);
                            let color=if ui.is_enabled(){ACCENT}else{MUTED};
                            icon(ui.painter(),kind,Rect::from_min_size(rect.min+Vec2::new(9.,13.),Vec2::new(40.,28.)),color,angle);
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
    started
}

pub fn icon(painter: &egui::Painter, kind: PortKind, rect: Rect, color: Color32, angle: f32) {
    let [w, h] = kind.dimensions();
    let (s, c) = angle.to_radians().sin_cos();
    let bounds = Vec2::new(w * c.abs() + h * s.abs(), w * s.abs() + h * c.abs());
    let scale = (rect.width() / bounds.x).min(rect.height() / bounds.y);
    let size = Vec2::new(w, h) * scale;
    let center = rect.center();
    let point = |p: [f32; 2]| {
        let x = p[0] * size.x;
        let y = p[1] * size.y;
        center + Vec2::new(x * c - y * s, -(x * s + y * c))
    };
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
    if let (Some(payload), Some(pos)) = (
        egui::DragAndDrop::payload::<PortDrag>(ctx),
        ctx.pointer_interact_pos(),
    ) {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("port-drag-indicator"),
        ));
        let label = painter.layout_no_wrap(
            format!(
                "{} · {:.0}° clockwise",
                payload.kind.name(),
                (-payload.angle).rem_euclid(360.)
            ),
            FontId::proportional(12.),
            ACCENT,
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        ctx: &egui::Context,
        turns: &mut u8,
        time: f64,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(360., 850.))),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show_rotated(ui, true, turns);
                });
            },
        )
    }
    fn text_position(out: &egui::FullOutput, text: &str) -> Pos2 {
        out.shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == text => {
                    Some(t.pos + t.galley.size() / 2.)
                }
                _ => None,
            })
            .unwrap()
    }
    fn press(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    #[test]
    fn rotation_button_is_above_icons_and_drag_payload_keeps_the_selected_angle() {
        let ctx = egui::Context::default();
        let mut turns = 0;
        let out = frame(&ctx, &mut turns, 0., vec![]);
        let button = text_position(&out, "Rotate 90° clockwise");
        let tile = text_position(&out, "USB-A");
        assert!(button.y < tile.y);
        frame(
            &ctx,
            &mut turns,
            0.1,
            vec![egui::Event::PointerMoved(button)],
        );
        frame(&ctx, &mut turns, 0.2, vec![press(button, true)]);
        let out = frame(&ctx, &mut turns, 0.3, vec![press(button, false)]);
        assert_eq!(turns, 1);
        let tile = text_position(&out, "USB-A");
        frame(&ctx, &mut turns, 0.4, vec![egui::Event::PointerMoved(tile)]);
        frame(&ctx, &mut turns, 0.5, vec![press(tile, true)]);
        frame(
            &ctx,
            &mut turns,
            0.6,
            vec![egui::Event::PointerMoved(tile - Vec2::new(80., 0.))],
        );
        let payload =
            egui::DragAndDrop::payload::<PortDrag>(&ctx).expect("rotated palette payload");
        assert_eq!(payload.kind, PortKind::UsbA);
        assert_eq!(payload.angle, 270.);
    }
    #[test]
    fn four_clockwise_clicks_restore_every_library_icon() {
        let ctx = egui::Context::default();
        let mut turns = 0;
        let out = frame(&ctx, &mut turns, 0., vec![]);
        let button = text_position(&out, "Rotate 90° clockwise");
        for n in 1..=4 {
            let time = f64::from(n);
            frame(
                &ctx,
                &mut turns,
                time,
                vec![egui::Event::PointerMoved(button), press(button, true)],
            );
            frame(&ctx, &mut turns, time + 0.1, vec![press(button, false)]);
            assert_eq!(turns, n as u8 % 4);
            for kind in PortKind::ALL {
                assert_eq!(
                    PortDrag::new(kind, turns).angle,
                    (-(n as f32) * 90.).rem_euclid(360.)
                );
            }
        }
    }
    #[test]
    fn asymmetric_icons_turn_clockwise_and_all_rotated_shapes_stay_within_their_tiles() {
        let ctx = egui::Context::default();
        for kind in PortKind::ALL {
            for turn in 0..4 {
                let rect = Rect::from_min_size(Pos2::new(20., 20.), Vec2::new(40., 28.));
                let out = ctx.run(Default::default(), |ctx| {
                    icon(
                        &ctx.debug_painter(),
                        kind,
                        rect,
                        ACCENT,
                        PortDrag::new(kind, turn).angle,
                    );
                });
                let points = out
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        Shape::Path(p) => Some(&p.points),
                        _ => None,
                    })
                    .unwrap();
                assert!(
                    points.iter().all(|p| rect.expand(0.01).contains(*p)),
                    "{kind:?}, turn {turn}"
                );
                if kind == PortKind::UsbB && turn == 1 {
                    assert!(
                        points[0].x < rect.center().x && points[0].y < rect.center().y,
                        "bottom-left must rotate to top-left"
                    );
                }
            }
        }
    }
}
