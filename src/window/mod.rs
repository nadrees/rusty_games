use anyhow::{anyhow, Ok, Result};
use glfw::{Action, Glfw, GlfwReceiver, Key, PWindow, WindowEvent};
use tracing::debug;

pub struct WindowManager {
    pub window: PWindow,
    receiver: GlfwReceiver<(f64, WindowEvent)>,
}

impl WindowManager {
    pub fn try_new(glfw: &mut Glfw) -> Result<Self> {
        let (window, events) = glfw
            .create_window(800, 600, "Hello, World!", glfw::WindowMode::Windowed)
            .ok_or(anyhow!("Failed to create GLFW window"))?;

        Ok(Self {
            window,
            receiver: events,
        })
    }

    pub fn run_event_loop(&mut self, glfw: &mut Glfw) {
        self.window.set_key_polling(true);

        while !self.window.should_close() {
            glfw.wait_events();
            for (_, event) in glfw::flush_messages(&self.receiver) {
                debug!(message = format!("Event = {:?}", event));
                match event {
                    glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                        self.window.set_should_close(true);
                    }
                    _ => {}
                }
            }
        }
    }
}
