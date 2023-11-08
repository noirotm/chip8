use chip8_system::port::InputPort;
use chip8_system::timer::TimerMessage;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BackendSpecificError, BuildStreamError, FromSample, Sample, SizedSample, Stream};
use crossbeam_channel::Sender;
use std::error::Error;
use std::thread;

pub enum Message {
    Play,
    Pause,
    Stop,
}

impl From<TimerMessage> for Message {
    fn from(m: TimerMessage) -> Self {
        match m {
            TimerMessage::Started => Message::Play,
            TimerMessage::Stopped => Message::Pause,
        }
    }
}

pub struct Beeper {
    sender: Sender<Message>,
}

impl Beeper {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No audio output device")?;
        let config = device.default_output_config()?;

        let (s, r) = crossbeam_channel::unbounded();
        thread::spawn(move || {
            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => Self::create_stream::<f32>(&device, &config.into()),
                cpal::SampleFormat::I16 => Self::create_stream::<i16>(&device, &config.into()),
                cpal::SampleFormat::U16 => Self::create_stream::<u16>(&device, &config.into()),
                sample_format => Err(BuildStreamError::BackendSpecific {
                    err: BackendSpecificError {
                        description: format!("Unsupported sample format '{sample_format}'"),
                    },
                }),
            };

            match stream {
                Ok(stream) => {
                    let _ = stream.pause();

                    loop {
                        match r.recv() {
                            Ok(Message::Play) => {
                                let _ = stream.play();
                            }
                            Ok(Message::Pause) => {
                                let _ = stream.pause();
                            }
                            Ok(Message::Stop) => {
                                let _ = stream.pause();
                                return;
                            }
                            Err(e) => {
                                eprintln!("Receive error: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("BuildStreamError {:?}", e);
                }
            }
        });

        Ok(Self { sender: s })
    }

    pub fn play(&self) {
        self.sender
            .try_send(Message::Play)
            .unwrap_or_else(|e| eprintln!("Send error: {}", e))
    }

    pub fn pause(&self) {
        self.sender
            .try_send(Message::Pause)
            .unwrap_or_else(|e| eprintln!("Send error: {}", e))
    }

    fn create_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
    ) -> Result<Stream, BuildStreamError>
    where
        T: SizedSample + FromSample<f32>,
    {
        let sample_rate = config.sample_rate.0 as f32;
        let channels = config.channels as usize;

        // Produce a sinusoid of maximum amplitude.
        let mut sample_clock = 0f32;
        let mut next_value = move || {
            sample_clock = (sample_clock + 1.0) % sample_rate;
            (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()
        };

        let err_fn = |err| eprintln!("Stream error: {}", err);

        device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                Self::write_data(data, channels, &mut next_value)
            },
            err_fn,
            None,
        )
    }

    fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> f32)
    where
        T: Copy + FromSample<f32>,
    {
        for frame in output.chunks_mut(channels) {
            let value = next_sample().to_sample::<T>();
            for sample in frame.iter_mut() {
                *sample = value;
            }
        }
    }
}

impl InputPort<Message> for Beeper {
    fn input(&self) -> Sender<Message> {
        self.sender.clone()
    }
}

impl Drop for Beeper {
    fn drop(&mut self) {
        self.sender
            .send(Message::Stop)
            .unwrap_or_else(|e| eprintln!("Send error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chip8_system::port::connect;
    use chip8_system::timer::CountDownTimer;
    use std::time::Duration;

    #[test]
    #[ignore]
    fn beeper_works() {
        let b = Beeper::new().unwrap();
        b.play();
        thread::sleep(Duration::from_secs(2));
        b.pause();
        thread::sleep(Duration::from_secs(2));
        b.play();
        thread::sleep(Duration::from_secs(2));
        b.pause();
    }

    #[test]
    #[ignore]
    fn beeper_with_timer_works() {
        let t = CountDownTimer::new();
        let b = Beeper::new().unwrap();
        connect(&t, &b);

        for _ in 0..10 {
            t.update(80);
            thread::sleep(Duration::from_secs(2));
        }
    }
}
