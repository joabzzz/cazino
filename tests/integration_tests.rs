#![cfg(feature = "sqlite")]

/// Integration tests for Cazino betting engine
/// These test full user workflows end-to-end
use cazino::db::SqliteDatabase;
use cazino::domain::models::{BetStatus, MarketStatus, Side};
use cazino::service::CazinoService;
use std::sync::Arc;

/// Helper to create an in-memory test database
async fn setup_test_db() -> CazinoService<SqliteDatabase> {
    let db = SqliteDatabase::new("sqlite::memory:").await.unwrap();
    db.run_migrations().await.unwrap();
    CazinoService::new(Arc::new(db))
}

#[tokio::test]
async fn test_full_market_lifecycle() {
    let service = setup_test_db().await;

    // 1. Create market
    let (market, admin) = service
        .create_market(
            "Test Market".to_string(),
            "admin-device".to_string(),
            "Admin".to_string(),
            "👑".to_string(),
            1000,
            24,
            None,
        )
        .await
        .unwrap();

    assert_eq!(market.name, "Test Market");
    assert_eq!(market.status, MarketStatus::Draft);
    assert_eq!(admin.balance, 1000);
    assert!(admin.is_admin);

    // 2. Join as users
    let (_, alice) = service
        .join_market(
            market.invite_code.clone(),
            "alice-device".to_string(),
            "Alice".to_string(),
            "👩".to_string(),
        )
        .await
        .unwrap();

    let (_, bob) = service
        .join_market(
            market.invite_code.clone(),
            "bob-device".to_string(),
            "Bob".to_string(),
            "👨".to_string(),
        )
        .await
        .unwrap();

    assert_eq!(alice.balance, 1000);
    assert!(!alice.is_admin);
    assert_eq!(bob.balance, 1000);

    // 3. Open market
    service.open_market(market.id, admin.id).await.unwrap();
    let market = service.get_market(market.id).await.unwrap();
    assert_eq!(market.status, MarketStatus::Open);

    // 4. Create bet about Bob
    let bet = service
        .create_bet(
            market.id,
            alice.id,
            bob.id,
            "Bob will fall asleep".to_string(),
            "1:1".to_string(),
            100,
        )
        .await
        .unwrap();

    assert_eq!(bet.status, BetStatus::Active);
    assert_eq!(bet.yes_pool, 100);
    assert_eq!(bet.no_pool, 0);

    // Alice's balance should be reduced
    let alice = service.get_user(alice.id).await.unwrap();
    assert_eq!(alice.balance, 900);

    // 6. Admin places wager
    let wager = service
        .place_wager(bet.id, admin.id, Side::No, 200)
        .await
        .unwrap();

    assert_eq!(wager.amount, 200);
    assert_eq!(wager.yes_pool_after, 100);
    assert_eq!(wager.no_pool_after, 200);
    // Probability should be 100/300 = 0.333...
    assert!((wager.probability_after - 0.333).abs() < 0.01);

    // 7. Resolve bet (YES wins)
    let payouts = service
        .resolve_bet(bet.id, admin.id, Side::Yes)
        .await
        .unwrap();

    // Alice wagered 100 on YES (as creator), admin wagered 200 on NO
    // Total pool: 300
    // Alice gets: (100/100) × 300 = 300 coins
    // Note: Alice is the only YES bettor (creators always bet YES)
    assert_eq!(payouts.len(), 1);
    assert_eq!(payouts[0].0, alice.id);
    assert_eq!(payouts[0].1, 300);

    // Check Alice's final balance
    let alice = service.get_user(alice.id).await.unwrap();
    assert_eq!(alice.balance, 1200); // 900 + 300

    // 8. Close market
    service.close_market(market.id, admin.id).await.unwrap();
    let market = service.get_market(market.id).await.unwrap();
    assert_eq!(market.status, MarketStatus::Closed);
}

