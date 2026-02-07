use tokio_fsm::fsm;

#[fsm(initial = "Idle")]
impl SimpleFsm {
    type Context = ();
    type Error = ();
}

#[test]
fn test_simple() {
    // Just test that it compiles
}

