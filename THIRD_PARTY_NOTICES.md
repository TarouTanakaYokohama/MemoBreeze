# Third-Party Notices

このプロジェクトの本体コードは `MIT` で提供されていますが、利用する外部コンポーネントには別ライセンスが適用されます。
再配布時は、以下のライセンス条件に従ってください。

## 主要な外部コンポーネント

1. `whisper.cpp`
- Upstream: <https://github.com/ggml-org/whisper.cpp>
- License: `MIT`
- Notes: ローカルセットアップ時に `tools/whisper.cpp` として導入する想定。

2. `Vosk API`
- Upstream: <https://github.com/alphacep/vosk-api>
- License: `Apache-2.0`
- Notes: `libvosk` をアプリに同梱して再配布する場合は、Apache 2.0 の条件（ライセンス表示の保持など）を満たしてください。

3. `Vosk model` (例: `vosk-model-ja-0.22`)
- Distribution page: <https://alphacephei.com/vosk/models>
- License (listed for `vosk-model-ja-0.22`): `Apache-2.0`
- Notes: このリポジトリにはモデルを同梱していません（`vosk-model-*` は `.gitignore` 管理）。
- Notes: モデルをアプリ配布物（インストーラ、zip、DMG など）に含める場合のみ二次配布として、当該モデルのライセンス条件に従ってください。
- Notes: モデルは種類ごとにライセンスが異なる場合があるため、使用モデルごとに配布ページの表示を確認してください。

4. Rust / Node dependencies
- Sources: `src-tauri/Cargo.lock`, `package.json`, `bun.lock`
- Licenses: 主に `MIT`, `Apache-2.0` 系（依存グラフ上で `MPL-2.0` を含む場合あり）
- Notes: 依存パッケージのライセンス情報は、各パッケージの `LICENSE` / `COPYING` / `package.json` を優先してください。

## 再配布時の実務チェック

1. 配布物に、上記コンポーネントのライセンス文書（または参照先）を含める。  
2. 同梱するバイナリ・モデルが変更された場合、対応するライセンスと著作権表示を更新する。  
3. 依存更新時に、ライセンス種別の差分を確認する。  

## 免責

この文書は法的助言ではありません。最終的な法務判断は必要に応じて専門家に確認してください。
