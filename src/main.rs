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

mod crimson;

use crimson::{ System, Actor, Sender, Receiver };

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