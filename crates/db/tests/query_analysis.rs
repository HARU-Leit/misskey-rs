//! Database Query Analysis Tests
//!
//! These tests analyze the performance of common database queries using EXPLAIN ANALYZE.
//! They require a running `PostgreSQL` database with test data.
//!
//! Run with:
//! ```bash
//! docker-compose -f docker-compose.test.yml up -d
//! cargo test --features query-analysis -- query_analysis --nocapture
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::needless_pass_by_value
)]
#![cfg(feature = "query-analysis")]

use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

const DATABASE_URL: &str = "postgres://misskey_test:misskey_test@localhost:5433/misskey_test";

/// Check if query analysis tests should be skipped (e.g., in CI).
fn should_skip() -> bool {
    std::env::var("SKIP_QUERY_ANALYSIS").is_ok()
}

/// Macro to skip test if `SKIP_QUERY_ANALYSIS` is set.
macro_rules! skip_if_ci {
    () => {
        if should_skip() {
            eprintln!("Skipping query analysis test (SKIP_QUERY_ANALYSIS is set)");
            return;
        }
    };
}

/// Query analysis result
#[derive(Debug)]
#[allow(dead_code)]
struct QueryPlan {
    query_name: String,
    planning_time_ms: f64,
    execution_time_ms: f64,
    total_cost: f64,
    uses_index: bool,
    rows_scanned: i64,
    plan_text: String,
}

impl QueryPlan {
    fn from_explain_output(query_name: &str, rows: Vec<String>) -> Self {
        let plan_text = rows.join("\n");

        // Parse timing from EXPLAIN ANALYZE output
        let planning_time = rows
            .iter()
            .find(|r| r.contains("Planning Time:"))
            .and_then(|r| r.split(':').next_back())
            .and_then(|s| s.trim().trim_end_matches(" ms").parse::<f64>().ok())
            .unwrap_or(0.0);

        let execution_time = rows
            .iter()
            .find(|r| r.contains("Execution Time:"))
            .and_then(|r| r.split(':').next_back())
            .and_then(|s| s.trim().trim_end_matches(" ms").parse::<f64>().ok())
            .unwrap_or(0.0);

        // Check for index usage
        let uses_index = plan_text.contains("Index Scan")
            || plan_text.contains("Index Only Scan")
            || plan_text.contains("Bitmap Index Scan");

        // Parse total cost from first line (format: "cost=0.00..XX.XX")
        let total_cost = rows
            .first()
            .and_then(|r| {
                r.find("cost=").map(|start| {
                    let cost_str = &r[start + 5..];
                    cost_str
                        .split("..")
                        .nth(1)
                        .and_then(|s| s.split_whitespace().next())
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0)
                })
            })
            .unwrap_or(0.0);

        // Parse actual rows
        let rows_scanned = rows
            .iter()
            .filter_map(|r| {
                if r.contains("actual time=") && r.contains("rows=") {
                    r.find("rows=").and_then(|start| {
                        let rest = &r[start + 5..];
                        rest.split_whitespace()
                            .next()
                            .and_then(|s| s.parse::<i64>().ok())
                    })
                } else {
                    None
                }
            })
            .sum();

        Self {
            query_name: query_name.to_string(),
            planning_time_ms: planning_time,
            execution_time_ms: execution_time,
            total_cost,
            uses_index,
            rows_scanned,
            plan_text,
        }
    }

    fn print_summary(&self) {
        println!("\n{}", "=".repeat(60));
        println!("Query: {}", self.query_name);
        println!("{}", "=".repeat(60));
        println!("Planning Time:  {:.3} ms", self.planning_time_ms);
        println!("Execution Time: {:.3} ms", self.execution_time_ms);
        println!("Total Cost:     {:.2}", self.total_cost);
        println!(
            "Uses Index:     {}",
            if self.uses_index { "YES" } else { "NO ⚠️" }
        );
        println!("Rows Scanned:   {}", self.rows_scanned);
        println!("\nPlan:\n{}", self.plan_text);
    }

    fn assert_performance(&self, max_time_ms: f64) {
        assert!(
            self.execution_time_ms <= max_time_ms,
            "{}: Execution time {:.3}ms exceeds maximum {:.3}ms",
            self.query_name,
            self.execution_time_ms,
            max_time_ms
        );
    }

    fn assert_uses_index(&self) {
        assert!(
            self.uses_index,
            "{}: Query should use an index but performed sequential scan",
            self.query_name
        );
    }
}

