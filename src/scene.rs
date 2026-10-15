use cable_management::ports::PortKind;
use cable_management::{geometry::*, model::*};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2};

pub const ACCENT: Color32 = Color32::from_rgb(99, 221, 180);
pub const MUTED: Color32 = Color32::from_rgb(142, 156, 168);
pub const ORANGE: Color32 = Color32::from_rgb(255, 193, 112);

struct ResizeDrag {
    part: Part,
    handle: [i8; 2],
    origin: Pos2,
    scale: f32,
    camera: Camera,
}
#[derive(Clone, Copy)]
pub struct NodeDrag {
    pub node: Id,
}

#[derive(Clone, Copy)]
struct LabelDrag {
    id: Id,
    offset: [f32; 2],
    origin: Pos2,
    scale: f32,
}

#[derive(Clone, Copy)]
struct SideSwipe {
    origin: Pos2,
    face: Face,
    changed: bool,
}

pub struct ViewState {
    pub camera: Camera,
    pub target: Camera,
    pub zoom: f32,
    pub labels: bool,
    pub face: Face,
    pub pan: Vec2,
    drag_port: Option<Id>,
    resize: Option<ResizeDrag>,
    label_drag: Option<LabelDrag>,
    swipe: Option<SideSwipe>,
    node: Option<Id>,
    context_port: Option<Id>,
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
            resize: None,
            label_drag: None,
            swipe: None,
            node: None,
            context_port: None,
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
            self.resize = None;
            self.label_drag = None;
            self.swipe = None;
            self.context_port = None;
        }
        let (min, max) = node_bounds(node);
        if self.resize.is_none() {
            self.camera.center = mul(add(min, max), 0.5);
        }
        self.target.center = self.camera.center;
    }
    pub fn cancel_gestures(&mut self) {
        self.drag_port = None;
        self.resize = None;
        self.label_drag = None;
        self.swipe = None;
    }
    pub fn set_face(&mut self, face: Face) {
        self.cancel_gestures();
        self.face = face;
        self.target = Camera::for_face(face);
        self.target.center = self.camera.center;
    }
    pub fn reset(&mut self) {
        self.cancel_gestures();
        self.zoom = 1.;
        self.pan = Vec2::ZERO;
    }
}

