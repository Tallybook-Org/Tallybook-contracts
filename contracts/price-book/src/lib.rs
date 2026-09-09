//! `price-book`: an append-only, versioned record of what an operator
//! charges and from when. See the workspace README for the full interface.
#![no_std]

use soroban_sdk::{contract, contractimpl};

mod types;

#[contract]
pub struct PriceBook;

#[contractimpl]
impl PriceBook {}
