pub mod postgres;

use anyhow::Result;

pub trait Store {
    type Connection: Connection;

    fn get_connection(&self) -> Result<Self::Connection>;
}

pub trait GenericConnection {}

pub trait Connection: GenericConnection {
    type Transaction<'a>: Transaction
    where
        Self: 'a;
    fn start_transaction<'a>(&'a mut self) -> Result<Self::Transaction<'a>>;
}

pub trait Transaction: GenericConnection {
    fn commit(&mut self) -> Result<()>;
}

#[derive(Default, Debug)]
pub struct InMemoryStore {}

impl Store for InMemoryStore {
    type Connection = InMemoryConnection;

    fn get_connection(&self) -> Result<Self::Connection> {
        Ok(InMemoryConnection::default())
    }
}

#[derive(Default, Debug)]
pub struct InMemoryConnection {}

impl GenericConnection for InMemoryConnection {}

impl Connection for InMemoryConnection {
    type Transaction<'a> = InMemoryTransaction;

    fn start_transaction<'a>(&'a mut self) -> Result<Self::Transaction<'a>> {
        Ok(InMemoryTransaction)
    }
}

#[derive(Default, Debug)]
pub struct InMemoryTransaction;

impl GenericConnection for InMemoryTransaction {}

impl Transaction for InMemoryTransaction {
    fn commit(&mut self) -> Result<()> {
        Ok(())
    }
}
