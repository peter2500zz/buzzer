use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowId};

// Your app state — owns windows, renderers, etc.
struct App {
    context: Context<OwnedDisplayHandle>,
    state: Option<AppState>,
}

struct AppState {
    window: Rc<Window>,
    surface: Surface<OwnedDisplayHandle, Rc<Window>>,
}

impl ApplicationHandler for App {
    // Platform signals ready — create windows here
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_decorations(false)
            .with_title("Buzzer");

        let window = Rc::new(event_loop.create_window(attrs).unwrap());

        let mut surface = Surface::new(&self.context, Rc::clone(&window)).unwrap();

        let size = window.inner_size();
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            // Resize surface
            surface.resize(width, height).unwrap();
        }

        self.state = Some(AppState { window, surface });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                // render here
                self.state.as_ref().unwrap().window.request_redraw();

                let surface = &mut self.state.as_mut().unwrap().surface;

                let mut buffer = surface.buffer_mut().unwrap();

                buffer.fill(0xFFF8CEF0);

                buffer.present().unwrap();
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        context: Context::new(event_loop.owned_display_handle()).unwrap(),
        state: None,
    };
    event_loop.run_app(&mut app).unwrap();
}
