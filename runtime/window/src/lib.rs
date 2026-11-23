// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Windows and the input that comes to them (spec 14.22, decision 0104),
//! for the interpreter and native code alike, through `winit`.
//!
//! The system's event loop runs on a thread of its own, started when the
//! first window opens. Windows are known by number. Commands go to the
//! loop through its proxy; events come back into each window's inbox,
//! where `take` finds them.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, mpsc};
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

mod events;
mod inbox;
pub use events::*;
use inbox::{Inbox, hub};
pub use inbox::{
    Waker, deliver_input, frame_shown, listen_input, on_event, ready, scale, size, take,
    take_numbers, texts, wake,
};

/// How long the event loop waits for a frame after a `Redraw`, while the
/// system may be in its own loop of a resize (spec 14.22).
const FRAME_WAIT: Duration = Duration::from_millis(50);

/// What a program asks the event loop to do.
enum Command {
    Open(String, u32, u32, mpsc::Sender<Result<i64, String>>),
    Title(i64, String),
    Fullscreen(i64, bool),
    LockCursor(i64, bool),
    /// Input methods on or off (spec 14.22.2).
    TextInput(i64, bool),
    /// Where the caret is: `x`, `y`, width and height in pixels.
    Caret(i64, [f64; 4]),
    Close(i64),
    /// Runs on the event loop's thread with the window, or `None` once it
    /// is closed: some systems give a window's handle only there.
    Run(i64, Box<dyn FnOnce(Option<Arc<Window>>) + Send>),
}

/// The running event loop, or why there is none.
fn proxy() -> Result<&'static EventLoopProxy<Command>, String> {
    static LOOP: OnceLock<Result<EventLoopProxy<Command>, String>> = OnceLock::new();
    LOOP.get_or_init(start).as_ref().map_err(Clone::clone)
}

/// Starts the event loop on a thread of its own.
fn start() -> Result<EventLoopProxy<Command>, String> {
    let (tell, told) = mpsc::channel();
    std::thread::Builder::new()
        .name("kavach-window".into())
        .spawn(move || match event_loop() {
            Ok(el) => {
                let _ = tell.send(Ok(el.create_proxy()));
                let _ = el.run_app(&mut App::default());
            }
            Err(why) => {
                let _ = tell.send(Err(why));
            }
        })
        .map_err(|e| e.to_string())?;
    told.recv().map_err(|e| e.to_string())?
}

/// An event loop that may run on any thread: the program's own first
/// thread runs the program.
fn event_loop() -> Result<EventLoop<Command>, String> {
    #[cfg(target_os = "macos")]
    {
        Err("macOS shows windows only from a program's first thread (spec 14.22)".into())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut b = EventLoop::<Command>::with_user_event();
        #[cfg(windows)]
        winit::platform::windows::EventLoopBuilderExtWindows::with_any_thread(&mut b, true);
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            winit::platform::x11::EventLoopBuilderExtX11::with_any_thread(&mut b, true);
            winit::platform::wayland::EventLoopBuilderExtWayland::with_any_thread(&mut b, true);
        }
        b.build().map_err(|e| shown_error(&e))
    }
}

/// The event loop's side: the windows, by number and by `winit`'s id.
#[derive(Default)]
struct App {
    windows: HashMap<i64, Arc<Window>>,
    numbers: HashMap<WindowId, i64>,
    next: i64,
    focused: Option<i64>,
}

impl App {
    fn open(&mut self, el: &ActiveEventLoop, title: &str, w: u32, h: u32) -> Result<i64, String> {
        let attributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(PhysicalSize::new(w.max(1), h.max(1)));
        let window = Arc::new(el.create_window(attributes).map_err(|e| shown_error(&e))?);
        self.next += 1;
        let n = self.next;
        let size = window.inner_size();
        let inbox = Inbox {
            size: (size.width, size.height),
            scale: window.scale_factor(),
            ..Inbox::default()
        };
        hub().add(n, inbox);
        self.numbers.insert(window.id(), n);
        self.windows.insert(n, window);
        Ok(n)
    }
}

impl ApplicationHandler<Command> for App {
    fn resumed(&mut self, _: &ActiveEventLoop) {}

