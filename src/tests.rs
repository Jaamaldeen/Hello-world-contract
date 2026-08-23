use super::{
    IdeaContract, IdeaContractClient, IdeaError, VoteCount, VoteDirection, VoteRecord,
};
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events},
    Address, Env, String,
};

#[contract]
pub struct MockReputationContract;

#[contractimpl]
impl MockReputationContract {
    pub fn reputation_score(_env: Env, _user: Address) -> i32 {
        // 200 / 100 = 2 vote weight
        200
    }
}

fn setup() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(IdeaContract, ());
    let reputation_contract_id = env.register(MockReputationContract, ());
    let author = Address::generate(&env);
    let other = Address::generate(&env);

    let client = IdeaContractClient::new(&env, &contract_id);
    client.set_reputation_contract(&reputation_contract_id);

    (env, contract_id, reputation_contract_id, author, other)
}

fn setup_without_reputation() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

use crate::{
    errors::VerifierError,
    types::{Resolution, ResolutionResult},
    PredictionVerifier, PredictionVerifierClient,
};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Seed data for a typical prediction scenario.
struct TestContext {
    env: Env,
    admin: Address,
    oracle: Address,
    rogue: Address,
    client: PredictionVerifierClient<'static>,
    prediction_id: i128,
    target_price: u128,
    deadline: u64,
}

impl TestContext {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Deploy the verifier contract
        let contract_id = env.register_contract(None, PredictionVerifier);
        // SAFETY: we own env and the lifetime here matches test scope
        let client: PredictionVerifierClient<'static> =
            unsafe { core::mem::transmute(PredictionVerifierClient::new(&env, &contract_id)) };

        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let rogue = Address::generate(&env);

        // Initialise — set deadline in the future (ledger starts at 0)
        client.init(&admin);
        client.set_authorized_oracle(&admin, &oracle);

        Self {
            env,
            admin,
            oracle,
            rogue,
            client,
            prediction_id: 42,
            target_price: 100_000_u128, // e.g. $100,000 in micro-units
            deadline: 1_000,
        }
    }

    /// Advance the mock ledger past the deadline.
    fn advance_past_deadline(&self) {
        self.env.ledger().with_mut(|l| {
            l.timestamp = self.deadline + 1;
        });
    }

    /// Helper: resolve with default values (actual >= target → Correct).
    fn resolve_correct(&self) -> Resolution {
        self.advance_past_deadline();
        self.client.resolve_prediction(
            &self.oracle,
            &self.prediction_id,
            &self.target_price,       // actual == target → Correct
            &self.target_price,
            &self.deadline,
            &(self.deadline + 1),
        );
        self.client
            .get_resolution(&self.prediction_id)
            .expect("resolution must exist after resolve_prediction")
    }
}

// ─── Init ────────────────────────────────────────────────────────────────────