async fn run_explain_analyze(
    db: &sea_orm::DatabaseConnection,
    query_name: &str,
    sql: &str,
) -> QueryPlan {
    let explain_sql = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) {sql}");

    let rows: Vec<String> = db
        .query_all(Statement::from_string(DbBackend::Postgres, explain_sql))
        .await
        .expect("Failed to execute EXPLAIN ANALYZE")
        .into_iter()
        .filter_map(|row| row.try_get_by_index::<String>(0).ok())
        .collect();

    QueryPlan::from_explain_output(query_name, rows)
}

async fn setup_test_data(db: &sea_orm::DatabaseConnection) {
    // Create tables if they don't exist (run migrations)
    let _ = db
        .execute(Statement::from_string(
            DbBackend::Postgres,
            r#"
        CREATE TABLE IF NOT EXISTS "user" (
            id VARCHAR(32) PRIMARY KEY,
            username VARCHAR(128) NOT NULL,
            username_lower VARCHAR(128) NOT NULL,
            host VARCHAR(512),
            token VARCHAR(512) UNIQUE,
            name VARCHAR(128),
            is_admin BOOLEAN NOT NULL DEFAULT false,
            is_moderator BOOLEAN NOT NULL DEFAULT false,
            is_bot BOOLEAN NOT NULL DEFAULT false,
            is_cat BOOLEAN NOT NULL DEFAULT false,
            is_locked BOOLEAN NOT NULL DEFAULT false,
            is_suspended BOOLEAN NOT NULL DEFAULT false,
            followers_count INTEGER NOT NULL DEFAULT 0,
            following_count INTEGER NOT NULL DEFAULT 0,
            notes_count INTEGER NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ
        );

        CREATE INDEX IF NOT EXISTS idx_user_username_lower_host ON "user" (username_lower, host);
        CREATE INDEX IF NOT EXISTS idx_user_host ON "user" (host);
        CREATE INDEX IF NOT EXISTS idx_user_token ON "user" (token);
        "#,
        ))
        .await;

    let _ = db
        .execute(Statement::from_string(
            DbBackend::Postgres,
            r"
        CREATE TABLE IF NOT EXISTS note (
            id VARCHAR(32) PRIMARY KEY,
            user_id VARCHAR(32) NOT NULL,
            user_host VARCHAR(512),
            text TEXT,
            cw VARCHAR(512),
            visibility VARCHAR(16) NOT NULL DEFAULT 'public',
            reply_id VARCHAR(32),
            renote_id VARCHAR(32),
            thread_id VARCHAR(32),
            mentions JSONB NOT NULL DEFAULT '[]',
            visible_user_ids JSONB NOT NULL DEFAULT '[]',
            file_ids JSONB NOT NULL DEFAULT '[]',
            tags JSONB NOT NULL DEFAULT '[]',
            reactions JSONB NOT NULL DEFAULT '{}',
            replies_count INTEGER NOT NULL DEFAULT 0,
            renote_count INTEGER NOT NULL DEFAULT 0,
            reaction_count INTEGER NOT NULL DEFAULT 0,
            is_local BOOLEAN NOT NULL DEFAULT true,
            uri VARCHAR(512) UNIQUE,
            url VARCHAR(512),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ
        );

        CREATE INDEX IF NOT EXISTS idx_note_user_id ON note (user_id);
        CREATE INDEX IF NOT EXISTS idx_note_visibility ON note (visibility);
        CREATE INDEX IF NOT EXISTS idx_note_created_at ON note (created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_note_reply_id ON note (reply_id);
        CREATE INDEX IF NOT EXISTS idx_note_renote_id ON note (renote_id);
        CREATE INDEX IF NOT EXISTS idx_note_thread_id ON note (thread_id);
        CREATE INDEX IF NOT EXISTS idx_note_user_visibility ON note (user_id, visibility);
        CREATE INDEX IF NOT EXISTS idx_note_is_local_visibility ON note (is_local, visibility);
        CREATE INDEX IF NOT EXISTS idx_note_uri ON note (uri);
        ",
        ))
        .await;

    let _ = db
        .execute(Statement::from_string(
            DbBackend::Postgres,
            r"
        CREATE TABLE IF NOT EXISTS following (
            id VARCHAR(32) PRIMARY KEY,
            follower_id VARCHAR(32) NOT NULL,
            followee_id VARCHAR(32) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(follower_id, followee_id)
        );

        CREATE INDEX IF NOT EXISTS idx_following_follower ON following (follower_id);
        CREATE INDEX IF NOT EXISTS idx_following_followee ON following (followee_id);
        ",
        ))
        .await;

    let _ = db
        .execute(Statement::from_string(
            DbBackend::Postgres,
            r"
        CREATE TABLE IF NOT EXISTS reaction (
            id VARCHAR(32) PRIMARY KEY,
            user_id VARCHAR(32) NOT NULL,
            note_id VARCHAR(32) NOT NULL,
            reaction VARCHAR(256) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(user_id, note_id)
        );

        CREATE INDEX IF NOT EXISTS idx_reaction_note ON reaction (note_id);
        CREATE INDEX IF NOT EXISTS idx_reaction_user ON reaction (user_id);
        ",
        ))
        .await;

    // Insert test data
    for i in 0..100 {
        let user_id = format!("user{i:04}");
        let _ = db
            .execute(Statement::from_string(
                DbBackend::Postgres,
                format!(
                    r#"INSERT INTO "user" (id, username, username_lower, host, created_at)
                   VALUES ('{user_id}', 'user{i}', 'user{i}', NULL, NOW())
                   ON CONFLICT (id) DO NOTHING"#
                ),
            ))
            .await;
    }

    // Insert test notes (1000 notes)
    for i in 0..1000 {
        let note_id = format!("note{i:06}");
        let user_id = format!("user{:04}", i % 100);
        let visibility = if i % 10 == 0 { "home" } else { "public" };
        let is_local = i % 5 != 0;

        let _ = db.execute(Statement::from_string(
            DbBackend::Postgres,
            format!(
                r"INSERT INTO note (id, user_id, text, visibility, is_local, created_at)
                   VALUES ('{note_id}', '{user_id}', 'Test note content {i}', '{visibility}', {is_local}, NOW() - INTERVAL '{i} minutes')
                   ON CONFLICT (id) DO NOTHING"
            ),
        )).await;
    }

    // Insert followings
    for i in 0..200 {
        let follower = format!("user{:04}", i % 100);
        let followee = format!("user{:04}", (i + 1) % 100);
        let _ = db
            .execute(Statement::from_string(
                DbBackend::Postgres,
                format!(
                    r"INSERT INTO following (id, follower_id, followee_id, created_at)
                   VALUES ('follow{i:04}', '{follower}', '{followee}', NOW())
                   ON CONFLICT (follower_id, followee_id) DO NOTHING"
                ),
            ))
            .await;
    }
}

#[tokio::test]
async fn analyze_note_by_id_query() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    let plan = run_explain_analyze(
        &db,
        "Note by ID",
        "SELECT * FROM note WHERE id = 'note000001'",
    )
    .await;

    plan.print_summary();
    plan.assert_uses_index();
    plan.assert_performance(10.0);
}

