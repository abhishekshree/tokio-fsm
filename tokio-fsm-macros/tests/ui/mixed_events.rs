use tokio_fsm::fsm;

#[fsm(initial = Idle)]
impl MixedEvents {
    type Context = ();
    type Error = ();

    #[on(state = Idle, event = Dummy, next = Running)]
    async fn dummy(&mut self) {
    }

    #[on(state = Idle, event = Start, next = Idle)]
    #[on(state = Running, event = Stop, next = Idle)]
    async fn start_or_stop(&mut self) {
    }
}

fn main() {}
