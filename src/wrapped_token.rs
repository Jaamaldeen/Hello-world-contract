//! Mint/Burn Wrapper Contract
//!
//! This contract wraps an existing Soroban/Stellar asset (e.g. USDC) into a
//! Soroban-native token. Users deposit the underlying asset and receive wrapped
//! tokens 1:1 (`wrap`). They can redeem back (`unwrap`) at any time. The admin
//! can also `mint` wrapped tokens directly (for testing / incentives) and
//! `burn` any wrapped tokens they hold.
//!
//! Storage layout
//! --------------
//! Instance storage (contract lifetime):
//!   - `WTokenKey::Admin`       → Address
//!   - `WTokenKey::Underlying`  → Address  (the wrapped asset contract)
//!   - `WTokenKey::Name`        → String
//!   - `WTokenKey::Symbol`      → String
//!   - `WTokenKey::Decimals`    → u32
//!
//! Persistent storage (per address):
//!   - `WTokenKey::Balance(Address)` → i128

#![allow(unused)]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, String};

// ─── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum WTokenKey {
    /// Admin address – set at initialization, never changes.
    Admin,
    /// The address of the underlying asset being wrapped (e.g. USDC).
    Underlying,
    /// Human-readable token name (e.g. "Wrapped USDC").
    Name,
    /// Short ticker symbol (e.g. "wUSDC").
    Symbol,
    /// Decimal places inherited from the underlying asset (typically 7).
    Decimals,
    /// Wrapped token balance per holder.
    Balance(Address),
    /// Total supply of wrapped tokens currently in circulation.
    TotalSupply,
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct WrapperContract;

#[contractimpl]
impl WrapperContract {
    // ── Lifecycle ────────────────────────────────────────────────────────────

    /// Initialize the wrapper contract.
    ///
    /// Must be called exactly once after deployment.  Subsequent calls panic.
    ///
    /// # Arguments
    /// * `env`        - The execution environment.
    /// * `admin`      - Administrator address (allowed to mint/burn).
    /// * `underlying` - Address of the existing Soroban token being wrapped.
    /// * `name`       - Display name for the wrapped token (e.g. "Wrapped USDC").
    /// * `symbol`     - Short ticker symbol (e.g. "wUSDC").
    /// * `decimals`   - Decimal precision to use (normally matches the underlying asset).
    pub fn initialize(
        env: Env,
        admin: Address,
        underlying: Address,
        name: String,
        symbol: String,
        decimals: u32,
    ) {
        if env.storage().instance().has(&WTokenKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&WTokenKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&WTokenKey::Underlying, &underlying);
        env.storage().instance().set(&WTokenKey::Name, &name);
        env.storage().instance().set(&WTokenKey::Symbol, &symbol);
        env.storage()
            .instance()
            .set(&WTokenKey::Decimals, &decimals);
        env.storage()
            .instance()
            .set(&WTokenKey::TotalSupply, &0_i128);
    }

    // ── Core wrap / unwrap ───────────────────────────────────────────────────

    /// Wrap underlying tokens: deposit `amount` of the underlying asset and
    /// receive an equal amount of wrapped tokens.
    ///
    /// The caller must have already approved the wrapper contract to transfer
    /// `amount` from their account (via the underlying token's `approve`
    /// instruction), or the call will fail.
    ///
    /// # Arguments
    /// * `env`    - The execution environment.
    /// * `caller` - Address performing the wrap.  Must authorize this call.
    /// * `amount` - Number of underlying tokens to deposit.
    ///
    /// # Returns
    /// The new wrapped-token balance of `caller` after the wrap.
    pub fn wrap(env: Env, caller: Address, amount: i128) -> i128 {
        caller.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }

        // Pull `amount` of underlying from `caller` into this contract.
        let underlying: Address = env
            .storage()
            .instance()
            .get(&WTokenKey::Underlying)
            .unwrap();
        let underlying_client = token::Client::new(&env, &underlying);
        underlying_client.transfer(&caller, &env.current_contract_address(), &amount);

        // Mint wrapped tokens to caller 1:1.
        Self::_credit(&env, &caller, amount);

