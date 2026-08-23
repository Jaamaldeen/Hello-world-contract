use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Symbol, Vec};

/// Default maximum age (seconds) of a price before it is considered stale.
const DEFAULT_HEARTBEAT: u64 = 3_600;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum OracleError {
    /// Admin has not been set; call `init` first.
    NotInitialized = 1,
    /// `init` was already called.
    AlreadyInitialized = 2,
    /// Caller is not authorized for this action.
    Unauthorized = 3,
    /// No price has been stored for the requested asset.
    PriceNotFound = 4,
    /// Stored price is older than the heartbeat window.
    StalePrice = 5,
    /// Signer is already registered.
    SignerAlreadyExists = 6,
    /// Signer is not in the authorized set.
    SignerNotFound = 7,
}

/// On-chain price snapshot for a single asset.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    /// Price in the smallest quote unit (e.g. micro-USD).
    pub price: i128,
    /// Ledger timestamp when the price was last updated.
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum OracleKey {
    Admin,
    /// Maximum age (seconds) before a price is considered stale.
    Heartbeat,
    /// Authorized price-update signers.
    Signers,
    /// Latest price keyed by asset symbol (e.g. BTC, ETH).
    Price(Symbol),
}

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    /// One-time setup: set admin and heartbeat window (seconds).
    /// Pass `0` for `heartbeat` to use the default (1 hour).
    /// Named `init` so it does not collide with other contracts in this crate.
    pub fn init(env: Env, admin: Address, heartbeat: u64) {
        if env.storage().instance().has(&OracleKey::Admin) {
            panic_err(&env, OracleError::AlreadyInitialized);
        }
        admin.require_auth();

        let hb = if heartbeat == 0 {
            DEFAULT_HEARTBEAT
        } else {
            heartbeat
        };

        env.storage().instance().set(&OracleKey::Admin, &admin);
        env.storage().instance().set(&OracleKey::Heartbeat, &hb);
        env.storage()
            .persistent()
            .set(&OracleKey::Signers, &Vec::<Address>::new(&env));
    }

    /// Admin-only: register an address allowed to push price updates.
    pub fn add_signer(env: Env, admin: Address, signer: Address) {
        require_admin(&env, &admin);

        let mut signers = get_signers(&env);
        if signers.contains(&signer) {
            panic_err(&env, OracleError::SignerAlreadyExists);
        }
        signers.push_back(signer);
        env.storage()
            .persistent()
            .set(&OracleKey::Signers, &signers);
    }

    /// Admin-only: revoke a signer's update rights.
    pub fn remove_signer(env: Env, admin: Address, signer: Address) {
        require_admin(&env, &admin);

        let signers = get_signers(&env);
        let mut updated = Vec::new(&env);
        let mut found = false;
        for s in signers.iter() {
            if s == signer {
                found = true;
            } else {
                updated.push_back(s);
            }
        }
        if !found {
            panic_err(&env, OracleError::SignerNotFound);
        }
        env.storage()
            .persistent()
            .set(&OracleKey::Signers, &updated);
    }

    /// Admin-only: update the freshness window (seconds).
    pub fn set_heartbeat(env: Env, admin: Address, heartbeat: u64) {
        require_admin(&env, &admin);
        let hb = if heartbeat == 0 {
            DEFAULT_HEARTBEAT
        } else {
            heartbeat
        };
        env.storage().instance().set(&OracleKey::Heartbeat, &hb);
    }

    /// Authorized signer: store/update the price for an asset.
    pub fn update_price(env: Env, signer: Address, asset: Symbol, price: i128) {
        signer.require_auth();
        if !is_signer(&env, &signer) {
            panic_err(&env, OracleError::Unauthorized);
        }

        let data = PriceData {
            price,
            timestamp: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&OracleKey::Price(asset), &data);
    }

    /// Read the latest price for `asset`. Panics if missing or stale
    /// (age exceeds the configured heartbeat). Safe for other contracts.
    pub fn get_price(env: Env, asset: Symbol) -> PriceData {
        let data = env
            .storage()
            .persistent()
            .get::<_, PriceData>(&OracleKey::Price(asset))
            .unwrap_or_else(|| panic_err(&env, OracleError::PriceNotFound));

        if !is_fresh(&env, data.timestamp) {
            panic_err(&env, OracleError::StalePrice);
        }
        data
    }

    /// Read the latest price without the heartbeat freshness check.
    pub fn get_price_unsafe(env: Env, asset: Symbol) -> PriceData {
        env.storage()
            .persistent()
            .get(&OracleKey::Price(asset))
            .unwrap_or_else(|| panic_err(&env, OracleError::PriceNotFound))
    }

    /// Returns `true` when a price exists and is within the heartbeat window.
    pub fn is_fresh(env: Env, asset: Symbol) -> bool {
        match env
            .storage()
            .persistent()
            .get::<_, PriceData>(&OracleKey::Price(asset))
        {
            Some(data) => is_fresh(&env, data.timestamp),
            None => false,
        }
    }

    /// Current heartbeat window in seconds.
    pub fn get_heartbeat(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&OracleKey::Heartbeat)
            .unwrap_or(DEFAULT_HEARTBEAT)
    }

    /// List of authorized price-update signers.
    pub fn get_signers(env: Env) -> Vec<Address> {
        get_signers(&env)
    }
}

fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&OracleKey::Admin)
        .unwrap_or_else(|| panic_err(env, OracleError::NotInitialized))
}

fn require_admin(env: &Env, admin: &Address) {
    let stored = get_admin(env);
    if stored != *admin {
        panic_err(env, OracleError::Unauthorized);
    }
    admin.require_auth();
}

fn get_signers(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&OracleKey::Signers)
        .unwrap_or_else(|| Vec::new(env))
}

fn is_signer(env: &Env, address: &Address) -> bool {
    get_signers(env).contains(address)
}

fn is_fresh(env: &Env, timestamp: u64) -> bool {
    let heartbeat: u64 = env
        .storage()
        .instance()
        .get(&OracleKey::Heartbeat)
        .unwrap_or(DEFAULT_HEARTBEAT);
    let now = env.ledger().timestamp();
    now.saturating_sub(timestamp) <= heartbeat
}

#[inline(always)]
fn panic_err(env: &Env, err: OracleError) -> ! {
    soroban_sdk::panic_with_error!(env, err);
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Ledger},
    };

    struct TestCtx {
        env: Env,
        client: OracleContractClient<'static>,
        admin: Address,
        signer: Address,
        stranger: Address,
    }

    impl TestCtx {
        fn new() -> Self {
            let env = Env::default();
            env.mock_all_auths();

            let contract_id = env.register_contract(None, OracleContract);
            // SAFETY: Env/client live for the duration of each test; 'static matches test scope.
            let client: OracleContractClient<'static> =
                unsafe { core::mem::transmute(OracleContractClient::new(&env, &contract_id)) };

            let admin = Address::generate(&env);
            let signer = Address::generate(&env);
            let stranger = Address::generate(&env);

            client.init(&admin, &60);
            client.add_signer(&admin, &signer);

            Self {
                env,
                client,
                admin,
                signer,
                stranger,
            }
        }
    }

    #[test]
    fn initialize_sets_heartbeat_and_empty_signers() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, OracleContract);
        let client = OracleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.init(&admin, &120);
        assert_eq!(client.get_heartbeat(), 120);
        assert_eq!(client.get_signers().len(), 0);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn initialize_twice_panics() {
        let ctx = TestCtx::new();
        ctx.client.init(&ctx.admin, &60);
    }

    #[test]
    fn authorized_signer_can_update_and_query_price() {
        let ctx = TestCtx::new();
        let btc = symbol_short!("BTC");
        let eth = symbol_short!("ETH");

        ctx.client
            .update_price(&ctx.signer, &btc, &95_000_000_000);
        ctx.client
            .update_price(&ctx.signer, &eth, &3_500_000_000);

        let btc_price = ctx.client.get_price(&btc);
        assert_eq!(btc_price.price, 95_000_000_000);
        assert_eq!(btc_price.timestamp, 0);

        let eth_price = ctx.client.get_price(&eth);
        assert_eq!(eth_price.price, 3_500_000_000);

        assert!(ctx.client.is_fresh(&btc));
        assert!(ctx.client.is_fresh(&eth));
    }

    #[test]
    fn price_update_overwrites_previous_value() {
        let ctx = TestCtx::new();
        let btc = symbol_short!("BTC");

        ctx.client.update_price(&ctx.signer, &btc, &90_000);
        ctx.env.ledger().with_mut(|l| l.timestamp = 10);
        ctx.client.update_price(&ctx.signer, &btc, &91_500);

        let data = ctx.client.get_price(&btc);
        assert_eq!(data.price, 91_500);
        assert_eq!(data.timestamp, 10);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")]
    fn non_signer_cannot_update_price() {
        let ctx = TestCtx::new();
        ctx.client
            .update_price(&ctx.stranger, &symbol_short!("BTC"), &1);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")]
    fn non_admin_cannot_add_signer() {
        let ctx = TestCtx::new();
        ctx.client.add_signer(&ctx.stranger, &ctx.stranger);
    }

    #[test]
    fn remove_signer_revokes_update_rights() {
        let ctx = TestCtx::new();
        let btc = symbol_short!("BTC");

        ctx.client.update_price(&ctx.signer, &btc, &100);
        ctx.client.remove_signer(&ctx.admin, &ctx.signer);

        let signers = ctx.client.get_signers();
        assert!(!signers.contains(&ctx.signer));

        let result = ctx.client.try_update_price(&ctx.signer, &btc, &200);
        assert!(result.is_err());
    }

    #[test]
    fn heartbeat_marks_stale_prices() {
        let ctx = TestCtx::new();
        let btc = symbol_short!("BTC");

        ctx.client.update_price(&ctx.signer, &btc, &50_000);
        assert!(ctx.client.is_fresh(&btc));

        ctx.env.ledger().with_mut(|l| l.timestamp = 61);
        assert!(!ctx.client.is_fresh(&btc));

        let data = ctx.client.get_price_unsafe(&btc);
        assert_eq!(data.price, 50_000);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")]
    fn get_price_panics_when_stale() {
        let ctx = TestCtx::new();
        let btc = symbol_short!("BTC");

        ctx.client.update_price(&ctx.signer, &btc, &50_000);
        ctx.env.ledger().with_mut(|l| l.timestamp = 61);
        let _ = ctx.client.get_price(&btc);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn get_price_panics_when_missing() {
        let ctx = TestCtx::new();
        let _ = ctx.client.get_price(&symbol_short!("XLM"));
    }

    #[test]
    fn set_heartbeat_updates_freshness_window() {
        let ctx = TestCtx::new();
        let btc = symbol_short!("BTC");

        ctx.client.update_price(&ctx.signer, &btc, &1);
        ctx.client.set_heartbeat(&ctx.admin, &30);

        ctx.env.ledger().with_mut(|l| l.timestamp = 31);
        assert!(!ctx.client.is_fresh(&btc));

        ctx.client.set_heartbeat(&ctx.admin, &100);
        assert!(ctx.client.is_fresh(&btc));
    }

    #[test]
    fn multiple_signers_can_update_different_assets() {
        let ctx = TestCtx::new();
        let signer2 = Address::generate(&ctx.env);
        ctx.client.add_signer(&ctx.admin, &signer2);

        ctx.client
            .update_price(&ctx.signer, &symbol_short!("BTC"), &100);
        ctx.client
            .update_price(&signer2, &symbol_short!("ETH"), &200);

        assert_eq!(ctx.client.get_price(&symbol_short!("BTC")).price, 100);
        assert_eq!(ctx.client.get_price(&symbol_short!("ETH")).price, 200);
        assert_eq!(ctx.client.get_signers().len(), 2);
    }
}
