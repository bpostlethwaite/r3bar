extern crate r3bar;

use r3bar::sensors::ipc::{Ipc};

#[test]
fn it_works() {
    Ipc{
        socket: test_socket(),
    };
}
