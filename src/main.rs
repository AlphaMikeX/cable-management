#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

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
            let _ = std::fs::remove_file(storage::backup_path(&file));
            let _ = std::fs::remove_file(&file);
            Ok(())
        };
        match test() {
            Ok(()) => {
                let _ = std::fs::write(
                    report,
                    "PASS: graph, endpoint route, Windows save/open and replacement backup\n",
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
