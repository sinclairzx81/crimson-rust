# crimson-rs

CSP experiments in the rust programming language.

```rust
mod crimson;

use crimson::{ System, Actor, Sender, Receiver };

type Message = &'static str;

struct A;
impl Actor<Message> for A {
  fn run(&mut self, sender: Sender<Message>, _: Receiver<Message>) {
    sender.send("B", "Hello").unwrap();
    sender.send("B", "World").unwrap();
  }
}

struct B;
impl Actor<Message> for B {
  fn run(&mut self, _: Sender<Message>, receiver: Receiver<Message>) {
    for message in receiver {
      println!("{}", message)
    }
  }
}

fn main() {
  let mut system = System::new();
  system.mount("A", Box::new(A));
  system.mount("B", Box::new(B));
  system.run(|info| println!("{:?}", info));
}
```

### overview

crimson-rs is a small experiment to test various concurrency abstractions over the top of rusts mpsc channels with a particular focus on `csp` - communicating sequential processes. This library offers a simple host (referred to as a `system`) with the processes themselves referred to as `actors`. This library allows one to host a number of actors within a larger system, and allow actors to communicate with each other across a common back plane (faciliated by the system in which they are hosted)

Additionally, this library provides some hooks to explore emergent actor network topologies by having the `system` emit `from -> to` message routing information to the caller (available on `info` above). This information can be used to construct actor dependency graphs, with `from -> to` representative of an edge within the graph, the actors being the nodes. Potentially useful for inspecting and debugging a large actor system.

The code for this library is offered as is for anyone who finds it useful or interesting.


### send

A call to `.send(address)` will result in a message being dispatched to ONE actor at the given `address` in a round robin fashion.

```rust
struct A;
impl Actor<u32> for A {
  fn run(&mut self, sender: Sender<u32>, _: Receiver<u32>) {
    sender.send("B", 1).unwrap(); // to -> 0
    sender.send("B", 1).unwrap(); // to -> 1
    sender.send("B", 1).unwrap(); // to -> 2
  }
}
...
system.mount("B", Box::new(B)); // 0
system.mount("B", Box::new(B)); // 1
system.mount("B", Box::new(B)); // 2
```

```
                  +-----+
           +---+  |     |
         +-| m |->|  B  |  
         | +---+  |     |
         |        +-----+
         |                   
+-----+  |        +-----+
|     |  |        |     |
|  A  |--+        |  B  |  # sent to "one" actor (round-robin)
|     |           |     |
+-----+           +-----+
                             
                  +-----+
                  |     |
                  |  B  |
                  |     |
                  +-----+
```

### publish

A call to `.publish(address)` will result in a message being dispatched to ALL actors sharing that `address`.

```rust
struct A;
impl Actor<u32> for A {
  fn run(&mut self, sender: Sender<u32>, _: Receiver<u32>) {
    sender.publish("B", 1).unwrap();
  }
}
...
system.mount("B", Box::new(B)); // 0
system.mount("B", Box::new(B)); // 1
system.mount("B", Box::new(B)); // 2
```

```
                  +-----+
           +---+  |     |
         +-| m |->|  B  |  
         | +---+  |     |
         |        +-----+
         |                   
+-----+  |        +-----+
|     |  | +---+  |     |
|  A  |--+-| m |->|  B  |  # published to "all" actors (fan-out)
|     |  | +---+  |     |
+-----+  |        +-----+
         |                  
         |        +-----+
         | +---+  |     |
         +-| m |->|  B  |
           +---+  |     |
                  +-----+
```