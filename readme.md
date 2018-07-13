# crimson-rs

CSP experiments in the rust programming language.

```rust
mod crimson;

use crimson::{System, Actor, Sender, Receiver};

/// (thread 0) A sends.
struct A;
impl Actor<u32> for A {
  fn run(&mut self, sender: Sender<u32>, _: Receiver<u32>) {
    sender.send("B", 1).unwrap();
    sender.send("B", 2).unwrap();
    sender.send("B", 3).unwrap();
  }
}

/// (thread 1) B receives.
struct B;
impl Actor<u32> for B {
  fn run(&mut self, _: Sender<u32>, receiver: Receiver<u32>) {
    for message in receiver {
      println!("{}", message)
    }
  }
}

fn main() {
  let mut system = System::new();
  system.mount("A", A);
  system.mount("B", B);
  system.run(|info| {
    println!("{}", info);
  });
}
```

### overview

crimson-rs is a small experiment to test various abstractions over the top of rusts mpsc sync channels. In this library, actors can be created to be mounted on the system (analogous to typical actor systems) with each actor run within its own thread. The top level system type will facilitate simple named message routing on behalf of the actors housed within. 

This project seeks to compare csp in the context of Rust against similar functionality found in languages such as Go. The code for this library is offered as is for anyone who finds it useful or interesting.