use tokio_fsm::{Transition, fsm};

pub struct NonClonePayload(String);

#[fsm(initial = Idle)]
impl NonClonePayloadFsm {
    type Context = ();
    type Error = std::convert::Infallible;

    #[on(state = Idle, event = Start)]
    async fn start(&mut self, payload: NonClonePayload) -> Transition<Done> {
        let _ = payload;
        Transition::to(Done)
    }
}

fn main() {
    let _ = NonClonePayload(String::new());
}