#[tokio::test]
async fn test_hidden_bet_mechanics() {
    let service = setup_test_db().await;

    // Create market and users
    let (market, admin) = service
        .create_market(
            "Hidden Bet Test".to_string(),
            "admin-device".to_string(),
            "Admin".to_string(),
            "👑".to_string(),
            1000,
            24,
            None,
        )
        .await
        .unwrap();

    let (_, alice) = service
        .join_market(
            market.invite_code.clone(),
            "alice-device".to_string(),
            "Alice".to_string(),
            "👩".to_string(),
        )
        .await
        .unwrap();

    let (_, bob) = service
        .join_market(
            market.invite_code.clone(),
            "bob-device".to_string(),
            "Bob".to_string(),
            "👨".to_string(),
        )
        .await
        .unwrap();

    service.open_market(market.id, admin.id).await.unwrap();

    // Create bet about Bob
    let bet = service
        .create_bet(
            market.id,
            alice.id,
            bob.id,
            "Bob secret bet".to_string(),
            "1:1".to_string(),
            100,
        )
        .await
        .unwrap();

    service.approve_bet(bet.id, admin.id).await.unwrap();

    // Test 1: Bob can't see the bet description
    let bets = service.get_bets(market.id, bob.id).await.unwrap();
    assert_eq!(bets.len(), 1);
    assert!(bets[0].is_hidden);
    assert!(bets[0].description.is_none());

    // Test 2: Alice CAN see the bet
    let bets = service.get_bets(market.id, alice.id).await.unwrap();
    assert_eq!(bets.len(), 1);
    assert!(!bets[0].is_hidden);
    assert_eq!(bets[0].description, Some("Bob secret bet".to_string()));

    // Test 3: Bob can't wager on bet about himself
    let result = service.place_wager(bet.id, bob.id, Side::Yes, 100).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Cannot bet on bets about yourself"));

    // Test 4: After resolution, Bob CAN see the bet
    service
        .resolve_bet(bet.id, admin.id, Side::Yes)
        .await
        .unwrap();

    let bets = service.get_bets(market.id, bob.id).await.unwrap();
    assert!(!bets[0].is_hidden); // No longer hidden
    assert!(bets[0].description.is_some());
}

#[tokio::test]
async fn test_parimutuel_payout_calculation() {
    let service = setup_test_db().await;

    let (market, admin) = service
        .create_market(
            "Payout Test".to_string(),
            "admin-device".to_string(),
            "Admin".to_string(),
            "👑".to_string(),
            2000,
            24,
            None,
        )
        .await
        .unwrap();

    let (_, alice) = service
        .join_market(
            market.invite_code.clone(),
            "alice-device".to_string(),
            "Alice".to_string(),
            "👩".to_string(),
        )
        .await
        .unwrap();

    let (_, bob) = service
        .join_market(
            market.invite_code.clone(),
            "bob-device".to_string(),
            "Bob".to_string(),
            "👨".to_string(),
        )
        .await
        .unwrap();

    let (_, carol) = service
        .join_market(
            market.invite_code.clone(),
            "carol-device".to_string(),
            "Carol".to_string(),
            "👵".to_string(),
        )
        .await
        .unwrap();

    service.open_market(market.id, admin.id).await.unwrap();

    // Create and approve bet
    let bet = service
        .create_bet(
            market.id,
            admin.id,
            alice.id,
            "Test bet".to_string(),
            "1:1".to_string(),
            100,
        )
        .await
        .unwrap();

    service.approve_bet(bet.id, admin.id).await.unwrap();

    // Place wagers:
    // YES side: Admin (100 from creation), Bob (200), Carol (100) = 400 total
    // NO side: (none initially)
    service
        .place_wager(bet.id, bob.id, Side::Yes, 200)
        .await
        .unwrap();

    service
        .place_wager(bet.id, carol.id, Side::Yes, 100)
        .await
        .unwrap();

    // Add a NO bet so there's something to win from
    service
        .place_wager(bet.id, bob.id, Side::No, 100)
        .await
        .unwrap();

    // Total pool: 500 (400 YES + 100 NO)
    // If YES wins:
    //   Admin gets: (100/400) × 500 = 125 coins
    //   Bob gets: (200/400) × 500 = 250 coins
    //   Carol gets: (100/400) × 500 = 125 coins

    let payouts = service
        .resolve_bet(bet.id, admin.id, Side::Yes)
        .await
        .unwrap();

    assert_eq!(payouts.len(), 3);

    // Find each user's payout
    let admin_payout = payouts.iter().find(|(id, _)| *id == admin.id).unwrap().1;
    let bob_payout = payouts.iter().find(|(id, _)| *id == bob.id).unwrap().1;
    let carol_payout = payouts.iter().find(|(id, _)| *id == carol.id).unwrap().1;

    assert_eq!(admin_payout, 125);
    assert_eq!(bob_payout, 250);
    assert_eq!(carol_payout, 125);

    // Total should equal pool
    assert_eq!(admin_payout + bob_payout + carol_payout, 500);

    // Check final balances
    let admin_final = service.get_user(admin.id).await.unwrap();
    let bob_final = service.get_user(bob.id).await.unwrap();
    let carol_final = service.get_user(carol.id).await.unwrap();

    assert_eq!(admin_final.balance, 2000 - 100 + 125); // Started 2000, bet 100, won 125
    assert_eq!(bob_final.balance, 2000 - 200 - 100 + 250); // Started 2000, bet 200 YES + 100 NO, won 250
    assert_eq!(carol_final.balance, 2000 - 100 + 125); // Started 2000, bet 100, won 125
}

