use crate::controllers::auth;
use crate::controllers::{
    BankAccountsController, BankStatementImportsController, BanksController,
    CategoriesController, CounterpartiesController, DashboardController,
    MeController, ReportsController, TransactionsController,
};
use crate::models::user::Model as User;
use doido::controller::axum::Router;
use doido_auth_macros::routes;
use tower_http::services::ServeDir;

pub fn router() -> Router {
    let app = routes! {
        get!("/", DashboardController::index);
        get!("/me", MeController::show);
        get!("/members", MeController::company_users);
        patch!("/memberships/{id}/salary", MeController::update_salary);

        resources!(bank_accounts, BankAccountsController, collection: [export]);
        resources!(
            bank_statement_imports,
            BankStatementImportsController,
            except: [edit, update, destroy],
            collection: [export]
        );
        resources!(categories, CategoriesController, collection: [export]);
        resources!(counterparties, CounterpartiesController, collection: [export]);
        resources!(transactions, TransactionsController, collection: [export]);
        resources!(
            banks,
            BanksController,
            except: [new, edit, create, update, destroy],
            collection: [export]
        );

        get!("/reports", ReportsController::index);

        auth_routes!(
            User,
            only: [sessions, registrations, passwords],
            controllers: {
                sessions: auth::SessionsController,
                registrations: auth::RegistrationsController,
                passwords: auth::PasswordsController,
            }
        );
        post!("/users/sign_out", auth::SessionsController::destroy);
    };

    app.nest_service("/assets", ServeDir::new("public/assets"))
}
