use tokio_fsm::{Transition, fsm};

#[fsm(initial = Idle)]
impl FirstFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start)]
    async fn start(&mut self) -> Transition<Done> {
        Transition::to(Done)
    }
}

#[fsm(initial = Idle)]
impl SecondFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start)]
    async fn start(&mut self) -> Transition<Done> {
        Transition::to(Done)
    }
}

fn main() {}
