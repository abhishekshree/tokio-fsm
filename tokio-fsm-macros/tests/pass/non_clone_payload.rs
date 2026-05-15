use tokio_fsm::fsm;

pub struct NonClonePayload(String);

#[fsm(initial = Idle)]
impl NonClonePayloadFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start, next = Done)]
    async fn start(&mut self, payload: NonClonePayload) {
        let _ = payload;
    }
}

fn main() {
    let _ = NonClonePayload(String::new());
}
