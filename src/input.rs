use winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{Key, NamedKey},
};

use crate::app::AppState;

pub enum Action {
    NextImage,
    PreviousImage,

    ZoomIn,
    ZoomOut,

    Quit,
}

pub fn handle_input(event: &WindowEvent, _state: &mut AppState) -> Option<Action> {
    match event {
        WindowEvent::MouseInput { state, button, .. } => {
            match (button, state) {
                (MouseButton::Left, ElementState::Pressed) => {
                    Some(Action::ZoomIn)
                },
                (MouseButton::Left, ElementState::Released) => {
                    Some(Action::ZoomOut)
                },

                _ => None,
            }
        }

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
