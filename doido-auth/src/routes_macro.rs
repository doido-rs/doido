//! Devise-style `routes!` wrapper — expands `auth_routes!` then delegates to
//! `doido_controller::routes!`.

/// Application route table with Devise-style `auth_routes!` (like `devise_for`).
///
/// ```ignore
/// doido_auth::routes! {
///     auth_routes!(User);
///     auth_routes!(User, only: [sessions], controllers: { sessions: MySessionsController });
///     get!("/", HomeController::index);
/// }
/// ```
#[macro_export]
macro_rules! routes {
    ( $( @acc $( $acc:tt )* );* auth_routes!($($auth:tt)*); $($rest:tt)* ) => {
        $crate::routes! {
            $( @acc $( $acc )* );*
            @acc $crate::__auth_routes_decl!($($auth)*);
            $($rest)*
        }
    };
    ( $( @acc $( $acc:tt )* );* $first:ident ! ( $($inner:tt)* ) ; $($rest:tt)* ) => {
        $crate::routes! {
            $( @acc $( $acc )* );*
            @acc $first ! ( $($inner)* ) ;
            $($rest)*
        }
    };
    ( $( @acc $( $acc:tt )* );* $first:ident ! ( $($inner:tt)* ) $($rest:tt)* ) => {
        $crate::routes! {
            $( @acc $( $acc )* );*
            @acc $first ! ( $($inner)* ) ;
            $($rest)*
        }
    };
    ( $( @acc $( $acc:tt )* );* ) => {
        doido_controller::routes! {
            $( $( $acc )* )*
        }
    };
    ( $($rest:tt)* ) => {
        $crate::routes! { @acc ; $($rest)* }
    };
}
