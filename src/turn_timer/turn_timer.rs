pub use crate::turn_timer::observer::{Notifier, Subscriber};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub enum TimerStatus {
    TimerNotComplete,
    TimerComplete,
}

pub struct TurnTimer {
    timer_duration: u64,
    subscribers: Vec<mpsc::Sender<TimerStatus>>,
}
impl TurnTimer {
    pub fn new(timer_duration: u64) -> TurnTimer {
        Self {
            timer_duration: timer_duration,
            subscribers: Vec::new(),
        }
    }
    pub fn run_timer(self) {
        // set up timer to accept input for
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(self.timer_duration));
            self.notify(&TimerStatus::TimerComplete);
        });
    }
}

impl Notifier<TimerStatus> for TurnTimer {
    fn subscribers(&self) -> &Vec<mpsc::Sender<TimerStatus>> {
        return &self.subscribers;
    }
    fn set_subscribers(&mut self) -> &mut Vec<mpsc::Sender<TimerStatus>> {
        return &mut self.subscribers;
    }
}

pub struct TurnTimerSubscriber {
    timer_status: TimerStatus,
    subscription: Option<mpsc::Receiver<TimerStatus>>,
}
impl TurnTimerSubscriber {
    pub fn new() -> TurnTimerSubscriber {
        Self {
            timer_status: TimerStatus::TimerNotComplete,
            subscription: None,
        }
    }
    pub fn get_timer_status(&mut self) -> TimerStatus {
        match self.timer_status {
            TimerStatus::TimerComplete => return TimerStatus::TimerComplete,
            TimerStatus::TimerNotComplete => {
                self.update();
                return self.timer_status.clone();
            }
        }
    }
}
impl Subscriber<TimerStatus> for TurnTimerSubscriber {
    fn update(&mut self) {
        if let Some(subscription) = &self.subscription {
            if let Ok(TimerStatus::TimerComplete) = subscription.try_recv() {
                self.timer_status = TimerStatus::TimerComplete;
            }
        }
    }
    fn add_subscription(&mut self, reciever: mpsc::Receiver<TimerStatus>) {
        self.subscription = Some(reciever);
    }
}
