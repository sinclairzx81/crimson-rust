/*--------------------------------------------------------------------------

crimson-rs - csp experiments in rust

The MIT License (MIT)

Copyright (c) 2018 Haydn Paterson (sinclair) <haydn.developer@gmail.com>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.

---------------------------------------------------------------------------*/

use std::sync::mpsc::{SyncSender, Receiver, SendError, TryRecvError};
use std::sync::mpsc::sync_channel;
use std::collections::HashMap;
use std::time::Duration;
use std::thread;

/// select<T>
/// 
/// Provides a multi mpsc receiver select() mechanism that
/// selects across the given number of receivers sharing the
/// same type. This function requires 1 thread select and does
/// so in a polling fashion at 1000 nano intervals.
pub fn select<T>(mut receivers: Vec<Receiver<T>>) -> Receiver<T> where T: Send + 'static {
    let (sender, receiver) = sync_channel(1);
    thread::spawn(move || {
        loop {
            receivers = receivers.into_iter().filter_map(|inner| {
              match inner.try_recv() {
                  Ok(value) => match sender.send(value) {
                      Ok(_) => Some(inner),
                      Err(_) => None
                  },
                  Err(error) => match error {
                    TryRecvError::Disconnected => None,
                    TryRecvError::Empty => Some(inner)
                  }
              }
            }).collect::<Vec<_>>();
            if receivers.len() == 0 {
              return
            } else {
              thread::sleep(Duration::from_nanos(1000))
            }
        }
    });
    receiver
}

/// spawn_actor<T>
/// 
/// Spawns a new actor in a new thread and returns
/// the sender / receiver pairs.
pub fn spawn_actor<A, T>(address: String, actor: Box<A>) -> (SyncSender<T>, Receiver<ActorEvent<T>>) 
    where  T: Send + 'static,
           A: Actor<T> + Send + 'static {
    let mut actor = Box::new(actor);
    let (tx0, rx0) = sync_channel::<ActorEvent<T>>(1);
    let (tx1, rx1) = sync_channel::<T>(1);
    thread::spawn(move || {
      tx0.send(ActorEvent::Started(address.clone())).unwrap();
      actor.run(Sender::new(address.clone(), tx0.clone()), rx1);
      tx0.send(ActorEvent::Stopped(address)).unwrap();
    });
    (tx1, rx0)
}


/// ActorEvent<T>
/// 
/// An actor event type. This event type is emitted 
/// during the lifecycle of an actor, and is received
/// by the system to signal actor start, actor stop and
/// actor messages flowing between actors.
pub enum ActorEvent<T> {
  Started(String),
  Send(String, String, T),
  Publish(String, String, T),
  Stopped(String)
}

/// Actor<T>
/// 
/// Base trait for which all other actors should
/// implement. Provides a single input output
/// type which is intended to be a simple message,
/// or an enum for more advance message types.
pub trait Actor<T: Send + 'static> {
  fn run(&mut self, sender: Sender<T>, receiver: Receiver<T>);
}

/// Sender<T>
/// 
/// A custom sender type given to actors to allow
/// them to send messages to other actors in a 
/// system. Implements a simple send function
/// with a address to the recipient actor.
#[derive(Clone)]
pub struct Sender<T> {
  address: String,
  sender: SyncSender<ActorEvent<T>>
}
impl<T> Sender<T> {
  pub fn new(address: String, sender: SyncSender<ActorEvent<T>>) -> Sender<T> {
    Sender { address, sender }
  }
  /// sends a message to 1 actor at the given address.
  pub fn send(&self, to: &str, value: T) -> Result<(), SendError<ActorEvent<T>>>  {
    let from = self.address.clone();
    let to   = to.to_string();
    self.sender.send(ActorEvent::Send(from, to, value))
  }
  /// sends a message to N actors at the given address.
  pub fn publish(&self, to: &str, value: T) -> Result<(), SendError<ActorEvent<T>>>  {
    let from = self.address.clone();
    let to   = to.to_string();
    self.sender.send(ActorEvent::Publish(from, to, value))
  }
}