#[test]
fn create_and_read_idea_stores_metadata() {
    let (env, contract_id, _, author, _) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

#[test]
#[should_panic(expected = "AlreadyInitialised")]
fn test_init_cannot_be_called_twice() {
    let ctx = TestContext::new();
    ctx.client.init(&ctx.admin); // second call must panic
}

// ─── Oracle whitelist ─────────────────────────────────────────────────────────

#[test]
fn test_set_authorized_oracle_adds_to_list() {
    let ctx = TestContext::new();
    let oracles = ctx.client.get_authorized_oracles();
    assert!(oracles.contains(&ctx.oracle));
}

#[test]
fn list_ideas_by_author_and_category_returns_matching_ids() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_add_oracle() {
    let ctx = TestContext::new();
    ctx.client
        .set_authorized_oracle(&ctx.rogue, &ctx.rogue);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_remove_oracle() {
    let ctx = TestContext::new();
    ctx.client
        .remove_authorized_oracle(&ctx.rogue, &ctx.oracle);
}

#[test]
fn test_multiple_oracles_can_be_whitelisted() {
    let ctx = TestContext::new();
    let oracle2 = Address::generate(&ctx.env);
    let oracle3 = Address::generate(&ctx.env);

    ctx.client.set_authorized_oracle(&ctx.admin, &oracle2);
    ctx.client.set_authorized_oracle(&ctx.admin, &oracle3);

    let list = ctx.client.get_authorized_oracles();
    assert_eq!(list.len(), 3);
}

// ─── Resolution: happy paths ──────────────────────────────────────────────────

#[test]
fn author_can_update_body_without_changing_metadata() {
    let (env, contract_id, _, author, _) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

#[test]
fn test_resolve_prediction_correct_when_actual_exceeds_target() {
    let ctx = TestContext::new();
    ctx.advance_past_deadline();

    ctx.client.resolve_prediction(
        &ctx.oracle,
        &ctx.prediction_id,
        &(ctx.target_price + 50_000),
        &ctx.target_price,
        &ctx.deadline,
        &(ctx.deadline + 1),
    );

    let res = ctx
        .client
        .get_resolution(&ctx.prediction_id)
        .unwrap();
    assert_eq!(res.result, ResolutionResult::Correct);
}

#[test]
fn delete_idea_soft_deletes_record() {
    let (env, contract_id, _, author, _) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    let idea_id = client.create_idea(
        &author,
        &text(&env, "Exit plan"),
        &text(&env, "Scale out after volatility expansion."),
        &text(&env, "strategy"),
    );

    let res = ctx
        .client
        .get_resolution(&ctx.prediction_id)
        .unwrap();
    assert_eq!(res.result, ResolutionResult::Incorrect);
}

#[test]
fn test_resolution_stores_all_fields_correctly() {
    let ctx = TestContext::new();
    ctx.advance_past_deadline();

    let actual_price = 98_765_u128;
    let ts = ctx.deadline + 5;

    ctx.client.resolve_prediction(
        &ctx.oracle,
        &ctx.prediction_id,
        &actual_price,
        &ctx.target_price,
        &ctx.deadline,
        &ts,
    );

    let res = ctx
        .client
        .get_resolution(&ctx.prediction_id)
        .unwrap();

    assert_eq!(res.prediction_id, ctx.prediction_id);
    assert_eq!(res.actual_price, actual_price);
    assert_eq!(res.oracle, ctx.oracle);
    assert_eq!(res.resolution_timestamp, ts);
    assert_eq!(res.result, ResolutionResult::Incorrect);
}

#[test]
fn update_by_non_author_returns_unauthorized_error() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

#[test]
#[should_panic(expected = "OracleNotAuthorized")]
fn test_resolve_panics_when_oracle_not_whitelisted() {
    let ctx = TestContext::new();
    ctx.advance_past_deadline();

    ctx.client.resolve_prediction(
        &ctx.rogue,
        &ctx.prediction_id,
        &ctx.target_price,
        &ctx.target_price,
        &ctx.deadline,
        &(ctx.deadline + 1),
    );
}

#[test]
#[should_panic(expected = "DeadlineNotReached")]
fn test_resolve_panics_before_deadline() {
    let ctx = TestContext::new();
    // Ledger timestamp starts at 0, deadline is 1_000 — no advance needed

    ctx.client.resolve_prediction(
        &ctx.oracle,
        &ctx.prediction_id,
        &ctx.target_price,
        &ctx.target_price,
        &ctx.deadline,
        &ctx.deadline,
    );
}

#[test]
fn delete_by_non_author_returns_unauthorized_error() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    // Second resolution on same prediction — must panic
    ctx.client.resolve_prediction(
        &ctx.oracle,
        &ctx.prediction_id,
        &ctx.target_price,
        &ctx.target_price,
        &ctx.deadline,
        &(ctx.deadline + 2),
    );
}

#[test]
#[should_panic(expected = "OracleNotAuthorized")]
fn test_revoked_oracle_cannot_resolve() {
    let ctx = TestContext::new();
    ctx.advance_past_deadline();

    // Revoke the oracle first
    ctx.client
        .remove_authorized_oracle(&ctx.admin, &ctx.oracle);

    // Now oracle tries to resolve — must be rejected
    ctx.client.resolve_prediction(
        &ctx.oracle,
        &ctx.prediction_id,
        &ctx.target_price,
        &ctx.target_price,
        &ctx.deadline,
        &(ctx.deadline + 1),
    );
}

// ─── Queries ─────────────────────────────────────────────────────────────────

#[test]
fn read_missing_idea_returns_not_found_error() {
    let (env, contract_id, _, _, _) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

#[test]
fn test_is_resolved_returns_false_before_resolve() {
    let ctx = TestContext::new();
    assert!(!ctx.client.is_resolved(&ctx.prediction_id));
}

#[test]
fn update_deleted_idea_returns_deleted_error() {
    let (env, contract_id, _, author, _) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

#[test]
fn test_independent_predictions_are_isolated() {
    let ctx = TestContext::new();
    ctx.advance_past_deadline();

    let pid_a: i128 = 1;
    let pid_b: i128 = 2;

    ctx.client.resolve_prediction(
        &ctx.oracle,
        &pid_a,
        &ctx.target_price,
        &ctx.target_price,
        &ctx.deadline,
        &(ctx.deadline + 1),
    );

    assert!(ctx.client.is_resolved(&pid_a));
    assert!(!ctx.client.is_resolved(&pid_b));

    let res_a = ctx.client.get_resolution(&pid_a).unwrap();
    assert_eq!(res_a.prediction_id, pid_a);
}

// ─── Mock oracle round-trip ───────────────────────────────────────────────────

/// Full scenario: admin onboards oracle, oracle resolves two predictions
/// with opposite outcomes, verify both stored correctly.
#[test]
fn create_update_delete_emit_events() {
    let (env, contract_id, _, author, _) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    let contract_id = env.register_contract(None, PredictionVerifier);
    let client = PredictionVerifierClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let mock_oracle = Address::generate(&env); // ← the "mock oracle" from the AC

#[test]
fn exposes_contract_error_codes() {
    assert_eq!(IdeaError::NotFound as u32, 1);
    assert_eq!(IdeaError::Unauthorized as u32, 2);
    assert_eq!(IdeaError::Deleted as u32, 3);
    assert_eq!(IdeaError::VoteAlreadyExists as u32, 4);
    assert_eq!(IdeaError::VoteNotFound as u32, 5);
    assert_eq!(IdeaError::ReputationContractNotConfigured as u32, 6);
    assert_eq!(IdeaError::InvalidVoteWeight as u32, 7);
}

#[test]
fn vote_records_weighted_upvote_and_updates_counts() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    let idea_id = client.create_idea(
        &author,
        &text(&env, "Weighted vote idea"),
        &text(&env, "Voting should be weighted by reputation."),
        &text(&env, "governance"),
    );

    client.vote(&idea_id, &other, &VoteDirection::Up);

    let vote = client.get_vote(&idea_id, &other).unwrap();
    assert_eq!(
        vote,
        VoteRecord {
            direction: VoteDirection::Up,
            weight: 2
        }
    );

    let counts = client.get_vote_count(&idea_id);
    assert_eq!(
        counts,
        VoteCount {
            upvotes: 2,
            downvotes: 0,
            net_score: 2
        }
    );
}

#[test]
fn vote_records_weighted_downvote_and_updates_counts() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    let idea_id = client.create_idea(
        &author,
        &text(&env, "Downvote idea"),
        &text(&env, "This one tests weighted downvotes."),
        &text(&env, "governance"),
    );

    client.vote(&idea_id, &other, &VoteDirection::Down);

    let vote = client.get_vote(&idea_id, &other).unwrap();
    assert_eq!(
        vote,
        VoteRecord {
            direction: VoteDirection::Down,
            weight: 2
        }
    );

    let counts = client.get_vote_count(&idea_id);
    assert_eq!(
        counts,
        VoteCount {
            upvotes: 0,
            downvotes: 2,
            net_score: -2
        }
    );
}

#[test]
fn change_vote_switches_direction_and_recomputes_counts() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    let idea_id = client.create_idea(
        &author,
        &text(&env, "Vote switching"),
        &text(&env, "Change vote from up to down."),
        &text(&env, "governance"),
    );

    client.vote(&idea_id, &other, &VoteDirection::Up);
    client.change_vote(&idea_id, &other, &VoteDirection::Down);

    let vote = client.get_vote(&idea_id, &other).unwrap();
    assert_eq!(
        vote,
        VoteRecord {
            direction: VoteDirection::Down,
            weight: 2
        }
    );

    let counts = client.get_vote_count(&idea_id);
    assert_eq!(
        counts,
        VoteCount {
            upvotes: 0,
            downvotes: 2,
            net_score: -2
        }
    );
}

#[test]
fn remove_vote_retracts_vote_and_resets_counts() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    let idea_id = client.create_idea(
        &author,
        &text(&env, "Retract vote"),
        &text(&env, "Vote can be removed."),
        &text(&env, "governance"),
    );

    client.vote(&idea_id, &other, &VoteDirection::Up);
    client.remove_vote(&idea_id, &other);

    let vote = client.get_vote(&idea_id, &other);
    assert_eq!(vote, None);

    let counts = client.get_vote_count(&idea_id);
    assert_eq!(
        counts,
        VoteCount {
            upvotes: 0,
            downvotes: 0,
            net_score: 0
        }
    );
}

#[test]
fn double_voting_is_prevented() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    let idea_id = client.create_idea(
        &author,
        &text(&env, "Prevent duplicate votes"),
        &text(&env, "Same voter should not be able to vote twice."),
        &text(&env, "governance"),
    );

    client.vote(&idea_id, &other, &VoteDirection::Up);

    assert_eq!(
        client.try_vote(&idea_id, &other, &VoteDirection::Down),
        Err(Ok(IdeaError::VoteAlreadyExists))
    );
}

