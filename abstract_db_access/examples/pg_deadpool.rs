use core::future::Future;

use abstract_db_access::{
    postgres_tokio::{PgError, PgUnit},
    ConnectionUnit, TransactionUnit,
};
use tokio_postgres::GenericClient;
use utilities::{connection, database};

#[derive(Debug, PartialEq)]
struct User {
    id: uuid::Uuid,
    name: String,
    email: String,
}

impl From<tokio_postgres::Row> for User {
    fn from(row: tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            name: row.get("name"),
            email: row.get("email"),
        }
    }
}

async fn insert_user(client: &impl GenericClient, user: &User) -> Result<(), PgError> {
    client
        .query(
            "INSERT INTO public.user (id, name, email) VALUES ($1, $2, $3)",
            &[&user.id, &user.name, &user.email],
        )
        .await?;
    Ok(())
}

async fn find_user(client: &impl GenericClient, id: uuid::Uuid) -> Result<Option<User>, PgError> {
    let row = client
        .query_opt(
            "SELECT (id, name, email) FROM public.user WHERE user.id = $1",
            &[&id],
        )
        .await?;

    Ok(row.map(User::from))
}

trait UserRepository {
    async fn insert(&self, user: &User) -> Result<(), PgError>;
    async fn find(&self, id: uuid::Uuid) -> Result<Option<User>, PgError>;
}

impl<C: GenericClient> UserRepository for C {
    fn insert(&self, user: &User) -> impl Future<Output = Result<(), PgError>> {
        insert_user(self, user)
    }

    fn find(&self, id: uuid::Uuid) -> impl Future<Output = Result<Option<User>, PgError>> {
        find_user(self, id)
    }
}

async fn multi_repo_transaction(unit: &mut PgUnit, user: &User) -> Result<(), PgError> {
    // TODO: improve example
    let trx = ConnectionUnit::transaction(unit).await?;

    UserRepository::insert(&trx, user).await?;

    trx.commit().await?;

    Ok(())
}

async fn multi_repo(unit: &PgUnit, user: &User) -> Result<(), PgError> {
    // TODO: improve example
    UserRepository::insert(unit, user).await?;

    Ok(())
}

async fn generic_function<Unit, Trx>(unit: &mut Unit, user: &User) -> Result<(), PgError>
where
    Unit: for<'trx> ConnectionUnit<Transaction<'trx> = Trx, TransactionError = PgError>,
    Unit: UserRepository,
    Trx: TransactionUnit<CommitError = PgError, RollbackError = PgError>,
    Trx: UserRepository,
{
    let trx = unit.transaction().await?;

    UserRepository::insert(&trx, user).await?;

    trx.commit().await?;

    let restored_user = UserRepository::find(unit, user.id).await?;

    assert_eq!(restored_user.as_ref(), Some(user));

    Ok(())
}

fn make_user(id: u32) -> User {
    User {
        id: uuid::Uuid::now_v7(),
        email: format!("rustac{id}@email.com"),
        name: format!("Rustacean {id}"),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut users_iter = (0..).map(make_user);

    let pool = connection::create_pg_deadpool();

    const SETUP_DB_COMMAND: &str = concat!(
        "DROP SCHEMA IF EXISTS public CASCADE;\n",
        "CREATE SCHEMA IF NOT EXISTS public;\n",
        "SET search_path TO public;\n",
        include_str!("dbschema.sql")
    );

    database::deadpool_setup_schema(&pool, SETUP_DB_COMMAND).await;

    {
        let client = pool.get().await.unwrap();
        multi_repo(&client, &users_iter.next().unwrap())
            .await
            .unwrap();
    }

    {
        let mut client = pool.get().await.unwrap();
        multi_repo_transaction(&mut client, &users_iter.next().unwrap())
            .await
            .unwrap();
    }

    // NOTE: HRTB issue
    // {
    //     let user = users_iter.next().unwrap();

    //     let mut client = pool.get().await.unwrap();
    //     let unit: &mut PgUnit = client.as_mut();

    //     generic_function(unit, &user).await.unwrap();
    // }
}
