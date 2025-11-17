# EURE Test Suite

テストケースを単一のEUREファイルに記述するテストスイートです。

## 構造

```
test-suite/
├── Cargo.toml
├── cases/              # テストケース（クレートルートに配置）
│   ├── basic/
│   │   ├── array-indexing.eure
│   │   └── simple-object.eure
│   ├── extensions/
│   └── ...
├── src/
│   ├── lib.rs         # TestRunner, TestCase など公開API
│   ├── test_case.rs   # TestCase データ構造
│   ├── parser.rs      # テストケースパーサー
│   └── runner.rs      # テストランナー
└── tests/
    └── suite.rs       # cargo test 統合
```

## テストケースの書き方

テストケースは単一の`.eure`ファイルに、複数のバリエーションをコードブロックで記述します：

```eure
# Test case: Array indexing with [] notation
description = "Array indexing with [] notation"

input = ```eure
actions[] = "build"
actions[] = "test"
```

normalized = ```eure
= {
  actions = ["build", "test"]
}
```

json = ```json
{
  "actions": ["build", "test"]
}
```
```

### シナリオの種類

- **input**: 入力EUREソースコード
- **normalized**: 正規化形式（ドキュメント全体を単一オブジェクトとして表現）
- **json**: 期待されるJSON出力
- **error**: エラー検証用（ネガティブテスト）

### テストの実行

- `input` と `normalized` が両方存在する場合：両方をパースして構造的に等しいか検証
- `input` と `json` が両方存在する場合：inputをJSONに変換して検証
- `normalized` と `json` が両方存在する場合：normalizedをJSONに変換して検証

## 実行方法

```bash
cargo test -p test-suite
```

## 設計の利点

1. **EURE自身の機能を活用**: コードブロック機能とシンタックスハイライトを使用
2. **関連データの凝集性**: 1つのテストケースに関する全てのバリエーションが1箇所に集約
3. **エディタでの扱いやすさ**: タブに異なる名前が表示されるので混乱しない
4. **自己文書化**: テストケース自体がEUREの使用例になる
5. **ネストなしでもOK**: エディタの表示順は名前順なので、フラットな構造で十分

## 現在の状態

⚠️ **注意**: 現在、EUREリポジトリにはいくつかのビルドエラーが存在します：

- `eure-tree`: `ahash::HashMap` のインポートエラー（修正済み: `std::collections::HashMap`を使用）
- `eure-parol`: `Root`型とフィールド参照のエラー
- `eure-json`: `ObjectKey`のリファクタリングによる不整合

これらのエラーはtest-suite作成前から存在していた問題で、test-suite自体の設計とは無関係です。

## 次のステップ

1. 上記のビルドエラーを修正
2. より多くのテストケースを追加
3. エラーハンドリングのテストを追加
4. CI統合
