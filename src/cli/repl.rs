/// Interactive CLI for testing Cazino locally
use crate::db::SqliteDatabase;
use crate::domain::models::Side;
use crate::service::CazinoService;
use std::io::{self, Write};
use uuid::Uuid;

pub struct Repl {
    service: CazinoService<SqliteDatabase>,
    current_market_id: Option<Uuid>,
    current_user_id: Option<Uuid>,
}

impl Repl {
    pub fn new(service: CazinoService<SqliteDatabase>) -> Self {
        Self {
            service,
            current_market_id: None,
            current_user_id: None,
        }
    }

    pub async fn run(&mut self) {
        println!("🎲 Welcome to Cazino CLI 🎲");
        println!("Type 'help' for available commands\n");

        loop {
            print!("> ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            let parts: Vec<&str> = input.split_whitespace().collect();
            let command = parts[0];

            match command {
                "help" => self.show_help(),
                "create" => self.create_market(&parts[1..]).await,
                "join" => self.join_market(&parts[1..]).await,
                "users" => self.list_users().await,
                "bet" => self.create_bet(&parts[1..]).await,
                "pending" => self.list_pending_bets().await,
                "approve" => self.approve_bet(&parts[1..]).await,
                "bets" => self.list_bets().await,
                "wager" => self.place_wager(&parts[1..]).await,
                "chart" => self.show_chart(&parts[1..]).await,
                "resolve" => self.resolve_bet(&parts[1..]).await,
                "leaderboard" => self.show_leaderboard().await,
                "reveal" => self.show_reveal(&parts[1..]).await,
                "open" => self.open_market().await,
                "close" => self.close_market().await,
                "status" => self.show_status().await,
                "quit" | "exit" => break,
                _ => println!("Unknown command: {}", command),
            }
        }

        println!("Thanks for playing! 🎰");
    }

    fn show_help(&self) {
        println!(
            r#"
Available Commands:
==================

Market Management:
  create <name> <hours>              Create a new market
  join <invite_code> <name> <emoji>  Join an existing market
  open                               Open market for betting
  close                              Close market (end betting)
  status                             Show current market status

Betting:
  bet <subject_name> <description> <odds> <amount>
                                     Create a bet about someone
  pending                            List bets awaiting approval
  approve <bet_index>                Approve a pending bet
  bets                               List all active bets
  wager <bet_index> <yes|no> <amount>
                                     Place a wager on a bet
  chart <bet_index>                  Show probability chart for a bet

Resolution:
  resolve <bet_index> <yes|no>       Resolve a bet
  leaderboard                        Show user rankings
  reveal <user_name>                 Show bets about a user

Other:
  users                              List all users in market
  help                               Show this help
  quit                               Exit the CLI

Examples:
  create "Thanksgiving 2024" 48
  join ABC123 "Alice" "👩"
  bet Bob "Bob falls asleep" "3:1" 100
  wager 1 yes 50
"#
        );
    }

    async fn create_market(&mut self, args: &[&str]) {
        if args.len() < 2 {
            println!("Usage: create <name> <hours>");
            return;
        }

        let name = args[0].to_string();
        let hours = args[1].parse::<i64>().unwrap_or(24);

        match self
            .service
            .create_market(
                name.clone(),
                "cli-admin".to_string(),
                "Admin".to_string(),
                "👑".to_string(),
                1000,
                hours,
            )
            .await
        {
            Ok((market, user)) => {
                println!("✅ Market created: {}", market.name);
                println!("   Invite code: {}", market.invite_code);
                println!("   Market ID: {}", market.id);
                self.current_market_id = Some(market.id);
                self.current_user_id = Some(user.id);
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn join_market(&mut self, args: &[&str]) {
        if args.len() < 3 {
            println!("Usage: join <invite_code> <name> <emoji>");
            return;
        }

        let invite_code = args[0].to_string();
        let name = args[1].to_string();
        let emoji = args[2].to_string();
        let device_id = format!("cli-{}", name.to_lowercase());

        match self
            .service
            .join_market(invite_code, device_id, name.clone(), emoji)
            .await
        {
            Ok((market, user)) => {
                println!("✅ Joined market: {}", market.name);
                println!(
                    "   Welcome, {} (balance: {} coins)",
                    user.display_name, user.balance
                );
                self.current_market_id = Some(market.id);
                self.current_user_id = Some(user.id);
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn list_users(&self) {
        let market_id = match self.current_market_id {
            Some(id) => id,
            None => {
                println!("❌ No market selected. Create or join a market first.");
                return;
            }
        };

        match self.service.get_users(market_id).await {
            Ok(users) => {
                println!("\nUsers in market:");
                for (i, user) in users.iter().enumerate() {
                    println!(
                        "  {}. {} {} - {} coins {}",
                        i + 1,
                        user.avatar,
                        user.display_name,
                        user.balance,
                        if user.is_admin { "👑" } else { "" }
                    );
                }
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn create_bet(&mut self, args: &[&str]) {
        if args.len() < 4 {
            println!("Usage: bet <subject_name> <description> <odds> <amount>");
            return;
        }

        let (market_id, user_id) = match (self.current_market_id, self.current_user_id) {
            (Some(m), Some(u)) => (m, u),
            _ => {
                println!("❌ No market/user selected");
                return;
            }
        };

        let subject_name = args[0];
        let description = args[1];
        let odds = args[2];
        let amount = args[3].parse::<i64>().unwrap_or(0);

        // Find subject user by name
        let users = self.service.get_users(market_id).await.unwrap();
        let subject = users
            .iter()
            .find(|u| u.display_name.to_lowercase() == subject_name.to_lowercase());

        let subject_id = match subject {
            Some(u) => u.id,
            None => {
                println!("❌ User '{}' not found", subject_name);
                return;
            }
        };

        match self
            .service
            .create_bet(
                market_id,
                user_id,
                subject_id,
                description.to_string(),
                "See what happens".to_string(),
                odds.to_string(),
                amount,
            )
            .await
        {
            Ok(bet) => {
                println!("✅ Bet created (pending approval)");
                println!("   ID: {}", bet.id);
                println!("   Description: {}", bet.description);
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn list_pending_bets(&self) {
        let market_id = match self.current_market_id {
            Some(id) => id,
            None => {
                println!("❌ No market selected");
                return;
            }
        };

        match self.service.get_pending_bets(market_id).await {
            Ok(bets) => {
                if bets.is_empty() {
                    println!("No pending bets");
                } else {
                    println!("\nPending bets:");
                    for (i, bet) in bets.iter().enumerate() {
                        println!("  {}. {}", i + 1, bet.description);
                    }
                }
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn approve_bet(&mut self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: approve <bet_index>");
            return;
        }

        let market_id = match self.current_market_id {
            Some(id) => id,
            None => {
                println!("❌ No market selected");
                return;
            }
        };

        let user_id = match self.current_user_id {
            Some(id) => id,
            None => {
                println!("❌ No user selected");
                return;
            }
        };

        let index = args[0].parse::<usize>().unwrap_or(0);
        let pending = self.service.get_pending_bets(market_id).await.unwrap();

        if index == 0 || index > pending.len() {
            println!("❌ Invalid bet index");
            return;
        }

        let bet_id = pending[index - 1].id;

        match self.service.approve_bet(bet_id, user_id).await {
            Ok(_) => println!("✅ Bet approved"),
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn list_bets(&self) {
        let (market_id, user_id) = match (self.current_market_id, self.current_user_id) {
            (Some(m), Some(u)) => (m, u),
            _ => {
                println!("❌ No market/user selected");
                return;
            }
        };

        match self.service.get_bets(market_id, user_id).await {
            Ok(bets) => {
                if bets.is_empty() {
                    println!("No active bets");
                } else {
                    println!("\nActive bets:");
                    for (i, bet) in bets.iter().enumerate() {
                        if bet.is_hidden {
                            println!(
                                "  {}. 🔒 [HIDDEN BET ABOUT YOU] - {} coins in pool",
                                i + 1,
                                bet.yes_pool + bet.no_pool
                            );
                        } else {
                            let prob = (bet.yes_pool as f64 / (bet.yes_pool + bet.no_pool) as f64
                                * 100.0) as i32;
                            println!(
                                "  {}. {} ({}% YES) - Pool: {} coins",
                                i + 1,
                                bet.description.as_ref().unwrap(),
                                prob,
                                bet.yes_pool + bet.no_pool
                            );
                        }
                    }
                }
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn place_wager(&mut self, args: &[&str]) {
        if args.len() < 3 {
            println!("Usage: wager <bet_index> <yes|no> <amount>");
            return;
        }

        let (market_id, user_id) = match (self.current_market_id, self.current_user_id) {
            (Some(m), Some(u)) => (m, u),
            _ => {
                println!("❌ No market/user selected");
                return;
            }
        };

        let index = args[0].parse::<usize>().unwrap_or(0);
        let side = match args[1].to_lowercase().as_str() {
            "yes" => Side::Yes,
            "no" => Side::No,
            _ => {
                println!("❌ Side must be 'yes' or 'no'");
                return;
            }
        };
        let amount = args[2].parse::<i64>().unwrap_or(0);

        let bets = self.service.get_bets(market_id, user_id).await.unwrap();

        if index == 0 || index > bets.len() {
            println!("❌ Invalid bet index");
            return;
        }

        let bet_id = bets[index - 1].id;

        match self
            .service
            .place_wager(bet_id, user_id, side, amount)
            .await
        {
            Ok(wager) => {
                println!("✅ Wager placed!");
                println!("   Amount: {} coins on {:?}", wager.amount, wager.side);
                println!(
                    "   New probability: {:.1}% YES",
                    wager.probability_after * 100.0
                );
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn show_chart(&self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: chart <bet_index>");
            return;
        }

        let (market_id, user_id) = match (self.current_market_id, self.current_user_id) {
            (Some(m), Some(u)) => (m, u),
            _ => {
                println!("❌ No market/user selected");
                return;
            }
        };

        let index = args[0].parse::<usize>().unwrap_or(0);
        let bets = self.service.get_bets(market_id, user_id).await.unwrap();

        if index == 0 || index > bets.len() {
            println!("❌ Invalid bet index");
            return;
        }

        let bet_id = bets[index - 1].id;

        match self.service.get_probability_chart(bet_id).await {
            Ok(points) => {
                println!("\nProbability Chart:");
                for point in points {
                    let bar_len = (point.yes_probability * 50.0) as usize;
                    let bar = "█".repeat(bar_len);
                    println!("{:.1}% | {}", point.yes_probability * 100.0, bar);
                }
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn resolve_bet(&mut self, args: &[&str]) {
        if args.len() < 2 {
            println!("Usage: resolve <bet_index> <yes|no>");
            return;
        }

        let (market_id, user_id) = match (self.current_market_id, self.current_user_id) {
            (Some(m), Some(u)) => (m, u),
            _ => {
                println!("❌ No market/user selected");
                return;
            }
        };

        let index = args[0].parse::<usize>().unwrap_or(0);
        let outcome = match args[1].to_lowercase().as_str() {
            "yes" => Side::Yes,
            "no" => Side::No,
            _ => {
                println!("❌ Outcome must be 'yes' or 'no'");
                return;
            }
        };

        let bets = self.service.get_bets(market_id, user_id).await.unwrap();

        if index == 0 || index > bets.len() {
            println!("❌ Invalid bet index");
            return;
        }

        let bet_id = bets[index - 1].id;

        match self.service.resolve_bet(bet_id, user_id, outcome).await {
            Ok(payouts) => {
                println!("✅ Bet resolved! Outcome: {:?}", outcome);
                if !payouts.is_empty() {
                    println!("\nPayouts:");
                    for (user_id, amount) in payouts {
                        println!("  User {}: {} coins", user_id, amount);
                    }
                }
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn show_leaderboard(&self) {
        let market_id = match self.current_market_id {
            Some(id) => id,
            None => {
                println!("❌ No market selected");
                return;
            }
        };

        match self.service.get_users(market_id).await {
            Ok(mut users) => {
                users.sort_by(|a, b| b.balance.cmp(&a.balance));

                println!("\n🏆 Leaderboard 🏆");
                for (i, user) in users.iter().enumerate() {
                    let profit = user.balance - 1000; // assuming starting balance of 1000
                    println!(
                        "  {}. {} {} - {} coins ({:+} profit)",
                        i + 1,
                        user.avatar,
                        user.display_name,
                        user.balance,
                        profit
                    );
                }
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn show_reveal(&self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: reveal <user_name>");
            return;
        }

        let market_id = match self.current_market_id {
            Some(id) => id,
            None => {
                println!("❌ No market selected");
                return;
            }
        };

        let name = args[0];
        let users = self.service.get_users(market_id).await.unwrap();
        let user = users
            .iter()
            .find(|u| u.display_name.to_lowercase() == name.to_lowercase());

        let user_id = match user {
            Some(u) => u.id,
            None => {
                println!("❌ User '{}' not found", name);
                return;
            }
        };

        match self.service.get_bets_about_user(user_id).await {
            Ok(bets) => {
                println!("\n🔓 Bets about {}:", name);
                for bet in bets {
                    println!("  - {} (Status: {:?})", bet.description, bet.status);
                }
            }
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn open_market(&mut self) {
        let (market_id, user_id) = match (self.current_market_id, self.current_user_id) {
            (Some(m), Some(u)) => (m, u),
            _ => {
                println!("❌ No market/user selected");
                return;
            }
        };

        match self.service.open_market(market_id, user_id).await {
            Ok(_) => println!("✅ Market opened for betting"),
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn close_market(&mut self) {
        let (market_id, user_id) = match (self.current_market_id, self.current_user_id) {
            (Some(m), Some(u)) => (m, u),
            _ => {
                println!("❌ No market/user selected");
                return;
            }
        };

        match self.service.close_market(market_id, user_id).await {
            Ok(_) => println!("✅ Market closed"),
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    async fn show_status(&self) {
        match (self.current_market_id, self.current_user_id) {
            (Some(m_id), Some(u_id)) => {
                println!("\nCurrent Session:");
                println!("  Market ID: {}", m_id);
                println!("  User ID: {}", u_id);
            }
            _ => {
                println!("❌ No active session");
            }
        }
    }
}