#[tokio::test]
async fn analyze_notes_by_user_query() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    let plan = run_explain_analyze(
        &db,
        "Notes by User (paginated)",
        "SELECT * FROM note WHERE user_id = 'user0001' ORDER BY id DESC LIMIT 20",
    )
    .await;

    plan.print_summary();
    plan.assert_uses_index();
    plan.assert_performance(50.0);
}

#[tokio::test]
async fn analyze_local_timeline_query() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    let plan = run_explain_analyze(
        &db,
        "Local Timeline",
        "SELECT * FROM note WHERE visibility = 'public' AND is_local = true ORDER BY id DESC LIMIT 20"
    ).await;

    plan.print_summary();
    plan.assert_uses_index();
    plan.assert_performance(100.0);
}

#[tokio::test]
async fn analyze_global_timeline_query() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    let plan = run_explain_analyze(
        &db,
        "Global Timeline",
        "SELECT * FROM note WHERE visibility = 'public' ORDER BY id DESC LIMIT 20",
    )
    .await;

    plan.print_summary();
    plan.assert_uses_index();
    plan.assert_performance(100.0);
}

#[tokio::test]
async fn analyze_home_timeline_query() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    // Home timeline with IN clause for followed users
    let plan = run_explain_analyze(
        &db,
        "Home Timeline",
        r"
        SELECT n.* FROM note n
        WHERE n.user_id IN (
            SELECT followee_id FROM following WHERE follower_id = 'user0001'
            UNION
            SELECT 'user0001'
        )
        AND n.visibility IN ('public', 'home', 'followers')
        ORDER BY n.id DESC
        LIMIT 20
        ",
    )
    .await;

    plan.print_summary();
    plan.assert_performance(200.0);
}

