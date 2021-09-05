use std::time::Duration;

use crate::{
    event_log,
    event_log::*,
    persistence::{self, Persistence},
};
use anyhow::Result;

#[test]
fn event_logs_sanity_check() -> Result<()> {
    let persistence = persistence::in_memory::new();
    let (event_writer, event_reader) = event_log::new_in_memory_shared();

    let start_offset = event_reader.get_start_offset()?;

    let mut conn = persistence.get_connection()?;

    assert_eq!(
        event_reader.read(&mut conn, start_offset, 0, Some(Duration::from_secs(0)))?,
        (start_offset, vec![])
    );

    assert_eq!(
        event_reader.read(&mut conn, start_offset, 1, Some(Duration::from_secs(0)))?,
        (start_offset, vec![])
    );

    let inserted_offset = event_writer.write(&mut conn, &[event_log::EventDetails::Test])?;

    assert_eq!(
        event_reader.read(
            &mut conn,
            inserted_offset.clone(),
            1,
            Some(Duration::from_secs(0))
        )?,
        (inserted_offset, vec![])
    );

    assert_eq!(
        event_reader.read(
            &mut conn,
            event_reader.get_start_offset()?,
            1,
            Some(Duration::from_secs(0))
        )?,
        (
            inserted_offset,
            vec![Event {
                offset: event_reader.get_start_offset()?,
                details: event_log::EventDetails::Test
            }]
        )
    );

    Ok(())
}
