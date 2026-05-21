//! The entity that produces telemetry — a service, host, or process.

use serde::{Deserialize, Serialize};

use crate::attributes::{AttributeValue, Attributes};

/// Well-known resource attribute keys, from the OpenTelemetry semantic
/// conventions.
pub mod keys {
    pub const SERVICE_NAME: &str = "service.name";
    pub const SERVICE_VERSION: &str = "service.version";
    pub const SERVICE_INSTANCE_ID: &str = "service.instance.id";
    pub const HOST_NAME: &str = "host.name";
    pub const DEPLOYMENT_ENVIRONMENT: &str = "deployment.environment";
}

/// The origin of a piece of telemetry: the attributes describing the service,
/// host, or process that emitted it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Resource {
    attributes: Attributes,
}

impl Resource {
    /// An empty resource.
    pub fn new() -> Self {
        Resource {
            attributes: Attributes::new(),
        }
    }

    /// A resource identified by its `service.name`.
    pub fn service(name: impl Into<String>) -> Self {
        Resource {
            attributes: Attributes::new().with(keys::SERVICE_NAME, name.into()),
        }
    }

    /// Set an attribute, returning `self` for chaining.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<AttributeValue>) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// The underlying attributes.
    pub fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    /// The `service.name`, if set.
    pub fn service_name(&self) -> Option<&str> {
        self.attributes.get_str(keys::SERVICE_NAME)
    }

    /// The `host.name`, if set.
    pub fn host_name(&self) -> Option<&str> {
        self.attributes.get_str(keys::HOST_NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_helpers_read_back() {
        let resource = Resource::service("payments")
            .with(keys::HOST_NAME, "node-7")
            .with(keys::DEPLOYMENT_ENVIRONMENT, "production");

        assert_eq!(resource.service_name(), Some("payments"));
        assert_eq!(resource.host_name(), Some("node-7"));
        assert_eq!(
            resource.attributes().get_str(keys::DEPLOYMENT_ENVIRONMENT),
            Some("production")
        );
    }

    #[test]
    fn empty_resource_has_no_service_name() {
        assert_eq!(Resource::new().service_name(), None);
    }
}
