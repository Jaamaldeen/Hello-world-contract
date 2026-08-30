//! Content Moderation Contract
//!
//! Implements community-driven content moderation with:
//! - Flagging: any user can flag content
//! - Moderator review: designated moderators approve/reject flags
//! - Appeal: content owners can appeal moderation decisions
//! - On-chain audit log: all actions are recorded as events

use soroban_sdk::{
    contracttype, symbol_short, Address, Env, String,
};

// ── Data types ────────────────────────────────────────────────────────────────

/// Reason a piece of content was flagged.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlagReason {
    /// Content is spam or unsolicited
    Spam,
    /// Content is abusive or harassing
    Abusive,
    /// Content violates platform rules
    PolicyViolation,
    /// Other reason (described in flag notes)
    Other,
}

/// Current state of a content flag.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlagStatus {
    /// Flag submitted, awaiting moderator review
    Pending,
    /// Moderator confirmed — content removed/restricted
    Confirmed,
    /// Moderator rejected — content restored/cleared
    Rejected,
    /// Content owner appealed a Confirmed decision
    Appealed,
    /// Appeal reviewed — original decision upheld
    AppealDenied,
    /// Appeal reviewed — decision reversed, content restored
    AppealGranted,
}

/// A flag submitted against a piece of content.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContentFlag {
    /// Unique identifier for this flag
    pub flag_id: u64,
    /// ID of the flagged content (off-chain reference)
    pub content_id: String,
    /// Address that submitted the flag
    pub flagged_by: Address,
    /// Reason for flagging
    pub reason: FlagReason,
    /// Optional notes from the flagger
    pub notes: Option<String>,
    /// Current status of this flag
    pub status: FlagStatus,
    /// Ledger when flag was submitted
    pub submitted_ledger: u32,
    /// Moderator who reviewed (None if still Pending)
    pub reviewed_by: Option<Address>,
    /// Ledger when reviewed (None if still Pending)
    pub reviewed_ledger: Option<u32>,
    /// Appeal notes from content owner (None if not appealed)
    pub appeal_notes: Option<String>,
}

