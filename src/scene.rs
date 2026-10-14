use cable_management::{geometry::*, model::*};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2};

pub const ACCENT: Color32 = Color32::from_rgb(99, 221, 180);
pub const MUTED: Color32 = Color32::from_rgb(142, 156, 168);
pub const ORANGE: Color32 = Color32::from_rgb(255, 193, 112);

pub struct ViewState {
    pub camera: Camera,
    pub target: Camera,
    pub zoom: f32,
    pub labels: bool,
    pub face: Face,
    pub pan: Vec2,
    drag_port: Option<Id>,
    node: Option<Id>,
}
impl Default for ViewState {
    fn default() -> Self {
        Self {
            camera: Camera::for_face(Face::Back),
            target: Camera::for_face(Face::Back),
            zoom: 1.,
            labels: true,
            face: Face::Back,
            pan: Vec2::ZERO,
            drag_port: None,
            node: None,
        }
    }
}
impl ViewState {
    pub fn select(&mut self, node: &Node) {
        if self.node != Some(node.id) {
            self.node = Some(node.id);
            self.camera = Camera::for_face(node.default_view);
            self.target = self.camera;
            self.face = node.default_view;
            self.zoom = 1.;
            self.pan = Vec2::ZERO;
            self.drag_port = None;
        }
        let (min, max) = node_bounds(node);
        self.camera.center = mul(add(min, max), 0.5);
        self.target.center = self.camera.center;
    }
    pub fn set_face(&mut self, face: Face) {
        self.face = face;
        self.target = Camera::for_face(face);
        self.target.center = self.camera.center;
    }
    pub fn reset(&mut self) {
        self.zoom = 1.;
        self.pan = Vec2::ZERO;
    }
}

#[derive(Default)]
pub struct SceneEvent {
    pub selected: Option<Id>,
    pub moved: Option<(Id, [f32; 2])>,
    pub drag_started: bool,
}

struct Polygon {
    socket: bool,
    points: Vec<Pos2>,
    depth: f32,
    fill: Color32,
    stroke: Stroke,
}
struct Socket {
    id: Id,
    center: Pos2,
    points: Vec<Pos2>,
    name: String,
    kind: String,
    connection: Option<String>,
    depth: f32,
}

fn inside(point: Pos2, polygon: &[Pos2]) -> bool {
    let mut sign = 0.;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let v = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);
        if v.abs() < 0.01 {
            continue;
        }
        if sign == 0. {
            sign = v.signum();
        } else if sign * v < 0. {
            return false;
        }
    }
    true
}

fn pick_socket(
    sockets: &[Socket],
    point: Pos2,
    selected: Option<Id>,
    radius: f32,
) -> Option<&Socket> {
    sockets
        .iter()
        .filter(|s| inside(point, &s.points) || s.center.distance(point) < radius)
        .min_by(|a, b| {
            (Some(a.id) != selected)
                .cmp(&(Some(b.id) != selected))
                .then_with(|| {
                    a.center
                        .distance(point)
                        .total_cmp(&b.center.distance(point))
                })
        })
}

