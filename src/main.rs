#![allow(dead_code)]

use std::cmp::Reverse;

/// An _event message_ is data that represents an imperative statement about a
/// future event. For our purposes, an event message is completely specified by
/// a _type_ and a _time_.
#[derive(Debug, Clone, Copy, PartialEq)]
struct EventMessage {
    event_message_type: EventMessageType,
    time: Time,
}

/// The `EventMessageType` is one of three possible values:
/// - `Arrive`: Signals the arrival of an item at the queue.
/// - `Serve`: Calls the next buffered item to be served.
/// - `Exit`: Signals the exit of an item from the queue.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EventMessageType {
    Arrive,
    Serve,
    Exit,
}

/// A "newtype" wrapper around a primitive type used to act as a simulation time.
///
/// The use of `u32` as the wrapped type allows us to sort by `Time`
/// with [impunity](https://users.rust-lang.org/t/cannot-sort-floats/35897).
/// If time were represented by a float, we'd have to jump through some extra
/// hoops because of possible NaNs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
struct Time(u32);

/// A priority queue that holds event messages in order of event time.
#[derive(Debug)]
struct EventMessageQueue {
    messages: Vec<EventMessage>,
    size: u32,
}

/// Implementation of the event message priority queue.
///
/// Note: This sorts on every push and is thus extremely inefficient.
impl EventMessageQueue {
    /// Create an empty `EventMessageQueue`.
    fn new() -> Self {
        Self {
            messages: vec![],
            size: 0,
        }
    }

    /// Push a new item to a the message queue.
    fn push(&mut self, message: EventMessage) -> &mut Self {
        self.messages.push(message);
        self.messages.sort_by_key(|e| Reverse(e.time));
        self.size += 1;
        self
    }

    /// Pop the item at the head of the message .
    fn pop(&mut self) -> Option<(EventMessage, &mut Self)> {
        if let Some(e) = self.messages.pop() {
            self.size -= 1;
            Some((e, self))
        } else {
            None
        }
    }

    /// Get the next item in the message queue without modifying the queue.
    fn peek(&self) -> Option<EventMessage> {
        if let Some(&e) = self.messages.last() {
            Some(e)
        } else {
            None
        }
    }
}

/// The system state, which includes the time, buffer and server counts, and
/// static server capacity and duration.
struct State {
    time: Time,
    buffer_count: u32,
    server_count: u32,
    server_capacity: u32,
    server_duration: u32,
}

/// Methods to construct and update the system state.
impl State {
    /// Create an empty queue.
    fn new(server_capacity: u32, server_duration: u32) -> Self {
        Self {
            time: Time(0),
            buffer_count: 0,
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

    // Decrement the server count.
    fn dec_server(&mut self) -> &mut Self {
        self.server_count -= 1;
        self
    }

    /// Check if the queue can serve the next item.
    ///
    /// The buffer must be occupied and the server must be under capacity.
    fn can_serve(&self) -> bool {
        self.buffer_count > 0 && self.server_count < self.server_capacity
    }
}

/// Step the simulation forward by handling the next event message.
///
/// Note: I'm not totally sure why lifetimes are needed here, but I had to
/// appease the compiler.
fn step<'a>(
    emq: &'a mut EventMessageQueue,
    state: &'a mut State,
) -> Option<(&'a mut EventMessageQueue, &'a mut State)> {
    if let Some((event_message, emq)) = emq.pop() {
        let (new_event_messages, state) = handle_message(event_message, state);
        let state = state.set_time(event_message.time);
        let emq = new_event_messages.iter().fold(emq, |acc, &e| acc.push(e));
        Some((emq, state))
    } else {
        None
    }
}

/// Handle the event message by updating the state and creating new followup
/// event messages.
fn handle_message(
    event_message: EventMessage,
    state: &mut State,
) -> (Vec<EventMessage>, &mut State) {
    match event_message.event_message_type {
        EventMessageType::Arrive => (
            vec![EventMessage {
                event_message_type: EventMessageType::Serve,
                time: event_message.time,
            }],
            state.inc_buffer(),
        ),
        EventMessageType::Serve => {
            if state.can_serve() {
                (
                    vec![EventMessage {
                        event_message_type: EventMessageType::Exit,
                        time: Time(event_message.time.0 + state.server_duration),
                    }],
                    state.dec_buffer().inc_server(),
                )
            } else {
                (vec![], state)
            }
        }
        EventMessageType::Exit => (
            vec![EventMessage {
                event_message_type: EventMessageType::Serve,
                time: event_message.time,
            }],
            state.dec_server(),
        ),
    }
}

fn main() {
    // Create an initial queue state
    let state = &mut State::new(2, 10);

    // Create some arrival event messages
    let event_messages: Vec<EventMessage> = (0..10)
        .map(|t| EventMessage {
            event_message_type: EventMessageType::Arrive,
            time: Time(t),
        })
        .collect();

    // Add these event messages to the event message queue
    let emq = &mut EventMessageQueue::new();
    let emq = event_messages.iter().fold(emq, |acc, &em| acc.push(em));

    // Loop on step until there are no event messages left
    println!("\n\n");
    println!("{0: >10} {1: >10} {2: >10}", "Time", "Buffer", "Server");
    while let Some((_emq, state)) = step(emq, state) {
        println!(
            "{0: >10} {1: >10} {2: >10}",
            state.time.0, state.buffer_count, state.server_count
        );
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emq_mechanics() {
        // Define some event messages.
        let event_messages = vec![
            EventMessage {
                event_message_type: EventMessageType::Arrive,
                time: Time(1),
            },
            EventMessage {
                event_message_type: EventMessageType::Arrive,
                time: Time(2),
            },
        ];

        // Instantiate and populate the event message queue.
        // Note that `EventMessage` implements Copy, so we can push it onto the
        // queue without cloning.
        let emq = &mut EventMessageQueue::new();
        let emq = event_messages.iter().fold(emq, |acc, &em| acc.push(em));

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

        // Peek an item and check the size.
        let _ = emq.peek();
        assert_eq!(1, emq.size);
    }

    #[test]
    fn test_state_updates() {
        // Instantiate the state.
        let state = &mut State::new(1, 10);

        // Apply a series of increments and decrements and check the final
        // counts.
        let state = state.inc_buffer().inc_buffer().inc_server().dec_buffer();
        assert_eq!(1, state.buffer_count);
        assert_eq!(1, state.server_count);
    }

    #[test]
    fn test_one_message_one_step() {
        let em = EventMessage {
            event_message_type: EventMessageType::Arrive,
            time: Time(0),
        };
        let emq = &mut EventMessageQueue::new();
        let state = &mut State::new(1, 10);
        let emq = emq.push(em);

        if let Some((emq, state)) = step(emq, state) {
            assert_eq!(1, state.buffer_count);
            if let Some(next_message) = emq.peek() {
                assert_eq!(EventMessageType::Serve, next_message.event_message_type);
            }
        }
    }
}
