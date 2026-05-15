use tokio_fsm::fsm;

#[fsm(initial = Idle)]
impl PayloadMismatch {
    type Context = ();
    type Error = ();

    #[on(state = Idle, event = Start, next = Idle)]
    async fn start1(&mut self, data: String) {
        let _ = data;
    }

    #[on(state = Idle, event = Start, next = Idle)]
    async fn start2(&mut self, data: u32) {
        let _ = data;
    }
}

fn main() {}
