//! Windows and the input that comes to them (spec 14.22, decision 0104),
//! for the interpreter and native code alike, through `winit`.
//!
//! The system's event loop runs on a thread of its own, started when the
//! first window opens. Windows are known by number. Commands go to the
//! loop through its proxy; events come back into each window's inbox,
//! where `take` finds them.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, OnceLock, mpsc};
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{
    DeviceEvent, DeviceId, ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

/// The kinds of event, numbered as `std.window`'s `Event` variants are.
pub const CLOSED: i64 = 0;
pub const RESIZED: i64 = 1;
pub const SCALED: i64 = 2;
pub const FOCUSED: i64 = 3;
pub const KEY: i64 = 4;
pub const TEXT: i64 = 5;
pub const MOUSE_MOVED: i64 = 6;
pub const MOUSE_BUTTON: i64 = 7;
pub const WHEEL: i64 = 8;
pub const MOTION: i64 = 9;

/// One event: its kind, two numbers, and the text of a key or of typing.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub kind: i64,
    pub a: f64,
    pub b: f64,
    pub text: String,
}

/// What a window has that its program reads.
#[derive(Default)]
struct Inbox {
    events: VecDeque<Event>,
    /// Whether keyboard, mouse and text events come (`listen_input`).
    input: bool,
    size: (u32, u32),
    scale: f64,
}

/// Every window's inbox, and a signal when one gets an event.
struct Hub {
    inboxes: Mutex<HashMap<i64, Inbox>>,
    ready: Condvar,
}

fn hub() -> &'static Hub {
    static HUB: OnceLock<Hub> = OnceLock::new();
    HUB.get_or_init(|| Hub {
        inboxes: Mutex::new(HashMap::new()),
        ready: Condvar::new(),
    })
}

/// What a program asks the event loop to do.
enum Command {
    Open(String, u32, u32, mpsc::Sender<Result<i64, String>>),
    Title(i64, String),
    Fullscreen(i64, bool),
    LockCursor(i64, bool),
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
        b.build().map_err(|e| e.to_string())
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
        let window = Arc::new(el.create_window(attributes).map_err(|e| e.to_string())?);
        self.next += 1;
        let n = self.next;
        let size = window.inner_size();
        let inbox = Inbox {
            size: (size.width, size.height),
            scale: window.scale_factor(),
            ..Inbox::default()
        };
        hub()
            .inboxes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(n, inbox);
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
            Command::Run(n, f) => f(self.windows.get(&n).cloned()),
            Command::Close(n) => {
                if let Some(w) = self.windows.remove(&n) {
                    self.numbers.remove(&w.id());
                }
                hub()
                    .inboxes
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&n);
                hub().ready.notify_all();
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
        let mut all = hub().inboxes.lock().unwrap_or_else(|e| e.into_inner());
        let Some(inbox) = all.get_mut(&n) else { return };
        let events = translate(event, inbox);
        if events.is_empty() {
            return;
        }
        inbox.events.extend(events);
        drop(all);
        hub().ready.notify_all();
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let (DeviceEvent::MouseMotion { delta }, Some(n)) = (event, self.focused) else {
            return;
        };
        let mut all = hub().inboxes.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(inbox) = all.get_mut(&n).filter(|i| i.input) {
            inbox.events.push_back(plain(MOTION, delta.0, delta.1));
            drop(all);
            hub().ready.notify_all();
        }
    }
}

fn plain(kind: i64, a: f64, b: f64) -> Event {
    Event {
        kind,
        a,
        b,
        text: String::new(),
    }
}

