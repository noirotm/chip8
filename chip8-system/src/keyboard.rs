use crate::port::InputPort;
use crossbeam_channel::{select, Receiver, Sender};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum KeyState {
    Up,
    Down,
}

#[derive(Copy, Clone, Debug, FromPrimitive, PartialEq, Eq)]
pub enum Key {
    Key0 = 0x0,
    Key1 = 0x1,
    Key2 = 0x2,
    Key3 = 0x3,
    Key4 = 0x4,
    Key5 = 0x5,
    Key6 = 0x6,
    Key7 = 0x7,
    Key8 = 0x8,
    Key9 = 0x9,
    KeyA = 0xa,
    KeyB = 0xb,
    KeyC = 0xc,
    KeyD = 0xd,
    KeyE = 0xe,
    KeyF = 0xf,
}

impl Key {
    pub fn from(v: u8) -> Option<Key> {
        Key::from_u8(v)
    }
}

pub struct KeyboardMessage {
    state: KeyState,
    key: Key,
}

impl KeyboardMessage {
    pub fn new(state: KeyState, key: Key) -> Self {
        Self { state, key }
    }

    pub fn up(key: Key) -> Self {
        Self {
            state: KeyState::Up,
            key,
        }
    }

    pub fn down(key: Key) -> Self {
        Self {
            state: KeyState::Down,
            key,
        }
    }
}

pub(crate) struct KeyboardController {
    stop_waiter_sender: Sender<()>,
}

impl KeyboardController {
    pub fn stop(&self) {
        _ = self.stop_waiter_sender.send(());
    }
}

pub struct Keyboard {
    key_states: Arc<RwLock<[KeyState; 16]>>,
    wait_for_key: Arc<Mutex<bool>>,
    stop_sender: Sender<()>,
    sender: Sender<KeyboardMessage>,
    wait_receiver: Receiver<Key>,
    stop_waiter_receiver: Receiver<()>,
    stop_waiter_sender: Sender<()>,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Keyboard {
    pub(crate) fn new() -> Self {
        // channel for input keyboard messages
        let (sender, receiver) = crossbeam_channel::bounded(128);

        // list of current key states (up or down)
        let key_states = Arc::new(RwLock::new([KeyState::Up; 16]));
        let key_states_clone = Arc::clone(&key_states);

        // are we currently in wait_for_key_press
        let wait_for_key = Arc::new(Mutex::new(false));
        let wait_for_key_clone = Arc::clone(&wait_for_key);

        // channel for stopping the keyboard controller thread
        let (stop_sender, stop_receiver) = crossbeam_channel::bounded(0);

        // channel to send pressed key to the blocked wait_for_key_press function
        let (wait_sender, wait_receiver) = crossbeam_channel::bounded(1);

        // channel to interrupt the wait_for_key_press function
        let (stop_waiter_sender, stop_waiter_receiver) = crossbeam_channel::bounded(1);

        thread::spawn(move || {
            loop {
                select! {
                    recv(stop_receiver) -> _ => {
                        break;
                    }
                    recv(receiver) -> msg => {
                        if let Ok(KeyboardMessage { state, key }) = msg {
                            if let Ok(mut key_states) = key_states_clone.write() {
                                // update key status
                                let idx = key as usize;
                                key_states[idx] = state;

                                // if a key has been pressed and we are waiting for a key press
                                if state == KeyState::Down {
                                    let mut waiting = wait_for_key_clone.lock().unwrap();
                                    if *waiting {
                                        // notify our condition variable
                                        //let (ref key_lock, ref cv) = &*wake_cond_clone;
                                        //*key_lock.lock() = Some(key);
                                        //cv.notify_one();

                                        _ = wait_sender.try_send(key);

                                        // stop waiting
                                        *waiting = false;
                                    }
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        });

        Self {
            key_states,
            wait_for_key,
            //wake_cond,
            stop_sender,
            sender,
            wait_receiver,
            stop_waiter_receiver,
            stop_waiter_sender,
        }
    }

    pub(crate) fn is_key_down(&self, key: Key) -> bool {
        self.key_states
            .read()
            .map(|ks| ks[key as usize] == KeyState::Down)
            .unwrap_or(false)
    }

    pub(crate) fn wait_for_key_press(&self) -> Option<Key> {
        {
            // register wait
            *self.wait_for_key.lock().unwrap() = true;
        }

        // wait for either interruption, or a key press
        select! {
            recv(self.stop_waiter_receiver) -> _ => {
                None
            }
            recv(self.wait_receiver) -> key => {
                key.ok()
            }
        }
    }

    pub(crate) fn controller(&self) -> KeyboardController {
        KeyboardController {
            stop_waiter_sender: self.stop_waiter_sender.clone(),
        }
    }
}

impl Drop for Keyboard {
    fn drop(&mut self) {
        _ = self.stop_sender.try_send(());
    }
}

impl InputPort<KeyboardMessage> for Keyboard {
    fn input(&self) -> Sender<KeyboardMessage> {
        self.sender.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_is_key_down_works() {
        let kb = Keyboard::new();
        let sender = kb.input();

        sender
            .send(KeyboardMessage::new(KeyState::Down, Key::Key0))
            .unwrap();

        thread::sleep(Duration::from_millis(100));

        assert!(kb.is_key_down(Key::Key0));
        assert!(!kb.is_key_down(Key::Key1));

        sender
            .send(KeyboardMessage::new(KeyState::Up, Key::Key0))
            .unwrap();

        thread::sleep(Duration::from_millis(100));

        assert!(!kb.is_key_down(Key::Key0));
        assert!(!kb.is_key_down(Key::Key1));
    }

    #[test]
    fn test_wait_for_key_press_works() {
        let kb = Keyboard::new();
        let sender = kb.input();

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            sender
                .send(KeyboardMessage::new(KeyState::Down, Key::Key0))
                .unwrap();
        });

        let k = kb.wait_for_key_press();
        assert_eq!(k.unwrap(), Key::Key0);
    }
}
