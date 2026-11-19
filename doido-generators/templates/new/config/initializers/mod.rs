//! Application initializers run on startup.
//!
//! Rails analogue: `config/initializers/*`.
//! Initializers run in registration order at startup before the application
//! boots. Use this module to dynamically configure application settings,
//! register custom inflection rules, initialize third-party clients,
//! set up external services, or validate runtime requirements.

use doido::initializers::Initializers;
use doido::Result;

// @generated-modules

/// Run all application initializers and set up dynamic configurations on startup.
#[allow(dead_code)]
pub fn init() -> Result<()> {
    registry().run_all()
}

/// Alias for [`init`].
#[allow(dead_code)]
pub fn run() -> Result<()> {
    init()
}

/// Builds and returns the [`Initializers`] registry for the application.
pub fn registry() -> Initializers {
    let mut inits = Initializers::with_defaults();
    register_initializers(&mut inits);
    inits
}

/// Register application initializers in order.
#[allow(unused_variables)]
fn register_initializers(inits: &mut Initializers) {
    // @generated-initializers
}
