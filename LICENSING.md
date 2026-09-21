# Licensing

Murabito uses two licences: one for code and one for assets.

| What | Licence | Text |
|---|---|---|
| Code, config, scripts, shaders and docs | MIT | [`LICENSE`](LICENSE) |
| Assets | Creative Commons Attribution-ShareAlike 4.0 International (CC BY-SA 4.0) | [`LICENSE-ASSETS`](LICENSE-ASSETS) |

## What counts as an asset

- Everything under `_Archives/UE-Try/Content/`, including Blueprints and maps. They are `.uasset`/`.umap` files even when they hold logic.
- Art, audio, video and raw-data source files anywhere in the repo: for example `.blend`, `.fbx`, `.psd`, `.kra`, `.png`, `.wav`, `.flac`, `.mp4`, heightmaps and masks.

Everything else is code and falls under the MIT licence.

## Attribution

Assets are © 2026 Lexa, licensed under CC BY-SA 4.0. When you reuse them, credit "Murabito (https://github.com/Lexa-B/Murabito)", link to the licence (https://creativecommons.org/licenses/by-sa/4.0/), say whether you changed them, and share your changed versions under the same licence.

## Third-party material

Files made by others keep their own licences, and they are not covered by either licence above. This includes fonts (for example, fonts under the SIL Open Font License), anything from Epic Games (Unreal Engine content, Starter Content, Fab), and assets from other sources. Put each one's licence file next to it and list it here.

| File | What | Licence |
|---|---|---|
| `_Archives/Bevy-Try-1/assets/fonts/NotoSansJP-Regular.otf` | Noto Sans JP, Regular — the Japanese subset from [notofonts/noto-cjk](https://github.com/notofonts/noto-cjk), © The Noto Project Authors. The archived Bevy attempt's UI font: it covers Latin as well as Japanese. | SIL Open Font License 1.1, in [`_Archives/Bevy-Try-1/assets/fonts/OFL.txt`](_Archives/Bevy-Try-1/assets/fonts/OFL.txt) |

The Unreal Engine itself is not part of this repository and is not covered by either licence; it is used under Epic's own terms.
