use crate::port::{ControlPin, OutputPort};
use crossbeam_channel::{Receiver, Sender};
use spin_sleep::LoopHelper;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

const TIMER_RESOLUTION: f64 = 60.0;

pub enum TimerMessage {
    Started,
    Stopped,
}

pub struct CountDownTimer {
    value: Arc<AtomicU8>,
    stop: ControlPin,
    ticker: JoinHandle<()>,
    sender: Sender<TimerMessage>,
    receiver: Receiver<TimerMessage>,
}

impl Default for CountDownTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// This is an atomic counting down timer.
/// When instantiated, it is originally at 0.
/// When the value is updated, the timer is started
/// and the value decremented at 60 Hz until it reaches 0.
/// It is possible to update the value while the timer is running.
impl CountDownTimer {
    pub fn new() -> Self {
        let value = Arc::new(AtomicU8::new(0));
        let value_clone = Arc::clone(&value);

        let stop = ControlPin::default();
        let stop_clone = stop.clone();

        let (s, r) = crossbeam_channel::bounded(1);
        let s_clone = s.clone();

        let ticker = thread::spawn(move || {
            let mut loop_helper = LoopHelper::builder().build_with_target_rate(TIMER_RESOLUTION);

            loop {
                thread::park();
                loop {
                    let _ = loop_helper.loop_start();
                    let r = value_clone.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                        if v == 0 {
                            None
                        } else {
                            Some(v - 1)
                        }
                    });
                    // we reached 0, exit the timer loop, wait for the next wakeup
                    if r.is_err() {
                        let _ = s_clone.try_send(TimerMessage::Stopped);
                        break;
                    }
                    loop_helper.loop_sleep();
                }
                if stop_clone.is_raised() {
                    break;
                }
            }
        });

        Self {
            value,
            stop,
            ticker,
            sender: s,
            receiver: r,
        }
    }

    pub fn update(&self, val: u8) {
        self.value.store(val, Ordering::Relaxed);
        if val != 0 {
            self.ticker.thread().unpark();
            let _ = self.sender.try_send(TimerMessage::Started);
        }
    }
}

pub(crate) trait ObservableTimer {
    fn value(&self) -> u8;
}

impl ObservableTimer for CountDownTimer {
    fn value(&self) -> u8 {
        self.value.load(Ordering::Relaxed)
    }
}

impl OutputPort<TimerMessage> for CountDownTimer {
    fn output(&self) -> Receiver<TimerMessage> {
        self.receiver.clone()
    }
}

/// Drop implementation for the DelayTimer so that we force
/// the timer to stop when the instance is dropped.
impl Drop for CountDownTimer {
    fn drop(&mut self) {
        self.stop.raise();
        self.update(0);
        self.ticker.thread().unpark();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::time::{Duration, Instant};

    #[test]
    fn timer_works() {
        let t = CountDownTimer::new();

        t.update(10);
        thread::sleep(Duration::from_secs(1));
        assert_eq!(t.value(), 0);

        t.update(10);
        thread::sleep(Duration::from_secs(1));
        assert_eq!(t.value(), 0);
    }

    #[test]
    fn timer_is_accurate() {
        let t = CountDownTimer::new();
        let now = Instant::now();
        t.update(60);
        loop {
            let v = t.value();
            thread::sleep(Duration::from_millis(2));
            if v == 0 {
                break;
            }
        }
        let elapsed = now.elapsed().as_secs_f64();
        assert_relative_eq!(elapsed, 1.0, epsilon = 0.01);
    }
}