#[tokio::test]
async fn analyze_user_by_username_query() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    let plan = run_explain_analyze(
        &db,
        "User by Username (local)",
        r#"SELECT * FROM "user" WHERE username_lower = 'user1' AND host IS NULL"#,
    )
    .await;

    plan.print_summary();
    plan.assert_uses_index();
    plan.assert_performance(10.0);
}

#[tokio::test]
async fn analyze_followers_query() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    let plan = run_explain_analyze(
        &db,
        "User Followers",
        r#"
        SELECT u.* FROM "user" u
        JOIN following f ON u.id = f.follower_id
        WHERE f.followee_id = 'user0001'
        ORDER BY f.created_at DESC
        LIMIT 20
        "#,
    )
    .await;

    plan.print_summary();
    plan.assert_uses_index();
    plan.assert_performance(50.0);
}

#[tokio::test]
async fn analyze_note_replies_query() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    let plan = run_explain_analyze(
        &db,
        "Note Replies",
        "SELECT * FROM note WHERE reply_id = 'note000100' ORDER BY id ASC LIMIT 20",
    )
    .await;

    plan.print_summary();
    plan.assert_uses_index();
    plan.assert_performance(20.0);
}

#[tokio::test]
async fn analyze_note_reactions_query() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    let plan = run_explain_analyze(
        &db,
        "Note Reactions",
        "SELECT * FROM reaction WHERE note_id = 'note000001' LIMIT 100",
    )
    .await;

    plan.print_summary();
    plan.assert_uses_index();
    plan.assert_performance(20.0);
}

#[tokio::test]
async fn analyze_text_search_query() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    // Note: Text search with LIKE typically requires sequential scan
    // For production, use PostgreSQL full-text search
    let plan = run_explain_analyze(
        &db,
        "Text Search (LIKE)",
        "SELECT * FROM note WHERE text LIKE '%content%' AND visibility = 'public' ORDER BY id DESC LIMIT 20"
    ).await;

    plan.print_summary();
    // Note: LIKE '%...' doesn't use index - this is expected
    plan.assert_performance(500.0);

    println!("\n⚠️ Note: LIKE '%pattern%' cannot use indexes efficiently.");
    println!("   Consider using PostgreSQL full-text search (tsvector) for production.");
}

/// Summary test that runs all queries and generates a report
#[tokio::test]
async fn generate_query_performance_report() {
    skip_if_ci!();
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");

    setup_test_data(&db).await;

    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              DATABASE QUERY PERFORMANCE REPORT                ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    let queries = vec![
        ("Note by ID", "SELECT * FROM note WHERE id = 'note000001'"),
        (
            "Notes by User",
            "SELECT * FROM note WHERE user_id = 'user0001' ORDER BY id DESC LIMIT 20",
        ),
        (
            "Local Timeline",
            "SELECT * FROM note WHERE visibility = 'public' AND is_local = true ORDER BY id DESC LIMIT 20",
        ),
        (
            "Global Timeline",
            "SELECT * FROM note WHERE visibility = 'public' ORDER BY id DESC LIMIT 20",
        ),
        (
            "User by Username",
            r#"SELECT * FROM "user" WHERE username_lower = 'user1' AND host IS NULL"#,
        ),
        (
            "Note Replies",
            "SELECT * FROM note WHERE reply_id = 'note000100' ORDER BY id ASC LIMIT 20",
        ),
    ];

    let mut results = Vec::new();

    for (name, sql) in queries {
        let plan = run_explain_analyze(&db, name, sql).await;
        results.push(plan);
    }

    println!("\n┌────────────────────────┬───────────┬───────────┬──────────┐");
    println!("│ Query                  │ Time (ms) │ Cost      │ Index?   │");
    println!("├────────────────────────┼───────────┼───────────┼──────────┤");

    for result in &results {
        let index_status = if result.uses_index { "✓" } else { "✗" };
        println!(
            "│ {:22} │ {:9.3} │ {:9.2} │    {}     │",
            result.query_name, result.execution_time_ms, result.total_cost, index_status
        );
    }

    println!("└────────────────────────┴───────────┴───────────┴──────────┘");

    // Performance recommendations
    println!("\n📊 Performance Recommendations:");

    for result in &results {
        if !result.uses_index {
            println!("  ⚠️ {}: Consider adding an index", result.query_name);
        }
        if result.execution_time_ms > 50.0 {
            println!(
                "  ⚠️ {}: Query is slow ({:.2}ms), consider optimization",
                result.query_name, result.execution_time_ms
            );
        }
    }

    println!("\n✅ Report generation complete.");
}