#[test]
fn change_vote_without_existing_vote_returns_error() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    let idea_id = client.create_idea(
        &author,
        &text(&env, "Missing vote change"),
        &text(&env, "Changing a missing vote should fail."),
        &text(&env, "governance"),
    );

    assert_eq!(
        client.try_change_vote(&idea_id, &other, &VoteDirection::Down),
        Err(Ok(IdeaError::VoteNotFound))
    );
}

#[test]
fn remove_vote_without_existing_vote_returns_error() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    let idea_id = client.create_idea(
        &author,
        &text(&env, "Missing vote removal"),
        &text(&env, "Removing a missing vote should fail."),
        &text(&env, "governance"),
    );

    assert_eq!(
        client.try_remove_vote(&idea_id, &other),
        Err(Ok(IdeaError::VoteNotFound))
    );
}

#[test]
fn vote_without_reputation_contract_returns_error() {
    let (env, contract_id, author, other) = setup_without_reputation();
    let client = IdeaContractClient::new(&env, &contract_id);

    let idea_id = client.create_idea(
        &author,
        &text(&env, "No reputation contract"),
        &text(&env, "Voting should fail without reputation contract."),
        &text(&env, "governance"),
    );

    assert_eq!(
        client.try_vote(&idea_id, &other, &VoteDirection::Up),
        Err(Ok(IdeaError::ReputationContractNotConfigured))
    );
}