#[tokio::test]
async fn test_insufficient_balance() {
    let service = setup_test_db().await;

    let (market, admin) = service
        .create_market(
            "Balance Test".to_string(),
            "admin-device".to_string(),
            "Admin".to_string(),
            "👑".to_string(),
            100, // Small starting balance
            24,
            None,
        )
        .await
        .unwrap();

    let (_, alice) = service
        .join_market(
            market.invite_code.clone(),
            "alice-device".to_string(),
            "Alice".to_string(),
            "👩".to_string(),
        )
        .await
        .unwrap();

    service.open_market(market.id, admin.id).await.unwrap();

    // Try to create bet with more than balance
    let result = service
        .create_bet(
            market.id,
            alice.id,
            admin.id,
            "Expensive bet".to_string(),
            "1:1".to_string(),
            200, // More than her 100 balance
        )
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Insufficient"));
}

#[tokio::test]
async fn test_probability_chart_tracking() {
    let service = setup_test_db().await;

    let (market, admin) = service
        .create_market(
            "Chart Test".to_string(),
            "admin-device".to_string(),
            "Admin".to_string(),
            "👑".to_string(),
            1000,
            24,
            None,
        )
        .await
        .unwrap();

    let (_, alice) = service
        .join_market(
            market.invite_code.clone(),
            "alice-device".to_string(),
            "Alice".to_string(),
            "👩".to_string(),
        )
        .await
        .unwrap();

    service.open_market(market.id, admin.id).await.unwrap();

    let bet = service
        .create_bet(
            market.id,
            admin.id,
            alice.id,
            "Chart bet".to_string(),
            "1:1".to_string(),
            100,
        )
        .await
        .unwrap();

    service.approve_bet(bet.id, admin.id).await.unwrap();

    // Place multiple wagers to create chart data
    // Note: Can't bet on bets about yourself, so admin places wagers instead
    service
        .place_wager(bet.id, admin.id, Side::No, 100)
        .await
        .unwrap();

    service
        .place_wager(bet.id, admin.id, Side::Yes, 50)
        .await
        .unwrap();

    // Get probability chart
    let chart = service.get_probability_chart(bet.id).await.unwrap();

    // Should have 3 data points (opening wager + 2 additional wagers)
    assert_eq!(chart.len(), 3);

    // First point: 100 YES (opening wager), 0 NO = 100% YES
    assert!((chart[0].yes_probability - 1.0).abs() < 0.01);

    // Second point: 100 YES, 100 NO (first additional wager) = 50%
    assert!((chart[1].yes_probability - 0.5).abs() < 0.01);

    // Third point: 150 YES, 100 NO = 60%
    assert!((chart[2].yes_probability - 0.6).abs() < 0.01);
}

