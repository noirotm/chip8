use chip8_system::display::{
    pixel_buffer, DisplayMessage, PixelBuffer, DISPLAY_HEIGHT, DISPLAY_WIDTH,
};
use chip8_system::keyboard::{Key, KeyboardMessage};
use chip8_system::port::{InputPort, OutputPort};
use crossbeam_channel::{Receiver, Sender};
use druid::widget::Align;
use druid::*;
use std::thread;

const SCALING_FACTOR: f64 = 8.0;

pub const UPDATE: Selector<DisplayMessage> = Selector::new("terminal.update");

#[derive(Clone, Data, Lens)]
struct AppState {}

pub struct Terminal {
    app_launcher: AppLauncher<AppState>,
    keyboard_receiver: Receiver<KeyboardMessage>,
    display_sender: Sender<DisplayMessage>,
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

impl Terminal {
    pub fn new() -> Self {
        let (ks, kr) = crossbeam_channel::bounded(128);
        let (ds, dr) = crossbeam_channel::bounded(128);

        let main_window = WindowDesc::new(Align::centered(TerminalWidget::new(ks)))
            .title("Chip-8")
            .window_size((
                DISPLAY_WIDTH as f64 * SCALING_FACTOR + 25.0,
                DISPLAY_HEIGHT as f64 * SCALING_FACTOR + 50.0,
            ))
            .resizable(true);

        let app_launcher = AppLauncher::with_window(main_window);

        // event sink where to push display messages received from the chip8 system
        let event_sink = app_launcher.get_external_handle();

        thread::spawn(move || {
            while let Ok(msg) = dr.recv() {
                event_sink
                    .submit_command(UPDATE, msg, Target::Global)
                    .expect("Failed to submit update command");
            }
        });

        Self {
            app_launcher,
            keyboard_receiver: kr,
            display_sender: ds,
        }
    }

    pub fn run(self) {
        self.app_launcher
            .launch(AppState {})
            .expect("Failed to launch application");
    }
}

impl OutputPort<KeyboardMessage> for Terminal {
    fn output(&self) -> Receiver<KeyboardMessage> {
        self.keyboard_receiver.clone()
    }
}

impl InputPort<DisplayMessage> for Terminal {
    fn input(&self) -> Sender<DisplayMessage> {
        self.display_sender.clone()
    }
}

struct TerminalWidget {
    key_sender: Sender<KeyboardMessage>,
    pixels: PixelBuffer,
}

impl TerminalWidget {
    fn new(key_sender: Sender<KeyboardMessage>) -> Self {
        Self {
            key_sender,
            pixels: pixel_buffer(),
        }
    }
}

impl Widget<AppState> for TerminalWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut AppState, _env: &Env) {
        match event {
            Event::WindowConnected => {
                ctx.request_focus();
                ctx.request_paint();
            }
            Event::KeyDown(k) => {
                //println!("Key Down: {:?}", k);
                if !k.repeat {
                    if let Some(k) = translate_key(&k.key) {
                        let _ = self.key_sender.try_send(KeyboardMessage::down(k));
                    }
                }
            }
            Event::KeyUp(k) => {
                //println!("Key Up: {:?}", k);
                if let Some(k) = translate_key(&k.key) {
                    let _ = self.key_sender.try_send(KeyboardMessage::up(k));
                }
            }
            Event::Command(c) => {
                if let Some(dm) = c.get(UPDATE) {
                    self.pixels = match dm {
                        DisplayMessage::Clear => pixel_buffer(),
                        DisplayMessage::Update(b) => b.clone(),
                    };
                    ctx.request_paint();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &AppState,
        _env: &Env,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &AppState, _data: &AppState, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        _bc: &BoxConstraints,
        _data: &AppState,
        _env: &Env,
    ) -> Size {
        Size::from((
            DISPLAY_WIDTH as f64 * SCALING_FACTOR,
            DISPLAY_HEIGHT as f64 * SCALING_FACTOR,
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &AppState, _env: &Env) {
        let bounds = ctx.size().to_rect();
        ctx.fill(bounds, &Color::BLACK);

        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                let i = DISPLAY_WIDTH * y + x;
                let r = Rect::from((
                    Point::new(x as f64 * SCALING_FACTOR, y as f64 * SCALING_FACTOR),
                    Size::new(SCALING_FACTOR, SCALING_FACTOR),
                ));
                if let Some(true) = self.pixels.get(i).as_deref() {
                    ctx.fill(r, &Color::grey8(200));
                }
            }
        }
    }
}

fn translate_key(k: &KbKey) -> Option<Key> {
    if let KbKey::Character(s) = k {
        match s.as_str() {
            "0" => Some(Key::Key0),
            "1" => Some(Key::Key1),
            "2" => Some(Key::Key2),
            "3" => Some(Key::Key3),
            "4" => Some(Key::Key4),
            "5" => Some(Key::Key5),
            "6" => Some(Key::Key6),
            "7" => Some(Key::Key7),
            "8" => Some(Key::Key8),
            "9" => Some(Key::Key9),
            "a" => Some(Key::KeyA),
            "b" => Some(Key::KeyB),
            "c" => Some(Key::KeyC),
            "d" => Some(Key::KeyD),
            "e" => Some(Key::KeyE),
            "f" => Some(Key::KeyF),
            _ => None,
        }
    } else {
        None
    }
}
