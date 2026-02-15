# MemoBreeze

リアルタイム音声書き起こし＆議事録自動生成アプリケーション

Tauri + React + TypeScript + Vosk で構築された、オフラインで動作する音声認識アプリです。

## 必要な環境

- **Rust**: 1.89.0以降
- **Bun**: 最新版
- **macOS**: 14.2以降（現在はmacOSのみ対応）

## セットアップ手順

### 1. 依存関係のインストール

```bash
# Rustのインストール（未インストールの場合）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Bunのインストール（未インストールの場合）
curl -fsSL https://bun.sh/install | bash

# プロジェクトの依存関係をインストール
bun install
```

### 2. Vosk APIライブラリのセットアップ

**macOS:**

```bash
# Vosk APIをダウンロード
cd ~
curl -LO https://github.com/alphacep/vosk-api/releases/download/v0.3.50/vosk-api-0.3.50.zip
unzip vosk-api-0.3.50.zip

# ライブラリをプロジェクトにコピー
cd <PROJECT_ROOT>  # プロジェクトディレクトリに移動
mkdir -p src-tauri/libs/macos
cp ~/vosk-api-0.3.50/python/lib/osx/libvosk.dylib src-tauri/libs/macos/
```

または、環境変数でカスタムパスを指定することもできます：

```bash
export VOSK_LIB_DIR=~/vosk-api-0.3.50/python/lib/osx
```

> **Note**: 将来的にWindows/Linux対応の際は、`src-tauri/libs/windows/`、`src-tauri/libs/linux/`に対応するライブラリを配置してください。

### 3. Vosk音声認識モデルのセットアップ

**日本語モデル（推奨）:**

```bash
cd ~
curl -LO https://alphacephei.com/vosk/models/vosk-model-ja-0.22.zip
unzip vosk-model-ja-0.22.zip
mv vosk-model-ja-0.22 vosk-model
```

**英語モデル:**

```bash
cd ~
curl -LO https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip
unzip vosk-model-en-us-0.22.zip
mv vosk-model-en-us-0.22 vosk-model
```

