mod error_propagation {
    use tokio_fsm::{ApplyError, TaskError, fsm};

    #[derive(Clone, Debug, thiserror::Error)]
    pub enum HandlerError {
        #[error("boom")]
        Boom,
    }

    #[fsm(initial = Idle)]
    impl ErrorPropagationFsm {
        type Context = ();
        type Error = HandlerError;

        #[on(state = Idle, event = Start, next = Running)]
        async fn on_start(&mut self, fail: bool) -> Result<(), HandlerError> {
            if fail {
                return Err(HandlerError::Boom);
            }

            Ok(())
        }
    }

    #[tokio::test]
    async fn test_fsm_handler_error_propagates_to_caller_and_task() {
        let (handle, task) = ErrorPropagationFsm::spawn(());

        assert!(matches!(
            handle.apply(ErrorPropagationFsmEvent::Start(true)).await,
            Err(ApplyError::HandlerFailed(HandlerError::Boom))
        ));

        match task.await {
            Err(TaskError::Fsm(HandlerError::Boom)) => {}
            other => panic!("expected TaskError::Fsm(HandlerError::Boom), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_direct_apply_handler_error_propagates_to_caller() {
        let mut fsm = ErrorPropagationFsm::new(());

        assert!(matches!(
            fsm.apply(ErrorPropagationFsmEvent::Start(true)).await,
            Err(ApplyError::HandlerFailed(HandlerError::Boom))
        ));
    }
}

mod dynamic_transition {
    use std::convert::Infallible;

    use tokio_fsm::{Transition, fsm};

    #[fsm(initial = Idle)]
    impl DynamicTransitionFsm {
        type Context = ();
        type Error = Infallible;

        #[on(state = Idle, event = Start, next = [Running, Failed])]
        async fn on_start(
            &mut self,
            fail: bool,
        ) -> Result<Transition<DynamicTransitionFsmState>, Infallible> {
            if fail {
                return Ok(Transition::to(DynamicTransitionFsmState::Failed));
            }

            Ok(Transition::to(DynamicTransitionFsmState::Running))
        }
    }

    #[tokio::test]
    async fn test_dynamic_transition_selects_declared_target() {
        let (handle, task) = DynamicTransitionFsm::spawn(());

        assert_eq!(
            handle
                .apply(DynamicTransitionFsmEvent::Start(true))
                .await
                .unwrap(),
            DynamicTransitionFsmState::Failed
        );

        handle.shutdown();
        task.await.unwrap();
    }
}

mod invalid_dynamic_transition {
    use std::convert::Infallible;

    use tokio_fsm::{ApplyError, Transition, fsm};

    #[fsm(initial = Idle)]
    impl InvalidDynamicTransitionFsm {
        type Context = ();
        type Error = Infallible;

        #[on(state = Idle, event = Start, next = [Running, Failed])]
        async fn on_start(&mut self) -> Transition<InvalidDynamicTransitionFsmState> {
            Transition::to(InvalidDynamicTransitionFsmState::Other)
        }

        #[on(state = Running, event = Reset, next = Other)]
        async fn on_reset(&mut self) {}
    }

    #[tokio::test]
    async fn test_dynamic_transition_rejects_undeclared_target() {
        let (handle, task) = InvalidDynamicTransitionFsm::spawn(());
        let _ = InvalidDynamicTransitionFsmState::Running;
        let _ = InvalidDynamicTransitionFsmState::Failed;
        let _ = InvalidDynamicTransitionFsmEvent::Reset;

        assert!(matches!(
            handle.apply(InvalidDynamicTransitionFsmEvent::Start).await,
            Err(ApplyError::InvalidTransition {
                state: InvalidDynamicTransitionFsmState::Other,
            })
        ));
        assert_eq!(handle.state(), InvalidDynamicTransitionFsmState::Idle);

        handle.shutdown();
        task.await.unwrap();
    }
}
