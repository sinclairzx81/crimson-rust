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
/// Provides a multi receive select mechanism across
/// the given receiver vec. Interleaves each message
/// for the given receivers back to receiver returned
/// from this function.
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

/// SystemEvent<T>
/// 
/// A general system event emitted during
/// the lifetime of an actor.
pub enum SystemEvent<T> {
  Started(String),
  Message(String, String, T),
  Finished(String)
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
  sender: SyncSender<SystemEvent<T>>
}
impl<T> Sender<T> {
  pub fn new(address: String, sender: SyncSender<SystemEvent<T>>) -> Sender<T> {
    Sender { address, sender }
  }
  pub fn send(&self, to: &str, value: T) -> Result<(), SendError<SystemEvent<T>>>  {
    let from = self.address.clone();
    let to   = to.to_string();
    self.sender.send(SystemEvent::Message(from, to, value))
  }
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

/// actor<T>
/// 
/// Spawns a new actor in a new thread and returns
/// the sender / receiver pairs.
pub fn actor<A, T>(address: String, actor: A) -> (SyncSender<T>, Receiver<SystemEvent<T>>) 
    where  T: Send + 'static,
           A: Actor<T> + Send + 'static {
    let mut actor = Box::new(actor);
    let (tx0, rx0) = sync_channel::<SystemEvent<T>>(1);
    let (tx1, rx1) = sync_channel::<T>(1);
    thread::spawn(move || {
      tx0.send(SystemEvent::Started(address.clone())).unwrap();
      actor.run(Sender::new(address.clone(), tx0.clone()), rx1);
      tx0.send(SystemEvent::Finished(address)).unwrap();
    });
    (tx1, rx0)
}

/// System<T>
/// 
/// An actor host and message routing system. 
/// Responsible for receiving messages from
/// actors and forwarding them to other actors
/// mounted within the system by simple address.
pub struct System<T> {
  receivers: Vec<Receiver<SystemEvent<T>>>,
  senders: HashMap<String, SyncSender<T>>
}
impl<T> System<T> where T: Send + 'static {
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
  pub fn mount<A: Actor<T> + Send + 'static>(&mut self, address: &str, a: A) {
    let address = address.to_string();
    let (tx, rx) = actor(address.clone(), a);
    self.receivers.push(rx);
    self.senders.insert(address.clone(), tx);
  }

  /// runs the system, causing messages
  /// to flow between actors. This function
  /// will block until all actors have run
  /// to completion. messages are emitted
  /// to the given function F.
  pub fn run<F>(self, f: F) where F: Fn(String) -> () {
    for event in select(self.receivers) {
      match event {
        SystemEvent::Started(address) => {
          f(format!("'{}' started", address))
        }
        SystemEvent::Message(_, to, value) => {
          match self.senders.get(&to) {
            None => f(format!("'{}' does not exist", to)),
            Some(ref sender) => match sender.send(value) {
              Err(_) => f(format!("'{}' send error", to)),
              Ok(_) => {},
            },
          }
        },
        SystemEvent::Finished(address) => {
          f(format!("'{}' finished", address))
        }
      }
    }
  }
}