> **Note**: その他の言語モデルは [Vosk Models](https://alphacephei.com/vosk/models) から入手できます。
>
> **Note**: このリポジトリには Vosk モデルは同梱していません（`.gitignore` で `vosk-model-*` を除外）。

### 4. マイク権限の設定

初回起動時にマイク許可が必要です。以下の2つの方法があります：

**方法1: デバッグビルドから実行（推奨）**

```bash
bunx tauri build --debug
open src-tauri/target/debug/bundle/macos/MemoBreeze.app
```

**注意**: エラーが出る場合は：
```bash
source "$HOME/.cargo/env" && bunx tauri build --debug
open src-tauri/target/debug/bundle/macos/MemoBreeze.app
```

初回起動でマイク許可ダイアログが表示されます。

**方法2: 開発時にバンドル経由で起動**

```bash
bunx tauri dev -- --bundle
```

ホットリロード付き開発環境を維持しつつ、常に .app バンドル経由で起動します。

**権限のリセット（必要な場合）:**

```bash
tccutil reset Microphone com.tarou.MemoBreeze
```

### 5. Whisper (whisper.cpp) のセットアップ（任意）

Whisperを使う場合は、`whisper-cli` とモデルファイルが必要です。

```bash
# プロジェクトルートで実行
# 例: cd /path/to/MemoBreeze
cd <PROJECT_ROOT>
mkdir -p tools
git clone https://github.com/ggerganov/whisper.cpp.git tools/whisper.cpp

# ビルド
cd tools/whisper.cpp
cmake -S . -B build -DWHISPER_BUILD_EXAMPLES=ON
cmake --build build -j

# モデル(base)をダウンロード
./models/download-ggml-model.sh base
```

動作確認:

```bash
./build/bin/whisper-cli -ng \
  -m ./models/ggml-base.bin \
  -f ./samples/jfk.wav \
  -l en -nt -otxt -of /tmp/whisper_test
cat /tmp/whisper_test.txt
```

アプリ側の設定（`Settings` タブ）:
- 文字起こしエンジン: `Whisper`
- Whisper Model Path: `<PROJECT_ROOT>/tools/whisper.cpp/models/ggml-base.bin`
- Whisper CLI Command: `whisper-cli`（PATH が通っている場合）または `<PROJECT_ROOT>/tools/whisper.cpp/build/bin/whisper-cli`
- Whisper Language: `ja`（任意）

### 6. Google Docs リアルタイム連携のセットアップ（任意）

Google Docs に書き起こしをリアルタイム追記するには、Google Cloud Console で OAuth クライアントを作成して `GOOGLE_OAUTH_CLIENT_ID` を設定します。

#### 6-1. Google Cloud Console 側の設定

1. Google Cloud Console でプロジェクトを作成（または既存プロジェクトを選択）
2. `API とサービス` > `ライブラリ` で **Google Docs API** を有効化
3. `API とサービス` > `OAuth 同意画面` を設定
   - ユーザータイプは用途に応じて選択（開発中は「外部」+ テストユーザー追加で可）
4. `API とサービス` > `認証情報` > `認証情報を作成` > `OAuth クライアント ID`
   - **アプリケーションの種類は必ず「デスクトップアプリ」** を選択
   - 作成後に表示される **Client ID** を控える

#### 6-2. `.env` への設定

プロジェクトルートの `.env` に Client ID を設定します。

```bash
cp .env.example .env
```

`.env`:

```env
GOOGLE_OAUTH_CLIENT_ID=your-client-id.apps.googleusercontent.com
GOOGLE_OAUTH_CLIENT_SECRET=
```

`GOOGLE_OAUTH_CLIENT_SECRET` は任意です。  
ただし、Google Cloud Console で **Web アプリ** の OAuth クライアントを使っている場合は `client_secret` が必要になるため、値を設定してください。

#### 6-3. リダイレクト URI の扱い

- MemoBreeze は OAuth 認証時に `http://127.0.0.1:<ランダムポート>/oauth2callback` を使うループバック方式です。
- **「デスクトップアプリ」タイプの OAuth クライアント**を使ってください。
- Web アプリ用クライアント ID を使うと、`redirect_uri_mismatch` になることがあります。
- `client_secret is missing` が出る場合は、Web アプリ用クライアントを使っている可能性があります。  
  1) デスクトップアプリの OAuth クライアントに切り替える（推奨）  
  2) もしくは `.env` に `GOOGLE_OAUTH_CLIENT_SECRET` を設定する

#### 6-4. アプリ側の設定手順

1. `Settings` タブ > `Google Docs Sync` で `Connect Google` をクリック
2. ブラウザで Google ログインと権限許可を実施
3. 追記先の Google ドキュメント URL を入力
4. `Enable realtime sync` を ON

これで、`final` になった書き起こしセグメントが Google Docs に順次追記されます。

## 開発

### 開発サーバーの起動

```bash
bun run tauri dev
```

**注意**: エラーが出る場合は、Cargo環境変数を読み込んでから実行してください：

```bash
source "$HOME/.cargo/env" && bun run tauri dev
```

- フロントエンド: http://localhost:1420
- Rustバックエンド: 自動起動

### ビルド

```bash
bun run build
```

TypeScriptのコンパイルとViteビルドが実行されます。

### 本番ビルド

```bash
bunx tauri build
```

**注意**: Rustをインストール直後や、新しいシェルセッションでは、Cargoの環境変数が読み込まれていない場合があります。エラーが出た場合は以下を実行してください：

```bash
source "$HOME/.cargo/env" && bunx tauri build
```

リリース版が `src-tauri/target/release/bundle/` に生成されます。

## 本番アプリケーションの使用

### 1. 本番ビルドの作成と起動

```bash
# リリースビルドを作成
bunx tauri build

# 生成されたアプリを起動
open src-tauri/target/release/bundle/macos/MemoBreeze.app
```

### 2. Applicationsフォルダへのインストール（推奨）

```bash
# アプリをApplicationsフォルダにコピー
cp -r src-tauri/target/release/bundle/macos/MemoBreeze.app /Applications/

# Applicationsフォルダから起動
open /Applications/MemoBreeze.app
```

または、Finderでアプリをダブルクリックして起動できます。

### 3. 初回起動時の設定

