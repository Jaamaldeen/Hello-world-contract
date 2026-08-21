//! Unit tests for the Token Vault contract.

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _},
        Address, Env,
    };
    use crate::{TokenVaultContract, TokenVaultContractClient};

    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        // Initialize
        assert!(client.try_initialize(&admin, &token).is_ok());

        // Verify config
        let config = client.get_config();
        assert_eq!(config.owner, admin);
        assert_eq!(config.token, token);
        assert_eq!(config.paused, false);
    }

    #[test]
    fn test_initialize_twice_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        assert!(client.try_initialize(&admin, &token).is_ok());
        assert!(client.try_initialize(&admin, &token).is_err());
    }

    #[test]
    fn test_deposit_zero_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token);

        // Deposit zero should fail
        assert!(client.try_deposit(&user, &0).is_err());
    }

    #[test]
    fn test_withdraw_zero_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token);

        // Withdraw zero should fail
        assert!(client.try_withdraw(&user, &0).is_err());
    }

    #[test]
    fn test_allowance() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token);

        // Approve allowance
        assert!(client.try_approve(&owner, &spender, &500).is_ok());
        assert_eq!(client.allowance(&owner, &spender), 500);

        // Spend allowance (will fail due to insufficient balance, but tests the function)
        assert!(client.try_spend_allowance(&spender, &owner, &100).is_err());
    }

    #[test]
    fn test_approve_negative_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token);

        // Approve negative amount should fail
        assert!(client.try_approve(&owner, &spender, &-100).is_err());
    }

    #[test]
    fn test_pause_and_unpause() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token);

        assert!(client.try_pause(&admin).is_ok());
        assert_eq!(client.is_paused(), true);

        assert!(client.try_unpause(&admin).is_ok());
        assert_eq!(client.is_paused(), false);
    }

    #[test]
    fn test_pause_only_admin() {
        let env = Env::default();
        
        // Create admin and attacker
        let admin = Address::generate(&env);
        let attacker = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        // Initialize with admin (needs auth)
        // We'll mock auth only for the initialization
        env.mock_all_auths();
        client.initialize(&admin, &token);
        // Revert to real auth mode by creating a new environment for the test
        // The contract uses admin.require_auth() in pause()
        // So we need to test that attacker cannot call pause
        
        // Test that attacker cannot pause
        // The contract should fail because attacker is not authorized
        // Since we're in test environment, we use try_pause which returns Result
        // and expect it to be an error
        assert!(client.try_pause(&attacker).is_err());
    }

    #[test]
    fn test_get_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token);

        let retrieved_admin = client.get_admin();
        assert_eq!(retrieved_admin, admin);
    }

    #[test]
    fn test_get_config() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token);

        let config = client.get_config();
        assert_eq!(config.owner, admin);
        assert_eq!(config.token, token);
        assert_eq!(config.paused, false);
    }

    #[test]
    fn test_balance_returns_zero_for_new_user() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, TokenVaultContract);
        let client = TokenVaultContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token);

        // New user balance should be 0
        assert_eq!(client.balance(&user), 0);
    }
}
