//! `statement-registry`: anchors a billing period's statement so a buyer
//! can verify one charge against it, check it against the price then in
//! force, and contest it publicly if it is wrong. See the workspace README
//! for the full interface.
#![no_std]

use soroban_sdk::{contract, contractimpl};

mod error;
mod storage;
mod types;

#[contract]
pub struct StatementRegistry;

#[contractimpl]
impl StatementRegistry {}
