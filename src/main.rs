use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::PathBuf;

use sqlparser::ast::*;
use sqlparser::dialect::{Dialect, GenericDialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect, SnowflakeDialect, BigQueryDialect, SQLiteDialect, HiveDialect, AnsiDialect, RedshiftSqlDialect};
use sqlparser::parser::Parser;

#[derive(Debug, Clone, Copy)]
enum DialectKind {
    Generic,
    Postgres,
    MySql,
    MsSql,
    Snowflake,
    BigQuery,
    SQLite,
    Hive,
    Ansi,
    Redshift,
}

fn parse_sql_with_dialect(sql: &str, dialect: DialectKind) -> Result<Vec<Statement>, String> {
    let dialect_impl: Box<dyn Dialect> = match dialect {
        DialectKind::Generic => Box::new(GenericDialect {}),
        DialectKind::Postgres => Box::new(PostgreSqlDialect {}),
        DialectKind::MySql => Box::new(MySqlDialect {}),
        DialectKind::MsSql => Box::new(MsSqlDialect {}),
        DialectKind::Snowflake => Box::new(SnowflakeDialect {}),
        DialectKind::BigQuery => Box::new(BigQueryDialect {}),
        DialectKind::SQLite => Box::new(SQLiteDialect {}),
        DialectKind::Hive => Box::new(HiveDialect {}),
        DialectKind::Ansi => Box::new(AnsiDialect {}),
        DialectKind::Redshift => Box::new(RedshiftSqlDialect {}),
    };
    Parser::parse_sql(&*dialect_impl, sql).map_err(|e| e.to_string())
}

fn collect_tables_from_query(query: &Query, out: &mut BTreeSet<String>) {
    // 探索用のキュー（FROM句・JOIN・サブクエリ・CTE・セット演算など）
    let mut queue: VecDeque<SetExpr> = VecDeque::new();
    queue.push_back((*query.body).clone());

    // WITH (CTE)
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_tables_from_query(&cte.query, out);
        }
    }

    while let Some(expr) = queue.pop_front() {
        match expr {
            SetExpr::Select(select) => {
                collect_tables_from_select(&select, out);
            }
            SetExpr::Query(q) => {
                collect_tables_from_query(&q, out);
            }
            SetExpr::SetOperation { left, right, .. } => {
                queue.push_back(*left);
                queue.push_back(*right);
            }
            SetExpr::Values(_) => {}
            _ => {}
        }
    }
}

fn object_name_to_string(name: &ObjectName) -> String {
    let idents: Vec<String> = name.0.iter().map(|i| i.value.clone()).collect();
    idents.join(".")
}

// 補助: なし（シンプルに副作用で集計）

fn from_table_with_joins_single(twj: &TableWithJoins, out: &mut BTreeSet<String>) {
    match &twj.relation {
        TableFactor::Table { name, .. } => {
            out.insert(object_name_to_string(name));
        }
        TableFactor::Derived { subquery, .. } => {
            collect_tables_from_query(subquery, out);
        }
        TableFactor::TableFunction { .. } => {}
        TableFactor::NestedJoin { table_with_joins: nested, .. } => {
            // ( ... ) の中身
            match &nested.relation {
                TableFactor::Table { name, .. } => {
                    out.insert(object_name_to_string(&name));
                }
                TableFactor::Derived { subquery, .. } => {
                    collect_tables_from_query(&subquery, out);
                }
                _ => {}
            }
            for j in &nested.joins {
                if let TableFactor::Table { name, .. } = &j.relation {
                    out.insert(object_name_to_string(&name));
                } else if let TableFactor::Derived { subquery, .. } = &j.relation {
                    collect_tables_from_query(&subquery, out);
                }
            }
        }
        _ => {}
    }

    // JOIN 側
    for j in &twj.joins {
        match &j.relation {
            TableFactor::Table { name, .. } => {
                out.insert(object_name_to_string(&name));
            }
            TableFactor::Derived { subquery, .. } => {
                collect_tables_from_query(&subquery, out);
            }
            _ => {}
        }
    }
}

fn from_table_with_joins(select: &Select, out: &mut BTreeSet<String>) {
    for twj in &select.from {
        from_table_with_joins_single(twj, out);
    }
}

fn collect_tables_from_select(select: &Select, out: &mut BTreeSet<String>) {
    from_table_with_joins(select, out);

    // SELECT リスト内の式に含まれるサブクエリ
    for item in &select.projection {
        match item {
            SelectItem::UnnamedExpr(e) => collect_tables_from_expr(e, out),
            SelectItem::ExprWithAlias { expr, .. } => collect_tables_from_expr(expr, out),
            _ => {}
        }
    }

    // WHERE / HAVING 内のサブクエリ
    if let Some(selection) = &select.selection {
        collect_tables_from_expr(selection, out);
    }
    if let Some(having) = &select.having {
        collect_tables_from_expr(having, out);
    }

    // GROUP BY は一旦スキップ（テーブル抽出には不要）
}

