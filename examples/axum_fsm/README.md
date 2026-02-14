# Axum Order Processing FSM Example

This example demonstrates how to integrate `tokio-fsm` into a real-world web application using [Axum](https://github.com/tokio-rs/axum).

## Architecture

The example implements a simple order processing flow:

- **FSM**: `OrderFsm` manages the state of an individual order.
- **States**: `Created` -> `Validated` -> `Charged` -> `Shipped`.
- **API**: An Axum server maps HTTP endpoints to FSM events.

## Features

- **In-Memory FSMs**: Each order spawns its own background task managed by `tokio-fsm`.
- **Async Logic**: Handlers simulate real-world delays (DB lookups, payment processing).
- **Monitoring**: Integrated with `tokio-console` for deep visibility into task performance.

## Running the Example

### 1. Start the Server
To enable **tokio-console**, the application must be compiled with unstable flags. This allows the console to instrument the Tokio runtime.

```bash
# Set RUST_LOG to see the info/debug logs we added
RUST_LOG=info RUSTFLAGS="--cfg tokio_unstable" cargo run -p axum_fsm
```

### 2. Monitor with tokio-console
In a separate terminal, start the console. It will automatically connect to `http://127.0.0.1:6669`.

```bash
# Install if you haven't: cargo install tokio-console
tokio-console
```

### 3. Run the Stress Test
Drive the state machine with high concurrency and watch the tasks in the console:

```bash
./stress_test.sh
```

## API Endpoints

- `POST /orders`: Create a new order (Payload: `{"id": "...", "items": [...], "total": 100}`)
- `POST /orders/:id/validate`: Drive to `Validated`
- `POST /orders/:id/charge`: Drive to `Charged`
- `POST /orders/:id/ship`: Drive to `Shipped`
- `GET /orders/:id`: Query current state
