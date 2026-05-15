use tokio_fsm::fsm;

#[fsm(initial = Idle)]
impl FirstFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start, next = Done)]
    async fn start(&mut self) {
    }
}

#[fsm(initial = Idle)]
impl SecondFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start, next = Done)]
    async fn start(&mut self) {
    }
}

fn main() {}