#[derive(Default)]
pub struct SceneEvent {
    pub selected: Option<Id>,
    pub moved: Option<(Id, [f32; 2])>,
    pub drag_started: bool,
    pub dropped: Option<PortDrop>,
    pub delete: Option<Id>,
    pub edit_label: Option<Id>,
    pub drop_rejected: bool,
    pub rotate: Option<Id>,
    pub connect: Option<Id>,
    pub disconnect: Option<Id>,
    pub label_moved: Option<(Id, [f32; 2])>,
    pub resized: Option<(Id, [f32; 3], [f32; 3])>,
    pub hovered_node: Option<Id>,
    pub node_dropped: Option<(Id, Id)>,
}
#[derive(Clone, Copy, Debug)]
pub struct PortDrop {
    pub kind: PortKind,
    pub hit: SurfaceHit,
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
            // A real socket hit always wins over a neighbour's enlarged click
            // tolerance. Selected preference only disambiguates real overlaps.
            let a_inside = inside(point, &a.points);
            let b_inside = inside(point, &b.points);
            b_inside
                .cmp(&a_inside)
                .then_with(|| {
                    if a_inside && b_inside {
                        (Some(a.id) != selected).cmp(&(Some(b.id) != selected))
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
                .then_with(|| {
                    a.center
                        .distance(point)
                        .total_cmp(&b.center.distance(point))
                })
        })
}

fn port_menu(response: &egui::Response, port: Id, event: &mut SceneEvent) {
    response.context_menu(|ui| {
        if ui.button("Edit connection / Open device…").clicked() {
            event.edit_label = Some(port);
            ui.close();
        }
        if ui.button("Rotate…").clicked() {
            event.rotate = Some(port);
            ui.close();
        }
        if ui.button("Connect to device…").clicked() {
            event.connect = Some(port);
            ui.close();
        }
        if ui.button("Disconnect").clicked() {
            event.disconnect = Some(port);
            ui.close();
        }
        ui.separator();
        if ui.button("Delete").clicked() {
            event.delete = Some(port);
            ui.close();
        }
    });
}

fn drop_preview(node: &Node, kind: PortKind, hit: SurfaceHit, camera: &Camera) -> Option<Port> {
    let part = node.parts.iter().find(|p| p.id == hit.part)?;
    let mut port = Port {
        id: 0,
        name: kind.name().into(),
        kind,
        part: hit.part,
        face: hit.face,
        uv: hit.uv,
        angle: 0.,
        metadata: String::new(),
        label_offset: None,
    };
    port.clamp_to_part(part);
    (port.fits_on(part) && port_visible(node, &port, camera)).then_some(port)
}

/// Pick an adjacent named face from screen direction; never derive a free angle.
fn swiped_face(face: Face, delta: Vec2) -> Option<Face> {
    if !delta.is_finite() || delta.x.abs().max(delta.y.abs()) < 35. {
        return None;
    }
    let camera = Camera::for_face(face);
    let direction = if delta.x.abs() >= delta.y.abs() {
        mul(camera.right(), -delta.x.signum())
    } else {
        mul(camera.up(), delta.y.signum())
    };
    Face::ALL.into_iter().max_by(|a, b| {
        dot(Camera::for_face(*a).direction(), direction)
            .total_cmp(&dot(Camera::for_face(*b).direction(), direction))
    })
}

fn resize_cursor(handle: [i8; 2]) -> egui::CursorIcon {
    match handle {
        [0, _] => egui::CursorIcon::ResizeVertical,
        [_, 0] => egui::CursorIcon::ResizeHorizontal,
        [x, y] if x == y => egui::CursorIcon::ResizeNwSe,
        _ => egui::CursorIcon::ResizeNeSw,
    }
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
    let scale = state
        .resize
        .as_ref()
        .map(|d| d.scale)
        .unwrap_or((rect.width().min(rect.height()) * 0.78 / radius) * state.zoom);
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
    let (socket_polygons, bodies): (Vec<_>, Vec<_>) = polygons.into_iter().partition(|p| p.socket);
    for p in bodies {
        painter.add(Shape::convex_polygon(p.points, p.fill, p.stroke));
    }
    sockets.sort_by(|a, b| b.depth.total_cmp(&a.depth));
    let pointer = response
        .hover_pos()
        .or_else(|| ui.input(|i| i.pointer.interact_pos()));
    let hovered = pointer
        .filter(|p| rect.contains(*p))
        .and_then(|p| pick_socket(&sockets, p, None, 9.));
    let mut event = SceneEvent {
        hovered_node: hovered
            .and_then(|s| {
                project.connected_peer(PortRef {
                    node: node.id,
                    port: s.id,
                })
            })
            .map(|p| p.node),
        ..Default::default()
    };
    let settled = dot(state.camera.direction(), state.target.direction()) > 0.9999;
    let mut resize_hit = None;
    if editing && settled {
        let mut bodies: Vec<_> = node.parts.iter().collect();
        bodies.sort_by(|a, b| {
            state
                .camera
                .project(a.offset)
                .1
                .total_cmp(&state.camera.project(b.offset).1)
        });
        for part in bodies {
            let bounds = Rect::from_points(
                &Face::ALL
                    .into_iter()
                    .flat_map(|f| face_vertices(part, f))
                    .map(|p| to_screen(p).0)
                    .collect::<Vec<_>>(),
            );
            for handle in [
                [-1_i8, -1],
                [0, -1],
                [1, -1],
                [-1, 0],
                [1, 0],
                [-1, 1],
                [0, 1],
                [1, 1],
            ] {
                let pos = bounds.center()
                    + bounds.size() * Vec2::new(f32::from(handle[0]), f32::from(handle[1])) / 2.;
                painter.rect_filled(Rect::from_center_size(pos, Vec2::splat(7.)), 1., ACCENT);
            }
            if let Some(point) = ui
                .input(|i| {
                    if i.pointer.primary_down() {
                        i.pointer.press_origin()
                    } else {
                        pointer
                    }
                })
                .filter(|p| bounds.expand(7.).contains(*p))
            {
                let dx = if (point.x - bounds.left()).abs() < 7. {
                    -1
                } else if (point.x - bounds.right()).abs() < 7. {
                    1
                } else {
                    0
                };
                let dy = if (point.y - bounds.top()).abs() < 7. {
                    -1
                } else if (point.y - bounds.bottom()).abs() < 7. {
                    1
                } else {
                    0
                };
                if dx != 0 || dy != 0 {
                    resize_hit = Some((part, [dx, dy]));
                }
            }
        }
    }
    if let Some((_, h)) = resize_hit.filter(|_| hovered.is_none()) {
        ui.ctx().set_cursor_icon(resize_cursor(h));
    }
    // Remember an eligible press before egui's drag threshold. A quick swipe
    // can deliver its movement and release together in the following frame.
    if response.is_pointer_button_down_on()
        && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
        && resize_hit.is_none()
        && let Some(origin) = ui
            .input(|i| i.pointer.press_origin())
            .filter(|p| rect.contains(*p))
        && pick_socket(&sockets, origin, None, 12.).is_none()
    {
        let at = (origin - center) / scale;
        if surface_at(node, &state.camera, [at.x, at.y]).is_some() {
            state.swipe = Some(SideSwipe {
                origin,
                face: state.face,
                changed: false,
            });
        }
    }
    if response.clicked()
        && let Some(socket) = hovered
    {
        event.selected = Some(socket.id);
    }
    if response.double_clicked_by(egui::PointerButton::Primary)
        && let Some(socket) = hovered
    {
        event.edit_label = Some(socket.id);
    }
    if response.secondary_clicked() {
        state.context_port = hovered.map(|s| s.id);
    }
    if let Some(port) = state
        .context_port
        .filter(|id| node.ports.iter().any(|p| p.id == *id))
    {
        port_menu(&response, port, &mut event);
    }
    if let Some(payload) = response.dnd_hover_payload::<NodeDrag>()
        && let Some(socket) = hovered
    {
        painter.circle_stroke(socket.center, 15., Stroke::new(2_f32, ACCENT));
        if response.dnd_release_payload::<NodeDrag>().is_some() {
            event.node_dropped = Some((socket.id, payload.node));
        }
    }
    if editing && let Some(kind) = response.dnd_hover_payload::<PortKind>() {
        let candidate = pointer.and_then(|point| {
            let at = (point - center) / scale;
            let hit = surface_at(node, &state.camera, [at.x, at.y])?;
            Some((hit, drop_preview(node, *kind, hit, &state.camera)?))
        });
        if let Some((hit, preview)) = &candidate {
            let part = node.parts.iter().find(|p| p.id == hit.part).unwrap();
            let frame = face_frame(part, hit.face, preview.uv);
            let points = socket_points(preview, &frame)
                .into_iter()
                .map(|p| to_screen(p).0)
                .collect();
            painter.add(Shape::convex_polygon(
                points,
                Color32::from_rgba_unmultiplied(99, 221, 180, 50),
                Stroke::new(2_f32, ACCENT),
            ));
            painter.add(Shape::closed_line(
                face_vertices(part, hit.face)
                    .into_iter()
                    .map(|p| to_screen(p).0)
                    .collect(),
                Stroke::new(1.5_f32, ACCENT),
            ));
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        } else {
            ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
        }
        if response.dnd_release_payload::<PortKind>().is_some() {
            if let Some((mut hit, preview)) = candidate {
                hit.uv = preview.uv;
                event.dropped = Some(PortDrop { kind: *kind, hit });
            } else {
                event.drop_rejected = true;
            }
        }
    }
    if response.drag_started_by(egui::PointerButton::Primary) {
        // egui reports drag-start after threshold: use the original press point.
        let press = ui.input(|i| i.pointer.press_origin());
        let hit = press.and_then(|p| pick_socket(&sockets, p, selected, 12.));
        if editing
            && settled
            && let Some(socket) = hit
        {
            state.drag_port = Some(socket.id);
            event.selected = Some(socket.id);
            event.drag_started = true;
        } else if editing
            && settled
            && let (Some((part, handle)), Some(origin)) = (resize_hit, press)
        {
            state.resize = Some(ResizeDrag {
                part: part.clone(),
                handle,
                origin,
                scale,
                camera: state.camera,
            });
            event.drag_started = true;
        }
    }
    if let Some(drag) = &state.resize {
        if let Some(point) = pointer {
            let delta = (point - drag.origin) / drag.scale;
            let next =
                crate::resize::resized(&drag.part, &drag.camera, drag.handle, [delta.x, delta.y]);
            event.resized = Some((next.id, next.size, next.offset));
            ui.ctx().set_cursor_icon(resize_cursor(drag.handle));
        }
        if !ui.input(|i| i.pointer.primary_down()) {
            state.resize = None;
        }
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        state.pan += ui.input(|i| i.pointer.delta());
    }
    if response.hovered()
        && state.resize.is_none()
        && state.label_drag.is_none()
        && state.swipe.is_none()
    {
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
        let text = project.port_label(PortRef {
            node: node.id,
            port: s.id,
        });
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
        let mut label_y = (s.center.y - 8.).max(last_y).min(rect.bottom() - 50.);
        for _ in 0..sockets.len() {
            let bounds = Rect::from_min_size(
                Pos2::new(label_x - 5., label_y - 3.),
                galley.size() + Vec2::new(10., 6.),
            );
            if let Some(bottom) = sockets
                .iter()
                .map(|s| Rect::from_points(&s.points).expand(3.))
                .filter(|r| r.intersects(bounds))
                .map(|r| r.bottom())
                .max_by(f32::total_cmp)
            {
                label_y = bottom + 7.;
            } else {
                break;
            }
        }
        last_y = label_y + 23.;
        let default_offset = [
            (label_x - s.center.x) / scale,
            (label_y - s.center.y) / scale,
        ];
        let offset = node
            .ports
            .iter()
            .find(|p| p.id == s.id)
            .and_then(|p| p.label_offset)
            .unwrap_or(default_offset);
        let mut pos = s.center + Vec2::new(offset[0], offset[1]) * scale;
        let initial_bg =
            Rect::from_min_size(pos - Vec2::new(5., 3.), galley.size() + Vec2::new(10., 6.));
        let label_response = ui.interact(
            initial_bg,
            ui.id().with(("port-label", s.id)),
            Sense::click_and_drag(),
        );
        if label_response.drag_started_by(egui::PointerButton::Primary) {
            let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(pos);
            if let Some(socket) = pick_socket(&sockets, origin, selected, 0.) {
                if editing && settled {
                    state.drag_port = Some(socket.id);
                    event.drag_started = true;
                    event.selected = Some(socket.id);
                }
            } else {
                state.label_drag = Some(LabelDrag {
                    id: s.id,
                    offset,
                    origin,
                    scale,
                });
                event.drag_started = true;
                event.selected = Some(s.id);
            }
        }
        if let Some(drag) = state.label_drag.filter(|d| d.id == s.id) {
            let delta = (ui
                .input(|i| i.pointer.interact_pos())
                .unwrap_or(drag.origin)
                - drag.origin)
                / drag.scale;
            let moved = [drag.offset[0] + delta.x, drag.offset[1] + delta.y];
            {
                event.label_moved = Some((drag.id, moved));
                pos = s.center + Vec2::new(moved[0], moved[1]) * scale;
            }
        }
        let bg = Rect::from_min_size(pos - Vec2::new(5., 3.), galley.size() + Vec2::new(10., 6.));
        painter.line_segment(
            [s.center, bg.clamp(s.center)],
            Stroke::new(0.9_f32, Color32::from_rgb(125, 151, 155)),
        );
        painter.rect_filled(bg, 3., Color32::from_rgba_unmultiplied(15, 22, 29, 240));
        painter.galley(pos, galley, Color32::WHITE);
        let hit = pointer
            .and_then(|point| pick_socket(&sockets, point, None, 0.))
            .map(|p| p.id)
            .unwrap_or(s.id);
        if label_response.clicked() {
            event.selected = Some(hit);
        }
        if label_response.double_clicked() {
            event.edit_label = Some(hit);
        }
        if label_response.hovered() || label_response.dragged() {
            event.hovered_node = project
                .connected_peer(PortRef {
                    node: node.id,
                    port: hit,
                })
                .map(|p| p.node);
            ui.ctx().set_cursor_icon(if label_response.dragged() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        if label_response.secondary_clicked() {
            state.context_port = Some(hit);
        }
        port_menu(
            &label_response,
            state.context_port.unwrap_or(s.id),
            &mut event,
        );
        if selected == Some(s.id) {
            painter.circle_stroke(s.center, 12., Stroke::new(1.5_f32, ORANGE));
        }
    }
    if let Some(id) = state.drag_port {
        if let (Some(position), Some(port)) = (pointer, node.ports.iter().find(|p| p.id == id))
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
    }
    // Physical sockets remain visible and interactive above annotation boxes.
    for p in socket_polygons {
        painter.add(Shape::convex_polygon(p.points, p.fill, p.stroke));
    }
    if !ui.input(|i| i.pointer.primary_down()) {
        state.label_drag = None;
    }
    if let Some(socket) = hovered {
        response.clone().on_hover_ui_at_pointer(|ui| {
            ui.strong(&socket.name);
            ui.label(&socket.kind);
            ui.label(socket.connection.as_deref().unwrap_or("not connected"));
            if editing {
                ui.small(
                    "Drag to move · double-click to edit or open device · right-click for actions.",
                );
            }
        });
        ui.ctx().set_cursor_icon(if editing {
            egui::CursorIcon::Grab
        } else {
            egui::CursorIcon::PointingHand
        });
    }
    if let Some(mut swipe) = state.swipe {
        if !swipe.changed
            && let Some(next) =
                pointer.and_then(|point| swiped_face(swipe.face, point - swipe.origin))
        {
            state.set_face(next);
            swipe.changed = true;
            state.swipe = Some(swipe);
            ui.ctx().request_repaint();
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        if !ui.input(|i| i.pointer.primary_down()) {
            state.swipe = None;
        }
    } else if hovered.is_none()
        && resize_hit.is_none()
        && response.hovered()
        && let Some(point) = pointer
    {
        let at = (point - center) / scale;
        if surface_at(node, &state.camera, [at.x, at.y]).is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
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
        format!("{}  /  {} ports visible", state.face.name(), sockets.len()),
        FontId::proportional(11.),
        MUTED,
    );
    event
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_socket_wins_over_a_neighbours_label_for_click_double_click_and_drag() {
        let mut p = Project::default();
        let node = p.add_node("PC", [200., 300., 200.]);
        let a = p.add_port(node, PortKind::UsbA, "A").unwrap();
        let b = p.add_port(node, PortKind::UsbA, "B").unwrap();
        p.port_mut(PortRef { node, port: b }).unwrap().uv = [0.22, -0.01];
        // Put A's label directly over B, as a user is allowed to do.
        p.port_mut(PortRef { node, port: a }).unwrap().label_offset = Some([24., -4.]);
        let ctx = egui::Context::default();
        let mut state = ViewState::default();
        let (_, out) = rendered(&ctx, &p, &mut state, 0., vec![]);
        let target = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                Shape::Path(path) if path.fill == Color32::from_rgb(11, 15, 20) => {
                    Some(Rect::from_points(&path.points).center())
                }
                _ => None,
            })
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap();
        rendered(
            &ctx,
            &p,
            &mut state,
            0.1,
            vec![egui::Event::PointerMoved(target)],
        );
        rendered(&ctx, &p, &mut state, 0.2, vec![press(target, true)]);
        let (e, _) = rendered(&ctx, &p, &mut state, 0.25, vec![press(target, false)]);
        assert_eq!(e.selected, Some(b));
        rendered(&ctx, &p, &mut state, 0.3, vec![press(target, true)]);
        let (e, _) = rendered(&ctx, &p, &mut state, 0.35, vec![press(target, false)]);
        assert_eq!(e.edit_label, Some(b));
        rendered(&ctx, &p, &mut state, 0.8, vec![press(target, true)]);
        let (e, _) = rendered(
            &ctx,
            &p,
            &mut state,
            0.9,
            vec![egui::Event::PointerMoved(target + Vec2::new(15., 25.))],
        );
        assert_eq!(e.selected, Some(b));
        assert_eq!(e.moved.map(|p| p.0), Some(b));
        assert!(e.label_moved.is_none());
    }
    #[test]
    fn device_swipes_choose_adjacent_fixed_views_in_view_and_edit_modes() {
        let directions = [
            Vec2::new(-80., 0.),
            Vec2::new(80., 0.),
            Vec2::new(0., -80.),
            Vec2::new(0., 80.),
        ];
        for editing in [false, true] {
            for (face, expected) in [
                (
                    Face::Front,
                    [Face::Right, Face::Left, Face::Bottom, Face::Top],
                ),
                (
                    Face::Back,
                    [Face::Left, Face::Right, Face::Bottom, Face::Top],
                ),
                (
                    Face::Left,
                    [Face::Front, Face::Back, Face::Bottom, Face::Top],
                ),
                (
                    Face::Right,
                    [Face::Back, Face::Front, Face::Bottom, Face::Top],
                ),
                (
                    Face::Top,
                    [Face::Right, Face::Left, Face::Front, Face::Back],
                ),
                (
                    Face::Bottom,
                    [Face::Right, Face::Left, Face::Back, Face::Front],
                ),
            ] {
                for (delta, expected) in directions.into_iter().zip(expected) {
                    let mut p = Project::default();
                    let node = p.add_node("Device", [200.; 3]);
                    p.node_mut(node).unwrap().default_view = face;
                    let ctx = egui::Context::default();
                    let mut state = ViewState::default();
                    let start = Pos2::new(400., 300.);
                    rendered_mode(
                        &ctx,
                        &p,
                        &mut state,
                        editing,
                        0.,
                        vec![egui::Event::PointerMoved(start)],
                    );
                    rendered_mode(&ctx, &p, &mut state, editing, 0.1, vec![press(start, true)]);
                    let (event, _) = rendered_mode(
                        &ctx,
                        &p,
                        &mut state,
                        editing,
                        0.2,
                        vec![egui::Event::PointerMoved(start + delta)],
                    );
                    assert_eq!(
                        state.face, expected,
                        "{face:?} + {delta:?}, editing={editing}"
                    );
                    let target = Camera::for_face(expected);
                    assert_eq!(
                        (state.target.yaw, state.target.pitch),
                        (target.yaw, target.pitch)
                    );
                    assert!(!event.drag_started);
                    assert!(event.moved.is_none());
                    assert!(event.resized.is_none());
                    assert!(event.label_moved.is_none());
                }
            }
        }
    }
    #[test]
    fn long_diagonal_swipe_takes_one_step_and_finishes_at_the_exact_fixed_angle() {
        let mut p = Project::default();
        p.add_node("Device", [200.; 3]);
        let ctx = egui::Context::default();
        let mut state = ViewState::default();
        let start = Pos2::new(400., 300.);
        rendered(
            &ctx,
            &p,
            &mut state,
            0.,
            vec![egui::Event::PointerMoved(start)],
        );
        rendered(&ctx, &p, &mut state, 0.1, vec![press(start, true)]);
        rendered(
            &ctx,
            &p,
            &mut state,
            0.2,
            vec![egui::Event::PointerMoved(start + Vec2::new(-80., 50.))],
        );
        assert_eq!(state.face, Face::Left);
        // Keep holding, extend the drag, then reverse: no further camera steps.
        rendered(
            &ctx,
            &p,
            &mut state,
            0.3,
            vec![egui::Event::PointerMoved(start + Vec2::new(-350., 150.))],
        );
        rendered(
            &ctx,
            &p,
            &mut state,
            0.4,
            vec![egui::Event::PointerMoved(start)],
        );
        assert_eq!(state.face, Face::Left);
        rendered(&ctx, &p, &mut state, 0.5, vec![press(start, false)]);
        for i in 1..=120 {
            rendered(&ctx, &p, &mut state, 0.5 + f64::from(i) / 60., vec![]);
        }
        let fixed = Camera::for_face(Face::Left);
        assert_eq!(
            (state.camera.yaw, state.camera.pitch),
            (fixed.yaw, fixed.pitch)
        );
        // A new physical stroke can advance another fixed step.
        rendered(&ctx, &p, &mut state, 3., vec![press(start, true)]);
        rendered(
            &ctx,
            &p,
            &mut state,
            3.1,
            vec![egui::Event::PointerMoved(start + Vec2::new(-80., 0.))],
        );
        assert_eq!(state.face, Face::Front);
    }
    #[test]
    fn fast_swipe_uses_final_release_position_when_move_and_release_share_a_frame() {
        let mut p = Project::default();
        p.add_node("Device", [200.; 3]);
        let ctx = egui::Context::default();
        let mut state = ViewState::default();
        let start = Pos2::new(400., 300.);
        rendered(
            &ctx,
            &p,
            &mut state,
            0.,
            vec![egui::Event::PointerMoved(start)],
        );
        rendered(&ctx, &p, &mut state, 0.1, vec![press(start, true)]);
        let end = start + Vec2::new(-80., 0.);
        let (e, _) = rendered(
            &ctx,
            &p,
            &mut state,
            0.12,
            vec![egui::Event::PointerMoved(end), press(end, false)],
        );
        assert_eq!(state.face, Face::Left);
        assert!(!e.drag_started);
        assert!(state.swipe.is_none());
    }
    #[test]
    fn small_body_drags_and_drags_starting_off_the_device_do_not_switch_sides() {
        let mut p = Project::default();
        p.add_node("Device", [200.; 3]);
        let ctx = egui::Context::default();
        let mut state = ViewState::default();
        let start = Pos2::new(400., 300.);
        rendered(
            &ctx,
            &p,
            &mut state,
            0.,
            vec![egui::Event::PointerMoved(start)],
        );
        rendered(&ctx, &p, &mut state, 0.1, vec![press(start, true)]);
        rendered(
            &ctx,
            &p,
            &mut state,
            0.2,
            vec![egui::Event::PointerMoved(start + Vec2::new(20., -8.))],
        );
        rendered(&ctx, &p, &mut state, 0.3, vec![press(start, false)]);
        assert_eq!(state.face, Face::Back);
        let outside = Pos2::new(60., 60.);
        rendered(
            &ctx,
            &p,
            &mut state,
            0.4,
            vec![egui::Event::PointerMoved(outside)],
        );
        rendered(&ctx, &p, &mut state, 0.5, vec![press(outside, true)]);
        rendered(
            &ctx,
            &p,
            &mut state,
            0.6,
            vec![egui::Event::PointerMoved(start)],
        );
        assert_eq!(state.face, Face::Back);
    }
    fn rendered(
        ctx: &egui::Context,
        p: &Project,
        state: &mut ViewState,
        time: f64,
        events: Vec<egui::Event>,
    ) -> (SceneEvent, egui::FullOutput) {
        rendered_mode(ctx, p, state, true, time, events)
    }
    fn rendered_mode(
        ctx: &egui::Context,
        p: &Project,
        state: &mut ViewState,
        editing: bool,
        time: f64,
        events: Vec<egui::Event>,
    ) -> (SceneEvent, egui::FullOutput) {
        let mut event = SceneEvent::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    event = draw(ui, p, &p.nodes[0], None, editing, state);
                });
            },
        );
        (event, output)
    }
    fn press(pos: Pos2, down: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: egui::Modifiers::NONE,
        }
    }
    #[test]
    fn dragging_an_edge_resizes_from_a_frozen_projection_across_frames() {
        let mut p = Project::default();
        let node = p.add_node("Device", [200., 300., 200.]);
        let ctx = egui::Context::default();
        let mut state = ViewState::default();
        let (_, out) = rendered(&ctx, &p, &mut state, 0., vec![]);
        let bounds = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                Shape::Path(path) if path.fill == Color32::from_rgb(74, 82, 91) => {
                    Some(Rect::from_points(&path.points))
                }
                _ => None,
            })
            .unwrap();
        let start = bounds.right_center();
        rendered(
            &ctx,
            &p,
            &mut state,
            0.1,
            vec![egui::Event::PointerMoved(start)],
        );
        rendered(&ctx, &p, &mut state, 0.2, vec![press(start, true)]);
        let (e, _) = rendered(
            &ctx,
            &p,
            &mut state,
            0.3,
            vec![egui::Event::PointerMoved(start + Vec2::new(40., 0.))],
        );
        assert!(e.drag_started);
        let (part, size, offset) = e.resized.expect("edge drag must resize");
        assert!(size[0] > 200.);
        assert_eq!(size[1..], [300., 200.]);
        p.resize_part(node, part, size, offset).unwrap();
        let (e, _) = rendered(
            &ctx,
            &p,
            &mut state,
            0.4,
            vec![egui::Event::PointerMoved(start + Vec2::new(80., 0.))],
        );
        let (_, next, _) = e.resized.unwrap();
        assert!(
            (next[0] - 200. - 2. * (size[0] - 200.)).abs() < 0.001,
            "drag must not refit on each frame"
        );
        assert!(!e.drag_started);
    }
    #[test]
    fn label_drag_moves_only_the_label_and_leader_follows_socket() {
        let mut p = Project::default();
        let node = p.add_node("Device", [200., 300., 200.]);
        let port = p.add_port(node, PortKind::UsbA, "USB-A 1").unwrap();
        let ctx = egui::Context::default();
        let mut state = ViewState::default();
        let (_, out) = rendered(&ctx, &p, &mut state, 0., vec![]);
        let label = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                Shape::Text(t) if t.galley.job.text == "not connected" => {
                    Some(t.pos + t.galley.size() / 2.)
                }
                _ => None,
            })
            .unwrap();
        assert!(
            !out.shapes
                .iter()
                .any(|s| matches!(&s.shape,Shape::Text(t) if t.galley.job.text=="USB-A 1"))
        );
        rendered(
            &ctx,
            &p,
            &mut state,
            0.1,
            vec![egui::Event::PointerMoved(label)],
        );
        rendered(&ctx, &p, &mut state, 0.2, vec![press(label, true)]);
        let (e, _) = rendered(
            &ctx,
            &p,
            &mut state,
            0.3,
            vec![egui::Event::PointerMoved(label + Vec2::new(-80., 60.))],
        );
        let (id, offset) = e.label_moved.expect("label drag emits persistent layout");
        assert_eq!(id, port);
        assert!(e.drag_started);
        assert!(e.moved.is_none());
        assert!(e.resized.is_none());
        p.port_mut(PortRef { node, port }).unwrap().label_offset = Some(offset);
        let (_, out) = rendered(
            &ctx,
            &p,
            &mut state,
            0.4,
            vec![press(label + Vec2::new(-80., 60.), false)],
        );
        assert_eq!(p.port(PortRef { node, port }).unwrap().uv, [0., 0.]);
        let line = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                Shape::LineSegment { points, stroke }
                    if stroke.color == Color32::from_rgb(125, 151, 155) =>
                {
                    Some(points)
                }
                _ => None,
            })
            .unwrap();
        assert!((line[0] - Pos2::new(400., 300.)).length() < 0.1);
        assert!(line[0].distance(line[1]) > 30.);
    }
    #[test]
    fn linked_port_hover_and_dragged_node_resolve_without_creating_nodes() {
        let mut p = Project::default();
        let node = p.add_node("PC", [200., 300., 200.]);
        let port = p.add_port(node, PortKind::UsbA, "A").unwrap();
        let hub = p.add_node("Hub", [80., 40., 80.]);
        let inlet = p.add_port(hub, PortKind::UsbC, "Input").unwrap();
        p.connect(
            PortRef { node, port },
            Target::Port(PortRef {
                node: hub,
                port: inlet,
            }),
            false,
            false,
        )
        .unwrap();
        let ctx = egui::Context::default();
        let mut state = ViewState::default();
        let (e, out) = rendered(
            &ctx,
            &p,
            &mut state,
            0.,
            vec![egui::Event::PointerMoved(Pos2::new(400., 300.))],
        );
        assert_eq!(e.hovered_node, Some(hub));
        assert!(
            out.shapes
                .iter()
                .any(|s| matches!(&s.shape,Shape::Text(t) if t.galley.job.text=="Hub"))
        );
        assert!(
            !out.shapes
                .iter()
                .any(|s| matches!(&s.shape,Shape::Text(t) if t.galley.job.text.contains("Input")))
        );
        rendered(
            &ctx,
            &p,
            &mut state,
            0.1,
            vec![press(Pos2::new(400., 300.), true)],
        );
        egui::DragAndDrop::set_payload(&ctx, NodeDrag { node: hub });
        let (e, _) = rendered(
            &ctx,
            &p,
            &mut state,
            0.2,
            vec![press(Pos2::new(400., 300.), false)],
        );
        assert_eq!(e.node_dropped, Some((port, hub)));
        assert_eq!(p.nodes.len(), 2);
    }

    #[test]
    fn drop_cannot_clamp_a_socket_behind_another_building_block() {
        let mut p = Project::default();
        let id = p.add_node("Enhet", [100.; 3]);
        p.add_part(id, Primitive::Box).unwrap();
        p.node_mut(id).unwrap().parts[1].size = [100.; 3];
        p.node_mut(id).unwrap().parts[1].offset = [5., 0., -60.];
        let node = p.node(id).unwrap();
        let camera = Camera::for_face(Face::Back);
        let hit = surface_at(node, &camera, [49., 0.]).unwrap();
        assert_eq!(hit.part, node.parts[0].id);
        assert!(drop_preview(node, PortKind::Xlr, hit, &camera).is_none());
    }
    #[test]
    fn exact_socket_hit_beats_a_selected_neighbour_within_click_tolerance() {
        let sockets: Vec<_> = [(1, 0_f32), (2, 5_f32)]
            .into_iter()
            .map(|(id, x)| Socket {
                id,
                center: Pos2::new(x, 0.),
                points: vec![
                    Pos2::new(x - 1.75, -1.75),
                    Pos2::new(x + 1.75, -1.75),
                    Pos2::new(x + 1.75, 1.75),
                    Pos2::new(x - 1.75, 1.75),
                ],
                name: String::new(),
                kind: String::new(),
                connection: None,
                depth: 0.,
            })
            .collect();
        assert_eq!(
            pick_socket(&sockets, Pos2::new(5., 0.), Some(1), 9.)
                .unwrap()
                .id,
            2
        );
    }
    #[test]
    fn right_click_menu_emits_delete_for_the_clicked_socket() {
        let mut p = Project::default();
        let node = p.add_node("PC", [200., 300., 200.]);
        let port = p.add_port(node, PortKind::UsbA, "USB 1").unwrap();
        let ctx = egui::Context::default();
        let mut state = ViewState::default();
        let pos = Pos2::new(400., 300.);
        frame(
            &ctx,
            &p,
            &mut state,
            0.,
            vec![egui::Event::PointerMoved(pos)],
        );
        frame(
            &ctx,
            &p,
            &mut state,
            0.1,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        frame(
            &ctx,
            &p,
            &mut state,
            0.2,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))),
                time: Some(0.25),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw(ui, &p, &p.nodes[0], None, true, &mut state);
                });
            },
        );
        let point = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                Shape::Text(t) if t.galley.job.text == "Delete" => {
                    Some(t.pos + t.galley.size() / 2.)
                }
                _ => None,
            })
            .expect("Delete menu item must be visible");
        frame(
            &ctx,
            &p,
            &mut state,
            0.3,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        let result = frame(
            &ctx,
            &p,
            &mut state,
            0.4,
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(result.delete, Some(port));
    }
    fn frame(
        ctx: &egui::Context,
        p: &Project,
        state: &mut ViewState,
        time: f64,
        events: Vec<egui::Event>,
    ) -> SceneEvent {
        let mut result = SceneEvent::default();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    result = draw(ui, p, &p.nodes[0], None, true, state);
                });
            },
        );
        result
    }
    #[test]
    fn native_drag_payload_drops_on_the_visible_device_face() {
        let mut p = Project::default();
        p.add_node("PC", [200., 300., 200.]);
        let mut state = ViewState::default();
        let ctx = egui::Context::default();
        let pos = Pos2::new(400., 300.);
        frame(
            &ctx,
            &p,
            &mut state,
            0.,
            vec![egui::Event::PointerMoved(pos)],
        );
        frame(
            &ctx,
            &p,
            &mut state,
            0.1,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        egui::DragAndDrop::set_payload(&ctx, PortKind::UsbC);
        let result = frame(
            &ctx,
            &p,
            &mut state,
            0.2,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        let dropped = result.dropped.unwrap();
        assert_eq!(dropped.kind, PortKind::UsbC);
        assert_eq!(dropped.hit.face, Face::Back);
    }
    #[test]
    fn double_clicking_a_socket_opens_its_equipment_text_editor() {
        let mut p = Project::default();
        let node = p.add_node("PC", [200., 300., 200.]);
        let port = p.add_port(node, PortKind::UsbA, "USB 1").unwrap();
        let mut state = ViewState::default();
        let ctx = egui::Context::default();
        let pos = Pos2::new(400., 300.);
        frame(
            &ctx,
            &p,
            &mut state,
            0.,
            vec![egui::Event::PointerMoved(pos)],
        );
        let mut result = SceneEvent::default();
        for (i, pressed) in [true, false, true, false].into_iter().enumerate() {
            result = frame(
                &ctx,
                &p,
                &mut state,
                0.05 + i as f64 * 0.05,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
        assert_eq!(result.edit_label, Some(port));
    }
    #[test]
    fn background_drags_cannot_change_the_named_side_orientation() {
        for button in [egui::PointerButton::Primary, egui::PointerButton::Secondary] {
            let p = Project::demo();
            let n = &p.nodes[0];
            let mut state = ViewState::default();
            state.select(n);
            let expected = (state.camera.yaw, state.camera.pitch);
            let ctx = egui::Context::default();
            let mut render = |time, events| {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))),
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            draw(ui, &p, n, None, true, &mut state);
                        });
                    },
                );
            };
            let start = Pos2::new(35., 50.);
            render(0., vec![egui::Event::PointerMoved(start)]);
            render(
                0.1,
                vec![egui::Event::PointerButton {
                    pos: start,
                    button,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            render(0.2, vec![egui::Event::PointerMoved(Pos2::new(155., 140.))]);
            assert_eq!((state.camera.yaw, state.camera.pitch), expected);
        }
    }
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