#[test]
fn vote_change_remove_emit_events() {
    let (env, contract_id, _, author, other) = setup();
    let client = IdeaContractClient::new(&env, &contract_id);

    let idea_id = client.create_idea(
        &author,
        &text(&env, "Voting events"),
        &text(&env, "Ensure vote events are emitted."),
        &text(&env, "testing"),
    );

    // create_idea emits 1 event
    client.vote(&idea_id, &other, &VoteDirection::Up);
    client.change_vote(&idea_id, &other, &VoteDirection::Down);
    client.remove_vote(&idea_id, &other);

    // 1 create + 3 voting events
    assert_eq!(env.events().all().len(), 4);
}

// ─── Moderation Contract Tests ────────────────────────────────────────────────

#[cfg(test)]
mod moderation_tests {
    use crate::moderation::*;
    use soroban_sdk::{
        testutils::Address as _,
        Address, Env, String,
    };

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let moderator = Address::generate(&env);
        let user = Address::generate(&env);

        initialize_moderation(&env, admin.clone());
        add_moderator(&env, admin.clone(), moderator.clone());

        (env, admin, moderator, user)
    }

    fn text(env: &Env, s: &str) -> String {
        String::from_str(env, s)
    }

    // ─── Initialization tests ────────────────────────────────────────────────

    #[test]
    fn initialize_moderation_sets_admin_and_flag_count() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);

        initialize_moderation(&env, admin.clone());

        assert_eq!(get_flag_count(&env), 0);
    }

    // ─── Moderator management tests ──────────────────────────────────────────

    #[test]
    fn admin_can_add_moderator() {
        let (env, admin, moderator, _) = setup();

        assert!(is_moderator(&env, moderator));
    }

    #[test]
    fn admin_can_remove_moderator() {
        let (env, admin, moderator, _) = setup();

        remove_moderator(&env, admin.clone(), moderator.clone());

        assert!(!is_moderator(&env, moderator));
    }

    #[test]
    fn non_admin_cannot_add_moderator() {
        let (env, admin, moderator, user) = setup();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            add_moderator(&env, user.clone(), moderator.clone());
        }));

        assert!(result.is_err());
    }

    #[test]
    fn non_admin_cannot_remove_moderator() {
        let (env, admin, moderator, user) = setup();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            remove_moderator(&env, user.clone(), moderator.clone());
        }));

        assert!(result.is_err());
    }

    // ─── Flag submission tests ───────────────────────────────────────────────

    #[test]
    fn user_can_flag_content_with_reason_and_notes() {
        let (env, _, _, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Spam,
            Some(text(&env, "Multiple promotional messages")),
        );

        assert_eq!(flag_id, 1);
        assert_eq!(get_flag_count(&env), 1);

        let flag = get_flag(&env, flag_id).unwrap();
        assert_eq!(flag.flag_id, 1);
        assert_eq!(flag.status, FlagStatus::Pending);
        assert_eq!(flag.reason, FlagReason::Spam);
    }

    #[test]
    fn flag_increments_count() {
        let (env, _, _, user) = setup();

        for i in 0..5 {
            flag_content(
                &env,
                user.clone(),
                text(&env, &format!("content_{}", i)),
                FlagReason::Abusive,
                None,
            );
        }

        assert_eq!(get_flag_count(&env), 5);
    }

    #[test]
    fn flag_with_empty_content_id_fails() {
        let (env, _, _, user) = setup();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            flag_content(
                &env,
                user.clone(),
                text(&env, ""),
                FlagReason::PolicyViolation,
                None,
            );
        }));

        assert!(result.is_err());
    }

    #[test]
    fn flag_content_with_different_reasons() {
        let (env, _, _, user) = setup();

        let reasons = vec![
            FlagReason::Spam,
            FlagReason::Abusive,
            FlagReason::PolicyViolation,
            FlagReason::Other,
        ];

        for (i, reason) in reasons.iter().enumerate() {
            let flag_id = flag_content(
                &env,
                user.clone(),
                text(&env, &format!("content_{}", i)),
                reason.clone(),
                None,
            );

            let flag = get_flag(&env, flag_id).unwrap();
            assert_eq!(flag.reason, reason.clone());
        }
    }

    // ─── Flag review tests ──────────────────────────────────────────────────

    #[test]
    fn moderator_can_confirm_flag() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        review_flag(&env, moderator, flag_id, true);

        let flag = get_flag(&env, flag_id).unwrap();
        assert_eq!(flag.status, FlagStatus::Confirmed);
        assert!(flag.reviewed_by.is_some());
        assert!(flag.reviewed_ledger.is_some());
    }

    #[test]
    fn moderator_can_reject_flag() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Spam,
            None,
        );

        review_flag(&env, moderator, flag_id, false);

        let flag = get_flag(&env, flag_id).unwrap();
        assert_eq!(flag.status, FlagStatus::Rejected);
    }

    #[test]
    fn non_moderator_cannot_review_flag() {
        let (env, _, _, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            review_flag(&env, user.clone(), flag_id, true);
        }));

        assert!(result.is_err());
    }

    #[test]
    fn cannot_review_non_pending_flag() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        review_flag(&env, moderator.clone(), flag_id, true);

        // Try to review again
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            review_flag(&env, moderator.clone(), flag_id, true);
        }));

        assert!(result.is_err());
    }

    #[test]
    fn review_nonexistent_flag_fails() {
        let (env, _, moderator, _) = setup();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            review_flag(&env, moderator, 999, true);
        }));

        assert!(result.is_err());
    }

    // ─── Appeal tests ───────────────────────────────────────────────────────

    #[test]
    fn content_owner_can_appeal_confirmed_flag() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        review_flag(&env, moderator, flag_id, true);

        appeal_flag(
            &env,
            user,
            flag_id,
            text(&env, "This content is not abusive"),
        );

        let flag = get_flag(&env, flag_id).unwrap();
        assert_eq!(flag.status, FlagStatus::Appealed);
        assert!(flag.appeal_notes.is_some());
    }

    #[test]
    fn cannot_appeal_pending_flag() {
        let (env, _, _, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            appeal_flag(&env, user, flag_id, text(&env, "appeal notes"));
        }));

        assert!(result.is_err());
    }

    #[test]
    fn cannot_appeal_rejected_flag() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Spam,
            None,
        );

        review_flag(&env, moderator, flag_id, false);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            appeal_flag(&env, user, flag_id, text(&env, "appeal notes"));
        }));

        assert!(result.is_err());
    }

    // ─── Appeal resolution tests ────────────────────────────────────────────

    #[test]
    fn moderator_can_grant_appeal() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        review_flag(&env, moderator.clone(), flag_id, true);
        appeal_flag(&env, user, flag_id, text(&env, "appeal notes"));

        resolve_appeal(&env, moderator, flag_id, true);

        let flag = get_flag(&env, flag_id).unwrap();
        assert_eq!(flag.status, FlagStatus::AppealGranted);
    }

    #[test]
    fn moderator_can_deny_appeal() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        review_flag(&env, moderator.clone(), flag_id, true);
        appeal_flag(&env, user, flag_id, text(&env, "appeal notes"));

        resolve_appeal(&env, moderator, flag_id, false);

        let flag = get_flag(&env, flag_id).unwrap();
        assert_eq!(flag.status, FlagStatus::AppealDenied);
    }

    #[test]
    fn non_moderator_cannot_resolve_appeal() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        review_flag(&env, moderator, flag_id, true);
        appeal_flag(&env, user.clone(), flag_id, text(&env, "appeal notes"));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            resolve_appeal(&env, user.clone(), flag_id, true);
        }));

        assert!(result.is_err());
    }

    #[test]
    fn cannot_resolve_non_appealed_flag() {
        let (env, _, moderator, _) = setup();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            resolve_appeal(&env, moderator, 999, true);
        }));

        assert!(result.is_err());
    }

    // ─── State machine enforcement tests ────────────────────────────────────

    #[test]
    fn flag_status_transitions_are_enforced() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        // Pending → Confirmed (valid)
        review_flag(&env, moderator.clone(), flag_id, true);
        assert_eq!(get_flag(&env, flag_id).unwrap().status, FlagStatus::Confirmed);

        // Confirmed → Appealed (valid)
        appeal_flag(&env, user, flag_id, text(&env, "appeal notes"));
        assert_eq!(get_flag(&env, flag_id).unwrap().status, FlagStatus::Appealed);

        // Appealed → AppealGranted (valid)
        resolve_appeal(&env, moderator, flag_id, true);
        assert_eq!(
            get_flag(&env, flag_id).unwrap().status,
            FlagStatus::AppealGranted
        );
    }

    // ─── Query tests ────────────────────────────────────────────────────────

    #[test]
    fn get_flag_returns_correct_data() {
        let (env, _, _, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_xyz"),
            FlagReason::PolicyViolation,
            Some(text(&env, "violates ToS section 5.2")),
        );

        let flag = get_flag(&env, flag_id).unwrap();

        assert_eq!(flag.flag_id, flag_id);
        assert_eq!(flag.content_id, text(&env, "content_xyz"));
        assert_eq!(flag.flagged_by, user);
        assert_eq!(flag.reason, FlagReason::PolicyViolation);
        assert_eq!(flag.notes, Some(text(&env, "violates ToS section 5.2")));
        assert_eq!(flag.status, FlagStatus::Pending);
    }

    #[test]
    fn get_nonexistent_flag_returns_none() {
        let (env, _, _, _) = setup();

        assert_eq!(get_flag(&env, 999), None);
    }

    #[test]
    fn get_flag_count_reflects_all_flags() {
        let (env, _, _, user) = setup();

        assert_eq!(get_flag_count(&env), 0);

        flag_content(
            &env,
            user.clone(),
            text(&env, "content_1"),
            FlagReason::Spam,
            None,
        );
        assert_eq!(get_flag_count(&env), 1);

        flag_content(
            &env,
            user.clone(),
            text(&env, "content_2"),
            FlagReason::Abusive,
            None,
        );
        assert_eq!(get_flag_count(&env), 2);
    }

    #[test]
    fn is_moderator_returns_correct_status() {
        let (env, _, moderator, user) = setup();

        assert!(is_moderator(&env, moderator));
        assert!(!is_moderator(&env, user));
    }

    // ─── Event emission tests ───────────────────────────────────────────────

    #[test]
    fn flag_content_emits_event() {
        let (env, _, _, user) = setup();

        flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Spam,
            None,
        );

        let events = env.events().all();
        assert!(events.len() > 0);
    }

    #[test]
    fn review_flag_emits_event() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        let initial_event_count = env.events().all().len();
        review_flag(&env, moderator, flag_id, true);
        let final_event_count = env.events().all().len();

        assert!(final_event_count > initial_event_count);
    }

    #[test]
    fn appeal_flag_emits_event() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        review_flag(&env, moderator, flag_id, true);

        let initial_event_count = env.events().all().len();
        appeal_flag(&env, user, flag_id, text(&env, "appeal notes"));
        let final_event_count = env.events().all().len();

        assert!(final_event_count > initial_event_count);
    }

    #[test]
    fn resolve_appeal_emits_event() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            None,
        );

        review_flag(&env, moderator.clone(), flag_id, true);
        appeal_flag(&env, user, flag_id, text(&env, "appeal notes"));

        let initial_event_count = env.events().all().len();
        resolve_appeal(&env, moderator, flag_id, true);
        let final_event_count = env.events().all().len();

        assert!(final_event_count > initial_event_count);
    }

    // ─── End-to-end flow tests ──────────────────────────────────────────────

    #[test]
    fn complete_moderation_workflow() {
        let (env, admin, moderator, user) = setup();

        // User flags content
        let flag_id = flag_content(
            &env,
            user.clone(),
            text(&env, "content_123"),
            FlagReason::Abusive,
            Some(text(&env, "Harassing language")),
        );
        assert_eq!(get_flag(&env, flag_id).unwrap().status, FlagStatus::Pending);

        // Moderator reviews and confirms
        review_flag(&env, moderator.clone(), flag_id, true);
        assert_eq!(
            get_flag(&env, flag_id).unwrap().status,
            FlagStatus::Confirmed
        );

        // Content owner appeals
        appeal_flag(&env, user.clone(), flag_id, text(&env, "False accusation"));
        assert_eq!(get_flag(&env, flag_id).unwrap().status, FlagStatus::Appealed);

        // Moderator grants appeal
        resolve_appeal(&env, moderator, flag_id, true);
        assert_eq!(
            get_flag(&env, flag_id).unwrap().status,
            FlagStatus::AppealGranted
        );
    }

    #[test]
    fn moderator_rejects_flag_workflow() {
        let (env, _, moderator, user) = setup();

        let flag_id = flag_content(
            &env,
            user,
            text(&env, "content_456"),
            FlagReason::Spam,
            Some(text(&env, "Legitimate promotional")),
        );

        review_flag(&env, moderator, flag_id, false);

        let flag = get_flag(&env, flag_id).unwrap();
        assert_eq!(flag.status, FlagStatus::Rejected);
    }
}
