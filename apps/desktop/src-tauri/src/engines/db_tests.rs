use std::collections::HashMap;

use super::*;
use crate::engines::slicing::{
    like_pattern, predicate, table_rows, Filter, Op, Slice, Sort,
};
use crate::engines::writing::{apply, edit_statements, finish, set_manual, Edit};
use crate::engines::ddl::table_ddl;
use crate::engines::export::{export_table, Format};
use crate::engines::objects::objects;
use crate::engines::plan::explain;

mod tests {
    use super::*;

    fn filter(op: Op, value: &str) -> Filter {
        return Filter {
            column: "name".into(),
            op,
            value: value.into(),
        };
    }

    #[test]
    fn a_quote_in_a_filter_value_cannot_close_the_string() {
        let sneaky = filter(Op::Eq, "x' or '1'='1");

        assert_eq!(
            predicate("\"name\"", &sneaky),
            "\"name\" = 'x'' or ''1''=''1'"
        );
    }

    #[test]
    fn like_wildcards_in_a_value_are_matched_literally() {
        assert_eq!(like_pattern("100%", true, true), "%100!%%");
        assert_eq!(like_pattern("a_b", false, true), "a!_b%");
        assert_eq!(like_pattern("bang!", false, false), "bang!!");
    }

    #[test]
    fn a_like_filter_declares_its_escape_character() {
        let clause = predicate("c", &filter(Op::Contains, "x"));

        assert!(clause.ends_with("escape '!'"), "{clause}");
    }

    #[test]
    fn value_less_operators_ignore_the_value() {
        assert_eq!(predicate("c", &filter(Op::IsNull, "junk")), "c is null");
        assert_eq!(
            predicate("c", &filter(Op::NotNull, "junk")),
            "c is not null"
        );
    }
}

// end to end against a real server; opt in with
// GPQL_TEST_PG="127.0.0.1:55432" so a machine without one is not failed
mod live_support {
    use super::*;

    pub async fn open_test() -> Option<Session> {
        let target = std::env::var("GPQL_TEST_PG").ok()?;
        let (host, port) = target.split_once(':')?;

        let config = SessionConfig {
            kind: "postgres".into(),
            host: host.into(),
            port: port.into(),
            user: "postgres".into(),
            password: String::new(),
            database: "postgres".into(),
            path: String::new(),
            read_only: false,
            tls: "prefer".into(),
            url: String::new(),
            token: String::new(),
            warehouse: String::new(),
            schema: String::new(),
            ..Default::default()
        };

        return Some(open(&config).await.expect("could not open the test server"));
    }
}

mod live {
    use super::*;
    use live_support::*;

    fn slice(limit: u32, offset: u32) -> Slice {
        return Slice {
            limit,
            offset,
            sort: None,
            filters: Vec::new(),
        };
    }

    fn column(result: &QueryResult, name: &str) -> usize {
        return result.columns.iter().position(|c| c == name).unwrap();
    }

    #[tokio::test]
    async fn sorting_ranks_the_whole_table_not_the_loaded_page() {
        let Some(session) = open_test().await else {
            return;
        };

        let mut want = slice(10, 0);
        want.sort = Some(Sort {
            column: "id".into(),
            descending: true,
        });

        let page = table_rows(&session, "big", &want).await.unwrap();
        let id = column(&page, "id");

        // the table holds 5000 rows; ranking only a loaded page would answer 10
        assert_eq!(page.rows[0][id].as_deref(), Some("5000"));
        assert_eq!(page.rows.len(), 10);
    }

    #[tokio::test]
    async fn paging_walks_forward_without_repeating() {
        let Some(session) = open_test().await else {
            return;
        };

        let sort = || {
            Some(Sort {
                column: "id".into(),
                descending: false,
            })
        };

        let mut first = slice(10, 0);
        first.sort = sort();

        let mut second = slice(10, 10);
        second.sort = sort();

        let a = table_rows(&session, "big", &first).await.unwrap();
        let b = table_rows(&session, "big", &second).await.unwrap();
        let id = column(&a, "id");

        assert_eq!(a.rows[0][id].as_deref(), Some("1"));
        assert_eq!(b.rows[0][id].as_deref(), Some("11"));
    }

    #[tokio::test]
    async fn a_percent_in_a_filter_value_is_not_a_wildcard() {
        let Some(session) = open_test().await else {
            return;
        };

        for (value, expected) in [("100%", "100%"), ("a_b", "a_b"), ("bang!", "bang!")] {
            let mut want = slice(50, 0);
            want.filters = vec![Filter {
                column: "v".into(),
                op: Op::Contains,
                value: value.into(),
            }];

            let page = table_rows(&session, "pct", &want).await.unwrap();

            assert_eq!(page.rows.len(), 1, "{value} matched {:?}", page.rows);
            assert_eq!(page.rows[0][0].as_deref(), Some(expected));
        }
    }

