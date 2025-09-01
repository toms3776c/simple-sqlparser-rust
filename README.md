# Simple SQL Parser

SQLクエリからテーブル名を抽出するRust製のコマンドライン・ツールです。複数のSQLダイアレクトに対応し、複雑なSQL構文（CTE、サブクエリ、JOIN、セット演算など）を解析してテーブル依存関係を特定できます。

## ✨ 主な機能

- 🗃️ **テーブル名抽出**: SQLクエリから参照されているテーブル名を自動抽出
- 🔧 **複数ダイアレクト対応**: 10種類のSQLダイアレクト（Generic、PostgreSQL、MySQL、MS SQL、Snowflake、BigQuery、SQLite、Hive、ANSI、Redshift）に対応
- 🧩 **高度なSQL構文サポート**: CTE、サブクエリ、JOIN、セット演算、CREATE VIEW文を完全解析
- 📁 **柔軟な入力**: SQLファイル読み込みまたは直接文字列指定
- ⚡ **高性能**: Rustによる高速処理と効率的なメモリ使用

## 🔧 対応SQL構文

### 基本構文
- `SELECT ... FROM table_name`
- `JOIN` (INNER, LEFT, RIGHT, FULL OUTER)
- `WHERE`, `HAVING` 句
- テーブルエイリアス (`AS` 句)

### 高度な構文
- **CTE (Common Table Expression)**
  ```sql
  WITH cte_name AS (SELECT ...) SELECT ... FROM cte_name
  ```

- **サブクエリ**
  ```sql
  SELECT ... WHERE col IN (SELECT ...)
  SELECT ... WHERE EXISTS (SELECT ...)
  ```

- **セット演算**
  ```sql
  SELECT ... UNION SELECT ...
  SELECT ... INTERSECT SELECT ...
  ```

- **ネストしたJOIN・派生テーブル**
  ```sql
  FROM (table1 JOIN table2) JOIN table3
  FROM (SELECT ... FROM table1) AS derived
  ```

- **CREATE VIEW文**
  ```sql
  CREATE OR REPLACE VIEW view_name AS
  SELECT ... FROM table1 JOIN table2 ...
  ```

- **CREATE TABLE AS SELECT文**
  ```sql
  CREATE TABLE new_table AS
  SELECT ... FROM existing_table
  ```

## 📦 インストール

### 前提条件
- Rust 1.70+ (edition 2024対応)

### ビルド
```bash
git clone <repository-url>
cd sqlparser
cargo build --release
```

実行ファイルは `target/release/sqlparser` に生成されます。

## 🚀 使用方法

### 基本的な使用法

```bash
# SQLファイルから読み込み
sqlparser --dialect <dialect> --file <path>

# SQL文字列を直接指定
sqlparser --dialect <dialect> --sql "<sql_query>"
```

### ダイアレクト指定

| パラメータ | 対応データベース | カテゴリ |
|-----------|-----------------|----------|
| `generic` | 汎用SQL | 標準・汎用 |
| `ansi` | ANSI SQL標準 | 標準・汎用 |
| `postgres`/`postgresql` | PostgreSQL | 従来型DB |
| `mysql` | MySQL | 従来型DB |
| `mssql` | Microsoft SQL Server | 従来型DB |
| `sqlite` | SQLite | 従来型DB |
| `snowflake` | Snowflake | クラウドDWH |
| `bigquery` | Google BigQuery | クラウドDWH |
| `redshift` | Amazon Redshift | クラウドDWH |
| `hive` | Apache Hive | ビッグデータ |

### 使用例

#### 1. シンプルなSQLファイルの解析
```bash
$ sqlparser --dialect postgres --file ./sql/sample1.sql
users
```

#### 2. 複雑なクエリの解析
```bash
$ sqlparser --dialect postgres --file ./sql/complex.sql
customers
customer_stats
order_items
orders
products
recent_orders
```

#### 3. 直接SQL文字列を指定
```bash
$ sqlparser --dialect mysql --sql "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id"
posts
users
```

#### 4. CTEを含む複雑なクエリ
```bash
$ sqlparser --dialect postgres --sql "
WITH recent_orders AS (
    SELECT customer_id FROM orders WHERE date > '2023-01-01'
)
SELECT c.name 
FROM customers c 
JOIN recent_orders ro ON c.id = ro.customer_id"
customers
orders
recent_orders
```

#### 5. CREATE VIEW文の解析
```bash
$ sqlparser --dialect generic --file ./sql/create_view_sample1.sql
users
```

#### 6. CREATE VIEW文（複雑な例）
```bash
$ sqlparser --dialect postgres --sql "
CREATE OR REPLACE VIEW customer_summary AS
SELECT c.name, COUNT(o.id) as order_count
FROM customers c 
LEFT JOIN orders o ON c.id = o.customer_id 
GROUP BY c.name"
customers
orders
```