pub fn draw(
    ui: &mut egui::Ui,
    project: &Project,
    node: &Node,
    selected: Option<Id>,
    editing: bool,
    state: &mut ViewState,
) -> SceneEvent {
    state.select(node);
    let size = ui.available_size().max(Vec2::splat(100.));
    let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
    let rect = response.rect;
    painter.rect_filled(rect, 0., Color32::from_rgb(18, 24, 31));
    let dt = ui.input(|i| i.stable_dt);
    if state.camera.animate(&state.target, dt) {
        ui.ctx().request_repaint();
    }
    let (min, max) = node_bounds(node);
    let radius = length(sub(max, min)).max(20.);
    let scale = (rect.width().min(rect.height()) * 0.78 / radius) * state.zoom;
    let center = rect.center() + state.pan;
    let to_screen = |point| {
        let (p, depth) = state.camera.project(point);
        (center + Vec2::new(p[0], p[1]) * scale, depth)
    };
    // Subtle screen-space grid stays readable at every physical scale.
    let spacing = 40.;
    let mut x = rect.left() + (state.pan.x % spacing);
    while x < rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(0.5_f32, Color32::from_rgb(29, 36, 44)),
        );
        x += spacing;
    }
    let mut y = rect.top() + (state.pan.y % spacing);
    while y < rect.bottom() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5_f32, Color32::from_rgb(29, 36, 44)),
        );
        y += spacing;
    }

    let mut polygons = vec![];
    for (index, part) in node.parts.iter().enumerate() {
        for face in Face::ALL {
            let f = face_frame(part, face, [0.; 2]);
            let facing = dot(f.normal, state.camera.direction());
            if facing <= 0.001 {
                continue;
            }
            let projected: Vec<_> = face_vertices(part, face)
                .into_iter()
                .map(to_screen)
                .collect();
            let depth = projected.iter().map(|p| p.1).sum::<f32>() / projected.len() as f32;
            let shade = (47. + facing * 27. + (index % 3) as f32 * 9.) as u8;
            polygons.push(Polygon {
                socket: false,
                points: projected.into_iter().map(|p| p.0).collect(),
                depth,
                fill: Color32::from_rgb(shade, shade + 8, shade + 17),
                stroke: Stroke::new(1.3_f32, Color32::from_rgb(110, 126, 140)),
            });
        }
    }
    let mut sockets = vec![];
    for port in &node.ports {
        if !port_visible(node, port, &state.camera) {
            continue;
        }
        let Some(part) = node.parts.iter().find(|p| p.id == port.part) else {
            continue;
        };
        let frame = face_frame(part, port.face, port.uv);
        let (position, depth) = to_screen(frame.center);
        let points: Vec<_> = socket_points(port, &frame)
            .into_iter()
            .map(|p| to_screen(p).0)
            .collect();
        let connection = project
            .connection_at(PortRef {
                node: node.id,
                port: port.id,
            })
            .map(|c| {
                project.target_label(
                    c,
                    PortRef {
                        node: node.id,
                        port: port.id,
                    },
                )
            });
        polygons.push(Polygon {
            socket: true,
            points: points.clone(),
            depth: depth + 0.15,
            fill: Color32::from_rgb(11, 15, 20),
            stroke: Stroke::new(
                if selected == Some(port.id) {
                    2.2
                } else {
                    1.4_f32
                },
                if selected == Some(port.id) {
                    ORANGE
                } else if connection.is_some() {
                    ACCENT
                } else {
                    Color32::from_rgb(173, 185, 196)
                },
            ),
        });
        sockets.push(Socket {
            id: port.id,
            center: position,
            points,
            name: port.name.clone(),
            kind: port.kind.name().into(),
            connection,
            depth,
        });
    }
    // Visibility is resolved against opaque parts before socket creation. Draw
    // these visible surface details after the chassis so face-average depth
    // cannot hide a socket on the farther half of its own supporting polygon.
    polygons.sort_by(|a, b| a.socket.cmp(&b.socket).then(a.depth.total_cmp(&b.depth)));
    for p in polygons {
        painter.add(Shape::convex_polygon(p.points, p.fill, p.stroke));
    }
    sockets.sort_by(|a, b| b.depth.total_cmp(&a.depth));
    let pointer = response
        .hover_pos()
        .or_else(|| ui.input(|i| i.pointer.interact_pos()));
    let hovered = pointer
        .filter(|p| rect.contains(*p))
        .and_then(|p| pick_socket(&sockets, p, selected, 9.));
    let mut event = SceneEvent::default();
    if response.clicked()
        && let Some(socket) = hovered
    {
        event.selected = Some(socket.id);
    }
    if response.drag_started_by(egui::PointerButton::Primary) && editing {
        // egui reports drag-start after threshold: use the original press point.
        let press = ui.input(|i| i.pointer.press_origin());
        let hit = press.and_then(|p| pick_socket(&sockets, p, selected, 12.));
        if let Some(socket) = hit {
            state.drag_port = Some(socket.id);
            event.selected = Some(socket.id);
            event.drag_started = true;
        }
    }
    if let Some(id) = state.drag_port {
        if response.dragged_by(egui::PointerButton::Primary)
            && let (Some(position), Some(port)) = (pointer, node.ports.iter().find(|p| p.id == id))
            && let Some(part) = node.parts.iter().find(|p| p.id == port.part)
        {
            let p = (position - center) / scale;
            if let Some(uv) = face_uv_at(part, port.face, &state.camera, [p.x, p.y]) {
                let mut copy = port.clone();
                copy.uv = uv;
                copy.clamp_to_part(part);
                event.moved = Some((id, copy.uv));
            }
        }
        if !ui.input(|i| i.pointer.primary_down()) {
            state.drag_port = None;
        }
    } else if response.dragged_by(egui::PointerButton::Primary)
        || response.dragged_by(egui::PointerButton::Secondary)
    {
        let delta = ui.input(|i| i.pointer.delta());
        state.camera.yaw -= delta.x * 0.008;
        state.camera.pitch = (state.camera.pitch + delta.y * 0.008).clamp(-1.56, 1.56);
        state.target = state.camera;
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        state.pan += ui.input(|i| i.pointer.delta());
    }
    if response.hovered() {
        let wheel = ui.input(|i| i.smooth_scroll_delta.y);
        if wheel != 0. {
            state.zoom = (state.zoom * (wheel * 0.0025).exp()).clamp(0.2, 18.);
        }
    }

    // Labels use leader lines and vertical separation without enlarging physical sockets.
    let mut labels: Vec<_> = sockets
        .iter()
        .filter(|s| state.labels || selected == Some(s.id) || hovered.is_some_and(|h| h.id == s.id))
        .collect();
    labels.sort_by(|a, b| a.center.y.total_cmp(&b.center.y));
    let mut last_y = rect.top() + 35.;
    for s in labels {
        let text = if let Some(c) = &s.connection {
            format!("{}  >  {}", s.name, c)
        } else {
            s.name.clone()
        };
        let galley = painter.layout_no_wrap(
            text,
            FontId::proportional(12.),
            if selected == Some(s.id) {
                ORANGE
            } else if s.connection.is_some() {
                ACCENT
            } else {
                Color32::from_rgb(208, 217, 225)
            },
        );
        let label_x = (s.center.x + 24.)
            .min(rect.right() - galley.size().x - 15.)
            .max(rect.left() + 12.);
        let label_y = (s.center.y - 8.).max(last_y).min(rect.bottom() - 50.);
        last_y = label_y + 23.;
        let pos = Pos2::new(label_x, label_y);
        let bg = Rect::from_min_size(pos - Vec2::new(5., 3.), galley.size() + Vec2::new(10., 6.));
        painter.line_segment(
            [s.center, Pos2::new(bg.left(), bg.center().y)],
            Stroke::new(0.9_f32, Color32::from_rgb(125, 151, 155)),
        );
        painter.rect_filled(bg, 3., Color32::from_rgba_unmultiplied(15, 22, 29, 240));
        painter.galley(pos, galley, Color32::WHITE);
        if selected == Some(s.id) {
            painter.circle_stroke(s.center, 12., Stroke::new(1.5_f32, ORANGE));
        }
    }
    if let Some(socket) = hovered {
        response.clone().on_hover_ui_at_pointer(|ui| {
            ui.strong(&socket.name);
            ui.label(&socket.kind);
            ui.label(socket.connection.as_deref().unwrap_or("Ledig port"));
            if editing {
                ui.small("Dra porten för att flytta den på ytan.");
            }
        });
        ui.ctx().set_cursor_icon(if editing {
            egui::CursorIcon::Grab
        } else {
            egui::CursorIcon::PointingHand
        });
    }
    // Millimetre scale and orientation triad.
    let mm = if scale > 4. {
        10.
    } else if scale > 1. {
        50.
    } else {
        100.
    };
    let left = rect.left_bottom() + Vec2::new(24., -30.);
    painter.line_segment(
        [left, left + Vec2::new(mm * scale, 0.)],
        Stroke::new(2_f32, MUTED),
    );
    painter.text(
        left + Vec2::new(0., 7.),
        Align2::LEFT_TOP,
        format!("{mm:.0} mm"),
        FontId::proportional(11.),
        MUTED,
    );
    let origin = rect.right_bottom() + Vec2::new(-57., -57.);
    for (axis, label, color) in [
        ([1., 0., 0.], "X", Color32::from_rgb(230, 116, 112)),
        ([0., 1., 0.], "Y", ACCENT),
        ([0., 0., 1.], "Z", Color32::from_rgb(122, 167, 242)),
    ] {
        let end = origin
            + Vec2::new(
                dot(axis, state.camera.right()),
                -dot(axis, state.camera.up()),
            ) * 29.;
        painter.line_segment([origin, end], Stroke::new(1.8_f32, color));
        painter.text(
            end,
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(10.),
            color,
        );
    }
    painter.text(
        rect.left_top() + Vec2::new(20., 18.),
        Align2::LEFT_TOP,
        format!("{}  /  {} portar synliga", state.face.name(), sockets.len()),
        FontId::proportional(11.),
        MUTED,
    );
    event
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_port_can_be_dragged_when_new_ports_overlap() {
        let sockets: Vec<_> = (1..=2)
            .map(|id| Socket {
                id,
                center: Pos2::ZERO,
                points: vec![
                    Pos2::new(-5., -2.),
                    Pos2::new(5., -2.),
                    Pos2::new(5., 2.),
                    Pos2::new(-5., 2.),
                ],
                name: String::new(),
                kind: String::new(),
                connection: None,
                depth: 0.,
            })
            .collect();
        assert_eq!(
            pick_socket(&sockets, Pos2::ZERO, Some(2), 12.).unwrap().id,
            2
        );
    }
    #[test]
    fn oblique_socket_is_not_painted_under_its_supporting_face() {
        let p = Project::demo();
        let n = &p.nodes[0];
        let mut state = ViewState::default();
        state.select(n);
        state.camera.yaw += 0.5;
        state.target = state.camera;
        state.labels = false;
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900., 600.))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw(ui, &p, n, None, false, &mut state);
                });
            },
        );
        let socket_color = Color32::from_rgb(11, 15, 20);
        let socket = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                Shape::Path(path) if path.fill == socket_color => Some(path),
                _ => None,
            })
            .unwrap();
        let center = socket
            .points
            .iter()
            .fold(Vec2::ZERO, |sum, p| sum + p.to_vec2())
            / socket.points.len() as f32;
        let center = Pos2::new(center.x, center.y);
        let last = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                Shape::Path(path)
                    if path.closed
                        && inside(center, &path.points)
                        && path.fill != Color32::TRANSPARENT =>
                {
                    Some(path.fill)
                }
                _ => None,
            })
            .next_back();
        assert_eq!(last, Some(socket_color));
    }
}
