use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{Key, NamedKey},
};

use crate::app::AppState;

pub enum Action {
    NextImage,
    PreviousImage,

    Quit,
}

pub fn handle_input(event: &WindowEvent, _state: &mut AppState) -> Option<Action> {
    match event {
        WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    logical_key: Key::Named(named),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } => match named {
            NamedKey::Escape => Some(Action::Quit),
            NamedKey::ArrowLeft => Some(Action::PreviousImage),
            NamedKey::ArrowRight => Some(Action::NextImage),
            _ => None,
        },
        _ => None,
    }
}
