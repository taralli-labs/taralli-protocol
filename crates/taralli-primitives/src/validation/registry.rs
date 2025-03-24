use std::collections::HashMap;

use alloy::primitives::Address;

use crate::{
    intents::{offer::ComputeOffer, request::ComputeRequest, ComputeIntent},
    systems::{SystemId, SystemParams},
    PrimitivesError, Result,
};

use super::{
    offer::{OfferValidationConfig, OfferVerifierConstraints},
    request::{RequestValidationConfig, RequestVerifierConstraints},
    CommonValidationConfig, CommonVerifierConstraints, IntentValidator,
};

/// A trait for validator registries that can validate specific intent types
pub trait ValidatorRegistry {
    type Intent: ComputeIntent;
    type ValidationConfig: CommonValidationConfig;
    type VerifierConstraints: CommonVerifierConstraints;

    /// Get the default config
    fn default_config(&self) -> &Self::ValidationConfig;

    /// Get the default constraints
    fn default_constraints(&self) -> &Self::VerifierConstraints;

    /// Register a validator for a specific system
    fn register<V>(&mut self, system_id: SystemId, validator: V)
    where
        V: IntentValidator<
                Self::Intent,
                ValidationConfig = Self::ValidationConfig,
                VerifierConstraints = Self::VerifierConstraints,
            > + 'static;

    /// Validate an intent
    fn validate(
        &self,
        intent: &Self::Intent,
        latest_timestamp: u64,
        market_address: &Address,
    ) -> Result<()>;
}

/// Concrete implementation of ValidatorRegistry
pub struct StandardValidatorRegistry<I, C, V>
where
    I: ComputeIntent,
    C: CommonValidationConfig,
    V: CommonVerifierConstraints,
{
    validators: HashMap<
        SystemId,
        Box<dyn IntentValidator<I, ValidationConfig = C, VerifierConstraints = V>>,
    >,
    default_config: C,
    default_constraints: V,
}

impl<I, C, V> StandardValidatorRegistry<I, C, V>
where
    I: ComputeIntent,
    C: CommonValidationConfig,
    V: CommonVerifierConstraints,
{
    pub fn new(default_config: C, default_constraints: V) -> Self {
        Self {
            validators: HashMap::new(),
            default_config,
            default_constraints,
        }
    }
}

impl<I, C, V> ValidatorRegistry for StandardValidatorRegistry<I, C, V>
where
    I: ComputeIntent,
    C: CommonValidationConfig,
    V: CommonVerifierConstraints,
{
    type Intent = I;
    type ValidationConfig = C;
    type VerifierConstraints = V;

    fn default_config(&self) -> &Self::ValidationConfig {
        &self.default_config
    }

    fn default_constraints(&self) -> &Self::VerifierConstraints {
        &self.default_constraints
    }

    fn register<Validator>(&mut self, system_id: SystemId, validator: Validator)
    where
        Validator: IntentValidator<
                Self::Intent,
                ValidationConfig = Self::ValidationConfig,
                VerifierConstraints = Self::VerifierConstraints,
            > + 'static,
    {
        self.validators.insert(system_id, Box::new(validator));
    }

    fn validate(
        &self,
        intent: &Self::Intent,
        latest_timestamp: u64,
        market_address: &Address,
    ) -> Result<()> {
        // Get the appropriate validator for this system
        let validator = self.validators.get(&intent.system_id()).ok_or_else(|| {
            PrimitivesError::ValidationError(format!(
                "No validator for system ID: {:?}",
                intent.system_id()
            ))
        })?;

        validator.validate(intent, latest_timestamp, market_address)
    }
}

// Type aliases for common registry types
pub type ComputeRequestValidatorRegistry = StandardValidatorRegistry<
    ComputeRequest<SystemParams>,
    RequestValidationConfig,
    RequestVerifierConstraints,
>;
pub type ComputeOfferValidatorRegistry = StandardValidatorRegistry<
    ComputeOffer<SystemParams>,
    OfferValidationConfig,
    OfferVerifierConstraints,
>;
