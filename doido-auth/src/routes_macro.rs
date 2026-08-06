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
    (@collect [ $( $collected:tt )* ]) => {
        doido_controller::routes! {
            $( $collected )*
        }
    };
    (
        @collect [ $( $collected:tt )* ]
        auth_routes!( $( $auth:tt )* );
        $( $rest:tt )*
    ) => {
        $crate::routes! {
            @collect [ $( $collected )* $crate::__auth_routes_decl!( $( $auth )* ); ]
            $( $rest )*
        }
    };
    (
        @collect [ $( $collected:tt )* ]
        $first:ident ! ( $( $inner:tt )* ) ;
        $( $rest:tt )*
    ) => {
        $crate::routes! {
            @collect [ $( $collected )* $first ! ( $( $inner )* ) ; ]
            $( $rest )*
        }
    };
    (
        @collect [ $( $collected:tt )* ]
        $first:ident ! ( $( $inner:tt )* )
        $( $rest:tt )*
    ) => {
        $crate::routes! {
            @collect [ $( $collected )* $first ! ( $( $inner )* ) ; ]
            $( $rest )*
        }
    };
    ( $( $rest:tt )* ) => {
        $crate::routes! { @collect [] $( $rest )* }
    };
}
