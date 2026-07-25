//! Active Record-style lifecycle callbacks.
//!
//! A model implements [`Callbacks`] (all hooks default to no-op) and persists
//! through [`run_create`]/[`run_update`]/[`run_destroy`], which fire the hooks in
//! Rails' order around the supplied persistence step. A hook returning `Err`
//! aborts before the record is written.

use doido_core::Result;

/// Lifecycle hooks fired around persistence. Override the ones you need.
pub trait Callbacks {
    fn before_validation(&mut self) -> Result<()> {
        Ok(())
    }
    fn before_save(&mut self) -> Result<()> {
        Ok(())
    }
    fn after_save(&mut self) -> Result<()> {
        Ok(())
    }
    fn before_create(&mut self) -> Result<()> {
        Ok(())
    }
    fn after_create(&mut self) -> Result<()> {
        Ok(())
    }
    fn before_update(&mut self) -> Result<()> {
        Ok(())
    }
    fn after_update(&mut self) -> Result<()> {
        Ok(())
    }
    fn before_destroy(&mut self) -> Result<()> {
        Ok(())
    }
    fn after_destroy(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Insert: `before_validation → before_save → before_create → persist →
/// after_create → after_save`.
pub async fn run_create<M: Callbacks>(
    model: &mut M,
    persist: impl AsyncFnOnce(&mut M) -> Result<()>,
) -> Result<()> {
    model.before_validation()?;
    model.before_save()?;
    model.before_create()?;
    persist(model).await?;
    model.after_create()?;
    model.after_save()?;
    Ok(())
}

/// Update: `before_validation → before_save → before_update → persist →
/// after_update → after_save`.
pub async fn run_update<M: Callbacks>(
    model: &mut M,
    persist: impl AsyncFnOnce(&mut M) -> Result<()>,
) -> Result<()> {
    model.before_validation()?;
    model.before_save()?;
    model.before_update()?;
    persist(model).await?;
    model.after_update()?;
    model.after_save()?;
    Ok(())
}

/// Destroy: `before_destroy → persist → after_destroy`.
pub async fn run_destroy<M: Callbacks>(
    model: &mut M,
    persist: impl AsyncFnOnce(&mut M) -> Result<()>,
) -> Result<()> {
    model.before_destroy()?;
    persist(model).await?;
    model.after_destroy()?;
    Ok(())
}
