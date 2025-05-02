use core::future::Future;

use tokio_postgres::{Client, Error, GenericClient, Transaction};

use crate::{ConnectionUnit, SavePoint, SavePointUnit, TransactionUnit, Transactor};

pub type PgUnit = Client;
pub type PgError = Error;
pub type PgTrxUnit<'t> = Transaction<'t>;

impl<C: GenericClient> Transactor for C {
    type Transaction<'t> = PgTrxUnit<'t>;
}

impl<C: GenericClient> ConnectionUnit for C {
    type TransactionError = PgError;

    fn transaction(
        &mut self,
    ) -> impl Future<Output = Result<Self::Transaction<'_>, Self::TransactionError>> {
        GenericClient::transaction(self)
    }
}

impl TransactionUnit for PgTrxUnit<'_> {
    type CommitError = PgError;
    type RollbackError = PgError;

    fn commit(self) -> impl Future<Output = Result<(), Self::CommitError>> {
        Transaction::commit(self)
    }

    fn rollback(self) -> impl Future<Output = Result<(), Self::RollbackError>> {
        Transaction::rollback(self)
    }
}

impl SavePoint for PgTrxUnit<'_> {
    type SavePointError = PgError;

    fn save_point<'s>(
        &'s mut self,
        name: &str,
    ) -> impl Future<Output = Result<Self::Transaction<'s>, Self::SavePointError>> {
        self.savepoint(name)
    }
}

impl SavePointUnit for PgTrxUnit<'_> {
    type RollbackToError = PgError;
    type ReleaseError = PgError;

    fn rollback_to(self) -> impl Future<Output = Result<(), Self::RollbackToError>> {
        Transaction::rollback(self)
    }

    fn release(self) -> impl Future<Output = Result<(), Self::ReleaseError>> {
        Transaction::commit(self)
    }
}
