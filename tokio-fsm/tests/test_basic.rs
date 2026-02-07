use tokio_fsm::{fsm, Transition};

#[derive(Debug)]
struct TestContext;

#[derive(Debug)]
enum TestError {
    Test,
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Test")
    }
}

impl std::error::Error for TestError {}

#[fsm(initial = "Idle", channel_size = 10)]
impl TestFsm {
    type Context = TestContext;
    type Error = TestError;

    #[event(Start)]
    async fn handle_start(&mut self) -> Transition<Running> {
        Transition::to(Running)
    }
}

#[test]
fn test_basic() {
    // Just test that it compiles
    let _context = TestContext;
}

