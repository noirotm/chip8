use crate::port::{InputPort, Shared};
use crossbeam_channel::Sender;
use num_derive::FromPrimitive;
use parking_lot::{Condvar, Mutex};
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Copy, Clone, PartialEq)]
pub enum KeyState {
    Up,
    Down,
}

#[derive(Copy, Clone, Debug, FromPrimitive, PartialEq)]
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
    key_states: Shared<[KeyState; 16]>,
    wait_for_key: Arc<Mutex<bool>>,
    wake_cond: Arc<(Mutex<Option<Key>>, Condvar)>,
    sender: Sender<KeyboardMessage>,
}

impl Default for KeyboardController {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardController {
    pub fn new() -> Self {
        let (s, r) = crossbeam_channel::bounded(128);

        let key_states = Arc::new(RwLock::new([KeyState::Up; 16]));
        let key_states_clone = Arc::clone(&key_states);

        let wait_for_key = Arc::new(Mutex::new(false));
        let wait_for_key_clone = Arc::clone(&wait_for_key);

        let wake_cond = Arc::new((Mutex::new(None), Condvar::new()));
        let wake_cond_clone = Arc::clone(&wake_cond);

        thread::spawn(move || {
            while let Ok(KeyboardMessage { state, key }) = r.recv() {
                if let Ok(mut key_states) = key_states_clone.write() {
                    // update key status
                    let idx = key as usize;
                    key_states[idx] = state;

                    // if a key has been pressed and we are waiting for a key press
                    if state == KeyState::Down {
                        let mut waiting = wait_for_key_clone.lock();
                        if *waiting {
                            // notify our condition variable
                            let &(ref key_lock, ref cv) = &*wake_cond_clone;
                            *key_lock.lock() = Some(key);
                            cv.notify_one();

                            // stop waiting
                            *waiting = false;
                        }
                    }
                }
            }
        });

        Self {
            key_states,
            wait_for_key,
            wake_cond,
            sender: s,
        }
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        self.key_states
            .read()
            .map(|ks| ks[key as usize] == KeyState::Down)
            .unwrap_or(false)
    }

    pub fn wait_for_key_press(&self) -> Key {
        {
            // register wait
            *self.wait_for_key.lock() = true;
        }

        // wait!
        let &(ref lock, ref cv) = &*self.wake_cond;
        let mut key_pressed = lock.lock();
        if key_pressed.is_none() {
            cv.wait(&mut key_pressed);
        }
        let k = key_pressed.expect("key cannot be empty");

        // reset key for the next wait
        *key_pressed = None;

        k
    }
}

impl InputPort<KeyboardMessage> for KeyboardController {
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
        let kc = KeyboardController::new();
        let sender = kc.input();

        sender
            .send(KeyboardMessage::new(KeyState::Down, Key::Key0))
            .unwrap();

        thread::sleep(Duration::from_millis(100));

        assert!(kc.is_key_down(Key::Key0));
        assert!(!kc.is_key_down(Key::Key1));

        sender
            .send(KeyboardMessage::new(KeyState::Up, Key::Key0))
            .unwrap();

        thread::sleep(Duration::from_millis(100));

        assert!(!kc.is_key_down(Key::Key0));
        assert!(!kc.is_key_down(Key::Key1));
    }

    #[test]
    fn test_wait_for_key_press_works() {
        let kc = KeyboardController::new();
        let sender = kc.input();

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            sender
                .send(KeyboardMessage::new(KeyState::Down, Key::Key0))
                .unwrap();
        });

        let k = kc.wait_for_key_press();
        assert_eq!(k, Key::Key0);
    }
}
