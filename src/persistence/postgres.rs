use super::*;

#[derive(Debug, Clone)]
pub struct PostgresPersistence {
    pool: r2d2::Pool<r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>>,
}

impl Persistence for PostgresPersistence {
    type Connection = PostgresConnection;
    type Transaction<'a> = PostgresTransaction<'a>;

    fn get_connection(&self) -> Result<Self::Connection> {
        Ok(self.pool.get()?)
    }
}

impl ErasedPersistence for PostgresPersistence {
    fn get_connection(&self) -> Result<Box<dyn ErasedConnection>> {
        Ok(Box::new(self.pool.get()?))
    }
}

pub type PostgresConnection = r2d2::PooledConnection<
    r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>,
>;

impl Connection for PostgresConnection {
    type Transaction<'a> = PostgresTransaction<'a>;
    fn start_transaction<'a>(&'a mut self) -> Result<Self::Transaction<'a>> {
        Ok(self.transaction()?)
    }
}

impl ErasedConnection for PostgresConnection {
    fn start_transaction<'a>(&'a mut self) -> Result<Box<dyn ErasedTransaction<'a> + 'a>> {
        Ok(Box::new(self.transaction()?))
    }
}

pub type PostgresTransaction<'a> = ::postgres::Transaction<'a>;

impl<'a> Transaction<'a> for PostgresTransaction<'a> {
    fn commit(self) -> Result<()> {
        Ok((self as ::postgres::Transaction<'a>).commit()?)
    }

    fn rollback(self) -> Result<()> {
        Ok((self as ::postgres::Transaction<'a>).rollback()?)
    }
}

impl<'a> ErasedTransaction<'a> for PostgresTransaction<'a> {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn commit(self) -> Result<()> {
        <Self as Transaction>::commit(self)
    }

    fn rollback(self) -> Result<()> {
        <Self as Transaction>::rollback(self)
    }
}