fn collect_tables_from_expr(expr: &Expr, out: &mut BTreeSet<String>) {
    match expr {
        Expr::InSubquery { subquery, .. }
        | Expr::Exists { subquery, .. }
        | Expr::Subquery(subquery) => {
            collect_tables_from_query(subquery, out);
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_tables_from_expr(left, out);
            collect_tables_from_expr(right, out);
        }
        Expr::UnaryOp { expr, .. } => collect_tables_from_expr(expr, out),
        Expr::Cast { expr, .. } => collect_tables_from_expr(expr, out),
        Expr::Extract { expr, .. } => collect_tables_from_expr(expr, out),
        Expr::Nested(e) => collect_tables_from_expr(e, out),
        Expr::Case { operand, conditions, results, else_result } => {
            if let Some(op) = operand { collect_tables_from_expr(op, out); }
            for c in conditions { collect_tables_from_expr(c, out); }
            for r in results { collect_tables_from_expr(r, out); }
            if let Some(er) = else_result { collect_tables_from_expr(er, out); }
        }
        // 関数内の式解析は一旦スキップ（テーブル抽出には不要なことが多い）
        Expr::Function(_) => {}
        Expr::Between { expr, low, high, .. } => {
            collect_tables_from_expr(expr, out);
            collect_tables_from_expr(low, out);
            collect_tables_from_expr(high, out);
        }
        Expr::Tuple(exprs) => { for e in exprs { collect_tables_from_expr(e, out); } }
        _ => {}
    }
}

fn extract_tables(statements: &[Statement]) -> BTreeSet<String> {
    let mut tables = BTreeSet::new();
    for stmt in statements {
        match stmt {
            Statement::Query(q) => collect_tables_from_query(q, &mut tables),
            
            // CREATE VIEW文の対応
            Statement::CreateView { query, .. } => {
                collect_tables_from_query(query, &mut tables);
            }
            
            // CREATE TABLE AS SELECT文の対応
            Statement::CreateTable { query, .. } => {
                if let Some(q) = query {
                    collect_tables_from_query(q, &mut tables);
                }
            }
            
            // その他のDML/DDL文のサポートは将来追加予定
            // INSERT, UPDATE, DELETE文等も今後対応可能
            _ => {
                // 他のSQL文タイプは今のところスキップ
                // 必要に応じて段階的に追加していく
            }
        }
    }
    tables
}

#[derive(Debug)]
struct CliArgs {
    dialect: DialectKind,
    sql: String,
}

fn parse_args() -> Result<CliArgs, String> {
    let mut args = std::env::args().skip(1);
    let mut dialect = DialectKind::Generic;
    let mut sql: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dialect" => {
                let v = args.next().ok_or("--dialect の値が必要です")?;
                dialect = match v.to_lowercase().as_str() {
                    "generic" => DialectKind::Generic,
                    "postgres" | "postgresql" => DialectKind::Postgres,
                    "mysql" => DialectKind::MySql,
                    "mssql" => DialectKind::MsSql,
                    "snowflake" => DialectKind::Snowflake,
                    "bigquery" => DialectKind::BigQuery,
                    "sqlite" => DialectKind::SQLite,
                    "hive" => DialectKind::Hive,
                    "ansi" => DialectKind::Ansi,
                    "redshift" => DialectKind::Redshift,
                    _ => return Err(format!("未知のdialect: {}", v)),
                };
            }
            "--file" => {
                let path = PathBuf::from(args.next().ok_or("--file の値が必要です")?);
                sql = Some(fs::read_to_string(&path).map_err(|e| format!("ファイル読み込み失敗: {}", e))?);
            }
            "--sql" => {
                sql = Some(args.next().ok_or("--sql の値が必要です")?);
            }
            _ => return Err(format!("未知の引数: {}", arg)),
        }
    }

    let sql = sql.ok_or("--file もしくは --sql でSQLを与えてください")?;
    Ok(CliArgs { dialect, sql })
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!(
                "使い方: sqlparser --dialect <generic|postgres|mysql|mssql|snowflake|bigquery|sqlite|hive|ansi|redshift> (--file <path> | --sql \"...\")\nエラー: {}",
                e
            );
            std::process::exit(2);
        }
    };

    let statements = match parse_sql_with_dialect(&args.sql, args.dialect) {
        Ok(stmts) => stmts,
        Err(e) => {
            eprintln!("SQLパースに失敗しました: {}", e);
            std::process::exit(1);
        }
    };

    let tables = extract_tables(&statements);
    for t in tables {
        println!("{}", t);
    }
}
