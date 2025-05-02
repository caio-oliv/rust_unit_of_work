use core::future::Future;

use sqlx::{Connection, Error, PgConnection, PgTransaction};

use crate::{ConnectionUnit, TransactionUnit, Transactor};

pub type PgUnit = PgConnection;
pub type PgTrxUnit<'trx> = PgTransaction<'trx>;
pub type PgError = Error;

impl Transactor for PgUnit {
    type Transaction<'trx> = PgTrxUnit<'trx>;
}

impl ConnectionUnit for PgUnit {
    type TransactionError = PgError;

    fn transaction(
        &mut self,
    ) -> impl Future<Output = Result<Self::Transaction<'_>, Self::TransactionError>> {
        <PgConnection as Connection>::begin(self)
    }
}

impl TransactionUnit for PgTrxUnit<'_> {
    type CommitError = PgError;
    type RollbackError = PgError;

    fn commit(self) -> impl Future<Output = Result<(), Self::CommitError>> {
        PgTransaction::commit(self)
    }

    fn rollback(self) -> impl Future<Output = Result<(), Self::RollbackError>> {
        PgTransaction::rollback(self)
    }
}

impl Transactor for PgTrxUnit<'_> {
    type Transaction<'trx> = PgTrxUnit<'trx>;
}

// NOTE: https://github.com/launchbadge/sqlx/issues/3634
// impl SavePoint for PgSqlxTrxUnit<'_> {
//     type SavePointError = PgSqlxError;

//     fn save_point<'s>(
//         &'s mut self,
//         name: &str,
//     ) -> impl Future<Output = Result<Self::Transaction<'s>, Self::SavePointError>> {
//         self.begin()
//     }
// }
