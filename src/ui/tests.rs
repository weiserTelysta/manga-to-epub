use super::*;
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};

fn app() -> (TelystaApp, egui::Context) {
    let ctx = egui::Context::default();
    view::install_fonts(&ctx);
    ctx.enable_accesskit();
    Appearance::Campus.apply(&ctx);
    let mut app = TelystaApp::with_appearance(Appearance::Campus);
    app.tasks.push(Task {
        id: 0,
        source: BookSource::Path(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/page.rar"),
        ),
        title: "交互测试".into(),
        status: Status::Pending,
        thumbnail: None,
        preview_requested: true,
    });
    app.selected = Some(0);
    app.next_id = 1;
    frame(&mut app, &ctx, Vec::new(), Modifiers::NONE);
    frame(&mut app, &ctx, Vec::new(), Modifiers::NONE);
    (app, ctx)
}

fn frame(
    app: &mut TelystaApp,
    ctx: &egui::Context,
    events: Vec<Event>,
    modifiers: Modifiers,
) -> Vec<(String, Rect)> {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))),
        events,
        modifiers,
        focused: true,
        ..Default::default()
    };
    let output = ctx.run(input, |ctx| {
        app.receive(ctx);
        app.shortcuts(ctx);
        app.render(ctx);
    });
    output
        .platform_output
        .accesskit_update
        .expect("accessibility enabled")
        .nodes
        .into_iter()
        .filter_map(|(_, node)| {
            let name = node.label()?.to_owned();
            let rect = node.bounds()?;
            Some((
                name,
                Rect::from_min_max(
                    Pos2::new(rect.x0 as f32, rect.y0 as f32),
                    Pos2::new(rect.x1 as f32, rect.y1 as f32),
                ),
            ))
        })
        .collect()
}

fn click(app: &mut TelystaApp, ctx: &egui::Context, name: &str) {
    let controls = frame(app, ctx, Vec::new(), Modifiers::NONE);
    let pos = controls
        .iter()
        .find(|(label, _)| label == name)
        .unwrap_or_else(|| panic!("Control {name} missing: {controls:?}"))
        .1
        .center();
    for pressed in [true, false] {
        frame(
            app,
            ctx,
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
            Modifiers::NONE,
        );
    }
}

#[test]
fn real_format_and_theme_controls_change_application_state() {
    let (mut app, ctx) = app();
    click(&mut app, &ctx, "PDF");
    assert_eq!(
        app.format,
        BookFormat::Pdf,
        "The visible PDF control must select PDF"
    );
    click(&mut app, &ctx, "切换到深色");
    assert_eq!(app.appearance, Appearance::Night);
    click(&mut app, &ctx, "切换到浅色");
    assert_eq!(app.appearance, Appearance::Campus);
}

#[test]
fn keyboard_shortcut_converts_using_the_format_selected_in_the_ui() {
    let (mut app, ctx) = app();
    let output = tempfile::tempdir().unwrap();
    app.output_parent = Some(output.path().to_owned());
    click(&mut app, &ctx, "PDF");
    assert_eq!(app.format, BookFormat::Pdf);
    frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers {
                ctrl: true,
                command: true,
                ..Default::default()
            },
        }],
        Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    );
    assert!(
        app.busy,
        "Ctrl+Enter must start the actual conversion worker"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.busy && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
        app.receive(&ctx);
    }
    assert!(!app.busy, "Conversion worker did not finish");
    let Status::Done(result) = &app.tasks[0].status else {
        panic!("The GUI conversion did not succeed")
    };
    assert_eq!(result.path.extension().unwrap(), "pdf");
    assert!(std::fs::read(&result.path).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn removing_a_selected_row_keeps_the_remaining_selection_valid() {
    let (mut app, ctx) = app();
    click(&mut app, &ctx, "移除书籍");
    assert!(
        app.tasks.is_empty(),
        "The visible row remove control must remove its task"
    );
    assert_eq!(app.selected, None);
}
