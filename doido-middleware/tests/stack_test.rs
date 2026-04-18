use doido_middleware::stack::MiddlewareStack;
use axum::Router;

#[test]
fn test_middleware_stack_builds_without_panic() {
    let app: Router = Router::new();
    let _layered = MiddlewareStack::new().apply(app);
}

#[test]
fn test_middleware_stack_with_cors_builds() {
    let app: Router = Router::new();
    let _layered = MiddlewareStack::new().with_cors().apply(app);
}
