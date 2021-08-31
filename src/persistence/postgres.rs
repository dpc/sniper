use super::*;
use ::postgres;

#[derive(Debug)]
pub struct PostgresStore {
    pool: r2d2::Pool<r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>>,
}

impl Store for PostgresStore {
    type Connection = PostgresConnection;

    fn get_connection(&mut self) -> Result<Self::Connection> {
        todo!()
    }
}

pub struct PostgresConnection {
    transaction: postgres::Client,
}

impl GenericConnection for PostgresConnection {}

impl Connection for PostgresConnection {
    type Transaction<'a> = PostgresTransaction<'a>;

    fn start_transaction<'a>(&'a mut self) -> Result<Self::Transaction<'a>> {
        Ok(PostgresTransaction{ phantom: std::marker::PhantomData })
    }
}

#[derive(Default, Debug)]
pub struct PostgresTransaction<'a> {
    phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> GenericConnection for PostgresTransaction<'a> {}

impl<'a> Transaction for PostgresTransaction<'a> {
    fn commit(&mut self) -> Result<()> {
        Ok(())
    }
}
