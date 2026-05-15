use tokio_fsm::fsm;

#[fsm(initial = Idle)]
impl DuplicateTransitionFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start)]
    async fn first(&mut self) -> tokio_fsm::Transition<One> {
        tokio_fsm::Transition::to(One)
    }

    #[on(state = Idle, event = Start)]
    async fn second(&mut self) -> tokio_fsm::Transition<Two> {
        tokio_fsm::Transition::to(Two)
    }
}

fn main() {}