    fn user_event(&mut self, el: &ActiveEventLoop, command: Command) {
        match command {
            Command::Open(title, w, h, reply) => {
                let _ = reply.send(self.open(el, &title, w, h));
            }
            Command::Title(n, title) => {
                if let Some(w) = self.windows.get(&n) {
                    w.set_title(&title);
                }
            }
            Command::Fullscreen(n, on) => {
                if let Some(w) = self.windows.get(&n) {
                    w.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
                }
            }
            Command::LockCursor(n, on) => {
                if let Some(w) = self.windows.get(&n) {
                    let mode = if on {
                        CursorGrabMode::Locked
                    } else {
                        CursorGrabMode::None
                    };
                    // Some systems confine the pointer but can't lock it.
                    if w.set_cursor_grab(mode).is_err() && on {
                        let _ = w.set_cursor_grab(CursorGrabMode::Confined);
                    }
                    w.set_cursor_visible(!on);
                }
            }
            Command::TextInput(n, on) => {
                if let Some(w) = self.windows.get(&n) {
                    w.set_ime_allowed(on);
                }
            }
            Command::Caret(n, [x, y, width, height]) => {
                if let Some(w) = self.windows.get(&n) {
                    w.set_ime_cursor_area(
                        winit::dpi::PhysicalPosition::new(x, y),
                        PhysicalSize::new(width, height),
                    );
                }
            }
            Command::Run(n, f) => f(self.windows.get(&n).cloned()),
            Command::Close(n) => {
                if let Some(w) = self.windows.remove(&n) {
                    self.numbers.remove(&w.id());
                }
                hub().remove(n);
            }
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(&n) = self.numbers.get(&id) else {
            return;
        };
        if let WindowEvent::Focused(on) = event {
            self.focused = on.then_some(n);
        }
        let redraw = matches!(event, WindowEvent::RedrawRequested);
        let seen = inbox::shown(n);
        hub().change(n, |inbox| {
            let events = events::translate(event, inbox);
            let any = !events.is_empty();
            events.into_iter().for_each(|e| inbox.push(e));
            any
        });
        // A window the program draws into is drawn again before the
        // system goes on: so while a person drags its border, it is drawn
        // at each new size (spec 14.22).
        if let (true, Some(seen @ 1..)) = (redraw, seen) {
            inbox::wait_for_frame(n, seen, FRAME_WAIT);
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let (DeviceEvent::MouseMotion { delta }, Some(n)) = (event, self.focused) else {
            return;
        };
        hub().change(n, |inbox| {
            if inbox.input {
                inbox.push(plain(MOTION, delta.0, delta.1));
            }
            inbox.input
        });
    }
}

/// Calls `f` with window `n` on the event loop's thread, where every
/// system gives its handle, and gives back what `f` gives; `None` when
/// the window is closed or there is no event loop. `std.gpu` makes its
/// surface on the window here (spec 14.24); `f` must be quick, as the
/// window's events wait meanwhile.
pub fn with_window<R: Send + 'static>(
    n: i64,
    f: impl FnOnce(Arc<Window>) -> R + Send + 'static,
) -> Option<R> {
    let (reply, answer) = mpsc::channel();
    let run = Box::new(move |w: Option<Arc<Window>>| {
        let _ = reply.send(w.map(f));
    });
    proxy().ok()?.send_event(Command::Run(n, run)).ok()?;
    answer.recv().ok().flatten()
}

/// Opens a window of `width` × `height` pixels titled `title`: its
/// number, or why the system shows no windows here.
pub fn open(title: &str, width: i64, height: i64) -> Result<i64, String> {
    let (reply, answer) = mpsc::channel();
    let fit = |n: i64| u32::try_from(n.clamp(1, 16_384)).unwrap_or(1);
    let command = Command::Open(title.to_string(), fit(width), fit(height), reply);
    proxy()?
        .send_event(command)
        .map_err(|_| "the window event loop has stopped".to_string())?;
    answer.recv().map_err(|e| e.to_string())?
}

fn tell(command: Command) {
    if let Ok(p) = proxy() {
        let _ = p.send_event(command);
    }
}

pub fn set_title(n: i64, title: &str) {
    tell(Command::Title(n, title.to_string()));
}

pub fn set_fullscreen(n: i64, on: bool) {
    tell(Command::Fullscreen(n, on));
}

pub fn lock_cursor(n: i64, on: bool) {
    tell(Command::LockCursor(n, on));
}

/// Lets input methods compose text in window `n`, or not.
pub fn set_text_input(n: i64, on: bool) {
    tell(Command::TextInput(n, on));
}

/// The caret of window `n` is at `x`, `y`, `width` × `height` pixels.
pub fn set_caret(n: i64, area: [f64; 4]) {
    tell(Command::Caret(n, area));
}

/// Closes window `n`; closing it again does nothing. Its state goes at
/// once, before the event loop closes it, so `size()` right after is
/// `(0, 0)` and `poll()` gives nothing, in both modes (spec 14.22).
pub fn close(n: i64) {
    hub().remove(n);
    tell(Command::Close(n));
}

#[cfg(test)]
mod tests;

mod errors;
use errors::shown_error;
