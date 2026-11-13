# Changelog — doido-auth

All notable changes to `doido-auth` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every crate in the `doido` workspace shares one version; the release process is
documented in `RELEASING.md` at the repository root.

> Unified authentication: extractors, cookie/JWT/OAuth, optional 2FA (Devise analogue).

## Unreleased

### Changed

- **Breaking:** `AuthUser` / `RegisterableAuthUser` replaced by [`UserModel`](src/user_model/mod.rs) / [`RegisterableUserModel`](src/user_model/mod.rs) with a storage-agnostic [`UserStore`](src/user_model/store.rs).
- Devise modules (`lockable`, `confirmable`, `trackable`, `recoverable`, `rememberable`, `validatable`, `timeoutable`) moved under [`src/middleware/`](src/middleware/) and implement the registrable [`AuthMiddleware`](src/middleware/trait_.rs) pipeline ([`register_middleware`](src/middleware/registry.rs)).
- Sign-in validation uses a composable [`FetchPlan`](src/user_model/plan.rs) so enabled middleware columns load in **one** `UserStore::fetch_by_email` call.
- `auth.user_store` config (`sea_orm` default, optional `redis` with feature `auth-redis`).
- `sign_out` is async and accepts an optional email (remember-me cleanup).
- Built-in HTTP surface moved under [`src/controllers/`](src/controllers/) (`*_controller.rs`, `layer`, `extractors`, `state`, `messages`). Crate-root `layer`, `extractors`, `state`, `user`, and `oauth` modules are compatibility shims.
- OAuth providers and `OmniauthableMiddleware` live in [`middleware/omniauthable.rs`](src/middleware/omniauthable.rs); legacy `src/oauth.rs` and `src/handlers.rs` removed.
- 2FA logic in `middleware/two_factorable.rs` (feature `auth-2fa`).

### Added

- [`register_middleware`](src/middleware/registry.rs) for app-defined [`AuthMiddleware`](src/middleware/trait_.rs) hooks; docs include custom strategy (`X-Api-Key`) and audit middleware examples.

## 0.0.32-test2 - 2026-10-15

_No user-facing changes yet._

## 0.0.32-test2 - 2026-10-15

_No user-facing changes yet._

## 0.0.32-test1 - 2026-10-14

_No user-facing changes yet._

## 0.0.32 - 2026-10-13

_No user-facing changes yet._

## 0.0.24 - 2026-08-14

- Baseline changelog. Releases up to and including `0.0.24` are recorded only in
  the git tags (`v0.0.*`) and their GitHub Releases; per-crate history starts here.
