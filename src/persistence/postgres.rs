use super::*;

#[derive(Debug, Clone)]
pub struct PostgresPersistence {
    pool: r2d2::Pool<r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>>,
}

impl Persistence for PostgresPersistence {
    fn get_connection(&self) -> Result<Box<dyn Connection>> {
        Ok(Box::new(PostgresConnection(self.pool.get()?)))
    }
}

pub struct PostgresConnection(
    pub  r2d2::PooledConnection<
        r2d2_postgres::PostgresConnectionManager<r2d2_postgres::postgres::NoTls>,
    >,
);

impl<'a> dyno::Tag<'a> for PostgresConnection {
    type Type = PostgresConnection;
}
impl Connection for PostgresConnection {
    fn start_transaction<'a>(&'a mut self) -> Result<Box<dyn Transaction<'a> + 'a>> {
        Ok(Box::new(PostgresTransaction(self.0.transaction()?)))
    }

    fn cast<'b>(&'b mut self) -> Caster<'b, 'static> {
        Caster::new::<PostgresConnection>(self)
    }
}

pub struct PostgresTransaction<'a>(pub ::postgres::Transaction<'a>);

impl<'a> dyno::Tag<'a> for PostgresTransaction<'static> {
    type Type = PostgresTransaction<'a>;
}

impl<'a> Transaction<'a> for PostgresTransaction<'a> {
    fn commit(self: Box<Self>) -> Result<()> {
        Ok(((self.0) as ::postgres::Transaction<'a>).commit()?)
    }

    fn rollback(self: Box<Self>) -> Result<()> {
        Ok(((self.0) as ::postgres::Transaction<'a>).rollback()?)
    }

    fn cast<'caster>(&'caster mut self) -> Caster<'caster, 'a>
    where
        'a: 'caster,
    {
        Caster::new::<PostgresTransaction<'static>>(self)
    }
}