#[tokio::test]
async fn test_admin_only_actions() {
    let service = setup_test_db().await;

    let (market, admin) = service
        .create_market(
            "Admin Test".to_string(),
            "admin-device".to_string(),
            "Admin".to_string(),
            "👑".to_string(),
            1000,
            24,
            None,
        )
        .await
        .unwrap();

    let (_, alice) = service
        .join_market(
            market.invite_code.clone(),
            "alice-device".to_string(),
            "Alice".to_string(),
            "👩".to_string(),
        )
        .await
        .unwrap();

    service.open_market(market.id, admin.id).await.unwrap();

    let bet = service
        .create_bet(
            market.id,
            alice.id,
            admin.id,
            "Admin bet".to_string(),
            "1:1".to_string(),
            100,
        )
        .await
        .unwrap();

    // Non-admin tries to approve bet
    let result = service.approve_bet(bet.id, alice.id).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Admin only") || err_msg.contains("Constraint"),
        "Expected admin error but got: {}",
        err_msg
    );

    // Approve as admin
    service.approve_bet(bet.id, admin.id).await.unwrap();

    // Non-admin tries to resolve bet
    let result = service.resolve_bet(bet.id, alice.id, Side::Yes).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Admin only") || err_msg.contains("Constraint"),
        "Expected admin error but got: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_returning_user() {
    let service = setup_test_db().await;

    let (market, _admin) = service
        .create_market(
            "Returning User Test".to_string(),
            "admin-device".to_string(),
            "Admin".to_string(),
            "👑".to_string(),
            1000,
            24,
            None,
        )
        .await
        .unwrap();

    // Alice joins first time
    let (_, alice1) = service
        .join_market(
            market.invite_code.clone(),
            "alice-device".to_string(),
            "Alice".to_string(),
            "👩".to_string(),
        )
        .await
        .unwrap();

    // Alice "returns" with same device ID
    let (_, alice2) = service
        .join_market(
            market.invite_code.clone(),
            "alice-device".to_string(),  // Same device ID
            "Alice Updated".to_string(), // Different name (should be ignored)
            "👸".to_string(),            // Different avatar (should be ignored)
        )
        .await
        .unwrap();

    // Should get same user back
    assert_eq!(alice1.id, alice2.id);
    assert_eq!(alice2.display_name, "Alice"); // Original name preserved
    assert_eq!(alice2.avatar, "👩"); // Original avatar preserved
}

#[tokio::test]
async fn test_multiple_bets_same_subject() {
    let service = setup_test_db().await;

    let (market, admin) = service
        .create_market(
            "Multiple Bets Test".to_string(),
            "admin-device".to_string(),
            "Admin".to_string(),
            "👑".to_string(),
            1000,
            24,
            None,
        )
        .await
        .unwrap();

    let (_, alice) = service
        .join_market(
            market.invite_code.clone(),
            "alice-device".to_string(),
            "Alice".to_string(),
            "👩".to_string(),
        )
        .await
        .unwrap();

    service.open_market(market.id, admin.id).await.unwrap();

    // Create multiple bets about Alice
    let bet1 = service
        .create_bet(
            market.id,
            admin.id,
            alice.id,
            "Alice will be late".to_string(),
            "1:1".to_string(),
            50,
        )
        .await
        .unwrap();

    let bet2 = service
        .create_bet(
            market.id,
            admin.id,
            alice.id,
            "Alice will spill drink".to_string(),
            "1:1".to_string(),
            50,
        )
        .await
        .unwrap();

    service.approve_bet(bet1.id, admin.id).await.unwrap();
    service.approve_bet(bet2.id, admin.id).await.unwrap();

    // Alice should see 2 hidden bets
    let bets = service.get_bets(market.id, alice.id).await.unwrap();
    assert_eq!(bets.len(), 2);
    assert!(bets.iter().all(|b| b.is_hidden));

    // After resolution, Alice can see them
    service
        .resolve_bet(bet1.id, admin.id, Side::Yes)
        .await
        .unwrap();
    service
        .resolve_bet(bet2.id, admin.id, Side::No)
        .await
        .unwrap();

    let bets = service.get_bets(market.id, alice.id).await.unwrap();
    assert!(bets.iter().all(|b| !b.is_hidden));

    // Get bets about Alice for reveal screen
    let reveal_bets = service.get_bets_about_user(alice.id).await.unwrap();
    assert_eq!(reveal_bets.len(), 2);
}
