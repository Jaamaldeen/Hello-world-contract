#![no_std]

mod errors;
mod storage;
mod tests;
mod types;

use errors::Error;
use soroban_sdk::{contract, contractimpl, Address, Env};
use storage::Storage;
use types::{Allowance, VaultConfig};

#[contract]
pub struct TokenVaultContract;

#[contractimpl]
impl TokenVaultContract {
    // ─── Initialization ──────────────────────────────────────────────────────

    pub fn initialize(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        if Storage::is_initialized(&env) {
            return Err(Error::AlreadyInitialized);
        }

        Storage::set_admin(&env, &admin);
        Storage::set_token(&env, &token);

        let config = VaultConfig {
            paused: false,
            owner: admin.clone(),
            token: token.clone(),
        };
        Storage::set_config(&env, &config);
        Storage::set_initialized(&env);

        Ok(())
    }

    // ─── Deposit ─────────────────────────────────────────────────────────────

    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if Storage::is_paused(&env) {
            return Err(Error::VaultPaused);
        }

        from.require_auth();

        let token = Storage::get_token(&env)?;
        let token_client = soroban_sdk::token::TokenClient::new(&env, &token);

        let balance = token_client.balance(&from);
        if balance < amount {
            return Err(Error::InsufficientBalance);
        }

        token_client.transfer(&from, &env.current_contract_address(), &amount);

        let mut user_balance = Storage::get_balance(&env, &from);
        user_balance.amount += amount;
        Storage::set_balance(&env, &from, &user_balance);

        Storage::add_to_total_deposits(&env, amount);
        Storage::emit_deposit_event(&env, &from, amount);

        Ok(())
    }

    // ─── Withdraw ────────────────────────────────────────────────────────────

    pub fn withdraw(env: Env, to: Address, amount: i128) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if Storage::is_paused(&env) {
            return Err(Error::VaultPaused);
        }

        to.require_auth();

        let mut user_balance = Storage::get_balance(&env, &to);
        if user_balance.amount < amount {
            return Err(Error::InsufficientBalance);
        }

        let token = Storage::get_token(&env)?;
        let token_client = soroban_sdk::token::TokenClient::new(&env, &token);

        token_client.transfer(&env.current_contract_address(), &to, &amount);

        user_balance.amount -= amount;
        Storage::set_balance(&env, &to, &user_balance);

        Storage::emit_withdraw_event(&env, &to, amount);

        Ok(())
    }

    // ─── Balance ─────────────────────────────────────────────────────────────

    pub fn balance(env: Env, user: Address) -> i128 {
        Storage::get_balance(&env, &user).amount
    }

    pub fn total_balance(env: Env) -> i128 {
        let token = Storage::get_token(&env).unwrap();
        let token_client = soroban_sdk::token::TokenClient::new(&env, &token);
        token_client.balance(&env.current_contract_address())
    }

    // ─── Allowance ──────────────────────────────────────────────────────────

    pub fn approve(env: Env, owner: Address, spender: Address, amount: i128) -> Result<(), Error> {
        if amount < 0 {
            return Err(Error::InvalidAmount);
        }

        owner.require_auth();

        let allowance = Allowance {
            amount,
            spender: spender.clone(),
        };

        Storage::set_allowance(&env, &owner, &spender, &allowance);
        Storage::emit_approval_event(&env, &owner, &spender, amount);

        Ok(())
    }

    pub fn allowance(env: Env, owner: Address, spender: Address) -> i128 {
        Storage::get_allowance(&env, &owner, &spender).amount
    }

    pub fn spend_allowance(
        env: Env,
        spender: Address,
        owner: Address,
        amount: i128,
    ) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        spender.require_auth();

        let mut allowance = Storage::get_allowance(&env, &owner, &spender);
        if allowance.amount < amount {
            return Err(Error::InsufficientAllowance);
        }

        let mut user_balance = Storage::get_balance(&env, &owner);
        if user_balance.amount < amount {
            return Err(Error::InsufficientBalance);
        }

        allowance.amount -= amount;
        Storage::set_allowance(&env, &owner, &spender, &allowance);

        user_balance.amount -= amount;
        Storage::set_balance(&env, &owner, &user_balance);

        let mut spender_balance = Storage::get_balance(&env, &spender);
        spender_balance.amount += amount;
        Storage::set_balance(&env, &spender, &spender_balance);

        Ok(())
    }

    // ─── Admin Functions ────────────────────────────────────────────────────

    pub fn pause(env: Env, admin: Address) -> Result<(), Error> {
        // Check that the caller is the admin
        let stored_admin = Storage::get_admin(&env)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        // Also require auth for the admin
        admin.require_auth();
        Storage::pause_vault(&env);
        Ok(())
    }

    pub fn unpause(env: Env, admin: Address) -> Result<(), Error> {
        let stored_admin = Storage::get_admin(&env)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();
        Storage::unpause_vault(&env);
        Ok(())
    }

    pub fn is_paused(env: Env) -> bool {
        Storage::is_paused(&env)
    }

    pub fn get_config(env: Env) -> Result<VaultConfig, Error> {
        Storage::get_config(&env)
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        Storage::get_admin(&env)
    }
}
