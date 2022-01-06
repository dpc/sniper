use super::*;

#[derive(Debug, Clone)]
pub struct PostgresPersistence {
    pool: r2d2::Pool<r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>>,
}

impl Persistence for PostgresPersistence {
    fn get_connection(&self) -> Result<Box<dyn Connection>> {
        Ok(Box::new(self.pool.get()?))
    }
}

pub type PostgresConnection = r2d2::PooledConnection<
    r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>,
>;

impl Connection for PostgresConnection {
    fn start_transaction<'a>(&'a mut self) -> Result<Box<dyn Transaction<'a> + 'a>> {
        Ok(Box::new(self.transaction()?))
    }

    fn cast<'b>(&'b mut self) -> Caster<'b> {
        Caster::new(self)
    }
}

pub type PostgresTransaction<'a> = ::postgres::Transaction<'a>;

impl<'a> Transaction<'a> for PostgresTransaction<'a> {
    fn commit(self: Box<Self>) -> Result<()> {
        Ok((*self as ::postgres::Transaction<'a>).commit()?)
    }

    fn rollback(self: Box<Self>) -> Result<()> {
        Ok((*self as ::postgres::Transaction<'a>).rollback()?)
    }

    fn cast<'caster>(&'caster mut self) -> Caster<'caster>
    where
        'a: 'caster,
    {
        unsafe {
            Caster::new_transmute::<'a, PostgresTransaction<'a>, PostgresTransaction<'static>>(self)
        }
    }
}
