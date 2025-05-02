#![allow(async_fn_in_trait)]

pub trait Transactor {
    type Transaction<'t>: TransactionUnit;
}

pub trait ConnectionUnit: Transactor {
    type TransactionError;

    /// Creates a new transaction.
    async fn transaction(&mut self) -> Result<Self::Transaction<'_>, Self::TransactionError>;
}

pub trait TransactionUnit {
    type CommitError;
    type RollbackError;

    async fn commit(self) -> Result<(), Self::CommitError>;
    async fn rollback(self) -> Result<(), Self::RollbackError>;
}

pub trait SavePoint: TransactionUnit + Transactor {
    type SavePointError;

    async fn save_point<'s>(
        &'s mut self,
        name: &str,
    ) -> Result<Self::Transaction<'s>, Self::SavePointError>;
}

pub trait SavePointUnit {
    type RollbackToError;
    type ReleaseError;

    async fn rollback_to(self) -> Result<(), Self::RollbackToError>;
    async fn release(self) -> Result<(), Self::ReleaseError>;
}

#[cfg(feature = "postgres_tokio")]
pub mod postgres_tokio;

#[cfg(feature = "postgres_sqlx")]
pub mod postgres_sqlx;

#[cfg(all(feature = "postgres_tokio", feature = "postgres_sqlx"))]
compile_error!("features `postgres_tokio` and `postgres_sqlx` are mutually exclusive");
