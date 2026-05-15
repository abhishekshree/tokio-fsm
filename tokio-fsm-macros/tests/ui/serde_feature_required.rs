use tokio_fsm::fsm;

#[fsm(initial = Idle, serde = true)]
impl SerdeFeatureRequiredFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start, next = Done)]
    async fn on_start(&mut self) {
    }
}

fn main() {}
