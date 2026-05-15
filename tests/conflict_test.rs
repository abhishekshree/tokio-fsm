#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use tokio_fsm::fsm;

    // FSM 1
    #[derive(Debug, Default)]
    pub struct Context1;

    #[fsm(initial = Idle1)]
    impl Fsm1 {
        type Context = Context1;
        type Error = std::convert::Infallible;

        #[on(state = Idle1, event = Start, next = Running1)]
        async fn start(&mut self) {}
    }

    // FSM 2
    #[derive(Debug, Default)]
    pub struct Context2;

    #[fsm(initial = Idle2)]
    impl Fsm2 {
        type Context = Context2;
        type Error = std::convert::Infallible;

        #[on(state = Idle2, event = Start, next = Running2)]
        async fn start(&mut self) {}
    }

    #[tokio::test]
    async fn test_no_conflict() {
        let (handle1, _task1) = Fsm1::spawn(Context1);
        let (handle2, _task2) = Fsm2::spawn(Context2);

        let _s1: Fsm1State = handle1.current_state();
        let _s2: Fsm2State = handle2.current_state();
    }
}