初回起動時には以下の設定が必要です：

1. **マイク権限の許可**
   - 初回起動時にマイク許可ダイアログが表示されます
   - 「OK」をクリックして許可してください

2. **Vosk APIライブラリの確認**
   - `src-tauri/libs/macos/libvosk.dylib` が存在することを確認
   - まだインストールしていない場合は、上記の「Vosk APIライブラリのセットアップ」を実施

3. **Voskモデルの確認**
   - `~/vosk-model` にモデルがインストールされていることを確認
   - まだインストールしていない場合は、上記の「Vosk音声認識モデルのセットアップ」を実施

4. **Ollamaのインストール（議事録生成に必要）**
   ```bash
   # Ollamaのインストール
   curl -fsSL https://ollama.com/install.sh | sh

   # モデルのダウンロード（例：llama3）
   ollama pull llama3
   ```

### 4. アプリケーションの配布

他のユーザーに配布する場合：

1. `src-tauri/target/release/bundle/macos/MemoBreeze.app` を配布
2. 受け取ったユーザーは以下を実施：
   - アプリを `/Applications/` にコピー
   - Vosk APIライブラリのセットアップ（上記手順参照）
   - Voskモデルのダウンロード（上記手順参照）
   - Ollamaのインストール（議事録機能を使う場合）

**注意**:
- macOSのセキュリティ設定により、未署名のアプリは初回起動時に警告が表示される場合があります。その場合は、システム環境設定 > プライバシーとセキュリティ から手動で許可してください。
- Vosk APIライブラリ（`libvosk.dylib`）は、ライセンスの関係上アプリバンドルに含まれていません。各ユーザーが個別にセットアップする必要があります。

## 使い方

1. **録音の開始**
   - 「Recording」タブで「録音を開始」ボタンをクリック
   - Vosk利用時: Model Pathが `~/vosk-model` になっていることを確認
   - Whisper利用時: Whisper Model Path / Whisper CLI Command が正しいことを確認
   - 必要に応じて「Capture Input」（マイク）と「Capture Output」（システム音声）を選択

2. **文字起こしの編集**
   - リアルタイムで文字起こしされたテキストが表示されます
   - セグメントごとに編集・話者の割り当てが可能

3. **議事録の生成**
   - 「Minutes」タブに移動
   - Ollamaモデルを選択（事前にOllamaのインストールが必要）
   - 「Generate」ボタンで議事録を自動生成
   - 生成された議事録をエクスポート可能

## トラブルシューティング

### "Failed to resolve Vosk model path"

Voskモデルが正しくインストールされていません。上記の「Vosk音声認識モデルのセットアップ」を実施してください。

### "Whisper command ... was not found"

`whisper-cli` が見つかっていません。上記の「Whisper (whisper.cpp) のセットアップ」を実施し、以下のいずれかを設定してください。

- `Settings` の `Whisper CLI Command` に `whisper-cli`（PATH が通っている場合）
- `Settings` の `Whisper CLI Command` に `whisper-cli` の絶対パス

### マイク権限のエラー

システム環境設定 > プライバシーとセキュリティ > マイク で、MemoBreeze.app にチェックが入っているか確認してください。

### ビルドが無限ループする

`src-tauri/libs/macos/libvosk.dylib` の変更検出による問題です。本プロジェクトの `build.rs` は既に最適化されていますが、問題が発生した場合は：

```bash
find src-tauri/libs -name "*.dylib.XXXXXX" -delete
```

## IDE設定

**推奨:**
- [VS Code](https://code.visualstudio.com/)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## ライセンス

このリポジトリの**アプリ本体コード**は `MIT` です。

ただし、実行時または配布時に利用・同梱する第三者コンポーネント
（例: `Vosk API`、`Vosk model`、`whisper.cpp`、各種 `npm` / `cargo` 依存）は
それぞれのライセンスに従います。

詳細は `THIRD_PARTY_NOTICES.md` を参照してください。

## 技術スタック

- **フロントエンド**: React 19, TypeScript, Vite, Tailwind CSS, shadcn/ui
- **バックエンド**: Rust, Tauri 2.0
- **音声認識**: Vosk
- **状態管理**: Zustand
- **多言語対応**: i18next
