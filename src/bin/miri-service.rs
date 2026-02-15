use niri_ipc::{
    Action, Request,
    socket::Socket,
    state::{EventStreamState, EventStreamStatePart},
};

fn main() {
    let mut event_socket = Socket::connect().expect("Failed to connect to niri_ipc event socket");

    if let Err(e) = event_socket.send(Request::EventStream) {
        eprintln!("Failed to subscribe to event stream: {e}");
        std::process::exit(1);
    }

    let mut action_socket = Socket::connect().expect("Failed to connect niri_ipc action socket");

    let mut state = EventStreamState::default();
    let mut read_next = event_socket.read_events();

    loop {
        let event = read_next().expect("Failed to read event");

        match &event {
            niri_ipc::Event::WindowOpenedOrChanged { window } => {
                println!("[EVENT]: window opened or changed");
            }
            niri_ipc::Event::WindowClosed { id } => {
                println!("[EVENT]: window closed");
            }
            niri_ipc::Event::WindowsChanged { windows } => {
                println!("[EVENT]: windows changed");
            }
            _ => {}
        }

        state.apply(event);
    }
}
