use crossbeam_channel::{Receiver, Sender, TrySendError};
use std::sync::{Arc, RwLock};
use std::thread;

pub trait InputPort<TInput> {
    fn input(&self) -> Sender<TInput>;
}

pub trait OutputPort<TOutput> {
    fn output(&self) -> Receiver<TOutput>;
}

pub type Shared<T> = Arc<RwLock<T>>;

pub trait SharedData<T> {
    fn data(&self) -> Shared<T>;
}

pub struct PortAdapter<TFrom, TInto> {
    input_receiver: Receiver<TFrom>,
    output_sender: Sender<TInto>,
}

impl<TFrom, TInto> PortAdapter<TFrom, TInto>
where
    TFrom: Send + 'static,
    TInto: From<TFrom> + Send + 'static,
{
    fn start(self) {
        thread::spawn(move || {
            self.run();
        });
    }

    fn run(&self) {
        while let Ok(msg) = self.input_receiver.recv() {
            let to = msg.into();
            if let Err(TrySendError::Disconnected(_)) = self.output_sender.try_send(to) {
                break;
            }
        }
    }
}

pub fn connect<F, T, TFrom, TInto>(from: &F, to: &T)
where
    F: OutputPort<TFrom>,
    T: InputPort<TInto>,
    TFrom: Send + 'static,
    TInto: From<TFrom> + Send + 'static,
{
    PortAdapter {
        input_receiver: from.output(),
        output_sender: to.input(),
    }
    .start();
}