        Self::balance(env, caller)
    }

    /// Unwrap: burn `amount` of wrapped tokens and return the underlying asset
    /// 1:1 to the caller.
    ///
    /// # Arguments
    /// * `env`    - The execution environment.
    /// * `caller` - Address performing the unwrap.  Must authorize this call.
    /// * `amount` - Number of wrapped tokens to burn.
    ///
    /// # Returns
    /// The new wrapped-token balance of `caller` after the unwrap.
    pub fn unwrap(env: Env, caller: Address, amount: i128) -> i128 {
        caller.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }

        // Debit wrapped tokens first (panics if insufficient balance).
        Self::_debit(&env, &caller, amount);

        // Return underlying tokens to caller.
        let underlying: Address = env
            .storage()
            .instance()
            .get(&WTokenKey::Underlying)
            .unwrap();
        let underlying_client = token::Client::new(&env, &underlying);
        underlying_client.transfer(&env.current_contract_address(), &caller, &amount);

        Self::balance(env, caller)
    }

    // ── Admin-only mint / burn ────────────────────────────────────────────────

    /// Mint wrapped tokens directly to a recipient without depositing the
    /// underlying asset.  Restricted to the admin (e.g. for incentive programs
    /// or testing).
    ///
    /// # Arguments
    /// * `env`       - The execution environment.
    /// * `admin`     - The contract admin.  Must authorize this call.
    /// * `recipient` - Address that will receive the minted tokens.
    /// * `amount`    - Number of wrapped tokens to mint.
    pub fn mint(env: Env, admin: Address, recipient: Address, amount: i128) {
        admin.require_auth();
        Self::_require_admin(&env, &admin);

        if amount <= 0 {
            panic!("amount must be positive");
        }

        Self::_credit(&env, &recipient, amount);
    }

    /// Burn wrapped tokens held by the caller, permanently destroying them.
    ///
    /// Unlike `unwrap`, no underlying asset is returned.  Use this when the
    /// underlying has already been reclaimed through another route, or for
    /// deflationary mechanics.
    ///
    /// # Arguments
    /// * `env`    - The execution environment.
    /// * `caller` - Address whose wrapped tokens will be burned.  Must authorize.
    /// * `amount` - Number of wrapped tokens to burn.
    pub fn burn(env: Env, caller: Address, amount: i128) {
        caller.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }

        Self::_debit(&env, &caller, amount);
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Return the wrapped-token balance of `account`.
    ///
    /// # Arguments
    /// * `env`     - The execution environment.
    /// * `account` - Address to query.
    ///
    /// # Returns
    /// The balance as `i128` (returns 0 for unknown accounts).
    pub fn balance(env: Env, account: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&WTokenKey::Balance(account))
            .unwrap_or(0_i128)
    }

    /// Return the total supply of wrapped tokens currently in circulation.
    pub fn total_supply(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&WTokenKey::TotalSupply)
            .unwrap_or(0_i128)
    }

    /// Return the human-readable name of the wrapped token.
    pub fn name(env: Env) -> String {
        env.storage().instance().get(&WTokenKey::Name).unwrap()
    }

    /// Return the ticker symbol of the wrapped token.
    pub fn symbol(env: Env) -> String {
        env.storage().instance().get(&WTokenKey::Symbol).unwrap()
    }

    /// Return the decimal precision of the wrapped token.
    pub fn decimals(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&WTokenKey::Decimals)
            .unwrap()
    }

    /// Return the address of the underlying asset being wrapped.
    pub fn underlying(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&WTokenKey::Underlying)
            .unwrap()
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Credit `amount` wrapped tokens to `account`, increasing total supply.
    fn _credit(env: &Env, account: &Address, amount: i128) {
        let current: i128 = env
            .storage()
            .persistent()
            .get(&WTokenKey::Balance(account.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&WTokenKey::Balance(account.clone()), &(current + amount));

        let supply: i128 = env
            .storage()
            .instance()
            .get(&WTokenKey::TotalSupply)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&WTokenKey::TotalSupply, &(supply + amount));
    }

    /// Debit `amount` wrapped tokens from `account`, decreasing total supply.
    /// Panics if `account` has insufficient balance.
    fn _debit(env: &Env, account: &Address, amount: i128) {
        let current: i128 = env
            .storage()
            .persistent()
            .get(&WTokenKey::Balance(account.clone()))
            .unwrap_or(0);
        if current < amount {
            panic!("insufficient wrapped token balance");
        }
        env.storage()
            .persistent()
            .set(&WTokenKey::Balance(account.clone()), &(current - amount));

        let supply: i128 = env
            .storage()
            .instance()
            .get(&WTokenKey::TotalSupply)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&WTokenKey::TotalSupply, &(supply - amount));
    }

    /// Assert that `caller` is the stored admin; panic otherwise.
    fn _require_admin(env: &Env, caller: &Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&WTokenKey::Admin)
            .expect("not initialized");
        if admin != *caller {
            panic!("caller is not admin");
        }
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::Address as _,
        token::{Client as TokenClient, StellarAssetClient},
        Env,
    };

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Deploy a fresh wrapper + underlying stellar asset; return
    /// (env, wrapper_client, underlying_client, asset_admin, admin, user).
    fn setup() -> (
        Env,
        WrapperContractClient<'static>,
        TokenClient<'static>,
        Address,
        Address,
        Address,
    ) {
        let env = Env::default();
        env.mock_all_auths();

        // Deploy an underlying Stellar asset (simulates USDC).
        let asset_admin = Address::generate(&env);
        let underlying_id = env.register_stellar_asset_contract(asset_admin.clone());
        let underlying_client = TokenClient::new(&env, &underlying_id);

        // Deploy the wrapper contract.
        let wrapper_id = env.register(WrapperContract, ());
        // SAFETY: env and all clients are owned in the same test scope.
        let wrapper_client: WrapperContractClient<'static> =
            unsafe { core::mem::transmute(WrapperContractClient::new(&env, &wrapper_id)) };
        let underlying_client: TokenClient<'static> =
            unsafe { core::mem::transmute(underlying_client) };

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        // Initialize the wrapper.
        wrapper_client.initialize(
            &admin,
            &underlying_id,
            &soroban_sdk::String::from_str(&env, "Wrapped USDC"),
            &soroban_sdk::String::from_str(&env, "wUSDC"),
            &7_u32,
        );

        // Fund the user with 1_000 underlying tokens.
        let asset_admin_client = StellarAssetClient::new(&env, &underlying_id);
        asset_admin_client.mint(&user, &1_000);

        (env, wrapper_client, underlying_client, asset_admin, admin, user)
    }

    // ── wrap ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_wrap_moves_underlying_and_credits_wrapped() {
        let (env, wrapper, underlying, _asset_admin, _admin, user) = setup();

        // Wrap 500 tokens.
        let new_balance = wrapper.wrap(&user, &500);

        assert_eq!(new_balance, 500, "wrapped balance should be 500 after wrapping");
        assert_eq!(
            underlying.balance(&user),
            500,
            "underlying balance should decrease by 500"
        );
        assert_eq!(
            underlying.balance(&wrapper.address),
            500,
            "contract should hold the deposited underlying"
        );
        assert_eq!(wrapper.total_supply(), 500);
    }

    #[test]
    fn test_wrap_full_amount() {
        let (_env, wrapper, underlying, _asset_admin, _admin, user) = setup();

        wrapper.wrap(&user, &1_000);

        assert_eq!(wrapper.balance(&user), 1_000);
        assert_eq!(underlying.balance(&user), 0);
        assert_eq!(wrapper.total_supply(), 1_000);
    }

    #[test]
    #[should_panic(expected = "amount must be positive")]
    fn test_wrap_zero_amount_panics() {
        let (_env, wrapper, _underlying, _asset_admin, _admin, user) = setup();
        wrapper.wrap(&user, &0);
    }

    // ── unwrap ───────────────────────────────────────────────────────────────

    #[test]
    fn test_unwrap_returns_underlying_and_burns_wrapped() {
        let (_env, wrapper, underlying, _asset_admin, _admin, user) = setup();

        // Wrap first, then unwrap half.
        wrapper.wrap(&user, &800);
        let new_balance = wrapper.unwrap(&user, &400);

        assert_eq!(new_balance, 400, "wrapped balance should be 400 after unwrapping 400");
        assert_eq!(
            underlying.balance(&user),
            600,
            "user should have original 200 + 400 returned = 600"
        );
        assert_eq!(wrapper.total_supply(), 400);
    }

    #[test]
    #[should_panic(expected = "insufficient wrapped token balance")]
    fn test_unwrap_more_than_balance_panics() {
        let (_env, wrapper, _underlying, _asset_admin, _admin, user) = setup();

        wrapper.wrap(&user, &100);
        wrapper.unwrap(&user, &200); // should panic
    }

    #[test]
    #[should_panic(expected = "amount must be positive")]
    fn test_unwrap_zero_amount_panics() {
        let (_env, wrapper, _underlying, _asset_admin, _admin, user) = setup();
        wrapper.unwrap(&user, &0);
    }

    // ── wrap → unwrap round-trip ─────────────────────────────────────────────

    #[test]
    fn test_wrap_unwrap_roundtrip_restores_balance() {
        let (_env, wrapper, underlying, _asset_admin, _admin, user) = setup();

        let initial = underlying.balance(&user); // 1_000
        wrapper.wrap(&user, &1_000);
        wrapper.unwrap(&user, &1_000);

        assert_eq!(underlying.balance(&user), initial, "full round-trip should restore underlying");
        assert_eq!(wrapper.balance(&user), 0, "wrapped balance should be zero after round-trip");
        assert_eq!(wrapper.total_supply(), 0);
    }

    #[test]
    fn test_wrap_unwrap_partial_roundtrip() {
        let (_env, wrapper, underlying, _asset_admin, _admin, user) = setup();

        wrapper.wrap(&user, &600);
        wrapper.unwrap(&user, &250);

        assert_eq!(wrapper.balance(&user), 350);
        assert_eq!(underlying.balance(&user), 650); // 400 still wrapped, 250 returned
        assert_eq!(wrapper.total_supply(), 350);
    }

    // ── mint (admin) ─────────────────────────────────────────────────────────

    #[test]
    fn test_mint_credits_recipient_without_deposit() {
        let (env, wrapper, _underlying, _asset_admin, admin, user) = setup();

        wrapper.mint(&admin, &user, &250);

        assert_eq!(wrapper.balance(&user), 250);
        assert_eq!(wrapper.total_supply(), 250);
    }

    #[test]
    #[should_panic(expected = "caller is not admin")]
    fn test_mint_non_admin_panics() {
        let (_env, wrapper, _underlying, _asset_admin, _admin, user) = setup();

        // `user` is not the admin — should panic.
        wrapper.mint(&user, &user, &100);
    }

    #[test]
    #[should_panic(expected = "amount must be positive")]
    fn test_mint_zero_amount_panics() {
        let (_env, wrapper, _underlying, _asset_admin, admin, user) = setup();
        wrapper.mint(&admin, &user, &0);
    }

    // ── burn ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_burn_destroys_wrapped_tokens() {
        let (_env, wrapper, _underlying, _asset_admin, admin, user) = setup();

        wrapper.mint(&admin, &user, &500);
        wrapper.burn(&user, &200);

        assert_eq!(wrapper.balance(&user), 300);
        assert_eq!(wrapper.total_supply(), 300);
    }

    #[test]
    fn test_burn_full_balance() {
        let (_env, wrapper, _underlying, _asset_admin, admin, user) = setup();

        wrapper.mint(&admin, &user, &300);
        wrapper.burn(&user, &300);

        assert_eq!(wrapper.balance(&user), 0);
        assert_eq!(wrapper.total_supply(), 0);
    }

    #[test]
    #[should_panic(expected = "insufficient wrapped token balance")]
    fn test_burn_more_than_balance_panics() {
        let (_env, wrapper, _underlying, _asset_admin, admin, user) = setup();

        wrapper.mint(&admin, &user, &100);
        wrapper.burn(&user, &200); // should panic
    }

    #[test]
    #[should_panic(expected = "amount must be positive")]
    fn test_burn_zero_amount_panics() {
        let (_env, wrapper, _underlying, _asset_admin, _admin, user) = setup();
        wrapper.burn(&user, &0);
    }

    // ── metadata queries ─────────────────────────────────────────────────────

    #[test]
    fn test_metadata_queries_return_initialized_values() {
        let (env, wrapper, _underlying, _asset_admin, _admin, _user) = setup();

        assert_eq!(
            wrapper.name(),
            soroban_sdk::String::from_str(&env, "Wrapped USDC")
        );
        assert_eq!(
            wrapper.symbol(),
            soroban_sdk::String::from_str(&env, "wUSDC")
        );
        assert_eq!(wrapper.decimals(), 7_u32);
    }

    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_double_initialize_panics() {
        let (env, wrapper, _underlying, _asset_admin, admin, _user) = setup();
        let underlying_id = wrapper.underlying();
        wrapper.initialize(
            &admin,
            &underlying_id,
            &soroban_sdk::String::from_str(&env, "X"),
            &soroban_sdk::String::from_str(&env, "X"),
            &7,
        );
    }

    // ── total supply across multiple users ───────────────────────────────────

    #[test]
    fn test_total_supply_tracks_across_multiple_users() {
        let (env, wrapper, _underlying, asset_admin, admin, user1) = setup();

        // Create a second user and fund them.
        let user2 = Address::generate(&env);
        let underlying_id = wrapper.underlying();
        let asset_admin_client = StellarAssetClient::new(&env, &underlying_id);
        asset_admin_client.mint(&user2, &500);

        wrapper.wrap(&user1, &300);
        wrapper.wrap(&user2, &200);
        wrapper.mint(&admin, &user2, &100);

        assert_eq!(wrapper.total_supply(), 600);

        wrapper.burn(&user1, &100);
        assert_eq!(wrapper.total_supply(), 500);

        wrapper.unwrap(&user2, &150);
        assert_eq!(wrapper.total_supply(), 350);
    }
}
