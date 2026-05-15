use tokio_fsm::fsm;

#[fsm(initial = Idle)]
impl InvalidReturnTypeFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start, next = Running)]
    async fn on_start(&mut self) -> usize {
        1
    }
}

fn main() {}
