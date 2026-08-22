# Faucet Contract

This is a token faucet smart contract on Soroban. It dispenses test tokens to users while enforcing a claim limit and a cooldown period between claims.

## Public Functions

### `initialize(env: Env, admin: Address, token: Address, claim_limit: i128, cooldown: u64)`
Initializes the faucet contract with its settings.
* **Parameters**:
  * `env`: The execution environment.
  * `admin`: The administrator address of the faucet.
  * `token`: The address of the token to be dispensed.
  * `claim_limit`: The maximum amount of tokens allowed per claim.
  * `cooldown`: The cooldown period in seconds required between claims for a single user.
* **Returns**: None.

### `claim(env: Env, user: Address, amount: i128)`
Allows a user to claim tokens from the faucet, up to the `claim_limit`, and enforces the `cooldown` period.
* **Parameters**:
  * `env`: The execution environment.
  * `user`: The address claiming the tokens. Must have authorization.
  * `amount`: The amount of tokens requested.
* **Returns**: None.