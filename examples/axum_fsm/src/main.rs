use std::{collections::HashMap, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    sync::Mutex,
    time::{Duration, sleep},
};
use tokio_fsm::fsm;

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

#[fsm(initial = Created, tracing = true, serde = true)]
impl OrderFsm {
    type Context = OrderContext;
    type Error = std::convert::Infallible;

    // 1. Created -> Validated
    #[on(state = Created, event = Validate, next = Validated)]
    async fn handle_validate(&mut self) {
        tracing::info!(id = %self.context.order.id, "Validating order...");
        // Simulate validation logic
        sleep(Duration::from_millis(100)).await;
        tracing::debug!(id = %self.context.order.id, "Order validated");
    }

    // 2. Validated -> Charged
    #[on(state = Validated, event = Charge, next = Charged)]
    async fn handle_charge(&mut self) {
        tracing::info!(id = %self.context.order.id, "Charging order...");
        // Simulate payment processing
        sleep(Duration::from_millis(200)).await;
        tracing::debug!(id = %self.context.order.id, "Payment successful");
    }

    // 3. Charged -> Shipped
    #[on(state = Charged, event = Ship, next = Shipped)]
    async fn handle_ship(&mut self) {
        tracing::info!(id = %self.context.order.id, "Shipping order...");
        // Simulate shipping logic
        sleep(Duration::from_millis(300)).await;
        tracing::debug!(id = %self.context.order.id, "Order shipped");
    }

    // Error handling transitions (simplified for demo)
    #[on(state = Created, event = Error, next = Failed)]
    #[on(state = Validated, event = Error, next = Failed)]
    #[on(state = Charged, event = Error, next = Failed)]
    async fn handle_error(&mut self) {
        tracing::error!("Order {} failed", self.context.order.id);
    }
}

// --- API STATE & ERRORS ---

#[derive(Debug)]
enum AppError {
    NotFound,
    FsmRejected,
    AlreadyExists,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Order not found"),
            AppError::FsmRejected => (StatusCode::CONFLICT, "Order FSM rejected the event"),
            AppError::AlreadyExists => (StatusCode::CONFLICT, "Order already exists"),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

struct AppState {
    // Map of OrderID -> (FSM Handle, FSM Task)
    orders: Mutex<HashMap<String, (OrderFsmHandle, OrderFsmTask)>>,
}

impl AppState {
    async fn apply_event(&self, id: &str, event: OrderFsmEvent) -> Result<(), AppError> {
        let handle = {
            let orders = self.orders.lock().await;
            orders.get(id).map(|(h, _)| h.clone())
        }
        .ok_or(AppError::NotFound)?;

        handle.apply(event).await.map_err(|_| AppError::FsmRejected)?;
        Ok(())
    }
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
) -> Result<impl IntoResponse, AppError> {
    let order = Order {
        id: payload.id.clone(),
        items: payload.items,
        total: payload.total,
    };

    let context = OrderContext { order };
    let (handle, task) = OrderFsm::spawn(context);

    let mut orders = state.orders.lock().await;
    if orders.contains_key(&payload.id) {
        return Err(AppError::AlreadyExists);
    }
    orders.insert(payload.id.clone(), (handle, task));

    Ok((
        StatusCode::CREATED,
        Json(json!({ "status": "Order created" })),
    ))
}

async fn validate_order(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    state.apply_event(&id, OrderFsmEvent::Validate).await?;
    Ok((
        StatusCode::OK,
        Json(json!({ "status": "Validation completed" })),
    ))
}

async fn charge_order(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    state.apply_event(&id, OrderFsmEvent::Charge).await?;
    Ok((
        StatusCode::OK,
        Json(json!({ "status": "Charging completed" })),
    ))
}

async fn ship_order(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    state.apply_event(&id, OrderFsmEvent::Ship).await?;
    Ok((
        StatusCode::OK,
        Json(json!({ "status": "Shipping completed" })),
    ))
}

async fn get_order_status(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let orders = state.orders.lock().await;
    let (handle, _) = orders.get(&id).ok_or(AppError::NotFound)?;
    let state = handle.state();
    Ok((StatusCode::OK, Json(json!({ "state": state }))))
}

async fn stop_order(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let (handle, task) = {
        let mut orders = state.orders.lock().await;
        orders.remove(&id).ok_or(AppError::NotFound)?
    };

    handle.shutdown();
    let _ = task.await; // Wait for the FSM task to finish gracefully

    Ok((
        StatusCode::OK,
        Json(json!({ "status": "Order stopped and cleaned up" })),
    ))
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
        .route("/orders/:id/stop", post(stop_order))
        .route("/orders/:id", get(get_order_status))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
