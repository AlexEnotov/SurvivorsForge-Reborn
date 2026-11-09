//! Preserve native Latin shortcuts; translate their physical keys for non-Latin layouts.
use slint::winit_030::{winit, EventResult, WinitWindowAccessor};
use winit::keyboard::{Key, KeyCode, ModifiersState, PhysicalKey};

pub(super) fn shortcut_text(key: PhysicalKey, modifiers: ModifiersState, logical: Option<&str>) -> Option<&'static str> {
    let command = if cfg!(target_os="macos") { modifiers.super_key() } else { modifiers.control_key() };
    if !command || modifiers.alt_key() { return None; } // In particular, preserve AltGr typing.
    if logical.is_some_and(|text| text.len() == 1 && text.is_ascii() && text.as_bytes()[0].is_ascii_alphanumeric()) {
        return None; // Keep QWERTZ/AZERTY and ordinary native Latin shortcuts.
    }
    match key {
        PhysicalKey::Code(KeyCode::KeyA) => Some("a"), PhysicalKey::Code(KeyCode::KeyC) => Some("c"),
        PhysicalKey::Code(KeyCode::KeyV) => Some("v"), PhysicalKey::Code(KeyCode::KeyX) => Some("x"),
        PhysicalKey::Code(KeyCode::KeyZ) => Some("z"), PhysicalKey::Code(KeyCode::KeyY) => Some("y"),
        PhysicalKey::Code(KeyCode::KeyF) => Some("f"), PhysicalKey::Code(KeyCode::KeyU) => Some("u"),
        PhysicalKey::Code(KeyCode::Digit1) => Some("1"), PhysicalKey::Code(KeyCode::Digit2) => Some("2"),
        PhysicalKey::Code(KeyCode::Digit3) => Some("3"), _ => None,
    }
}

pub fn install(window: &slint::Window) {
    let mut modifiers = ModifiersState::empty();
    window.on_winit_window_event(move |window,event| {
        use winit::event::{ElementState, WindowEvent};
        match event {
            WindowEvent::ModifiersChanged(value) => modifiers=value.state(),
            WindowEvent::Focused(false) => modifiers=ModifiersState::empty(),
            WindowEvent::KeyboardInput { event,is_synthetic:false,.. } => {
                let logical=match &event.logical_key { Key::Character(text) => Some(text.as_str()), _ => None };
                if let Some(text)=shortcut_text(event.physical_key,modifiers,logical) {
                    use slint::platform::WindowEvent as SlintEvent;
                    window.dispatch_event(match event.state {
                        ElementState::Pressed if event.repeat => SlintEvent::KeyPressRepeated { text:text.into() },
                        ElementState::Pressed => SlintEvent::KeyPressed { text:text.into() },
                        ElementState::Released => SlintEvent::KeyReleased { text:text.into() },
                    });
                    return EventResult::PreventDefault;
                }
            }
            _ => {}
        }
        EventResult::Propagate
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn non_latin_shortcuts_work_without_changing_typing_altgr_or_german_keys() {
        let ctrl=if cfg!(target_os="macos") { ModifiersState::SUPER } else { ModifiersState::CONTROL };
        for (code,logical,expected) in [(KeyCode::KeyC,"с","c"),(KeyCode::KeyV,"м","v"),(KeyCode::KeyX,"ч","x"),(KeyCode::KeyA,"ф","a"),(KeyCode::KeyF,"а","f"),(KeyCode::KeyU,"г","u")] {
            assert_eq!(shortcut_text(PhysicalKey::Code(code),ctrl,Some(logical)),Some(expected));
            assert_eq!(shortcut_text(PhysicalKey::Code(code),ModifiersState::empty(),Some(logical)),None);
            assert_eq!(shortcut_text(PhysicalKey::Code(code),ctrl|ModifiersState::ALT,Some(logical)),None);
        }
        assert_eq!(shortcut_text(PhysicalKey::Code(KeyCode::KeyY),ctrl,Some("z")),None);
        assert_eq!(shortcut_text(PhysicalKey::Code(KeyCode::KeyC),ctrl,Some("c")),None);
    }
}
