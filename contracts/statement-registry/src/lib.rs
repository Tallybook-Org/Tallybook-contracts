//! `statement-registry`: anchors a billing period's statement so a buyer
//! can verify one charge against it, check it against the price then in
//! force, and contest it publicly if it is wrong. See the workspace README
//! for the full interface.
#![no_std]

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

mod error;
mod event;
mod merkle;
mod price_book;
mod storage;
mod test;
mod types;

use error::Error;
use event::AnchorEvent;
use types::{Protocol, Statement, Status};

#[contract]
pub struct StatementRegistry;

#[contractimpl]
impl StatementRegistry {
    /// Deploy-time only. Stores `admin` and `price_book` in instance
    /// storage and extends the instance TTL.
    ///
    /// `admin` holds no power over operator data. `price_book` is
    /// immutable after construction — there is no setter. A mutable price
    /// book address would let an operator swap in a permissive registry
    /// and invalidate every historical statement; if the price book must
    /// change, a new `statement_registry` is deployed.
    ///
    /// Errors: `AlreadyInitialized` if called a second time.
    pub fn __constructor(env: Env, admin: Address, price_book: Address) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        storage::set_admin(&env, &admin);
        storage::set_price_book(&env, &price_book);
        storage::extend_instance_ttl(&env);
        Ok(())
    }

    /// Anchors a billing period's statement. Callable only by `operator`.
    ///
    /// Validates, in order: the period is non-empty and already finished
    /// (`period_start < period_end <= current ledger`); `request_count` is
    /// non-zero; amounts are non-negative and `amount_settled <=
    /// amount_billed`; `channel` is present iff `protocol == MppSession`;
    /// and — the single most important check in this contract — that
    /// `price_book_version` is the version actually in force at
    /// `period_end`, per a live call to the price_book contract. This last
    /// check is what stops an operator anchoring against a favourable old
    /// schedule.
    ///
    /// Returns the new sequence number for `operator` (the first anchor is
    /// 1).
    ///
    /// Errors: `BadPeriod`, `EmptyStatement`, `BadAmounts`,
    /// `ChannelMismatch`, `PriceVersionUnknown`, `PriceVersionStale`,
    /// `IndexFull`.
    // Argument count is fixed by the spec this contract is built against;
    // splitting it into a struct would change the public function
    // signature, which this repo has no authority to do.
    #[allow(clippy::too_many_arguments)]
    pub fn anchor(
        env: Env,
        operator: Address,
        consumer: Address,
        period_start: u32,
        period_end: u32,
        usage_root: BytesN<32>,
        request_count: u64,
        token: Address,
        amount_billed: i128,
        amount_settled: i128,
        price_book_version: u32,
        protocol: Protocol,
        channel: Option<Address>,
    ) -> Result<u64, Error> {
        operator.require_auth();

        let current_ledger = env.ledger().sequence();
        if period_start >= period_end || period_end > current_ledger {
            return Err(Error::BadPeriod);
        }
        if request_count == 0 {
            return Err(Error::EmptyStatement);
        }
        if amount_billed < 0 || amount_settled < 0 || amount_settled > amount_billed {
            return Err(Error::BadAmounts);
        }
        let channel_expected = matches!(protocol, Protocol::MppSession);
        if channel.is_some() != channel_expected {
            return Err(Error::ChannelMismatch);
        }

        // get_price_book() is None only for an unconstructed instance,
        // which cannot be executing this call — the host never invokes a
        // non-constructor function before construction. Kept as an error
        // rather than an unwrap because the invariant is enforced
        // elsewhere, not by the type system.
        let price_book_id = storage::get_price_book(&env).ok_or(Error::PriceVersionUnknown)?;
        let price_book_client = price_book::Client::new(&env, &price_book_id);
        let live_version = price_book_client
            .try_version_at(&operator, &period_end)
            .map_err(|_| Error::PriceVersionUnknown)?
            .map_err(|_| Error::PriceVersionUnknown)?;
        if live_version != price_book_version {
            return Err(Error::PriceVersionStale);
        }

        let mut consumer_idx = storage::get_consumer_idx(&env, &operator, &consumer);
        if consumer_idx.len() >= storage::CONSUMER_IDX_CAP {
            return Err(Error::IndexFull);
        }

        // Unchecked +1 is deliberate: Seq(operator) has no cap in this
        // contract's error surface (unlike ConsumerIdx), and reaching
        // u64::MAX would require on the order of 10^19 anchor() calls —
        // not a case worth inventing an unspecified error variant for.
        let seq = storage::get_seq(&env, &operator).unwrap_or(0) + 1;

        let statement = Statement {
            operator: operator.clone(),
            consumer: consumer.clone(),
            period_start,
            period_end,
            usage_root: usage_root.clone(),
            request_count,
            token,
            amount_billed,
            amount_settled,
            price_book_version,
            protocol,
            channel,
            anchored_ledger: current_ledger,
            status: Status::Anchored,
        };
        storage::set_statement(&env, &operator, seq, &statement);
        storage::set_seq(&env, &operator, seq);
        consumer_idx.push_back(seq);
        storage::set_consumer_idx(&env, &operator, &consumer, &consumer_idx);
        storage::extend_instance_ttl(&env);

        AnchorEvent {
            operator,
            consumer,
            seq,
            usage_root,
            amount_billed,
            amount_settled,
            protocol,
        }
        .publish(&env);

        Ok(seq)
    }
}
