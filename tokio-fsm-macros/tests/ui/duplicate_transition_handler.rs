use tokio_fsm::fsm;

#[fsm(initial = Idle)]
impl DuplicateTransitionFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start, next = One)]
    async fn first(&mut self) {
    }

    #[on(state = Idle, event = Start, next = Two)]
    async fn second(&mut self) {
    }
}

fn main() {}