/// RoundRobin
/// 
/// Simple round-robin sending device. Produces
/// indices which are used to index into recipient
/// actors sharing the same address.
struct RoundRobin {
  counters: HashMap<String, (usize, usize)> // total, current
}
impl RoundRobin {
  /// constructs a round-robin from senders hashmap.
  fn new<T>(senders: &HashMap<String, Vec<SyncSender<T>>>) -> RoundRobin {
    let mut counters = HashMap::new();
    let keys = senders.keys();
    for key in keys.into_iter() {
      let key     = key.clone();
      let vec     = senders.get(&key).unwrap();
      let total   = vec.len();
      let current = vec.len() - 1;
      counters.insert(key.clone(), (total, current));
    }
    RoundRobin { counters }
  }
  /// returns the 'next' index for this address.
  fn next(&mut self, address: String) -> usize {
    match self.counters.remove(&address) {
      Some((total, current)) => {
        let current = current + 1;
        let current = if current >= total { 0 } else { current };
        self.counters.insert(address, (total, current));
        current
      },
      None => 0
    }
  }
}

/// SystemEvent
/// 
/// System event message passed out the system
/// on run(). The system event pushes actor
/// start, stop and forwarding information
/// out to the caller allowing them to inspect
/// the internal actions taking place within
/// the system at a high level.
#[derive(Debug, Clone)]
pub enum SystemEvent {
  Started(String),                // address
  Forward(String, String, usize), // from, to, idx
  Error(String, String),          // address, reason
  Stopped(String)                 // actor
}

/// System<T>
/// 
/// An actor host and message routing system. 
/// Responsible for receiving messages from
/// actors and forwarding them to other actors
/// mounted within the system by simple address.
pub struct System<T> {
  receivers: Vec<Receiver<ActorEvent<T>>>,
  senders: HashMap<String, Vec<SyncSender<T>>>
}
impl<T> System<T> where T: Send + Clone + 'static {
  pub fn new() -> System<T> {
    let receivers = Vec::new();
    let senders = HashMap::new();
    System { receivers, senders }
  }
  /// mounts and spawns an actor causing its
  /// run() function to execute. The actor
  /// will suspend following its first call
  /// to send and resume when the system is
  /// run()
  pub fn mount<A: Actor<T> + Send + 'static>(&mut self, address: &str, a: Box<A>) {
    let address = address.to_string();
    let (sender, receiver) = spawn_actor(address.clone(), a);
    self.receivers.push(receiver);
    if !self.senders.contains_key(&address) {
      self.senders.insert(address.clone(), Vec::new());
    }
    match self.senders.get_mut(&address) {
      Some(ref mut vec) => {
        vec.push(sender)
      },
      None => {}
    }
  }

  /// runs the system, causing messages
  /// to flow between actors. This function
  /// will block until all actors have run
  /// to completion. messages are emitted
  /// to the given function F.
  pub fn run<F>(self, f: F) where F: Fn(SystemEvent) -> () {
    let mut round_robin = RoundRobin::new(&self.senders);
    for event in select(self.receivers) {
      match event {
        // general actor events.
        ActorEvent::Started(address) => f(SystemEvent::Started(address)),
        ActorEvent::Stopped(address) => f(SystemEvent::Stopped(address)),
        // sends to one actor.
        ActorEvent::Send(from, to, value) => {
          f(SystemEvent::Forward(from.clone(), to.clone(), 0));
          match self.senders.get(&to) {
            None => f(SystemEvent::Error(to, format!("does not exist"))),
            Some(ref mut vec) => {
              let sender = &vec[round_robin.next(to.clone())];
              match sender.send(value) {
                Err(_) => f(SystemEvent::Error(to, format!("send error"))),
                Ok(_) => {},
              }
            }
          }
        },
        // sends to many actors
        ActorEvent::Publish(from, to, value) => {
          f(SystemEvent::Forward(from.clone(), to.clone(), 0));
          match self.senders.get(&to) {
            None => f(SystemEvent::Error(to, format!("does not exist"))),
            Some(ref mut vec) => {
              for sender in vec.iter() {
                match sender.send(value.clone()) {
                  Err(_) => f(SystemEvent::Error(to.clone(), format!("send error"))),
                  Ok(_) => {},
                }
              }
            }
          }
        }
      }
    }
  }
}
