use anyhow::{anyhow, Ok, Result};
use glfw::{fail_on_errors, Action, Context, Glfw, GlfwReceiver, Key, PWindow, WindowEvent};

pub struct WindowManager {
    glfw: Glfw,
    window: PWindow,
    receiver: GlfwReceiver<(f64, WindowEvent)>,
}

impl WindowManager {
    pub fn try_new() -> Result<Self> {
        let mut glfw = glfw::init(fail_on_errors!())?;
        let (mut window, events) = glfw
            .create_window(800, 600, "Hello, World!", glfw::WindowMode::Windowed)
            .ok_or(anyhow!("Failed to create GLFW window"))?;

        window.make_current();
        window.set_key_polling(true);

        Ok(Self {
            window,
            glfw,
            receiver: events,
        })
    }

    pub fn run_event_loop(&mut self) {
        while !self.window.should_close() {
            self.window.swap_buffers();
            self.glfw.wait_events();
            for (_, event) in glfw::flush_messages(&self.receiver) {
                println!("{:?}", event);
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
