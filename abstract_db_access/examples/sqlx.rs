use abstract_db_access::{
    postgres_sqlx::{PgError, PgTrxUnit, PgUnit},
    ConnectionUnit, TransactionUnit,
};
use utilities::{connection, database};

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
struct User {
    id: uuid::Uuid,
    name: String,
    email: String,
}

trait UserRepository {
    async fn insert(&mut self, user: &User) -> Result<(), PgError>;
    async fn find(&mut self, id: uuid::Uuid) -> Result<Option<User>, PgError>;
}

async fn insert_user<'e, E>(executor: E, user: &User) -> Result<(), PgError>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query("INSERT INTO public.user (id, name, email) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(&user.name)
        .bind(&user.email)
        .execute(executor)
        .await?;
    Ok(())
}

async fn find_user<'e, E>(executor: E, id: uuid::Uuid) -> Result<Option<User>, PgError>
where
    E: sqlx::PgExecutor<'e>,
{
    if let Some(user) =
        sqlx::query_as::<_, User>("SELECT (id, name, email) FROM public.user WHERE user.id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await?
    {
        return Ok(Some(user));
    }

    Ok(None)
}

impl UserRepository for PgUnit {
    async fn insert(&mut self, user: &User) -> Result<(), PgError> {
        insert_user(self, user).await
    }

    async fn find(&mut self, id: uuid::Uuid) -> Result<Option<User>, PgError> {
        find_user(self, id).await
    }
}

impl UserRepository for PgTrxUnit<'_> {
    async fn insert(&mut self, user: &User) -> Result<(), PgError> {
        insert_user(self.as_mut(), user).await
    }

    async fn find(&mut self, id: uuid::Uuid) -> Result<Option<User>, PgError> {
        find_user(self.as_mut(), id).await
    }
}

async fn multi_repo_transaction(unit: &mut PgUnit, user: &User) -> Result<(), PgError> {
    // TODO: improve example
    let mut trx = PgUnit::transaction(unit).await?;

    UserRepository::insert(&mut trx, user).await?;

    trx.commit().await?;

    Ok(())
}

async fn multi_repo(unit: &mut PgUnit, user: &User) -> Result<(), PgError> {
    // TODO: improve example
    UserRepository::insert(unit, user).await?;

    Ok(())
}

#[allow(dead_code)]
async fn generic_function<Unit, Trx>(unit: &mut Unit, user: &User) -> Result<(), PgError>
where
    Unit: for<'trx> ConnectionUnit<Transaction<'trx> = Trx, TransactionError = PgError>,
    Unit: UserRepository,
    Trx: TransactionUnit<CommitError = PgError, RollbackError = PgError>,
    Trx: UserRepository,
{
    let mut trx = unit.transaction().await?;

    UserRepository::insert(&mut trx, user).await.unwrap();

    trx.commit().await.unwrap();

    let restored_user = UserRepository::find(unit, user.id).await.unwrap();

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

    let pool = connection::create_sqlx_pool().await;

    const SETUP_DB_COMMAND: &str = concat!(
        "DROP SCHEMA IF EXISTS public CASCADE;\n",
        "CREATE SCHEMA IF NOT EXISTS public;\n",
        "SET search_path TO public;\n",
        include_str!("dbschema.sql")
    );

    database::sqlx_setup_schema(&pool, SETUP_DB_COMMAND).await;

    let mut client = pool.acquire().await.unwrap();

    {
        let user = users_iter.next().unwrap();
        multi_repo(&mut client, &user).await.unwrap();
    }

    {
        let user = users_iter.next().unwrap();
        multi_repo_transaction(&mut client, &user).await.unwrap();
    }

    // NOTE: HRTB issue
    // {
    //     let conn: &mut PgUnit = &mut client;
    //     generic_function(conn, &user).await.unwrap();
    // }
}