/// What a window event means to a program, keeping the inbox's size and
/// scale up to date; input events only once it listens.
fn translate(event: WindowEvent, inbox: &mut Inbox) -> Vec<Event> {
    let input = inbox.input;
    let flag = |on: bool| if on { 1.0 } else { 0.0 };
    match event {
        WindowEvent::CloseRequested => vec![plain(CLOSED, 0.0, 0.0)],
        WindowEvent::Resized(size) => {
            inbox.size = (size.width, size.height);
            vec![plain(
                RESIZED,
                f64::from(size.width),
                f64::from(size.height),
            )]
        }
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            inbox.scale = scale_factor;
            vec![plain(SCALED, scale_factor, 0.0)]
        }
        WindowEvent::Focused(on) => vec![plain(FOCUSED, flag(on), 0.0)],
        WindowEvent::KeyboardInput { event, .. } if input => {
            let down = event.state == ElementState::Pressed;
            let mut out = Vec::new();
            if let (PhysicalKey::Code(code), false) = (event.physical_key, event.repeat) {
                let mut e = plain(KEY, flag(down), 0.0);
                e.text = format!("{code:?}");
                out.push(e);
            }
            if let (Some(text), true) = (event.text, down) {
                let mut e = plain(TEXT, 0.0, 0.0);
                e.text = text.to_string();
                out.push(e);
            }
            out
        }
        WindowEvent::Ime(Ime::Commit(text)) if input => {
            let mut e = plain(TEXT, 0.0, 0.0);
            e.text = text;
            vec![e]
        }
        WindowEvent::CursorMoved { position, .. } if input => {
            vec![plain(MOUSE_MOVED, position.x, position.y)]
        }
        WindowEvent::MouseInput { state, button, .. } if input => {
            let number = match button {
                MouseButton::Left => 0.0,
                MouseButton::Right => 1.0,
                MouseButton::Middle => 2.0,
                MouseButton::Back => 3.0,
                MouseButton::Forward => 4.0,
                MouseButton::Other(n) => f64::from(n),
            };
            vec![plain(
                MOUSE_BUTTON,
                number,
                flag(state == ElementState::Pressed),
            )]
        }
        WindowEvent::MouseWheel { delta, .. } if input => {
            let (x, y) = match delta {
                MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y)),
                // About 20 pixels make a line, as browsers count them.
                MouseScrollDelta::PixelDelta(p) => (p.x / 20.0, p.y / 20.0),
            };
            vec![plain(WHEEL, x, y)]
        }
        _ => Vec::new(),
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

/// The events window `n` has had since the last `take`; with `wait`,
/// waiting for at least one first. None once the window is closed.
pub fn take(n: i64, wait: bool) -> Vec<Event> {
    let mut all = hub().inboxes.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        let Some(inbox) = all.get_mut(&n) else {
            return Vec::new();
        };
        if !inbox.events.is_empty() || !wait {
            return inbox.events.drain(..).collect();
        }
        all = (hub().ready)
            .wait_timeout(all, Duration::from_millis(250))
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
}

/// `take`, as `std.window` reads it: three numbers an event (its kind and
/// two numbers), with the texts kept for `texts`, one an event.
pub fn take_numbers(n: i64, wait: bool) -> Vec<f64> {
    let got = take(n, wait);
    let texts = got.iter().map(|e| e.text.clone()).collect();
    last_texts()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(n, texts);
    got.iter().flat_map(|e| [e.kind as f64, e.a, e.b]).collect()
}

/// The texts of window `n`'s last `take_numbers`: a key's name, or text
/// typed; empty for other events.
pub fn texts(n: i64) -> Vec<String> {
    let mut all = last_texts().lock().unwrap_or_else(|e| e.into_inner());
    all.remove(&n).unwrap_or_default()
}

fn last_texts() -> &'static Mutex<HashMap<i64, Vec<String>>> {
    static LAST: OnceLock<Mutex<HashMap<i64, Vec<String>>>> = OnceLock::new();
    LAST.get_or_init(|| Mutex::new(HashMap::new()))
}

/// From now on, keyboard, mouse and text events come to window `n` too.
pub fn listen_input(n: i64) {
    let mut all = hub().inboxes.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(inbox) = all.get_mut(&n) {
        inbox.input = true;
    }
}

/// The inside of window `n`, in pixels; `(0, 0)` once it is closed.
pub fn size(n: i64) -> (i64, i64) {
    let all = hub().inboxes.lock().unwrap_or_else(|e| e.into_inner());
    all.get(&n)
        .map_or((0, 0), |i| (i64::from(i.size.0), i64::from(i.size.1)))
}

/// How many pixels make a point on window `n`'s screen.
pub fn scale(n: i64) -> f64 {
    let all = hub().inboxes.lock().unwrap_or_else(|e| e.into_inner());
    all.get(&n).map_or(1.0, |i| i.scale)
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

/// Closes window `n`; closing it again does nothing.
pub fn close(n: i64) {
    tell(Command::Close(n));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_events_come_only_after_listening() {
        let mut inbox = Inbox::default();
        let moved = || WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: winit::dpi::PhysicalPosition::new(3.0, 4.0),
        };
        assert!(translate(moved(), &mut inbox).is_empty());
        inbox.input = true;
        assert_eq!(
            translate(moved(), &mut inbox),
            vec![plain(MOUSE_MOVED, 3.0, 4.0)]
        );
        let resized = WindowEvent::Resized(PhysicalSize::new(640, 480));
        assert_eq!(
            translate(resized, &mut inbox),
            vec![plain(RESIZED, 640.0, 480.0)]
        );
        assert_eq!(inbox.size, (640, 480));
    }

    #[test]
    fn a_window_opens_or_says_why_not() {
        // A machine with no screen (a server, CI) has no windows: that is
        // an answer too, never a hang or a crash.
        match open("kavach test", 64, 48) {
            Ok(n) => {
                assert!(size(n).0 > 0);
                close(n);
            }
            Err(why) => assert!(!why.is_empty()),
        }
    }
}