#### 7. 新しいダイアレクトの使用例

```bash
# BigQuery
$ sqlparser --dialect bigquery --sql "SELECT * FROM project_dataset_table"
project_dataset_table

# SQLite
$ sqlparser --dialect sqlite --sql "SELECT * FROM users LIMIT 10"
users

# Apache Hive
$ sqlparser --dialect hive --sql "SELECT * FROM warehouse.users WHERE year=2023"
warehouse.users

# Amazon Redshift
$ sqlparser --dialect redshift --sql "SELECT * FROM sales.orders WHERE date >= '2023-01-01'"
sales.orders

# ANSI SQL
$ sqlparser --dialect ansi --sql "SELECT name FROM customers"
customers
```

## 📁 プロジェクト構成

```
sqlparser/
├── Cargo.toml          # プロジェクト設定・依存関係
├── Cargo.lock          # 依存関係ロックファイル
├── README.md           # このファイル
├── docs/
│   └── architecture.md # 詳細な設計文書
├── sql/
│   ├── sample1.sql            # 基本テスト用SQL
│   ├── create_view_sample1.sql # CREATE VIEW テスト用SQL
│   └── complex.sql            # 複雑なクエリテスト用SQL
└── src/
    └── main.rs         # メインプログラム
```

## 🔍 出力形式

- **成功時**: 抽出されたテーブル名を1行ずつアルファベット順で出力
- **エラー時**: 標準エラー出力にエラーメッセージを表示

### 終了コード
- `0`: 正常終了
- `1`: SQLパースエラー
- `2`: 引数エラーまたはファイル読み込みエラー

## 💡 使用例・ユースケース

### 1. データベース依存関係分析
```bash
# アプリケーションで使用されているテーブルを調査
sqlparser --dialect postgres --file app_queries.sql > table_dependencies.txt
```

### 2. SQLレビュー支援
```bash
# プルリクエストでのSQL変更確認
sqlparser --dialect mysql --sql "$(cat new_feature.sql)"
```

### 3. データマイグレーション計画
```bash
# 複雑なクエリが参照するテーブル一覧を確認
sqlparser --dialect snowflake --file migration_script.sql
```

### 4. テスト用データ準備
```bash
# テストで必要なテーブルを特定
sqlparser --dialect postgres --file test_queries.sql | xargs -I {} echo "TRUNCATE TABLE {};"
```

## ⚡ パフォーマンス

- **時間計算量**: O(n) - nはSQL AST のノード数
- **空間計算量**: O(m) - mは一意なテーブル名数
- **処理能力**: 中規模のSQLファイル（数千行）を数ミリ秒で処理
- **ダイアレクト対応**: 10種類のSQLダイアレクトに対応

## 🛠️ 技術仕様

### 使用技術
- **言語**: Rust (edition 2024)
- **SQLパーサー**: [sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs) v0.47
- **データ構造**: `BTreeSet` による効率的な重複排除・ソート

### アーキテクチャ特徴
- **単一バイナリ**: 依存関係なしで実行可能
- **メモリ安全**: Rustの所有権システムによる安全性
- **拡張可能**: 新しいダイアレクトや構文への対応が容易

## 📚 詳細ドキュメント

プロジェクトの詳細な設計・アーキテクチャについては [docs/architecture.md](./docs/architecture.md) を参照してください。

## 🚧 現在の制限事項

- SELECT文、CREATE VIEW文、CREATE TABLE AS SELECT文のみ対応（INSERT/UPDATE/DELETE未対応）
- 関数内の式は未解析（`FUNCTION(SELECT ...)` 形式）
- スキーマ情報の検証なし（存在しないテーブルでもエラーにならない）

## 🔮 今後の拡張予定

- [ ] DML文対応（INSERT/UPDATE/DELETE）
- [ ] 追加SQLダイアレクト（Oracle、Teradata、DuckDB、ClickHouse）
- [ ] JSON/XML出力形式サポート
- [ ] テーブル種別判定（テーブル vs ビュー vs CTE）
- [ ] 列レベル依存関係分析
- [ ] ダイアレクト固有構文の詳細対応
- [ ] パフォーマンス最適化

## 🤝 コントリビューション

プロジェクトへの貢献を歓迎します！以下の方法で参加できます：

1. **バグ報告**: Issueでバグを報告
2. **機能要望**: 新機能のアイデアを提案
3. **プルリクエスト**: コード改善・新機能の実装
4. **ドキュメント**: README や設計書の改善

## 📄 ライセンス

このプロジェクトは [MIT License](LICENSE) の下で公開されています。

## 🙋‍♂️ サポート

質問やサポートが必要な場合は、以下の方法でお気軽にお問い合わせください：

- **GitHub Issues**: バグ報告・機能要望
- **GitHub Discussions**: 一般的な質問・使用方法

---
