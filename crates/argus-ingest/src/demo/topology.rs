//! The simulated service topology the demo generator emits telemetry for.

/// A service in the simulated system, with a latency profile and error rate.
#[derive(Debug, Clone, Copy)]
pub struct Service {
    /// `service.name` as it appears on emitted telemetry.
    pub name: &'static str,
    /// The operation name used for this service's span.
    pub op: &'static str,
    /// Typical self-time in milliseconds (work done in this service alone).
    pub base_latency_ms: f64,
    /// Plus or minus jitter applied to the base latency.
    pub jitter_ms: f64,
    /// Per-request probability that this service errors.
    pub error_rate: f64,
    /// Display color, as an RGB triple.
    pub color: (u8, u8, u8),
}

/// A node in the call tree: a service and the downstream services it calls.
#[derive(Debug, Clone)]
pub struct Call {
    pub service: Service,
    pub children: Vec<Call>,
}

impl Call {
    fn leaf(service: Service) -> Self {
        Call {
            service,
            children: Vec::new(),
        }
    }

    fn calling(service: Service, children: Vec<Call>) -> Self {
        Call { service, children }
    }
}

pub const GATEWAY: Service = Service {
    name: "api-gateway",
    op: "GET /checkout",
    base_latency_ms: 6.0,
    jitter_ms: 3.0,
    error_rate: 0.0,
    color: (94, 234, 212),
};
pub const AUTH: Service = Service {
    name: "auth",
    op: "auth.verify",
    base_latency_ms: 13.0,
    jitter_ms: 7.0,
    error_rate: 0.01,
    color: (167, 139, 250),
};
pub const CATALOG: Service = Service {
    name: "catalog",
    op: "catalog.lookup",
    base_latency_ms: 22.0,
    jitter_ms: 12.0,
    error_rate: 0.015,
    color: (96, 165, 250),
};
pub const CART: Service = Service {
    name: "cart",
    op: "cart.get",
    base_latency_ms: 9.0,
    jitter_ms: 5.0,
    error_rate: 0.01,
    color: (52, 211, 153),
};
pub const PAYMENTS: Service = Service {
    name: "payments",
    op: "payments.charge",
    base_latency_ms: 38.0,
    jitter_ms: 16.0,
    error_rate: 0.05,
    color: (251, 191, 36),
};
pub const POSTGRES: Service = Service {
    name: "postgres",
    op: "SELECT orders",
    base_latency_ms: 17.0,
    jitter_ms: 9.0,
    error_rate: 0.03,
    color: (244, 114, 182),
};

/// Every service in the simulated system.
pub const SERVICES: [Service; 6] = [GATEWAY, AUTH, CATALOG, CART, PAYMENTS, POSTGRES];

/// The display color for a service name (falls back to grey if unknown).
pub fn color_for(name: &str) -> (u8, u8, u8) {
    SERVICES
        .iter()
        .find(|service| service.name == name)
        .map_or((148, 148, 148), |service| service.color)
}

/// The canonical request flow: a checkout that fans out through the system.
pub fn checkout_flow() -> Call {
    Call::calling(
        GATEWAY,
        vec![
            Call::leaf(AUTH),
            Call::leaf(CATALOG),
            Call::calling(
                CART,
                vec![Call::calling(PAYMENTS, vec![Call::leaf(POSTGRES)])],
            ),
        ],
    )
}
