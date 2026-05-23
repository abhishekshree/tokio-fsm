use syn::{Ident, Type};

/// Name of the generated FSM type.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FsmName(pub Ident);

impl From<Ident> for FsmName {
    fn from(name: Ident) -> Self {
        Self(name)
    }
}

impl From<&Ident> for FsmName {
    fn from(name: &Ident) -> Self {
        Self(name.clone())
    }
}

impl AsRef<Ident> for FsmName {
    fn as_ref(&self) -> &Ident {
        &self.0
    }
}

/// Name of an FSM state.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateName(pub Ident);

impl From<Ident> for StateName {
    fn from(name: Ident) -> Self {
        Self(name)
    }
}

impl From<&Ident> for StateName {
    fn from(name: &Ident) -> Self {
        Self(name.clone())
    }
}

impl AsRef<Ident> for StateName {
    fn as_ref(&self) -> &Ident {
        &self.0
    }
}

/// Name of an FSM event.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventName(pub Ident);

impl From<Ident> for EventName {
    fn from(name: Ident) -> Self {
        Self(name)
    }
}

impl From<&Ident> for EventName {
    fn from(name: &Ident) -> Self {
        Self(name.clone())
    }
}

impl AsRef<Ident> for EventName {
    fn as_ref(&self) -> &Ident {
        &self.0
    }
}

/// Macro configuration derived from FSM attributes.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FsmConfig {
    pub initial: StateName,
    pub channel_size: usize,
    pub tracing: bool,
    pub serde: bool,
}

/// Required associated types declared on the FSM impl.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AssociatedTypes {
    pub context: Type,
    pub error: Type,
}

/// Represents a discovered state in the FSM.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    pub name: Ident,
}

/// Represents a discovered event in the FSM.
#[derive(Debug, Clone, PartialEq)]
pub enum EventSpec {
    Unit { name: EventName },
    Payload { name: EventName, ty: Box<Type> },
}

impl EventSpec {
    pub fn from_parts(name: Ident, payload_type: Option<Type>) -> Self {
        let name = EventName(name);
        match payload_type {
            Some(ty) => Self::Payload {
                name,
                ty: Box::new(ty),
            },
            None => Self::Unit { name },
        }
    }

    pub fn name(&self) -> &Ident {
        match self {
            Self::Unit { name } | Self::Payload { name, .. } => name.as_ref(),
        }
    }

    pub fn payload_type(&self) -> Option<&Type> {
        match self {
            Self::Unit { .. } => None,
            Self::Payload { ty, .. } => Some(ty),
        }
    }

    pub fn payload_description(&self) -> String {
        self.payload_type()
            .map(|ty| quote::quote!(#ty).to_string())
            .unwrap_or_else(|| "None".to_string())
    }
}

/// Represents a handler method in the FSM, including all derived semantic
/// fields.
#[derive(Debug, Clone)]
pub struct Handler {
    pub method: syn::ImplItemFn,
    pub event: Option<EventSpec>,
    pub target_states: Vec<State>,
    pub return_kind: Option<HandlerReturnKind>,
    /// Source states this handler is valid in.
    pub source_states: Vec<Ident>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerReturnKind {
    Unit,
    ResultUnit,
    Transition,
    ResultTransitionError,
}

/// The complete FSM structure after parsing and validation.
#[derive(Debug)]
pub struct FsmStructure {
    pub fsm_name: Ident,
    pub initial_state: Ident,
    pub channel_size: usize,
    pub context_type: Type,
    pub error_type: Type,
    pub states: Vec<State>,
    pub events: Vec<EventSpec>,
    pub handlers: Vec<Handler>,
    pub tracing: bool,
    pub serde: bool,
}
