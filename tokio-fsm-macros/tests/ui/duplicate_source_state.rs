use tokio_fsm::fsm;

#[fsm(initial = Idle)]
impl DuplicateSourceStateFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start, next = Running)]
    #[on(state = Idle, event = Start, next = Running)]
    async fn on_start(&mut self) {
    }
}

fn main() {}
