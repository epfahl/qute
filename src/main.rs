#![allow(dead_code)]

use std::cmp::Reverse;

/// The system state, which includes the time, buffer and server counts, and
/// static server capacity and duration.
#[derive(Debug)]
struct QueueState {
    time: Time,
    buffer_count: u32,
    buffer_capacity: u32,
    server_count: u32,
    server_capacity: u32,
    server_duration: u32,
}

/// A "newtype" wrapper around a primitive type that represents simulation time.
///
/// The use of `u32` as the wrapped type allows us to sort by `Time`
/// with [impunity](https://users.rust-lang.org/t/cannot-sort-floats/35897).
/// If time were represented by a float (e.g., `f32`), we'd have to jump through
/// some extra hoops because of possible NaNs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
struct Time(u32);

/// Methods to construct and update the system state.
impl QueueState {
    /// Create an empty queue.
    fn new(buffer_capacity: u32, server_capacity: u32, server_duration: u32) -> Self {
        Self {
            time: Time(0),
            buffer_count: 0,
            buffer_capacity,
            server_count: 0,
            server_capacity,
            server_duration,
        }
    }

    /// Set the time.
    fn set_time(&mut self, time: Time) -> &mut Self {
        self.time = time;
        self
    }

    /// Increment the buffer count.
    fn inc_buffer(&mut self) -> &mut Self {
        self.buffer_count += 1;
        self
    }

    /// Decrement the buffer count.
    fn dec_buffer(&mut self) -> &mut Self {
        self.buffer_count -= 1;
        self
    }

    /// Increment the server count.
    fn inc_server(&mut self) -> &mut Self {
        self.server_count += 1;
        self
    }

    /// Decrement the server count.
    fn dec_server(&mut self) -> &mut Self {
        self.server_count -= 1;
        self
    }

    /// Check if the queue can accommodate a newly arrived item.
    ///
    /// This returns `true` if the buffer is under capacity.
    fn can_buffer(&self) -> bool {
        self.buffer_count < self.buffer_capacity
    }

    /// Check if the queue can serve the next item.
    ///
    /// This returns `true` if the buffer is occupied and the server is
    /// under capacity.
    fn can_serve(&self) -> bool {
        self.buffer_count > 0 && self.server_count < self.server_capacity
    }
}

/// An _event message_ is data that represents a statement about a future
/// event. For our purposes, an event message is completely specified by
/// a _type_ and a _time_.
#[derive(Debug, Clone, Copy, PartialEq)]
struct EventMessage {
    event_message_type: EventMessageType,
    time: Time,
}

/// The _event message type_ is one of three possible values:
/// - `Arrive`: Signals the arrival of an item at the queue.
/// - `CallToServe`: Calls the next buffered item to be served.
/// - `Exit`: Signals the exit of an item from the queue.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EventMessageType {
    Arrive,
    CallToServe,
    Exit,
}

/// A priority queue that holds event messages in order of event time.
#[derive(Debug)]
struct EventMessageQueue {
    messages: Vec<EventMessage>,
    size: u32,
}

/// The vent message priority queue, where message at the head of the queue
/// always has the smallest time.
///
/// Note: This implementation sorts on every push and is thus extremely
/// inefficient.
impl EventMessageQueue {
    /// Create an empty message queue.
    fn new() -> Self {
        Self {
            messages: vec![],
            size: 0,
        }
    }

    /// Push a new item onto the message queue.
    fn push(&mut self, message: EventMessage) -> &mut Self {
        self.messages.push(message);
        self.messages.sort_by_key(|e| Reverse(e.time));
        self.size += 1;
        self
    }

    /// Pop the item at the head of the message queue.
    fn pop(&mut self) -> Option<(EventMessage, &mut Self)> {
        if let Some(e) = self.messages.pop() {
            self.size -= 1;
            Some((e, self))
        } else {
            None
        }
    }
}

/// An _event_ is data that represents a declarative statement about something
/// that happened.
///
/// There can be a one-to-one corresponds between an event message and an
/// event, but, in general, multiple events can follow the successful
/// hanlding of a single event message.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Event {
    time: Time,
    event_type: EventType,
}

/// The _event types_ defines here reflect the operations on the `State`.
///
/// While `EventType` can mirror each value of `EventMessageType`
/// (e.g., `EventMessageType::Arive` -> `EventType::Arrived`), event types
/// can be as granular as is needed for logging and analytical purposes.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EventType {
    BufferIncremented,
    BufferDecremented,
    ServerIncremented,
    ServerDecremented,
}

/// The event log is essentially a wrapper around a vector of events. This is
/// implemented as a struct with a single `contents` field to make it easier
/// to add new features later.
#[derive(Debug)]
struct EventLog {
    contents: Vec<Event>,
    size: u32,
}

impl EventLog {
    /// Create an empty log.
    fn new() -> Self {
        Self {
            contents: vec![],
            size: 0,
        }
    }

    /// Add a new event to the log.
    fn push(&mut self, event: Event) -> &mut Self {
        self.contents.push(event);
        self.size += 1;
        self
    }
}

/// Step the simulation forward by handling the next event message.
///
/// Note: I'm not totally sure why lifetimes are needed here, but I had to
/// appease the compiler.
fn step<'a>(
    emq: &'a mut EventMessageQueue,
    queue_state: &'a mut QueueState,
    event_log: &'a mut EventLog,
) -> Option<(
    &'a mut EventMessageQueue,
    &'a mut QueueState,
    &'a mut EventLog,
)> {
    if let Some((event_message, emq)) = emq.pop() {
        let (queue_state, event_messages, events) = handle_message(event_message, queue_state);
        let queue_state = queue_state.set_time(event_message.time);
        let emq = event_messages.iter().fold(emq, |acc, &em| acc.push(em));
        let event_log = events.iter().fold(event_log, |acc, &e| acc.push(e));
        Some((emq, queue_state, event_log))
    } else {
        None
    }
}

