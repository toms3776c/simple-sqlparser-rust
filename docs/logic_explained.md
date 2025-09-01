## 何をするプログラムか（全体像）

- **目的**: 入力されたSQLを解析し、その中で参照されているテーブル名を重複なく一覧出力するCLIツール
- **対応方言**: `generic / postgres / mysql / mssql / snowflake / bigquery / sqlite / hive / ansi / redshift`
- **入力方法**: `--file`（SQLファイル）または `--sql`（直接文字列）
- **出力**: テーブル名を1行ずつ標準出力（重複排除・ソート済み）

## 主要コンポーネント

- **`DialectKind`**: SQL方言を表す列挙型
- **`parse_sql_with_dialect`**: 指定方言のパーサ（`sqlparser`クレート）でSQL文字列をAST（抽象構文木）へ変換
- **抽出ロジック**（AST走査）
  - `extract_tables`: ステートメント配列からテーブル名集合（`BTreeSet<String>`）を作成
  - `collect_tables_from_query` / `collect_tables_from_select` / `collect_tables_from_expr` / `from_table_with_joins`:
    クエリ本体・サブクエリ・JOIN・CTEなどを再帰的にたどってテーブル名を収集
  - `object_name_to_string`: `schema.table` 等の識別子列をドット連結して文字列化
- **`parse_args`**: コマンドライン引数の解釈（方言とSQLの取得）
- **`main`**: 引数→パース→抽出→出力の実行フロー

## ASTの歩き方（どこからテーブルを拾うか）

- 主対象は **SELECT系**（`Statement::Query`）と **DDL文**（`Statement::CreateView`, `Statement::CreateTable`）
- INSERT/UPDATE/DELETE等の他のDMLは現状スキップ
- **WITH/CTE**: `WITH`節内の各サブクエリも再帰的に解析
- **クエリ本体（`Query.body`）** は `SetExpr` をキューで幅優先的に処理
  - `Select` → `FROM` / `JOIN` からテーブル名を取得
  - `Query` → ネストしたサブクエリへ再帰
  - `SetOperation`（UNION/INTERSECT 等）→ 左右の式をキューに追加
  - `Values` → テーブル参照なしとして無視
- **FROM/JOIN**（`from_table_with_joins`）
  - `TableFactor::Table` → そのままテーブル名を追加
  - `Derived`（サブクエリ）→ 再帰解析
  - `NestedJoin` → 括弧内のテーブル・サブクエリ・ネストJOINも解析
  - `TableFunction` 等は現状スキップ
- **SELECTリスト・WHERE・HAVING内のサブクエリ**（`collect_tables_from_expr`）
  - `IN (subquery)`, `EXISTS (subquery)`, `Subquery(...)` を検出して再帰
  - 複合式（`BinaryOp`/`UnaryOp`/`Case`/`Between`/`Tuple` 等）は中の式を順にたどる
  - `Function` 内部は現状スキップ（多くの場合テーブル抽出に不要）

## DDL文の処理

- **CREATE VIEW文**（`Statement::CreateView`）
  - VIEW定義内のクエリ（`query`フィールド）を `collect_tables_from_query` で再帰的に解析
  - `CREATE OR REPLACE VIEW` 文も同様に処理
  - VIEWが参照するテーブル名をすべて抽出
- **CREATE TABLE AS SELECT文**（`Statement::CreateTable`）
  - `query` フィールドが存在する場合（CTAS）、そのクエリを再帰的に解析
  - 通常の `CREATE TABLE` 文（`query` が `None`）はテーブル参照なしとしてスキップ

## データ構造と出力

- **`BTreeSet<String>`** でテーブル名を重複なくソートしながら保持
- 最終的に1行ずつ `println!` で出力

## エラーハンドリングと終了コード

- 引数エラー時: 使い方メッセージを表示し終了コード 2
- SQLパース失敗時: エラーメッセージを表示し終了コード 1
- 成功時: 終了コード 0

## 実行例

```bash
# SELECT文（CTE含む）
cargo run -- --dialect postgres --sql "WITH c AS (SELECT * FROM s.t) SELECT * FROM c JOIN u.v ON c.id = v.id"

# CREATE VIEW文
cargo run -- --dialect generic --file ./sql/create_view_sample1.sql

# シンプルなファイル読み込み
cargo run -- --dialect mysql --file ./sql/sample1.sql

# 新しいダイアレクトの例
cargo run -- --dialect bigquery --sql "SELECT * FROM project_dataset_table"
cargo run -- --dialect sqlite --sql "SELECT * FROM users LIMIT 10"
cargo run -- --dialect hive --sql "SELECT * FROM warehouse.users WHERE year=2023"
cargo run -- --dialect redshift --sql "SELECT * FROM sales.orders WHERE date >= '2023-01-01'"
cargo run -- --dialect ansi --sql "SELECT name FROM customers"
```

出力例（ソート済み・重複なし）:

```text
# CTE例の出力
s.t
u.v

# CREATE VIEW例の出力
users

# 新しいダイアレクト例の出力
project_dataset_table
warehouse.users
sales.orders
customers
```

## 実装のポイント（Rustを知らない方向け）

- 外部ライブラリ `sqlparser` がSQL文字列を機械可読なツリー（AST）に変換
- プログラムはそのツリーをたどり、テーブル名に相当するノードを集める
- 収集は再帰的（入れ子のサブクエリやCTEも対象）で、集合に追加するため重複しない
- 方言差は「パーサの選択」のみで、抽出ロジックは共通

## 制限と拡張ポイント

- 現状は `SELECT` 文、`CREATE VIEW` 文、`CREATE TABLE AS SELECT` 文に対応
- 他のDML（INSERT/UPDATE/DELETEなど）は未対応
- `TableFunction` や関数内のサブクエリ解析は省略中
- 必要に応じて以下を拡張可能:
  - 残りのDML文の対応追加（INSERT/UPDATE/DELETE）
  - 関数呼び出し内部の解析
  - `TableFunction` の対応
  - その他のDDL文（CREATE INDEX、ALTER TABLEなど）の対応

## 処理フロー要約

1. 引数を読み取り、方言とSQL文字列（もしくはファイル内容）を取得
2. 方言に応じてSQLをASTへパース
3. ASTを広くたどってテーブル名を収集
4. 重複なくソートしたテーブル名を出力


