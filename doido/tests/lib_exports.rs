// Compile-time tests: verify that the doido umbrella crate re-exports
// public APIs from every sibling crate.

#[test]
fn core_result_type_is_exported() {
    let _: doido::Result<()> = Ok(());
}

#[test]
fn middleware_stack_is_exported() {
    let _: fn() -> doido::MiddlewareStack = doido::MiddlewareStack::new;
}

#[test]
fn generator_types_are_exported() {
    fn _accepts_generator<G: doido::Generator>() {}
    let _f = doido::GeneratedFile {
        path: "test.rs".into(),
        content: "".into(),
    };
}

#[test]
fn cache_store_trait_is_exported() {
    fn _accepts_store<S: doido::store::CacheStore>() {}
}

#[test]
fn mailer_deliverer_is_exported() {
    fn _accepts_deliverer<D: doido::Deliverer>() {}
    let _: doido::LogDeliverer = doido::LogDeliverer;
}