/// Storage keys for the moderation contract.
#[contracttype]
#[derive(Clone, Debug)]
pub enum ModerationKey {
    /// Total flag count (used as next flag ID)
    FlagCount,
    /// A specific flag by ID
    Flag(u64),
    /// List of flag IDs for a content item
    ContentFlags(String),
    /// Whether an address is a moderator
    IsModerator(Address),
    /// Admin address (can add/remove moderators)
    Admin,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModerationError {
    /// Caller is not an authorized moderator
    NotModerator = 1,
    /// Caller is not the contract admin
    NotAdmin = 2,
    /// Flag not found
    FlagNotFound = 3,
    /// Flag is not in Pending status (cannot review)
    FlagNotPending = 4,
    /// Flag is not in Confirmed status (cannot appeal)
    FlagNotConfirmed = 5,
    /// Flag is not in Appealed status (cannot resolve appeal)
    FlagNotAppealed = 6,
    /// Caller is not the content owner (cannot appeal)
    NotContentOwner = 7,
    /// Content ID is empty
    EmptyContentId = 8,
}

impl ModerationError {
    pub fn panic_with_error(self, env: &Env) -> ! {
        let code = match self {
            ModerationError::NotModerator => 1,
            ModerationError::NotAdmin => 2,
            ModerationError::FlagNotFound => 3,
            ModerationError::FlagNotPending => 4,
            ModerationError::FlagNotConfirmed => 5,
            ModerationError::FlagNotAppealed => 6,
            ModerationError::NotContentOwner => 7,
            ModerationError::EmptyContentId => 8,
        };
        env.panic_with_error(code);
    }
}

// ── Contract functions ────────────────────────────────────────────────────────

/// Initializes the moderation contract with an admin address.
/// Must be called once before other functions.
pub fn initialize_moderation(env: &Env, admin: Address) {
    admin.require_auth();
    env.storage().instance().set(&ModerationKey::Admin, &admin);
    env.storage().instance().set(&ModerationKey::FlagCount, &0u64);
}

/// Adds a moderator address. Admin only.
pub fn add_moderator(env: &Env, caller: Address, moderator: Address) {
    caller.require_auth();

    let admin: Address = env
        .storage()
        .instance()
        .get(&ModerationKey::Admin)
        .unwrap_or_else(|| ModerationError::NotAdmin.panic_with_error(env));

    if caller != admin {
        ModerationError::NotAdmin.panic_with_error(env);
    }

    env.storage()
        .persistent()
        .set(&ModerationKey::IsModerator(moderator.clone()), &true);

    // Log: moderator added
    env.events().publish(
        (symbol_short!("mod_add"), moderator.clone()),
        caller,
    );
}

/// Removes a moderator address. Admin only.
pub fn remove_moderator(env: &Env, caller: Address, moderator: Address) {
    caller.require_auth();

    let admin: Address = env
        .storage()
        .instance()
        .get(&ModerationKey::Admin)
        .unwrap_or_else(|| ModerationError::NotAdmin.panic_with_error(env));

    if caller != admin {
        ModerationError::NotAdmin.panic_with_error(env);
    }

    env.storage()
        .persistent()
        .remove(&ModerationKey::IsModerator(moderator.clone()));

    env.events().publish(
        (symbol_short!("mod_rem"), moderator.clone()),
        caller,
    );
}

/// Flags content as inappropriate. Any authenticated user can flag.
///
/// Returns the new flag_id.
pub fn flag_content(
    env: &Env,
    caller: Address,
    content_id: String,
    reason: FlagReason,
    notes: Option<String>,
) -> u64 {
    caller.require_auth();

    if content_id.len() == 0 {
        ModerationError::EmptyContentId.panic_with_error(env);
    }

    let flag_id: u64 = env
        .storage()
        .instance()
        .get(&ModerationKey::FlagCount)
        .unwrap_or(0);

    let next_id = flag_id + 1;

    let flag = ContentFlag {
        flag_id: next_id,
        content_id: content_id.clone(),
        flagged_by: caller.clone(),
        reason: reason.clone(),
        notes,
        status: FlagStatus::Pending,
        submitted_ledger: env.ledger().sequence(),
        reviewed_by: None,
        reviewed_ledger: None,
        appeal_notes: None,
    };

    env.storage()
        .persistent()
        .set(&ModerationKey::Flag(next_id), &flag);

    env.storage()
        .instance()
        .set(&ModerationKey::FlagCount, &next_id);

    // Log: content flagged
    env.events().publish(
        (symbol_short!("flagged"), next_id),
        (caller, content_id, reason),
    );

    next_id
}

/// Moderator reviews a pending flag.
/// action: true = confirm (remove content), false = reject (restore content)
pub fn review_flag(env: &Env, moderator: Address, flag_id: u64, action: bool) {
    moderator.require_auth();

    // Verify moderator
    let is_mod: bool = env
        .storage()
        .persistent()
        .get(&ModerationKey::IsModerator(moderator.clone()))
        .unwrap_or(false);

    if !is_mod {
        ModerationError::NotModerator.panic_with_error(env);
    }

    let mut flag: ContentFlag = env
        .storage()
        .persistent()
        .get(&ModerationKey::Flag(flag_id))
        .unwrap_or_else(|| ModerationError::FlagNotFound.panic_with_error(env));

    if flag.status != FlagStatus::Pending {
        ModerationError::FlagNotPending.panic_with_error(env);
    }

    flag.status = if action {
        FlagStatus::Confirmed
    } else {
        FlagStatus::Rejected
    };
    flag.reviewed_by = Some(moderator.clone());
    flag.reviewed_ledger = Some(env.ledger().sequence());

    env.storage()
        .persistent()
        .set(&ModerationKey::Flag(flag_id), &flag);

    // Log: flag reviewed (on-chain audit)
    env.events().publish(
        (symbol_short!("reviewed"), flag_id),
        (moderator, action, env.ledger().sequence()),
    );
}

/// Content owner appeals a Confirmed flag decision.
pub fn appeal_flag(env: &Env, caller: Address, flag_id: u64, appeal_notes: String) {
    caller.require_auth();

    let mut flag: ContentFlag = env
        .storage()
        .persistent()
        .get(&ModerationKey::Flag(flag_id))
        .unwrap_or_else(|| ModerationError::FlagNotFound.panic_with_error(env));

    if flag.status != FlagStatus::Confirmed {
        ModerationError::FlagNotConfirmed.panic_with_error(env);
    }

    // Only the original flagger's target (content owner) can appeal
    // In this model, the caller asserts they own the content
    // A more complete implementation would verify content_owner on-chain

    flag.status = FlagStatus::Appealed;
    flag.appeal_notes = Some(appeal_notes.clone());

    env.storage()
        .persistent()
        .set(&ModerationKey::Flag(flag_id), &flag);

    env.events().publish(
        (symbol_short!("appealed"), flag_id),
        (caller, appeal_notes),
    );
}

/// Moderator resolves an appeal.
/// grant: true = reverse decision (restore content), false = uphold (keep removed)
pub fn resolve_appeal(env: &Env, moderator: Address, flag_id: u64, grant: bool) {
    moderator.require_auth();

    let is_mod: bool = env
        .storage()
        .persistent()
        .get(&ModerationKey::IsModerator(moderator.clone()))
        .unwrap_or(false);

    if !is_mod {
        ModerationError::NotModerator.panic_with_error(env);
    }

    let mut flag: ContentFlag = env
        .storage()
        .persistent()
        .get(&ModerationKey::Flag(flag_id))
        .unwrap_or_else(|| ModerationError::FlagNotFound.panic_with_error(env));

    if flag.status != FlagStatus::Appealed {
        ModerationError::FlagNotAppealed.panic_with_error(env);
    }

    flag.status = if grant {
        FlagStatus::AppealGranted
    } else {
        FlagStatus::AppealDenied
    };
    flag.reviewed_by = Some(moderator.clone());
    flag.reviewed_ledger = Some(env.ledger().sequence());

    env.storage()
        .persistent()
        .set(&ModerationKey::Flag(flag_id), &flag);

    env.events().publish(
        (symbol_short!("appeal_rv"), flag_id),
        (moderator, grant),
    );
}

/// Returns a flag by ID.
pub fn get_flag(env: &Env, flag_id: u64) -> Option<ContentFlag> {
    env.storage().persistent().get(&ModerationKey::Flag(flag_id))
}

/// Returns total number of flags submitted.
pub fn get_flag_count(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&ModerationKey::FlagCount)
        .unwrap_or(0)
}

/// Returns whether an address is a moderator.
pub fn is_moderator(env: &Env, address: Address) -> bool {
    env.storage()
        .persistent()
        .get(&ModerationKey::IsModerator(address))
        .unwrap_or(false)
}
