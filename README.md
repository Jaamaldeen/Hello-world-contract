# Token Vault Contract

A Soroban smart contract that implements a secure token vault with deposit, withdraw, balance accounting, and allowance support.

## Features

- ✅ **Deposit** - Users can deposit tokens into the vault
- ✅ **Withdraw** - Users can withdraw their deposited tokens
- ✅ **Balance Accounting** - Track user balances within the vault
- ✅ **Allowance Support** - Approve other addresses to spend on your behalf
- ✅ **Pause/Unpause** - Admin can pause the vault in emergencies
- ✅ **Events** - Emits events for deposits, withdrawals, and approvals

## Contract Functions

### `initialize(admin: Address, token: Address) -> Result<(), Error>`

Initializes the vault with an admin address and the token contract address.

| Parameter | Type | Description |
|-----------|------|-------------|
| `admin` | `Address` | Address of the vault administrator |
| `token` | `Address` | Address of the token contract |

**Returns:** `Ok(())` on success, `Error::AlreadyInitialized` if already initialized.

---

### `deposit(from: Address, amount: i128) -> Result<(), Error>`

Deposits tokens from `from` into the vault.

| Parameter | Type | Description |
|-----------|------|-------------|
| `from` | `Address` | Address depositing tokens (must be authenticated) |
| `amount` | `i128` | Amount of tokens to deposit (must be > 0) |

**Returns:** `Ok(())` on success.

**Errors:**
- `Error::InvalidAmount` - Amount must be > 0
- `Error::VaultPaused` - Vault is paused
- `Error::InsufficientBalance` - User doesn't have enough tokens

---

### `withdraw(to: Address, amount: i128) -> Result<(), Error>`

Withdraws tokens from the vault to `to`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `to` | `Address` | Address receiving the tokens (must be authenticated) |
| `amount` | `i128` | Amount of tokens to withdraw (must be > 0) |

**Returns:** `Ok(())` on success.

**Errors:**
- `Error::InvalidAmount` - Amount must be > 0
- `Error::VaultPaused` - Vault is paused
- `Error::InsufficientBalance` - User doesn't have enough vault balance

---

### `balance(user: Address) -> i128`

Returns the vault balance of a user.

| Parameter | Type | Description |
|-----------|------|-------------|
| `user` | `Address` | Address to query balance for |

**Returns:** The user's vault balance (i128)

---

### `total_balance() -> i128`

Returns the total vault balance (sum of all user deposits).

**Returns:** Total vault balance (i128)

---

### `approve(owner: Address, spender: Address, amount: i128) -> Result<(), Error>`

Approves `spender` to spend tokens on behalf of `owner`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `owner` | `Address` | Address owning the tokens (must be authenticated) |
| `spender` | `Address` | Address allowed to spend tokens |
| `amount` | `i128` | Amount allowed to spend (must be >= 0) |

**Returns:** `Ok(())` on success.

**Errors:**
- `Error::InvalidAmount` - Amount must be >= 0

---

### `allowance(owner: Address, spender: Address) -> i128`

Returns the allowance from `owner` to `spender`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `owner` | `Address` | Address owning the tokens |
| `spender` | `Address` | Address allowed to spend tokens |

**Returns:** The allowance amount (i128)

---

### `spend_allowance(spender: Address, owner: Address, amount: i128) -> Result<(), Error>`

Spends tokens from `owner`'s balance via `spender`'s allowance.

| Parameter | Type | Description |
|-----------|------|-------------|
| `spender` | `Address` | Address spending the tokens (must be authenticated) |
| `owner` | `Address` | Address owning the tokens |
| `amount` | `i128` | Amount to spend (must be > 0) |

**Returns:** `Ok(())` on success.

**Errors:**
- `Error::InvalidAmount` - Amount must be > 0
- `Error::InsufficientAllowance` - Not enough allowance
- `Error::InsufficientBalance` - Not enough balance

---

### `pause(admin: Address) -> Result<(), Error>`

Pauses the vault (admin only).

| Parameter | Type | Description |
|-----------|------|-------------|
| `admin` | `Address` | Admin address (must be authenticated) |

**Returns:** `Ok(())` on success.

**Errors:**
- `Error::Unauthorized` - Caller is not the admin

---

### `unpause(admin: Address) -> Result<(), Error>`

Unpauses the vault (admin only).

| Parameter | Type | Description |
|-----------|------|-------------|
| `admin` | `Address` | Admin address (must be authenticated) |

**Returns:** `Ok(())` on success.

**Errors:**
- `Error::Unauthorized` - Caller is not the admin

---

### `is_paused() -> bool`

Returns whether the vault is paused.

**Returns:** `true` if paused, `false` otherwise.

---

### `get_config() -> Result<VaultConfig, Error>`

Returns the vault configuration.

**Returns:** `VaultConfig` containing owner, token, and paused status.

**Errors:**
- `Error::NotInitialized` - Vault not initialized

---

### `get_admin() -> Result<Address, Error>`

Returns the admin address.

**Returns:** Admin address.

**Errors:**
- `Error::NotInitialized` - Vault not initialized

---

## Error Codes

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` | Contract not initialized yet |
| 2 | `AlreadyInitialized` | Contract already initialized |
| 3 | `InvalidAmount` | Invalid amount (must be > 0) |
| 4 | `InsufficientBalance` | Insufficient balance |
| 5 | `InsufficientAllowance` | Insufficient allowance |
| 6 | `VaultPaused` | Vault is paused |
| 7 | `Unauthorized` | Unauthorized access |
| 8 | `TokenNotFound` | Token not found |
| 9 | `TransferFailed` | Transfer failed |

---

## Events

| Event | Description |
|-------|-------------|
| `deposit(user, amount)` | Emitted when tokens are deposited |
| `withdraw(user, amount)` | Emitted when tokens are withdrawn |
| `approval(owner, spender, amount)` | Emitted when an allowance is set |

---

## Testing

Run the tests:

```bash
cargo test
Output:

text
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
Build
Build the contract:

bash
cargo build --target wasm32-unknown-unknown --release
Usage Example
rust
// Initialize vault
client.initialize(&admin, &token);

// Deposit tokens
client.deposit(&user, &1000);

// Check balance
assert_eq!(client.balance(&user), 1000);

// Approve spender
client.approve(&user, &spender, &500);

// Spender uses allowance
client.spend_allowance(&spender, &user, &200);

// Withdraw tokens
client.withdraw(&user, &300);
License
MIT
