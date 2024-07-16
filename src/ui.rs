use crossterm::event::{poll, read, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::sync::mpsc;

use crate::turn_timer::turn_timer::{TimerStatus, TurnTimerSubscriberTrait};

use std::thread;
use std::time::Duration;
// Struct that runs enable_raw_mode on start and disables when it is
// dropped so that it is only active in the scope of the instantiation
struct ScopedRawMode;

impl ScopedRawMode {
    fn new() -> ScopedRawMode {
        enable_raw_mode().expect("Failed to enable raw mode required to display correctly.");
        ScopedRawMode
    }
}

impl Drop for ScopedRawMode {
    fn drop(&mut self) {
        disable_raw_mode()
            .expect("Failed to disable raw mode. Restart terminal to resume normal behaviour.");
    }
}
// TODO: Move the run_user_input_loop fn into a class that implements an interface so
// we don't have to pass in all of these dependencies to this fn.
pub fn timed_user_input<'a, T: CommandCollector, U: TurnTimerSubscriberTrait + Send + 'a>(
    mut turn_timer_subscriber: U,
    command_dispatcher: mpsc::Sender<MoveCommand>,
) {
    // set up thread for getting cli input
    thread::scope(|s| {
        s.spawn(move || {
            let _guard = ScopedRawMode::new();
            let mut command_collector = T::new();
            run_user_input_loop::<T, U>(
                &mut turn_timer_subscriber,
                command_dispatcher,
                command_collector,
            );
        });
    });
}

/// Runs a loop to collect commands from the user while the turn timer
/// is not yet complete. This has been implemented with dependency injection
/// through the use of generics in order to make testing easier.
///
/// Args:
/// turn_timer_subscriber: a mutable reference to an object that
/// implements the TurnTimerSubscriberTrait and the deived trait Send
/// (so that it can be sent into a thread).
/// command_dispatcher: an mpsc::Sender of type MoveCommand, which
/// is used to send the read commands back to the main thread.
/// command_collector: an object that implements the CommandCollector trait. This
/// reference is mutable to make testing easier.
///
/// Edge cases:
/// - Timer never completes: unhandled - user must interrupt program
/// - Get command fails when reading from input
/// - Command is not recognised
/// - Send to main fails
fn run_user_input_loop<'a, T: CommandCollector, U: TurnTimerSubscriberTrait + Send + 'a>(
    turn_timer_subscriber: &mut U,
    command_dispatcher: mpsc::Sender<MoveCommand>,
    mut command_collector: T,
) {
    loop {
        match turn_timer_subscriber.get_timer_status() {
            TimerStatus::TimerComplete => {
                println!("timer complete!");
                return;
            }
            TimerStatus::TimerNotComplete => match command_collector.get_command() {
                Ok(Some(command)) => {
                    println!("{:?}", command);
                    if let Err(error) = command_dispatcher.send(command) {
                        eprint!("{:?}", error.to_string());
                    }
                }
                Ok(None) => (),
                Err(_) => {
                    println!("Invalid command recieved");
                    return;
                }
            },
        }
    }
}

pub trait CommandCollector {
    fn new() -> Self;
    fn get_command(&mut self) -> Result<Option<MoveCommand>, ()>;
}
#[derive(Debug)]
pub enum MoveCommand {
    Left,
    Down,
    Right,
    Clockwise,
    Anticlockwise,
}
pub struct CliCommandCollector {}
impl CommandCollector for CliCommandCollector {
    fn new() -> Self {
        Self {}
    }
    fn get_command(&mut self) -> Result<Option<MoveCommand>, ()> {
        if poll(Duration::from_millis(100)).expect("Poll of CLI buffer failed.") {
            return match read().expect("Read of CLI buffer failed.") {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Down => Ok(Some(MoveCommand::Down)),
                    KeyCode::Left => Ok(Some(MoveCommand::Left)),
                    KeyCode::Right => Ok(Some(MoveCommand::Right)),
                    KeyCode::Char('z') => Ok(Some(MoveCommand::Anticlockwise)),
                    KeyCode::Char('x') => Ok(Some(MoveCommand::Clockwise)),

                    _other => Err(()),
                },
                _other => Err(()),
            };
        }
        return Ok(None);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::terminal::is_raw_mode_enabled;

    #[test]
    fn test_scoped_raw_mode_controls_raw_mode() {
        assert!(!is_raw_mode_enabled().unwrap());
        {
            let _guard = ScopedRawMode::new();
            assert!(is_raw_mode_enabled().unwrap());
        }
        assert!(!is_raw_mode_enabled().unwrap());
    }
    struct TestTurnTimerSubscriber {
        outputs: Vec<TimerStatus>,
    }
    impl TurnTimerSubscriberTrait for TestTurnTimerSubscriber {
        fn get_timer_status(&mut self) -> TimerStatus {
            self.outputs.pop().unwrap_or(TimerStatus::TimerComplete)
        }
    }

    struct TestCommandCollector {
        outputs: Vec<Result<Option<MoveCommand>, ()>>,
    }
    impl CommandCollector for TestCommandCollector {
        fn new() -> Self {
            Self { outputs: vec![] }
        }
        fn get_command(&mut self) -> Result<Option<MoveCommand>, ()> {
            match self.outputs.pop() {
                Some(val) => val,
                None => Ok(None),
            }
        }
    }
    #[test]
    fn test_loop_does_exit_on_invalid_input() {
        let mut test_turn_timer = TestTurnTimerSubscriber {
            outputs: vec![
                TimerStatus::TimerNotComplete,
                TimerStatus::TimerNotComplete,
                TimerStatus::TimerNotComplete,
            ],
        };
        let (command_dispatcher, command_reciever) = mpsc::channel();
        let mut command_collector = TestCommandCollector::new();
        command_collector.outputs.push(Ok(Some(MoveCommand::Down)));
        command_collector.outputs.push(Err(()));

        run_user_input_loop::<TestCommandCollector, TestTurnTimerSubscriber>(
            &mut test_turn_timer,
            command_dispatcher,
            command_collector,
        );
        assert_eq!(
            test_turn_timer.get_timer_status(),
            TimerStatus::TimerNotComplete
        );
    }
    #[test]
    fn test_loop_does_not_exit_on_valid_input() {
        let mut test_turn_timer = TestTurnTimerSubscriber {
            outputs: vec![
                TimerStatus::TimerComplete,
                TimerStatus::TimerComplete,
                TimerStatus::TimerNotComplete,
                TimerStatus::TimerNotComplete,
                TimerStatus::TimerNotComplete,
            ],
        };
        let (command_dispatcher, command_reciever) = mpsc::channel();
        let mut command_collector = TestCommandCollector::new();
        command_collector.outputs.push(Ok(Some(MoveCommand::Down)));

        run_user_input_loop::<TestCommandCollector, TestTurnTimerSubscriber>(
            &mut test_turn_timer,
            command_dispatcher,
            command_collector,
        );
        assert_eq!(test_turn_timer.outputs.len(), 1);
    }
}