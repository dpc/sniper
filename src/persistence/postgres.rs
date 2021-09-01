use super::*;

#[derive(Debug, Clone)]
pub struct PostgresPersistence {
    pool: r2d2::Pool<r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>>,
}

impl Persistence for PostgresPersistence {
    type Connection = PostgresConnection;

    fn get_connection(&self) -> Result<Self::Connection> {
        Ok(PostgresConnection{ client: self.pool.get()?})
    }
}

pub struct PostgresConnection {
    pub client: r2d2::PooledConnection<r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>>,
}

impl Connection for PostgresConnection {
    type Transaction<'a> = PostgresTransaction<'a>;

    fn start_transaction<'a>(&'a mut self) -> Result<Self::Transaction<'a>> {
        Ok(PostgresTransaction{
            transaction: self.client.transaction()?,
        })
    }
}

pub struct PostgresTransaction<'a> {
    transaction: ::postgres::Transaction<'a>
}

impl<'a> Transaction for PostgresTransaction<'a> {
    fn commit(self) -> Result<()> {
        Ok(self.transaction.commit()?)
    }

    fn rollback(self) -> Result<()> {
        Ok(self.transaction.rollback()?)
    }
}