    #[tokio::test]
    async fn an_identifier_with_a_space_is_quoted() {
        let Some(session) = open_test().await else {
            return;
        };

        let mut want = slice(10, 0);
        want.sort = Some(Sort {
            column: "col A".into(),
            descending: true,
        });

        let page = table_rows(&session, "odd Name", &want).await.unwrap();

        assert_eq!(page.rows[0][0].as_deref(), Some("2"));
    }

    #[tokio::test]
    async fn null_filters_reach_the_server() {
        let Some(session) = open_test().await else {
            return;
        };

        let mut want = slice(5000, 0);
        want.filters = vec![Filter {
            column: "note".into(),
            op: Op::IsNull,
            value: "ignored".into(),
        }];

        let page = table_rows(&session, "big", &want).await.unwrap();

        // every seventh row was seeded null
        assert_eq!(page.rows.len(), 5000 / 7);
    }

    #[tokio::test]
    async fn a_quote_in_a_filter_value_stays_data() {
        let Some(session) = open_test().await else {
            return;
        };

        let mut want = slice(50, 0);
        want.filters = vec![Filter {
            column: "v".into(),
            op: Op::Eq,
            value: "x' or '1'='1".into(),
        }];

        let page = table_rows(&session, "pct", &want).await.unwrap();

        assert!(page.rows.is_empty(), "injection returned {:?}", page.rows);
    }
}

mod exporting {
    use super::*;
    use live_support::*;
    use crate::engines::export::{export_table, Format};

    fn temp(name: &str) -> String {
        return std::env::temp_dir()
            .join(name)
            .to_string_lossy()
            .to_string();
    }

