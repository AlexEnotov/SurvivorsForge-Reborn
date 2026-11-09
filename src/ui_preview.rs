//! Offline visual fixtures: no configuration, network calls or game files are used.
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter};
use slint::{ComponentHandle, Rgb8Pixel};
use std::rc::Rc;

struct Software(Rc<MinimalSoftwareWindow>);
impl Platform for Software {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> { Ok(self.0.clone()) }
}
fn save(window: &MinimalSoftwareWindow, name: &str, width: u32, height: u32) {
    window.set_size(slint::PhysicalSize::new(width,height));
    let mut pixels=vec![Rgb8Pixel::default();(width*height) as usize];
    window.request_redraw();
    window.draw_if_needed(|renderer| { renderer.render(&mut pixels,width as usize); });
    let bytes=pixels.iter().flat_map(|p| [p.r,p.g,p.b]).collect();
    let dir=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".qa/ui");
    std::fs::create_dir_all(&dir).unwrap();
    let prefix=std::env::var("PREVIEW_LABEL").unwrap_or_else(|_| "preview".into());
    image::RgbImage::from_raw(width,height,bytes).unwrap().save(dir.join(format!("{prefix}-{name}.png"))).unwrap();
}

#[test]
#[ignore = "Offline visual capture; run separately with --ignored"]
fn capture_ui() {
    let window=MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(Software(window.clone()))).unwrap();
    let ui=crate::AppWindow::new().unwrap();
    let _=slint::select_bundled_translation("en");
    ui.set_app_version("0.1.0".into());
    ui.set_addons_example(r"...InterfaceAddOns".into());
    ui.set_addons_dir(r"D:\Games\Reborn\Interface\AddOns".into());
    ui.set_settings_path(r"D:\Games\Reborn\Interface\AddOns".into());
    ui.set_switches_available(true);
    ui.set_installed_total(4);
    ui.set_update_count(1);
    ui.set_enabled_total(3); ui.set_disabled_total(1);
    let row=|id:&str,name:&str,status:&str,kind:i32,action:&str,switch:&str| crate::AddonRow {
        id:id.into(),name:name.into(),initial:name[..1].into(),author:"Sample author".into(),
        version:"v1.2.0".into(),badge:"GitHub".into(),description:"A sample addon used only in this offline interface preview.".into(),
        status:status.into(),status_kind:kind,action:action.into(),switch_label:switch.into(),disabled:switch=="Enable",can_remove:true,can_open:true,
        ..Default::default()
    };
    ui.set_installed_rows(crate::model(vec![
        row("a","Combat Meter","Update: v1.3.0",2,"Update","Disable"),
        row("b","Build Planner","Up to date",1,"","Disable"),
        row("c","Quest Tracker","Installed from zip",0,"","Enable"),
        crate::AddonRow { warning:"This addon uses a different Interface version. Check compatibility with Reborn.".into(),..row("d","Extended Tooltip","Not managed",0,"","Disable") },
    ]));
    ui.show().unwrap();
    save(&window,"installed",1180,800);
    use slint::platform::{WindowEvent,PointerEventButton};
    let toggled=Rc::new(std::cell::Cell::new(false));
    let seen=toggled.clone(); ui.on_row_toggle(move |_| seen.set(true));
    let click=|x,y| {
        let position=slint::LogicalPosition::new(x,y);
        ui.window().dispatch_event(WindowEvent::PointerMoved{position});
        ui.window().dispatch_event(WindowEvent::PointerPressed{position,button:PointerEventButton::Left});
        ui.window().dispatch_event(WindowEvent::PointerReleased{position,button:PointerEventButton::Left});
    };
    click(1100.0,347.0); assert!(toggled.get(),"The visible switch invokes its addon callback");
    click(80.0,265.0); assert_eq!(ui.get_page(),2,"Sidebar navigation opens settings");
    let press=|key:slint::SharedString| {
        ui.window().dispatch_event(WindowEvent::KeyPressed{text:key.clone()});
        ui.window().dispatch_event(WindowEvent::KeyReleased{text:key});
    };
    ui.window().dispatch_event(WindowEvent::KeyPressed{text:slint::platform::Key::Control.into()});
    press("2".into());
    ui.window().dispatch_event(WindowEvent::KeyReleased{text:slint::platform::Key::Control.into()});
    assert_eq!(ui.get_page(),1,"Ctrl+2 opens the catalog");
    let refreshed=Rc::new(std::cell::Cell::new(false));
    let seen=refreshed.clone(); ui.on_check_updates(move || seen.set(true));
    press(slint::platform::Key::F5.into()); assert!(refreshed.get(),"F5 reaches the refresh callback");
    ui.window().dispatch_event(WindowEvent::KeyPressed{text:slint::platform::Key::Control.into()});
    press("f".into());
    ui.window().dispatch_event(WindowEvent::KeyReleased{text:slint::platform::Key::Control.into()});
    press("meter".into()); assert_eq!(ui.get_search().as_str(),"meter","Ctrl+F focuses the search field");
    press(slint::platform::Key::Escape.into()); assert!(ui.get_search().is_empty(),"Escape clears search");
    ui.set_page(2); save(&window,"settings",1180,800);
    save(&window,"settings-compact",980,660);
    let _=slint::select_bundled_translation("ru"); ui.set_language_index(1);
    save(&window,"settings-ru",980,660);
    let position=slint::LogicalPosition::new(700.0,400.0);
    ui.window().dispatch_event(WindowEvent::PointerScrolled{position,delta_x:0.0,delta_y:-600.0});
    save(&window,"settings-ru-scrolled",980,660);
    let _=slint::select_bundled_translation("de"); ui.set_language_index(2);
    save(&window,"settings-de",980,660);
    ui.set_page(0); save(&window,"installed-de",980,660);
    let _=slint::select_bundled_translation("en"); ui.set_language_index(0);
    ui.set_page(1); save(&window,"catalog",1180,800);
    ui.set_details_title("Combat Meter".into()); ui.set_details_text("A local addon description.\n\nThis window shows the full text, including compatibility warnings and folders, without requesting data from a website.".into());
    ui.set_details_open(true); save(&window,"details",980,660); ui.set_details_open(false);
    ui.set_needs_setup(true); ui.set_setup_searching(false);
    save(&window,"setup",980,660);
}
