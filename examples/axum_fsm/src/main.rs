use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_fsm::{fsm, Transition};

// --- DOMAIN TYPES ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub items: Vec<String>,
    pub total: u64,
}

#[derive(Debug)]
pub struct OrderContext {
    pub order: Order,
}

// --- FSM DEFINITION ---

#[fsm(initial = Created)]
impl OrderFsm {
    type Context = OrderContext;
    type Error = std::convert::Infallible;

    // 1. Created -> Validated
    #[on(state = Created, event = Validate)]
    async fn handle_validate(&mut self) -> Transition<Validated> {
        tracing::info!(id = %self.context.order.id, "Validating order...");
        // Simulate validation logic
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        tracing::debug!(id = %self.context.order.id, "Order validated");
        Transition::to(Validated)
    }

    // 2. Validated -> Charged
    #[on(state = Validated, event = Charge)]
    async fn handle_charge(&mut self) -> Transition<Charged> {
        tracing::info!(id = %self.context.order.id, "Charging order...");
        // Simulate payment processing
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        tracing::debug!(id = %self.context.order.id, "Payment successful");
        Transition::to(Charged)
    }

    // 3. Charged -> Shipped
    #[on(state = Charged, event = Ship)]
    async fn handle_ship(&mut self) -> Transition<Shipped> {
        tracing::info!(id = %self.context.order.id, "Shipping order...");
        // Simulate shipping logic
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        tracing::debug!(id = %self.context.order.id, "Order shipped");
        Transition::to(Shipped)
    }

    // Error handling transitions (simplified for demo)
    #[on(state = Created, event = Error)]
    #[on(state = Validated, event = Error)]
    #[on(state = Charged, event = Error)]
    async fn handle_error(&mut self) -> Transition<Failed> {
        tracing::error!("Order {} failed", self.context.order.id);
        Transition::to(Failed)
    }
}

// --- API STATE ---

struct AppState {
    // Map of OrderID -> FSM Handle
    // In a real app, you might use a DB and reconstruct FSMs, or use an actor registry.
    // For this demo, we keep handles in memory.
    orders: Mutex<HashMap<String, OrderFsmHandle>>,
}

// --- AXUM HANDLERS ---

#[derive(Deserialize)]
struct CreateOrderRequest {
    id: String,
    items: Vec<String>,
    total: u64,
}

async fn create_order(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateOrderRequest>,
) -> impl IntoResponse {
    let order = Order {
        id: payload.id.clone(),
        items: payload.items,
        total: payload.total,
    };

    let context = OrderContext { order };
    let (handle, _) = OrderFsm::spawn(context);

    state.orders.lock().await.insert(payload.id.clone(), handle);

    (StatusCode::CREATED, Json("Order created"))
}

async fn validate_order(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut orders = state.orders.lock().await;
    if let Some(handle) = orders.get_mut(&id) {
        if handle.send(OrderFsmEvent::Validate).await.is_ok() {
            return (StatusCode::OK, Json("Validation started"));
        }
    }
    (StatusCode::NOT_FOUND, Json("Order not found or closed"))
}

async fn charge_order(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut orders = state.orders.lock().await;
    if let Some(handle) = orders.get_mut(&id) {
        if handle.send(OrderFsmEvent::Charge).await.is_ok() {
            return (StatusCode::OK, Json("Charging started"));
        }
    }
    (StatusCode::NOT_FOUND, Json("Order not found or closed"))
}

async fn ship_order(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut orders = state.orders.lock().await;
    if let Some(handle) = orders.get_mut(&id) {
        if handle.send(OrderFsmEvent::Ship).await.is_ok() {
            return (StatusCode::OK, Json("Shipping started"));
        }
    }
    (StatusCode::NOT_FOUND, Json("Order not found or closed"))
}

async fn get_order_status(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let orders = state.orders.lock().await;
    if let Some(handle) = orders.get(&id) {
        // tokio-fsm handles expose current_state() synchronously if it's available
        let state = handle.current_state();
        return (StatusCode::OK, Json(format!("{:?}", state)));
    }
    (StatusCode::NOT_FOUND, Json("Order not found".to_string()))
}

// --- MAIN ---

#[tokio::main]
async fn main() {
    // 1. Initialize tokio-console and stdout logging
    // Requires RUSTFLAGS="--cfg tokio_unstable"
    use tracing_subscriber::prelude::*;

    tracing_subscriber::registry()
        .with(
            console_subscriber::ConsoleLayer::builder()
                .with_default_env()
                .spawn(),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting Axum FSM Server...");

    let app_state = Arc::new(AppState {
        orders: Mutex::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/orders", post(create_order))
        .route("/orders/:id/validate", post(validate_order))
        .route("/orders/:id/charge", post(charge_order))
        .route("/orders/:id/ship", post(ship_order))
        .route("/orders/:id", get(get_order_status))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