    #[tokio::test]
    async fn csv_covers_every_row_not_just_the_first_page() {
        let Some(session) = open_test().await else {
            return;
        };

        let path = temp("gpql-big.csv");
        let mut want = Slice::default();

        want.sort = Some(Sort {
            column: "id".into(),
            descending: false,
        });

        let written = export_table(&session, "big", &want, Format::Csv, &path)
            .await
            .unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<_> = text.lines().collect();

        assert_eq!(written, 5000);
        assert_eq!(lines.len(), 5001, "header plus every row");
        assert_eq!(lines[0], "id,name,note");
        assert_eq!(lines[1], "1,row1,note 1");
        assert_eq!(lines[5000], "5000,row5000,note 5000");

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn a_filter_carries_into_the_file() {
        let Some(session) = open_test().await else {
            return;
        };

        let path = temp("gpql-filtered.csv");
        let mut want = Slice::default();

        want.filters = vec![Filter {
            column: "note".into(),
            op: Op::IsNull,
            value: String::new(),
        }];

        let written = export_table(&session, "big", &want, Format::Csv, &path)
            .await
            .unwrap();

        assert_eq!(written, 5000 / 7);
        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn a_comma_or_quote_in_a_value_stays_one_csv_field() {
        let Some(session) = open_test().await else {
            return;
        };

        let path = temp("gpql-awkward.csv");
        let written = export_table(&session, "awkward", &Slice::default(), Format::Csv, &path)
            .await
            .unwrap();

        let text = std::fs::read_to_string(&path).unwrap();

        assert_eq!(written, 3);
        assert!(text.contains("\"a,b\""), "{text}");
        assert!(text.contains("\"say \"\"hi\"\"\""), "{text}");

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn json_is_valid_and_keeps_nulls() {
        let Some(session) = open_test().await else {
            return;
        };

        let path = temp("gpql-small.json");
        let mut want = Slice::default();

        want.limit = 8;
        want.sort = Some(Sort {
            column: "id".into(),
            descending: false,
        });

        export_table(&session, "big", &want, Format::Json, &path)
            .await
            .unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        let rows = parsed.as_array().unwrap();

        assert_eq!(rows.len(), 8);
        assert_eq!(rows[0]["name"], "row1");
        assert!(rows[6]["note"].is_null(), "seventh row was seeded null");

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn insert_statements_quote_names_and_escape_values() {
        let Some(session) = open_test().await else {
            return;
        };

        let path = temp("gpql-awkward.sql");

        export_table(&session, "awkward", &Slice::default(), Format::Sql, &path)
            .await
            .unwrap();

        let text = std::fs::read_to_string(&path).unwrap();

        assert!(text.contains("insert into \"awkward\" (\"v\") values"), "{text}");
        assert!(text.contains("'say \"hi\"'"), "{text}");
        assert!(text.contains("'it''s'"), "{text}");

        std::fs::remove_file(&path).ok();
    }
}

mod transacting {
    use super::*;
    use live_support::*;

    // these run in parallel against one server, so each owns its own row
    const AUTOCOMMIT: &str = "1";
    const HIDDEN: &str = "2";
    const ROLLED_BACK: &str = "3";
    const ABANDONED: &str = "4";
    const PREVIEWED: &str = "5";

    fn seeded(id: &str) -> String {
        return (id.parse::<i32>().unwrap() * 100).to_string();
    }

    fn edit(id: &str, amount: &str) -> Edit {
        let mut keys = HashMap::new();
        let mut set = HashMap::new();

        keys.insert("id".to_string(), Some(id.to_string()));
        set.insert("amount".to_string(), Some(amount.to_string()));

        return Edit { keys, set };
    }

    // repeated runs must start from the same place, so each test seeds its row
    async fn restore(session: &Session, id: &str) {
        query(
            session,
            &format!("update money_ set amount = {} where id = {id}", seeded(id)),
        )
        .await
        .unwrap();
    }

    async fn amount(session: &Session, id: &str) -> String {
        let page = query(session, &format!("select amount from money_ where id = {id}"))
            .await
            .unwrap();

        return page.rows[0][0].clone().unwrap();
    }

    #[tokio::test]
    async fn autocommit_writes_are_visible_to_another_session() {
        let (Some(writer), Some(reader)) = (open_test().await, open_test().await) else {
            return;
        };

        restore(&writer, AUTOCOMMIT).await;

        apply(&writer, "money_", &[edit(AUTOCOMMIT, "111")])
            .await
            .unwrap();

        assert_eq!(amount(&reader, AUTOCOMMIT).await, "111");
    }

    #[tokio::test]
    async fn a_manual_session_hides_its_write_until_it_commits() {
        let (Some(writer), Some(reader)) = (open_test().await, open_test().await) else {
            return;
        };

        restore(&writer, HIDDEN).await;
        set_manual(&writer, true).await.unwrap();
        apply(&writer, "money_", &[edit(HIDDEN, "999")]).await.unwrap();

        assert_eq!(amount(&writer, HIDDEN).await, "999", "its own view moves");
        assert_eq!(
            amount(&reader, HIDDEN).await,
            seeded(HIDDEN),
            "nobody else sees it yet"
        );

        assert!(finish(&writer, "commit").await.unwrap());
        assert_eq!(amount(&reader, HIDDEN).await, "999");
    }

    #[tokio::test]
    async fn rollback_actually_undoes_the_write() {
        let Some(session) = open_test().await else {
            return;
        };

        restore(&session, ROLLED_BACK).await;
        set_manual(&session, true).await.unwrap();
        apply(&session, "money_", &[edit(ROLLED_BACK, "777")])
            .await
            .unwrap();

        assert_eq!(amount(&session, ROLLED_BACK).await, "777");
        assert!(finish(&session, "rollback").await.unwrap());
        assert_eq!(amount(&session, ROLLED_BACK).await, seeded(ROLLED_BACK));
    }

    #[tokio::test]
    async fn leaving_manual_mode_discards_uncommitted_work() {
        let Some(session) = open_test().await else {
            return;
        };

        restore(&session, ABANDONED).await;
        set_manual(&session, true).await.unwrap();
        apply(&session, "money_", &[edit(ABANDONED, "555")])
            .await
            .unwrap();
        set_manual(&session, false).await.unwrap();

        assert_eq!(amount(&session, ABANDONED).await, seeded(ABANDONED));
    }

    #[tokio::test]
    async fn committing_with_nothing_open_is_not_an_error() {
        let Some(session) = open_test().await else {
            return;
        };

        assert!(!finish(&session, "commit").await.unwrap());
    }

    #[tokio::test]
    async fn the_preview_is_the_statement_that_runs() {
        let Some(session) = open_test().await else {
            return;
        };

        let statements = edit_statements(&session, "money_", &[edit(PREVIEWED, "42")]);

        assert_eq!(statements.len(), 1);
        assert_eq!(
            statements[0],
            format!(
                "update \"money_\" set \"amount\" = '42' where \"id\" = '{PREVIEWED}'"
            )
        );
    }
}

mod defining {
    use super::*;
    use live_support::*;
    use crate::engines::ddl::table_ddl;
    use crate::engines::plan::explain;

    #[tokio::test]
    async fn a_table_definition_round_trips_through_the_server() {
        let Some(session) = open_test().await else {
            return;
        };

        let text = table_ddl(&session, "book").await.unwrap();

        // the point of rebuilding from the catalog is exactness, so the
        // result has to be something the server will take back
        // only the create-table part is assembled here; the index and
        // constraint text comes back verbatim from the server
        let head = text.split_inclusive(");").next().unwrap().to_string();

        query(&session, "drop schema if exists scratch cascade").await.unwrap();
        query(&session, "create schema scratch").await.unwrap();
        query(&session, "set search_path to scratch, public").await.unwrap();

        let replayed = query(&session, &head).await;

        query(&session, "set search_path to public").await.unwrap();
        query(&session, "drop schema scratch cascade").await.unwrap();

        replayed.unwrap_or_else(|failure| panic!("{failure}
{text}"));
    }

    #[tokio::test]
    async fn a_definition_keeps_types_defaults_and_constraints() {
        let Some(session) = open_test().await else {
            return;
        };

        let text = table_ddl(&session, "book").await.unwrap();

        assert!(text.contains("character varying(200)"), "{text}");
        assert!(text.contains("numeric(10,2)"), "{text}");
        assert!(text.contains("text[]"), "{text}");
        assert!(text.contains("not null"), "{text}");
        assert!(text.contains("default"), "{text}");
        assert!(text.contains("FOREIGN KEY"), "{text}");
        assert!(text.contains("CHECK"), "{text}");
        assert!(text.to_lowercase().contains("create index"), "{text}");
    }

    #[tokio::test]
    async fn a_view_reports_its_select() {
        let Some(session) = open_test().await else {
            return;
        };

        let text = table_ddl(&session, "cheap_books").await.unwrap();

        assert!(text.starts_with("create view \"cheap_books\" as"), "{text}");
        assert!(text.contains("price"), "{text}");
    }

    #[tokio::test]
    async fn a_plan_comes_back_as_a_tree() {
        let Some(session) = open_test().await else {
            return;
        };

        let plan = explain(&session, "select * from big where id = 42", false)
            .await
            .unwrap();
        let tree = plan.tree.expect("postgres should give a tree");

        assert!(tree.label.contains("big"), "{}", tree.label);
        assert!(tree.cost.is_some());
        assert!(tree.time.is_none(), "no timing without analyze");
    }

    #[tokio::test]
    async fn analyze_adds_real_timings() {
        let Some(session) = open_test().await else {
            return;
        };

        let plan = explain(&session, "select count(*) from big", true)
            .await
            .unwrap();
        let tree = plan.tree.expect("postgres should give a tree");

        assert!(tree.time.is_some(), "analyze should time the run");
    }

    #[tokio::test]
    async fn analyze_refuses_to_run_a_write() {
        let Some(session) = open_test().await else {
            return;
        };

        let failure = explain(&session, "delete from money_", true).await;

        assert!(failure.is_err(), "analyze would have deleted the rows");

        let left = query(&session, "select count(*) from money_").await.unwrap();

        assert_eq!(left.rows[0][0].as_deref(), Some("20"));
    }

    #[tokio::test]
    async fn a_join_plan_has_children() {
        let Some(session) = open_test().await else {
            return;
        };

        let plan = explain(
            &session,
            "select b.title, a.name from book b join author a on a.id = b.author_id",
            false,
        )
        .await
        .unwrap();

        assert!(!plan.tree.unwrap().children.is_empty());
    }
}

mod browsing {
    use super::*;
    use live_support::*;
    use crate::engines::objects::objects;

    fn named<'a>(found: &'a [crate::engines::objects::DbObject], kind: &str) -> Vec<&'a str> {
        return found
            .iter()
            .filter(|entry| entry.kind == kind)
            .map(|entry| entry.name.as_str())
            .collect();
    }

    #[tokio::test]
    async fn the_browser_sees_past_tables() {
        let Some(session) = open_test().await else {
            return;
        };

        let found = objects(&session).await.unwrap();

        assert!(named(&found, "view").contains(&"cheap_books"), "views");
        assert!(named(&found, "index").contains(&"book_title_idx"), "indexes");
        assert!(
            named(&found, "sequence").iter().any(|n| n.contains("author")),
            "sequences: {:?}",
            named(&found, "sequence")
        );
    }

    #[tokio::test]
    async fn an_index_says_which_table_it_belongs_to() {
        let Some(session) = open_test().await else {
            return;
        };

        let found = objects(&session).await.unwrap();
        let entry = found
            .iter()
            .find(|entry| entry.name == "book_title_idx")
            .expect("index missing");

        assert_eq!(entry.detail, "book");
    }

    #[tokio::test]
    async fn tables_are_not_repeated_here() {
        let Some(session) = open_test().await else {
            return;
        };

        let found = objects(&session).await.unwrap();

        assert!(
            !found.iter().any(|entry| entry.name == "big"),
            "the tables list already covers tables"
        );
    }
}