/// Handle the event message by updating the state and creating new followup
/// event messages.
fn handle_message(
    event_message: EventMessage,
    queue_state: &mut QueueState,
) -> (&mut QueueState, Vec<EventMessage>, Vec<Event>) {
    match event_message.event_message_type {
        EventMessageType::Arrive => {
            if queue_state.can_buffer() {
                // If an item can be added to the buffer, increment the buffer
                // and create an event message to call for the next item to be
                // served.
                (
                    queue_state.inc_buffer(),
                    vec![EventMessage {
                        event_message_type: EventMessageType::CallToServe,
                        time: event_message.time,
                    }],
                    vec![Event {
                        event_type: EventType::BufferIncremented,
                        time: event_message.time,
                    }],
                )
            } else {
                // If the newly arrived item can't be added to the buffer, the
                // state is unchanged and there are no new messages. Items that
                // can't be buffered are effectively discarded.
                (queue_state, vec![], vec![])
            }
        }
        EventMessageType::CallToServe => {
            if queue_state.can_serve() {
                // Getting this value here to avoid a borrow checker complaint
                let server_duration = queue_state.server_duration;

                // If an item can be served, decrement the buffer, increment
                // the server, and create an exit event message.
                (
                    queue_state.dec_buffer().inc_server(),
                    vec![EventMessage {
                        event_message_type: EventMessageType::Exit,
                        time: Time(event_message.time.0 + server_duration),
                    }],
                    vec![
                        Event {
                            event_type: EventType::BufferDecremented,
                            time: event_message.time,
                        },
                        Event {
                            event_type: EventType::ServerIncremented,
                            time: event_message.time,
                        },
                    ],
                )
            } else {
                // If an item can't be served, the state is unchanged and there
                // are no new messages.
                (queue_state, vec![], vec![])
            }
        }
        EventMessageType::Exit => (
            queue_state.dec_server(),
            vec![EventMessage {
                event_message_type: EventMessageType::CallToServe,
                time: event_message.time,
            }],
            vec![Event {
                event_type: EventType::ServerDecremented,
                time: event_message.time,
            }],
        ),
    }
}

fn main() {
    // Create an initial queue state.
    //
    // CHANGE ME!
    //
    // The position argument to `new` are the buffer capacity,
    // server capacity, and server duration.
    let queue_state = &mut QueueState::new(5, 2, 10);

    // Prime the event message queue with some arrival event messasges.
    //
    // CHANGE ME!
    //
    // Change the number of arrivals.
    let n_arrivals = 10;
    let emq = &mut EventMessageQueue::new();
    let emq = (0..n_arrivals)
        .map(|t| EventMessage {
            event_message_type: EventMessageType::Arrive,
            time: Time(t),
        })
        .fold(emq, |acc, em| acc.push(em));

    // Create an initially empty event log
    let log = &mut EventLog::new();

    // Call `step` in a loop until the message queue is empty
    println!("\n\n");
    println!("{0: >10} {1: >10} {2: >10}", "Time", "Buffer", "Server");
    while let Some((_emq, state, _log)) = step(emq, queue_state, log) {
        println!(
            "{0: >10} {1: >10} {2: >10}",
            state.time.0, state.buffer_count, state.server_count
        );
    }

    // Print the contents of the event log
    println!("\n\n");
    log.contents.iter().for_each(|e| println!("{:?}", e));
}

// Below are some rudimentary unit tests.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emq_mechanics() {
        // Prime the EMQ with a couple messages
        let emq = &mut EventMessageQueue::new();
        let emq = vec![
            EventMessage {
                event_message_type: EventMessageType::Arrive,
                time: Time(1),
            },
            EventMessage {
                event_message_type: EventMessageType::Arrive,
                time: Time(2),
            },
        ]
        .iter()
        .fold(emq, |acc, &em| acc.push(em));

        // Check the initial size.
        assert_eq!(2, emq.size);

        // Pop an item, check the item, and check the resulting size.
        if let Some((e, emq)) = emq.pop() {
            assert_eq!(
                EventMessage {
                    event_message_type: EventMessageType::Arrive,
                    time: Time(1),
                },
                e,
            );
            assert_eq!(1, emq.size);
        }
    }

    #[test]
    fn test_state_updates() {
        // Instantiate the state.
        let state = &mut QueueState::new(5, 1, 10);

        // Apply a series of increments and decrements and check the final
        // counts.
        let state = state.inc_buffer().inc_buffer().inc_server().dec_buffer();
        assert_eq!(1, state.buffer_count);
        assert_eq!(1, state.server_count);
    }

    #[test]
    fn test_event_log() {
        // Add an event to the log and check the size.
        let log = &mut EventLog::new();
        let e = Event {
            time: Time(0),
            event_type: EventType::BufferIncremented,
        };
        let log = log.push(e);
        assert_eq!(1, log.size);
    }

    #[test]
    fn test_one_message_one_step() {
        // Instantiate the EMQ, queue state, and event log.
        let emq = &mut EventMessageQueue::new();
        let state = &mut QueueState::new(5, 1, 10);
        let log = &mut EventLog::new();
        let emq = emq.push(EventMessage {
            event_message_type: EventMessageType::Arrive,
            time: Time(0),
        });

        // Apply `step` once and check the EMQ and log contents.
        if let Some((emq, state, log)) = step(emq, state, log) {
            assert_eq!(1, state.buffer_count);
            if let Some((next_message, _emq)) = emq.pop() {
                assert_eq!(
                    EventMessageType::CallToServe,
                    next_message.event_message_type
                );
                assert_eq!(1, log.size);
            }
        }
    }
}
