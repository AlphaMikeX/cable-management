#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod palette;
mod port_label;
mod resize;
mod scene;
mod ui;

fn main() -> eframe::Result<()> {
    let arguments: Vec<_> = std::env::args_os().collect();
    if arguments.get(1).is_some_and(|a| a == "--self-test") {
        let report = arguments
            .get(2)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("cable-self-test.txt"));
        let test = || -> Result<(), String> {
            use cable_management::{model::*, storage};
            let p = Project::demo();
            p.validate()?;
            let file = report.with_extension("cable.json");
            storage::save(&file, &p)?;
            if storage::load(&file)? != p {
                return Err("Roundtrip mismatch".into());
            }
            let mut next = p.clone();
            next.name = "Windows replacement test".into();
            storage::save(&file, &next)?;
            if storage::load(&storage::backup_path(&file))? != p {
                return Err("Backup mismatch".into());
            }
            if p.route_to_endpoint(p.endpoints[0].id).len() != 2 {
                return Err("Route mismatch".into());
            }
            let mut annotated = p.clone();
            let node = annotated.nodes[0].id;
            let part = annotated.nodes[0].parts[0].id;
            let port = annotated.place_port_rotated(
                node,
                cable_management::ports::PortKind::UsbC,
                part,
                Face::Front,
                [0., 0.],
                270.,
            )?;
            if annotated.port(PortRef { node, port }).unwrap().angle != 270. {
                return Err("Pre-rotated placement mismatch".into());
            }
            annotated.label_port(PortRef { node, port }, "gamingmus")?;
            if annotated.nodes.len() != p.nodes.len() {
                return Err("Text label created a node".into());
            }
            storage::save(&file, &annotated)?;
            if storage::load(&file)? != annotated {
                return Err("Port/label roundtrip mismatch".into());
            }
            let reference = PortRef { node, port };
            annotated.rotate_port(reference, 405.)?;
            annotated.port_mut(reference).unwrap().label_offset = Some([-35., 28.]);
            annotated.resize_part(node, part, [230., 440., 390.], [10., 5., 0.])?;
            if annotated.port(reference).unwrap().angle != 45.
                || annotated.port_label(reference) != "gamingmus"
            {
                return Err("Canvas editing mismatch".into());
            }
            storage::save(&file, &annotated)?;
            if storage::load(&file)? != annotated {
                return Err("Canvas layout roundtrip mismatch".into());
            }
            annotated.remove_port(reference);
            annotated.validate()?;
            let _ = std::fs::remove_file(storage::backup_path(&file));
            let _ = std::fs::remove_file(&file);
            Ok(())
        };
        match test() {
            Ok(()) => {
                let _ = std::fs::write(
                    report,
                    "PASS: graph, endpoint route, port placement, equipment labels, rotation, resizing, label layout, save/open and replacement backup\n",
                );
            }
            Err(e) => {
                let _ = std::fs::write(report, format!("FAIL: {e}\n"));
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440., 930.])
            .with_min_inner_size([1080., 720.]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Cable Management",
        options,
        Box::new(move |cc| {
            Ok(Box::new(ui::CableApp::new(
                cc,
                arguments.get(1).map(std::path::PathBuf::from),
            )))
        }),
    )
}
