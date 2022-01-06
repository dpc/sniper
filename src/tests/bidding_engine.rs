use crate::{
    auction,
    auction::{Amount, BidDetails, Bidder, ItemBid, ItemIdRef},
    event::{BiddingEngineEvent, Event, UiEvent},
    event_log,
    persistence::{self, Connection, Persistence},
    service,
    service::{bidding_engine::*, LogFollowerService},
};
use anyhow::Result;

trait BiddingEngineTestExt {
    fn handle_max_bid_event(
        &mut self,
        conn: &mut dyn Connection,
        id: ItemIdRef,
        price: Amount,
    ) -> Result<()>;
}

impl BiddingEngineTestExt for BiddingEngine {
    fn handle_max_bid_event<'a>(
        &mut self,
        conn: &mut dyn Connection,
        id: ItemIdRef,
        price: Amount,
    ) -> Result<()> {
        self.handle_event(
            &mut *conn.start_transaction()?,
            Event::Ui(UiEvent::MaxBidSet(auction::ItemBid {
                item: id.to_owned(),
                price,
            })),
        )
    }
}

#[test]
fn sanity_check_sends_a_bid_when_asked_to_via_event_log() -> Result<()> {
    let persistence = persistence::InMemoryPersistence::new();
    let mut conn = persistence.get_connection()?;

    let (event_writer, event_reader) = event_log::new_in_memory_shared()?;
    let bidding_state_store = service::bidding_engine::InMemoryBiddingStateStore::new_shared();

    let mut bidding_engine =
        service::bidding_engine::BiddingEngine::new(bidding_state_store, event_writer);

    bidding_engine.handle_max_bid_event(&mut *conn, "foo", 100)?;

    let res = event_reader.read_one(&mut *conn, event_reader.get_start_offset()?)?;

    assert_eq!(
        res.data.clone().map(|e| e.details),
        Some(Event::BiddingEngine(BiddingEngineEvent::Bid(ItemBid {
            item: "foo".to_owned(),
            price: 0
        })))
    );

    let res = event_reader.read_one(&mut *conn, res.offset)?;
    assert_eq!(res.data.map(|e| e.details), None);

    // sending the same bid again makes no difference
    bidding_engine.handle_max_bid_event(&mut *conn, "foo", 100)?;

    let res = event_reader.read_one(&mut *conn, res.offset)?;
    assert_eq!(res.data.map(|e| e.details), None);
    Ok(())
}

#[test]
fn sends_an_initial_bid_when_max_bid_limit_set() -> Result<()> {
    assert_eq!(
        BiddingEngine::handle_max_bid_limit_event("foo", None, 100)?,
        (
            Some(AuctionBiddingState {
                max_bid_limit: 100,
                last_bid_sent: Some(0),
                auction_state: AuctionState {
                    higest_bid: None,
                    closed: false
                },
            }),
            vec![BiddingEngineEvent::Bid(ItemBid {
                item: "foo".to_string(),
                price: 0
            })]
        )
    );

    Ok(())
}

#[test]
fn sends_a_new_bid_when_max_bid_limit_raised() -> Result<()> {
    assert_eq!(
        BiddingEngine::handle_max_bid_limit_event(
            "foo",
            Some(AuctionBiddingState {
                max_bid_limit: 100,
                last_bid_sent: Some(0),
                auction_state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Other,
                        increment: 1,
                        price: 100
                    }),
                    closed: false
                },
            }),
            101
        )?,
        (
            Some(AuctionBiddingState {
                max_bid_limit: 101,
                last_bid_sent: Some(101),
                auction_state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Other,
                        increment: 1,
                        price: 100
                    }),
                    closed: false
                },
            }),
            vec![BiddingEngineEvent::Bid(ItemBid {
                item: "foo".to_string(),
                price: 101
            })]
        )
    );

    Ok(())
}

#[test]
fn doesnt_send_a_new_bid_when_max_bid_limit_raised_but_not_enough_to_outbid() -> Result<()> {
    assert_eq!(
        BiddingEngine::handle_max_bid_limit_event(
            "foo",
            Some(AuctionBiddingState {
                max_bid_limit: 100,
                last_bid_sent: Some(0),
                auction_state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Other,
                        increment: 1,
                        price: 101
                    }),
                    closed: false
                },
            }),
            101
        )?,
        (
            Some(AuctionBiddingState {
                max_bid_limit: 101,
                last_bid_sent: Some(0),
                auction_state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Other,
                        increment: 1,
                        price: 101
                    }),
                    closed: false
                },
            }),
            vec![]
        )
    );

    Ok(())
}

#[test]
fn doesnt_send_a_new_bid_when_max_bid_limit_raised_but_we_are_already_winning() -> Result<()> {
    assert_eq!(
        BiddingEngine::handle_max_bid_limit_event(
            "foo",
            Some(AuctionBiddingState {
                max_bid_limit: 100,
                last_bid_sent: Some(0),
                auction_state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Sniper,
                        increment: 1,
                        price: 0
                    }),
                    closed: false
                },
            }),
            101
        )?,
        (
            Some(AuctionBiddingState {
                max_bid_limit: 101,
                last_bid_sent: Some(0),
                auction_state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Sniper,
                        increment: 1,
                        price: 0
                    }),
                    closed: false
                },
            }),
            vec![]
        )
    );

    Ok(())
}

#[test]
fn doesnt_send_a_new_bid_when_max_bid_limit_raised_but_we_are_already_have_a_good_bid_sent(
) -> Result<()> {
    assert_eq!(
        BiddingEngine::handle_max_bid_limit_event(
            "foo",
            Some(AuctionBiddingState {
                max_bid_limit: 100,
                last_bid_sent: Some(10),
                auction_state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Other,
                        increment: 1,
                        price: 1
                    }),
                    closed: false
                },
            }),
            101
        )?,
        (
            Some(AuctionBiddingState {
                max_bid_limit: 101,
                last_bid_sent: Some(10),
                auction_state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Other,
                        increment: 1,
                        price: 1
                    }),
                    closed: false
                },
            }),
            vec![]
        )
    );

    Ok(())
}

#[test]
fn sends_a_new_bid_when_someone_outbids() -> Result<()> {
    assert_eq!(
        BiddingEngine::handle_auction_house_event(
            "foo",
            Some(AuctionBiddingState {
                max_bid_limit: 100,
                last_bid_sent: Some(10),
                auction_state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Sniper,
                        increment: 1,
                        price: 10
                    }),
                    closed: false
                },
            }),
            crate::event::AuctionHouseItemEvent::Bid(BidDetails {
                bidder: Bidder::Other,
                price: 11,
                increment: 1
            }),
        )?,
        (
            Some(AuctionBiddingState {
                max_bid_limit: 100,
                last_bid_sent: Some(12),
                auction_state: AuctionState {
                    higest_bid: Some(BidDetails {
                        bidder: Bidder::Other,
                        increment: 1,
                        price: 11
                    }),
                    closed: false
                },
            }),
            vec![BiddingEngineEvent::Bid(ItemBid {
                item: "foo".to_string(),
                price: 12
            })]
        )
    );

    Ok(())
}
